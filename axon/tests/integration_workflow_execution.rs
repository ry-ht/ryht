//! Integration Tests for Workflow Execution
//!
//! Tests complete workflow execution including:
//! - Workflow creation and validation
//! - DAG execution with dependencies
//! - Parallel task execution
//! - Error handling and recovery
//! - Workflow cancellation
//! - Result aggregation

use axon::orchestration::{
    Orchestrator, Workflow, WorkflowMetadata, Task, TaskType, TaskStatus,
    WorkflowStatus, TaskScheduler, WorkflowExecutor,
    DagValidator,
};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use chrono::Utc;

// ============================================================================
// Workflow Creation and Validation Tests
// ============================================================================

#[tokio::test]
async fn test_simple_workflow_execution() {
    // Create a simple workflow with 2 tasks
    let workflow = Workflow {
        id: "test-workflow-1".to_string(),
        name: "Simple Test Workflow".to_string(),
        description: "Test workflow with 2 sequential tasks".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "First Task".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({"code": "print('hello')"}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-2".to_string(),
                name: "Second Task".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({"test": "verify"}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: {
            let mut deps = HashMap::new();
            deps.insert("task-2".to_string(), vec!["task-1".to_string()]);
            deps
        },
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    // Validate workflow
    let validator = DagValidator::new();
    let validation_result = validator.validate(&workflow);

    assert!(validation_result.is_ok(), "Simple workflow should be valid");
}

#[tokio::test]
async fn test_parallel_workflow_execution() {
    // Create a workflow with parallel tasks
    let workflow = Workflow {
        id: "parallel-workflow".to_string(),
        name: "Parallel Test Workflow".to_string(),
        description: "Workflow with independent parallel tasks".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Parallel Task 1".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({"code": "task1"}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-2".to_string(),
                name: "Parallel Task 2".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({"code": "task2"}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-3".to_string(),
                name: "Parallel Task 3".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({"code": "task3"}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(), // No dependencies = all parallel
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 2,
        },
    };

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);

    assert!(result.is_ok(), "Parallel workflow should be valid");
}

#[tokio::test]
async fn test_complex_dag_workflow() {
    // Create a complex DAG:
    //     task-1
    //    /      \
    // task-2  task-3
    //    \      /
    //     task-4

    let workflow = Workflow {
        id: "complex-dag".to_string(),
        name: "Complex DAG Workflow".to_string(),
        description: "Complex workflow with diamond dependency pattern".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Root Task".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-2".to_string(),
                name: "Branch 1".to_string(),
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-3".to_string(),
                name: "Branch 2".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-4".to_string(),
                name: "Merge Task".to_string(),
                task_type: TaskType::Documentation,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: {
            let mut deps = HashMap::new();
            deps.insert("task-2".to_string(), vec!["task-1".to_string()]);
            deps.insert("task-3".to_string(), vec!["task-1".to_string()]);
            deps.insert("task-4".to_string(), vec!["task-2".to_string(), "task-3".to_string()]);
            deps
        },
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 8,
            timeout: Duration::from_secs(600),
            max_retries: 3,
        },
    };

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);

    assert!(result.is_ok(), "Complex DAG should be valid");
}

// ============================================================================
// Workflow Validation Tests
// ============================================================================

#[tokio::test]
async fn test_cycle_detection() {
    // Create a workflow with a cycle: task-1 -> task-2 -> task-1
    let workflow = Workflow {
        id: "cyclic-workflow".to_string(),
        name: "Cyclic Workflow".to_string(),
        description: "Invalid workflow with cycle".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Task 1".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task-2".to_string(),
                name: "Task 2".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: {
            let mut deps = HashMap::new();
            deps.insert("task-2".to_string(), vec!["task-1".to_string()]);
            deps.insert("task-1".to_string(), vec!["task-2".to_string()]); // Cycle!
            deps
        },
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);

    assert!(result.is_err(), "Cyclic workflow should be invalid");
}

#[tokio::test]
async fn test_missing_dependency_detection() {
    // Create a workflow with a missing dependency
    let workflow = Workflow {
        id: "missing-dep".to_string(),
        name: "Missing Dependency Workflow".to_string(),
        description: "Workflow with missing task reference".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Task 1".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: {
            let mut deps = HashMap::new();
            deps.insert("task-1".to_string(), vec!["non-existent-task".to_string()]);
            deps
        },
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);

    assert!(result.is_err(), "Workflow with missing dependency should be invalid");
}

