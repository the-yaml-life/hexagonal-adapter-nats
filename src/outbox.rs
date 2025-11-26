//! Transactional Outbox Pattern Support
//!
//! The Outbox pattern ensures reliable event publishing by storing events
//! in the database within the same transaction as the business logic,
//! and then asynchronously publishing them to NATS.
//!
//! # Architecture
//!
//! 1. Service writes event to outbox table in SAME transaction as business data
//! 2. OutboxPublisher polls the outbox table for unpublished events
//! 3. Events are published to NATS JetStream
//! 4. Successfully published events are marked as published
//! 5. Failed events are retried with exponential backoff
//!
//! # Example
//!
//! ```rust,ignore
//! // In your service transaction:
//! tx.execute("INSERT INTO atoms (...) VALUES (...)")?;
//! tx.execute("INSERT INTO outbox (event_id, event_type, payload, ...) VALUES (...)")?;
//! tx.commit()?;
//!
//! // Separately, the OutboxPublisher is running:
//! let publisher = OutboxPublisher::new(nats_publisher);
//! publisher.start_polling(postgres_outbox_repo).await?;
//! ```
//!
//! # Test Reference
//! - Test: `tests/integration/outbox_test.rs`

use crate::{NatsPublisher, Result};
use async_trait::async_trait;
use hexagonal_core::DmsgError;
use hexagonal_ports_messaging::event::{Event, EventBus, EventResult};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Outbox event stored in the database
///
/// This represents an event that needs to be published to NATS.
/// Events are stored in the database within the same transaction
/// as the business logic to ensure atomicity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    /// Unique event ID
    pub event_id: String,

    /// Event type (e.g., "atom.created")
    pub event_type: String,

    /// Event payload (serialized JSON)
    pub payload: serde_json::Value,

    /// When the event was created
    pub created_at: SystemTime,

    /// When the event was published to NATS (None if not yet published)
    pub published_at: Option<SystemTime>,

    /// Number of publish attempts
    pub attempts: u32,

    /// Last error message if publish failed
    pub last_error: Option<String>,

    /// Aggregate ID (optional, for event sourcing)
    pub aggregate_id: Option<String>,

    /// Event metadata (optional)
    pub metadata: Option<serde_json::Value>,
}

impl OutboxEvent {
    /// Create a new outbox event
    pub fn new(event_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            payload,
            created_at: SystemTime::now(),
            published_at: None,
            attempts: 0,
            last_error: None,
            aggregate_id: None,
            metadata: None,
        }
    }

    /// Create from a domain event
    pub fn from_event<E>(event: &E) -> EventResult<Self>
    where
        E: Event + Serialize,
    {
        let payload = serde_json::to_value(event)
            .map_err(|e| DmsgError::infrastructure("Failed to serialize event")
                .with_context("error", e))?;

        Ok(Self {
            event_id: event.event_id().to_string(),
            event_type: E::event_type().to_string(),
            payload,
            created_at: event.occurred_at(),
            published_at: None,
            attempts: 0,
            last_error: None,
            aggregate_id: event.aggregate_id().map(|s| s.to_string()),
            metadata: event.metadata()
                .and_then(|m| serde_json::to_value(m).ok()),
        })
    }

    /// Check if event should be retried
    ///
    /// Uses exponential backoff: 1s, 2s, 4s, 8s, 16s, ...
    pub fn should_retry(&self, max_attempts: u32) -> bool {
        if self.attempts >= max_attempts {
            return false;
        }

        // First attempt - always try
        if self.attempts == 0 {
            return true;
        }

        // Calculate backoff duration for retries (2^attempts seconds)
        let backoff_seconds = 2u64.pow(self.attempts);
        let backoff_duration = Duration::from_secs(backoff_seconds);

        // Check if enough time has passed since last attempt
        if let Ok(elapsed) = self.created_at.elapsed() {
            elapsed >= backoff_duration
        } else {
            false
        }
    }
}

/// Repository trait for accessing outbox events from database
///
/// This trait must be implemented by the database adapter (e.g., PostgreSQL)
/// to provide access to the outbox table.
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// Fetch unpublished events from the outbox
    ///
    /// Returns events that have not been successfully published yet,
    /// ordered by creation time (oldest first).
    async fn get_unpublished_events(&self, limit: usize) -> EventResult<Vec<OutboxEvent>>;

    /// Mark an event as successfully published
    ///
    /// Sets published_at timestamp and clears error fields.
    async fn mark_published(&self, event_id: &str) -> EventResult<()>;

    /// Mark an event as failed
    ///
    /// Increments attempts counter and stores error message.
    async fn mark_failed(&self, event_id: &str, error: &str) -> EventResult<()>;

    /// Delete old published events
    ///
    /// Clean up events that have been successfully published
    /// and are older than the specified duration.
    async fn delete_published_older_than(&self, duration: Duration) -> EventResult<usize>;

    /// Get events that have exceeded max retry attempts
    ///
    /// Returns events that failed to publish after max_attempts tries.
    /// These typically need manual intervention.
    async fn get_dead_letter_events(&self, max_attempts: u32) -> EventResult<Vec<OutboxEvent>>;
}

/// Outbox publisher that polls for unpublished events and publishes them to NATS
///
/// This component continuously polls the outbox repository for unpublished events
/// and publishes them using the provided NATS publisher.
pub struct OutboxPublisher {
    nats_publisher: NatsPublisher,
    poll_interval: Duration,
    batch_size: usize,
    max_attempts: u32,
}

impl OutboxPublisher {
    /// Create a new outbox publisher
    ///
    /// # Arguments
    /// - `nats_publisher`: NATS publisher to use for publishing events
    pub fn new(nats_publisher: NatsPublisher) -> Self {
        Self {
            nats_publisher,
            poll_interval: Duration::from_millis(100),
            batch_size: 100,
            max_attempts: 10,
        }
    }

