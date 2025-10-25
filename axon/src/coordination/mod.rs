//! Coordination Patterns and Protocols
//!
//! Message bus-based communication, pub/sub system, and topology management
//! for efficient agent coordination.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, RwLock};
use serde::{Deserialize, Serialize};

pub mod message_bus;
pub mod patterns;
pub mod topology;

pub use message_bus::*;
pub use patterns::*;
pub use topology::*;

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
