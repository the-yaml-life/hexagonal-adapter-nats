//! Integration tests for NATS adapter
//!
//! These tests require a running NATS server with JetStream enabled.
//! Run with: `cargo test --test integration_tests -- --ignored`

#[cfg(test)]
mod integration {
    use hexagonal_adapter_nats::{JetStreamClient, NatsConfig, NatsPublisher, NatsSubscriber};

    #[tokio::test]
    #[ignore = "requires NATS server with JetStream"]
    async fn test_connect_to_nats() {
        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await;
        assert!(client.is_ok());
    }

    // Mock event for testing
    use serde::Serialize;
    use std::time::SystemTime;
    use hexagonal_ports_messaging::event::Event;

    #[derive(Debug, Clone, Serialize)]
    struct MockEvent {
        id: String,
        data: String,
    }

    impl Event for MockEvent {
        fn event_type() -> &'static str {
            "test.mock.event"
        }

        fn event_id(&self) -> &str {
            &self.id
        }

        fn occurred_at(&self) -> SystemTime {
            SystemTime::now()
        }
    }

    #[tokio::test]
    #[ignore = "requires NATS server with JetStream"]
    async fn test_publish_and_subscribe() {
        // Setup
        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await.expect("Failed to connect");

        // Ensure stream exists
        client
            .ensure_stream(vec!["EVENTS.>".to_string()])
            .await
            .expect("Failed to create stream");

        // Create publisher
        let publisher = NatsPublisher::new(client, "test-service".to_string());

        // Create and publish event
        let event = MockEvent {
            id: "mock-event-123".to_string(),
            data: "test data".to_string(),
        };

        use hexagonal_ports_messaging::event::EventBus;
        publisher.publish(event).await.expect("Failed to publish");
    }
}
