# hexagonal-adapter-nats

NATS JetStream adapter for hexagonal architecture - Event-driven messaging with CloudEvents support.

## Overview

This adapter implements the messaging ports (`Publisher`/`Subscriber`) using NATS JetStream, enabling event-driven architectures with guaranteed delivery, durable consumers, and CloudEvents format.

## Features

- ✅ **JetStream Publishing**: Publish events with guaranteed delivery and acknowledgment
- ✅ **JetStream Subscribing**: Subscribe to event streams with durable consumers
- ✅ **CloudEvents Format**: Standard event format (CloudEvents 1.0 spec)
- ✅ **Outbox Pattern**: Transactional outbox support for reliable event publishing
- ✅ **Retry Logic**: Automatic retry with exponential backoff
- ✅ **Consumer Groups**: Load balancing across multiple subscribers
- ✅ **Wildcard Subscriptions**: Subscribe to multiple event types with patterns

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hexagonal-adapter-nats = "0.3.0"
hexagonal-ports-messaging = "0.3.0"
```

## Quick Start

### Configuration

```rust
use hexagonal_adapter_nats::NatsConfig;

let config = NatsConfig {
    url: "nats://localhost:4222".to_string(),
    stream_name: "EVENTS".to_string(),
    consumer_name: Some("my-service".to_string()),
    ..Default::default()
};
```

### Publishing Events

```rust
use hexagonal_adapter_nats::{JetStreamClient, NatsPublisher};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::from_env()?;
    let client = JetStreamClient::new(config).await?;
    let publisher = NatsPublisher::new(client, "my-service".to_string());

    // TODO: Publish when implementation is complete
    // publisher.publish(&event).await?;

    Ok(())
}
```

### Subscribing to Events

```rust
use hexagonal_adapter_nats::{JetStreamClient, NatsSubscriber};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::from_env()?;
    let client = JetStreamClient::new(config).await?;
    let subscriber = NatsSubscriber::new(client);

    // TODO: Subscribe when implementation is complete
    // subscriber.subscribe("events.>", handler).await?;

    Ok(())
}
```

### Transactional Outbox Pattern

The Outbox pattern ensures reliable event publishing by storing events in the database within the same transaction as business logic:

```rust
use hexagonal_adapter_nats::{OutboxPublisher, OutboxEvent, OutboxRepository};

// 1. In your service transaction:
async fn create_atom(tx: &mut Transaction, atom: Atom) -> Result<()> {
    // Insert business data
    sqlx::query("INSERT INTO atoms (...) VALUES (...)")
        .execute(&mut *tx)
        .await?;

    // Insert event to outbox (same transaction!)
    let event = OutboxEvent::new("atom.created", serde_json::json!({
        "atom_id": atom.id,
        "content": atom.content,
    }));

    sqlx::query("INSERT INTO outbox (event_id, event_type, payload, ...) VALUES ($1, $2, $3, ...)")
        .bind(&event.event_id)
        .bind(&event.event_type)
        .bind(&event.payload)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

// 2. Run the outbox publisher in background:
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_config = NatsConfig::from_env()?;
    let nats_client = JetStreamClient::new(nats_config).await?;
    let nats_publisher = NatsPublisher::new(nats_client, "atom-service".to_string());

    // Create outbox publisher with configuration
    let outbox_publisher = OutboxPublisher::new(nats_publisher)
        .with_poll_interval(Duration::from_millis(100))
        .with_batch_size(100)
        .with_max_attempts(10);

    // Implement OutboxRepository for your database
    let outbox_repo = PostgresOutboxRepository::new(db_pool);

    // Start polling (runs forever)
    tokio::spawn(async move {
        outbox_publisher.start_polling(outbox_repo).await
    });

    Ok(())
}
```

**Benefits**:
- ✅ Atomic: Events and business data are committed together
- ✅ Reliable: Events won't be lost even if NATS is down
- ✅ Retry: Failed events are retried with exponential backoff
- ✅ Dead Letter: Events exceeding max retries are tracked

**Implementing OutboxRepository**:

```rust
use hexagonal_adapter_nats::OutboxRepository;
use async_trait::async_trait;

pub struct PostgresOutboxRepository {
    pool: PgPool,
}

#[async_trait]
impl OutboxRepository for PostgresOutboxRepository {
    async fn get_unpublished_events(&self, limit: usize) -> EventResult<Vec<OutboxEvent>> {
        // SELECT * FROM outbox WHERE published_at IS NULL ORDER BY created_at LIMIT $1
    }

    async fn mark_published(&self, event_id: &str) -> EventResult<()> {
        // UPDATE outbox SET published_at = NOW() WHERE event_id = $1
    }

