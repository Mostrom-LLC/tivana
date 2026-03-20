//! Action methods for interacting with page elements
//!
//! This module provides methods for performing actions on the browser page
//! such as clicking, typing, and scrolling.

use std::sync::Arc;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::browser::PageHandle;
use crate::error::TivanaError;
use crate::perceive::{BoundingBox, PageState, Perceiver};

/// Result of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,

    /// Updated page state after action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_state: Option<PageState>,

    /// Action-specific result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Duration of the action in milliseconds
    pub duration_ms: u64,
}

impl ActionResult {
    pub fn success() -> Self {
        Self {
            success: true,
            page_state: None,
            data: None,
            duration_ms: 0,
        }
    }

    pub fn with_page_state(mut self, state: PageState) -> Self {
        self.page_state = Some(state);
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn failure(reason: &str) -> Self {
        Self {
            success: false,
            page_state: None,
            data: Some(serde_json::json!({ "error": reason })),
            duration_ms: 0,
        }
    }
}

/// Target selector for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTarget {
    /// Element ID (e.g., "e1", "e2" from perceive.elements)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,

    /// CSS selector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,

    /// Text content to match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Aria role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Aria label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Coordinates (x, y)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<(f64, f64)>,
}

impl ActionTarget {
    pub fn element_id(id: impl Into<String>) -> Self {
        Self {
            element_id: Some(id.into()),
            selector: None,
            text: None,
            role: None,
            label: None,
            coordinates: None,
        }
    }

    pub fn selector(selector: impl Into<String>) -> Self {
        Self {
            element_id: None,
            selector: Some(selector.into()),
            text: None,
            role: None,
            label: None,
            coordinates: None,
        }
    }

    pub fn role_and_label(role: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            element_id: None,
            selector: None,
            text: None,
            role: Some(role.into()),
            label: Some(label.into()),
            coordinates: None,
        }
    }

    pub fn coordinates(x: f64, y: f64) -> Self {
        Self {
            element_id: None,
            selector: None,
            text: None,
            role: None,
            label: None,
            coordinates: Some((x, y)),
        }
    }

    /// Check if target has any criteria
    pub fn is_empty(&self) -> bool {
        self.element_id.is_none()
            && self.selector.is_none()
            && self.text.is_none()
            && self.role.is_none()
            && self.label.is_none()
            && self.coordinates.is_none()
    }
}

/// Click options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickOptions {
    /// Button: "left", "right", "middle"
    #[serde(default = "default_button")]
    pub button: String,

    /// Number of clicks (1 for single, 2 for double)
    #[serde(default = "default_click_count")]
    pub click_count: u32,

    /// Delay between mousedown and mouseup in ms
    #[serde(default)]
    pub delay_ms: u64,

    /// Modifier keys to hold
    #[serde(default)]
    pub modifiers: Vec<String>,
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self {
            button: default_button(),
            click_count: default_click_count(),
            delay_ms: 0,
            modifiers: Vec::new(),
        }
    }
}

fn default_button() -> String {
    "left".to_string()
}

fn default_click_count() -> u32 {
    1
}

/// Type/input options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeOptions {
    /// Delay between keystrokes in ms
    #[serde(default)]
    pub delay_ms: u64,

    /// Clear existing content first
    #[serde(default)]
    pub clear_first: bool,
}

/// Scroll options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollOptions {
    /// Scroll direction
    pub direction: ScrollDirection,

    /// Amount to scroll in pixels
    #[serde(default = "default_scroll_amount")]
    pub amount: i32,

    /// Smooth scrolling
    #[serde(default = "default_smooth")]
    pub smooth: bool,
}

fn default_scroll_amount() -> i32 {
    100
}

fn default_smooth() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Conditions to wait for
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WaitCondition {
    /// Wait for element to exist
    Element { selector: String },

    /// Wait for element to be visible
    Visible { selector: String },

    /// Wait for element to be hidden
    Hidden { selector: String },

    /// Wait for navigation to complete
    Navigation,

    /// Wait for network idle
    NetworkIdle { idle_time_ms: u64 },

    /// Wait for specific time
    Delay { duration_ms: u64 },
}

/// Action executor
pub struct Actor;

