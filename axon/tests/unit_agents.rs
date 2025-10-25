//! Unit tests for all agent types
//!
//! Tests cover:
//! - Agent creation and initialization
//! - Capability verification
//! - Agent type classification
//! - Metrics tracking
//! - Agent lifecycle
//! - Edge cases and error handling

mod common;

use axon::agents::*;
use std::collections::HashSet;
use std::sync::atomic::Ordering;

// ============================================================================
// Developer Agent Tests
// ============================================================================

#[test]
fn test_developer_agent_creation() {
    let agent = DeveloperAgent::new("test-dev".to_string());
    assert_eq!(agent.name(), "test-dev");
    assert_eq!(agent.agent_type(), AgentType::Developer);
    assert!(!agent.id().to_string().is_empty());
}

#[test]
fn test_developer_agent_capabilities() {
    let agent = DeveloperAgent::new("dev-1".to_string());
    let caps = agent.capabilities();

    // Developer should have code-related capabilities
    assert!(caps.contains(&Capability::CodeGeneration));
    assert!(caps.contains(&Capability::CodeRefactoring));
    assert!(caps.contains(&Capability::CodeOptimization));

    // Should not have testing capabilities
    assert!(!caps.contains(&Capability::Testing));
}

#[test]
fn test_developer_agent_metrics() {
    let agent = DeveloperAgent::new("dev-metrics".to_string());
    let metrics = agent.metrics();

    // Initial metrics should be zero
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_completed, 0);
    assert_eq!(snapshot.tasks_failed, 0);
    assert_eq!(snapshot.success_rate, 100);
}

#[test]
fn test_developer_agent_id_uniqueness() {
    let agent1 = DeveloperAgent::new("dev-1".to_string());
    let agent2 = DeveloperAgent::new("dev-2".to_string());

    // Each agent should have unique ID
    assert_ne!(agent1.id(), agent2.id());
}

// ============================================================================
// Reviewer Agent Tests
// ============================================================================

#[test]
fn test_reviewer_agent_creation() {
    let agent = ReviewerAgent::new("test-reviewer".to_string());
    assert_eq!(agent.name(), "test-reviewer");
    assert_eq!(agent.agent_type(), AgentType::Reviewer);
}

#[test]
fn test_reviewer_agent_capabilities() {
    let agent = ReviewerAgent::new("rev-1".to_string());
    let caps = agent.capabilities();

    // Reviewer should have review capabilities
    assert!(caps.contains(&Capability::CodeReview));
    assert!(caps.contains(&Capability::StaticAnalysis));
    assert!(caps.contains(&Capability::SecurityAnalysis));

    // Should not have code generation
    assert!(!caps.contains(&Capability::CodeGeneration));
}

#[test]
fn test_reviewer_agent_multiple_instances() {
    let rev1 = ReviewerAgent::new("reviewer-1".to_string());
    let rev2 = ReviewerAgent::new("reviewer-2".to_string());

    assert_ne!(rev1.id(), rev2.id());
    assert_eq!(rev1.capabilities(), rev2.capabilities());
}

// ============================================================================
// Tester Agent Tests
// ============================================================================

#[test]
fn test_tester_agent_creation() {
    let agent = TesterAgent::new("test-tester".to_string());
    assert_eq!(agent.name(), "test-tester");
    assert_eq!(agent.agent_type(), AgentType::Tester);
}

#[test]
fn test_tester_agent_capabilities() {
    let agent = TesterAgent::new("tester-1".to_string());
    let caps = agent.capabilities();

    // Tester should have testing capabilities
    assert!(caps.contains(&Capability::Testing));
    assert!(caps.contains(&Capability::TestGeneration));
    assert!(caps.contains(&Capability::TestExecution));
    assert!(caps.contains(&Capability::CoverageAnalysis));
}

#[test]
fn test_tester_agent_capability_count() {
    let agent = TesterAgent::new("tester-cap".to_string());
    let caps = agent.capabilities();

    // Tester should have exactly 4 capabilities
    assert_eq!(caps.len(), 4);
}

