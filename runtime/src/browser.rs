//! Chromium browser management
//!
//! This module handles launching and controlling Chromium instances
//! using the chromiumoxide library for CDP communication.

use std::path::PathBuf;
use std::sync::Arc;

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::page::Page;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::error::TivanaError;

/// Browser launch configuration
#[derive(Debug, Clone)]
pub struct BrowserLaunchConfig {
    /// Run in headless mode
    pub headless: bool,

    /// Path to Chrome/Chromium executable
    pub chrome_path: Option<PathBuf>,

    /// Initial viewport width
    pub viewport_width: u32,

    /// Initial viewport height
    pub viewport_height: u32,

    /// User data directory
    pub user_data_dir: Option<PathBuf>,

    /// Additional Chrome arguments
    pub args: Vec<String>,
}

impl Default for BrowserLaunchConfig {
    fn default() -> Self {
        Self {
            headless: true,
            chrome_path: None,
            viewport_width: 1280,
            viewport_height: 720,
            user_data_dir: None,
            args: vec![],
        }
    }
}

/// Wrapper around chromiumoxide Page with additional state
#[derive(Debug)]
pub struct PageHandle {
    /// The underlying chromiumoxide page
    page: Page,

    /// Page ID for tracking
    pub id: String,
}

impl PageHandle {
    /// Create a new page handle
    pub fn new(page: Page) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self { page, id }
    }

    /// Get the underlying page reference
    pub fn inner(&self) -> &Page {
        &self.page
    }

    /// Get the current URL
    pub async fn url(&self) -> Result<String, TivanaError> {
        // Use evaluate to get the current URL reliably
        let url: String = self
            .page
            .evaluate("window.location.href")
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to get URL: {}", e)))?
            .into_value()
            .map_err(|e| TivanaError::Browser(format!("Failed to parse URL: {:?}", e)))?;
        Ok(url)
    }

    /// Get the page title
    pub async fn title(&self) -> Result<Option<String>, TivanaError> {
        let title: String = self
            .page
            .evaluate("document.title")
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to get title: {}", e)))?
            .into_value()
            .map_err(|e| TivanaError::Browser(format!("Failed to parse title: {:?}", e)))?;
        Ok(if title.is_empty() { None } else { Some(title) })
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<NavigationResult, TivanaError> {
        let start = std::time::Instant::now();

        self.page
            .goto(url)
            .await
            .map_err(|e| TivanaError::Browser(format!("Navigation failed: {}", e)))?;

        // Wait for page to be ready
        self.page
            .wait_for_navigation()
            .await
            .map_err(|e| TivanaError::Browser(format!("Wait for navigation failed: {}", e)))?;

        let load_time_ms = start.elapsed().as_millis() as u64;
        let final_url = self.url().await?;
        let title = self.title().await?;

        Ok(NavigationResult {
            url: final_url,
            title,
            load_time_ms,
        })
    }

    /// Execute JavaScript and return result
    pub async fn evaluate<T>(&self, script: &str) -> Result<T, TivanaError>
    where
        T: serde::de::DeserializeOwned,
    {
        let result = self
            .page
            .evaluate(script)
            .await
            .map_err(|e| TivanaError::Browser(format!("Evaluate failed: {}", e)))?;

        result
            .into_value()
            .map_err(|e| TivanaError::Browser(format!("Failed to parse result: {:?}", e)))
    }

    /// Execute JavaScript without expecting a return value
    pub async fn evaluate_void(&self, script: &str) -> Result<(), TivanaError> {
        self.page
            .evaluate(script)
            .await
            .map_err(|e| TivanaError::Browser(format!("Evaluate failed: {}", e)))?;
        Ok(())
    }

    /// Click at specific coordinates using JavaScript
    pub async fn click_at(&self, x: f64, y: f64) -> Result<(), TivanaError> {
        // Use JavaScript to dispatch click events at coordinates
        let script = format!(
            r#"(() => {{
                const el = document.elementFromPoint({}, {});
                if (el) {{
                    const events = ['mousedown', 'mouseup', 'click'];
                    for (const eventType of events) {{
                        const event = new MouseEvent(eventType, {{
                            view: window,
                            bubbles: true,
                            cancelable: true,
                            clientX: {},
                            clientY: {}
                        }});
                        el.dispatchEvent(event);
                    }}
                }}
            }})()"#,
            x, y, x, y
        );
        self.evaluate_void(&script).await
    }

    /// Type text by directly manipulating the active element
    pub async fn type_text(&self, text: &str) -> Result<(), TivanaError> {
        // Type by setting value or insertText depending on element type
        let script = format!(
            r#"(() => {{
                const text = {};
                const el = document.activeElement;
                if (!el) return false;
                
                if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                    // For form elements, set value and trigger events
                    const start = el.selectionStart || 0;
                    const end = el.selectionEnd || 0;
                    const before = el.value.substring(0, start);
                    const after = el.value.substring(end);
                    el.value = before + text + after;
                    el.selectionStart = el.selectionEnd = start + text.length;
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return true;
                }} else if (el.isContentEditable) {{
                    // For contenteditable, use execCommand
                    document.execCommand('insertText', false, text);
                    return true;
                }}
                return false;
            }})()"#,
            serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
        );
        
        let success: bool = self.evaluate(&script).await?;
        if !success {
            return Err(TivanaError::Browser("No active element to type into".to_string()));
        }
        Ok(())
    }

    /// Press a key using JavaScript keyboard events
    pub async fn press_key(&self, key: &str) -> Result<(), TivanaError> {
        // Dispatch keyboard event via JavaScript
        let script = format!(
            r#"(() => {{
                const key = {};
                const events = ['keydown', 'keypress', 'keyup'];
                for (const eventType of events) {{
                    const event = new KeyboardEvent(eventType, {{
                        key: key,
                        code: key,
                        bubbles: true,
                        cancelable: true
                    }});
                    document.activeElement?.dispatchEvent(event) || document.dispatchEvent(event);
                }}
            }})()"#,
            serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string())
        );
        self.evaluate_void(&script).await
    }

    /// Close the page
    pub async fn close(self) -> Result<(), TivanaError> {
        self.page
            .close()
            .await
            .map_err(|e| TivanaError::Browser(format!("Close page failed: {}", e)))?;
        Ok(())
    }
}

