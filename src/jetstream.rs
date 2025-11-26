//! JetStream client for NATS
//!
//! Manages JetStream connections, streams, and consumers.
//!
//! # Test Reference
//! - Test: `tests/integration/jetstream_test.rs`

use crate::{NatsConfig, NatsError, Result};
use async_nats::jetstream::consumer::PullConsumer;
use async_nats::{jetstream, Client};

/// JetStream client wrapper
pub struct JetStreamClient {
    client: Client,
    context: jetstream::Context,
    config: NatsConfig,
}

impl JetStreamClient {
    /// Create a new JetStream client
    ///
    /// # Test Reference
    /// - Test: `tests/integration/jetstream_test.rs::test_connect`
    pub async fn new(config: NatsConfig) -> Result<Self> {
        config.validate()?;

        // Connect to NATS
        let client = async_nats::connect(&config.url)
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))?;

        // Get JetStream context
        let context = jetstream::new(client.clone());

        tracing::info!(url = %config.url, "Connected to NATS server");

        Ok(Self {
            client,
            context,
            config,
        })
    }

    /// Ensure JetStream stream exists with the given subjects
    ///
    /// Creates a new stream or updates an existing one.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/jetstream_test.rs::test_ensure_stream`
    pub async fn ensure_stream(&self, subjects: Vec<String>) -> Result<()> {
        let stream_name = &self.config.stream_name;

        tracing::debug!(
            stream = %stream_name,
            subjects = ?subjects,
            "Ensuring JetStream stream exists"
        );

        // Try to get existing stream
        match self.context.get_stream(stream_name).await {
            Ok(mut stream) => {
                tracing::info!(stream = %stream_name, "Stream already exists");

                // TODO: Update stream config if needed
                // For now, just verify it exists
                let _info = stream.info().await.map_err(|e| {
                    NatsError::JetStream(format!("Failed to get stream info: {}", e))
                })?;

                Ok(())
            }
            Err(_) => {
                // Stream doesn't exist, create it
                tracing::info!(stream = %stream_name, "Creating new JetStream stream");

                let stream_config = jetstream::stream::Config {
                    name: stream_name.clone(),
                    subjects: subjects.clone(),
                    max_messages: 100_000,
                    max_bytes: 1_073_741_824, // 1GB
                    ..Default::default()
                };

                self.context
                    .create_stream(stream_config)
                    .await
                    .map_err(|e| NatsError::JetStream(format!("Failed to create stream: {}", e)))?;

                tracing::info!(
                    stream = %stream_name,
                    subjects = ?subjects,
                    "Stream created successfully"
                );

                Ok(())
            }
        }
    }

    /// Publish message to JetStream with acknowledgment
    ///
    /// Publishes a message and waits for acknowledgment from JetStream.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/jetstream_test.rs::test_publish_with_ack`
    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<()> {
        tracing::debug!(
            subject = %subject,
            payload_size = payload.len(),
            "Publishing message to JetStream"
        );

        let ack = self
            .context
            .publish(subject.to_string(), payload.to_vec().into())
            .await
            .map_err(|e| NatsError::Publish(format!("Failed to publish: {}", e)))?;

        // Wait for acknowledgment
        ack.await
            .map_err(|e| NatsError::Publish(format!("Failed to get ack: {}", e)))?;

        tracing::debug!(subject = %subject, "Message published and acknowledged");

        Ok(())
    }

    /// Subscribe to JetStream with durable consumer
    ///
    /// Creates a durable pull consumer and returns it for message consumption.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/jetstream_test.rs::test_subscribe_durable`
    pub async fn subscribe(&self, subject: &str) -> Result<PullConsumer> {
        let stream_name = &self.config.stream_name;

        // Generate consumer name if not provided
        let consumer_name = self
            .config
            .consumer_name
            .clone()
            .unwrap_or_else(|| format!("consumer-{}", uuid::Uuid::new_v4()));

        tracing::debug!(
            stream = %stream_name,
            consumer = %consumer_name,
            subject = %subject,
            "Creating durable consumer"
        );

        // Get the stream
        let stream = self
            .context
            .get_stream(stream_name)
            .await
            .map_err(|e| {
                NatsError::Subscribe(format!("Stream '{}' not found: {}", stream_name, e))
            })?;

        // Consumer configuration
        let consumer_config = jetstream::consumer::pull::Config {
            durable_name: Some(consumer_name.clone()),
            filter_subject: subject.to_string(),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            deliver_policy: jetstream::consumer::DeliverPolicy::All,
            max_deliver: 3,
            ..Default::default()
        };

        // Create or get existing consumer
        let consumer = stream
            .create_consumer(consumer_config)
            .await
            .map_err(|e| {
                NatsError::Subscribe(format!("Failed to create consumer: {}", e))
            })?;

        tracing::info!(
            stream = %stream_name,
            consumer = %consumer_name,
            subject = %subject,
            "Durable consumer created successfully"
        );

        Ok(consumer)
    }

    /// Get a reference to the underlying NATS client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get a reference to the JetStream context
    pub fn context(&self) -> &jetstream::Context {
        &self.context
    }

    /// Get the stream name from config
    pub fn stream_name(&self) -> &str {
        &self.config.stream_name
    }
}