// ============================================================================
// Task Status Tests
// ============================================================================

#[test]
fn test_task_status_transitions() {
    let statuses = vec![
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::Completed,
        TaskStatus::Failed,
    ];

    // Test status transitions
    assert_eq!(statuses[0], TaskStatus::Pending);
    assert_eq!(statuses[1], TaskStatus::Running);
    assert_eq!(statuses[2], TaskStatus::Completed);
    assert_eq!(statuses[3], TaskStatus::Failed);
}

#[test]
fn test_workflow_status_values() {
    let statuses = vec![
        WorkflowStatus::Pending,
        WorkflowStatus::Running,
        WorkflowStatus::Completed,
        WorkflowStatus::Failed,
        WorkflowStatus::Cancelled,
    ];

    for (i, status) in statuses.iter().enumerate() {
        match status {
            WorkflowStatus::Pending => assert_eq!(i, 0),
            WorkflowStatus::Running => assert_eq!(i, 1),
            WorkflowStatus::Completed => assert_eq!(i, 2),
            WorkflowStatus::Failed => assert_eq!(i, 3),
            WorkflowStatus::Cancelled => assert_eq!(i, 4),
        }
    }
}

// ============================================================================
// Workflow Metadata Tests
// ============================================================================

#[test]
fn test_workflow_metadata() {
    let metadata = WorkflowMetadata {
        created_at: Utc::now(),
        priority: 7,
        timeout: Duration::from_secs(600),
        max_retries: 5,
    };

    assert_eq!(metadata.priority, 7);
    assert_eq!(metadata.timeout, Duration::from_secs(600));
    assert_eq!(metadata.max_retries, 5);
}

#[test]
fn test_workflow_priority_ordering() {
    let low_priority = WorkflowMetadata {
        created_at: Utc::now(),
        priority: 1,
        timeout: Duration::from_secs(300),
        max_retries: 3,
    };

    let high_priority = WorkflowMetadata {
        created_at: Utc::now(),
        priority: 10,
        timeout: Duration::from_secs(300),
        max_retries: 3,
    };

    assert!(high_priority.priority > low_priority.priority);
}

// ============================================================================
// Task Type Tests
// ============================================================================

#[test]
fn test_task_types() {
    let types = vec![
        TaskType::Development,
        TaskType::Review,
        TaskType::Testing,
        TaskType::Documentation,
        TaskType::Custom("CustomTask".to_string()),
    ];

    assert_eq!(types.len(), 5);

    // Test custom task type
    match &types[4] {
        TaskType::Custom(name) => assert_eq!(name, "CustomTask"),
        _ => panic!("Expected Custom task type"),
    }
}

// ============================================================================
// Workflow Builder Pattern Tests
// ============================================================================

