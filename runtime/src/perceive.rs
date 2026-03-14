//! Perception methods for reading page state
//!
//! This module provides methods for extracting structured information
//! from the browser page using CDP commands and JavaScript evaluation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

use crate::browser::PageHandle;
use crate::error::TivanaError;

/// Current page state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageState {
    /// Current URL
    pub url: String,

    /// Page title
    pub title: Option<String>,

    /// Focused element ID (if any)
    pub focused_element_id: Option<String>,

    /// Scroll position
    pub scroll_x: f64,
    pub scroll_y: f64,

    /// Viewport dimensions
    pub viewport_width: f64,
    pub viewport_height: f64,

    /// Document dimensions
    pub document_width: f64,
    pub document_height: f64,

    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Element information with stable ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    /// Stable element ID (e1, e2, etc.)
    pub id: String,

    /// Accessibility role
    pub role: String,

    /// Accessible name/label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Current value (for inputs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Accessible description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Bounding box
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<BoundingBox>,

    /// Computed styles (subset)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub styles: Option<ElementStyles>,

    /// State flags
    pub focused: bool,
    pub enabled: bool,
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub required: Option<bool>,

    /// Child elements
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Element>,
}

/// Bounding box coordinates
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    /// Get center point
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Check if point is inside bounds
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

/// Subset of computed styles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementStyles {
    /// Font family
    pub font_family: Option<String>,
    /// Font size
    pub font_size: Option<String>,
    /// Font weight
    pub font_weight: Option<String>,
    /// Text color
    pub color: Option<String>,
    /// Background color
    pub background_color: Option<String>,
    /// Border style
    pub border: Option<String>,
    /// Display type
    pub display: Option<String>,
    /// Visibility
    pub visibility: Option<String>,
}

/// Snapshot of page accessibility tree
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessibilitySnapshot {
    /// Root element
    pub root: Option<Element>,

    /// Flat list of all interactive elements
    pub interactive_elements: Vec<Element>,

    /// Snapshot timestamp
    pub timestamp_ms: u64,
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

    /// Open Graph image
    pub og_image: Option<String>,

    /// Language
    pub language: Option<String>,
}

/// Mutation event for DOM changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum MutationEvent {
    /// Element was added
    Added {
        element_id: String,
        parent_id: Option<String>,
    },
    /// Element was removed
    Removed { element_id: String },
    /// Element attributes changed
    Changed {
        element_id: String,
        attribute: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    /// Text content changed
    TextChanged {
        element_id: String,
        text: String,
    },
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
    pub attributes: HashMap<String, String>,

    /// Bounding box
    pub bounds: Option<BoundingBox>,
}

/// Perception methods
pub struct Perceiver;

