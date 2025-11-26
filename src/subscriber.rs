//! NATS Subscriber implementation
//!
//! Implements event subscription for NATS JetStream with durable consumers.
//!
//! # Test Reference
//! - Test: `tests/integration/subscriber_test.rs`

use crate::JetStreamClient;
use futures::StreamExt;
use hexagonal_core::DmsgError;
use hexagonal_ports_messaging::event::{CloudEvent, CloudEventExt, CloudEventHelper, EventHandler, EventResult};
use serde::{de::DeserializeOwned, Serialize};

/// NATS JetStream subscriber
pub struct NatsSubscriber {
    client: JetStreamClient,
}

impl NatsSubscriber {
    /// Create a new NATS subscriber
    pub fn new(client: JetStreamClient) -> Self {
        Self { client }
    }

    /// Subscribe to events of a specific type
    ///
    /// Creates a durable consumer and processes messages using the provided handler.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/subscriber_test.rs::test_subscribe`
    pub async fn subscribe<H>(&self, event_type: &str, handler: H) -> EventResult<()>
    where
        H: EventHandler + Send + Sync + 'static,
        H::Event: DeserializeOwned + Serialize,
    {
        // Build subject pattern using CloudEventHelper (e.g., "EVENTS.atom.created")
        let subject = CloudEventHelper::build_subject(event_type, self.client.stream_name());

        // Create durable consumer
        let consumer = self
            .client
            .subscribe(&subject)
            .await
            .map_err(|e| DmsgError::infrastructure("Failed to create consumer").with_context("error", e))?;

        // Get message stream
        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| DmsgError::infrastructure("Failed to get message stream").with_context("error", e))?;

        // Spawn task to process messages
        tokio::spawn(async move {
            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => {
                        // Process message
                        match Self::process_message(&msg.payload, &handler).await {
                            Ok(_) => {
                                // Ack message
                                if let Err(e) = msg.ack().await {
                                    tracing::error!(error = %e, "Failed to ack message");
                                }
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to process message");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Error receiving message");
                    }
                }
            }
        });

        Ok(())
    }

    /// Subscribe with pattern matching (wildcards)
    ///
    /// # Test Reference
    /// - Test: `tests/integration/subscriber_test.rs::test_subscribe_pattern`
    pub async fn subscribe_pattern<H>(&self, pattern: &str, handler: H) -> EventResult<()>
    where
        H: EventHandler + Send + Sync + 'static,
        H::Event: DeserializeOwned + Serialize,
    {
        // Use the pattern directly as subject (e.g., "EVENTS.>")
        self.subscribe(pattern, handler).await
    }

    /// Process incoming message
    ///
    /// Deserializes CloudEvent and extracts the actual event data using
    /// the port's CloudEventExt trait.
    ///
    /// # Test Reference
    /// - Test: `tests/unit/subscriber_test.rs::test_process_message`
    async fn process_message<H>(payload: &[u8], handler: &H) -> EventResult<()>
    where
        H: EventHandler,
        H::Event: DeserializeOwned + Serialize,
    {
        // Deserialize CloudEvent
        let cloud_event: CloudEvent = serde_json::from_slice(payload)
            .map_err(|e| DmsgError::infrastructure("Failed to deserialize CloudEvent").with_context("error", e))?;

        // Convert CloudEvent to domain event using the port's trait
        let event = H::Event::from_cloud_event(&cloud_event)?;

        // Handle event
        handler.handle(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_subscriber() {
        // Placeholder test - integration tests will test actual functionality
    }
}
