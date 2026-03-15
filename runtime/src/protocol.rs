//! Protocol envelope types for Tivana WebSocket communication
//!
//! The protocol uses JSON-RPC-like messages with correlated request/response pairs
//! and asynchronous events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ProtocolError;

/// Protocol version
pub const PROTOCOL_VERSION: &str = "1.0";

/// Message type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Client-to-server request expecting a response
    Request,
    /// Server-to-client response to a request
    Response,
    /// Server-to-client asynchronous event
    Event,
}

/// Inbound message from client (request only)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMessage {
    /// Unique message ID for correlation
    pub id: String,

    /// Message type (always "request" for inbound)
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Method to invoke (e.g., "session.create", "browser.navigate")
    pub method: String,

    /// Session ID (optional, required for session-scoped methods)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Method-specific parameters
    #[serde(default)]
    pub params: serde_json::Value,

    /// Protocol version
    #[serde(default = "default_version")]
    pub version: String,
}

/// Outbound response message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMessage {
    /// Correlated request ID
    pub id: String,

    /// Message type (always "response")
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Result payload (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error details (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,

    /// Protocol version
    pub version: String,
}

impl ResponseMessage {
    /// Create a success response
    pub fn success(id: impl Into<String>, result: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            message_type: MessageType::Response,
            result: Some(result),
            error: None,
            version: PROTOCOL_VERSION.to_string(),
        }
    }

    /// Create an error response
    pub fn error(id: impl Into<String>, error: ProtocolError) -> Self {
        Self {
            id: id.into(),
            message_type: MessageType::Response,
            result: None,
            error: Some(error),
            version: PROTOCOL_VERSION.to_string(),
        }
    }
}

/// Server-initiated event message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMessage {
    /// Unique event ID
    pub id: String,

    /// Message type (always "event")
    #[serde(rename = "type")]
    pub message_type: MessageType,

    /// Event name (e.g., "session.stateChanged", "browser.pageLoaded")
    pub event: String,

    /// Session ID this event relates to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Event payload
    #[serde(default)]
    pub data: serde_json::Value,

    /// Protocol version
    pub version: String,
}

impl EventMessage {
    /// Create a new event
    pub fn new(event: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Event,
            event: event.into(),
            session_id: None,
            data,
            version: PROTOCOL_VERSION.to_string(),
        }
    }

    /// Create a session-scoped event
    pub fn for_session(
        session_id: impl Into<String>,
        event: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Event,
            event: event.into(),
            session_id: Some(session_id.into()),
            data,
            version: PROTOCOL_VERSION.to_string(),
        }
    }
}

/// Unified outbound message type for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutboundMessage {
    Response(ResponseMessage),
    Event(EventMessage),
}

impl From<ResponseMessage> for OutboundMessage {
    fn from(msg: ResponseMessage) -> Self {
        OutboundMessage::Response(msg)
    }
}

impl From<EventMessage> for OutboundMessage {
    fn from(msg: EventMessage) -> Self {
        OutboundMessage::Event(msg)
    }
}

fn default_version() -> String {
    PROTOCOL_VERSION.to_string()
}

/// Parse an inbound JSON message
pub fn parse_request(json: &str) -> Result<RequestMessage, ProtocolError> {
    serde_json::from_str(json).map_err(|e| ProtocolError::invalid_message(e.to_string()))
}

/// Serialize an outbound message to JSON
pub fn serialize_outbound(msg: &OutboundMessage) -> Result<String, ProtocolError> {
    serde_json::to_string(msg).map_err(|e| ProtocolError::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let json = r#"{
            "id": "req-1",
            "type": "request",
            "method": "session.create",
            "params": {"url": "https://example.com"}
        }"#;

        let msg = parse_request(json).unwrap();
        assert_eq!(msg.id, "req-1");
        assert_eq!(msg.method, "session.create");
        assert_eq!(msg.message_type, MessageType::Request);
    }

    #[test]
    fn test_response_success() {
        let response = ResponseMessage::success("req-1", serde_json::json!({"sessionId": "abc"}));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("req-1"));
        assert!(json.contains("sessionId"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_response_error() {
        let response = ResponseMessage::error("req-1", ProtocolError::session_not_found("unknown"));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("req-1"));
        assert!(json.contains("session_not_found"));
        assert!(!json.contains("result"));
    }

    #[test]
    fn test_event_message() {
        let event =
            EventMessage::for_session("sess-1", "browser.pageLoaded", serde_json::json!({}));
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("event"));
        assert!(json.contains("sess-1"));
        assert!(json.contains("pageLoaded"));
    }
}
