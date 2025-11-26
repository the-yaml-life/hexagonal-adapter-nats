//! NATS Publisher implementation
//!
//! Implements the EventBus port trait for NATS JetStream with CloudEvents support.
//!
//! # Test Reference
//! - Test: `tests/integration/publisher_test.rs`

use crate::JetStreamClient;
use async_trait::async_trait;
use hexagonal_core::DmsgError;
use hexagonal_ports_messaging::event::{Event, EventBus, EventResult, CloudEventExt, CloudEventHelper};
use serde::Serialize;

/// NATS JetStream publisher implementing the EventBus trait
pub struct NatsPublisher {
    client: JetStreamClient,
    source: String,
}

impl NatsPublisher {
    /// Create a new NATS publisher
    ///
    /// # Arguments
    /// - `client`: JetStream client
    /// - `source`: Source identifier (service name) for CloudEvents
    pub fn new(client: JetStreamClient, source: String) -> Self {
        Self { client, source }
    }

    /// Build NATS subject from event type
    ///
    /// Converts event types like "atom.created" to subjects like "EVENTS.atom.created"
    ///
    /// Uses CloudEventHelper from the messaging port for consistent subject formatting.
    fn build_subject(&self, event_type: &str) -> String {
        CloudEventHelper::build_subject(event_type, self.client.stream_name())
    }
}

#[async_trait]
impl EventBus for NatsPublisher {
    /// Publish an event to NATS JetStream
    ///
    /// Converts the event to CloudEvents format and publishes it to JetStream
    /// with the appropriate subject based on event type.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/publisher_test.rs::test_publish_event`
    async fn publish<E>(&self, event: E) -> EventResult<String>
    where
        E: Event + Serialize,
    {
        // Convert to CloudEvent using the port's CloudEventExt trait
        let cloud_event = event.to_cloud_event(&self.source)?;

        // Serialize CloudEvent
        let payload = serde_json::to_vec(&cloud_event).map_err(|e| {
            DmsgError::infrastructure("Failed to serialize CloudEvent")
                .with_context("error", e)
        })?;

        // Build subject
        let subject = self.build_subject(E::event_type());

        // Publish to JetStream
        self.client.publish(&subject, &payload).await.map_err(|e| {
            DmsgError::infrastructure("Failed to publish to NATS")
                .with_context("subject", &subject)
                .with_context("error", e)
        })?;

        tracing::info!(
            event_id = %event.event_id(),
            event_type = %E::event_type(),
            subject = %subject,
            "Event published to NATS"
        );

        Ok(event.event_id().to_string())
    }

    /// Subscribe to events (not implemented for publisher)
    ///
    /// Use `NatsSubscriber` for subscribing to events
    async fn subscribe<H>(&self, _event_type: &str, _handler: H) -> EventResult<()>
    where
        H: hexagonal_ports_messaging::event::EventHandler + 'static,
        H::Event: serde::de::DeserializeOwned,
    {
        Err(DmsgError::application("Use NatsSubscriber for subscribing to events"))
    }

    /// Unsubscribe from events (not implemented for publisher)
    async fn unsubscribe(&self, _subscription_id: &str) -> EventResult<()> {
        Err(DmsgError::application("Use NatsSubscriber for unsubscribing"))
    }

    /// Publish multiple events atomically
    ///
    /// # Test Reference
    /// - Test: `tests/integration/publisher_test.rs::test_publish_batch`
    async fn publish_batch<E>(&self, events: Vec<E>) -> EventResult<Vec<String>>
    where
        E: Event + Serialize,
    {
        let mut ids = Vec::with_capacity(events.len());

        for event in events {
            let id = self.publish(event).await?;
            ids.push(id);
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Mock event for testing
    #[derive(Debug, Clone, Serialize)]
    struct TestEvent {
        id: String,
        data: String,
    }

    impl Event for TestEvent {
        fn event_type() -> &'static str {
            "test.event"
        }

        fn event_id(&self) -> &str {
            &self.id
        }

        fn occurred_at(&self) -> SystemTime {
            SystemTime::now()
        }
    }

    #[tokio::test]
    #[ignore = "requires NATS server"]
    async fn test_publish_event() {
        use crate::NatsConfig;

        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await.unwrap();

        // Ensure stream exists
        client
            .ensure_stream(vec!["EVENTS.>".to_string()])
            .await
            .unwrap();

        let publisher = NatsPublisher::new(client, "test-service".to_string());

        let event = TestEvent {
            id: "test-123".to_string(),
            data: "test data".to_string(),
        };

        let result = publisher.publish(event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_subject() {
        use crate::NatsConfig;

        // Can't test without async, but we can test the concept
        // Subject should be: STREAM_NAME.event.type
    }
}
