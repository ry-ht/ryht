//! Core types and abstractions for the Cortex cognitive memory system.
//!
//! This crate provides the foundational types, traits, and error handling
//! used across all Cortex components.

pub mod error;
pub mod types;
pub mod traits;
pub mod id;
pub mod metadata;
pub mod config;

pub use error::{CortexError, Result};
pub use types::*;
pub use traits::*;
pub use id::CortexId;
pub use config::{GlobalConfig, ConfigManager, ConfigProfile, ConfigMetadata};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::error::{CortexError, Result};
    pub use crate::types::*;
    pub use crate::traits::*;
    pub use crate::id::CortexId;
    pub use crate::config::{GlobalConfig, ConfigManager, ConfigProfile, ConfigMetadata};
}
