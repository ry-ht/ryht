//! Agent Capabilities
//!
//! Defines the capabilities that agents can provide and the system for matching
//! agents to tasks based on required capabilities.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use super::AgentId;

/// Core capabilities that agents can provide
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    // Code Operations
    CodeGeneration,
    CodeReview,
    CodeRefactoring,
    CodeOptimization,
    CodeAnalysis,

    // Testing
    Testing,
    TestGeneration,
    TestExecution,
    CoverageAnalysis,

    // Documentation
    Documentation,
    DocGeneration,
    DiagramCreation,

    // Analysis
    StaticAnalysis,
    SecurityAnalysis,
    PerformanceAnalysis,
    DependencyAnalysis,

    // Design
    SystemDesign,
    APIDesign,
    DatabaseDesign,
    ArchitectureAnalysis,

    // Research
    InformationRetrieval,
    FactChecking,
    TrendAnalysis,
    TechnologyResearch,

    // Orchestration
    TaskDecomposition,
    WorkflowManagement,
    AgentCoordination,
    ResourceAllocation,

    // Optimization
    CostOptimization,
    PerformanceOptimization,
    ResourceOptimization,

    // Communication
    NaturalLanguageProcessing,
    CodeExplanation,
    TechnicalWriting,

    // Specialized
    Refactoring,
    Debugging,
    DebuggingAssistance,
    SecurityAudit,
    ComplianceCheck,
}

impl Capability {
    /// Get human-readable description of capability
    pub fn description(&self) -> &'static str {
        match self {
            Self::CodeGeneration => "Generate code from specifications",
            Self::CodeReview => "Review code for quality and correctness",
            Self::CodeRefactoring => "Refactor code for better structure",
            Self::CodeOptimization => "Optimize code for performance",
            Self::CodeAnalysis => "Analyze code structure and complexity",

            Self::Testing => "General testing capabilities",
            Self::TestGeneration => "Generate comprehensive test suites",
            Self::TestExecution => "Execute tests and analyze results",
            Self::CoverageAnalysis => "Analyze test coverage",

            Self::Documentation => "General documentation capabilities",
            Self::DocGeneration => "Generate documentation from code",
            Self::DiagramCreation => "Create diagrams and visualizations",

            Self::StaticAnalysis => "Perform static code analysis",
            Self::SecurityAnalysis => "Analyze security vulnerabilities",
            Self::PerformanceAnalysis => "Analyze performance characteristics",
            Self::DependencyAnalysis => "Analyze code dependencies",

            Self::SystemDesign => "Design system architecture",
            Self::APIDesign => "Design API interfaces",
            Self::DatabaseDesign => "Design database schemas",
            Self::ArchitectureAnalysis => "Analyze system architecture",

            Self::InformationRetrieval => "Retrieve and synthesize information",
            Self::FactChecking => "Verify facts and claims",
            Self::TrendAnalysis => "Analyze trends and patterns",
            Self::TechnologyResearch => "Research technologies and tools",

            Self::TaskDecomposition => "Break down complex tasks",
            Self::WorkflowManagement => "Manage multi-step workflows",
            Self::AgentCoordination => "Coordinate multiple agents",
            Self::ResourceAllocation => "Allocate resources efficiently",

            Self::CostOptimization => "Optimize costs",
            Self::PerformanceOptimization => "Optimize performance",
            Self::ResourceOptimization => "Optimize resource usage",

            Self::NaturalLanguageProcessing => "Process natural language",
            Self::CodeExplanation => "Explain code functionality",
            Self::TechnicalWriting => "Write technical documentation",

            Self::Refactoring => "Refactor and improve code",
            Self::Debugging => "Debug and fix issues",
            Self::DebuggingAssistance => "Assist with debugging",
            Self::SecurityAudit => "Perform security audits",
            Self::ComplianceCheck => "Check compliance with standards",
        }
    }

    /// Get category of this capability
    pub fn category(&self) -> CapabilityCategory {
        match self {
            Self::CodeGeneration
            | Self::CodeReview
            | Self::CodeRefactoring
            | Self::CodeOptimization
            | Self::CodeAnalysis => CapabilityCategory::Code,

            Self::Testing
            | Self::TestGeneration
            | Self::TestExecution
            | Self::CoverageAnalysis => CapabilityCategory::Testing,

            Self::Documentation
            | Self::DocGeneration
            | Self::DiagramCreation => CapabilityCategory::Documentation,

            Self::StaticAnalysis
            | Self::SecurityAnalysis
            | Self::PerformanceAnalysis
            | Self::DependencyAnalysis => CapabilityCategory::Analysis,

            Self::SystemDesign
            | Self::APIDesign
            | Self::DatabaseDesign
            | Self::ArchitectureAnalysis => CapabilityCategory::Design,

            Self::InformationRetrieval
            | Self::FactChecking
            | Self::TrendAnalysis
            | Self::TechnologyResearch => CapabilityCategory::Research,

            Self::TaskDecomposition
            | Self::WorkflowManagement
            | Self::AgentCoordination
            | Self::ResourceAllocation => CapabilityCategory::Orchestration,

            Self::CostOptimization
            | Self::PerformanceOptimization
            | Self::ResourceOptimization => CapabilityCategory::Optimization,

            Self::NaturalLanguageProcessing
            | Self::CodeExplanation
            | Self::TechnicalWriting => CapabilityCategory::Communication,

            Self::Refactoring
            | Self::Debugging
            | Self::DebuggingAssistance
            | Self::SecurityAudit
            | Self::ComplianceCheck => CapabilityCategory::Specialized,
        }
    }
}