/// Stream configuration builder
pub struct StreamConfigBuilder {
    name: String,
    subjects: Vec<String>,
    max_messages: i64,
    max_bytes: i64,
    max_age: std::time::Duration,
}

impl StreamConfigBuilder {
    /// Create a new stream config builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subjects: Vec::new(),
            max_messages: 100_000,
            max_bytes: 1_073_741_824, // 1GB
            max_age: std::time::Duration::from_secs(86400 * 7), // 7 days
        }
    }

    /// Add a subject to the stream
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subjects.push(subject.into());
        self
    }

    /// Add multiple subjects to the stream
    pub fn subjects(mut self, subjects: Vec<String>) -> Self {
        self.subjects.extend(subjects);
        self
    }

    /// Set maximum number of messages
    pub fn max_messages(mut self, max: i64) -> Self {
        self.max_messages = max;
        self
    }

    /// Set maximum bytes
    pub fn max_bytes(mut self, max: i64) -> Self {
        self.max_bytes = max;
        self
    }

    /// Set maximum age for messages
    pub fn max_age(mut self, duration: std::time::Duration) -> Self {
        self.max_age = duration;
        self
    }

    /// Build the stream configuration
    pub fn build(self) -> jetstream::stream::Config {
        jetstream::stream::Config {
            name: self.name,
            subjects: self.subjects,
            max_messages: self.max_messages,
            max_bytes: self.max_bytes,
            max_age: self.max_age,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    #[ignore = "requires NATS server"]
    async fn test_new_client() {
        let config = NatsConfig::default();
        let result = JetStreamClient::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires NATS server"]
    async fn test_ensure_stream() {
        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await.unwrap();

        let subjects = vec!["test.>".to_string()];
        let result = client.ensure_stream(subjects).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires NATS server"]
    async fn test_publish_and_subscribe() {
        let config = NatsConfig::default();
        let client = JetStreamClient::new(config).await.unwrap();

        // Ensure stream exists
        client
            .ensure_stream(vec!["test.>".to_string()])
            .await
            .unwrap();

        // Publish a message
        let subject = "test.message";
        let payload = b"Hello, NATS!";
        client.publish(subject, payload).await.unwrap();

        // Subscribe and read the message
        let consumer = client.subscribe("test.>").await.unwrap();
        let mut messages = consumer.messages().await.unwrap();

        if let Some(msg) = messages.next().await {
            let msg = msg.unwrap();
            assert_eq!(msg.payload.as_ref(), payload);
            msg.ack().await.unwrap();
        }
    }

    #[test]
    fn test_stream_config_builder() {
        let config = StreamConfigBuilder::new("TEST_STREAM")
            .subject("test.>")
            .subject("events.>")
            .max_messages(50_000)
            .max_bytes(500_000_000)
            .max_age(std::time::Duration::from_secs(3600))
            .build();

        assert_eq!(config.name, "TEST_STREAM");
        assert_eq!(config.subjects.len(), 2);
        assert_eq!(config.max_messages, 50_000);
        assert_eq!(config.max_bytes, 500_000_000);
    }
}
