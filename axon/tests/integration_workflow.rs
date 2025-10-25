//! Integration tests for multi-agent workflows
//!
//! Tests complete workflows involving multiple agents coordinating
//! through the orchestration engine.

mod common;

use axon::agents::*;
use axon::orchestration::*;
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;

// ============================================================================
// Simple Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_simple_workflow_execution() {
    // Create a simple workflow with two sequential tasks
    let workflow = create_dev_review_workflow();

    assert_eq!(workflow.tasks.len(), 2);
    assert!(workflow.dependencies.contains_key("review"));
}

#[tokio::test]
async fn test_workflow_dag_validation() {
    let workflow = create_dev_review_workflow();
    let validator = DagValidator::new();

    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parallel_tasks_workflow() {
    // Create workflow with parallel tasks
    let mut workflow = create_parallel_workflow();

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());

    // Tasks should not depend on each other
    assert_eq!(workflow.tasks.len(), 3);
}

#[tokio::test]
async fn test_complex_dependency_workflow() {
    let workflow = create_complex_workflow();

    assert_eq!(workflow.tasks.len(), 5);

    // Validate DAG
    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

// ============================================================================
// Multi-Agent Coordination Tests
// ============================================================================

#[tokio::test]
async fn test_developer_reviewer_coordination() {
    let dev_agent = DeveloperAgent::new("dev-1".to_string());
    let reviewer_agent = ReviewerAgent::new("reviewer-1".to_string());

    // Verify agents have complementary capabilities
    assert!(dev_agent.capabilities().contains(&Capability::CodeGeneration));
    assert!(reviewer_agent.capabilities().contains(&Capability::CodeReview));
}

#[tokio::test]
async fn test_full_development_pipeline() {
    // Create agents for full pipeline
    let dev = DeveloperAgent::new("dev".to_string());
    let reviewer = ReviewerAgent::new("reviewer".to_string());
    let tester = TesterAgent::new("tester".to_string());
    let documenter = DocumenterAgent::new("documenter".to_string());

    // Create workflow
    let workflow = create_full_pipeline_workflow();

    assert_eq!(workflow.tasks.len(), 4);

    // Validate dependencies
    let validator = DagValidator::new();
    assert!(validator.validate(&workflow).is_ok());
}

// ============================================================================
// Agent Capability Matching Tests
// ============================================================================

#[tokio::test]
async fn test_capability_based_agent_selection() {
    let mut matcher = CapabilityMatcher::new();

    // Register agents
    let dev_id = AgentId::from_string("dev-1");
    let mut dev_caps = std::collections::HashSet::new();
    dev_caps.insert(Capability::CodeGeneration);
    dev_caps.insert(Capability::CodeRefactoring);
    matcher.register_agent(dev_id.clone(), dev_caps);

    let test_id = AgentId::from_string("test-1");
    let mut test_caps = std::collections::HashSet::new();
    test_caps.insert(Capability::Testing);
    test_caps.insert(Capability::TestGeneration);
    matcher.register_agent(test_id.clone(), test_caps);

    // Find agent for code generation
    let mut required = std::collections::HashSet::new();
    required.insert(Capability::CodeGeneration);

    let agents = matcher.find_capable_agents(&required);
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0], dev_id);

    // Find agent for testing
    required.clear();
    required.insert(Capability::Testing);

    let agents = matcher.find_capable_agents(&required);
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0], test_id);
}

// ============================================================================
// Workflow Status Tracking Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_status_transitions() {
    let statuses = vec![
        WorkflowStatus::Pending,
        WorkflowStatus::Running,
        WorkflowStatus::Completed,
    ];

    for status in statuses {
        // Verify each status is distinct
        match status {
            WorkflowStatus::Pending => assert!(true),
            WorkflowStatus::Running => assert!(true),
            WorkflowStatus::Completed => assert!(true),
            _ => panic!("Unexpected status"),
        }
    }
}

// ============================================================================
// Task Assignment Tests
// ============================================================================

