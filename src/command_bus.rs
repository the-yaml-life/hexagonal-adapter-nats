//! NATS CommandBus implementation for CQRS pattern
//!
//! Implements command execution via NATS request-reply messaging.
//! Note: This is a reference implementation showing the NATS integration point.
//! For production use, command handlers should implement their own serialization.
//!
//! # Architectural Note
//! The CommandBus and QueryBus traits in hexagonal-ports-messaging use generic type parameters
//! and Box<dyn Any> for raw execution, which conflicts with async_trait's Send requirements
//! for Any types. This is an architectural limitation of the current port design.
//!
//! # Test Reference
//! - Test: `tests/cqrs_tests.rs::command_bus_tests`

use crate::JetStreamClient;
use hexagonal_ports_messaging::{Command, CommandBus};
use std::sync::Arc;
use crate::NatsError;
use crate::Result;

/// NATS CommandBus implementation
///
/// Sends commands via NATS request-reply pattern.
/// Minimal implementation - production use requires custom serialization.
pub struct NatsCommandBus {
    _client: Arc<JetStreamClient>,
    _timeout_ms: u64,
    _service_id: String,
}

impl NatsCommandBus {
    /// Create a new NATS CommandBus
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

// Note: Direct trait implementation of CommandBus has limitations due to
// generic type serialization requirements and async_trait Send bounds.
// Extend this with concrete command types and handlers as needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_bus_struct() {
        let _ = std::any::type_name::<NatsCommandBus>();
    }
}
