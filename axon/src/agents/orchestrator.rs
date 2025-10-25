//! Orchestrator Agent Implementation

use super::*;

/// Orchestrator agent for coordinating multi-agent workflows
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
