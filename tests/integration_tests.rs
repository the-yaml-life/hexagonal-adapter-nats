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

    #[tokio::test]
    #[ignore = "requires NATS server with JetStream"]
    async fn test_publish_and_subscribe() {
        // Setup
        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await.expect("Failed to connect");

        // Create publisher
        let publisher = NatsPublisher::new(client, "test-service".to_string());

        // TODO: Implement when publisher is ready
        // let event = MockEvent { ... };
        // publisher.publish(&event).await.expect("Failed to publish");
    }
}
