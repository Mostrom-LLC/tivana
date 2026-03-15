//! Session registry and lifecycle management
//!
//! Each session represents one browser context with:
//! - A unique ID
//! - A state machine (Created → Launching → Active → Closed)
//! - A browser handle (when active)

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::browser::{BrowserHandle, PageHandle};
use crate::error::{ProtocolError, TivanaError};
use crate::perceive::{MutationEvent, MutationObserverHandle};

/// Session lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Session created but browser not launched
    Created,
    /// Browser is launching
    Launching,
    /// Browser is active and ready
    Active,
    /// Session is closed
    Closed,
}

/// A browser session
pub struct Session {
    /// Unique session ID
    pub id: String,

    /// Current state
    pub state: SessionState,

    /// Browser handle (present when Active)
    pub browser: Option<BrowserHandle>,

    /// Creation timestamp
    pub created_at: std::time::Instant,

    /// Session configuration
    pub config: SessionConfig,

    /// Mutation observer handle (for stopping)
    pub mutation_observer_handle: Option<MutationObserverHandle>,

    /// Mutation events receiver
    pub mutation_rx: Option<mpsc::Receiver<MutationEvent>>,
}

// Manual Debug implementation since BrowserHandle contains non-Debug types
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("browser", &self.browser.is_some())
            .field("created_at", &self.created_at)
            .field("config", &self.config)
            .field(
                "mutation_observer",
                &self.mutation_observer_handle.is_some(),
            )
            .finish()
    }
}

/// Session configuration
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    /// Initial URL to navigate to
    pub initial_url: Option<String>,

    /// Run in headless mode
    pub headless: bool,

    /// Viewport width
    pub viewport_width: Option<u32>,

    /// Viewport height
    pub viewport_height: Option<u32>,
}

impl Session {
    /// Create a new session with the given ID
    pub fn new(id: String, config: SessionConfig) -> Self {
        Self {
            id,
            state: SessionState::Created,
            browser: None,
            created_at: std::time::Instant::now(),
            config,
            mutation_observer_handle: None,
            mutation_rx: None,
        }
    }

    /// Transition to launching state
    pub fn start_launch(&mut self) -> Result<(), TivanaError> {
        if self.state != SessionState::Created {
            return Err(TivanaError::Session(format!(
                "Cannot launch session in state {:?}",
                self.state
            )));
        }
        self.state = SessionState::Launching;
        Ok(())
    }

    /// Complete launch with browser handle
    pub fn complete_launch(&mut self, browser: BrowserHandle) -> Result<(), TivanaError> {
        if self.state != SessionState::Launching {
            return Err(TivanaError::Session(format!(
                "Cannot complete launch in state {:?}",
                self.state
            )));
        }
        self.browser = Some(browser);
        self.state = SessionState::Active;
        Ok(())
    }

    /// Close the session
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
        self.browser = None;
        // Stop mutation observer if running
        if let Some(handle) = self.mutation_observer_handle.take() {
            handle.stop();
        }
        self.mutation_rx = None;
    }

    /// Start mutation observation
    pub fn start_mutation_observer(
        &mut self,
        rx: mpsc::Receiver<MutationEvent>,
        handle: MutationObserverHandle,
    ) {
        self.mutation_rx = Some(rx);
        self.mutation_observer_handle = Some(handle);
    }

    /// Stop mutation observation
    pub fn stop_mutation_observer(&mut self) {
        if let Some(handle) = self.mutation_observer_handle.take() {
            handle.stop();
        }
        self.mutation_rx = None;
    }

    /// Check if mutation observer is running
    pub fn is_mutation_observer_running(&self) -> bool {
        self.mutation_observer_handle.is_some()
    }

    /// Take the mutation receiver (for streaming)
    pub fn take_mutation_rx(&mut self) -> Option<mpsc::Receiver<MutationEvent>> {
        self.mutation_rx.take()
    }

    /// Check if session is active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Get session info for API responses
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            state: self.state,
            age_secs: self.created_at.elapsed().as_secs(),
        }
    }

    /// Get the default page from the browser
    pub async fn default_page(&self) -> Result<Arc<PageHandle>, TivanaError> {
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| TivanaError::Session("Session has no browser".to_string()))?;
        browser.default_page().await
    }
}

