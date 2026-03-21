//! Extension bridge: manages Chrome extension-connected tabs as Tivana sessions.
//!
//! When the Tivana Chrome extension connects, it forwards CDP commands through
//! the extension's chrome.debugger API instead of a direct browser connection.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info};

/// Info about an extension-connected tab
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionTab {
    pub tab_id: i64,
    pub target_id: String,
    pub session_id: String,
    pub url: String,
    pub title: String,
}

/// A pending CDP request sent through the extension
struct PendingCdpRequest {
    tx: tokio::sync::oneshot::Sender<CdpResponse>,
}

/// Response from a CDP command forwarded through the extension
#[derive(Debug)]
pub struct CdpResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Manages extension WebSocket connections and their attached tabs
pub struct ExtensionManager {
    /// Extension tabs: sessionId → ExtensionTab
    tabs: Arc<RwLock<HashMap<String, ExtensionTab>>>,

    /// The writer channel for sending messages to the extension WebSocket.
    /// Set when an extension connects, cleared on disconnect.
    ws_tx: Arc<Mutex<Option<mpsc::Sender<String>>>>,

    /// Pending CDP requests: message id → oneshot sender
    pending: Arc<Mutex<HashMap<String, PendingCdpRequest>>>,

    /// Counter for CDP request IDs
    id_counter: Arc<Mutex<u64>>,

    /// Tivana session ID → extension session ID mapping
    session_map: Arc<RwLock<HashMap<String, String>>>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(RwLock::new(HashMap::new())),
            ws_tx: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            id_counter: Arc::new(Mutex::new(0)),
            session_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an extension WebSocket connection
    pub async fn set_connection(&self, tx: mpsc::Sender<String>) {
        let mut ws = self.ws_tx.lock().await;
        *ws = Some(tx);
        info!("Extension WebSocket connected");
    }

    /// Clear the extension connection (on disconnect)
    pub async fn clear_connection(&self) {
        let mut ws = self.ws_tx.lock().await;
        *ws = None;

        // Clear all tabs since the extension disconnected
        let mut tabs = self.tabs.write().await;
        let count = tabs.len();
        tabs.clear();

        // Reject all pending CDP requests
        let mut pending = self.pending.lock().await;
        for (_, req) in pending.drain() {
            let _ = req.tx.send(CdpResponse {
                result: None,
                error: Some("Extension disconnected".to_string()),
            });
        }

        if count > 0 {
            info!(count, "Extension disconnected, cleared tabs");
        }
    }

    /// Handle a tab.attached message from the extension
    pub async fn handle_tab_attached(&self, params: &serde_json::Value) {
        let tab = ExtensionTab {
            tab_id: params
                .get("tabId")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            target_id: params
                .get("targetId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            session_id: params
                .get("sessionId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            url: params
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: params
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        };

        info!(
            session_id = %tab.session_id,
            url = %tab.url,
            "Extension tab attached"
        );

        let mut tabs = self.tabs.write().await;
        tabs.insert(tab.session_id.clone(), tab);
    }

    /// Handle a tab.detached message from the extension
    pub async fn handle_tab_detached(&self, params: &serde_json::Value) {
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let mut tabs = self.tabs.write().await;
        if tabs.remove(session_id).is_some() {
            info!(session_id, reason, "Extension tab detached");
        }
    }

    /// Handle a tab.navigated message from the extension
    pub async fn handle_tab_navigated(&self, params: &serde_json::Value) {
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.get_mut(session_id) {
            tab.url = url.to_string();
            tab.title = title.to_string();
            debug!(session_id, url, "Extension tab navigated");
        }
    }

    /// Handle a CDP response from the extension (matched by id)
    pub async fn handle_cdp_response(&self, msg: &serde_json::Value) {
        let id = match msg.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                // Try numeric id
                match msg.get("id").and_then(|v| v.as_u64()) {
                    Some(id) => id.to_string(),
                    None => return,
                }
            }
        };

        let mut pending = self.pending.lock().await;
        if let Some(req) = pending.remove(&id) {
            let response = CdpResponse {
                result: msg.get("result").cloned(),
                error: msg.get("error").and_then(|v| v.as_str()).map(String::from),
            };
            let _ = req.tx.send(response);
        }
    }

    /// List all extension-connected tabs
    pub async fn list_tabs(&self) -> Vec<ExtensionTab> {
        let tabs = self.tabs.read().await;
        tabs.values().cloned().collect()
    }

    /// Get a specific tab by session ID
    pub async fn get_tab(&self, session_id: &str) -> Option<ExtensionTab> {
        let tabs = self.tabs.read().await;
        tabs.get(session_id).cloned()
    }

    /// Check if the extension is connected
    pub async fn is_connected(&self) -> bool {
        self.ws_tx.lock().await.is_some()
    }

    /// Register a mapping from Tivana session ID to extension session ID
    pub async fn register_session_mapping(&self, tivana_id: &str, extension_id: &str) {
        let mut map = self.session_map.write().await;
        map.insert(tivana_id.to_string(), extension_id.to_string());
    }

    /// Check if a Tivana session ID is backed by an extension tab
    pub async fn is_extension_session(&self, tivana_id: &str) -> bool {
        let map = self.session_map.read().await;
        map.contains_key(tivana_id)
    }

    /// Get extension session ID for a Tivana session
    pub async fn get_extension_session_id(&self, tivana_id: &str) -> Option<String> {
        let map = self.session_map.read().await;
        map.get(tivana_id).cloned()
    }

