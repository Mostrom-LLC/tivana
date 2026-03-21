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
            viewport_width: 1440,
            viewport_height: 900,
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

    /// Dispatch a mouseMoved CDP event to the given coordinates
    pub async fn move_mouse_to(&self, x: f64, y: f64) -> Result<(), TivanaError> {
        use chromiumoxide::cdp::browser_protocol::input::{
            DispatchMouseEventParams, DispatchMouseEventType,
        };

        let moved = DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MouseMoved)
            .x(x)
            .y(y)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build mouse event: {}", e)))?;

        self.page
            .execute(moved)
            .await
            .map_err(|e| TivanaError::Browser(format!("CDP mouseMoved failed: {}", e)))?;

        Ok(())
    }

    /// Click at specific coordinates using CDP Input.dispatchMouseEvent
    /// Includes a random human-like delay between mousedown and mouseup (50-150ms)
    pub async fn click_at(&self, x: f64, y: f64) -> Result<(), TivanaError> {
        use chromiumoxide::cdp::browser_protocol::input::{
            DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
        };
        use rand::Rng;

        // Mouse pressed
        let pressed = DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MousePressed)
            .x(x)
            .y(y)
            .button(MouseButton::Left)
            .click_count(1)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build mouse event: {}", e)))?;

        self.page
            .execute(pressed)
            .await
            .map_err(|e| TivanaError::Browser(format!("CDP mousePressed failed: {}", e)))?;

        // Human-like delay between mousedown and mouseup (50-150ms gaussian)
        let delay_ms = {
            let mut rng = rand::thread_rng();
            let base: f64 = rng.gen_range(50.0..150.0);
            base as u64
        };
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

        // Mouse released
        let released = DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MouseReleased)
            .x(x)
            .y(y)
            .button(MouseButton::Left)
            .click_count(1)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build mouse event: {}", e)))?;

        self.page
            .execute(released)
            .await
            .map_err(|e| TivanaError::Browser(format!("CDP mouseReleased failed: {}", e)))?;

        Ok(())
    }

    /// Type text using CDP Input.dispatchKeyEvent for each character
    ///
    /// Uses KeyDown (without text) + Char (with text) + KeyUp pattern.
    /// The Char event is what actually inserts the character; KeyDown/KeyUp
    /// handle modifier state and key identification only.
    pub async fn type_text(&self, text: &str) -> Result<(), TivanaError> {
        use chromiumoxide::cdp::browser_protocol::input::{
            DispatchKeyEventParams, DispatchKeyEventType,
        };

        for ch in text.chars() {
            let text_str = ch.to_string();

            // keyDown without text (just signals the key press, does not insert)
            let key_down = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::RawKeyDown)
                .key(text_str.clone())
                .build()
                .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

            self.page
                .execute(key_down)
                .await
                .map_err(|e| TivanaError::Browser(format!("CDP keyDown failed: {}", e)))?;

            // char event — this is what inserts the character
            let char_event = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::Char)
                .text(text_str.clone())
                .build()
                .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

            self.page
                .execute(char_event)
                .await
                .map_err(|e| TivanaError::Browser(format!("CDP char failed: {}", e)))?;

            // keyUp
            let key_up = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::KeyUp)
                .key(text_str)
                .build()
                .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

            self.page
                .execute(key_up)
                .await
                .map_err(|e| TivanaError::Browser(format!("CDP keyUp failed: {}", e)))?;
        }

        Ok(())
    }

    /// Press a key using CDP Input.dispatchKeyEvent with proper modifier support
    pub async fn press_key(&self, key: &str, modifiers: &[String]) -> Result<(), TivanaError> {
        use chromiumoxide::cdp::browser_protocol::input::{
            DispatchKeyEventParams, DispatchKeyEventType,
        };

        // Calculate modifier bitmask: Alt=1, Ctrl=2, Meta=4, Shift=8
        let modifier_flags: i64 = modifiers
            .iter()
            .map(|m| match m.to_lowercase().as_str() {
                "alt" => 1,
                "control" | "ctrl" => 2,
                "meta" | "command" | "cmd" => 4,
                "shift" => 8,
                _ => 0,
            })
            .sum();

        // Press modifier keys down
        for modifier in modifiers {
            let cmd = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::RawKeyDown)
                .key(modifier.clone())
                .modifiers(modifier_flags)
                .build()
                .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

            self.page
                .execute(cmd)
                .await
                .map_err(|e| TivanaError::Browser(format!("CDP modifier keyDown failed: {}", e)))?;
        }

        // Press the main key
        let key_down = DispatchKeyEventParams::builder()
            .r#type(DispatchKeyEventType::RawKeyDown)
            .key(key)
            .modifiers(modifier_flags)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

        self.page
            .execute(key_down)
            .await
            .map_err(|e| TivanaError::Browser(format!("CDP keyDown failed: {}", e)))?;

        let key_up = DispatchKeyEventParams::builder()
            .r#type(DispatchKeyEventType::KeyUp)
            .key(key)
            .modifiers(modifier_flags)
            .build()
            .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

        self.page
            .execute(key_up)
            .await
            .map_err(|e| TivanaError::Browser(format!("CDP keyUp failed: {}", e)))?;

        // Release modifier keys
        for modifier in modifiers.iter().rev() {
            let cmd = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::KeyUp)
                .key(modifier.clone())
                .build()
                .map_err(|e| TivanaError::Browser(format!("Failed to build key event: {}", e)))?;

            self.page
                .execute(cmd)
                .await
                .map_err(|e| TivanaError::Browser(format!("CDP modifier keyUp failed: {}", e)))?;
        }

        Ok(())
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

    /// Whether this browser was connected to externally (don't kill on close)
    pub is_external: bool,

    /// Last known mouse position for human-like movement interpolation
    pub last_mouse_position: Arc<RwLock<(f64, f64)>>,
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
            is_external: false,
            last_mouse_position: Arc::new(RwLock::new((0.0, 0.0))),
        }
    }

    /// Create a new browser handle for an externally-connected browser
    pub fn new_external(
        config: BrowserLaunchConfig,
        browser: Browser,
        handler: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            config,
            browser: Arc::new(RwLock::new(Some(browser))),
            pages: Arc::new(RwLock::new(Vec::new())),
            handler_task: Some(handler),
            is_external: true,
            last_mouse_position: Arc::new(RwLock::new((0.0, 0.0))),
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

    /// List all open tabs/pages in the browser via CDP
    pub async fn list_tabs(&self) -> Result<Vec<TabInfo>, TivanaError> {
        use chromiumoxide::cdp::browser_protocol::target::GetTargetsParams;

        let browser_guard = self.browser.read().await;
        let browser = browser_guard
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Browser is closed".to_string()))?;

        let resp = browser
            .execute(GetTargetsParams::builder().build())
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to get targets: {}", e)))?;

        let pages = self.pages.read().await;
        let active_page_id = pages.first().map(|p| p.id.clone());

        let mut tabs = Vec::new();
        for target in resp.target_infos.iter() {
            if target.r#type == "page" {
                let is_active = pages.iter().any(|p| {
                    // Match by checking if this is our tracked page
                    // The first page in our list is the "active" one
                    p.id == active_page_id.as_deref().unwrap_or("")
                });

                tabs.push(TabInfo {
                    target_id: target.target_id.inner().clone(),
                    url: target.url.clone(),
                    title: target.title.clone(),
                    active: is_active && tabs.is_empty(), // First match is active
                });
            }
        }

        Ok(tabs)
    }

    /// Switch to a different tab by target ID
    pub async fn switch_tab(&self, target_id: &str) -> Result<Arc<PageHandle>, TivanaError> {
        use chromiumoxide::cdp::browser_protocol::target::{ActivateTargetParams, TargetId};

        let browser_guard = self.browser.read().await;
        let browser = browser_guard
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Browser is closed".to_string()))?;

        let tid: TargetId = TargetId::from(target_id.to_string());

        // Activate the target (brings it to front)
        browser
            .execute(ActivateTargetParams::new(tid.clone()))
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to activate target: {}", e)))?;

        // Get the page for this target
        let page = browser
            .get_page(tid)
            .await
            .map_err(|e| {
                TivanaError::Browser(format!("Target {} not found as page: {}", target_id, e))
            })?;

        let handle = Arc::new(PageHandle::new(page));

        // Put this page at the front of our pages list (making it the "active" page)
        let mut pages = self.pages.write().await;
        pages.insert(0, Arc::clone(&handle));

        info!(target_id = %target_id, "Switched to tab");
        Ok(handle)
    }

    /// Open a new tab with optional URL
    pub async fn open_tab(&self, url: Option<&str>) -> Result<(Arc<PageHandle>, String), TivanaError> {
        let browser_guard = self.browser.read().await;
        let browser = browser_guard
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Browser is closed".to_string()))?;

        let target_url = url.unwrap_or("about:blank");
        let page = browser
            .new_page(target_url)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to create tab: {}", e)))?;

        // Get the target ID for this page
        let target_id = page.target_id().inner().clone();
        let handle = Arc::new(PageHandle::new(page));

        // Make it the active page (insert at front)
        let mut pages = self.pages.write().await;
        pages.insert(0, Arc::clone(&handle));

        info!(target_id = %target_id, url = %target_url, "Opened new tab");
        Ok((handle, target_id))
    }

    /// Close a tab by target ID
    pub async fn close_tab(&self, target_id: &str) -> Result<(), TivanaError> {
        use chromiumoxide::cdp::browser_protocol::target::{CloseTargetParams, TargetId};

        let browser_guard = self.browser.read().await;
        let browser = browser_guard
            .as_ref()
            .ok_or_else(|| TivanaError::Browser("Browser is closed".to_string()))?;

        let tid: TargetId = TargetId::from(target_id.to_string());
        browser
            .execute(CloseTargetParams::new(tid))
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to close target: {}", e)))?;

        info!(target_id = %target_id, "Closed tab");
        Ok(())
    }

    /// Close the browser and all pages.
    /// For external browsers (--connect mode), only disconnects without killing Chrome.
    pub async fn close(self) -> Result<(), TivanaError> {
        if self.is_external {
            info!("Disconnecting from external browser (Chrome will keep running)");
        } else {
            info!("Closing browser");
        }

        // Clear our page handles
        self.pages.write().await.clear();

        // Take ownership of browser and drop it (closes CDP connection)
        let mut browser_guard = self.browser.write().await;
        if let Some(browser) = browser_guard.take() {
            drop(browser);
        }

        // Abort handler task if still running
        if let Some(handler) = self.handler_task {
            handler.abort();
        }

        Ok(())
    }
}

