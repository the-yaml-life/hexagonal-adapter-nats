//! NATS Subscriber implementation
//!
//! Implements event subscription for NATS JetStream with durable consumers
//! and request-reply patterns for synchronous message handling.
//!
//! # Test Reference
//! - Test: `tests/integration/subscriber_test.rs`

use crate::JetStreamClient;
use futures::StreamExt;
use hexagonal_core::DmsgError;
use hexagonal_ports_messaging::event::{CloudEvent, CloudEventExt, CloudEventHelper, EventHandler, EventResult, ReplyHandler};
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

    /// Consume messages and reply with responses (request-reply pattern)
    ///
    /// Creates a durable consumer that processes incoming messages and sends responses
    /// back to the reply subject. This implements the NATS request-reply pattern.
    ///
    /// # Arguments
    ///
    /// * `subject` - Subject pattern to consume (e.g., "requests.process")
    /// * `handler` - Handler implementing ReplyHandler trait
    ///
    /// # Test Reference
    /// - Test: `tests/integration/subscriber_test.rs::test_consume_and_reply`
    pub async fn consume_and_reply<H>(&self, subject: &str, handler: H) -> EventResult<()>
    where
        H: ReplyHandler + 'static,
        H::Request: DeserializeOwned,
        H::Response: Serialize,
    {
        // Build subject pattern using CloudEventHelper if needed
        let subject = CloudEventHelper::build_subject(subject, self.client.stream_name());

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

        // Get reference to the NATS client for publishing responses
        let nats_client = self.client.client().clone();

        tracing::info!(
            subject = %subject,
            "Started consuming messages for request-reply pattern"
        );

        // Spawn task to process messages
        tokio::spawn(async move {
            while let Some(msg) = messages.next().await {
                match msg {
                    Ok(msg) => {
                        // Extract reply subject (if present)
                        let Some(reply_subject) = msg.reply.as_ref() else {
                            tracing::warn!("Received message without reply subject, skipping");
                            continue;
                        };

                        // Process request and send response
                        match Self::process_request(&msg.payload, &handler).await {
                            Ok(response_bytes) => {
                                // Send response back to reply subject using NATS client
                                match nats_client.publish(reply_subject.clone(), response_bytes.into()).await {
                                    Ok(_) => {
                                        tracing::debug!(
                                            reply_subject = %reply_subject,
                                            "Response sent successfully"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            error = %e,
                                            reply_subject = %reply_subject,
                                            "Failed to send response"
                                        );
                                    }
                                }

                                // Ack message
                                if let Err(e) = msg.ack().await {
                                    tracing::error!(error = %e, "Failed to ack message");
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    reply_subject = %reply_subject,
                                    "Failed to process request"
                                );

                                // Try to send error response
                                let error_response = serde_json::json!({
                                    "error": e.to_string(),
                                    "status": "error"
                                });

                                if let Ok(error_bytes) = serde_json::to_vec(&error_response) {
                                    let _ = nats_client.publish(reply_subject.clone(), error_bytes.into()).await;
                                }

                                // Still ack the message to avoid redelivery loop
                                let _ = msg.ack().await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Error receiving message");
                    }
                }
            }

            tracing::info!("Consumer loop ended");
        });

        Ok(())
    }

    /// Process incoming request and return serialized response
    ///
    /// Deserializes the request payload and calls the handler.
    ///
    /// # Test Reference
    /// - Test: `tests/unit/subscriber_test.rs::test_process_request`
    async fn process_request<H>(payload: &[u8], handler: &H) -> EventResult<Vec<u8>>
    where
        H: ReplyHandler,
        H::Request: DeserializeOwned,
        H::Response: Serialize,
    {
        // Deserialize request
        let request: H::Request = serde_json::from_slice(payload)
            .map_err(|e| DmsgError::infrastructure("Failed to deserialize request").with_context("error", e))?;

        // Handle request
        let response = handler.handle(request).await?;

        // Serialize response
        serde_json::to_vec(&response)
            .map_err(|e| DmsgError::infrastructure("Failed to serialize response").with_context("error", e))
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