impl Actor {
    /// Scroll a target element into the viewport so CDP clicks can reach it
    async fn scroll_target_into_view(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
    ) -> Result<(), TivanaError> {
        // Build a JS expression to find the element and scrollIntoView
        let script = if let Some(ref id) = target.element_id {
            // Element ID — find by data-tivana-id attribute
            format!(
                r#"(() => {{
                    const el = document.querySelector('[data-tivana-id="' + {} + '"]');
                    if (el) {{
                        el.scrollIntoView({{ behavior: 'instant', block: 'center' }});
                        return true;
                    }}
                    return false;
                }})()"#,
                serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string())
            )
        } else if let Some(ref selector) = target.selector {
            format!(
                r#"(() => {{
                    const el = document.querySelector({});
                    if (el) {{ el.scrollIntoView({{ behavior: 'instant', block: 'center' }}); return true; }}
                    return false;
                }})()"#,
                serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string())
            )
        } else {
            // For coordinates or role-based targets, skip scroll
            return Ok(());
        };

        let _: Option<bool> = page.evaluate(&script).await.ok();
        Ok(())
    }

    /// Resolve action target to bounding box
    async fn resolve_target(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
    ) -> Result<BoundingBox, TivanaError> {
        // Priority: coordinates > element_id > selector > role+label

        if let Some((x, y)) = target.coordinates {
            // Direct coordinates - return a point-sized box
            return Ok(BoundingBox {
                x,
                y,
                width: 1.0,
                height: 1.0,
            });
        }

        if let Some(ref id) = target.element_id {
            // Element ID from perceive
            if let Some(bounds) = Perceiver::resolve_element_bounds(page, id).await? {
                return Ok(bounds);
            }
            return Err(TivanaError::Browser(format!("Element not found: {}", id)));
        }

        if let Some(ref selector) = target.selector {
            // CSS selector
            let script = format!(
                r#"(() => {{
                    const el = document.querySelector({});
                    if (!el) return null;
                    const rect = el.getBoundingClientRect();
                    if (rect.width === 0 && rect.height === 0) return null;
                    return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};
                }})()"#,
                serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string())
            );

            let bounds: Option<BoundingBox> = page.evaluate(&script).await?;
            if let Some(b) = bounds {
                return Ok(b);
            }
            return Err(TivanaError::Browser(format!(
                "Element not found: {}",
                selector
            )));
        }

        if target.role.is_some() || target.label.is_some() {
            // Role and/or label based lookup
            let role = target.role.as_deref().unwrap_or("*");
            let label = target.label.as_deref();

            if let Some((bounds, _)) = Perceiver::find_by_role_and_name(page, role, label).await? {
                return Ok(bounds);
            }
            return Err(TivanaError::Browser(format!(
                "Element not found with role='{}' label={:?}",
                role, label
            )));
        }

        Err(TivanaError::Browser(
            "No target specified for action".to_string(),
        ))
    }

    /// Navigate to a URL
    pub async fn navigate(page: &Arc<PageHandle>, url: &str) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(url, "Navigating");

        let nav_result = page.navigate(url).await?;

        // Wait for navigation to settle
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let duration = start.elapsed().as_millis() as u64;

        let page_state = Perceiver::page_state(page).await.ok();

        let mut result = ActionResult::success()
            .with_data(serde_json::to_value(&nav_result).unwrap_or_default())
            .with_duration(duration);

        if let Some(state) = page_state {
            result = result.with_page_state(state);
        }

        Ok(result)
    }

    /// Generate intermediate points along a cubic Bezier curve for human-like mouse movement
    fn bezier_points(
        from: (f64, f64),
        to: (f64, f64),
        num_points: usize,
    ) -> Vec<(f64, f64)> {
        let mut rng = rand::thread_rng();
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;

        // Random control points offset perpendicular to the line
        let cp1 = (
            from.0 + dx * 0.25 + rng.gen_range(-30.0..30.0),
            from.1 + dy * 0.25 + rng.gen_range(-30.0..30.0),
        );
        let cp2 = (
            from.0 + dx * 0.75 + rng.gen_range(-30.0..30.0),
            from.1 + dy * 0.75 + rng.gen_range(-30.0..30.0),
        );

        let mut points = Vec::with_capacity(num_points);
        for i in 1..=num_points {
            let t = i as f64 / (num_points + 1) as f64;
            let u = 1.0 - t;
            let x = u * u * u * from.0
                + 3.0 * u * u * t * cp1.0
                + 3.0 * u * t * t * cp2.0
                + t * t * t * to.0;
            let y = u * u * u * from.1
                + 3.0 * u * u * t * cp1.1
                + 3.0 * u * t * t * cp2.1
                + t * t * t * to.1;
            points.push((x, y));
        }
        points
    }

    /// Click on a target element with human-like mouse movement
    pub async fn click(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
        options: &ClickOptions,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(?target, ?options, "Clicking");

        // Scroll element into view first, then re-resolve bounds
        Self::scroll_target_into_view(page, target).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let bounds = Self::resolve_target(page, target).await?;
        let (cx, cy) = bounds.center();

        // Add small random offset to click position (±2px)
        // Compute all random values before any .await (ThreadRng is not Send)
        let (x, y, move_delays) = {
            let mut rng = rand::thread_rng();
            let x = cx + rng.gen_range(-2.0..2.0);
            let y = cy + rng.gen_range(-2.0..2.0);
            // Pre-generate per-point delays for mouse movement (3-5 points, 5-15ms each)
            let num_delays = rng.gen_range(3..=5);
            let delays: Vec<u64> = (0..num_delays).map(|_| rng.gen_range(5..=15)).collect();
            (x, y, delays)
        };

        // Human-like mouse movement via Bezier curve
        if let Some(pos_lock) = mouse_pos {
            let from = { *pos_lock.read().await };
            let points = Self::bezier_points(from, (x, y), move_delays.len());

            for (point, delay) in points.iter().zip(move_delays.iter()) {
                page.move_mouse_to(point.0, point.1).await?;
                tokio::time::sleep(tokio::time::Duration::from_millis(*delay)).await;
            }

            // Update last mouse position
            *pos_lock.write().await = (x, y);
        }

        // Click at the target
        page.click_at(x, y).await?;

        // Handle double-click
        if options.click_count > 1 {
            for _ in 1..options.click_count {
                if options.delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(options.delay_ms)).await;
                }
                page.click_at(x, y).await?;
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        // Wait briefly for potential navigation to settle
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Page state may fail if the click caused navigation (context destroyed)
        let page_state = Perceiver::page_state(page).await.ok();

        let mut result = ActionResult::success()
            .with_data(serde_json::json!({
                "clickedAt": { "x": x, "y": y }
            }))
            .with_duration(duration);

        if let Some(state) = page_state {
            result = result.with_page_state(state);
        }

        Ok(result)
    }

    /// Generate a gaussian-distributed random value using Box-Muller transform
    fn gaussian(mean: f64, stddev: f64) -> f64 {
        let mut rng = rand::thread_rng();
        let u1: f64 = rng.gen_range(0.0001..1.0);
        let u2: f64 = rng.gen_range(0.0..1.0);
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        (mean + stddev * z).max(10.0) // floor at 10ms
    }

    /// Type text with human-like cadence (gaussian per-character delay, occasional pauses)
    pub async fn type_text(
        page: &Arc<PageHandle>,
        text: &str,
        target: Option<&ActionTarget>,
        options: &TypeOptions,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(text_len = text.len(), ?target, ?options, "Typing");

        // If target specified, click it first to focus
        if let Some(t) = target {
            if !t.is_empty() {
                Self::click(page, t, &ClickOptions::default(), mouse_pos).await?;
                // Small delay after click to ensure focus
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }

        // Clear existing content if requested
        if options.clear_first {
            page.evaluate_void("document.activeElement?.select?.()")
                .await?;
        }

        // Pre-compute all per-character delays (ThreadRng is not Send)
        let char_delays: Vec<(u64, Option<u64>)> = {
            let mut rng = rand::thread_rng();
            let pause_interval: usize = rng.gen_range(5..=10);
            text.chars()
                .enumerate()
                .map(|(i, _)| {
                    let delay = Self::gaussian(80.0, 30.0) as u64;
                    let pause = if i > 0 && i % pause_interval == 0 {
                        Some(rng.gen_range(200..=400u64))
                    } else {
                        None
                    };
                    (delay, pause)
                })
                .collect()
        };

        // Type each character with human-like cadence
        for (ch, (delay, pause)) in text.chars().zip(char_delays.iter()) {
            page.type_text(&ch.to_string()).await?;

            tokio::time::sleep(tokio::time::Duration::from_millis(*delay)).await;

            if let Some(p) = pause {
                tokio::time::sleep(tokio::time::Duration::from_millis(*p)).await;
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_data(serde_json::json!({
                "typed": text.len()
            }))
            .with_duration(duration))
    }

    /// Press a key or key combination
    pub async fn press(
        page: &Arc<PageHandle>,
        key: &str,
        modifiers: &[String],
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(key, ?modifiers, "Pressing key");

        page.press_key(key, modifiers).await?;

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        let key_desc = if modifiers.is_empty() {
            key.to_string()
        } else {
            format!("{}+{}", modifiers.join("+"), key)
        };

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_data(serde_json::json!({
                "key": key_desc
            }))
            .with_duration(duration))
    }

    /// Scroll the page or element into view
    pub async fn scroll(
        page: &Arc<PageHandle>,
        target: Option<&ActionTarget>,
        options: &ScrollOptions,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(?target, ?options, "Scrolling");

        if let Some(t) = target {
            if !t.is_empty() {
                // Scroll element into view
                if let Some(ref selector) = t.selector {
                    let script = format!(
                        r#"document.querySelector({})?.scrollIntoView({{ behavior: '{}', block: 'center' }})"#,
                        serde_json::to_string(selector).unwrap_or_default(),
                        if options.smooth { "smooth" } else { "instant" }
                    );
                    page.evaluate_void(&script).await?;
                } else if let Some(ref element_id) = t.element_id {
                    // Re-query elements to find the matching one and scroll
                    let elements = Perceiver::elements(page).await?;
                    if let Some(el) = elements.iter().find(|e| &e.id == element_id) {
                        if let Some(bounds) = &el.bounds {
                            let script = format!(
                                "window.scrollTo({{ top: {}, behavior: '{}' }})",
                                bounds.y,
                                if options.smooth { "smooth" } else { "instant" }
                            );
                            page.evaluate_void(&script).await?;
                        }
                    }
                }
            }
        } else {
            // Scroll page in direction
            let (dx, dy) = match options.direction {
                ScrollDirection::Up => (0, -(options.amount as i64)),
                ScrollDirection::Down => (0, options.amount as i64),
                ScrollDirection::Left => (-(options.amount as i64), 0),
                ScrollDirection::Right => (options.amount as i64, 0),
            };

            let script = format!(
                "window.scrollBy({{ left: {}, top: {}, behavior: '{}' }})",
                dx,
                dy,
                if options.smooth { "smooth" } else { "instant" }
            );
            page.evaluate_void(&script).await?;
        }

        // Small delay for scroll animation
        if options.smooth {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_duration(duration))
    }

    /// Hover over a target element
    pub async fn hover(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        debug!(?target, "Hovering");

        let bounds = Self::resolve_target(page, target).await?;
        let (x, y) = bounds.center();

        // Move mouse to element (uses chromiumoxide's mouse_move equivalent via evaluate)
        let script = format!(
            r#"(() => {{
                const event = new MouseEvent('mouseover', {{
                    view: window,
                    bubbles: true,
                    cancelable: true,
                    clientX: {},
                    clientY: {}
                }});
                const el = document.elementFromPoint({}, {});
                el?.dispatchEvent(event);
            }})()"#,
            x, y, x, y
        );
        page.evaluate_void(&script).await?;

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_duration(duration))
    }

    /// Focus a target element
    pub async fn focus(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        debug!(?target, "Focusing");

        if let Some(ref selector) = target.selector {
            let script = format!(
                "document.querySelector({})?.focus()",
                serde_json::to_string(selector).unwrap_or_default()
            );
            page.evaluate_void(&script).await?;
        } else if target.element_id.is_some() {
            // Click to focus element by ID
            Self::click(page, target, &ClickOptions::default(), None).await?;
        } else {
            return Err(TivanaError::Browser(
                "Focus requires selector or element_id".to_string(),
            ));
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_duration(duration))
    }

    /// Select an option from a dropdown
    pub async fn select(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
        value: &str,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(?target, value, "Selecting");

        if let Some(ref selector) = target.selector {
            let script = format!(
                r#"(() => {{
                    const el = document.querySelector({});
                    if (!el) return false;
                    el.value = {};
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }})()"#,
                serde_json::to_string(selector).unwrap_or_default(),
                serde_json::to_string(value).unwrap_or_default()
            );

            let success: bool = page.evaluate(&script).await?;
            if !success {
                return Err(TivanaError::Browser(format!(
                    "Select element not found: {}",
                    selector
                )));
            }
        } else {
            return Err(TivanaError::Browser("Select requires selector".to_string()));
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_data(serde_json::json!({ "selected": value }))
            .with_duration(duration))
    }

    /// Wait for a condition
    pub async fn wait_for(
        page: &Arc<PageHandle>,
        condition: &WaitCondition,
        timeout_ms: u64,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        debug!(?condition, timeout_ms, "Waiting");

        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        match condition {
            WaitCondition::Element { selector } => {
                let script = format!(
                    "document.querySelector({}) !== null",
                    serde_json::to_string(selector).unwrap_or_default()
                );

                loop {
                    let exists: bool = page.evaluate(&script).await?;
                    if exists {
                        break;
                    }
                    if tokio::time::Instant::now() > deadline {
                        return Err(TivanaError::Browser(format!(
                            "Timeout waiting for element: {}",
                            selector
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            WaitCondition::Visible { selector } => {
                let script = format!(
                    r#"(() => {{
                        const el = document.querySelector({});
                        if (!el) return false;
                        const style = window.getComputedStyle(el);
                        const rect = el.getBoundingClientRect();
                        return style.display !== 'none' &&
                               style.visibility !== 'hidden' &&
                               rect.width > 0 &&
                               rect.height > 0;
                    }})()"#,
                    serde_json::to_string(selector).unwrap_or_default()
                );

                loop {
                    let visible: bool = page.evaluate(&script).await?;
                    if visible {
                        break;
                    }
                    if tokio::time::Instant::now() > deadline {
                        return Err(TivanaError::Browser(format!(
                            "Timeout waiting for visible: {}",
                            selector
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            WaitCondition::Hidden { selector } => {
                let script = format!(
                    r#"(() => {{
                        const el = document.querySelector({});
                        if (!el) return true;
                        const style = window.getComputedStyle(el);
                        return style.display === 'none' || style.visibility === 'hidden';
                    }})()"#,
                    serde_json::to_string(selector).unwrap_or_default()
                );

                loop {
                    let hidden: bool = page.evaluate(&script).await?;
                    if hidden {
                        break;
                    }
                    if tokio::time::Instant::now() > deadline {
                        return Err(TivanaError::Browser(format!(
                            "Timeout waiting for hidden: {}",
                            selector
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            WaitCondition::Navigation => {
                // Wait for document ready state
                loop {
                    let ready: String = page.evaluate("document.readyState").await?;
                    if ready == "complete" {
                        break;
                    }
                    if tokio::time::Instant::now() > deadline {
                        return Err(TivanaError::Browser(
                            "Timeout waiting for navigation".to_string(),
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            WaitCondition::NetworkIdle { idle_time_ms: _ } => {
                // Simplified: just wait for document complete
                loop {
                    let ready: String = page.evaluate("document.readyState").await?;
                    if ready == "complete" {
                        break;
                    }
                    if tokio::time::Instant::now() > deadline {
                        return Err(TivanaError::Browser(
                            "Timeout waiting for network idle".to_string(),
                        ));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            WaitCondition::Delay { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_duration(duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_target_element_id() {
        let target = ActionTarget::element_id("e5");
        assert_eq!(target.element_id, Some("e5".to_string()));
        assert!(!target.is_empty());
    }

    #[test]
    fn test_action_target_selector() {
        let target = ActionTarget::selector("button.submit");
        assert_eq!(target.selector, Some("button.submit".to_string()));
    }

    #[test]
    fn test_action_target_coordinates() {
        let target = ActionTarget::coordinates(100.0, 200.0);
        assert_eq!(target.coordinates, Some((100.0, 200.0)));
    }

    #[test]
    fn test_click_options_default() {
        let opts = ClickOptions::default();
        assert_eq!(opts.button, "left");
        assert_eq!(opts.click_count, 1);
    }

    #[test]
    fn test_action_result() {
        let result = ActionResult::success()
            .with_data(serde_json::json!({"clicked": true}))
            .with_duration(50);
        assert!(result.success);
        assert!(result.data.is_some());
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn test_action_result_failure() {
        let result = ActionResult::failure("Element not found");
        assert!(!result.success);
        assert!(result.data.is_some());
    }
}
