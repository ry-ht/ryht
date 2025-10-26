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
//! - `OrchestratorAgent` - Coordinates task delegation and workflow management
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
pub mod orchestrator;
pub mod documenter;
pub mod architect;
pub mod researcher;
pub mod optimizer;

pub use types::*;
pub use capabilities::*;
pub use lifecycle::*;
pub use developer::DeveloperAgent;
pub use reviewer::ReviewerAgent;
pub use tester::TesterAgent;
pub use orchestrator::OrchestratorAgent;
pub use documenter::DocumenterAgent;
pub use architect::ArchitectAgent;
pub use researcher::ResearcherAgent;
pub use optimizer::OptimizerAgent;

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