/// Session info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub state: SessionState,
    pub age_secs: u64,
}

/// Thread-safe session registry
#[derive(Debug, Clone)]
pub struct SessionRegistry {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return its ID
    pub async fn create(&self, config: SessionConfig) -> String {
        let id = Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), config);

        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), session);

        info!(session_id = %id, "Session created");
        id
    }

    /// Get a session by ID
    pub async fn get(&self, id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(id).map(|s| s.info())
    }

    /// Check if a session exists
    pub async fn exists(&self, id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(id)
    }

    /// List all sessions
    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|s| s.info()).collect()
    }

    /// Update session state to launching
    pub async fn start_launch(&self, id: &str) -> Result<(), TivanaError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| TivanaError::Session(format!("Session not found: {}", id)))?;
        session.start_launch()?;
        debug!(session_id = %id, "Session launching");
        Ok(())
    }

    /// Complete session launch with browser handle
    pub async fn complete_launch(
        &self,
        id: &str,
        browser: BrowserHandle,
    ) -> Result<(), TivanaError> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| TivanaError::Session(format!("Session not found: {}", id)))?;
        session.complete_launch(browser)?;
        info!(session_id = %id, "Session active");
        Ok(())
    }

    /// Get the default page for a session
    pub async fn get_page(&self, id: &str) -> Result<Arc<PageHandle>, TivanaError> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| TivanaError::Session(format!("Session not found: {}", id)))?;

        if !session.is_active() {
            return Err(TivanaError::Session(format!(
                "Session {} is not active (state: {:?})",
                id, session.state
            )));
        }

        session.default_page().await
    }

    /// Execute a function with mutable access to a session
    pub async fn with_session<F, T>(&self, id: &str, f: F) -> Result<T, TivanaError>
    where
        F: FnOnce(&mut Session) -> Result<T, TivanaError>,
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| TivanaError::Session(format!("Session not found: {}", id)))?;
        f(session)
    }

    /// Close and remove a session
    pub async fn close(&self, id: &str) -> Result<SessionInfo, ProtocolError> {
        let mut sessions = self.sessions.write().await;
        match sessions.remove(id) {
            Some(mut session) => {
                session.close();
                info!(session_id = %id, "Session closed");
                Ok(session.info())
            }
            None => {
                warn!(session_id = %id, "Attempted to close non-existent session");
                Err(ProtocolError::session_not_found(id))
            }
        }
    }

    /// Get count of active sessions
    pub async fn active_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.values().filter(|s| s.is_active()).count()
    }

    /// Close all sessions
    pub async fn close_all(&self) {
        let mut sessions = self.sessions.write().await;
        let count = sessions.len();
        for session in sessions.values_mut() {
            session.close();
        }
        sessions.clear();
        info!(count, "All sessions closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let registry = SessionRegistry::new();

        // Create session
        let id = registry.create(SessionConfig::default()).await;
        assert!(registry.exists(&id).await);

        // Check initial state
        let info = registry.get(&id).await.unwrap();
        assert_eq!(info.state, SessionState::Created);

        // Start launch
        registry.start_launch(&id).await.unwrap();
        let info = registry.get(&id).await.unwrap();
        assert_eq!(info.state, SessionState::Launching);

        // Close session
        let result = registry.close(&id).await;
        assert!(result.is_ok());
        assert!(!registry.exists(&id).await);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let registry = SessionRegistry::new();

        registry.create(SessionConfig::default()).await;
        registry.create(SessionConfig::default()).await;

        let sessions = registry.list().await;
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_close_nonexistent() {
        let registry = SessionRegistry::new();
        let result = registry.close("nonexistent").await;
        assert!(result.is_err());
    }
}
