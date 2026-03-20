//! Session persistence for crash recovery
//!
//! Saves session state to ~/.tivana/sessions.json so that sessions can be
//! reattached after a runtime restart (in --connect mode).

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::TivanaError;

/// Persisted state for a single session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedSession {
    /// Unique session ID
    pub session_id: String,

    /// Whether the session was running headless
    pub headless: bool,

    /// Active tab target IDs
    pub target_ids: Vec<String>,

    /// Page URLs corresponding to targets
    pub page_urls: Vec<String>,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// Whether the session is considered stale (target no longer reachable)
    #[serde(default)]
    pub stale: bool,
}

/// Top-level persistence file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedState {
    /// Runtime version that wrote this file
    pub version: String,

    /// When this file was last written
    pub updated_at: DateTime<Utc>,

    /// Persisted sessions
    pub sessions: Vec<PersistedSession>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            updated_at: Utc::now(),
            sessions: Vec::new(),
        }
    }
}

/// Get the path to the persistence file (~/.tivana/sessions.json)
pub fn persistence_path() -> Result<PathBuf, TivanaError> {
    let home = dirs::home_dir()
        .ok_or_else(|| TivanaError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        )))?;
    Ok(home.join(".tivana").join("sessions.json"))
}

/// Load persisted state from disk. Returns default if file doesn't exist.
pub fn load() -> Result<PersistedState, TivanaError> {
    let path = persistence_path()?;
    if !path.exists() {
        debug!("No persistence file found at {}", path.display());
        return Ok(PersistedState::default());
    }

    let data = std::fs::read_to_string(&path).map_err(|e| {
        TivanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read {}: {}", path.display(), e),
        ))
    })?;

    let state: PersistedState = serde_json::from_str(&data).map_err(|e| {
        warn!("Corrupted persistence file, starting fresh: {}", e);
        TivanaError::Session(format!("Failed to parse persistence file: {}", e))
    })?;

    info!(
        sessions = state.sessions.len(),
        "Loaded persisted session state"
    );
    Ok(state)
}

/// Save persisted state to disk. Creates ~/.tivana/ if needed.
pub fn save(state: &PersistedState) -> Result<(), TivanaError> {
    let path = persistence_path()?;

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            TivanaError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create {}: {}", parent.display(), e),
            ))
        })?;
    }

    let mut state = state.clone();
    state.updated_at = Utc::now();
    state.version = env!("CARGO_PKG_VERSION").to_string();

    let json = serde_json::to_string_pretty(&state).map_err(|e| {
        TivanaError::Session(format!("Failed to serialize persistence state: {}", e))
    })?;

    std::fs::write(&path, json).map_err(|e| {
        TivanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write {}: {}", path.display(), e),
        ))
    })?;

    debug!(path = %path.display(), sessions = state.sessions.len(), "Persisted session state");
    Ok(())
}

/// Record a session creation in the persistence file
pub fn persist_session_create(
    session_id: &str,
    headless: bool,
    target_ids: Vec<String>,
    page_urls: Vec<String>,
) {
    let result = (|| -> Result<(), TivanaError> {
        let mut state = load().unwrap_or_default();

        // Remove any existing entry for this session
        state.sessions.retain(|s| s.session_id != session_id);

        state.sessions.push(PersistedSession {
            session_id: session_id.to_string(),
            headless,
            target_ids,
            page_urls,
            created_at: Utc::now(),
            stale: false,
        });

        save(&state)
    })();

    if let Err(e) = result {
        error!("Failed to persist session create: {}", e);
    }
}

/// Record a session closure in the persistence file
pub fn persist_session_close(session_id: &str) {
    let result = (|| -> Result<(), TivanaError> {
        let mut state = load().unwrap_or_default();
        state.sessions.retain(|s| s.session_id != session_id);
        save(&state)
    })();

    if let Err(e) = result {
        error!("Failed to persist session close: {}", e);
    }
}

/// Mark a session as stale (target no longer reachable)
pub fn mark_session_stale(session_id: &str) {
    let result = (|| -> Result<(), TivanaError> {
        let mut state = load().unwrap_or_default();
        if let Some(session) = state.sessions.iter_mut().find(|s| s.session_id == session_id) {
            session.stale = true;
        }
        save(&state)
    })();

    if let Err(e) = result {
        error!("Failed to mark session stale: {}", e);
    }
}

/// Get all non-stale persisted sessions
pub fn get_active_persisted_sessions() -> Vec<PersistedSession> {
    match load() {
        Ok(state) => state.sessions.into_iter().filter(|s| !s.stale).collect(),
        Err(e) => {
            warn!("Failed to load persisted sessions: {}", e);
            Vec::new()
        }
    }
}

/// Clear all persisted sessions
pub fn clear_all() {
    let result = save(&PersistedState::default());
    if let Err(e) = result {
        error!("Failed to clear persisted sessions: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_persisted_state_default() {
        let state = PersistedState::default();
        assert!(state.sessions.is_empty());
        assert_eq!(state.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_persisted_session_serialization() {
        let session = PersistedSession {
            session_id: "test-123".to_string(),
            headless: true,
            target_ids: vec!["target-1".to_string()],
            page_urls: vec!["https://example.com".to_string()],
            created_at: Utc::now(),
            stale: false,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: PersistedSession = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, "test-123");
        assert_eq!(deserialized.target_ids.len(), 1);
        assert!(!deserialized.stale);
    }

    #[test]
    fn test_persisted_state_roundtrip() {
        let mut state = PersistedState::default();
        state.sessions.push(PersistedSession {
            session_id: "s1".to_string(),
            headless: false,
            target_ids: vec!["t1".to_string(), "t2".to_string()],
            page_urls: vec![
                "https://example.com".to_string(),
                "https://example.org".to_string(),
            ],
            created_at: Utc::now(),
            stale: false,
        });

        let json = serde_json::to_string_pretty(&state).unwrap();
        let restored: PersistedState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sessions.len(), 1);
        assert_eq!(restored.sessions[0].session_id, "s1");
        assert_eq!(restored.sessions[0].target_ids.len(), 2);
    }

    #[test]
    fn test_persistence_path() {
        let path = persistence_path().unwrap();
        assert!(path.ends_with(".tivana/sessions.json"));
    }
}
