//! Action methods for interacting with page elements
//!
//! This module provides methods for performing actions on the browser page
//! such as clicking, typing, and scrolling.

use std::sync::Arc;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::browser::PageHandle;
use crate::error::TivanaError;
use crate::perceive::{BoundingBox, FormField, PageState, Perceiver};

/// Maximum retries for stale element recovery
const STALE_ELEMENT_MAX_RETRIES: u32 = 3;

/// Delay between stale element retries in milliseconds
const STALE_ELEMENT_RETRY_DELAY_MS: u64 = 200;

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

    /// Human-like pacing delays
    #[serde(default)]
    pub pacing: PacingConfig,
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self {
            button: default_button(),
            click_count: default_click_count(),
            delay_ms: 0,
            modifiers: Vec::new(),
            pacing: PacingConfig::default(),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeOptions {
    /// Delay between keystrokes in ms
    #[serde(default)]
    pub delay_ms: u64,

    /// Clear existing content first
    #[serde(default)]
    pub clear_first: bool,

    /// Human-like pacing delays (applied before/after the full type sequence)
    #[serde(default)]
    pub pacing: PacingConfig,
}

impl Default for TypeOptions {
    fn default() -> Self {
        Self {
            delay_ms: 0,
            clear_first: false,
            pacing: PacingConfig::default(),
        }
    }
}

/// Human-like pacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacingConfig {
    /// Minimum delay before action in milliseconds (default: 0)
    #[serde(default)]
    pub pre_delay_ms: u64,

    /// Maximum delay before action in milliseconds (default: 0)
    /// Actual delay is random between pre_delay_ms and this value
    #[serde(default)]
    pub pre_delay_max_ms: u64,

    /// Minimum delay after action in milliseconds (default: 0)
    #[serde(default)]
    pub post_delay_ms: u64,

    /// Maximum delay after action in milliseconds (default: 0)
    #[serde(default)]
    pub post_delay_max_ms: u64,
}

impl Default for PacingConfig {
    fn default() -> Self {
        Self {
            pre_delay_ms: 0,
            pre_delay_max_ms: 0,
            post_delay_ms: 0,
            post_delay_max_ms: 0,
        }
    }
}

impl PacingConfig {
    /// Human-like preset: 200-800ms before, 100-400ms after
    pub fn human() -> Self {
        Self {
            pre_delay_ms: 200,
            pre_delay_max_ms: 800,
            post_delay_ms: 100,
            post_delay_max_ms: 400,
        }
    }

    /// Apply pre-action delay
    pub async fn pre_delay(&self) {
        if self.pre_delay_max_ms > 0 {
            let delay = if self.pre_delay_max_ms > self.pre_delay_ms {
                let mut rng = rand::thread_rng();
                rng.gen_range(self.pre_delay_ms..=self.pre_delay_max_ms)
            } else {
                self.pre_delay_ms
            };
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }
    }

