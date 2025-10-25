//! Reviewer Agent Implementation

use super::*;

/// Reviewer agent for code review and quality assessment
pub struct ReviewerAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
}

impl ReviewerAgent {
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeReview);
        capabilities.insert(Capability::StaticAnalysis);
        capabilities.insert(Capability::SecurityAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
        }
    }
}

impl Agent for ReviewerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Reviewer
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}