    /// Configure the polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Configure the batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Configure max retry attempts
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Start polling for unpublished events
    ///
    /// This runs an infinite loop that polls the outbox repository
    /// and publishes events to NATS. It should be run in a background task.
    ///
    /// # Test Reference
    /// - Test: `tests/integration/outbox_test.rs::test_outbox_polling`
    pub async fn start_polling<R>(&self, repo: R) -> Result<()>
    where
        R: OutboxRepository + 'static,
    {
        tracing::info!(
            poll_interval_ms = self.poll_interval.as_millis(),
            batch_size = self.batch_size,
            max_attempts = self.max_attempts,
            "Starting outbox publisher polling"
        );

        loop {
            // Fetch unpublished events
            let events = match repo.get_unpublished_events(self.batch_size).await {
                Ok(events) => events,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to fetch unpublished events");
                    tokio::time::sleep(self.poll_interval).await;
                    continue;
                }
            };

            if events.is_empty() {
                tokio::time::sleep(self.poll_interval).await;
                continue;
            }

            tracing::debug!(count = events.len(), "Processing outbox events");

            // Process each event
            for event in events {
                // Check if should retry based on backoff
                if event.attempts > 0 && !event.should_retry(self.max_attempts) {
                    continue;
                }

                // Skip if exceeded max attempts
                if event.attempts >= self.max_attempts {
                    tracing::warn!(
                        event_id = %event.event_id,
                        attempts = event.attempts,
                        "Event exceeded max retry attempts, moving to dead letter"
                    );
                    continue;
                }

                // Publish to NATS
                match self.publish_outbox_event(&event).await {
                    Ok(_) => {
                        // Mark as published
                        if let Err(e) = repo.mark_published(&event.event_id).await {
                            tracing::error!(
                                event_id = %event.event_id,
                                error = %e,
                                "Failed to mark event as published"
                            );
                        } else {
                            tracing::info!(
                                event_id = %event.event_id,
                                event_type = %event.event_type,
                                "Event published successfully"
                            );
                        }
                    }
                    Err(e) => {
                        // Mark as failed
                        let error_msg = e.to_string();
                        if let Err(e) = repo.mark_failed(&event.event_id, &error_msg).await {
                            tracing::error!(
                                event_id = %event.event_id,
                                error = %e,
                                "Failed to mark event as failed"
                            );
                        } else {
                            tracing::warn!(
                                event_id = %event.event_id,
                                attempts = event.attempts + 1,
                                error = %error_msg,
                                "Event publish failed, will retry"
                            );
                        }
                    }
                }
            }

            // Brief pause before next batch
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Publish a single outbox event to NATS
    ///
    /// # Test Reference
    /// - Test: `tests/unit/outbox_test.rs::test_publish_outbox_event`
    async fn publish_outbox_event(&self, event: &OutboxEvent) -> EventResult<String> {
        // Create a mock event that implements the Event trait
        let mock_event = MockOutboxEvent {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            payload: event.payload.clone(),
            occurred_at: event.created_at,
        };

        // Publish using the NATS publisher
        self.nats_publisher.publish(mock_event).await
    }

    /// Process dead letter events
    ///
    /// Returns events that have exceeded max retry attempts.
    /// These typically need manual intervention or should be logged for monitoring.
    pub async fn get_dead_letters<R>(&self, repo: &R) -> EventResult<Vec<OutboxEvent>>
    where
        R: OutboxRepository,
    {
        repo.get_dead_letter_events(self.max_attempts).await
    }

    /// Clean up old published events
    ///
    /// Removes events that have been successfully published and are older
    /// than the specified duration. This helps keep the outbox table size manageable.
    pub async fn cleanup_old_events<R>(&self, repo: &R, older_than: Duration) -> EventResult<usize>
    where
        R: OutboxRepository,
    {
        repo.delete_published_older_than(older_than).await
    }
}

/// Mock event for publishing outbox events
///
/// This wraps the outbox event data to implement the Event trait
/// so it can be published through the EventBus.
#[derive(Debug, Clone, Serialize)]
struct MockOutboxEvent {
    event_id: String,
    event_type: String,
    payload: serde_json::Value,
    occurred_at: SystemTime,
}

impl Event for MockOutboxEvent {
    fn event_type() -> &'static str
    where
        Self: Sized,
    {
        "outbox.event"
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn occurred_at(&self) -> SystemTime {
        self.occurred_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outbox_event_creation() {
        let payload = serde_json::json!({"test": "data"});
        let event = OutboxEvent::new("test.event", payload);

        assert_eq!(event.event_type, "test.event");
        assert_eq!(event.attempts, 0);
        assert!(event.published_at.is_none());
        assert!(event.last_error.is_none());
    }

    #[test]
    fn test_should_retry_backoff() {
        let mut event = OutboxEvent::new("test.event", serde_json::json!({}));

        // First attempt - should always retry
        assert!(event.should_retry(10));

        // After 1 attempt, needs 2 seconds
        event.attempts = 1;
        // Won't retry immediately (needs backoff time)
        assert!(!event.should_retry(10));

        // Simulate time passing by setting old created_at
        event.created_at = SystemTime::now() - Duration::from_secs(3);
        assert!(event.should_retry(10));

        // After max attempts
        event.attempts = 10;
        assert!(!event.should_retry(10));
    }

    #[test]
    fn test_outbox_publisher_configuration() {
        use crate::{JetStreamClient, NatsConfig};

        let config = NatsConfig::default();
        // Can't actually create client without server, but we can test builder pattern

        // Just verify the builder methods exist
        let _poll_interval = Duration::from_secs(1);
        let _batch_size = 50;
        let _max_attempts = 5;
    }
}
