//! Agent Types and Implementations
//!
//! This module provides the core agent abstractions and implementations for the Axon
//! multi-agent system. All agents are stateless executors that leverage Cortex for
//! memory, learning, and context management.
//!
//! # Architecture
//!
//! - Agents are stateless - all state lives in Cortex sessions
//! - Each agent type specializes in specific capabilities
//! - Agents communicate via message passing
//! - Compile-time state machine for agent lifecycle
//!
//! # Agent Types
//!
//! - `DeveloperAgent` - Code generation, modification, and refactoring
//! - `ReviewerAgent` - Code review, quality assessment, and validation
//! - `TesterAgent` - Test generation, execution, and coverage analysis
//! - `DocumenterAgent` - Documentation generation and maintenance
//! - `ArchitectAgent` - System design and architecture planning
//! - `ResearcherAgent` - Information gathering and analysis
//! - `OptimizerAgent` - Performance and cost optimization

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

// Re-export submodules
pub mod types;
pub mod capabilities;
pub mod lifecycle;
pub mod developer;
pub mod reviewer;
pub mod tester;
pub mod documenter;
pub mod architect;
pub mod researcher;
pub mod optimizer;
pub mod tool_registry;
pub mod models;
pub mod cc;
pub mod cortex_bridge;

// TEMPORARY: Re-export AgentId from axon's cortex_bridge
// TODO: Move cortex_bridge to cortex in Phase 3
// pub use axon::cortex_bridge::models::AgentId;
// For now, define AgentId locally to avoid circular dependency

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

pub use types::*;
pub use capabilities::*;
pub use lifecycle::*;
pub use developer::DeveloperAgent;
pub use reviewer::ReviewerAgent;
pub use tester::TesterAgent;
pub use documenter::DocumenterAgent;
pub use architect::ArchitectAgent;
pub use researcher::ResearcherAgent;
pub use optimizer::OptimizerAgent;
pub use tool_registry::{ToolRegistry, ToolRegistryStats};

// Re-export types from cortex-types and cortex-intelligence for agent implementations
pub use cortex_types::{SessionId, WorkspaceId};
pub use cortex_intelligence::{
    CortexBridge, Episode, EpisodeOutcome, Pattern, PatternType,
    SessionScope, SearchFilters, UnitFilters,
};
pub use cortex_core::types::{CodeUnit, SearchResult};

// Type alias for convenience
pub type CodeSearchResult = SearchResult<CodeUnit>;

// Additional stub types needed by agents
pub use crate::stub_types::*;

mod stub_types {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum EpisodeType {
        Task,
        Query,
        Learning,
        Error,
        Feature,  // Added for developer agent
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum MergeStrategy {
        Auto,
        Manual,
        ConflictResolution,
    }

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct TokenUsage {
        pub prompt_tokens: usize,
        pub completion_tokens: usize,
        pub total_tokens: usize,
    }
}

/// Core Agent trait that all agent types implement
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent
    fn id(&self) -> &AgentId;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Agent type classification
    fn agent_type(&self) -> AgentType;

    /// Capabilities this agent provides
    fn capabilities(&self) -> &HashSet<Capability>;

    /// Current runtime metrics
    fn metrics(&self) -> &AgentMetrics;
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Agent-specific errors
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    #[error("Cortex integration error: {0}")]
    CortexError(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id = AgentId::new();
        assert!(!id.to_string().is_empty());
    }

    #[test]
    fn test_capability_matching() {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeGeneration);
        capabilities.insert(Capability::CodeReview);

        assert!(capabilities.contains(&Capability::CodeGeneration));
        assert!(!capabilities.contains(&Capability::Testing));
    }
}