/// Handle to a running browser instance
#[derive(Debug)]
pub struct BrowserHandle {
    /// Configuration used to launch
    pub config: BrowserLaunchConfig,

    /// The chromiumoxide browser instance
    browser: Arc<RwLock<Option<Browser>>>,

    /// Pages in this browser
    pages: Arc<RwLock<Vec<Arc<PageHandle>>>>,

    /// Browser handler task
    handler_task: Option<tokio::task::JoinHandle<()>>,
}

impl BrowserHandle {
    /// Create a new browser handle from a launched browser
    pub fn new(
        config: BrowserLaunchConfig,
        browser: Browser,
        handler: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            config,
            browser: Arc::new(RwLock::new(Some(browser))),
            pages: Arc::new(RwLock::new(Vec::new())),
            handler_task: Some(handler),
        }
    }

    /// Create a new page in this browser
    pub async fn new_page(&self) -> Result<Arc<PageHandle>, TivanaError> {
        let browser_guard = self.browser.read().await;
        let browser = browser_guard
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Browser is closed".to_string()))?;

        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to create page: {}", e)))?;

        let handle = Arc::new(PageHandle::new(page));
        self.pages.write().await.push(Arc::clone(&handle));

        info!(page_id = %handle.id, "Created new page");
        Ok(handle)
    }

    /// Get the first/default page
    pub async fn default_page(&self) -> Result<Arc<PageHandle>, TivanaError> {
        let pages = self.pages.read().await;
        pages
            .first()
            .cloned()
            .ok_or_else(|| TivanaError::Browser("No pages available".to_string()))
    }

    /// Get a page by ID
    pub async fn get_page(&self, id: &str) -> Option<Arc<PageHandle>> {
        let pages = self.pages.read().await;
        pages.iter().find(|p| p.id == id).cloned()
    }

    /// Close a specific page
    pub async fn close_page(&self, page_id: &str) -> Result<(), TivanaError> {
        let mut pages = self.pages.write().await;
        if let Some(pos) = pages.iter().position(|p| p.id == page_id) {
            let _page = pages.remove(pos);
            // Note: Can't call close() without owning the Arc
            // The page will be dropped when the Arc count reaches 0
            debug!(page_id = %page_id, "Page removed from tracking");
        }
        Ok(())
    }

    /// Close the browser and all pages
    pub async fn close(self) -> Result<(), TivanaError> {
        info!("Closing browser");

        // Clear pages
        self.pages.write().await.clear();

        // Take ownership of browser and close it
        let mut browser_guard = self.browser.write().await;
        if let Some(browser) = browser_guard.take() {
            // Browser will be dropped, which closes the connection
            drop(browser);
        }

        // Abort handler task if still running
        if let Some(handler) = self.handler_task {
            handler.abort();
        }

        Ok(())
    }
}

