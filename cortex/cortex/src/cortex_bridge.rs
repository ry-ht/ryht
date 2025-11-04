//! Bridge module for Cortex types and functionality
//!
//! This module re-exports types from cortex-types and provides stub
//! implementations for missing types during migration.

// Re-export types from cortex-types
pub use cortex_types::{AgentId, SessionId, WorkspaceId, AgentType, AgentStatus};

// Re-export Episode and Pattern from cortex-intelligence
pub use cortex_intelligence::{Episode, EpisodeOutcome, Pattern};

// Additional stub types needed by MCP tools
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merge strategy for sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    Auto,
    Manual,
    ConflictResolution,
}

/// Session scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionScope {
    Workspace,
    Global,
    Temporary,
}

/// Episode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeType {
    Task,
    Query,
    Learning,
    Error,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Code,
    Architecture,
    Bug,
    Optimization,
}

/// Search filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
}

/// Unit filters for code units
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitFilters {
    pub language: Option<String>,
    pub unit_type: Option<String>,
    pub min_score: Option<f32>,
    pub limit: Option<usize>,
}

/// Token usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Stub CortexBridge for MCP tool compatibility
pub struct CortexBridge {
    // TODO: Implement actual bridge to cortex subsystems
    _placeholder: (),
}

impl CortexBridge {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl Default for CortexBridge {
    fn default() -> Self {
        Self::new()
    }
}