    async fn mark_failed(&self, event_id: &str, error: &str) -> EventResult<()> {
        // UPDATE outbox SET attempts = attempts + 1, last_error = $2 WHERE event_id = $1
    }

    // ... implement other methods
}
```

## Configuration

Environment variables:

- `NATS_URL`: NATS server URL (default: `nats://localhost:4222`)
- `NATS_STREAM_NAME`: JetStream stream name (default: `EVENTS`)
- `NATS_CONSUMER_NAME`: Consumer name for durable subscriptions (optional)
- `NATS_CONSUMER_GROUP`: Consumer group for load balancing (optional)
- `NATS_MAX_RECONNECT_ATTEMPTS`: Max reconnect attempts (default: `10`)
- `NATS_CONNECTION_TIMEOUT_SECS`: Connection timeout in seconds (default: `5`)
- `NATS_ENABLE_TLS`: Enable TLS (default: `false`)
- `NATS_TLS_CERT_PATH`: TLS certificate path (required if TLS enabled)

## CloudEvents Format

Events are published in CloudEvents 1.0 format:

```json
{
  "specversion": "1.0",
  "type": "second-brain.local.atoms.created",
  "source": "atom-write-service",
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "time": "2025-11-25T10:30:00Z",
  "datacontenttype": "application/json",
  "data": {
    "atom_id": "123e4567-e89b-12d3-a456-426614174000",
    "content_type": "text",
    "created_at": "2025-11-25T10:30:00Z"
  }
}
```

## NATS Subject Patterns

Example subjects for the Bob project:

```
second-brain.local.atoms.created
second-brain.local.atoms.updated
second-brain.local.atoms.deleted
second-brain.local.scopes.assigned
second-brain.local.relationships.created
agents.{agent-id}.messages
agents.broadcast.all
tasks.priority.{high|normal|low}
```

## Development Status

**Current Version**: 0.3.0
**Status**: 🚀 Ready for Integration Testing (Phases 1-7 complete)

### Completed (Phases 1-7)
- ✅ Project structure and Cargo.toml
- ✅ Error handling (`NatsError`)
- ✅ Configuration (`NatsConfig`)
- ✅ JetStream client connection
- ✅ JetStream stream management (`ensure_stream`, `publish`, `subscribe`)
- ✅ Publisher implementation (EventBus trait)
- ✅ Subscriber implementation with durable consumers
- ✅ CloudEvents 1.0 conversion
- ✅ Transactional Outbox Pattern support
- ✅ Exponential backoff retry logic
- ✅ Dead letter event tracking
- ✅ Unit tests (13 passed)

### In Progress (Phase 8)
- 🚧 Integration tests with real NATS server
- 🚧 End-to-end testing

### Planned (Phase 9+)
- ⏳ Advanced monitoring and observability
- ⏳ Performance benchmarks
- ⏳ Additional CloudEvents features

## Testing

### Unit Tests

```bash
cargo test -p hexagonal-adapter-nats --lib
```

### Integration Tests

Requires a running NATS server with JetStream enabled:

```bash
# Start NATS with JetStream
docker run -d --name nats -p 4222:4222 nats:latest --jetstream

# Run integration tests
cargo test -p hexagonal-adapter-nats --test integration_tests -- --ignored
```

## Architecture

This adapter follows the hexagonal architecture pattern:

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Uses Publisher/Subscriber traits)     │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│         Ports (Interfaces)              │
│  hexagonal-ports-messaging              │
│  - Publisher trait                      │
│  - Subscriber trait                     │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│         NATS Adapter                    │
│  hexagonal-adapter-nats                 │
│  - NatsPublisher (implements Publisher) │
│  - NatsSubscriber (implements Sub...)   │
│  - JetStreamClient (NATS connection)    │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│      NATS JetStream Server              │
└─────────────────────────────────────────┘
```

## Contributing

This adapter is part of the hexagonal-template workspace. See the main repository for contribution guidelines.

## License

MIT License - See [LICENSE](LICENSE) for details.

## Links

- **Repository**: https://github.com/the-yaml-life/hexagonal-adapter-nats
- **Hexagonal Template**: https://github.com/the-yaml-life/hexagonal-template
- **NATS JetStream Docs**: https://docs.nats.io/nats-concepts/jetstream
- **CloudEvents Spec**: https://cloudevents.io/

## Related Adapters

- [hexagonal-adapter-redis](../redis) - Redis adapter for caching and pub/sub
- [hexagonal-adapter-postgres](../postgres) - PostgreSQL adapter for data persistence
- [hexagonal-adapter-event_pipeline](../event_pipeline) - Event processing pipeline

## Support

For issues and questions:
- **GitHub Issues**: https://github.com/the-yaml-life/hexagonal-adapter-nats/issues
- **Main Project Issues**: https://github.com/the-yaml-life/hexagonal-template/issues