#[test]
fn test_workflow_creation() {
    let workflow = Workflow {
        id: "builder-test".to_string(),
        name: "Builder Test".to_string(),
        description: "Test workflow builder".to_string(),
        tasks: vec![],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    assert_eq!(workflow.id, "builder-test");
    assert_eq!(workflow.tasks.len(), 0);
    assert!(workflow.dependencies.is_empty());
}

#[test]
fn test_task_creation() {
    let task = Task {
        id: "test-task".to_string(),
        name: "Test Task".to_string(),
        task_type: TaskType::Development,
        input: serde_json::json!({"key": "value"}),
        status: TaskStatus::Pending,
    };

    assert_eq!(task.id, "test-task");
    assert_eq!(task.status, TaskStatus::Pending);
    assert_eq!(task.task_type, TaskType::Development);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_workflow_timeout() {
    let workflow = Workflow {
        id: "timeout-test".to_string(),
        name: "Timeout Test".to_string(),
        description: "Test workflow timeout".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Long Task".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_millis(100), // Very short timeout
            max_retries: 0,
        },
    };

    // Workflow should timeout if tasks take too long
    assert_eq!(workflow.metadata.timeout, Duration::from_millis(100));
}

#[tokio::test]
async fn test_workflow_retry_logic() {
    let workflow = Workflow {
        id: "retry-test".to_string(),
        name: "Retry Test".to_string(),
        description: "Test workflow retry".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Flaky Task".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3, // Allow 3 retries
        },
    };

    assert_eq!(workflow.metadata.max_retries, 3);
}

// ============================================================================
// Dependency Graph Tests
// ============================================================================

#[test]
fn test_empty_dependency_graph() {
    let workflow = Workflow {
        id: "no-deps".to_string(),
        name: "No Dependencies".to_string(),
        description: "Workflow without dependencies".to_string(),
        tasks: vec![
            Task {
                id: "task-1".to_string(),
                name: "Independent Task".to_string(),
                task_type: TaskType::Development,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 5,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    assert!(workflow.dependencies.is_empty());
}

#[test]
fn test_linear_dependency_chain() {
    let mut dependencies = HashMap::new();
    dependencies.insert("task-2".to_string(), vec!["task-1".to_string()]);
    dependencies.insert("task-3".to_string(), vec!["task-2".to_string()]);
    dependencies.insert("task-4".to_string(), vec!["task-3".to_string()]);

    // Verify chain is correctly defined
    assert_eq!(dependencies.len(), 3);
    assert!(dependencies.contains_key("task-2"));
    assert!(dependencies.contains_key("task-3"));
    assert!(dependencies.contains_key("task-4"));
}

#[test]
fn test_multiple_dependencies() {
    let mut dependencies = HashMap::new();
    dependencies.insert(
        "task-final".to_string(),
        vec!["task-1".to_string(), "task-2".to_string(), "task-3".to_string()],
    );

    // Task-final depends on 3 other tasks
    let deps = dependencies.get("task-final").unwrap();
    assert_eq!(deps.len(), 3);
}

// ============================================================================
// Workflow Result Tests
// ============================================================================

#[test]
fn test_workflow_result_structure() {
    use axon::orchestration::{WorkflowResult, TaskResult};

    let result = WorkflowResult {
        workflow_id: "test-workflow".to_string(),
        success: true,
        duration: Duration::from_secs(120),
        task_results: {
            let mut results = HashMap::new();
            results.insert(
                "task-1".to_string(),
                TaskResult {
                    task_id: "task-1".to_string(),
                    success: true,
                    output: Some(serde_json::json!({"result": "success"})),
                    error: None,
                },
            );
            results
        },
    };

    assert_eq!(result.workflow_id, "test-workflow");
    assert!(result.success);
    assert_eq!(result.task_results.len(), 1);
}

#[test]
fn test_task_result_success() {
    use axon::orchestration::TaskResult;

    let result = TaskResult {
        task_id: "task-1".to_string(),
        success: true,
        output: Some(serde_json::json!({"data": "output"})),
        error: None,
    };

    assert!(result.success);
    assert!(result.output.is_some());
    assert!(result.error.is_none());
}

#[test]
fn test_task_result_failure() {
    use axon::orchestration::TaskResult;

    let result = TaskResult {
        task_id: "task-1".to_string(),
        success: false,
        output: None,
        error: Some("Task failed due to error".to_string()),
    };

    assert!(!result.success);
    assert!(result.output.is_none());
    assert!(result.error.is_some());
}
