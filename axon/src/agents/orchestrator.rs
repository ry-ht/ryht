//! Orchestrator Agent Implementation
//!
//! # DEPRECATED - Use LeadAgent Instead
//!
//! This is a minimal stub implementation for backwards compatibility.
//! For actual orchestration functionality, use `crate::orchestration::lead_agent::LeadAgent`
//! which provides:
//! - Strategy-based task decomposition
//! - Multi-agent coordination
//! - Context-aware delegation
//! - Session management with Cortex integration
//! - Message bus coordination
//!
//! This stub exists only for:
//! - Type compatibility in legacy code
//! - Agent registry during initialization
//!
//! See: `/Users/taaliman/projects/luxquant/ry-ht/ryht/axon/src/orchestration/lead_agent.rs`

use super::*;

/// Orchestrator agent stub for coordinating multi-agent workflows
///
/// **DEPRECATED:** Use `crate::orchestration::lead_agent::LeadAgent` for actual orchestration.
/// This is a minimal stub kept for backwards compatibility.
pub struct OrchestratorAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
}

impl OrchestratorAgent {
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::TaskDecomposition);
        capabilities.insert(Capability::WorkflowManagement);
        capabilities.insert(Capability::AgentCoordination);
        capabilities.insert(Capability::ResourceAllocation);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
        }
    }
}

impl Agent for OrchestratorAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Orchestrator
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}
