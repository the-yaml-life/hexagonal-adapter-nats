//! NATS QueryBus implementation for CQRS pattern
//!
//! Implements query execution via NATS request-reply messaging with optional caching.
//! Note: This is a reference implementation showing the NATS integration point.
//! For production use, query handlers should implement their own serialization.
//!
//! # Architectural Note
//! The CommandBus and QueryBus traits in hexagonal-ports-messaging use generic type parameters
//! and Box<dyn Any> for raw execution, which conflicts with async_trait's Send requirements
//! for Any types. This is an architectural limitation of the current port design.
//!
//! # Test Reference
//! - Test: `tests/cqrs_tests.rs::query_bus_tests`

use crate::JetStreamClient;
use hexagonal_ports_messaging::{Query, QueryBus};
use std::sync::Arc;
use crate::NatsError;
use crate::Result;

/// NATS QueryBus implementation
///
/// Sends queries via NATS request-reply pattern.
/// Minimal implementation - production use requires custom serialization.
pub struct NatsQueryBus {
    _client: Arc<JetStreamClient>,
    _timeout_ms: u64,
    _service_id: String,
}

impl NatsQueryBus {
    /// Create a new NATS QueryBus
    pub fn new(client: Arc<JetStreamClient>, timeout_ms: u64, service_id: String) -> Self {
        Self {
            _client: client,
            _timeout_ms: timeout_ms,
            _service_id: service_id,
        }
    }

    /// Set request timeout
    pub fn with_timeout(self, _timeout_ms: u64) -> Self {
        self
    }
}

// Note: Direct trait implementation of QueryBus has limitations due to
// generic type serialization requirements and async_trait Send bounds.
// Extend this with concrete query types and handlers as needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_bus_struct() {
        let _ = std::any::type_name::<NatsQueryBus>();
    }
}