/// Information about a browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    /// CDP target ID
    pub target_id: String,

    /// Current URL
    pub url: String,

    /// Page title
    pub title: String,

    /// Whether this is the currently active tab in Tivana
    pub active: bool,
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

    /// Get the default headless setting from server config
    pub fn default_headless(&self) -> bool {
        self.default_config.headless
    }

    /// Access the default browser launch configuration
    pub fn default_config(&self) -> &BrowserLaunchConfig {
        &self.default_config
    }

    /// Launch a new browser instance
    pub async fn launch(
        &self,
        config: Option<BrowserLaunchConfig>,
        proxy: Option<&crate::proxy::ProxyConfig>,
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

        // Anti-bot-detection: remove automation fingerprints
        builder = builder
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--disable-features=AutomationControlled")
            .arg("--disable-infobars");

        // Use realistic user agent
        builder = builder.arg(
            "--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        );

        // Add custom args
        for arg in &config.args {
            builder = builder.arg(arg);
        }

        // Add proxy server flag if configured
        if let Some(proxy) = proxy {
            let proxy_arg = proxy.to_chrome_arg();
            info!(proxy = %proxy_arg, "Configuring proxy");
            builder = builder.arg(proxy_arg);
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

        // Inject stealth patches to defeat bot detection
        inject_stealth_scripts(&handle).await;

        info!("Browser launched successfully");
        Ok(handle)
    }

    /// Connect to an existing Chrome instance via debugging port or WebSocket URL
    pub async fn connect_existing(
        &self,
        target: &str,
    ) -> Result<BrowserHandle, TivanaError> {
        // Determine the WebSocket URL
        let ws_url = if target.starts_with("ws://") || target.starts_with("wss://") {
            target.to_string()
        } else {
            // Treat as a port number — discover the WebSocket URL
            let port = target
                .parse::<u16>()
                .map_err(|_| TivanaError::Browser(format!("Invalid connect target: {}. Use a port number or ws:// URL", target)))?;

            // Query Chrome's /json/version endpoint to get the WebSocket debugger URL
            let version_url = format!("http://127.0.0.1:{}/json/version", port);
            info!(url = %version_url, "Discovering Chrome WebSocket URL");

            let resp = reqwest::get(&version_url)
                .await
                .map_err(|e| TivanaError::Browser(format!(
                    "Failed to connect to Chrome on port {}. Is Chrome running with --remote-debugging-port={}? Error: {}",
                    port, port, e
                )))?;

            let version_info: serde_json::Value = resp.json().await.map_err(|e| {
                TivanaError::Browser(format!("Failed to parse Chrome version info: {}", e))
            })?;

            version_info["webSocketDebuggerUrl"]
                .as_str()
                .ok_or_else(|| TivanaError::Browser("Chrome did not provide webSocketDebuggerUrl".to_string()))?
                .to_string()
        };

        info!(ws_url = %ws_url, "Connecting to existing Chrome instance");

        let (browser, mut handler) = Browser::connect(&ws_url)
            .await
            .map_err(|e| TivanaError::Browser(format!("Failed to connect to Chrome: {}", e)))?;

        // Spawn handler task
        let handler_task = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                debug!(?event, "Browser event");
            }
        });

        let config = self.default_config.clone();
        let handle = BrowserHandle::new_external(config, browser, handler_task);

        // Open a new tab (don't touch existing tabs)
        handle.new_page().await?;

        // Inject stealth patches to defeat bot detection
        inject_stealth_scripts(&handle).await;

        info!("Connected to existing Chrome instance successfully");
        Ok(handle)
    }

    /// Launch Chrome using the user's default browser profile.
    /// This quits any running Chrome, then relaunches with --remote-debugging-port
    /// but WITHOUT --user-data-dir, so Chrome uses its default profile
    /// (~/Library/Application Support/Google/Chrome on macOS).
    /// This gives the highest reCAPTCHA trust score since the browser has
    /// real cookies, login sessions, and browsing history.
    pub async fn launch_default_browser(
        &self,
        debug_port: u16,
    ) -> Result<BrowserHandle, TivanaError> {
        info!(port = debug_port, "Launching Chrome with default browser profile");

        // Step 1: Check if Chrome is already running with debugging on this port
        let already_running = reqwest::get(format!("http://127.0.0.1:{}/json/version", debug_port))
            .await
            .is_ok();

        if already_running {
            info!("Chrome already running with debugging on port {} — connecting", debug_port);
            return self.connect_existing(&debug_port.to_string()).await;
        }

        // Step 2: Gracefully quit any running Chrome
        info!("Quitting existing Chrome instances...");
        #[cfg(target_os = "macos")]
        {
            // Check if Chrome is already running with debugging on this port
            if reqwest::get(format!("http://127.0.0.1:{}/json/version", debug_port))
                .await
                .is_ok()
            {
                info!("Chrome already has debugging on port {} — connecting directly", debug_port);
                return self.connect_existing(&debug_port.to_string()).await;
            }

            // Quit Chrome gracefully via AppleScript
            let quit_result = tokio::process::Command::new("osascript")
                .args(["-e", "tell application \"Google Chrome\" to quit"])
                .output()
                .await;
            match quit_result {
                Ok(output) if output.status.success() => {
                    info!("Chrome quit requested, waiting for process to exit...");
                }
                _ => {
                    info!("No running Chrome to quit (or quit failed)");
                }
            }
            
            // Wait for ALL Chrome processes to exit (renderer, helper, gpu, etc.)
            for i in 0..40 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let check = tokio::process::Command::new("pgrep")
                    .args(["-f", "Google Chrome.app"])
                    .output()
                    .await;
                if let Ok(output) = check {
                    if !output.status.success() {
                        info!(attempt = i + 1, "All Chrome processes exited");
                        break;
                    }
                }
                if i == 20 {
                    // Force kill after 10 seconds
                    info!("Force killing Chrome processes...");
                    let _ = tokio::process::Command::new("pkill")
                        .args(["-9", "-f", "Google Chrome"])
                        .output()
                        .await;
                }
                if i == 39 {
                    info!("Chrome processes still running after 20s, proceeding anyway");
                }
            }
            
            // Extra grace period for file locks to release
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        #[cfg(not(target_os = "macos"))]
        {
            // On Linux/Windows, try pkill
            let _ = tokio::process::Command::new("pkill")
                .args(["-f", "chrome"])
                .output()
                .await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        // Step 3: Launch Chrome with a persistent profile at ~/.tivana/chrome-profile.
        // Chrome requires --user-data-dir for --remote-debugging-port to work.
        // macOS Keychain encryption prevents sharing cookies across profiles.
        // Users log in once and the session persists across Tivana restarts.
        let chrome_path = self.find_chrome_path()?;
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let profile_dir = home.join(".tivana").join("chrome-profile");
        
        if !profile_dir.exists() {
            std::fs::create_dir_all(&profile_dir).map_err(|e| {
                TivanaError::Browser(format!("Failed to create profile dir: {}", e))
            })?;
            info!(profile_dir = %profile_dir.display(), "Created persistent Chrome profile");
        } else {
            info!(profile_dir = %profile_dir.display(), "Using existing persistent Chrome profile");
        }

        info!(path = %chrome_path.display(), "Launching Chrome with persistent Tivana profile");

        let mut cmd = tokio::process::Command::new(&chrome_path);
        cmd.arg(format!("--remote-debugging-port={}", debug_port))
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .arg("--no-first-run")
            .arg("--window-size=1440,900")
            .arg("about:blank")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        cmd.spawn().map_err(|e| {
            TivanaError::Browser(format!("Failed to launch Chrome: {}", e))
        })?;

        // Step 4: Wait for Chrome to be ready (default profile can take longer)
        info!("Waiting for Chrome to start...");
        for i in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(resp) = reqwest::get(format!("http://127.0.0.1:{}/json/version", debug_port)).await {
                if resp.status().is_success() {
                    info!(attempt = i + 1, "Chrome is ready");
                    // Extra wait for profile to fully load
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    break;
                }
            }
            if i == 39 {
                return Err(TivanaError::Browser(
                    "Chrome failed to start within 20 seconds".to_string(),
                ));
            }
        }

        // Step 5: Connect to the newly launched Chrome
        self.connect_existing(&debug_port.to_string()).await
    }

    /// Find the Chrome executable path
    fn find_chrome_path(&self) -> Result<PathBuf, TivanaError> {
        if let Some(ref path) = self.default_config.chrome_path {
            return Ok(path.clone());
        }

        #[cfg(target_os = "macos")]
        {
            let path = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
            if path.exists() {
                return Ok(path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            for name in &["google-chrome", "google-chrome-stable", "chromium-browser", "chromium"] {
                if let Ok(output) = std::process::Command::new("which").arg(name).output() {
                    if output.status.success() {
                        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        return Ok(PathBuf::from(path));
                    }
                }
            }
        }

        Err(TivanaError::Browser("Could not find Chrome. Install Chrome or pass --chrome-path".to_string()))
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new(BrowserLaunchConfig::default())
    }
}

/// Inject comprehensive stealth scripts into a browser handle to defeat bot detection.
/// This is called automatically on every browser launch and connect — zero config.
async fn inject_stealth_scripts(handle: &BrowserHandle) {
    if let Ok(page) = handle.default_page().await {
        let stealth_script = get_stealth_script();

        use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
        let cmd = AddScriptToEvaluateOnNewDocumentParams::new(&stealth_script);
        let _ = page.inner().execute(cmd).await;

        // Also run immediately on current page
        let _ = page.evaluate_void(&stealth_script).await;
    }
}

/// Returns the full stealth JavaScript payload.
/// Every Tivana session gets this — no flags, no config.
fn get_stealth_script() -> String {
    r#"
// === navigator.webdriver ===
// Delete the property entirely, then redefine as undefined
delete Object.getPrototypeOf(navigator).webdriver;
Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined,
    configurable: true,
});