/// Category grouping for capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityCategory {
    Code,
    Testing,
    Documentation,
    Analysis,
    Design,
    Research,
    Orchestration,
    Optimization,
    Communication,
    Specialized,
}

/// Matches agents to tasks based on capabilities
pub struct CapabilityMatcher {
    /// Map of agent IDs to their capabilities
    agent_capabilities: HashMap<AgentId, HashSet<Capability>>,

    /// Map of task types to required capabilities
    task_requirements: HashMap<String, HashSet<Capability>>,
}

impl CapabilityMatcher {
    /// Create a new capability matcher
    pub fn new() -> Self {
        Self {
            agent_capabilities: HashMap::new(),
            task_requirements: HashMap::new(),
        }
    }

    /// Register an agent with its capabilities
    pub fn register_agent(&mut self, agent_id: AgentId, capabilities: HashSet<Capability>) {
        self.agent_capabilities.insert(agent_id, capabilities);
    }

    /// Unregister an agent
    pub fn unregister_agent(&mut self, agent_id: &AgentId) {
        self.agent_capabilities.remove(agent_id);
    }

    /// Register task requirements
    pub fn register_task_requirements(&mut self, task_type: String, capabilities: HashSet<Capability>) {
        self.task_requirements.insert(task_type, capabilities);
    }

    /// Find all agents capable of handling a specific capability
    pub fn find_capable_agents(&self, required: &HashSet<Capability>) -> Vec<AgentId> {
        self.agent_capabilities
            .iter()
            .filter(|(_, caps)| required.is_subset(caps))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Find the best agent for a set of required capabilities
    pub fn find_best_agent(&self, required: &HashSet<Capability>) -> Option<AgentId> {
        self.agent_capabilities
            .iter()
            .filter(|(_, caps)| required.is_subset(caps))
            .max_by_key(|(_, caps)| {
                // Prefer agents with capabilities closer to requirements (fewer extra capabilities)
                let extra = caps.difference(required).count();
                usize::MAX - extra
            })
            .map(|(id, _)| id.clone())
    }

    /// Score how well an agent matches required capabilities
    pub fn score_match(&self, agent_id: &AgentId, required: &HashSet<Capability>) -> f32 {
        if let Some(caps) = self.agent_capabilities.get(agent_id) {
            let matched = required.intersection(caps).count() as f32;
            let total = required.len() as f32;

            if total > 0.0 {
                matched / total
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Get all agents by capability category
    pub fn agents_by_category(&self, category: CapabilityCategory) -> Vec<AgentId> {
        self.agent_capabilities
            .iter()
            .filter(|(_, caps)| {
                caps.iter().any(|cap| cap.category() == category)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl Default for CapabilityMatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Requirements for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    /// Required capabilities
    pub capabilities: Vec<Capability>,

    /// Estimated duration
    pub estimated_duration: std::time::Duration,

    /// Resource requirements
    pub resources: ResourceRequirements,

    /// Priority level
    pub priority: u32,
}

/// Resource requirements for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU cores needed
    pub cpu_cores: u32,

    /// Memory in MB
    pub memory_mb: u64,

    /// GPU required
    pub gpu_required: bool,

    /// Estimated tokens (for LLM tasks)
    pub estimated_tokens: Option<usize>,

    /// Maximum cost in cents
    pub max_cost_cents: Option<u64>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            memory_mb: 512,
            gpu_required: false,
            estimated_tokens: None,
            max_cost_cents: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_description() {
        let cap = Capability::CodeGeneration;
        assert_eq!(cap.description(), "Generate code from specifications");
    }

    #[test]
    fn test_capability_category() {
        assert_eq!(Capability::CodeGeneration.category(), CapabilityCategory::Code);
        assert_eq!(Capability::Testing.category(), CapabilityCategory::Testing);
    }

    #[test]
    fn test_capability_matcher() {
        let mut matcher = CapabilityMatcher::new();

        let agent1 = AgentId::from_string("agent1");
        let mut caps1 = HashSet::new();
        caps1.insert(Capability::CodeGeneration);
        caps1.insert(Capability::Testing);

        matcher.register_agent(agent1.clone(), caps1);

        let mut required = HashSet::new();
        required.insert(Capability::CodeGeneration);

        let capable = matcher.find_capable_agents(&required);
        assert_eq!(capable.len(), 1);
        assert_eq!(capable[0], agent1);
    }

    #[test]
    fn test_capability_scoring() {
        let mut matcher = CapabilityMatcher::new();

        let agent1 = AgentId::from_string("agent1");
        let mut caps = HashSet::new();
        caps.insert(Capability::CodeGeneration);
        caps.insert(Capability::Testing);

        matcher.register_agent(agent1.clone(), caps);

        let mut required = HashSet::new();
        required.insert(Capability::CodeGeneration);

        let score = matcher.score_match(&agent1, &required);
        assert_eq!(score, 1.0);
    }
}
