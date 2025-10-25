//! Developer Agent Implementation

use super::*;

/// Developer agent for code generation and modification
pub struct DeveloperAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
}

impl DeveloperAgent {
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeGeneration);
        capabilities.insert(Capability::CodeRefactoring);
        capabilities.insert(Capability::CodeOptimization);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
        }
    }
}

impl Agent for DeveloperAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Developer
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}
