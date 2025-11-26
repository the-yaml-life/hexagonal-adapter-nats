//! NATS adapter configuration
//!
//! # Test Reference
//! - Test: `tests/unit/config_test.rs`

use serde::{Deserialize, Serialize};
use crate::error::{NatsError, Result};

/// NATS JetStream configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NatsConfig {
    /// NATS server URL (e.g., "nats://localhost:4222")
    pub url: String,

    /// JetStream stream name
    pub stream_name: String,

    /// Consumer name (for durable subscriptions)
    pub consumer_name: Option<String>,

    /// Consumer group (for load balancing)
    pub consumer_group: Option<String>,

    /// Max reconnect attempts
    pub max_reconnect_attempts: u32,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Enable TLS
    pub enable_tls: bool,

    /// TLS certificate path (if enable_tls)
    pub tls_cert_path: Option<String>,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".to_string(),
            stream_name: "EVENTS".to_string(),
            consumer_name: None,
            consumer_group: None,
            max_reconnect_attempts: 10,
            connection_timeout_secs: 5,
            enable_tls: false,
            tls_cert_path: None,
        }
    }
}

impl NatsConfig {
    /// Validate configuration
    ///
    /// # Test Reference
    /// - Test: `tests/unit/config_test.rs::test_validate_valid_config`
    /// - Test: `tests/unit/config_test.rs::test_validate_invalid_url`
    pub fn validate(&self) -> Result<()> {
        // Validate URL is not empty
        if self.url.is_empty() {
            return Err(NatsError::Config("URL cannot be empty".to_string()));
        }

        // Validate stream name is not empty
        if self.stream_name.is_empty() {
            return Err(NatsError::Config("Stream name cannot be empty".to_string()));
        }

        // Validate TLS config
        if self.enable_tls && self.tls_cert_path.is_none() {
            return Err(NatsError::Config(
                "TLS enabled but no certificate path provided".to_string(),
            ));
        }

        Ok(())
    }

    /// Create configuration from environment variables
    ///
    /// # Environment Variables
    /// - `NATS_URL`: NATS server URL
    /// - `NATS_STREAM_NAME`: JetStream stream name
    /// - `NATS_CONSUMER_NAME`: Consumer name (optional)
    /// - `NATS_CONSUMER_GROUP`: Consumer group (optional)
    ///
    /// # Test Reference
    /// - Test: `tests/unit/config_test.rs::test_from_env`
    pub fn from_env() -> Result<Self> {
        let config = Self {
            url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            stream_name: std::env::var("NATS_STREAM_NAME")
                .unwrap_or_else(|_| "EVENTS".to_string()),
            consumer_name: std::env::var("NATS_CONSUMER_NAME").ok(),
            consumer_group: std::env::var("NATS_CONSUMER_GROUP").ok(),
            max_reconnect_attempts: std::env::var("NATS_MAX_RECONNECT_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            connection_timeout_secs: std::env::var("NATS_CONNECTION_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            enable_tls: std::env::var("NATS_ENABLE_TLS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            tls_cert_path: std::env::var("NATS_TLS_CERT_PATH").ok(),
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NatsConfig::default();
        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.stream_name, "EVENTS");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_url() {
        let mut config = NatsConfig::default();
        config.url = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_stream_name() {
        let mut config = NatsConfig::default();
        config.stream_name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_tls_without_cert() {
        let mut config = NatsConfig::default();
        config.enable_tls = true;
        config.tls_cert_path = None;
        assert!(config.validate().is_err());
    }
}
