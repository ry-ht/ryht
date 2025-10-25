//! Tester Agent Implementation

use super::*;

/// Tester agent for test generation and execution
pub struct TesterAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
}

impl TesterAgent {
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Testing);
        capabilities.insert(Capability::TestGeneration);
        capabilities.insert(Capability::TestExecution);
        capabilities.insert(Capability::CoverageAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
        }
    }
}

impl Agent for TesterAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Tester
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}
