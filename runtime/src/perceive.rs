//! Perception methods for reading page state
//!
//! This module provides methods for extracting structured information
//! from the browser page. Currently a stub for Phase 1.

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::browser::BrowserHandle;
use crate::error::TivanaError;

/// Snapshot of page accessibility tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilitySnapshot {
    /// Root node of the accessibility tree
    pub root: Option<AccessibilityNode>,

    /// Snapshot timestamp
    pub timestamp_ms: u64,
}

/// Single node in the accessibility tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilityNode {
    /// Role (e.g., "button", "link", "textbox")
    pub role: String,

    /// Accessible name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Accessible description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Value (for inputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Bounding box
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<BoundingBox>,

    /// Child nodes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AccessibilityNode>,

    /// Whether element is focusable
    #[serde(default)]
    pub focusable: bool,

    /// Whether element is focused
    #[serde(default)]
    pub focused: bool,

    /// Whether element is disabled
    #[serde(default)]
    pub disabled: bool,
}

/// Bounding box coordinates
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Page text content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// Raw text content
    pub text: String,

    /// Word count
    pub word_count: usize,

    /// Character count
    pub char_count: usize,
}

/// Page metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMetadata {
    /// Page URL
    pub url: String,

    /// Page title
    pub title: Option<String>,

    /// Meta description
    pub description: Option<String>,

    /// Favicon URL
    pub favicon: Option<String>,
}

/// Perception methods
pub struct Perceiver;

impl Perceiver {
    /// Get accessibility tree snapshot
    pub async fn accessibility_snapshot(
        _browser: &BrowserHandle,
    ) -> Result<AccessibilitySnapshot, TivanaError> {
        debug!("Getting accessibility snapshot (stub)");
        // TODO: Implement actual accessibility tree extraction in Phase 2
        Ok(AccessibilitySnapshot {
            root: Some(AccessibilityNode {
                role: "document".to_string(),
                name: Some("Stub Document".to_string()),
                description: None,
                value: None,
                bounds: Some(BoundingBox {
                    x: 0.0,
                    y: 0.0,
                    width: 1280.0,
                    height: 720.0,
                }),
                children: vec![],
                focusable: false,
                focused: false,
                disabled: false,
            }),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        })
    }

    /// Get page text content
    pub async fn text_content(_browser: &BrowserHandle) -> Result<TextContent, TivanaError> {
        debug!("Getting text content (stub)");
        // TODO: Implement in Phase 2
        Ok(TextContent {
            text: String::new(),
            word_count: 0,
            char_count: 0,
        })
    }

    /// Get page metadata
    pub async fn metadata(_browser: &BrowserHandle) -> Result<PageMetadata, TivanaError> {
        debug!("Getting page metadata (stub)");
        // TODO: Implement in Phase 2
        Ok(PageMetadata {
            url: "about:blank".to_string(),
            title: None,
            description: None,
            favicon: None,
        })
    }

    /// Find elements matching a selector
    pub async fn find_elements(
        _browser: &BrowserHandle,
        _selector: &str,
    ) -> Result<Vec<ElementInfo>, TivanaError> {
        debug!("Finding elements (stub)");
        // TODO: Implement in Phase 2
        Ok(vec![])
    }
}

/// Information about a found element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementInfo {
    /// CSS selector path
    pub selector: String,

    /// Element tag name
    pub tag_name: String,

    /// Element text content
    pub text: Option<String>,

    /// Element attributes
    pub attributes: std::collections::HashMap<String, String>,

    /// Bounding box
    pub bounds: Option<BoundingBox>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.width, 100.0);
    }

    #[test]
    fn test_accessibility_node_serialization() {
        let node = AccessibilityNode {
            role: "button".to_string(),
            name: Some("Submit".to_string()),
            description: None,
            value: None,
            bounds: None,
            children: vec![],
            focusable: true,
            focused: false,
            disabled: false,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("button"));
        assert!(json.contains("Submit"));
    }
}
