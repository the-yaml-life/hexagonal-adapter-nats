//! NATS JetStream Adapter for Hexagonal Architecture
//!
//! This adapter implements the messaging ports (Publisher/Subscriber) using NATS JetStream
//! with CloudEvents format for event-driven architectures.
//!
//! Includes support for CQRS pattern with CommandBus and QueryBus implementations.
//!
//! # Features
//!
//! - **JetStream Publishing**: Publish events with guaranteed delivery and acknowledgment
//! - **JetStream Subscribing**: Subscribe to event streams with durable consumers
//! - **CloudEvents**: Standard event format (CloudEvents 1.0 spec)
//! - **CQRS Pattern**: CommandBus and QueryBus for command and query handling (hex-4.2.0)
//! - **Outbox Pattern**: Transactional outbox support for reliable event publishing
//! - **Retry Logic**: Automatic retry with exponential backoff
//! - **Consumer Groups**: Load balancing across multiple subscribers
//!
//! # Example: Event Publishing
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
//!
//! # Example: CQRS
//!
//! ```rust,ignore
//! use hexagonal_adapter_nats::{NatsConfig, NatsCommandBus, NatsQueryBus, JetStreamClient};
//! use hexagonal_ports_messaging::{CommandBus, QueryBus};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = NatsConfig::default();
//!     let client = Arc::new(JetStreamClient::new(config).await?);
//!
//!     let command_bus = NatsCommandBus::new(client.clone(), 5000, "my-service".to_string());
//!     let query_bus = NatsQueryBus::new(client, 5000, "my-service".to_string());
//!
//!     // Execute command
//!     let result = command_bus.execute(my_command).await?;
//!
//!     // Execute query
//!     let data = query_bus.execute(my_query).await?;
//!
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
pub mod command_bus;
pub mod query_bus;
pub mod cqrs_handler;

// Re-exports
pub use config::NatsConfig;
pub use error::{NatsError, Result};
pub use jetstream::{JetStreamClient, StreamConfigBuilder};
pub use outbox::{OutboxEvent, OutboxPublisher, OutboxRepository};
pub use publisher::NatsPublisher;
pub use subscriber::NatsSubscriber;
pub use command_bus::NatsCommandBus;
pub use query_bus::NatsQueryBus;
pub use cqrs_handler::CqrsRequestHandler;

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
        let _ = std::any::type_name::<NatsCommandBus>();
        let _ = std::any::type_name::<NatsQueryBus>();
        let _ = std::any::type_name::<CqrsRequestHandler>();
    }
}
