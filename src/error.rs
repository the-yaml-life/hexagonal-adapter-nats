//! Error types for NATS adapter
//!
//! # Test Reference
//! - Test: `tests/unit/error_test.rs`

use thiserror::Error;

/// NATS adapter error types
#[derive(Debug, Error)]
pub enum NatsError {
    /// Connection error to NATS server
    #[error("Connection error: {0}")]
    Connection(String),

    /// JetStream-specific error
    #[error("JetStream error: {0}")]
    JetStream(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Event publishing error
    #[error("Publish error: {0}")]
    Publish(String),

    /// Event subscription error
    #[error("Subscribe error: {0}")]
    Subscribe(String),

    /// NATS client error
    #[error("NATS client error: {0}")]
    Client(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// CloudEvents error
    #[error("CloudEvents error: {0}")]
    CloudEvents(String),
}

/// Result type alias for NATS operations
pub type Result<T> = std::result::Result<T, NatsError>;

// Implement conversion from async_nats errors
impl From<async_nats::Error> for NatsError {
    fn from(err: async_nats::Error) -> Self {
        NatsError::Client(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = NatsError::Connection("failed to connect".to_string());
        assert_eq!(err.to_string(), "Connection error: failed to connect");
    }

    #[test]
    fn test_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json");
        assert!(json_err.is_err());

        let nats_err: NatsError = json_err.unwrap_err().into();
        assert!(matches!(nats_err, NatsError::Serialization(_)));
    }
}
