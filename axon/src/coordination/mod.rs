//! Coordination Patterns and Protocols
//!
//! Unified messaging system integrating with Cortex for persistent, intelligent,
//! and resilient multi-agent coordination. Includes message bus, pub/sub,
//! distributed locking, episodic memory integration, and coordination patterns.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, RwLock};
use serde::{Deserialize, Serialize};

pub mod message_bus;
pub mod patterns;
pub mod topology;
pub mod unified_message_bus;
pub mod message_coordinator;
pub mod agent_messaging_adapter;

pub use message_bus::*;
pub use patterns::*;
pub use topology::*;
pub use unified_message_bus::*;
pub use message_coordinator::*;
pub use agent_messaging_adapter::*;

/// Main coordination pattern trait
pub trait CoordinationPattern: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

/// Result type for coordination operations
pub type Result<T> = std::result::Result<T, CoordinationError>;

/// Coordination errors
#[derive(Debug, thiserror::Error)]
pub enum CoordinationError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Send failed to {target}")]
    SendFailed { target: String },

    #[error("Publish failed to topic {topic}")]
    PublishFailed { topic: String },

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
