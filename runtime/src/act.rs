//! Action methods for interacting with page elements
//!
//! This module provides methods for performing actions on the browser page
//! such as clicking, typing, and scrolling. Currently a stub for Phase 1.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::browser::BrowserHandle;
use crate::error::TivanaError;

/// Result of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,

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
            data: None,
            duration_ms: 0,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Target selector for actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionTarget {
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
    pub fn selector(selector: impl Into<String>) -> Self {
        Self {
            selector: Some(selector.into()),
            text: None,
            role: None,
            label: None,
            coordinates: None,
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            selector: None,
            text: Some(text.into()),
            role: None,
            label: None,
            coordinates: None,
        }
    }

    pub fn coordinates(x: f64, y: f64) -> Self {
        Self {
            selector: None,
            text: None,
            role: None,
            label: None,
            coordinates: Some((x, y)),
        }
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

/// Action executor
pub struct Actor;

impl Actor {
    /// Click on a target element
    pub async fn click(
        _browser: &BrowserHandle,
        target: ActionTarget,
        options: ClickOptions,
    ) -> Result<ActionResult, TivanaError> {
        info!(?target, ?options, "Click (stub)");
        // TODO: Implement actual click in Phase 2
        Ok(ActionResult::success())
    }

    /// Type text into a target element
    pub async fn type_text(
        _browser: &BrowserHandle,
        target: ActionTarget,
        text: &str,
        options: TypeOptions,
    ) -> Result<ActionResult, TivanaError> {
        info!(?target, text_len = text.len(), ?options, "Type (stub)");
        // TODO: Implement actual typing in Phase 2
        Ok(ActionResult::success())
    }

    /// Press a key or key combination
    pub async fn press(
        _browser: &BrowserHandle,
        key: &str,
        modifiers: Vec<String>,
    ) -> Result<ActionResult, TivanaError> {
        info!(key, ?modifiers, "Press (stub)");
        // TODO: Implement actual key press in Phase 2
        Ok(ActionResult::success())
    }

    /// Scroll the page or element
    pub async fn scroll(
        _browser: &BrowserHandle,
        target: Option<ActionTarget>,
        options: ScrollOptions,
    ) -> Result<ActionResult, TivanaError> {
        info!(?target, ?options, "Scroll (stub)");
        // TODO: Implement actual scroll in Phase 2
        Ok(ActionResult::success())
    }

    /// Hover over a target element
    pub async fn hover(
        _browser: &BrowserHandle,
        target: ActionTarget,
    ) -> Result<ActionResult, TivanaError> {
        debug!(?target, "Hover (stub)");
        // TODO: Implement actual hover in Phase 2
        Ok(ActionResult::success())
    }

    /// Focus a target element
    pub async fn focus(
        _browser: &BrowserHandle,
        target: ActionTarget,
    ) -> Result<ActionResult, TivanaError> {
        debug!(?target, "Focus (stub)");
        // TODO: Implement actual focus in Phase 2
        Ok(ActionResult::success())
    }

    /// Select an option from a dropdown
    pub async fn select(
        _browser: &BrowserHandle,
        target: ActionTarget,
        value: &str,
    ) -> Result<ActionResult, TivanaError> {
        info!(?target, value, "Select (stub)");
        // TODO: Implement actual select in Phase 2
        Ok(ActionResult::success())
    }

    /// Upload a file
    pub async fn upload(
        _browser: &BrowserHandle,
        target: ActionTarget,
        file_path: &str,
    ) -> Result<ActionResult, TivanaError> {
        info!(?target, file_path, "Upload (stub)");
        // TODO: Implement actual upload in Phase 2
        Ok(ActionResult::success())
    }

    /// Wait for a condition
    pub async fn wait_for(
        _browser: &BrowserHandle,
        condition: WaitCondition,
        timeout_ms: u64,
    ) -> Result<ActionResult, TivanaError> {
        debug!(?condition, timeout_ms, "WaitFor (stub)");
        // TODO: Implement actual wait in Phase 2
        Ok(ActionResult::success())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
