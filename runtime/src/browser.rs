//! Chromium browser management
//!
//! This module handles launching and controlling Chromium instances.
//! Currently a stub for Phase 1 - will be fully implemented in Phase 2.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::TivanaError;

/// Browser launch configuration
#[derive(Debug, Clone)]
pub struct BrowserConfig {
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
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            chrome_path: None,
            viewport_width: 1280,
            viewport_height: 720,
            user_data_dir: None,
        }
    }
}

/// Handle to a running browser instance
#[derive(Debug)]
pub struct BrowserHandle {
    /// Configuration used to launch
    pub config: BrowserConfig,

    /// Browser process ID (when available)
    pub pid: Option<u32>,

    // TODO: Add chromiumoxide Browser and Page handles in Phase 2
    // browser: chromiumoxide::Browser,
    // page: chromiumoxide::Page,
}

impl BrowserHandle {
    /// Create a new browser handle (stub for Phase 1)
    pub fn new_stub(config: BrowserConfig) -> Self {
        Self { config, pid: None }
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<NavigationResult, TivanaError> {
        info!(url = %url, "Navigate (stub)");
        // TODO: Implement actual navigation in Phase 2
        Ok(NavigationResult {
            url: url.to_string(),
            title: Some("Stub Page".to_string()),
            load_time_ms: 0,
        })
    }

    /// Get current page URL
    pub async fn current_url(&self) -> Result<String, TivanaError> {
        // TODO: Implement in Phase 2
        Ok("about:blank".to_string())
    }

    /// Get current page title
    pub async fn title(&self) -> Result<Option<String>, TivanaError> {
        // TODO: Implement in Phase 2
        Ok(None)
    }

    /// Execute JavaScript
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value, TivanaError> {
        debug!(script_len = script.len(), "Evaluate (stub)");
        // TODO: Implement in Phase 2
        Ok(serde_json::Value::Null)
    }

    /// Close the browser
    pub async fn close(self) -> Result<(), TivanaError> {
        info!("Browser close (stub)");
        // TODO: Implement actual browser close in Phase 2
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
    default_config: BrowserConfig,
}

impl BrowserManager {
    /// Create a new browser manager
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            default_config: config,
        }
    }

    /// Launch a new browser instance
    pub async fn launch(&self, config: Option<BrowserConfig>) -> Result<BrowserHandle, TivanaError> {
        let config = config.unwrap_or_else(|| self.default_config.clone());

        info!(
            headless = config.headless,
            chrome_path = ?config.chrome_path,
            "Launching browser (stub)"
        );

        // TODO: Actually launch chromium in Phase 2
        // For now, return a stub handle
        Ok(BrowserHandle::new_stub(config))
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new(BrowserConfig::default())
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
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1280);
        assert_eq!(config.viewport_height, 720);
    }

    #[tokio::test]
    async fn test_browser_manager_launch_stub() {
        let manager = BrowserManager::default();
        let handle = manager.launch(None).await.unwrap();
        assert!(handle.config.headless);
    }
}