// === chrome.runtime ===
if (!window.chrome) { window.chrome = {}; }
if (!window.chrome.runtime) {
    window.chrome.runtime = {
        connect: function() {},
        sendMessage: function() {},
        id: undefined,
    };
}
// chrome.app for iframe contentWindow checks
if (!window.chrome.app) {
    window.chrome.app = {
        isInstalled: false,
        InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
        RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' },
        getDetails: function() { return null; },
        getIsInstalled: function() { return false; },
    };
}

// === Permissions API ===
const _origPermQuery = navigator.permissions.query.bind(navigator.permissions);
Object.defineProperty(navigator.permissions, 'query', {
    value: (params) => {
        if (params.name === 'notifications') {
            return Promise.resolve({ state: 'prompt', onchange: null });
        }
        return _origPermQuery(params);
    },
    writable: true,
    configurable: true,
});

// === Navigator properties ===
Object.defineProperty(navigator, 'languages', {
    get: () => ['en-US', 'en'],
    configurable: true,
});
Object.defineProperty(navigator, 'hardwareConcurrency', {
    get: () => 8,
    configurable: true,
});
Object.defineProperty(navigator, 'deviceMemory', {
    get: () => 8,
    configurable: true,
});
Object.defineProperty(navigator, 'maxTouchPoints', {
    get: () => 0,
    configurable: true,
});
// Desktop Chrome removed navigator.getBattery
if (navigator.getBattery) {
    Object.defineProperty(navigator, 'getBattery', {
        get: () => undefined,
        configurable: true,
    });
}
// navigator.connection (NetworkInformation API)
Object.defineProperty(navigator, 'connection', {
    get: () => ({
        effectiveType: '4g',
        rtt: 50,
        downlink: 10,
        saveData: false,
        onchange: null,
    }),
    configurable: true,
});

