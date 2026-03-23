//! WebSocket server for Tivana protocol
//!
//! Handles client connections, message routing, and response correlation.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// How often to send WebSocket ping frames
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// If no pong is received within this duration, consider the connection stale
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);

use crate::act::{ActionTarget, Actor, BatchAction, ClickOptions, ScrollDirection, ScrollOptions, TypeOptions};
use crate::browser::{BrowserLaunchConfig, BrowserManager};
use crate::captcha::CaptchaSolver;
use crate::cli::Args;
use crate::error::{ProtocolError, TivanaError};
use crate::extension::{self, ExtensionManager};
use crate::network::NetworkManager;
use crate::perceive::{setup_mutation_observer, setup_page_events, stop_mutation_observer, stop_page_events, PageEvent, Perceiver, ScreenshotOptions};
use crate::persistence;
use crate::protocol::{
    parse_request, serialize_outbound, EventMessage, OutboundMessage, ResponseMessage,
    PROTOCOL_VERSION,
};
use crate::proxy::{ProxyConfig, ProxyPool};
use crate::session::{SessionConfig, SessionRegistry};

/// WebSocket server
pub struct Server {
    /// Server address
    addr: SocketAddr,

    /// Session registry
    sessions: SessionRegistry,

    /// Browser manager
    browser_manager: BrowserManager,

    /// Connect to existing Chrome instance (port or ws:// URL)
    connect_target: Option<String>,

    /// Use the default browser profile (highest reCAPTCHA trust)
    use_default_browser: bool,

    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,