    /// Apply post-action delay
    pub async fn post_delay(&self) {
        if self.post_delay_max_ms > 0 {
            let delay = if self.post_delay_max_ms > self.post_delay_ms {
                let mut rng = rand::thread_rng();
                rng.gen_range(self.post_delay_ms..=self.post_delay_max_ms)
            } else {
                self.post_delay_ms
            };
            if delay > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

/// Options for fill action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillOptions {
    /// Clear existing content before filling (default: true)
    #[serde(default = "default_true")]
    pub clear_first: bool,
}

impl Default for FillOptions {
    fn default() -> Self {
        Self { clear_first: true }
    }
}

fn default_true() -> bool {
    true
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

/// Result of a smart form fill operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartFillResult {
    pub fields_filled: Vec<SmartFillMatch>,
    pub fields_skipped: Vec<SmartFillSkip>,
    pub total_fields: usize,
    pub filled_count: usize,
    pub duration_ms: u64,
    pub errors: Vec<FormFieldError>,
}

/// A field that was successfully matched and filled
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartFillMatch {
    pub tivana_id: String,
    pub label: String,
    pub profile_key: String,
    pub value: String,
}

/// A field that could not be matched
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartFillSkip {
    pub tivana_id: Option<String>,
    pub label: Option<String>,
    pub reason: String,
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

    /// Resolve action target with automatic retry on stale element references.
    ///
    /// When an element_id lookup fails (DOM mutation made the data-tivana-id stale),
    /// re-enumerates elements via Perceiver::elements() to refresh IDs and retries.
    async fn resolve_target_with_retry(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
    ) -> Result<BoundingBox, TivanaError> {
        let result = Self::resolve_target(page, target).await;

        match &result {
            Ok(_) => return result,
            Err(_) if target.element_id.is_some() => {
                // Element ID lookup failed — try re-enumerating to refresh data-tivana-id attrs
                let element_id = target.element_id.as_ref().unwrap();
                for attempt in 1..=STALE_ELEMENT_MAX_RETRIES {
                    warn!(
                        element_id,
                        attempt,
                        "Stale element detected, re-enumerating"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        STALE_ELEMENT_RETRY_DELAY_MS,
                    ))
                    .await;

                    // Re-enumerate refreshes data-tivana-id attributes on the DOM
                    let _ = Perceiver::elements(page).await;

                    // Retry resolution
                    if let Ok(bounds) = Self::resolve_target(page, target).await {
                        info!(element_id, attempt, "Stale element recovered");
                        return Ok(bounds);
                    }
                }
                // All retries exhausted — return original error
                result
            }
            Err(_) => result,
        }
    }

    /// Navigate to a URL, waiting for DOMContentLoaded before returning
    pub async fn navigate(page: &Arc<PageHandle>, url: &str) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(url, "Navigating");

        let nav_result = page.navigate(url).await?;

        // Wait for at least DOMContentLoaded (interactive) instead of a hardcoded sleep
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
        loop {
            let ready: String = page
                .evaluate("document.readyState")
                .await
                .unwrap_or_else(|_| "loading".to_string());
            if ready == "interactive" || ready == "complete" {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                break; // Don't error — navigation happened, page just didn't fully load
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

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

    /// Handle a JavaScript dialog (alert/confirm/prompt)
    pub async fn handle_dialog(
        page: &Arc<PageHandle>,
        action: &str,
        prompt_text: Option<&str>,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(action, ?prompt_text, "Handling dialog");

        let accept = action == "accept";
        let text = prompt_text.unwrap_or("");

        // Use CDP Page.handleJavaScriptDialog
        let cmd = chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams::builder()
            .accept(accept)
            .prompt_text(text)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build dialog command: {}", e)))?;

        page.inner()
            .execute(cmd)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to handle dialog: {}", e)))?;

        let duration = start.elapsed().as_millis() as u64;

        Ok(ActionResult::success()
            .with_data(serde_json::json!({
                "action": action,
                "promptText": text,
            }))
            .with_duration(duration))
    }

    /// Upload files to a file input element via CDP DOM.setFileInputFiles
    pub async fn upload_file(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
        file_paths: &[String],
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(?target, file_count = file_paths.len(), "Uploading files");

        // Build JS to find the element and get its backendNodeId via Runtime.evaluate
        let find_script = if let Some(ref id) = target.element_id {
            format!(
                r#"document.querySelector('[data-tivana-id="' + {} + '"]')"#,
                serde_json::to_string(id).unwrap_or_else(|_| "\"\"".to_string())
            )
        } else if let Some(ref selector) = target.selector {
            format!(
                "document.querySelector({})",
                serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string())
            )
        } else {
            return Err(TivanaError::Browser(
                "uploadFile requires elementId or selector target".to_string(),
            ));
        };

        // Use Runtime.evaluate to get a RemoteObject, then DOM.describeNode for backendNodeId
        use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
        let eval_params = EvaluateParams::builder()
            .expression(&find_script)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build evaluate: {}", e)))?;

        let eval_result = page
            .inner()
            .execute(eval_params)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to evaluate: {}", e)))?;

        let object_id = eval_result
            .result
            .result
            .object_id
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Element not found for file upload".to_string()))?;

        // Use DOM.describeNode to get backendNodeId
        use chromiumoxide::cdp::browser_protocol::dom::{DescribeNodeParams, SetFileInputFilesParams};
        let describe_params = DescribeNodeParams::builder()
            .object_id(object_id.clone())
            .build();

        let describe_result = page
            .inner()
            .execute(describe_params)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to describe node: {}", e)))?;

        let backend_node_id = describe_result.node.backend_node_id;

        // Use DOM.setFileInputFiles with backendNodeId
        let set_files_params = SetFileInputFilesParams::builder()
            .files(file_paths.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
            .backend_node_id(backend_node_id)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build setFileInputFiles: {}", e)))?;

        page.inner()
            .execute(set_files_params)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to set files: {}", e)))?;

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await.ok();

        let mut result = ActionResult::success()
            .with_data(serde_json::json!({
                "filesSet": file_paths.len(),
                "filePaths": file_paths,
            }))
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

        options.pacing.pre_delay().await;

        // Scroll element into view first, then re-resolve bounds
        Self::scroll_target_into_view(page, target).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let bounds = Self::resolve_target_with_retry(page, target).await?;
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

        options.pacing.post_delay().await;

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

        options.pacing.pre_delay().await;

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

        let result = ActionResult::success()
            .with_page_state(page_state)
            .with_data(serde_json::json!({
                "typed": text.len()
            }))
            .with_duration(duration);

        options.pacing.post_delay().await;

        Ok(result)
    }

    /// Fill an element's value directly via JavaScript
    ///
    /// Unlike `type_text` which sends character-by-character CDP key events,
    /// `fill` sets `element.value` directly and dispatches input/change events.
    /// This is instant regardless of text length and avoids typing detection.
    pub async fn fill(
        page: &Arc<PageHandle>,
        target: &ActionTarget,
        value: &str,
        options: &FillOptions,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        info!(value_len = value.len(), ?target, "Filling");

        // If target specified, click it first to focus
        if !target.is_empty() {
            Self::click(page, target, &ClickOptions::default(), None).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Resolve element — prefer element_id with data-tivana-id, fall back to selector
        let selector = if let Some(ref eid) = target.element_id {
            format!("[data-tivana-id=\"{}\"]", eid)
        } else if let Some(ref sel) = target.selector {
            sel.clone()
        } else {
            // Use activeElement as fallback
            "null".to_string()
        };

        let clear_first = if options.clear_first { "true" } else { "false" };

        let script = format!(
            r#"(() => {{
                const el = {} === "null"
                    ? document.activeElement
                    : document.querySelector({});
                if (!el) return {{ success: false, error: "Element not found" }};

                if ({}) {{
                    el.value = '';
                }}

                // Set value using native setter to trigger React/Vue/etc.
                const nativeSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype, 'value'
                )?.set || Object.getOwnPropertyDescriptor(
                    window.HTMLTextAreaElement.prototype, 'value'
                )?.set;

                if (nativeSetter) {{
                    nativeSetter.call(el, {});
                }} else {{
                    el.value = {};
                }}

                // Dispatch events frameworks listen for
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));

                return {{ success: true, length: el.value.length }};
            }})()"#,
            serde_json::to_string(&selector).unwrap_or_default(),
            serde_json::to_string(&selector).unwrap_or_default(),
            clear_first,
            serde_json::to_string(value).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default()
        );

        let result: serde_json::Value = page.evaluate(&script).await?;
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Fill failed");
            return Err(TivanaError::Browser(error.to_string()));
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_data(serde_json::json!({
                "filled": true,
                "length": value.len(),
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

        let bounds = Self::resolve_target_with_retry(page, target).await?;
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

        // Resolve selector — from element_id OR direct selector
        let selector = if let Some(ref eid) = target.element_id {
            format!("[data-tivana-id=\"{}\"]", eid)
        } else if let Some(ref sel) = target.selector {
            sel.clone()
        } else {
            return Err(TivanaError::Browser(
                "Select requires element ID or selector".to_string(),
            ));
        };

        let script = format!(
            r#"(() => {{
                const el = document.querySelector({});
                if (!el) return {{ success: false, error: "Element not found" }};

                // Handle native <select>
                if (el.tagName === 'SELECT') {{
                    el.value = {};
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return {{ success: true, selected: el.value }};
                }}

                // Handle custom dropdowns — set value and dispatch
                const nativeSetter = Object.getOwnPropertyDescriptor(
                    window.HTMLInputElement.prototype, 'value'
                )?.set;
                if (nativeSetter) {{
                    nativeSetter.call(el, {});
                }} else {{
                    el.value = {};
                }}
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return {{ success: true, selected: el.value }};
            }})()"#,
            serde_json::to_string(&selector).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default()
        );

        let result: serde_json::Value = page.evaluate(&script).await?;
        let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Select failed");
            return Err(TivanaError::Browser(format!(
                "Select failed: {}",
                error
            )));
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

    /// Wait for a CSS selector to match a visible element on the page.
    ///
    /// Polls every 100ms until `document.querySelector(selector)` exists and is visible.
    /// Returns the matched element info on success or errors on timeout.
    pub async fn wait_for_selector(
        page: &Arc<PageHandle>,
        selector: &str,
        timeout_ms: u64,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        let script = format!(
            r#"(() => {{
                const el = document.querySelector({sel});
                if (!el) return null;
                const style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return null;
                const rect = el.getBoundingClientRect();
                if (rect.width === 0 && rect.height === 0) return null;
                return {{
                    tagName: el.tagName.toLowerCase(),
                    text: el.innerText?.trim()?.slice(0, 200) || null,
                    bounds: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }}
                }};
            }})()"#,
            sel = serde_json::to_string(selector).unwrap_or_default()
        );

        loop {
            let result: Option<serde_json::Value> = page.evaluate(&script).await?;
            if let Some(element_data) = result {
                let duration = start.elapsed().as_millis() as u64;
                let page_state = Perceiver::page_state(page).await?;
                return Ok(ActionResult::success()
                    .with_page_state(page_state)
                    .with_data(element_data)
                    .with_duration(duration));
            }
            if tokio::time::Instant::now() > deadline {
                return Err(TivanaError::Browser(format!(
                    "Timeout waiting for selector: {}",
                    selector
                )));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Wait for a page navigation (URL change + DOMContentLoaded).
    ///
    /// Records the current URL, polls every 100ms until the URL changes,
    /// then waits for the page to reach at least "interactive" readyState.
    pub async fn wait_for_navigation(
        page: &Arc<PageHandle>,
        timeout_ms: u64,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        // Record current URL
        let initial_url: String = page.evaluate("window.location.href").await?;

        // Wait for URL to change
        loop {
            let current_url: String = page.evaluate("window.location.href").await?;
            if current_url != initial_url {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                return Err(TivanaError::Browser(
                    "Timeout waiting for navigation".to_string(),
                ));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Wait for page to settle (at least DOMContentLoaded / interactive)
        loop {
            let ready: String = page
                .evaluate("document.readyState")
                .await
                .unwrap_or_else(|_| "loading".to_string());
            if ready == "interactive" || ready == "complete" {
                break;
            }
            if tokio::time::Instant::now() > deadline {
                break; // Don't error — navigation happened, just didn't fully load
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let duration = start.elapsed().as_millis() as u64;
        let page_state = Perceiver::page_state(page).await?;

        Ok(ActionResult::success()
            .with_page_state(page_state)
            .with_duration(duration))
    }

    /// Wait for a JavaScript expression to return a truthy value.
    ///
    /// Polls every 100ms until `expression` evaluates to a truthy value.
    /// Returns the expression result on success or errors on timeout.
    pub async fn wait_for_function(
        page: &Arc<PageHandle>,
        expression: &str,
        timeout_ms: u64,
    ) -> Result<ActionResult, TivanaError> {
        let start = std::time::Instant::now();
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        // Wrap in an IIFE that returns the value only if truthy, else null
        let script = format!(
            r#"(() => {{
                const __result = (function() {{ return ({expr}); }})();
                return __result ? __result : null;
            }})()"#,
            expr = expression
        );

        loop {
            let result: serde_json::Value = page
                .evaluate(&script)
                .await
                .unwrap_or(serde_json::Value::Null);

            if !result.is_null() {
                let duration = start.elapsed().as_millis() as u64;
                let page_state = Perceiver::page_state(page).await?;
                return Ok(ActionResult::success()
                    .with_page_state(page_state)
                    .with_data(serde_json::json!({ "result": result }))
                    .with_duration(duration));
            }
            if tokio::time::Instant::now() > deadline {
                return Err(TivanaError::Browser(format!(
                    "Timeout waiting for function: {}",
                    expression
                )));
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Smart fill a form by matching field labels to profile keys using fuzzy matching
    pub async fn smart_fill(
        page: &Arc<PageHandle>,
        profile: &serde_json::Map<String, serde_json::Value>,
        skip_recaptcha: bool,
        mouse_pos: Option<&Arc<RwLock<(f64, f64)>>>,
    ) -> SmartFillResult {
        let start = std::time::Instant::now();
        let mut fields_filled = Vec::new();
        let mut fields_skipped = Vec::new();
        let mut errors = Vec::new();

        // Get form fields via perceive
        let form_fields = match Perceiver::form_fields(page).await {
            Ok(f) => f,
            Err(e) => {
                return SmartFillResult {
                    fields_filled: vec![],
                    fields_skipped: vec![],
                    total_fields: 0,
                    filled_count: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    errors: vec![FormFieldError {
                        field: "*".into(),
                        error: format!("Failed to get form fields: {}", e),
                    }],
                };
            }
        };

        let total_fields = form_fields.len();

        for field in &form_fields {
            // Skip invisible fields
            if !field.visible {
                continue;
            }

            // Skip disabled fields
            if field.disabled {
                continue;
            }

            let tivana_id = match &field.tivana_id {
                Some(id) => id.clone(),
                None => {
                    fields_skipped.push(SmartFillSkip {
                        tivana_id: None,
                        label: field.label.clone(),
                        reason: "No tivana ID".into(),
                    });
                    continue;
                }
            };

            // Skip recaptcha fields if requested
            if skip_recaptcha {
                if let Some(ref name) = field.name {
                    if name.to_lowercase().contains("recaptcha") {
                        continue;
                    }
                }
            }

            // Compute a normalized label for matching
            let label_str = field
                .label
                .as_deref()
                .or(field.name.as_deref())
                .or(field.id.as_deref())
                .unwrap_or("");

            let label_lower = label_str.to_lowercase();

            // Try to match the label to a profile key
            let matched_key = Self::match_label_to_profile_key(&label_lower, profile, field);

            match matched_key {
                Some((key, value)) => {
                    let field_type = field.r#type.as_deref().unwrap_or("");
                    let tag = field.tag_name.as_str();

                    let fill_result = if tag == "select" {
                        // For select: pick the best matching option
                        Self::smart_fill_select(page, &tivana_id, &value, field).await
                    } else if field_type == "radio" {
                        // For radio: determine yes/no from value
                        Self::smart_fill_radio(page, &tivana_id, &value, field).await
                    } else if field_type == "checkbox" {
                        // For checkbox: check if value is truthy
                        let should_check = matches!(
                            value.to_lowercase().as_str(),
                            "true" | "yes" | "1" | "y"
                        );
                        let is_checked = field.checked.unwrap_or(false);
                        if should_check != is_checked {
                            let target = ActionTarget::element_id(&tivana_id);
                            Self::click(page, &target, &ClickOptions::default(), mouse_pos).await
                        } else {
                            Ok(ActionResult::success())
                        }
                    } else {
                        // Text input: click + type
                        let target = ActionTarget::element_id(&tivana_id);
                        Self::type_text(
                            page,
                            &value,
                            Some(&target),
                            &TypeOptions {
                                clear_first: true,
                                ..Default::default()
                            },
                            mouse_pos,
                        )
                        .await
                    };

                    match fill_result {
                        Ok(_) => {
                            fields_filled.push(SmartFillMatch {
                                tivana_id: tivana_id.clone(),
                                label: label_str.to_string(),
                                profile_key: key.clone(),
                                value,
                            });
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                        }
                        Err(e) => {
                            errors.push(FormFieldError {
                                field: tivana_id.clone(),
                                error: e.to_string(),
                            });
                        }
                    }
                }
                None => {
                    fields_skipped.push(SmartFillSkip {
                        tivana_id: Some(tivana_id),
                        label: Some(label_str.to_string()),
                        reason: "No matching profile key".into(),
                    });
                }
            }
        }

        let filled_count = fields_filled.len();
        SmartFillResult {
            fields_filled,
            fields_skipped,
            total_fields,
            filled_count,
            duration_ms: start.elapsed().as_millis() as u64,
            errors,
        }
    }

    /// Match a field label to a profile key using fuzzy matching
    fn match_label_to_profile_key(
        label: &str,
        profile: &serde_json::Map<String, serde_json::Value>,
        field: &FormField,
    ) -> Option<(String, String)> {
        // Common label → profile key mappings (label patterns to profile keys)
        let mappings: &[(&[&str], &str)] = &[
            (&["first name", "first_name", "firstname", "given name", "fname"], "firstName"),
            (&["last name", "last_name", "lastname", "surname", "family name", "lname"], "lastName"),
            (&["full name", "full_name", "fullname", "your name"], "fullName"),
            (&["email", "e-mail", "email address"], "email"),
            (&["phone", "telephone", "tel", "phone number", "mobile", "cell"], "phone"),
            (&["city", "town"], "city"),
            (&["state", "province", "region"], "state"),
            (&["zip", "zip code", "zipcode", "postal code", "postal", "postcode"], "zip"),
            (&["address", "street address", "street", "address line"], "address"),
            (&["years of experience", "years experience", "experience", "yrs experience", "years_experience"], "yearsExperience"),
            (&["salary", "expected salary", "desired salary", "compensation", "salary expectation"], "salary"),
            (&["linkedin", "linkedin url", "linkedin profile"], "linkedIn"),
            (&["github", "github url", "github profile"], "github"),
            (&["current title", "job title", "title", "position", "role"], "currentTitle"),
            (&["current company", "company", "employer", "organization", "company name"], "currentCompany"),
            (&["education", "degree", "school", "university", "highest education"], "education"),
            (&["authorized", "work authorization", "authorized to work", "legally authorized", "eligible to work"], "authorized"),
            (&["sponsorship", "visa sponsorship", "require sponsorship", "need sponsorship"], "sponsorship"),
            (&["start date", "available start", "earliest start", "availability", "date available"], "startDate"),
            (&["website", "portfolio", "personal website", "url"], "website"),
            (&["cover letter", "cover_letter"], "coverLetter"),
            (&["summary", "about", "bio", "about yourself"], "summary"),
            (&["country", "nationality"], "country"),
        ];

        // First try: direct profile key match against label
        for (key, value) in profile {
            let key_lower = key.to_lowercase();
            if label.contains(&key_lower) || key_lower.contains(label) {
                if let Some(s) = value.as_str() {
                    return Some((key.clone(), s.to_string()));
                }
                if let Some(n) = value.as_f64() {
                    return Some((key.clone(), n.to_string()));
                }
                if let Some(b) = value.as_bool() {
                    return Some((key.clone(), if b { "Yes" } else { "No" }.to_string()));
                }
            }
        }

        // Second try: fuzzy match via mapping table
        for (patterns, profile_key) in mappings {
            let matches = patterns.iter().any(|p| {
                label.contains(p) || p.contains(label)
            });

            if matches {
                if let Some(value) = profile.get(*profile_key) {
                    if let Some(s) = value.as_str() {
                        return Some((profile_key.to_string(), s.to_string()));
                    }
                    if let Some(n) = value.as_f64() {
                        return Some((profile_key.to_string(), n.to_string()));
                    }
                    if let Some(b) = value.as_bool() {
                        return Some((
                            profile_key.to_string(),
                            if b { "Yes" } else { "No" }.to_string(),
                        ));
                    }
                }
            }
        }

        // Third try: match by field name/id attribute against profile keys
        let alt_label = field.name.as_deref().or(field.id.as_deref()).unwrap_or("");
        let alt_lower = alt_label.to_lowercase();
        if !alt_lower.is_empty() && alt_lower != label {
            for (key, value) in profile {
                let key_lower = key.to_lowercase();
                if alt_lower.contains(&key_lower) || key_lower.contains(&alt_lower) {
                    if let Some(s) = value.as_str() {
                        return Some((key.clone(), s.to_string()));
                    }
                    if let Some(n) = value.as_f64() {
                        return Some((key.clone(), n.to_string()));
                    }
                    if let Some(b) = value.as_bool() {
                        return Some((key.clone(), if b { "Yes" } else { "No" }.to_string()));
                    }
                }
            }
        }

        None
    }

    /// Smart fill a select element by finding the best matching option
    async fn smart_fill_select(
        page: &Arc<PageHandle>,
        tivana_id: &str,
        value: &str,
        field: &FormField,
    ) -> Result<ActionResult, TivanaError> {
        let value_lower = value.to_lowercase();

        // Find the best matching option
        if let Some(ref options) = field.options {
            let best = options.iter().find(|o| {
                o.value.to_lowercase() == value_lower || o.text.to_lowercase() == value_lower
            }).or_else(|| {
                // Fuzzy: contains match
                options.iter().find(|o| {
                    o.text.to_lowercase().contains(&value_lower)
                        || value_lower.contains(&o.text.to_lowercase())
                        || o.value.to_lowercase().contains(&value_lower)
                })
            });

            if let Some(option) = best {
                let script = format!(
                    r#"(() => {{
                        const el = document.querySelector('[data-tivana-id="{}"]');
                        if (!el) return false;
                        el.value = {};
                        el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        return true;
                    }})()"#,
                    tivana_id,
                    serde_json::to_string(&option.value).unwrap_or_default()
                );
                let _: bool = page.evaluate(&script).await?;
                return Ok(ActionResult::success());
            }
        }

        Err(TivanaError::Browser(format!(
            "No matching option for value: {}",
            value
        )))
    }

    /// Smart fill a radio button group based on yes/no value
    async fn smart_fill_radio(
        page: &Arc<PageHandle>,
        tivana_id: &str,
        value: &str,
        _field: &FormField,
    ) -> Result<ActionResult, TivanaError> {
        let value_lower = value.to_lowercase();
        let is_affirmative = matches!(
            value_lower.as_str(),
            "yes" | "true" | "1" | "y"
        );

        // Try to click the right radio in the group
        let script = format!(
            r#"(() => {{
                const el = document.querySelector('[data-tivana-id="{}"]');
                if (!el) return 'not_found';
                const name = el.name;
                if (!name) {{ el.click(); return 'clicked'; }}

                // Find all radios in this group
                const radios = document.querySelectorAll('input[type="radio"][name="' + CSS.escape(name) + '"]');
                for (const radio of radios) {{
                    const label = radio.closest('label')?.textContent?.trim()?.toLowerCase() || '';
                    const val = radio.value?.toLowerCase() || '';
                    const affirmative = label.includes('yes') || val === 'yes' || val === 'true' || val === '1';
                    const negative = label.includes('no') || val === 'no' || val === 'false' || val === '0';

                    if (({is_aff} && affirmative) || (!{is_aff} && negative)) {{
                        radio.click();
                        return 'clicked';
                    }}
                }}

                // Fallback: click based on value match
                for (const radio of radios) {{
                    if (radio.value?.toLowerCase() === {val}) {{
                        radio.click();
                        return 'clicked';
                    }}
                }}

                return 'no_match';
            }})()"#,
            tivana_id,
            is_aff = is_affirmative,
            val = serde_json::to_string(&value_lower).unwrap_or_default()
        );

        let result: String = page.evaluate(&script).await?;
        if result == "not_found" {
            return Err(TivanaError::Browser(format!(
                "Radio element not found: {}",
                tivana_id
            )));
        }

        Ok(ActionResult::success())
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