    /// Evaluate JavaScript in the extension tab
    pub async fn evaluate(&self, tivana_session_id: &str, expression: &str) -> Result<serde_json::Value, String> {
        let ext_session_id = self.get_extension_session_id(tivana_session_id).await
            .ok_or_else(|| "No extension session mapping found".to_string())?;

        self.send_cdp_command(
            &ext_session_id,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            }),
        ).await
    }

    /// Navigate the extension tab
    pub async fn navigate(&self, tivana_session_id: &str, url: &str) -> Result<serde_json::Value, String> {
        let ext_session_id = self.get_extension_session_id(tivana_session_id).await
            .ok_or_else(|| "No extension session mapping found".to_string())?;

        // Use Page.navigate which the extension intercepts and implements
        // via chrome.tabs.update (avoids debugger detach)
        let result = self.send_cdp_command(
            &ext_session_id,
            "Page.navigate",
            serde_json::json!({ "url": url }),
        ).await;

        // Wait for navigation to complete
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        match result {
            Ok(r) => Ok(r),
            Err(_) => Ok(serde_json::json!({"navigated": true, "url": url})),
        }
    }

    /// Get page state from extension tab
    pub async fn page_state(&self, tivana_session_id: &str) -> Result<serde_json::Value, String> {
        let ext_session_id = self.get_extension_session_id(tivana_session_id).await
            .ok_or_else(|| "No extension session mapping found".to_string())?;

        // Get URL and title via JS
        let result = self.send_cdp_command(
            &ext_session_id,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "JSON.stringify({url: location.href, title: document.title, viewportWidth: window.innerWidth, viewportHeight: window.innerHeight, scrollX: window.scrollX, scrollY: window.scrollY})",
                "returnByValue": true,
            }),
        ).await?;

        // Parse the nested result
        if let Some(value) = result.get("result").and_then(|r| r.get("value")) {
            if let Some(s) = value.as_str() {
                return serde_json::from_str(s).map_err(|e| e.to_string());
            }
            return Ok(value.clone());
        }
        Ok(result)
    }

    /// Click at coordinates in extension tab
    pub async fn click(&self, tivana_session_id: &str, x: f64, y: f64) -> Result<serde_json::Value, String> {
        let ext_session_id = self.get_extension_session_id(tivana_session_id).await
            .ok_or_else(|| "No extension session mapping found".to_string())?;

        // Mouse pressed
        self.send_cdp_command(
            &ext_session_id,
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mousePressed",
                "x": x, "y": y,
                "button": "left",
                "clickCount": 1,
            }),
        ).await?;

        // Mouse released
        self.send_cdp_command(
            &ext_session_id,
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseReleased",
                "x": x, "y": y,
                "button": "left",
                "clickCount": 1,
            }),
        ).await
    }

    /// Type text in extension tab
    pub async fn type_text(&self, tivana_session_id: &str, text: &str) -> Result<(), String> {
        let ext_session_id = self.get_extension_session_id(tivana_session_id).await
            .ok_or_else(|| "No extension session mapping found".to_string())?;

        for ch in text.chars() {
            // RawKeyDown
            self.send_cdp_command(
                &ext_session_id,
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "rawKeyDown",
                    "key": ch.to_string(),
                    "text": ch.to_string(),
                }),
            ).await?;

            // Char
            self.send_cdp_command(
                &ext_session_id,
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "char",
                    "text": ch.to_string(),
                }),
            ).await?;

            // KeyUp
            self.send_cdp_command(
                &ext_session_id,
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyUp",
                    "key": ch.to_string(),
                }),
            ).await?;
        }
        Ok(())
    }

    /// Send a CDP command through the extension and wait for the response
    pub async fn send_cdp_command(
        &self,
        session_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let ws = self.ws_tx.lock().await;
        let tx = ws
            .as_ref()
            .ok_or_else(|| "Extension not connected".to_string())?
            .clone();

        // Generate request ID
        let id = {
            let mut counter = self.id_counter.lock().await;
            *counter += 1;
            counter.to_string()
        };

        // Create oneshot channel for the response
        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id.clone(), PendingCdpRequest { tx: resp_tx });
        }

        // Send the CDP command
        let msg = serde_json::json!({
            "id": id,
            "method": "cdp",
            "params": {
                "method": method,
                "params": params,
                "sessionId": session_id,
            }
        });

        let msg_str = serde_json::to_string(&msg).map_err(|e| e.to_string())?;

        // Drop the ws lock before sending
        drop(ws);

        tx.send(msg_str)
            .await
            .map_err(|_| "Failed to send to extension".to_string())?;

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(30), resp_rx).await {
            Ok(Ok(response)) => {
                if let Some(err) = response.error {
                    Err(err)
                } else {
                    Ok(response.result.unwrap_or(serde_json::Value::Null))
                }
            }
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                // Remove from pending on timeout
                let mut pending = self.pending.lock().await;
                pending.remove(&id);
                Err("CDP command timed out".to_string())
            }
        }
    }
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if an incoming WebSocket message looks like it's from the extension
/// (as opposed to a regular Tivana SDK client).
pub fn is_extension_message(text: &str) -> bool {
    // Extension messages use a different protocol: {method: "tab.attached"|"tab.detached"|...}
    // SDK messages use: {id, type: "request", method: "session.create"|...}
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
        // Extension messages have "method" but no "type" field
        if val.get("type").is_none() {
            if let Some(method) = val.get("method").and_then(|v| v.as_str()) {
                return matches!(
                    method,
                    "extension.hello"
                        | "tab.attached"
                        | "tab.detached"
                        | "tab.navigated"
                        | "cdp.event"
                        | "pong"
                );
            }
            // Also match responses: {id, result} or {id, error} without "type"
            if val.get("id").is_some()
                && (val.get("result").is_some() || val.get("error").is_some())
                && val.get("type").is_none()
            {
                return true;
            }
        }
    }
    false
}