/// Result of a navigation action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationResult {
    /// Final URL after navigation (may differ due to redirects)
    pub url: String,

    /// Page title
    pub title: Option<String>,

    /// Time to load in milliseconds
    pub load_time_ms: u64,
}

/// Browser manager for launching and tracking browser instances
#[derive(Debug, Clone)]
pub struct BrowserManager {
    /// Default configuration for new browsers
    default_config: BrowserLaunchConfig,
}

impl BrowserManager {
    /// Create a new browser manager
    pub fn new(config: BrowserLaunchConfig) -> Self {
        Self {
            default_config: config,
        }
    }

    /// Launch a new browser instance
    pub async fn launch(
        &self,
        config: Option<BrowserLaunchConfig>,
    ) -> Result<BrowserHandle, TivanaError> {
        let config = config.unwrap_or_else(|| self.default_config.clone());

        info!(
            headless = config.headless,
            chrome_path = ?config.chrome_path,
            viewport = format!("{}x{}", config.viewport_width, config.viewport_height),
            "Launching browser"
        );

        // Build browser config
        let mut builder = BrowserConfig::builder();

        // NOTE: chromiumoxide's with_head() means "show the window" (headed mode)
        // So we only call it when headless=false
        if !config.headless {
            builder = builder.with_head();
        }

        // Set viewport
        builder = builder.window_size(config.viewport_width, config.viewport_height);

        // Set chrome path if specified
        if let Some(ref path) = config.chrome_path {
            builder = builder.chrome_executable(path);
        }

        // Set user data dir if specified
        if let Some(ref dir) = config.user_data_dir {
            builder = builder.user_data_dir(dir);
        }

        // Add common args for stability
        builder = builder
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-networking")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            .arg("--disable-translate");

        // Add custom args
        for arg in &config.args {
            builder = builder.arg(arg);
        }

        let browser_config = builder
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build browser config: {}", e)))?;

        // Launch browser
        let (browser, mut handler) = Browser::launch(browser_config)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to launch browser: {}", e)))?;

        // Spawn handler task
        let handler_task = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                debug!(?event, "Browser event");
            }
        });

        let handle = BrowserHandle::new(config, browser, handler_task);

        // Create initial page
        handle.new_page().await?;

        info!("Browser launched successfully");
        Ok(handle)
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new(BrowserLaunchConfig::default())
    }
}

/// Viewport dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_default() {
        let config = BrowserLaunchConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1280);
        assert_eq!(config.viewport_height, 720);
    }
}