// ============================================================================
// Documenter Agent Tests
// ============================================================================

#[test]
fn test_documenter_agent_creation() {
    let agent = DocumenterAgent::new("test-doc".to_string());
    assert_eq!(agent.name(), "test-doc");
    assert_eq!(agent.agent_type(), AgentType::Documenter);
}

#[test]
fn test_documenter_agent_capabilities() {
    let agent = DocumenterAgent::new("doc-1".to_string());
    let caps = agent.capabilities();

    // Documenter should have documentation capabilities
    assert!(caps.contains(&Capability::Documentation));
    assert!(caps.contains(&Capability::DocGeneration));
    assert!(caps.contains(&Capability::TechnicalWriting));
}

// ============================================================================
// Architect Agent Tests
// ============================================================================

#[test]
fn test_architect_agent_creation() {
    let agent = ArchitectAgent::new("test-arch".to_string());
    assert_eq!(agent.name(), "test-arch");
    assert_eq!(agent.agent_type(), AgentType::Architect);
}

#[test]
fn test_architect_agent_capabilities() {
    let agent = ArchitectAgent::new("arch-1".to_string());
    let caps = agent.capabilities();

    // Architect should have design capabilities
    assert!(caps.contains(&Capability::SystemDesign));
    assert!(caps.contains(&Capability::APIDesign));
    assert!(caps.contains(&Capability::ArchitectureAnalysis));
}

// ============================================================================
// Researcher Agent Tests
// ============================================================================

#[test]
fn test_researcher_agent_creation() {
    let agent = ResearcherAgent::new("test-researcher".to_string());
    assert_eq!(agent.name(), "test-researcher");
    assert_eq!(agent.agent_type(), AgentType::Researcher);
}

#[test]
fn test_researcher_agent_capabilities() {
    let agent = ResearcherAgent::new("res-1".to_string());
    let caps = agent.capabilities();

    // Researcher should have research capabilities
    assert!(caps.contains(&Capability::InformationRetrieval));
    assert!(caps.contains(&Capability::TechnologyResearch));
}

// ============================================================================
// Optimizer Agent Tests
// ============================================================================

#[test]
fn test_optimizer_agent_creation() {
    let agent = OptimizerAgent::new("test-opt".to_string());
    assert_eq!(agent.name(), "test-opt");
    assert_eq!(agent.agent_type(), AgentType::Optimizer);
}

#[test]
fn test_optimizer_agent_capabilities() {
    let agent = OptimizerAgent::new("opt-1".to_string());
    let caps = agent.capabilities();

    // Optimizer should have optimization capabilities
    assert!(caps.contains(&Capability::PerformanceOptimization));
    assert!(caps.contains(&Capability::CostOptimization));
}

// ============================================================================
// Orchestrator Agent Tests
// ============================================================================

#[test]
fn test_orchestrator_agent_creation() {
    let agent = OrchestratorAgent::new("test-orch".to_string());
    assert_eq!(agent.name(), "test-orch");
    assert_eq!(agent.agent_type(), AgentType::Orchestrator);
}

#[test]
fn test_orchestrator_agent_capabilities() {
    let agent = OrchestratorAgent::new("orch-1".to_string());
    let caps = agent.capabilities();

    // Orchestrator should have orchestration capabilities
    assert!(caps.contains(&Capability::TaskDecomposition));
    assert!(caps.contains(&Capability::WorkflowManagement));
    assert!(caps.contains(&Capability::AgentCoordination));
}

// ============================================================================
// Agent Metrics Tests
// ============================================================================

#[test]
fn test_agent_metrics_success_tracking() {
    let metrics = AgentMetrics::new();

    // Record successful task
    metrics.record_success(100, 1000, 50);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_completed, 1);
    assert_eq!(snapshot.tokens_used, 1000);
    assert_eq!(snapshot.total_cost_cents, 50);
    assert_eq!(snapshot.avg_task_duration_ms, 100);
    assert_eq!(snapshot.success_rate, 100);
}

