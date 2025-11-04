//! Bridge module to re-export Cortex types
//!
//! This module provides a unified import point for types that cortex-orchestration
//! needs from various Cortex crates.

// Re-export from cortex-types
pub use cortex_types::{SessionId, WorkspaceId};

// Re-export from cortex-agents
pub use cortex_agents::AgentId;

// Re-export from cortex-intelligence
pub use cortex_intelligence::{
    CortexBridge, Episode, EpisodeOutcome, Pattern, PatternType,
};

// Type aliases for convenience
pub type EpisodeId = String;

// Stub types that may not exist yet but are referenced in the code
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeType {
    Task,
    Query,
    Learning,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub tool_name: String,
    pub call_count: usize,
    pub total_duration_ms: u64,
}

// Module for nested types
pub mod models {
    pub use super::{Episode, EpisodeOutcome, EpisodeType, TokenUsage, ToolUsage};
}
