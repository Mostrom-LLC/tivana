//! WebSocket server for Tivana protocol
//!
//! Handles client connections, message routing, and response correlation.

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::browser::{BrowserConfig, BrowserManager};
use crate::cli::Args;
use crate::error::{ProtocolError, TivanaError};
use crate::protocol::{
    parse_request, serialize_outbound, EventMessage, OutboundMessage, ResponseMessage,
    PROTOCOL_VERSION,
};
use crate::session::{SessionConfig, SessionRegistry};

/// WebSocket server
pub struct Server {
    /// Server address
    addr: SocketAddr,

    /// Session registry
    sessions: SessionRegistry,

    /// Browser manager
    browser_manager: BrowserManager,

    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
}

impl Server {
    /// Create a new server from CLI args
    pub fn new(args: Args) -> Result<Self, TivanaError> {
        let addr: SocketAddr = format!("{}:{}", args.host, args.port)
            .parse()
            .map_err(|e| {
                TivanaError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
            })?;

        let browser_config = BrowserConfig {
            headless: args.is_headless(),
            chrome_path: args.chrome_path.clone(),
            ..Default::default()
        };

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            addr,
            sessions: SessionRegistry::new(),
            browser_manager: BrowserManager::new(browser_config),
            shutdown_tx,
        })
    }

    /// Run the server
    pub async fn run(self) -> Result<(), TivanaError> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!(addr = %self.addr, "WebSocket server listening");

        // Wrap self in Arc for sharing across connections
        let server = Arc::new(self);

        // Setup shutdown handler
        let shutdown_tx = server.shutdown_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Failed to listen for ctrl+c: {}", e);
                return;
            }
            info!("Shutdown signal received");
            let _ = shutdown_tx.send(());
        });

        let mut shutdown_rx = server.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            info!(peer = %addr, "New connection");
                            let server_clone = Arc::clone(&server);
                            tokio::spawn(async move {
                                if let Err(e) = server_clone.handle_connection(stream, addr).await {
                                    error!(peer = %addr, error = %e, "Connection error");
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Shutting down server");
                    server.sessions.close_all().await;
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single client connection
    async fn handle_connection(
        self: Arc<Self>,
        stream: TcpStream,
        addr: SocketAddr,
    ) -> Result<(), TivanaError> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send hello event
        let hello = EventMessage::new(
            "server.hello",
            serde_json::json!({
                "version": PROTOCOL_VERSION,
                "capabilities": ["session", "browser", "perceive", "act"]
            }),
        );
        let hello_msg = serialize_outbound(&OutboundMessage::Event(hello))?;
        write.send(Message::Text(hello_msg.into())).await?;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!(peer = %addr, len = text.len(), "Received message");
                    let response = self.handle_message(&text).await;
                    let response_json = serialize_outbound(&response)?;
                    write.send(Message::Text(response_json.into())).await?;
                }
                Ok(Message::Binary(data)) => {
                    warn!(peer = %addr, len = data.len(), "Received binary (not supported)");
                }
                Ok(Message::Ping(data)) => {
                    write.send(Message::Pong(data)).await?;
                }
                Ok(Message::Pong(_)) => {}
                Ok(Message::Close(_)) => {
                    info!(peer = %addr, "Client disconnected");
                    break;
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    error!(peer = %addr, error = %e, "WebSocket error");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a single message and return response
    async fn handle_message(&self, text: &str) -> OutboundMessage {
        match parse_request(text) {
            Ok(request) => self.route_request(request).await.into(),
            Err(e) => ResponseMessage::error("unknown", e).into(),
        }
    }

    /// Route request to appropriate handler
    async fn route_request(
        &self,
        request: crate::protocol::RequestMessage,
    ) -> ResponseMessage {
        let id = request.id.clone();

        let result = match request.method.as_str() {
            // Session methods
            "session.create" => self.handle_session_create(&request).await,
            "session.close" => self.handle_session_close(&request).await,
            "session.list" => self.handle_session_list().await,
            "session.get" => self.handle_session_get(&request).await,

            // Browser methods
            "browser.navigate" => self.handle_browser_navigate(&request).await,
            "browser.url" => self.handle_browser_url(&request).await,

            // Perception methods (stubs)
            "perceive.accessibility" => self.handle_perceive_accessibility(&request).await,
            "perceive.text" => self.handle_perceive_text(&request).await,
            "perceive.metadata" => self.handle_perceive_metadata(&request).await,

            // Action methods (stubs)
            "act.click" => self.handle_act_click(&request).await,
            "act.type" => self.handle_act_type(&request).await,
            "act.press" => self.handle_act_press(&request).await,
            "act.scroll" => self.handle_act_scroll(&request).await,

            // Unknown method
            _ => Err(ProtocolError::new(
                crate::error::ErrorCode::UnknownMethod,
                format!("Unknown method: {}", request.method),
            )),
        };

        match result {
            Ok(value) => ResponseMessage::success(id, value),
            Err(e) => ResponseMessage::error(id, e),
        }
    }

    // Session handlers

    async fn handle_session_create(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let config = SessionConfig {
            initial_url: request
                .params
                .get("url")
                .and_then(|v| v.as_str())
                .map(String::from),
            headless: request
                .params
                .get("headless")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            viewport_width: request
                .params
                .get("viewportWidth")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            viewport_height: request
                .params
                .get("viewportHeight")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        };

        let session_id = self.sessions.create(config).await;

        // Start browser launch
        self.sessions
            .start_launch(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Launch browser (stub for now)
        let browser = self
            .browser_manager
            .launch(None)
            .await
            .map_err(|e| ProtocolError::browser_launch_failed(e.to_string()))?;

        // Complete launch
        self.sessions
            .complete_launch(&session_id, browser)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let info = self.sessions.get(&session_id).await.unwrap();

        Ok(serde_json::json!({
            "sessionId": session_id,
            "state": info.state
        }))
    }

    async fn handle_session_close(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let info = self.sessions.close(&session_id).await?;

        Ok(serde_json::json!({
            "sessionId": info.id,
            "state": info.state
        }))
    }

    /// Helper to extract session ID from request
    fn extract_session_id(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<String, ProtocolError> {
        request
            .session_id
            .clone()
            .or_else(|| {
                request
                    .params
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))
    }

    async fn handle_session_list(&self) -> Result<serde_json::Value, ProtocolError> {
        let sessions = self.sessions.list().await;
        Ok(serde_json::json!({
            "sessions": sessions
        }))
    }

    async fn handle_session_get(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let info = self
            .sessions
            .get(&session_id)
            .await
            .ok_or_else(|| ProtocolError::session_not_found(&session_id))?;

        Ok(serde_json::to_value(info).unwrap())
    }

    // Browser handlers (stubs)

    async fn handle_browser_navigate(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        let url = request
            .params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("url"))?;

        info!(url, "Navigate request (stub)");

        Ok(serde_json::json!({
            "url": url,
            "title": "Stub Page",
            "loadTimeMs": 0
        }))
    }

    async fn handle_browser_url(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        Ok(serde_json::json!({
            "url": "about:blank"
        }))
    }

    // Perception handlers (stubs)

    async fn handle_perceive_accessibility(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        Ok(serde_json::json!({
            "root": {
                "role": "document",
                "name": "Stub Document",
                "children": []
            },
            "timestampMs": 0
        }))
    }

    async fn handle_perceive_text(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        Ok(serde_json::json!({
            "text": "",
            "wordCount": 0,
            "charCount": 0
        }))
    }

    async fn handle_perceive_metadata(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        Ok(serde_json::json!({
            "url": "about:blank",
            "title": null
        }))
    }

    // Action handlers (stubs)

    async fn handle_act_click(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        let target = request
            .params
            .get("target")
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        info!(?target, "Click action (stub)");

        Ok(serde_json::json!({
            "success": true,
            "durationMs": 0
        }))
    }

    async fn handle_act_type(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        let text = request
            .params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("text"))?;

        info!(text_len = text.len(), "Type action (stub)");

        Ok(serde_json::json!({
            "success": true,
            "durationMs": 0
        }))
    }

    async fn handle_act_press(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        let key = request
            .params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("key"))?;

        info!(key, "Press action (stub)");

        Ok(serde_json::json!({
            "success": true,
            "durationMs": 0
        }))
    }

    async fn handle_act_scroll(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let _session_id = request
            .session_id
            .as_ref()
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        info!("Scroll action (stub)");

        Ok(serde_json::json!({
            "success": true,
            "durationMs": 0
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_addr_parsing() {
        let args = Args {
            port: 9876,
            headless: false,
            headed: true,
            chrome_path: None,
            host: "127.0.0.1".to_string(),
        };
        let server = Server::new(args).unwrap();
        assert_eq!(server.addr.port(), 9876);
    }
}