#[test]
fn test_agent_metrics_failure_tracking() {
    let metrics = AgentMetrics::new();

    // Record failure
    metrics.record_failure();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_failed, 1);
    assert_eq!(snapshot.success_rate, 0);
}

#[test]
fn test_agent_metrics_success_rate_calculation() {
    let metrics = AgentMetrics::new();

    // Record mixed results
    metrics.record_success(100, 1000, 50);
    metrics.record_success(200, 2000, 100);
    metrics.record_failure();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_completed, 2);
    assert_eq!(snapshot.tasks_failed, 1);
    // Success rate should be 66% (2/3)
    assert_eq!(snapshot.success_rate, 66);
}

#[test]
fn test_agent_metrics_average_duration() {
    let metrics = AgentMetrics::new();

    // Record tasks with different durations
    metrics.record_success(100, 1000, 50);
    metrics.record_success(200, 1000, 50);

    let snapshot = metrics.snapshot();
    // Average should be 150ms
    assert_eq!(snapshot.avg_task_duration_ms, 150);
}

#[test]
fn test_agent_metrics_concurrent_updates() {
    let metrics = AgentMetrics::new();

    // Simulate concurrent updates
    for _ in 0..10 {
        metrics.record_success(100, 100, 10);
    }

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_completed, 10);
    assert_eq!(snapshot.tokens_used, 1000);
    assert_eq!(snapshot.total_cost_cents, 100);
}

// ============================================================================
// Agent ID Tests
// ============================================================================

#[test]
fn test_agent_id_creation() {
    let id = AgentId::new();
    assert!(!id.to_string().is_empty());
}

#[test]
fn test_agent_id_uniqueness() {
    let id1 = AgentId::new();
    let id2 = AgentId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_agent_id_from_string() {
    let id = AgentId::from_string("custom-agent-id");
    assert_eq!(id.to_string(), "custom-agent-id");
}

#[test]
fn test_agent_id_system() {
    let id = AgentId::system();
    assert_eq!(id.to_string(), "system");
}

// ============================================================================
// Capability Tests
// ============================================================================

#[test]
fn test_capability_description() {
    assert_eq!(
        Capability::CodeGeneration.description(),
        "Generate code from specifications"
    );
    assert_eq!(
        Capability::Testing.description(),
        "General testing capabilities"
    );
}

#[test]
fn test_capability_category() {
    assert_eq!(
        Capability::CodeGeneration.category(),
        CapabilityCategory::Code
    );
    assert_eq!(
        Capability::Testing.category(),
        CapabilityCategory::Testing
    );
    assert_eq!(
        Capability::SystemDesign.category(),
        CapabilityCategory::Design
    );
}

// ============================================================================
// Capability Matcher Tests
// ============================================================================

#[test]
fn test_capability_matcher_registration() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("test-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);
    caps.insert(Capability::Testing);

    matcher.register_agent(agent_id.clone(), caps);

    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);

    let capable = matcher.find_capable_agents(&required);
    assert_eq!(capable.len(), 1);
    assert_eq!(capable[0], agent_id);
}

#[test]
fn test_capability_matcher_subset_matching() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("multi-cap-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);
    caps.insert(Capability::Testing);
    caps.insert(Capability::CodeReview);

    matcher.register_agent(agent_id.clone(), caps);

    // Agent should match if it has a superset of required capabilities
    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);
    required.insert(Capability::Testing);

    let capable = matcher.find_capable_agents(&required);
    assert_eq!(capable.len(), 1);
}

#[test]
fn test_capability_matcher_no_match() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("dev-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);

    matcher.register_agent(agent_id, caps);

    // Should not match if agent lacks required capability
    let mut required = HashSet::new();
    required.insert(Capability::Testing);

    let capable = matcher.find_capable_agents(&required);
    assert_eq!(capable.len(), 0);
}