// === Plugins & MimeTypes with proper prototype chain ===
(function() {
    const pluginData = [
        { name: 'PDF Viewer', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
        { name: 'Chrome PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
        { name: 'Chromium PDF Plugin', filename: 'internal-pdf-viewer', description: 'Portable Document Format' },
        { name: 'Chrome PDF Viewer', filename: 'internal-pdf-viewer', description: '' },
        { name: 'Native Client', filename: 'internal-nacl-plugin', description: '' },
    ];

    const mimeTypes = [
        { type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format' },
        { type: 'application/x-google-chrome-pdf', suffixes: 'pdf', description: 'Portable Document Format' },
    ];

    // Build fake PluginArray
    const fakePlugins = Object.create(PluginArray.prototype);
    pluginData.forEach((p, i) => {
        const plugin = Object.create(Plugin.prototype);
        Object.defineProperties(plugin, {
            name: { value: p.name, enumerable: true },
            filename: { value: p.filename, enumerable: true },
            description: { value: p.description, enumerable: true },
            length: { value: mimeTypes.length, enumerable: true },
        });
        // Index access
        mimeTypes.forEach((m, j) => {
            const mt = Object.create(MimeType.prototype);
            Object.defineProperties(mt, {
                type: { value: m.type, enumerable: true },
                suffixes: { value: m.suffixes, enumerable: true },
                description: { value: m.description, enumerable: true },
                enabledPlugin: { value: plugin, enumerable: true },
            });
            Object.defineProperty(plugin, j, { value: mt, enumerable: false });
            Object.defineProperty(plugin, m.type, { value: mt, enumerable: false });
        });
        Object.defineProperty(fakePlugins, i, { value: plugin, enumerable: true });
        Object.defineProperty(fakePlugins, p.name, { value: plugin, enumerable: false });
    });
    Object.defineProperty(fakePlugins, 'length', { value: pluginData.length, enumerable: true });
    fakePlugins.item = function(i) { return this[i] || null; };
    fakePlugins.namedItem = function(n) { return this[n] || null; };
    fakePlugins.refresh = function() {};

    Object.defineProperty(navigator, 'plugins', {
        get: () => fakePlugins,
        configurable: true,
    });

    // Build fake MimeTypeArray
    const fakeMimes = Object.create(MimeTypeArray.prototype);
    mimeTypes.forEach((m, i) => {
        const mt = Object.create(MimeType.prototype);
        Object.defineProperties(mt, {
            type: { value: m.type, enumerable: true },
            suffixes: { value: m.suffixes, enumerable: true },
            description: { value: m.description, enumerable: true },
            enabledPlugin: { value: fakePlugins[0], enumerable: true },
        });
        Object.defineProperty(fakeMimes, i, { value: mt, enumerable: true });
        Object.defineProperty(fakeMimes, m.type, { value: mt, enumerable: false });
    });
    Object.defineProperty(fakeMimes, 'length', { value: mimeTypes.length, enumerable: true });
    fakeMimes.item = function(i) { return this[i] || null; };
    fakeMimes.namedItem = function(n) { return this[n] || null; };

    Object.defineProperty(navigator, 'mimeTypes', {
        get: () => fakeMimes,
        configurable: true,
    });
})();

// === WebGL fingerprint ===
(function() {
    const getParamOrig = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(param) {
        // UNMASKED_VENDOR_WEBGL
        if (param === 0x9245) return 'Google Inc. (Apple)';
        // UNMASKED_RENDERER_WEBGL
        if (param === 0x9246) return 'ANGLE (Apple, Apple M1, OpenGL 4.1)';
        return getParamOrig.call(this, param);
    };
    // Also patch WebGL2 if available
    if (typeof WebGL2RenderingContext !== 'undefined') {
        const getParam2Orig = WebGL2RenderingContext.prototype.getParameter;
        WebGL2RenderingContext.prototype.getParameter = function(param) {
            if (param === 0x9245) return 'Google Inc. (Apple)';
            if (param === 0x9246) return 'ANGLE (Apple, Apple M1, OpenGL 4.1)';
            return getParam2Orig.call(this, param);
        };
    }
})();

// === Canvas fingerprint noise ===
(function() {
    // Unique per-session seed for consistent-but-unique noise
    const seed = Math.floor(Math.random() * 0xFFFFFFFF);
    function mulberry32(a) {
        return function() {
            a |= 0; a = a + 0x6D2B79F5 | 0;
            let t = Math.imul(a ^ a >>> 15, 1 | a);
            t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t;
            return ((t ^ t >>> 14) >>> 0) / 4294967296;
        };
    }
    const rng = mulberry32(seed);

    // Patch toDataURL to add subtle noise
    const origToDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function() {
        try {
            const ctx = this.getContext('2d');
            if (ctx && this.width > 0 && this.height > 0) {
                const imageData = ctx.getImageData(0, 0, Math.min(this.width, 16), Math.min(this.height, 16));
                // Flip 1-2 bits in a few random pixels
                for (let i = 0; i < 4; i++) {
                    const idx = Math.floor(rng() * imageData.data.length);
                    imageData.data[idx] = imageData.data[idx] ^ 1;
                }
                ctx.putImageData(imageData, 0, 0);
            }
        } catch(e) { /* cross-origin or WebGL canvas, skip */ }
        return origToDataURL.apply(this, arguments);
    };

    // Patch toBlob similarly
    const origToBlob = HTMLCanvasElement.prototype.toBlob;
    HTMLCanvasElement.prototype.toBlob = function() {
        try {
            const ctx = this.getContext('2d');
            if (ctx && this.width > 0 && this.height > 0) {
                const imageData = ctx.getImageData(0, 0, Math.min(this.width, 16), Math.min(this.height, 16));
                for (let i = 0; i < 4; i++) {
                    const idx = Math.floor(rng() * imageData.data.length);
                    imageData.data[idx] = imageData.data[idx] ^ 1;
                }
                ctx.putImageData(imageData, 0, 0);
            }
        } catch(e) {}
        return origToBlob.apply(this, arguments);
    };

    // Patch getImageData to add 1-bit noise to random pixels
    const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
    CanvasRenderingContext2D.prototype.getImageData = function() {
        const imageData = origGetImageData.apply(this, arguments);
        // Flip the LSB of 2-4 random pixel components
        const numFlips = 2 + Math.floor(rng() * 3);
        for (let i = 0; i < numFlips; i++) {
            const idx = Math.floor(rng() * imageData.data.length);
            imageData.data[idx] = imageData.data[idx] ^ 1;
        }
        return imageData;
    };
})();

// === AudioContext fingerprint noise ===
(function() {
    // Add tiny frequency variations to oscillators
    if (typeof OscillatorNode !== 'undefined') {
        const origSetValue = Object.getOwnPropertyDescriptor(AudioParam.prototype, 'value');
        if (origSetValue && origSetValue.set) {
            const origSetter = origSetValue.set;
            Object.defineProperty(AudioParam.prototype, 'value', {
                get: origSetValue.get,
                set: function(val) {
                    // Add ±0.001% noise to frequency values
                    if (typeof val === 'number' && val > 0) {
                        val = val * (1 + (Math.random() - 0.5) * 0.00002);
                    }
                    origSetter.call(this, val);
                },
                configurable: true,
                enumerable: true,
            });
        }
    }

    // Patch OfflineAudioContext.startRendering to add subtle noise to output
    if (typeof OfflineAudioContext !== 'undefined') {
        const origStartRendering = OfflineAudioContext.prototype.startRendering;
        OfflineAudioContext.prototype.startRendering = function() {
            return origStartRendering.apply(this, arguments).then(function(buffer) {
                // Add tiny noise to the first channel
                try {
                    const data = buffer.getChannelData(0);
                    for (let i = 0; i < Math.min(data.length, 100); i++) {
                        data[i] += (Math.random() - 0.5) * 0.0000001;
                    }
                } catch(e) {}
                return buffer;
            });
        };
    }
})();

// === iframe contentWindow protection ===
// Ensure iframes don't leak automation signals through contentWindow
(function() {
    const origContentWindow = Object.getOwnPropertyDescriptor(HTMLIFrameElement.prototype, 'contentWindow');
    if (origContentWindow && origContentWindow.get) {
        Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {
            get: function() {
                const win = origContentWindow.get.call(this);
                // For same-origin iframes, ensure chrome object is consistent
                try {
                    if (win && win.chrome === undefined) {
                        win.chrome = window.chrome;
                    }
                } catch(e) { /* cross-origin, expected */ }
                return win;
            },
            configurable: true,
        });
    }
})();
"#.to_string()
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
        assert_eq!(config.viewport_width, 1440);
        assert_eq!(config.viewport_height, 900);
    }
}