impl Perceiver {
    /// Get current page state
    pub async fn page_state(page: &Arc<PageHandle>) -> Result<PageState, TivanaError> {
        debug!("Getting page state");

        let url = page.url().await?;
        let title = page.title().await?;

        // Get scroll position and viewport info via JS
        #[derive(Deserialize)]
        struct ViewportInfo {
            scroll_x: f64,
            scroll_y: f64,
            viewport_width: f64,
            viewport_height: f64,
            document_width: f64,
            document_height: f64,
            focused_element_id: Option<String>,
        }

        let viewport_info: ViewportInfo = page
            .evaluate(
                r#"(() => {
                const focused = document.activeElement;
                return {
                    scroll_x: window.scrollX || window.pageXOffset || 0,
                    scroll_y: window.scrollY || window.pageYOffset || 0,
                    viewport_width: window.innerWidth || document.documentElement.clientWidth || 0,
                    viewport_height: window.innerHeight || document.documentElement.clientHeight || 0,
                    document_width: Math.max(
                        document.body?.scrollWidth || 0,
                        document.documentElement?.scrollWidth || 0
                    ),
                    document_height: Math.max(
                        document.body?.scrollHeight || 0,
                        document.documentElement?.scrollHeight || 0
                    ),
                    focused_element_id: focused && focused !== document.body ? focused.id || null : null
                };
            })()"#,
            )
            .await?;

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(PageState {
            url,
            title,
            focused_element_id: viewport_info.focused_element_id,
            scroll_x: viewport_info.scroll_x,
            scroll_y: viewport_info.scroll_y,
            viewport_width: viewport_info.viewport_width,
            viewport_height: viewport_info.viewport_height,
            document_width: viewport_info.document_width,
            document_height: viewport_info.document_height,
            timestamp_ms,
        })
    }

    /// Get interactive elements on the page
    pub async fn elements(page: &Arc<PageHandle>) -> Result<Vec<Element>, TivanaError> {
        debug!("Getting page elements");

        // JavaScript to extract interactive elements
        let script = r#"(() => {
            const elements = [];
            let elementCounter = 1;
            
            // Interactive element selectors
            const selector = [
                'a[href]',
                'button',
                'input',
                'select',
                'textarea',
                '[role="button"]',
                '[role="link"]',
                '[role="checkbox"]',
                '[role="radio"]',
                '[role="menuitem"]',
                '[role="tab"]',
                '[role="option"]',
                '[role="switch"]',
                '[role="slider"]',
                '[role="spinbutton"]',
                '[role="searchbox"]',
                '[role="textbox"]',
                '[role="combobox"]',
                '[tabindex]:not([tabindex="-1"])',
                '[contenteditable="true"]'
            ].join(', ');
            
            const interactiveElements = document.querySelectorAll(selector);
            
            for (const el of interactiveElements) {
                // Skip hidden elements
                const style = window.getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') {
                    continue;
                }
                
                const rect = el.getBoundingClientRect();
                if (rect.width === 0 && rect.height === 0) {
                    continue;
                }
                
                // Determine role
                let role = el.getAttribute('role') || el.tagName.toLowerCase();
                if (role === 'input') {
                    role = el.type || 'text';
                }
                
                // Get accessible name
                let name = el.getAttribute('aria-label') ||
                           el.getAttribute('aria-labelledby') && document.getElementById(el.getAttribute('aria-labelledby'))?.textContent ||
                           el.getAttribute('title') ||
                           el.getAttribute('placeholder') ||
                           el.innerText?.trim()?.slice(0, 100) ||
                           el.value ||
                           null;
                
                // Get value for inputs
                let value = null;
                if (el.value !== undefined && el.value !== '') {
                    value = el.value;
                }
                
                // Check states
                const focused = document.activeElement === el;
                const enabled = !el.disabled;
                const checked = el.type === 'checkbox' || el.type === 'radio' ? el.checked : undefined;
                const selected = el.selected !== undefined ? el.selected : undefined;
                const expanded = el.getAttribute('aria-expanded') ? el.getAttribute('aria-expanded') === 'true' : undefined;
                const required = el.required !== undefined ? el.required : undefined;
                
                // Get styles
                const styles = {
                    font_family: style.fontFamily,
                    font_size: style.fontSize,
                    font_weight: style.fontWeight,
                    color: style.color,
                    background_color: style.backgroundColor,
                    border: style.border,
                    display: style.display,
                    visibility: style.visibility
                };
                
                elements.push({
                    id: 'e' + (elementCounter++),
                    role: role,
                    name: name,
                    value: value,
                    description: el.getAttribute('aria-description') || null,
                    bounds: {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: rect.height
                    },
                    styles: styles,
                    focused: focused,
                    enabled: enabled,
                    checked: checked,
                    selected: selected,
                    expanded: expanded,
                    required: required,
                    children: []
                });
            }
            
            return elements;
        })()"#;

        let elements: Vec<Element> = page.evaluate(script).await?;
        debug!(count = elements.len(), "Found interactive elements");

        Ok(elements)
    }

    /// Get full accessibility tree snapshot
    pub async fn accessibility_snapshot(
        page: &Arc<PageHandle>,
    ) -> Result<AccessibilitySnapshot, TivanaError> {
        debug!("Getting accessibility snapshot");

        let elements = Self::elements(page).await?;

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Create root document element
        let root = Element {
            id: "e0".to_string(),
            role: "document".to_string(),
            name: page.title().await?,
            value: None,
            description: None,
            bounds: None,
            styles: None,
            focused: false,
            enabled: true,
            checked: None,
            selected: None,
            expanded: None,
            required: None,
            children: vec![],
        };

        Ok(AccessibilitySnapshot {
            root: Some(root),
            interactive_elements: elements,
            timestamp_ms,
        })
    }

    /// Get page text content
    pub async fn text_content(page: &Arc<PageHandle>) -> Result<TextContent, TivanaError> {
        debug!("Getting text content");

        let text: String = page
            .evaluate("document.body?.innerText || ''")
            .await?;

        let word_count = text.split_whitespace().count();
        let char_count = text.chars().count();

        Ok(TextContent {
            text,
            word_count,
            char_count,
        })
    }

    /// Get page metadata
    pub async fn metadata(page: &Arc<PageHandle>) -> Result<PageMetadata, TivanaError> {
        debug!("Getting page metadata");

        #[derive(Deserialize)]
        struct MetaInfo {
            url: String,
            title: Option<String>,
            description: Option<String>,
            favicon: Option<String>,
            og_image: Option<String>,
            language: Option<String>,
        }

        let meta: MetaInfo = page
            .evaluate(
                r#"(() => {
                const getMeta = (name) => {
                    const el = document.querySelector(`meta[name="${name}"], meta[property="${name}"]`);
                    return el?.content || null;
                };
                
                const getLink = (rel) => {
                    const el = document.querySelector(`link[rel="${rel}"], link[rel="shortcut icon"]`);
                    return el?.href || null;
                };
                
                return {
                    url: window.location.href,
                    title: document.title || null,
                    description: getMeta('description') || getMeta('og:description'),
                    favicon: getLink('icon') || getLink('shortcut icon'),
                    og_image: getMeta('og:image'),
                    language: document.documentElement.lang || null
                };
            })()"#,
            )
            .await?;

        Ok(PageMetadata {
            url: meta.url,
            title: meta.title,
            description: meta.description,
            favicon: meta.favicon,
            og_image: meta.og_image,
            language: meta.language,
        })
    }

    /// Find elements matching a selector
    pub async fn find_elements(
        page: &Arc<PageHandle>,
        selector: &str,
    ) -> Result<Vec<ElementInfo>, TivanaError> {
        debug!(selector, "Finding elements");

        let script = format!(
            r#"(() => {{
                const elements = document.querySelectorAll({});
                return Array.from(elements).map((el, i) => {{
                    const rect = el.getBoundingClientRect();
                    const attrs = {{}};
                    for (const attr of el.attributes) {{
                        attrs[attr.name] = attr.value;
                    }}
                    return {{
                        selector: {0},
                        tag_name: el.tagName.toLowerCase(),
                        text: el.innerText?.trim()?.slice(0, 200) || null,
                        attributes: attrs,
                        bounds: rect.width > 0 ? {{
                            x: rect.x,
                            y: rect.y,
                            width: rect.width,
                            height: rect.height
                        }} : null
                    }};
                }});
            }})()"#,
            serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string())
        );

        let elements: Vec<ElementInfo> = page.evaluate(&script).await?;
        debug!(count = elements.len(), "Found elements");

        Ok(elements)
    }

    /// Find element by role and name (for Tivana-style targeting)
    pub async fn find_by_role_and_name(
        page: &Arc<PageHandle>,
        role: &str,
        name: Option<&str>,
    ) -> Result<Option<(BoundingBox, String)>, TivanaError> {
        debug!(role, ?name, "Finding element by role and name");

        let name_match = name
            .map(|n| format!(r#"&& (name?.toLowerCase().includes({}.toLowerCase()) || el.innerText?.toLowerCase().includes({}.toLowerCase()))"#, 
                serde_json::to_string(n).unwrap_or_default(),
                serde_json::to_string(n).unwrap_or_default()))
            .unwrap_or_default();

        let script = format!(
            r#"(() => {{
                const elements = document.querySelectorAll('*');
                for (const el of elements) {{
                    const elRole = el.getAttribute('role') || el.tagName.toLowerCase();
                    const name = el.getAttribute('aria-label') || el.innerText?.trim();
                    
                    const targetRole = {role_json}.toLowerCase();
                    const matchRole = elRole.toLowerCase() === targetRole ||
                        (targetRole === 'button' && (elRole === 'button' || el.tagName === 'BUTTON')) ||
                        (targetRole === 'link' && (elRole === 'link' || el.tagName === 'A')) ||
                        (targetRole === 'textbox' && (elRole === 'textbox' || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA'));
                    
                    if (matchRole {name_match}) {{
                        const rect = el.getBoundingClientRect();
                        if (rect.width > 0 && rect.height > 0) {{
                            const style = window.getComputedStyle(el);
                            if (style.display !== 'none' && style.visibility !== 'hidden') {{
                                return {{
                                    bounds: {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }},
                                    selector: el.id ? '#' + el.id : el.tagName.toLowerCase()
                                }};
                            }}
                        }}
                    }}
                }}
                return null;
            }})()"#,
            role_json = serde_json::to_string(role).unwrap_or_default(),
            name_match = name_match
        );

        #[derive(Deserialize)]
        struct FindResult {
            bounds: BoundingBox,
            selector: String,
        }

        let result: Option<FindResult> = page.evaluate(&script).await?;
        Ok(result.map(|r| (r.bounds, r.selector)))
    }

    /// Resolve an element ID to bounding box
    pub async fn resolve_element_bounds(
        page: &Arc<PageHandle>,
        element_id: &str,
    ) -> Result<Option<BoundingBox>, TivanaError> {
        debug!(element_id, "Resolving element bounds");

        // Element IDs are like e1, e2, etc. - need to re-query
        // Since we generate these dynamically, we need to re-enumerate and find
        let elements = Self::elements(page).await?;

        Ok(elements
            .into_iter()
            .find(|e| e.id == element_id)
            .and_then(|e| e.bounds))
    }
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
        assert_eq!(bbox.center(), (60.0, 45.0));
        assert!(bbox.contains(50.0, 40.0));
        assert!(!bbox.contains(0.0, 0.0));
    }

    #[test]
    fn test_element_serialization() {
        let element = Element {
            id: "e1".to_string(),
            role: "button".to_string(),
            name: Some("Submit".to_string()),
            value: None,
            description: None,
            bounds: Some(BoundingBox {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 40.0,
            }),
            styles: None,
            focused: false,
            enabled: true,
            checked: None,
            selected: None,
            expanded: None,
            required: None,
            children: vec![],
        };

        let json = serde_json::to_string(&element).unwrap();
        assert!(json.contains("button"));
        assert!(json.contains("Submit"));
    }
}
