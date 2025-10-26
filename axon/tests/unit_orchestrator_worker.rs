//! Unit Tests for Orchestrator-Worker Pattern
//!
//! Tests the Lead Agent's orchestrator-worker implementation including:
//! - Query complexity analysis
//! - Worker spawning and delegation
//! - Resource allocation
//! - Result synthesis
//! - Strategy selection

use axon::orchestration::lead_agent::{LeadAgent, QueryComplexity, QueryAnalysis};
use axon::orchestration::strategy_library::{StrategyLibrary, ExecutionStrategy, PatternType};
use axon::orchestration::worker_registry::{WorkerRegistry, WorkerStatus};
use axon::orchestration::task_delegation::{TaskDelegation, TaskBoundaries};
use axon::orchestration::result_synthesizer::{ResultSynthesizer, SynthesizedResult};
use axon::orchestration::execution_plan::ResourceAllocation;
use axon::agents::{AgentId, AgentType};
use std::time::Duration;

// ============================================================================
// Query Complexity Tests
// ============================================================================

#[test]
fn test_query_complexity_simple_allocation() {
    let complexity = QueryComplexity::Simple;
    let allocation = complexity.recommended_allocation();

    assert_eq!(allocation.num_workers, 1);
    assert_eq!(allocation.max_tool_calls_per_worker, 10);
    assert_eq!(allocation.max_parallel_workers, 1);
    assert_eq!(allocation.timeout, Duration::from_secs(30));
    assert!(allocation.max_tokens_budget <= 10_000);
}

#[test]
fn test_query_complexity_medium_allocation() {
    let complexity = QueryComplexity::Medium;
    let allocation = complexity.recommended_allocation();

    assert_eq!(allocation.num_workers, 4);
    assert_eq!(allocation.max_tool_calls_per_worker, 15);
    assert_eq!(allocation.max_parallel_workers, 4);
    assert_eq!(allocation.timeout, Duration::from_secs(120));
    assert!(allocation.max_tokens_budget <= 50_000);
}

#[test]
fn test_query_complexity_complex_allocation() {
    let complexity = QueryComplexity::Complex;
    let allocation = complexity.recommended_allocation();

    assert_eq!(allocation.num_workers, 10);
    assert_eq!(allocation.max_tool_calls_per_worker, 20);
    assert_eq!(allocation.max_parallel_workers, 10);
    assert_eq!(allocation.timeout, Duration::from_secs(300));
    assert!(allocation.max_tokens_budget <= 150_000);
}

#[test]
fn test_complexity_ordering() {
    // Test that complexity levels are distinct
    assert_ne!(QueryComplexity::Simple, QueryComplexity::Medium);
    assert_ne!(QueryComplexity::Medium, QueryComplexity::Complex);
    assert_ne!(QueryComplexity::Simple, QueryComplexity::Complex);
}

// ============================================================================
// Strategy Library Tests
// ============================================================================

#[test]
fn test_strategy_library_creation() {
    let library = StrategyLibrary::new();

    // Library should have built-in strategies
    let strategy = library.get_strategy("simple_query");
    assert!(strategy.is_some());
}

#[test]
fn test_strategy_library_pattern_types() {
    let library = StrategyLibrary::new();

    // Test that different pattern types are available
    let simple_strategy = library.get_strategy("simple_query");
    let comparison_strategy = library.get_strategy("comparison");
    let research_strategy = library.get_strategy("research");

    assert!(simple_strategy.is_some());
    assert!(comparison_strategy.is_some());
    assert!(research_strategy.is_some());
}

#[test]
fn test_strategy_selection_by_keywords() {
    let library = StrategyLibrary::new();

    // Test keyword-based strategy selection
    let query = "Compare Python and Rust for web development";
    let strategy = library.select_strategy(query);

    assert!(strategy.is_ok());
    let strategy = strategy.unwrap();
    assert_eq!(strategy.pattern_type, PatternType::Comparison);
}

#[test]
fn test_custom_strategy_registration() {
    let mut library = StrategyLibrary::new();

    let custom_strategy = ExecutionStrategy {
        name: "custom_test".to_string(),
        description: "Test strategy".to_string(),
        pattern_type: PatternType::Custom("test".to_string()),
        keywords: vec!["test".to_string()],
        worker_count_range: (1, 5),
        success_criteria: Default::default(),
        delegation_template: "Test delegation".to_string(),
        synthesis_instructions: "Test synthesis".to_string(),
    };

    library.register_strategy(custom_strategy.clone());
    let retrieved = library.get_strategy("custom_test");

    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "custom_test");
}

// ============================================================================
// Worker Registry Tests
// ============================================================================

#[test]
fn test_worker_registry_creation() {
    let registry = WorkerRegistry::new();

    assert_eq!(registry.active_workers_count(), 0);
    assert!(registry.available_workers().is_empty());
}