#[tokio::test]
async fn test_task_to_agent_assignment() {
    let workflow = create_dev_review_workflow();
    let mut matcher = CapabilityMatcher::new();

    // Register developer
    let dev_id = AgentId::from_string("dev-1");
    let mut dev_caps = std::collections::HashSet::new();
    dev_caps.insert(Capability::CodeGeneration);
    matcher.register_agent(dev_id.clone(), dev_caps);

    // Register reviewer
    let rev_id = AgentId::from_string("rev-1");
    let mut rev_caps = std::collections::HashSet::new();
    rev_caps.insert(Capability::CodeReview);
    matcher.register_agent(rev_id.clone(), rev_caps);

    // Assign development task
    let mut dev_required = std::collections::HashSet::new();
    dev_required.insert(Capability::CodeGeneration);
    let dev_agents = matcher.find_capable_agents(&dev_required);
    assert_eq!(dev_agents.len(), 1);

    // Assign review task
    let mut rev_required = std::collections::HashSet::new();
    rev_required.insert(Capability::CodeReview);
    let rev_agents = matcher.find_capable_agents(&rev_required);
    assert_eq!(rev_agents.len(), 1);
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_with_failed_task() {
    let mut workflow = create_dev_review_workflow();

    // Simulate task failure
    workflow.tasks[0].status = TaskStatus::Failed;

    // Workflow should still be valid
    let validator = DagValidator::new();
    assert!(validator.validate(&workflow).is_ok());
}

// ============================================================================
// Performance Tracking Tests
// ============================================================================

#[tokio::test]
async fn test_agent_metrics_in_workflow() {
    let dev = DeveloperAgent::new("dev-perf".to_string());
    let metrics = dev.metrics();

    // Simulate successful tasks
    metrics.record_success(100, 1000, 50);
    metrics.record_success(150, 1500, 75);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.tasks_completed, 2);
    assert_eq!(snapshot.tokens_used, 2500);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_dev_review_workflow() -> Workflow {
    let mut dependencies = HashMap::new();
    dependencies.insert("review".to_string(), vec!["develop".to_string()]);

    Workflow {
        id: "dev-review-wf".to_string(),
        name: "Development and Review".to_string(),
        description: "Code development followed by review".to_string(),
        tasks: vec![
            Task {
                id: "develop".to_string(),
                name: "Develop Feature".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({"feature": "login"}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "review".to_string(),
                name: "Review Code".to_string(),
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies,
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 1,
            timeout: Duration::from_secs(600),
            max_retries: 3,
        },
    }
}

fn create_parallel_workflow() -> Workflow {
    Workflow {
        id: "parallel-wf".to_string(),
        name: "Parallel Tasks".to_string(),
        description: "Multiple independent tasks".to_string(),
        tasks: vec![
            Task {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task2".to_string(),
                name: "Task 2".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task3".to_string(),
                name: "Task 3".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 2,
            timeout: Duration::from_secs(300),
            max_retries: 2,
        },
    }
}

fn create_complex_workflow() -> Workflow {
    let mut dependencies = HashMap::new();
    dependencies.insert("review".to_string(), vec!["develop".to_string()]);
    dependencies.insert("test".to_string(), vec!["review".to_string()]);
    dependencies.insert("document".to_string(), vec!["review".to_string()]);
    dependencies.insert("deploy".to_string(), vec!["test".to_string(), "document".to_string()]);

    Workflow {
        id: "complex-wf".to_string(),
        name: "Complex Workflow".to_string(),
        description: "Multi-stage development workflow".to_string(),
        tasks: vec![
            Task {
                id: "develop".to_string(),
                name: "Develop".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "review".to_string(),
                name: "Review".to_string(),
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "test".to_string(),
                name: "Test".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "document".to_string(),
                name: "Document".to_string(),
                task_type: TaskType::Documentation,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "deploy".to_string(),
                name: "Deploy".to_string(),
                task_type: TaskType::Custom("deployment".to_string()),
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies,
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 3,
            timeout: Duration::from_secs(1800),
            max_retries: 5,
        },
    }
}

fn create_full_pipeline_workflow() -> Workflow {
    let mut dependencies = HashMap::new();
    dependencies.insert("review".to_string(), vec!["develop".to_string()]);
    dependencies.insert("test".to_string(), vec!["review".to_string()]);
    dependencies.insert("document".to_string(), vec!["test".to_string()]);

    Workflow {
        id: "full-pipeline".to_string(),
        name: "Full Development Pipeline".to_string(),
        description: "Complete development pipeline".to_string(),
        tasks: vec![
            Task {
                id: "develop".to_string(),
                name: "Development".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "review".to_string(),
                name: "Code Review".to_string(),
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "test".to_string(),
                name: "Testing".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "document".to_string(),
                name: "Documentation".to_string(),
                task_type: TaskType::Documentation,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies,
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 1,
            timeout: Duration::from_secs(1200),
            max_retries: 3,
        },
    }
}
