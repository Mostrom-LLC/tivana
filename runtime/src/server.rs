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

use crate::act::{ActionTarget, Actor, ClickOptions, ScrollDirection, ScrollOptions, TypeOptions};
use crate::browser::{BrowserLaunchConfig, BrowserManager};
use crate::cli::Args;
use crate::error::{ProtocolError, TivanaError};
use crate::perceive::{setup_mutation_observer, stop_mutation_observer, Perceiver};
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

        let browser_config = BrowserLaunchConfig {
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

            // Perception methods
            "perceive.pageState" => self.handle_perceive_page_state(&request).await,
            "perceive.elements" => self.handle_perceive_elements(&request).await,
            "perceive.accessibility" => self.handle_perceive_accessibility(&request).await,
            "perceive.text" => self.handle_perceive_text(&request).await,
            "perceive.metadata" => self.handle_perceive_metadata(&request).await,
            "perceive.mutations" => self.handle_perceive_mutations(&request).await,
            "perceive.mutations.poll" => self.handle_perceive_mutations_poll(&request).await,
            "perceive.mutations.stop" => self.handle_perceive_mutations_stop(&request).await,

            // Action methods
            "act.click" => self.handle_act_click(&request).await,
            "act.type" => self.handle_act_type(&request).await,
            "act.press" => self.handle_act_press(&request).await,
            "act.scroll" => self.handle_act_scroll(&request).await,
            "act.navigate" => self.handle_browser_navigate(&request).await,
            "act.hover" => self.handle_act_hover(&request).await,
            "act.focus" => self.handle_act_focus(&request).await,
            "act.select" => self.handle_act_select(&request).await,
            "act.waitFor" => self.handle_act_wait_for(&request).await,

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

    // Helper to extract session ID from request
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

    // Helper to parse action target from params
    fn parse_action_target(params: &serde_json::Value) -> Option<ActionTarget> {
        let target = params.get("target")?;

        Some(ActionTarget {
            element_id: target.get("elementId").and_then(|v| v.as_str()).map(String::from),
            selector: target.get("selector").and_then(|v| v.as_str()).map(String::from),
            text: target.get("text").and_then(|v| v.as_str()).map(String::from),
            role: target.get("role").and_then(|v| v.as_str()).map(String::from),
            label: target.get("label").and_then(|v| v.as_str()).map(String::from),
            coordinates: target.get("coordinates").and_then(|v| {
                let x = v.get("x").and_then(|x| x.as_f64())?;
                let y = v.get("y").and_then(|y| y.as_f64())?;
                Some((x, y))
            }),
        })
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

        let session_id = self.sessions.create(config.clone()).await;

        // Start browser launch
        self.sessions
            .start_launch(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Build browser launch config
        let browser_config = BrowserLaunchConfig {
            headless: config.headless,
            viewport_width: config.viewport_width.unwrap_or(1280),
            viewport_height: config.viewport_height.unwrap_or(720),
            ..Default::default()
        };

        // Launch browser
        let browser = self
            .browser_manager
            .launch(Some(browser_config))
            .await
            .map_err(|e| ProtocolError::browser_launch_failed(e.to_string()))?;

        // Complete launch
        self.sessions
            .complete_launch(&session_id, browser)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Navigate to initial URL if specified
        if let Some(url) = config.initial_url {
            let page = self
                .sessions
                .get_page(&session_id)
                .await
                .map_err(|e| ProtocolError::internal(e.to_string()))?;

            Actor::navigate(&page, &url)
                .await
                .map_err(|e| ProtocolError::new(crate::error::ErrorCode::NavigationFailed, e.to_string()))?;
        }

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

    // Browser handlers

    async fn handle_browser_navigate(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let url = request
            .params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("url"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::navigate(&page, url)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::NavigationFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_browser_url(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let url = page.url().await.map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "url": url
        }))
    }

    // Perception handlers

    async fn handle_perceive_page_state(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let state = Perceiver::page_state(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&state).unwrap_or_default())
    }

    async fn handle_perceive_elements(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let elements = Perceiver::elements(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        Ok(serde_json::json!({
            "elements": elements,
            "count": elements.len()
        }))
    }

    async fn handle_perceive_accessibility(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let snapshot = Perceiver::accessibility_snapshot(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&snapshot).unwrap_or_default())
    }

    async fn handle_perceive_text(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let content = Perceiver::text_content(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&content).unwrap_or_default())
    }

    async fn handle_perceive_metadata(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let metadata = Perceiver::metadata(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&metadata).unwrap_or_default())
    }

    async fn handle_perceive_mutations(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        // Check if already running
        let already_running = self
            .sessions
            .with_session(&session_id, |session| {
                Ok(session.is_mutation_observer_running())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        if already_running {
            return Ok(serde_json::json!({
                "status": "already_running",
                "sessionId": session_id
            }));
        }

        // Get the page
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Set up the mutation observer
        let (rx, handle) = setup_mutation_observer(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        // Store the observer handle in the session
        self.sessions
            .with_session(&session_id, |session| {
                session.start_mutation_observer(rx, handle);
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        info!(session_id = %session_id, "Mutation observer started");

        Ok(serde_json::json!({
            "status": "started",
            "sessionId": session_id,
            "message": "Mutation events will be streamed as server events"
        }))
    }

    async fn handle_perceive_mutations_poll(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        // Maximum mutations to return per poll
        let limit = request
            .params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(100);

        // Collect mutations from the receiver
        let mut mutations = Vec::new();

        self.sessions
            .with_session(&session_id, |session| {
                if let Some(rx) = session.mutation_rx.as_mut() {
                    // Non-blocking poll for available mutations
                    while mutations.len() < limit {
                        match rx.try_recv() {
                            Ok(event) => mutations.push(event),
                            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                                // Observer stopped
                                break;
                            }
                        }
                    }
                }
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "mutations": mutations,
            "count": mutations.len()
        }))
    }

    async fn handle_perceive_mutations_stop(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        // Get the page to cleanup JS side
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Stop the JavaScript observer
        stop_mutation_observer(&page)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string()))?;

        // Stop the session-side observer
        self.sessions
            .with_session(&session_id, |session| {
                session.stop_mutation_observer();
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        info!(session_id = %session_id, "Mutation observer stopped");

        Ok(serde_json::json!({
            "status": "stopped",
            "sessionId": session_id
        }))
    }

    // Action handlers

    async fn handle_act_click(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params)
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        let options: ClickOptions = request
            .params
            .get("options")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::click(&page, &target, &options)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_type(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let text = request
            .params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("text"))?;

        let target = Self::parse_action_target(&request.params);

        let options: TypeOptions = request
            .params
            .get("options")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::type_text(&page, text, target.as_ref(), &options)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_press(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let key = request
            .params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("key"))?;

        let modifiers: Vec<String> = request
            .params
            .get("modifiers")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::press(&page, key, &modifiers)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_scroll(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params);

        let direction: ScrollDirection = request
            .params
            .get("direction")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(ScrollDirection::Down);

        let amount: i32 = request
            .params
            .get("amount")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(100);

        let smooth: bool = request
            .params
            .get("smooth")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let options = ScrollOptions {
            direction,
            amount,
            smooth,
        };

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::scroll(&page, target.as_ref(), &options)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_hover(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params)
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::hover(&page, &target)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_focus(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params)
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::focus(&page, &target)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_select(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params)
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        let value = request
            .params
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("value"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::select(&page, &target, value)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_wait_for(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let condition: crate::act::WaitCondition = request
            .params
            .get("condition")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .ok_or_else(|| ProtocolError::missing_field("condition"))?;

        let timeout_ms: u64 = request
            .params
            .get("timeoutMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::wait_for(&page, &condition, timeout_ms)
            .await
            .map_err(|e| ProtocolError::new(crate::error::ErrorCode::ActionTimeout, e.to_string()))?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
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
