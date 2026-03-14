//! Error types for Tivana
//!
//! Errors are categorized by source:
//! - ProtocolError: malformed messages, missing fields
//! - SessionError: invalid session, session closed
//! - BrowserError: launch failed, crashed, disconnected
//! - ActionError: target not found, action failed
//! - PerceptionError: failed to read page state

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Error codes for protocol responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Protocol errors (1xxx)
    InvalidMessage = 1001,
    MissingField = 1002,
    InvalidField = 1003,
    UnknownMethod = 1004,

    // Session errors (2xxx)
    SessionNotFound = 2001,
    SessionClosed = 2002,
    SessionExists = 2003,
    InvalidSessionState = 2004,

    // Browser errors (3xxx)
    BrowserLaunchFailed = 3001,
    BrowserCrashed = 3002,
    BrowserDisconnected = 3003,
    NavigationFailed = 3004,

    // Action errors (4xxx)
    TargetNotFound = 4001,
    TargetAmbiguous = 4002,
    ActionFailed = 4003,
    ActionTimeout = 4004,

    // Perception errors (5xxx)
    PerceptionFailed = 5001,
    ElementNotAccessible = 5002,
    StyleComputationFailed = 5003,

    // Internal errors (9xxx)
    InternalError = 9001,
}

impl ErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Structured error for protocol responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

impl ProtocolError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    // Convenience constructors
    pub fn invalid_message(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidMessage, message)
    }

    pub fn missing_field(field: &str) -> Self {
        Self::new(
            ErrorCode::MissingField,
            format!("Missing required field: {}", field),
        )
    }

    pub fn session_not_found(session_id: &str) -> Self {
        Self::new(
            ErrorCode::SessionNotFound,
            format!("Session not found: {}", session_id),
        )
    }

    pub fn target_not_found(target: &str) -> Self {
        Self::new(
            ErrorCode::TargetNotFound,
            format!("Target element not found: {}", target),
        )
    }

    pub fn target_ambiguous(target: &str, count: usize) -> Self {
        Self::new(
            ErrorCode::TargetAmbiguous,
            format!("Target selector matched {} elements: {}", count, target),
        )
        .with_data(serde_json::json!({ "matchCount": count }))
    }

    pub fn browser_launch_failed(reason: impl Into<String>) -> Self {
        Self::new(ErrorCode::BrowserLaunchFailed, reason)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }
}

/// Main error type for internal use
#[derive(Error, Debug)]
pub enum TivanaError {
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Browser error: {0}")]
    Browser(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Session error: {0}")]
    Session(String),
}

impl From<TivanaError> for ProtocolError {
    fn from(err: TivanaError) -> Self {
        match err {
            TivanaError::Protocol(e) => e,
            TivanaError::WebSocket(e) => ProtocolError::internal(e.to_string()),
            TivanaError::Json(e) => ProtocolError::invalid_message(e.to_string()),
            TivanaError::Browser(e) => ProtocolError::new(ErrorCode::BrowserCrashed, e),
            TivanaError::Io(e) => ProtocolError::internal(e.to_string()),
            TivanaError::Session(e) => ProtocolError::new(ErrorCode::SessionNotFound, e),
        }
    }
}

pub type Result<T> = std::result::Result<T, TivanaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let err = ProtocolError::target_ambiguous("button[Submit]", 3);
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("target_ambiguous"));
        assert!(json.contains("matchCount"));
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ErrorCode::InvalidMessage.as_i32(), 1001);
        assert_eq!(ErrorCode::TargetNotFound.as_i32(), 4001);
    }

    #[test]
    fn test_error_display() {
        let err = ProtocolError::session_not_found("abc123");
        let display = format!("{}", err);
        assert!(display.contains("abc123"));
    }
}
