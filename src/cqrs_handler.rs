//! CQRS Request Handler for NATS
//!
//! Handles incoming command and query requests from NATS.
//! This is a reference implementation showing the request-reply pattern.
//!
//! # Test Reference
//! - Test: `tests/cqrs_tests.rs::cqrs_handler_tests`

use crate::{NatsError, Result, JetStreamClient};
use serde_json::{json, Value};
use std::sync::Arc;

/// CQRS Request Handler
///
/// Listens for command and query requests on NATS subjects.
/// Minimal reference implementation - extend for production use.
pub struct CqrsRequestHandler {
    _client: Arc<JetStreamClient>,
    _service_id: String,
}

impl CqrsRequestHandler {
    /// Create a new CQRS request handler
    pub fn new(client: Arc<JetStreamClient>, service_id: String) -> Self {
        Self {
            _client: client,
            _service_id: service_id,
        }
    }

    /// Build subject for command type
    pub fn command_subject(command_type: &str) -> String {
        format!("commands.{}", command_type)
    }

    /// Build subject for query type
    pub fn query_subject(query_type: &str) -> String {
        format!("queries.{}", query_type)
    }

    /// Create a standard request envelope
    pub fn create_request_envelope(
        request_type: &str,
        request_id: &str,
        service_id: &str,
        payload: Value,
    ) -> Value {
        json!({
            "request_type": request_type,
            "request_id": request_id,
            "service_id": service_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "payload": payload,
        })
    }

    /// Create a standard response envelope
    pub fn create_response_envelope(
        success: bool,
        request_id: &str,
        payload: Option<Value>,
        error: Option<String>,
    ) -> Value {
        json!({
            "success": success,
            "request_id": request_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "payload": payload,
            "error": error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_subject_format() {
        let subject = CqrsRequestHandler::command_subject("CreateUserCommand");
        assert_eq!(subject, "commands.CreateUserCommand");
    }

    #[test]
    fn test_query_subject_format() {
        let subject = CqrsRequestHandler::query_subject("GetUserQuery");
        assert_eq!(subject, "queries.GetUserQuery");
    }

    #[test]
    fn test_request_envelope_creation() {
        let envelope = CqrsRequestHandler::create_request_envelope(
            "CreateUserCommand",
            "req-123",
            "user-service",
            json!({"name": "Alice"}),
        );

        assert_eq!(envelope["request_type"], "CreateUserCommand");
        assert_eq!(envelope["request_id"], "req-123");
        assert_eq!(envelope["service_id"], "user-service");
        assert!(envelope["timestamp"].is_string());
    }

    #[test]
    fn test_response_envelope_success() {
        let response = CqrsRequestHandler::create_response_envelope(
            true,
            "req-123",
            Some(json!({"user_id": "user-456"})),
            None,
        );

        assert_eq!(response["success"], true);
        assert_eq!(response["request_id"], "req-123");
        assert!(response["payload"].is_object());
        assert!(response["error"].is_null());
    }

    #[test]
    fn test_response_envelope_error() {
        let response = CqrsRequestHandler::create_response_envelope(
            false,
            "req-123",
            None,
            Some("User not found".to_string()),
        );

        assert_eq!(response["success"], false);
        assert_eq!(response["error"], "User not found");
        assert!(response["payload"].is_null());
    }
}
