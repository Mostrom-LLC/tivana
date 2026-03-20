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

/// A single action within a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchAction {
    /// Action type: click, type, press, scroll, navigate, focus, hover, select
    #[serde(rename = "type")]
    pub action_type: String,

    /// Target element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ActionTarget>,

    /// Text to type (for "type" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Key to press (for "press" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Modifier keys (for "press" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<Vec<String>>,

    /// Scroll direction (for "scroll" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,

    /// Scroll amount (for "scroll" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,

    /// URL (for "navigate" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Value (for "select" action)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Override delay between this action and the next (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
}

/// Result of a single action within a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchActionResult {
    pub success: bool,
    pub action: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub results: Vec<BatchActionResult>,
    pub total_duration_ms: u64,
}

/// Result of a form fill operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFillResult {
    pub fields_completed: usize,
    pub total_fields: usize,
    pub duration_ms: u64,
    pub submitted: bool,
    pub errors: Vec<FormFieldError>,
}

/// Error for a specific form field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormFieldError {
    pub field: String,
    pub error: String,
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

    /// Execute a batch of actions sequentially
    pub async fn execute_batch(
        page: &Arc<PageHandle>,
        actions: &[BatchAction],
        stop_on_error: bool,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> BatchResult {
        let start = std::time::Instant::now();
        let mut results = Vec::with_capacity(actions.len());

        for action in actions {
            let action_start = std::time::Instant::now();
            let action_type = action.action_type.clone();

            let outcome = Self::execute_single_batch_action(page, action, mouse_pos).await;

            let duration_ms = action_start.elapsed().as_millis() as u64;

            match outcome {
                Ok(_) => {
                    results.push(BatchActionResult {
                        success: true,
                        action: action_type,
                        duration_ms,
                        error: None,
                    });
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    results.push(BatchActionResult {
                        success: false,
                        action: action_type,
                        duration_ms,
                        error: Some(err_msg),
                    });
                    if stop_on_error {
                        break;
                    }
                }
            }

            // Inter-action delay (human-like default ~80ms, overridable)
            let delay = action.delay_ms.unwrap_or(80);
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        BatchResult {
            results,
            total_duration_ms,
        }
    }

    /// Execute a single action from a batch
    async fn execute_single_batch_action(
        page: &Arc<PageHandle>,
        action: &BatchAction,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> Result<ActionResult, TivanaError> {
        match action.action_type.as_str() {
            "click" => {
                let target = action
                    .target
                    .as_ref()
                    .ok_or_else(|| TivanaError::Browser("click requires target".into()))?;
                Self::click(page, target, &ClickOptions::default(), mouse_pos).await
            }
            "type" => {
                let text = action
                    .text
                    .as_deref()
                    .ok_or_else(|| TivanaError::Browser("type requires text".into()))?;
                Self::type_text(
                    page,
                    text,
                    action.target.as_ref(),
                    &TypeOptions::default(),
                    mouse_pos,
                )
                .await
            }
            "press" => {
                let key = action
                    .key
                    .as_deref()
                    .ok_or_else(|| TivanaError::Browser("press requires key".into()))?;
                let modifiers = action.modifiers.as_deref().unwrap_or(&[]);
                Self::press(page, key, modifiers).await
            }
            "scroll" => {
                let direction = match action.direction.as_deref() {
                    Some("up") => ScrollDirection::Up,
                    Some("left") => ScrollDirection::Left,
                    Some("right") => ScrollDirection::Right,
                    _ => ScrollDirection::Down,
                };
                let amount = action.amount.map(|a| a as i32).unwrap_or(100);
                let options = ScrollOptions {
                    direction,
                    amount,
                    smooth: true,
                };
                Self::scroll(page, action.target.as_ref(), &options).await
            }
            "navigate" => {
                let url = action
                    .url
                    .as_deref()
                    .ok_or_else(|| TivanaError::Browser("navigate requires url".into()))?;
                Self::navigate(page, url).await
            }
            "hover" => {
                let target = action
                    .target
                    .as_ref()
                    .ok_or_else(|| TivanaError::Browser("hover requires target".into()))?;
                Self::hover(page, target).await
            }
            "focus" => {
                let target = action
                    .target
                    .as_ref()
                    .ok_or_else(|| TivanaError::Browser("focus requires target".into()))?;
                Self::focus(page, target).await
            }
            "select" => {
                let target = action
                    .target
                    .as_ref()
                    .ok_or_else(|| TivanaError::Browser("select requires target".into()))?;
                let value = action
                    .value
                    .as_deref()
                    .ok_or_else(|| TivanaError::Browser("select requires value".into()))?;
                Self::select(page, target, value).await
            }
            other => Err(TivanaError::Browser(format!(
                "Unknown batch action type: {}",
                other
            ))),
        }
    }

    /// Fill a form by mapping field IDs to values
    pub async fn fill_form(
        page: &Arc<PageHandle>,
        fields: &serde_json::Map<String, serde_json::Value>,
        submit: Option<&str>,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> FormFillResult {
        let start = std::time::Instant::now();
        let total_fields = fields.len();
        let mut fields_completed = 0;
        let mut errors = Vec::new();

        for (field_id, value) in fields {
            let target = ActionTarget::element_id(field_id);

            let result = match value {
                serde_json::Value::String(text) => {
                    // String value → click + type
                    Self::type_text(
                        page,
                        text,
                        Some(&target),
                        &TypeOptions::default(),
                        mouse_pos,
                    )
                    .await
                }
                serde_json::Value::Bool(true) => {
                    // true → click (checkbox/radio)
                    Self::click(page, &target, &ClickOptions::default(), mouse_pos).await
                }
                serde_json::Value::Bool(false) => {
                    // false → skip
                    fields_completed += 1;
                    continue;
                }
                _ => {
                    errors.push(FormFieldError {
                        field: field_id.clone(),
                        error: "Unsupported value type".into(),
                    });
                    continue;
                }
            };

            match result {
                Ok(_) => {
                    fields_completed += 1;
                    // Small delay between fields for human-like pacing
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                Err(e) => {
                    errors.push(FormFieldError {
                        field: field_id.clone(),
                        error: e.to_string(),
                    });
                }
            }
        }

        // Submit if requested
        let mut submitted = false;
        if let Some(submit_id) = submit {
            let submit_target = ActionTarget::element_id(submit_id);
            match Self::click(page, &submit_target, &ClickOptions::default(), mouse_pos).await {
                Ok(_) => submitted = true,
                Err(e) => {
                    errors.push(FormFieldError {
                        field: submit_id.to_string(),
                        error: format!("Submit failed: {}", e),
                    });
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        FormFillResult {
            fields_completed,
            total_fields,
            duration_ms,
            submitted,
            errors,
        }
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

    #[test]
    fn test_batch_action_deserialize() {
        let json = serde_json::json!({
            "type": "click",
            "target": { "elementId": "e5" }
        });
        let action: BatchAction = serde_json::from_value(json).unwrap();
        assert_eq!(action.action_type, "click");
        assert!(action.target.is_some());
        assert_eq!(action.target.unwrap().element_id, Some("e5".to_string()));
    }

    #[test]
    fn test_batch_action_type_with_text() {
        let json = serde_json::json!({
            "type": "type",
            "target": { "elementId": "e13" },
            "text": "hello",
            "delayMs": 50
        });
        let action: BatchAction = serde_json::from_value(json).unwrap();
        assert_eq!(action.action_type, "type");
        assert_eq!(action.text, Some("hello".to_string()));
        assert_eq!(action.delay_ms, Some(50));
    }

    #[test]
    fn test_batch_result_serialize() {
        let result = BatchResult {
            results: vec![
                BatchActionResult {
                    success: true,
                    action: "click".to_string(),
                    duration_ms: 45,
                    error: None,
                },
                BatchActionResult {
                    success: false,
                    action: "type".to_string(),
                    duration_ms: 10,
                    error: Some("target not found".to_string()),
                },
            ],
            total_duration_ms: 55,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["results"].as_array().unwrap().len(), 2);
        assert_eq!(json["totalDurationMs"], 55);
    }

    #[test]
    fn test_form_fill_result_serialize() {
        let result = FormFillResult {
            fields_completed: 3,
            total_fields: 4,
            duration_ms: 500,
            submitted: true,
            errors: vec![FormFieldError {
                field: "e10".to_string(),
                error: "not found".to_string(),
            }],
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["fieldsCompleted"], 3);
        assert_eq!(json["submitted"], true);
        assert_eq!(json["errors"].as_array().unwrap().len(), 1);
    }
}