#[test]
fn test_capability_matcher_best_agent() {
    let mut matcher = CapabilityMatcher::new();

    // Agent with exact match
    let exact_id = AgentId::from_string("exact-agent");
    let mut exact_caps = HashSet::new();
    exact_caps.insert(Capability::CodeGeneration);
    matcher.register_agent(exact_id.clone(), exact_caps);

    // Agent with extra capabilities
    let extra_id = AgentId::from_string("extra-agent");
    let mut extra_caps = HashSet::new();
    extra_caps.insert(Capability::CodeGeneration);
    extra_caps.insert(Capability::Testing);
    extra_caps.insert(Capability::CodeReview);
    matcher.register_agent(extra_id, extra_caps);

    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);

    // Should prefer agent with fewer extra capabilities
    let best = matcher.find_best_agent(&required);
    assert_eq!(best, Some(exact_id));
}

#[test]
fn test_capability_matcher_scoring() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("partial-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);
    caps.insert(Capability::Testing);

    matcher.register_agent(agent_id.clone(), caps);

    // Perfect match
    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);
    required.insert(Capability::Testing);
    assert_eq!(matcher.score_match(&agent_id, &required), 1.0);

    // Partial match
    let mut partial = HashSet::new();
    partial.insert(Capability::CodeGeneration);
    assert_eq!(matcher.score_match(&agent_id, &partial), 1.0);

    // No match
    let mut no_match = HashSet::new();
    no_match.insert(Capability::CodeReview);
    assert_eq!(matcher.score_match(&agent_id, &no_match), 0.0);
}

#[test]
fn test_capability_matcher_unregister() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("temp-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);

    matcher.register_agent(agent_id.clone(), caps.clone());

    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);

    // Should find agent before unregister
    let before = matcher.find_capable_agents(&required);
    assert_eq!(before.len(), 1);

    // Unregister agent
    matcher.unregister_agent(&agent_id);

    // Should not find agent after unregister
    let after = matcher.find_capable_agents(&required);
    assert_eq!(after.len(), 0);
}

#[test]
fn test_capability_matcher_by_category() {
    let mut matcher = CapabilityMatcher::new();

    let dev_id = AgentId::from_string("dev-agent");
    let mut dev_caps = HashSet::new();
    dev_caps.insert(Capability::CodeGeneration);
    matcher.register_agent(dev_id, dev_caps);

    let test_id = AgentId::from_string("test-agent");
    let mut test_caps = HashSet::new();
    test_caps.insert(Capability::Testing);
    matcher.register_agent(test_id, test_caps);

    // Find code agents
    let code_agents = matcher.agents_by_category(CapabilityCategory::Code);
    assert_eq!(code_agents.len(), 1);

    // Find testing agents
    let test_agents = matcher.agents_by_category(CapabilityCategory::Testing);
    assert_eq!(test_agents.len(), 1);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_agent_with_empty_name() {
    let agent = DeveloperAgent::new("".to_string());
    assert_eq!(agent.name(), "");
    assert_eq!(agent.agent_type(), AgentType::Developer);
}

#[test]
fn test_agent_with_special_characters_in_name() {
    let agent = DeveloperAgent::new("dev-agent-#1@test".to_string());
    assert_eq!(agent.name(), "dev-agent-#1@test");
}

#[test]
fn test_metrics_zero_division() {
    let metrics = AgentMetrics::new();

    let snapshot = metrics.snapshot();
    // Should not panic on zero tasks
    assert_eq!(snapshot.success_rate, 100);
}

#[test]
fn test_capability_matcher_empty_requirements() {
    let mut matcher = CapabilityMatcher::new();

    let agent_id = AgentId::from_string("any-agent");
    let mut caps = HashSet::new();
    caps.insert(Capability::CodeGeneration);
    matcher.register_agent(agent_id.clone(), caps);

    // Empty requirements should match all agents
    let required = HashSet::new();
    let capable = matcher.find_capable_agents(&required);
    assert_eq!(capable.len(), 1);
}

#[test]
fn test_capability_matcher_no_agents() {
    let matcher = CapabilityMatcher::new();

    let mut required = HashSet::new();
    required.insert(Capability::CodeGeneration);

    let capable = matcher.find_capable_agents(&required);
    assert_eq!(capable.len(), 0);
}