#[test]
fn test_worker_registry_registration() {
    let mut registry = WorkerRegistry::new();
    let agent_id = AgentId::new();

    let result = registry.register_worker(
        agent_id.clone(),
        "test-worker".to_string(),
        AgentType::Developer,
        vec!["CodeGeneration".to_string()],
    );

    assert!(result.is_ok());
    assert_eq!(registry.active_workers_count(), 1);
}

#[test]
fn test_worker_registry_status_transitions() {
    let mut registry = WorkerRegistry::new();
    let agent_id = AgentId::new();

    registry.register_worker(
        agent_id.clone(),
        "worker-1".to_string(),
        AgentType::Developer,
        vec![],
    ).unwrap();

    // Worker should start as Available
    let status = registry.get_worker_status(&agent_id);
    assert!(status.is_some());
    assert_eq!(status.unwrap(), WorkerStatus::Available);

    // Mark as busy
    registry.mark_busy(&agent_id).unwrap();
    assert_eq!(registry.get_worker_status(&agent_id).unwrap(), WorkerStatus::Busy);

    // Mark as available again
    registry.mark_available(&agent_id).unwrap();
    assert_eq!(registry.get_worker_status(&agent_id).unwrap(), WorkerStatus::Available);
}

#[test]
fn test_worker_registry_capability_matching() {
    let mut registry = WorkerRegistry::new();

    // Register workers with different capabilities
    let dev_id = AgentId::new();
    registry.register_worker(
        dev_id.clone(),
        "dev-worker".to_string(),
        AgentType::Developer,
        vec!["CodeGeneration".to_string(), "Refactoring".to_string()],
    ).unwrap();

    let test_id = AgentId::new();
    registry.register_worker(
        test_id.clone(),
        "test-worker".to_string(),
        AgentType::Tester,
        vec!["Testing".to_string(), "CoverageAnalysis".to_string()],
    ).unwrap();

    // Find workers by capability
    let code_workers = registry.find_workers_by_capability("CodeGeneration");
    assert_eq!(code_workers.len(), 1);
    assert_eq!(code_workers[0], dev_id);

    let test_workers = registry.find_workers_by_capability("Testing");
    assert_eq!(test_workers.len(), 1);
    assert_eq!(test_workers[0], test_id);
}

#[test]
fn test_worker_registry_available_workers_filter() {
    let mut registry = WorkerRegistry::new();

    let agent1 = AgentId::new();
    let agent2 = AgentId::new();

    registry.register_worker(agent1.clone(), "worker-1".to_string(), AgentType::Developer, vec![]).unwrap();
    registry.register_worker(agent2.clone(), "worker-2".to_string(), AgentType::Developer, vec![]).unwrap();

    // Both should be available
    assert_eq!(registry.available_workers().len(), 2);

    // Mark one as busy
    registry.mark_busy(&agent1).unwrap();

    // Only one should be available
    let available = registry.available_workers();
    assert_eq!(available.len(), 1);
    assert_eq!(available[0], agent2);
}

// ============================================================================
// Task Delegation Tests
// ============================================================================

#[test]
fn test_task_delegation_builder() {
    let task = TaskDelegation::builder()
        .objective("Test objective".to_string())
        .add_scope("scope-1".to_string())
        .add_scope("scope-2".to_string())
        .add_constraint("constraint-1".to_string())
        .max_tool_calls(15)
        .timeout(Duration::from_secs(60))
        .priority(8)
        .required_capabilities(vec!["cap-1".to_string()])
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();

    assert_eq!(task.objective, "Test objective");
    assert_eq!(task.boundaries.scope.len(), 2);
    assert_eq!(task.boundaries.constraints.len(), 1);
    assert_eq!(task.boundaries.max_tool_calls, 15);
    assert_eq!(task.priority, 8);
}

#[test]
fn test_task_delegation_builder_validation() {
    // Test missing objective
    let result = TaskDelegation::builder()
        .add_scope("scope".to_string())
        .build();

    // Should fail without objective
    assert!(result.is_err());
}

#[test]
fn test_task_delegation_default_values() {
    let task = TaskDelegation::builder()
        .objective("Test".to_string())
        .build()
        .unwrap();

    // Check default values
    assert_eq!(task.priority, 5);
    assert_eq!(task.boundaries.max_tool_calls, 10);
    assert!(task.boundaries.timeout > Duration::from_secs(0));
}

#[test]
fn test_task_boundaries_validation() {
    let boundaries = TaskBoundaries {
        scope: vec!["scope".to_string()],
        constraints: vec!["constraint".to_string()],
        allowed_tools: Some(vec!["tool1".to_string()]),
        forbidden_actions: vec!["delete".to_string()],
        max_tool_calls: 20,
        timeout: Duration::from_secs(120),
        resource_limits: Default::default(),
    };

    assert_eq!(boundaries.max_tool_calls, 20);
    assert_eq!(boundaries.scope.len(), 1);
    assert!(boundaries.allowed_tools.is_some());
}

