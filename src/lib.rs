//! NATS JetStream Adapter for Hexagonal Architecture
//!
//! This adapter implements the messaging ports (Publisher/Subscriber) using NATS JetStream
//! with CloudEvents format for event-driven architectures.
//!
//! # Features
//!
//! - **JetStream Publishing**: Publish events with guaranteed delivery and acknowledgment
//! - **JetStream Subscribing**: Subscribe to event streams with durable consumers
//! - **CloudEvents**: Standard event format (CloudEvents 1.0 spec)
//! - **Outbox Pattern**: Transactional outbox support for reliable event publishing
//! - **Retry Logic**: Automatic retry with exponential backoff
//! - **Consumer Groups**: Load balancing across multiple subscribers
//!
//! # Example
//!
//! ```rust,ignore
//! use hexagonal_adapter_nats::{NatsConfig, NatsPublisher, JetStreamClient};
//! use hexagonal_ports_messaging::Publisher;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = NatsConfig::default();
//!     let client = JetStreamClient::new(config).await?;
//!     let publisher = NatsPublisher::new(client, "my-service".to_string());
//!
//!     // Publish event
//!     publisher.publish(&event).await?;
//!     Ok(())
//! }
//! ```

// Module declarations
pub mod config;
pub mod error;
pub mod jetstream;
pub mod outbox;
pub mod publisher;
pub mod subscriber;

// Re-exports
pub use config::NatsConfig;
pub use error::{NatsError, Result};
pub use jetstream::{JetStreamClient, StreamConfigBuilder};
pub use outbox::{OutboxEvent, OutboxPublisher, OutboxRepository};
pub use publisher::NatsPublisher;
pub use subscriber::NatsSubscriber;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = std::any::type_name::<NatsConfig>();
        let _ = std::any::type_name::<NatsError>();
        let _ = std::any::type_name::<JetStreamClient>();
        let _ = std::any::type_name::<NatsPublisher>();
        let _ = std::any::type_name::<NatsSubscriber>();
    }
}