    /// Extension manager for Chrome extension bridge connections
    extension_manager: ExtensionManager,
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
            user_data_dir: args.user_data_dir.clone(),
            ..Default::default()
        };

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            addr,
            sessions: SessionRegistry::new(),
            browser_manager: BrowserManager::new(browser_config),
            connect_target: args.connect.clone(),
            use_default_browser: args.use_default_browser,
            shutdown_tx,
            extension_manager: ExtensionManager::new(),
        })
    }

    /// Attempt to reattach persisted sessions (only in --connect or --use-default-browser mode)
    async fn reattach_sessions(&self) {
        if self.connect_target.is_none() && !self.use_default_browser {
            return;
        }

        let persisted = persistence::get_active_persisted_sessions();
        if persisted.is_empty() {
            return;
        }

        info!(
            count = persisted.len(),
            "Attempting to reattach persisted sessions"
        );

        let connect_target = self.connect_target.as_deref().unwrap_or("9222");

        for ps in &persisted {
            // Try to connect to Chrome and verify the targets still exist
            match self.browser_manager.connect_existing(connect_target).await {
                Ok(browser) => {
                    // Check if any of the persisted targets are still alive
                    match browser.list_tabs().await {
                        Ok(tabs) => {
                            let live_target_ids: Vec<String> =
                                tabs.iter().map(|t| t.target_id.clone()).collect();

                            let has_match = ps
                                .target_ids
                                .iter()
                                .any(|tid| live_target_ids.contains(tid));

                            if has_match {
                                // Recreate the session with the existing browser
                                let config = SessionConfig {
                                    initial_url: None,
                                    headless: ps.headless,
                                    viewport_width: None,
                                    viewport_height: None,
                                    proxy: None,
                                };

                                let session_id =
                                    self.sessions.create_with_id(ps.session_id.clone(), config).await;

                                if let Err(e) = self.sessions.start_launch(&session_id).await {
                                    warn!(session_id = %session_id, error = %e, "Failed to start reattach launch");
                                    persistence::mark_session_stale(&session_id);
                                    continue;
                                }

                                if let Err(e) =
                                    self.sessions.complete_launch(&session_id, browser).await
                                {
                                    warn!(session_id = %session_id, error = %e, "Failed to complete reattach");
                                    persistence::mark_session_stale(&session_id);
                                    continue;
                                }

                                info!(session_id = %session_id, "Session reattached successfully");
                            } else {
                                info!(session_id = %ps.session_id, "No matching targets found, marking stale");
                                persistence::mark_session_stale(&ps.session_id);
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to list tabs during reattach");
                            persistence::mark_session_stale(&ps.session_id);
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        session_id = %ps.session_id,
                        error = %e,
                        "Failed to connect to Chrome for reattach"
                    );
                    persistence::mark_session_stale(&ps.session_id);
                }
            }
        }
    }

    /// Run the server
    pub async fn run(self) -> Result<(), TivanaError> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!(addr = %self.addr, "WebSocket server listening");

        // Wrap self in Arc for sharing across connections
        let server = Arc::new(self);

        // Attempt to reattach persisted sessions in --connect mode
        server.reattach_sessions().await;

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
                    // Clear persisted sessions on clean shutdown
                    persistence::clear_all();
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
        let (write, mut read) = ws_stream.split();

        // Wrap the writer in Arc<Mutex> so heartbeat task can also send pings
        let write = Arc::new(Mutex::new(write));

        // Channel for sending outbound messages (responses + events)
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(256);

        // Send hello event
        let hello = EventMessage::new(
            "server.hello",
            serde_json::json!({
                "version": PROTOCOL_VERSION,
                "capabilities": ["session", "browser", "perceive", "act"]
            }),
        );
        let hello_msg = serialize_outbound(&OutboundMessage::Event(hello))?;
        write.lock().await.send(Message::Text(hello_msg)).await?;

        // Track last pong received
        let last_pong = Arc::new(Mutex::new(Instant::now()));

        // Writer task: forwards outbound channel messages to WebSocket
        let write_clone = Arc::clone(&write);
        let writer_task = tokio::spawn(async move {
            while let Some(msg) = outbound_rx.recv().await {
                if write_clone.lock().await.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Heartbeat task: sends pings every HEARTBEAT_INTERVAL, closes if pong is stale
        let write_hb = Arc::clone(&write);
        let last_pong_hb = Arc::clone(&last_pong);
        let hb_addr = addr;
        let heartbeat_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
            loop {
                interval.tick().await;

                // Check if last pong is too old
                let elapsed = last_pong_hb.lock().await.elapsed();
                if elapsed > HEARTBEAT_TIMEOUT {
                    warn!(peer = %hb_addr, elapsed_secs = elapsed.as_secs(), "Connection stale (no pong), closing");
                    let _ = write_hb.lock().await.send(Message::Close(None)).await;
                    break;
                }

                // Send ping
                if write_hb.lock().await.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
                debug!(peer = %hb_addr, "Sent ping");
            }
        });

        // Track whether this connection is an extension
        let mut is_extension_conn = false;

        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    debug!(peer = %addr, len = text.len(), "Received message");

                    // Detect extension connections on first message
                    if !is_extension_conn && extension::is_extension_message(&text) {
                        info!(peer = %addr, "Detected Chrome extension connection");
                        is_extension_conn = true;
                        self.extension_manager.set_connection(outbound_tx.clone()).await;
                    }

                    if is_extension_conn {
                        // Handle extension protocol messages
                        self.handle_extension_message(&text).await;
                    } else {
                        // Normal SDK client message
                        let response = self.handle_message(&text).await;
                        let response_json = serialize_outbound(&response)?;
                        if outbound_tx.send(response_json).await.is_err() {
                            break;
                        }

                        // If this was a mutations/observe start request, kick off event pushing
                        if let Ok(req) = parse_request(&text) {
                            let sid = req.session_id.clone().or_else(|| {
                                req.params
                                    .get("sessionId")
                                    .and_then(|v| v.as_str())
                                    .map(String::from)
                            });

                            if req.method == "perceive.mutations" {
                                if let Some(session_id) = sid {
                                    let tx = outbound_tx.clone();
                                    let sessions = self.sessions.clone();
                                    tokio::spawn(async move {
                                        Self::push_mutation_events(sessions, session_id, tx).await;
                                    });
                                }
                            } else if req.method == "perceive.observe" {
                                if let Some(session_id) = sid {
                                    // Push mutation events
                                    let tx1 = outbound_tx.clone();
                                    let sessions1 = self.sessions.clone();
                                    let sid1 = session_id.clone();
                                    tokio::spawn(async move {
                                        Self::push_mutation_events(sessions1, sid1, tx1).await;
                                    });
                                    // Push page events
                                    let tx2 = outbound_tx.clone();
                                    let sessions2 = self.sessions.clone();
                                    tokio::spawn(async move {
                                        Self::push_page_events(sessions2, session_id, tx2).await;
                                    });
                                }
                            }
                        }
                    }
                }
                Ok(Message::Binary(data)) => {
                    warn!(peer = %addr, len = data.len(), "Received binary (not supported)");
                }
                Ok(Message::Ping(data)) => {
                    // Respond with pong
                    let _ = write.lock().await.send(Message::Pong(data)).await;
                }
                Ok(Message::Pong(_)) => {
                    // Update last pong timestamp
                    *last_pong.lock().await = Instant::now();
                    debug!(peer = %addr, "Received pong");
                }
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

        // Cleanup
        if is_extension_conn {
            self.extension_manager.clear_connection().await;
        }
        heartbeat_task.abort();
        drop(outbound_tx);
        let _ = writer_task.await;

        Ok(())
    }

    /// Push mutation events from a session's observer to the WebSocket client
    async fn push_mutation_events(
        sessions: SessionRegistry,
        session_id: String,
        tx: mpsc::Sender<String>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            let mut mutations = Vec::new();

            let result = sessions
                .with_session(&session_id, |session| {
                    if let Some(rx) = session.mutation_rx.as_mut() {
                        while mutations.len() < 50 {
                            match rx.try_recv() {
                                Ok(event) => mutations.push(event),
                                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                                    return Err(crate::error::TivanaError::Session(
                                        "Observer disconnected".to_string(),
                                    ));
                                }
                            }
                        }
                    } else {
                        return Err(crate::error::TivanaError::Session(
                            "No observer".to_string(),
                        ));
                    }
                    Ok(())
                })
                .await;

            // Stop if session closed or observer gone
            if result.is_err() {
                debug!(session_id = %session_id, "Stopping mutation event push");
                break;
            }

            if !mutations.is_empty() {
                let event = EventMessage::for_session(
                    &session_id,
                    "page.mutation",
                    serde_json::to_value(&mutations).unwrap_or_default(),
                );
                let msg = match serialize_outbound(&OutboundMessage::Event(event)) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if tx.send(msg).await.is_err() {
                    break; // Client disconnected
                }
            }
        }
    }

    /// Push page events from a session's page event receiver to the WebSocket client
    async fn push_page_events(
        sessions: SessionRegistry,
        session_id: String,
        tx: mpsc::Sender<String>,
    ) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            let mut events: Vec<PageEvent> = Vec::new();

            let result = sessions
                .with_session(&session_id, |session| {
                    if let Some(rx) = session.page_event_rx.as_mut() {
                        while events.len() < 50 {
                            match rx.try_recv() {
                                Ok(event) => events.push(event),
                                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                                    return Err(crate::error::TivanaError::Session(
                                        "Page event observer disconnected".to_string(),
                                    ));
                                }
                            }
                        }
                    } else {
                        return Err(crate::error::TivanaError::Session(
                            "No page event observer".to_string(),
                        ));
                    }
                    Ok(())
                })
                .await;

            if result.is_err() {
                debug!(session_id = %session_id, "Stopping page event push");
                break;
            }

            // Send each page event as its own event message with the appropriate event name
            for page_event in events {
                let event_name = match &page_event {
                    PageEvent::Loaded { .. } => "page.loaded",
                    PageEvent::Navigated { .. } => "page.navigated",
                    PageEvent::Focus { .. } => "page.focus",
                    PageEvent::Scroll { .. } => "page.scroll",
                    PageEvent::Resize { .. } => "page.resize",
                };
                let event = EventMessage::for_session(
                    &session_id,
                    event_name,
                    serde_json::to_value(&page_event).unwrap_or_default(),
                );
                let msg = match serialize_outbound(&OutboundMessage::Event(event)) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if tx.send(msg).await.is_err() {
                    return; // Client disconnected
                }
            }
        }
    }

    /// Handle a message from the Chrome extension
    async fn handle_extension_message(&self, text: &str) {
        let msg: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return,
        };

        let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");

        match method {
            "tab.attached" => {
                if let Some(params) = msg.get("params") {
                    self.extension_manager.handle_tab_attached(params).await;
                }
            }
            "tab.detached" => {
                if let Some(params) = msg.get("params") {
                    self.extension_manager.handle_tab_detached(params).await;
                }
            }
            "tab.navigated" => {
                if let Some(params) = msg.get("params") {
                    self.extension_manager.handle_tab_navigated(params).await;
                }
            }
            "cdp.event" => {
                // CDP events from extension — could forward to SDK clients in the future
                debug!("Extension CDP event: {:?}", msg.get("params"));
            }
            "extension.hello" => {
                info!("Extension handshake received");
            }
            "tab.error" => {
                if let Some(params) = msg.get("params") {
                    warn!("Extension tab error: {:?}", params);
                }
            }
            "pong" => {
                // Keepalive response, nothing to do
            }
            "" => {
                // Likely a CDP response: {id, result} or {id, error}
                self.extension_manager.handle_cdp_response(&msg).await;
            }
            _ => {
                debug!("Unknown extension message method: {}", method);
            }
        }
    }

    /// Handle a single message and return response
    async fn handle_message(&self, text: &str) -> OutboundMessage {
        match parse_request(text) {
            Ok(request) => self.route_request(request).await.into(),
            Err(e) => ResponseMessage::error("unknown", e).into(),
        }
    }

    /// Route request to appropriate handler
    async fn route_request(&self, request: crate::protocol::RequestMessage) -> ResponseMessage {
        let id = request.id.clone();

        // Check if this request targets an extension-backed session
        // Skip interception for session management methods
        let skip_extension_check = matches!(
            request.method.as_str(),
            "session.create" | "session.fromExtension" | "session.list" | "extension.tabs"
        );
        let maybe_session_id = request.session_id.as_deref()
            .or_else(|| request.params.get("sessionId").and_then(|v| v.as_str()));
        if !skip_extension_check {
            if let Some(session_id) = maybe_session_id {
                if self.extension_manager.is_extension_session(session_id).await {
                    let ext_result = self.route_extension_request(&request).await;
                    return match ext_result {
                        Ok(data) => ResponseMessage::success(id, data),
                        Err(e) => ResponseMessage::error(id, e),
                    };
                }
            }
        }

        let result = match request.method.as_str() {
            // Session methods
            "session.create" => self.handle_session_create(&request).await,
            "session.close" => self.handle_session_close(&request).await,
            "session.list" => self.handle_session_list().await,
            "session.tabs" => self.handle_session_tabs(&request).await,
            "session.switchTab" => self.handle_session_switch_tab(&request).await,
            "session.newTab" => self.handle_session_new_tab(&request).await,
            "session.closeTab" => self.handle_session_close_tab(&request).await,
            "session.get" => self.handle_session_get(&request).await,
            "session.cleanTabs" => self.handle_session_clean_tabs(&request).await,

            // Extension methods
            "session.fromExtension" => self.handle_session_from_extension(&request).await,
            "extension.tabs" => self.handle_extension_list_tabs().await,

            // Browser methods
            "browser.navigate" => self.handle_browser_navigate(&request).await,
            "browser.url" => self.handle_browser_url(&request).await,

            // Perception methods
            "perceive.pageState" => self.handle_perceive_page_state(&request).await,
            "perceive.elements" => self.handle_perceive_elements(&request).await,
            "perceive.accessibility" | "perceive.accessibilitySnapshot" => {
                self.handle_perceive_accessibility(&request).await
            }
            "perceive.text" | "perceive.textContent" => self.handle_perceive_text(&request).await,
            "perceive.metadata" => self.handle_perceive_metadata(&request).await,
            "perceive.findElements" => self.handle_perceive_find_elements(&request).await,
            "perceive.mutations" => self.handle_perceive_mutations(&request).await,
            "perceive.mutations.poll" => self.handle_perceive_mutations_poll(&request).await,
            "perceive.mutations.stop" => self.handle_perceive_mutations_stop(&request).await,
            "perceive.observe" => self.handle_perceive_observe(&request).await,
            "perceive.unobserve" => self.handle_perceive_unobserve(&request).await,
            "perceive.formFields" => self.handle_perceive_form_fields(&request).await,
            "perceive.evaluate" => self.handle_perceive_evaluate(&request).await,
            "perceive.evaluateVoid" => self.handle_perceive_evaluate_void(&request).await,

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
            "act.waitForSelector" => self.handle_act_wait_for_selector(&request).await,
            "act.waitForNavigation" => self.handle_act_wait_for_navigation(&request).await,
            "act.waitForFunction" => self.handle_act_wait_for_function(&request).await,
            "act.batch" => self.handle_act_batch(&request).await,
            "act.fillForm" => self.handle_act_fill_form(&request).await,
            "act.smartFill" => self.handle_act_smart_fill(&request).await,

            // Screenshot
            "perceive.screenshot" => self.handle_perceive_screenshot(&request).await,

            // Network monitoring
            "network.enable" => self.handle_network_enable(&request).await,
            "network.requests" => self.handle_network_requests(&request).await,
            "network.clear" => self.handle_network_clear(&request).await,

            // Dialog handling
            "act.handleDialog" => self.handle_act_dialog(&request).await,

            // File upload
            "act.uploadFile" => self.handle_act_upload_file(&request).await,

            // Storage methods
            "storage.getCookies" => self.handle_storage_get_cookies(&request).await,
            "storage.setCookie" => self.handle_storage_set_cookie(&request).await,
            "storage.clearCookies" => self.handle_storage_clear_cookies(&request).await,
            "storage.getLocalStorage" => self.handle_storage_get_local_storage(&request).await,
            "storage.setLocalStorage" => self.handle_storage_set_local_storage(&request).await,
            "storage.getSessionStorage" => self.handle_storage_get_session_storage(&request).await,
            "storage.setSessionStorage" => self.handle_storage_set_session_storage(&request).await,
            "storage.clear" => self.handle_storage_clear(&request).await,

            // CAPTCHA methods
            "captcha.detect" => self.handle_captcha_detect(&request).await,
            "captcha.solve" => self.handle_captcha_solve(&request).await,

            // Proxy methods
            "proxy.set" => self.handle_proxy_set(&request).await,
            "proxy.pool" => self.handle_proxy_pool(&request).await,
            "proxy.rotate" => self.handle_proxy_rotate(&request).await,
            "proxy.current" => self.handle_proxy_current(&request).await,

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
            element_id: target
                .get("elementId")
                .and_then(|v| v.as_str())
                .map(String::from),
            selector: target
                .get("selector")
                .and_then(|v| v.as_str())
                .map(String::from),
            text: target
                .get("text")
                .and_then(|v| v.as_str())
                .map(String::from),
            role: target
                .get("role")
                .and_then(|v| v.as_str())
                .map(String::from),
            label: target
                .get("label")
                .and_then(|v| v.as_str())
                .map(String::from),
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
        // Parse optional proxy config from params
        let proxy: Option<ProxyConfig> = request
            .params
            .get("proxy")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

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
                .unwrap_or(self.browser_manager.default_headless()),
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
            proxy,
        };

        let session_id = self.sessions.create(config.clone()).await;

        // Start browser launch
        self.sessions
            .start_launch(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Either connect to existing Chrome, use default browser, or launch a new one
        let browser = if self.use_default_browser {
            self.browser_manager
                .launch_default_browser(9222)
                .await
                .map_err(|e| ProtocolError::browser_launch_failed(e.to_string()))?
        } else if let Some(ref target) = self.connect_target {
            self.browser_manager
                .connect_existing(target)
                .await
                .map_err(|e| ProtocolError::browser_launch_failed(e.to_string()))?
        } else {
            let browser_config = BrowserLaunchConfig {
                headless: config.headless,
                viewport_width: config.viewport_width.unwrap_or(1440),
                viewport_height: config.viewport_height.unwrap_or(900),
                user_data_dir: self.browser_manager.default_config().user_data_dir.clone(),
                ..Default::default()
            };
            self.browser_manager
                .launch(Some(browser_config), config.proxy.as_ref())
                .await
                .map_err(|e| ProtocolError::browser_launch_failed(e.to_string()))?
        };

        // Complete launch
        self.sessions
            .complete_launch(&session_id, browser)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Note: auto-clean removed — in --connect mode, the session's page starts as about:blank
        // and would be erroneously cleaned. Users can call session.cleanTabs explicitly.

        // Navigate to initial URL if specified
        if let Some(url) = config.initial_url {
            let page = self
                .sessions
                .get_page(&session_id)
                .await
                .map_err(|e| ProtocolError::internal(e.to_string()))?;

            Actor::navigate(&page, &url).await.map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::NavigationFailed, e.to_string())
            })?;
        }

        let info = self.sessions.get(&session_id).await.unwrap();

        // Persist session state to disk
        let target_ids = self
            .sessions
            .list_tabs(&session_id)
            .await
            .map(|tabs| tabs.iter().map(|t| t.target_id.clone()).collect())
            .unwrap_or_default();
        let page_urls = self
            .sessions
            .list_tabs(&session_id)
            .await
            .map(|tabs| tabs.iter().map(|t| t.url.clone()).collect())
            .unwrap_or_default();
        persistence::persist_session_create(
            &session_id,
            config.headless,
            target_ids,
            page_urls,
        );

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

        // Remove from persistence
        persistence::persist_session_close(&session_id);

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

    async fn handle_session_clean_tabs(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let closed = self.clean_blank_tabs(&session_id).await?;
        Ok(serde_json::json!({ "closed": closed }))
    }

    /// Close all about:blank tabs except the active one. Returns count of closed tabs.
    async fn clean_blank_tabs(&self, session_id: &str) -> Result<usize, ProtocolError> {
        let tabs = self
            .sessions
            .list_tabs(session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let mut closed = 0usize;
        for tab in &tabs {
            if tab.url == "about:blank" && !tab.active {
                if let Err(e) = self.sessions.close_tab(session_id, &tab.target_id).await {
                    warn!(target_id = %tab.target_id, error = %e, "Failed to close blank tab");
                } else {
                    closed += 1;
                }
            }
        }

        if closed > 0 {
            debug!(session_id = %session_id, closed, "Cleaned orphaned about:blank tabs");
        }

        Ok(closed)
    }

    // Extension-backed session routing
    async fn route_extension_request(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = request.session_id.as_deref()
            .or_else(|| request.params.get("sessionId").and_then(|v| v.as_str()))
            .ok_or_else(|| ProtocolError::missing_field("sessionId"))?;

        match request.method.as_str() {
            "perceive.pageState" => {
                let state = self.extension_manager.page_state(session_id).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(state)
            }
            "perceive.evaluate" => {
                let expression = request.params.get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::missing_field("expression"))?;
                let result = self.extension_manager.evaluate(session_id, expression).await
                    .map_err(|e| ProtocolError::internal(e))?;
                // Extract the value from Runtime.evaluate result
                if let Some(val) = result.get("result").and_then(|r| r.get("value")) {
                    Ok(serde_json::json!({ "result": val }))
                } else {
                    Ok(serde_json::json!({ "result": result }))
                }
            }
            "perceive.evaluateVoid" => {
                let expression = request.params.get("expression")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::missing_field("expression"))?;
                self.extension_manager.evaluate(session_id, expression).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(serde_json::json!({ "ok": true }))
            }
            "perceive.elements" => {
                // Run the perception script via evaluate
                let result = self.extension_manager.evaluate(session_id, 
                    &crate::perceive::elements_script()
                ).await.map_err(|e| ProtocolError::internal(e))?;
                if let Some(val) = result.get("result").and_then(|r| r.get("value")) {
                    if let Some(s) = val.as_str() {
                        let parsed: serde_json::Value = serde_json::from_str(s)
                            .map_err(|e| ProtocolError::internal(e.to_string()))?;
                        return Ok(parsed);
                    }
                    return Ok(val.clone());
                }
                Ok(serde_json::json!([]))
            }
            "perceive.text" | "perceive.textContent" => {
                let result = self.extension_manager.evaluate(session_id, "document.body?.innerText || ''").await
                    .map_err(|e| ProtocolError::internal(e))?;
                if let Some(val) = result.get("result").and_then(|r| r.get("value")) {
                    Ok(serde_json::json!({ "text": val }))
                } else {
                    Ok(serde_json::json!({ "text": "" }))
                }
            }
            "act.navigate" | "browser.navigate" => {
                let url = request.params.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::missing_field("url"))?;
                let _result = self.extension_manager.navigate(session_id, url).await
                    .map_err(|e| ProtocolError::internal(e))?;
                
                // Try page state with retries — debugger may need time after navigation
                for attempt in 0..3u32 {
                    let state = tokio::time::timeout(
                        std::time::Duration::from_millis(5000),
                        self.extension_manager.page_state(session_id)
                    ).await;
                    if let Ok(Ok(s)) = state {
                        return Ok(s);
                    }
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    }
                }
                // Give up on page state, return minimal info
                Ok(serde_json::json!({"url": url, "title": "unknown", "navigated": true}))
            }
            "act.click" => {
                let target = request.params.get("target")
                    .and_then(|v| v.as_str())
                    .or_else(|| request.params.get("elementId").and_then(|v| v.as_str()));
                
                if let Some(target) = target {
                    // Resolve element bounds via JS, then click
                    let script = format!(
                        r#"(function() {{
                            var el = document.querySelector('[data-tivana-id="{}"]');
                            if (!el) {{
                                // Try by element ID prefix (e.g. "e5")
                                var id = '{}';
                                var allEls = document.querySelectorAll('[data-tivana-id]');
                                for (var e of allEls) {{
                                    if (e.getAttribute('data-tivana-id') === id) {{ el = e; break; }}
                                }}
                            }}
                            if (!el) return JSON.stringify({{error: 'Element not found'}});
                            el.scrollIntoView({{block: 'center'}});
                            var rect = el.getBoundingClientRect();
                            return JSON.stringify({{x: rect.x + rect.width/2, y: rect.y + rect.height/2}});
                        }})()"#,
                        target, target
                    );
                    let result = self.extension_manager.evaluate(session_id, &script).await
                        .map_err(|e| ProtocolError::internal(e))?;
                    
                    if let Some(val) = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()) {
                        let coords: serde_json::Value = serde_json::from_str(val)
                            .map_err(|e| ProtocolError::internal(e.to_string()))?;
                        if coords.get("error").is_some() {
                            return Err(ProtocolError::new(
                                crate::error::ErrorCode::ActionFailed,
                                format!("Click failed: {}", coords["error"]),
                            ));
                        }
                        let x = coords["x"].as_f64().unwrap_or(0.0);
                        let y = coords["y"].as_f64().unwrap_or(0.0);
                        self.extension_manager.click(session_id, x, y).await
                            .map_err(|e| ProtocolError::internal(e))?;
                        return Ok(serde_json::json!({ "clicked": true }));
                    }
                }
                
                // Fallback: click by coordinates
                let x = request.params.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = request.params.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                self.extension_manager.click(session_id, x, y).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(serde_json::json!({ "clicked": true }))
            }
            "act.type" => {
                let text = request.params.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProtocolError::missing_field("text"))?;
                
                // Click target first if specified
                if let Some(target) = request.params.get("target").and_then(|v| v.as_str()) {
                    // Focus and click the target element via JS
                    let script = format!(
                        r#"(function() {{
                            var el = document.querySelector('[data-tivana-id="{}"]');
                            if (el) {{ el.scrollIntoView({{block:'center'}}); el.focus(); el.click(); return 'ok'; }}
                            return 'not found';
                        }})()"#,
                        target
                    );
                    self.extension_manager.evaluate(session_id, &script).await
                        .map_err(|e| ProtocolError::internal(e))?;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                
                self.extension_manager.type_text(session_id, text).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(serde_json::json!({ "typed": true }))
            }
            "act.scroll" => {
                let ext_session_id = self.extension_manager.get_extension_session_id(session_id).await
                    .ok_or_else(|| ProtocolError::internal("No extension session mapping".to_string()))?;
                let dx = request.params.get("deltaX").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let dy = request.params.get("deltaY").and_then(|v| v.as_f64()).unwrap_or(300.0);
                self.extension_manager.send_cdp_command(&ext_session_id, "Input.dispatchMouseEvent", serde_json::json!({
                    "type": "mouseWheel",
                    "x": 400, "y": 400,
                    "deltaX": dx, "deltaY": dy,
                })).await.map_err(|e| ProtocolError::internal(e))?;
                Ok(serde_json::json!({ "scrolled": true }))
            }
            "perceive.screenshot" | "screenshot.capture" => {
                let ext_session_id = self.extension_manager.get_extension_session_id(session_id).await
                    .ok_or_else(|| ProtocolError::internal("No extension session mapping".to_string()))?;
                let result = self.extension_manager.send_cdp_command(&ext_session_id, "Page.captureScreenshot", serde_json::json!({
                    "format": "png",
                })).await.map_err(|e| ProtocolError::internal(e))?;
                Ok(result)
            }
            "session.close" => {
                // Just remove the mapping, don't close the tab
                Ok(serde_json::json!({ "closed": true }))
            }
            "session.tabs" => {
                // Extension only has one tab
                let tabs = self.extension_manager.list_tabs().await;
                Ok(serde_json::json!(tabs))
            }
            "session.switchTab" | "session.newTab" | "session.closeTab" | "session.cleanTabs" => {
                // Not applicable for extension sessions
                Ok(serde_json::json!({ "ok": true }))
            }
            "perceive.formFields" => {
                let result = self.extension_manager.evaluate(session_id, 
                    &crate::perceive::elements_script()
                ).await.map_err(|e| ProtocolError::internal(e))?;
                if let Some(val) = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()) {
                    let parsed: serde_json::Value = serde_json::from_str(val)
                        .map_err(|e| ProtocolError::internal(e.to_string()))?;
                    return Ok(parsed);
                }
                Ok(serde_json::json!([]))
            }
            "act.press" => {
                let ext_session_id = self.extension_manager.get_extension_session_id(session_id).await
                    .ok_or_else(|| ProtocolError::internal("No extension session mapping".to_string()))?;
                let key = request.params.get("key").and_then(|v| v.as_str()).unwrap_or("Enter");
                self.extension_manager.send_cdp_command(&ext_session_id, "Input.dispatchKeyEvent", serde_json::json!({
                    "type": "rawKeyDown",
                    "key": key,
                    "code": format!("Key{}", key.chars().next().unwrap_or('A').to_uppercase()),
                    "windowsVirtualKeyCode": match key { "Enter" => 13, "Tab" => 9, "Escape" => 27, "Backspace" => 8, _ => 0 },
                })).await.map_err(|e| ProtocolError::internal(e))?;
                self.extension_manager.send_cdp_command(&ext_session_id, "Input.dispatchKeyEvent", serde_json::json!({
                    "type": "keyUp",
                    "key": key,
                })).await.map_err(|e| ProtocolError::internal(e))?;
                Ok(serde_json::json!({ "pressed": true }))
            }
            "act.smartFill" | "act.fillForm" | "act.batch" => {
                // Not yet supported for extension sessions — use evaluate as workaround
                Ok(serde_json::json!({ "ok": true, "note": "Use evaluate for form operations in extension mode" }))
            }
            "act.waitForSelector" | "act.waitForNavigation" | "act.waitForFunction" => {
                // Simple wait
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                Ok(serde_json::json!({ "ok": true }))
            }
            "act.uploadFile" => {
                Ok(serde_json::json!({ "ok": true, "note": "File upload not supported in extension mode" }))
            }
            "perceive.accessibility" | "perceive.accessibilitySnapshot" => {
                // Return elements as accessibility tree
                let result = self.extension_manager.evaluate(session_id, 
                    &crate::perceive::elements_script()
                ).await.map_err(|e| ProtocolError::internal(e))?;
                if let Some(val) = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()) {
                    let parsed: serde_json::Value = serde_json::from_str(val)
                        .map_err(|e| ProtocolError::internal(e.to_string()))?;
                    return Ok(parsed);
                }
                Ok(serde_json::json!([]))
            }
            "perceive.metadata" => {
                let state = self.extension_manager.page_state(session_id).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(state)
            }
            "perceive.screenshot" => {
                let ext_session_id = self.extension_manager.get_extension_session_id(session_id).await
                    .ok_or_else(|| ProtocolError::internal("No extension session mapping".to_string()))?;
                let result = self.extension_manager.send_cdp_command(&ext_session_id, "Page.captureScreenshot", serde_json::json!({
                    "format": "png",
                })).await.map_err(|e| ProtocolError::internal(e))?;
                Ok(result)
            }
            "browser.navigate" | "browser.url" => {
                let url = request.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.is_empty() {
                    let state = self.extension_manager.page_state(session_id).await
                        .map_err(|e| ProtocolError::internal(e))?;
                    return Ok(state);
                }
                self.extension_manager.navigate(session_id, url).await
                    .map_err(|e| ProtocolError::internal(e))?;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let state = self.extension_manager.page_state(session_id).await
                    .map_err(|e| ProtocolError::internal(e))?;
                Ok(state)
            }
            "act.hover" | "act.focus" | "act.select" | "act.waitFor" => {
                Ok(serde_json::json!({ "ok": true }))
            }
            "captcha.detect" | "captcha.solve" => {
                Ok(serde_json::json!({ "ok": true, "note": "Use evaluate for CAPTCHA in extension mode" }))
            }
            "network.enable" | "network.clear" | "network.requests" => {
                Ok(serde_json::json!({ "ok": true }))
            }
            "storage.getCookies" | "storage.setCookie" | "storage.clearCookies" 
            | "storage.getLocalStorage" | "storage.setLocalStorage" 
            | "storage.getSessionStorage" | "storage.setSessionStorage" | "storage.clear" => {
                Ok(serde_json::json!({ "ok": true }))
            }
            _ => {
                Err(ProtocolError::new(
                    crate::error::ErrorCode::UnknownMethod,
                    format!("Method '{}' not yet supported for extension sessions", request.method),
                ))
            }
        }
    }

    // Extension handlers

    async fn handle_session_from_extension(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        // List available extension tabs and optionally pick one by sessionId
        let ext_session_id = request
            .params
            .get("extensionSessionId")
            .and_then(|v| v.as_str());

        let tabs = self.extension_manager.list_tabs().await;
        if tabs.is_empty() {
            return Err(ProtocolError::new(
                crate::error::ErrorCode::BrowserDisconnected,
                "No extension tabs available. Make sure the Chrome extension is connected and a tab is attached.",
            ));
        }

        // Pick the requested tab, or the first one
        let tab = if let Some(ext_sid) = ext_session_id {
            tabs.iter()
                .find(|t| t.session_id == ext_sid)
                .ok_or_else(|| {
                    ProtocolError::session_not_found(ext_sid)
                })?
                .clone()
        } else {
            tabs.into_iter().next().unwrap()
        };

        // Create a Tivana session ID and map it to the extension session
        let session_id = uuid::Uuid::new_v4().to_string();
        self.extension_manager.register_session_mapping(
            &session_id,
            &tab.session_id,
        ).await;

        info!(
            session_id = %session_id,
            extension_session = %tab.session_id,
            url = %tab.url,
            "Created extension-backed session"
        );

        Ok(serde_json::json!({
            "sessionId": session_id,
            "extensionSessionId": tab.session_id,
            "tabId": tab.tab_id,
            "targetId": tab.target_id,
            "url": tab.url,
            "title": tab.title,
            "connected": self.extension_manager.is_connected().await,
        }))
    }

    async fn handle_extension_list_tabs(&self) -> Result<serde_json::Value, ProtocolError> {
        let tabs = self.extension_manager.list_tabs().await;
        Ok(serde_json::json!({
            "tabs": tabs,
            "connected": self.extension_manager.is_connected().await,
        }))
    }

    // Tab management handlers

    async fn handle_session_tabs(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let tabs = self
            .sessions
            .list_tabs(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "tabs": tabs,
            "count": tabs.len()
        }))
    }

    async fn handle_session_switch_tab(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target_id = request
            .params
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("targetId"))?;

        let page = self
            .sessions
            .switch_tab(&session_id, target_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let url = page.url().await.unwrap_or_default();
        let title = page.title().await.unwrap_or_default();

        Ok(serde_json::json!({
            "targetId": target_id,
            "url": url,
            "title": title
        }))
    }

    async fn handle_session_new_tab(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let url = request.params.get("url").and_then(|v| v.as_str());

        let (page, target_id) = self
            .sessions
            .open_tab(&session_id, url)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let final_url = page.url().await.unwrap_or_default();
        let title = page.title().await.unwrap_or_default();

        Ok(serde_json::json!({
            "targetId": target_id,
            "url": final_url,
            "title": title
        }))
    }

    async fn handle_session_close_tab(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target_id = request
            .params
            .get("targetId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("targetId"))?;

        self.sessions
            .close_tab(&session_id, target_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "closed": true,
            "targetId": target_id
        }))
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

        let result = Actor::navigate(&page, url).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::NavigationFailed, e.to_string())
        })?;

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

        let url = page
            .url()
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

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

        let state = Perceiver::page_state(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

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

        let elements = Perceiver::elements(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

        // Return array directly to match SDK expectation
        Ok(serde_json::to_value(&elements).unwrap_or_default())
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
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
            })?;

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

        let content = Perceiver::text_content(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

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

        let metadata = Perceiver::metadata(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

        Ok(serde_json::to_value(&metadata).unwrap_or_default())
    }

    async fn handle_perceive_find_elements(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let selector = request
            .params
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("selector"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let elements = Perceiver::find_elements(&page, selector)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
            })?;

        Ok(serde_json::to_value(&elements).unwrap_or_default())
    }

    async fn handle_perceive_form_fields(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let fields = Perceiver::form_fields(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

        Ok(serde_json::json!({ "fields": fields }))
    }

    async fn handle_perceive_evaluate(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let expression = request
            .params
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("expression"))?;

        let await_promise = request
            .params
            .get("awaitPromise")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let js = if await_promise {
            format!("(async () => {{ return {}; }})()", expression)
        } else {
            expression.to_string()
        };

        let timeout_ms = request
            .params
            .get("timeoutMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        let eval_future = page.evaluate::<serde_json::Value>(&js);
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            eval_future,
        )
        .await
        .map_err(|_| {
            ProtocolError::new(
                crate::error::ErrorCode::PerceptionFailed,
                format!("Evaluate timed out after {}ms", timeout_ms),
            )
        })?
        .map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::PerceptionFailed,
                format!("Evaluate failed: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "result": result }))
    }

    async fn handle_perceive_evaluate_void(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let expression = request
            .params
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("expression"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let timeout_ms = request
            .params
            .get("timeoutMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            page.evaluate_void(expression),
        )
        .await
        .map_err(|_| {
            ProtocolError::new(
                crate::error::ErrorCode::PerceptionFailed,
                format!("EvaluateVoid timed out after {}ms", timeout_ms),
            )
        })?
        .map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::PerceptionFailed,
                format!("Evaluate failed: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "success": true }))
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
        let (rx, handle) = setup_mutation_observer(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

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
        stop_mutation_observer(&page).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
        })?;

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

    /// Handle perceive.observe — starts both mutation stream AND page event injection
    async fn handle_perceive_observe(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        // Get the page
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Start mutation observer if not already running
        let mutation_already_running = self
            .sessions
            .with_session(&session_id, |session| {
                Ok(session.is_mutation_observer_running())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        if !mutation_already_running {
            let (rx, handle) = setup_mutation_observer(&page).await.map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
            })?;

            self.sessions
                .with_session(&session_id, |session| {
                    session.start_mutation_observer(rx, handle);
                    Ok(())
                })
                .await
                .map_err(|e| ProtocolError::internal(e.to_string()))?;
        }

        // Start page events if not already running
        let page_events_already_running = self
            .sessions
            .with_session(&session_id, |session| {
                Ok(session.is_page_events_running())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        if !page_events_already_running {
            let (rx, handle) = setup_page_events(&page).await.map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::PerceptionFailed, e.to_string())
            })?;

            self.sessions
                .with_session(&session_id, |session| {
                    session.start_page_events(rx, handle);
                    Ok(())
                })
                .await
                .map_err(|e| ProtocolError::internal(e.to_string()))?;
        }

        info!(session_id = %session_id, "Observation started (mutations + page events)");

        Ok(serde_json::json!({
            "status": "started",
            "sessionId": session_id,
            "message": "Mutation and page events will be streamed as server events"
        }))
    }

    /// Handle perceive.unobserve — stops both mutation stream AND page events
    async fn handle_perceive_unobserve(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Stop mutation observer
        let _ = stop_mutation_observer(&page).await;
        self.sessions
            .with_session(&session_id, |session| {
                session.stop_mutation_observer();
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Stop page events
        let _ = stop_page_events(&page).await;
        self.sessions
            .with_session(&session_id, |session| {
                session.stop_page_events();
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        info!(session_id = %session_id, "Observation stopped");

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

        let mouse_pos = self
            .sessions
            .get_mouse_position(&session_id)
            .await
            .ok();

        let result = Actor::click(&page, &target, &options, mouse_pos.as_ref())
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
            })?;

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

        let mouse_pos = self
            .sessions
            .get_mouse_position(&session_id)
            .await
            .ok();

        let result = Actor::type_text(&page, text, target.as_ref(), &options, mouse_pos.as_ref())
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
            })?;

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

        let result = Actor::press(&page, key, &modifiers).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
        })?;

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
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
            })?;

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

        let result = Actor::hover(&page, &target).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
        })?;

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

        let result = Actor::focus(&page, &target).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
        })?;

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

        let result = Actor::select(&page, &target, value).await.map_err(|e| {
            ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
        })?;

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
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionTimeout, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_wait_for_selector(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let selector = request
            .params
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("selector"))?;

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

        let result = Actor::wait_for_selector(&page, selector, timeout_ms)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionTimeout, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_wait_for_navigation(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

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

        let result = Actor::wait_for_navigation(&page, timeout_ms)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionTimeout, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_wait_for_function(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let expression = request
            .params
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("expression"))?;

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

        let result = Actor::wait_for_function(&page, expression, timeout_ms)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionTimeout, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // Batch action handler

    async fn handle_act_batch(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let actions: Vec<BatchAction> = request
            .params
            .get("actions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .ok_or_else(|| ProtocolError::missing_field("actions"))?;

        let stop_on_error: bool = request
            .params
            .get("stopOnError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let mouse_pos = self
            .sessions
            .get_mouse_position(&session_id)
            .await
            .ok();

        let result =
            Actor::execute_batch(&page, &actions, stop_on_error, mouse_pos.as_ref()).await;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // Form fill handler

    async fn handle_act_fill_form(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let fields = request
            .params
            .get("fields")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ProtocolError::missing_field("fields"))?;

        let submit = request
            .params
            .get("submit")
            .and_then(|v| v.as_str());

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let mouse_pos = self
            .sessions
            .get_mouse_position(&session_id)
            .await
            .ok();

        let result =
            Actor::fill_form(&page, fields, submit, mouse_pos.as_ref()).await;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    async fn handle_act_smart_fill(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let profile = request
            .params
            .get("profile")
            .and_then(|v| v.as_object())
            .ok_or_else(|| ProtocolError::missing_field("profile"))?;

        let skip_recaptcha = request
            .params
            .get("skipRecaptcha")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let mouse_pos = self
            .sessions
            .get_mouse_position(&session_id)
            .await
            .ok();

        let result =
            Actor::smart_fill(&page, profile, skip_recaptcha, mouse_pos.as_ref()).await;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // CAPTCHA handlers

    async fn handle_captcha_detect(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let info = CaptchaSolver::detect(&page).await?;

        Ok(serde_json::to_value(&info).unwrap_or_default())
    }

    async fn handle_captcha_solve(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = CaptchaSolver::solve(&page).await?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // =========================================================================
    // Screenshot handler
    // =========================================================================

    async fn handle_perceive_screenshot(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let options: ScreenshotOptions = serde_json::from_value(request.params.clone())
            .unwrap_or_default();

        let result = Perceiver::screenshot(&page, options)
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::PerceptionFailed,
                    format!("Screenshot failed: {}", e),
                )
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // =========================================================================
    // Network monitoring handlers
    // =========================================================================

    async fn handle_network_enable(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        NetworkManager::enable(&page).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::InternalError,
                format!("Network enable failed: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "enabled": true }))
    }

    async fn handle_network_requests(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let url_pattern = request
            .params
            .get("urlPattern")
            .and_then(|v| v.as_str());

        let requests = NetworkManager::get_requests(&page, url_pattern)
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::InternalError,
                    format!("Network requests failed: {}", e),
                )
            })?;

        Ok(serde_json::json!({
            "requests": requests,
            "count": requests.len(),
        }))
    }

    async fn handle_network_clear(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        NetworkManager::clear(&page).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::InternalError,
                format!("Network clear failed: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "cleared": true }))
    }

    // =========================================================================
    // Dialog handling (MOS-122)
    // =========================================================================

    async fn handle_act_dialog(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let action = request
            .params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("action"))?;

        let prompt_text = request
            .params
            .get("promptText")
            .and_then(|v| v.as_str());

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::handle_dialog(&page, action, prompt_text)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // =========================================================================
    // File upload (MOS-128)
    // =========================================================================

    async fn handle_act_upload_file(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let target = Self::parse_action_target(&request.params)
            .ok_or_else(|| ProtocolError::missing_field("target"))?;

        let file_paths: Vec<String> = request
            .params
            .get("filePaths")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .ok_or_else(|| ProtocolError::missing_field("filePaths"))?;

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let result = Actor::upload_file(&page, &target, &file_paths)
            .await
            .map_err(|e| {
                ProtocolError::new(crate::error::ErrorCode::ActionFailed, e.to_string())
            })?;

        Ok(serde_json::to_value(&result).unwrap_or_default())
    }

    // =========================================================================
    // Cookie & storage management (MOS-127)
    // =========================================================================

    async fn handle_storage_get_cookies(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let cookies: serde_json::Value = page
            .evaluate(
                r#"(() => {
                    return document.cookie.split('; ').filter(Boolean).map(c => {
                        const [name, ...rest] = c.split('=');
                        return {
                            name: name,
                            value: rest.join('='),
                            domain: window.location.hostname,
                            path: '/'
                        };
                    });
                })()"#,
            )
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::ActionFailed,
                    format!("Failed to get cookies: {}", e),
                )
            })?;

        Ok(serde_json::json!({ "cookies": cookies }))
    }

    async fn handle_storage_set_cookie(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let name = request
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("name"))?;

        let value = request
            .params
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("value"))?;

        let path = request
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        let domain = request
            .params
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let max_age = request
            .params
            .get("maxAge")
            .and_then(|v| v.as_i64());

        let secure = request
            .params
            .get("secure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Build cookie string
        let mut parts = vec![format!(
            "{}={}",
            serde_json::to_string(name).unwrap_or_default().trim_matches('"'),
            serde_json::to_string(value).unwrap_or_default().trim_matches('"')
        )];
        parts.push(format!("path={}", path));
        if !domain.is_empty() {
            parts.push(format!("domain={}", domain));
        }
        if let Some(age) = max_age {
            parts.push(format!("max-age={}", age));
        }
        if secure {
            parts.push("secure".to_string());
        }

        let cookie_str = parts.join("; ");
        let script = format!(
            "document.cookie = {}",
            serde_json::to_string(&cookie_str).unwrap_or_default()
        );

        page.evaluate_void(&script).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::ActionFailed,
                format!("Failed to set cookie: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "success": true }))
    }

    async fn handle_storage_clear_cookies(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        // Use CDP Network.clearBrowserCookies
        use chromiumoxide::cdp::browser_protocol::network::ClearBrowserCookiesParams;
        let cmd = ClearBrowserCookiesParams::default();
        page.inner().execute(cmd).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::ActionFailed,
                format!("Failed to clear cookies: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "cleared": true }))
    }

    async fn handle_storage_get_local_storage(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let entries: serde_json::Value = page
            .evaluate("Object.fromEntries(Object.entries(localStorage))")
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::ActionFailed,
                    format!("Failed to get localStorage: {}", e),
                )
            })?;

        Ok(serde_json::json!({ "entries": entries }))
    }

    async fn handle_storage_set_local_storage(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let key = request
            .params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("key"))?;

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

        let script = format!(
            "localStorage.setItem({}, {})",
            serde_json::to_string(key).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default()
        );

        page.evaluate_void(&script).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::ActionFailed,
                format!("Failed to set localStorage: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "success": true }))
    }

    async fn handle_storage_get_session_storage(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        let entries: serde_json::Value = page
            .evaluate("Object.fromEntries(Object.entries(sessionStorage))")
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::ActionFailed,
                    format!("Failed to get sessionStorage: {}", e),
                )
            })?;

        Ok(serde_json::json!({ "entries": entries }))
    }

    async fn handle_storage_set_session_storage(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let key = request
            .params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("key"))?;

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

        let script = format!(
            "sessionStorage.setItem({}, {})",
            serde_json::to_string(key).unwrap_or_default(),
            serde_json::to_string(value).unwrap_or_default()
        );

        page.evaluate_void(&script).await.map_err(|e| {
            ProtocolError::new(
                crate::error::ErrorCode::ActionFailed,
                format!("Failed to set sessionStorage: {}", e),
            )
        })?;

        Ok(serde_json::json!({ "success": true }))
    }

    async fn handle_storage_clear(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;
        let page = self
            .sessions
            .get_page(&session_id)
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        page.evaluate_void("localStorage.clear(); sessionStorage.clear()")
            .await
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::ActionFailed,
                    format!("Failed to clear storage: {}", e),
                )
            })?;

        Ok(serde_json::json!({ "cleared": true }))
    }

    //=========================================================================
    // Proxy handlers
    //=========================================================================

    async fn handle_proxy_set(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let server = request
            .params
            .get("server")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing_field("server"))?
            .to_string();

        let protocol: crate::proxy::ProxyProtocol = request
            .params
            .get("protocol")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let username = request
            .params
            .get("username")
            .and_then(|v| v.as_str())
            .map(String::from);

        let password = request
            .params
            .get("password")
            .and_then(|v| v.as_str())
            .map(String::from);

        let proxy = ProxyConfig {
            server,
            protocol,
            username,
            password,
        };

        self.sessions
            .with_session(&session_id, |session| {
                session.proxy = Some(proxy.clone());
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "proxy": proxy
        }))
    }

    async fn handle_proxy_pool(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let proxies_value = request
            .params
            .get("proxies")
            .ok_or_else(|| ProtocolError::missing_field("proxies"))?;

        let proxies: Vec<ProxyConfig> = serde_json::from_value(proxies_value.clone())
            .map_err(|e| {
                ProtocolError::new(
                    crate::error::ErrorCode::InvalidField,
                    format!("Invalid proxies array: {}", e),
                )
            })?;

        let count = proxies.len();
        let pool = ProxyPool::from_list(proxies);

        // Set the first proxy as current
        let current = pool.current().cloned();

        self.sessions
            .with_session(&session_id, |session| {
                if let Some(ref proxy) = current {
                    session.proxy = Some(proxy.clone());
                }
                session.proxy_pool = Some(pool);
                Ok(())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "poolSize": count,
            "current": current
        }))
    }

    async fn handle_proxy_rotate(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let next_proxy = self
            .sessions
            .with_session(&session_id, |session| {
                let pool = session.proxy_pool.as_ref().ok_or_else(|| {
                    TivanaError::Session("No proxy pool configured for this session".to_string())
                })?;
                let next = pool.next().cloned().ok_or_else(|| {
                    TivanaError::Session("Proxy pool is empty".to_string())
                })?;
                session.proxy = Some(next.clone());
                Ok(next)
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "proxy": next_proxy
        }))
    }

    async fn handle_proxy_current(
        &self,
        request: &crate::protocol::RequestMessage,
    ) -> Result<serde_json::Value, ProtocolError> {
        let session_id = self.extract_session_id(request)?;

        let proxy = self
            .sessions
            .with_session(&session_id, |session| {
                Ok(session.proxy.clone())
            })
            .await
            .map_err(|e| ProtocolError::internal(e.to_string()))?;

        Ok(serde_json::json!({
            "proxy": proxy
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
            connect: None,
            use_default_browser: false,
            user_data_dir: None,
        };
        let server = Server::new(args).unwrap();
        assert_eq!(server.addr.port(), 9876);
    }
}