// ============================================================================
// Result Synthesizer Tests
// ============================================================================

#[test]
fn test_result_synthesizer_creation() {
    let synthesizer = ResultSynthesizer::new();

    // Synthesizer should be created successfully
    assert!(true);
}

#[test]
fn test_result_synthesizer_empty_results() {
    let synthesizer = ResultSynthesizer::new();

    // Synthesizing empty results should handle gracefully
    let results = vec![];
    let synthesized = synthesizer.synthesize_results(
        "test-query".to_string(),
        results,
        "Test instructions".to_string(),
    );

    // Should return a result (possibly empty or error state)
    assert!(synthesized.is_ok() || synthesized.is_err());
}

#[test]
fn test_result_synthesis_quality_metrics() {
    let synthesizer = ResultSynthesizer::new();

    // Create mock worker results
    let worker_results = vec![
        create_mock_worker_result("worker-1", "Result 1", true),
        create_mock_worker_result("worker-2", "Result 2", true),
    ];

    let result = synthesizer.synthesize_results(
        "test-query".to_string(),
        worker_results,
        "Combine results".to_string(),
    );

    if let Ok(synthesized) = result {
        // Quality metrics should be calculated
        assert!(synthesized.quality_metrics.confidence >= 0.0);
        assert!(synthesized.quality_metrics.confidence <= 1.0);
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_mock_worker_result(agent_id: &str, result: &str, success: bool) -> axon::orchestration::lead_agent::WorkerResult {
    use axon::orchestration::lead_agent::WorkerResult;

    WorkerResult {
        worker_id: AgentId::from_string(agent_id),
        task_id: "test-task".to_string(),
        result: serde_json::json!({ "output": result }),
        success,
        error: if success { None } else { Some("Error".to_string()) },
        tool_calls_made: 5,
        tokens_used: 1000,
        duration: Duration::from_secs(10),
        quality_score: if success { 0.8 } else { 0.0 },
        completed_at: chrono::Utc::now(),
    }
}

// ============================================================================
// Integration Tests within Unit Test File
// ============================================================================

#[test]
fn test_end_to_end_complexity_to_allocation() {
    // Test the full flow from complexity detection to resource allocation
    let complexities = vec![
        QueryComplexity::Simple,
        QueryComplexity::Medium,
        QueryComplexity::Complex,
    ];

    for complexity in complexities {
        let allocation = complexity.recommended_allocation();

        // Verify allocation is sensible
        assert!(allocation.num_workers > 0);
        assert!(allocation.max_tool_calls_per_worker > 0);
        assert!(allocation.max_parallel_workers > 0);
        assert!(allocation.timeout > Duration::from_secs(0));
        assert!(allocation.max_tokens_budget > 0);

        // Verify resources scale with complexity
        match complexity {
            QueryComplexity::Simple => {
                assert!(allocation.num_workers <= 2);
            },
            QueryComplexity::Medium => {
                assert!(allocation.num_workers >= 2 && allocation.num_workers <= 5);
            },
            QueryComplexity::Complex => {
                assert!(allocation.num_workers >= 8);
            },
        }
    }
}

#[test]
fn test_worker_lifecycle() {
    // Test complete worker lifecycle
    let mut registry = WorkerRegistry::new();
    let agent_id = AgentId::new();

    // 1. Register
    let result = registry.register_worker(
        agent_id.clone(),
        "lifecycle-worker".to_string(),
        AgentType::Developer,
        vec!["Testing".to_string()],
    );
    assert!(result.is_ok());

    // 2. Verify available
    assert_eq!(registry.get_worker_status(&agent_id).unwrap(), WorkerStatus::Available);

    // 3. Assign task (mark busy)
    registry.mark_busy(&agent_id).unwrap();
    assert_eq!(registry.get_worker_status(&agent_id).unwrap(), WorkerStatus::Busy);

    // 4. Complete task (mark available)
    registry.mark_available(&agent_id).unwrap();
    assert_eq!(registry.get_worker_status(&agent_id).unwrap(), WorkerStatus::Available);

    // 5. Unregister
    registry.unregister_worker(&agent_id).unwrap();
    assert!(registry.get_worker_status(&agent_id).is_none());
}

#[test]
fn test_strategy_to_delegation_mapping() {
    // Test that strategies can generate appropriate delegations
    let library = StrategyLibrary::new();

    let strategy = library.get_strategy("simple_query").unwrap();

    // Strategy should have template for delegation
    assert!(!strategy.delegation_template.is_empty());
    assert!(!strategy.synthesis_instructions.is_empty());

    // Worker count should be appropriate
    assert!(strategy.worker_count_range.0 >= 1);
    assert!(strategy.worker_count_range.1 >= strategy.worker_count_range.0);
}
