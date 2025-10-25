//! Unit tests for orchestration engine
//!
//! Tests cover:
//! - Workflow creation and validation
//! - DAG validation and cycle detection
//! - Task dependency management
//! - Workflow execution status
//! - Error handling

mod common;

use axon::orchestration::*;
use std::collections::HashMap;
use std::time::Duration;
use chrono::Utc;

// ============================================================================
// Workflow Tests
// ============================================================================

#[test]
fn test_workflow_creation() {
    let workflow = create_simple_workflow();
    assert_eq!(workflow.name, "test-workflow");
    assert_eq!(workflow.tasks.len(), 2);
}

#[test]
fn test_workflow_with_dependencies() {
    let mut workflow = create_simple_workflow();

    // Add dependency: task2 depends on task1
    workflow.dependencies.insert("task2".to_string(), vec!["task1".to_string()]);

    assert!(workflow.dependencies.contains_key("task2"));
    assert_eq!(workflow.dependencies.get("task2").unwrap().len(), 1);
}

#[test]
fn test_workflow_metadata() {
    let workflow = create_simple_workflow();

    assert_eq!(workflow.metadata.priority, 1);
    assert_eq!(workflow.metadata.max_retries, 3);
    assert!(workflow.metadata.timeout > Duration::from_secs(0));
}

// ============================================================================
// Task Tests
// ============================================================================

#[test]
fn test_task_creation() {
    let task = Task {
        id: "task-1".to_string(),
        name: "Test Task".to_string(),
        task_type: TaskType::Development,
        input: serde_json::json!({"key": "value"}),
        status: TaskStatus::Pending,
    };

    assert_eq!(task.id, "task-1");
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(matches!(task.task_type, TaskType::Development));
}

#[test]
fn test_task_status_transitions() {
    let statuses = vec![
        TaskStatus::Pending,
        TaskStatus::Running,
        TaskStatus::Completed,
    ];

    for status in statuses {
        let task = Task {
            id: "task-1".to_string(),
            name: "Test".to_string(),
            task_type: TaskType::Development,
            input: serde_json::json!({}),
            status,
        };

        assert_eq!(task.status, status);
    }
}

#[test]
fn test_task_types() {
    let types = vec![
        TaskType::Development,
        TaskType::Review,
        TaskType::Testing,
        TaskType::Documentation,
        TaskType::Custom("custom".to_string()),
    ];

    for task_type in types {
        let task = Task {
            id: "task-1".to_string(),
            name: "Test".to_string(),
            task_type: task_type.clone(),
            input: serde_json::json!({}),
            status: TaskStatus::Pending,
        };

        // Verify task type is set correctly
        match task.task_type {
            TaskType::Development => assert!(matches!(task_type, TaskType::Development)),
            TaskType::Review => assert!(matches!(task_type, TaskType::Review)),
            TaskType::Testing => assert!(matches!(task_type, TaskType::Testing)),
            TaskType::Documentation => assert!(matches!(task_type, TaskType::Documentation)),
            TaskType::Custom(_) => assert!(matches!(task_type, TaskType::Custom(_))),
        }
    }
}

// ============================================================================
// DAG Validator Tests
// ============================================================================

#[test]
fn test_dag_validator_valid_workflow() {
    let validator = DagValidator::new();
    let workflow = create_simple_workflow();

    // Should validate successfully
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[test]
fn test_dag_validator_detects_simple_cycle() {
    let validator = DagValidator::new();
    let workflow = create_workflow_with_cycle();

    // Should detect cycle
    let result = validator.validate(&workflow);
    assert!(result.is_err());

    if let Err(OrchestrationError::CycleDetected { task_id }) = result {
        assert!(!task_id.is_empty());
    } else {
        panic!("Expected CycleDetected error");
    }
}

#[test]
fn test_dag_validator_detects_missing_dependency() {
    let validator = DagValidator::new();
    let mut workflow = create_simple_workflow();

    // Add dependency to non-existent task
    workflow.dependencies.insert(
        "task1".to_string(),
        vec!["non-existent".to_string()],
    );

    let result = validator.validate(&workflow);
    assert!(result.is_err());

    if let Err(OrchestrationError::DependencyNotFound { task_id, dependency_id }) = result {
        assert_eq!(task_id, "task1");
        assert_eq!(dependency_id, "non-existent");
    } else {
        panic!("Expected DependencyNotFound error");
    }
}

#[test]
fn test_dag_validator_detects_missing_task() {
    let validator = DagValidator::new();
    let mut workflow = create_simple_workflow();

    // Add dependency for non-existent task
    workflow.dependencies.insert(
        "non-existent".to_string(),
        vec!["task1".to_string()],
    );

    let result = validator.validate(&workflow);
    assert!(result.is_err());

    if let Err(OrchestrationError::TaskNotFound { task_id }) = result {
        assert_eq!(task_id, "non-existent");
    } else {
        panic!("Expected TaskNotFound error");
    }
}

#[test]
fn test_dag_validator_complex_valid_dag() {
    let validator = DagValidator::new();
    let workflow = create_complex_workflow();

    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[test]
fn test_dag_validator_self_dependency() {
    let validator = DagValidator::new();
    let mut workflow = create_simple_workflow();

    // Task depends on itself
    workflow.dependencies.insert(
        "task1".to_string(),
        vec!["task1".to_string()],
    );

    let result = validator.validate(&workflow);
    assert!(result.is_err());
}

// ============================================================================
// Workflow Result Tests
// ============================================================================

#[test]
fn test_workflow_result_success() {
    let result = WorkflowResult {
        workflow_id: "workflow-1".to_string(),
        success: true,
        duration: Duration::from_secs(10),
        task_results: HashMap::new(),
    };

    assert!(result.success);
    assert_eq!(result.workflow_id, "workflow-1");
}

#[test]
fn test_workflow_result_with_task_results() {
    let mut task_results = HashMap::new();
    task_results.insert(
        "task1".to_string(),
        TaskResult {
            task_id: "task1".to_string(),
            success: true,
            output: Some(serde_json::json!({"result": "ok"})),
            error: None,
        },
    );

    let result = WorkflowResult {
        workflow_id: "workflow-1".to_string(),
        success: true,
        duration: Duration::from_secs(10),
        task_results,
    };

    assert_eq!(result.task_results.len(), 1);
    assert!(result.task_results.contains_key("task1"));
}

#[test]
fn test_task_result_success() {
    let result = TaskResult {
        task_id: "task1".to_string(),
        success: true,
        output: Some(serde_json::json!({"data": "value"})),
        error: None,
    };

    assert!(result.success);
    assert!(result.output.is_some());
    assert!(result.error.is_none());
}

#[test]
fn test_task_result_failure() {
    let result = TaskResult {
        task_id: "task1".to_string(),
        success: false,
        output: None,
        error: Some("Task execution failed".to_string()),
    };

    assert!(!result.success);
    assert!(result.output.is_none());
    assert!(result.error.is_some());
}

// ============================================================================
// Workflow Status Tests
// ============================================================================

#[test]
fn test_workflow_status_transitions() {
    let statuses = vec![
        WorkflowStatus::Pending,
        WorkflowStatus::Running,
        WorkflowStatus::Completed,
        WorkflowStatus::Failed,
        WorkflowStatus::Cancelled,
    ];

    for status in statuses {
        match status {
            WorkflowStatus::Pending => assert_eq!(status, WorkflowStatus::Pending),
            WorkflowStatus::Running => assert_eq!(status, WorkflowStatus::Running),
            WorkflowStatus::Completed => assert_eq!(status, WorkflowStatus::Completed),
            WorkflowStatus::Failed => assert_eq!(status, WorkflowStatus::Failed),
            WorkflowStatus::Cancelled => assert_eq!(status, WorkflowStatus::Cancelled),
        }
    }
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_orchestration_error_cycle_detected() {
    let error = OrchestrationError::CycleDetected {
        task_id: "task1".to_string(),
    };

    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Cycle detected"));
    assert!(error_msg.contains("task1"));
}

#[test]
fn test_orchestration_error_task_not_found() {
    let error = OrchestrationError::TaskNotFound {
        task_id: "missing-task".to_string(),
    };

    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Task not found"));
    assert!(error_msg.contains("missing-task"));
}

#[test]
fn test_orchestration_error_dependency_not_found() {
    let error = OrchestrationError::DependencyNotFound {
        task_id: "task1".to_string(),
        dependency_id: "missing-dep".to_string(),
    };

    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Dependency not found"));
    assert!(error_msg.contains("task1"));
    assert!(error_msg.contains("missing-dep"));
}

#[test]
fn test_orchestration_error_no_suitable_agent() {
    let error = OrchestrationError::NoSuitableAgent {
        task_id: "task1".to_string(),
    };

    let error_msg = format!("{}", error);
    assert!(error_msg.contains("No suitable agent"));
    assert!(error_msg.contains("task1"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_workflow() {
    let workflow = Workflow {
        id: "empty".to_string(),
        name: "Empty Workflow".to_string(),
        description: "Workflow with no tasks".to_string(),
        tasks: vec![],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 1,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    };

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[test]
fn test_workflow_with_no_dependencies() {
    let mut workflow = create_simple_workflow();
    workflow.dependencies.clear();

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[test]
fn test_workflow_with_many_tasks() {
    let mut workflow = create_simple_workflow();

    // Add many tasks
    for i in 3..100 {
        workflow.tasks.push(Task {
            id: format!("task{}", i),
            name: format!("Task {}", i),
            task_type: TaskType::Development,
            input: serde_json::json!({}),
            status: TaskStatus::Pending,
        });
    }

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

#[test]
fn test_linear_dependency_chain() {
    let mut workflow = create_simple_workflow();

    // Add more tasks to create a chain
    for i in 3..10 {
        workflow.tasks.push(Task {
            id: format!("task{}", i),
            name: format!("Task {}", i),
            task_type: TaskType::Development,
            input: serde_json::json!({}),
            status: TaskStatus::Pending,
        });

        // Each task depends on the previous one
        workflow.dependencies.insert(
            format!("task{}", i),
            vec![format!("task{}", i - 1)],
        );
    }

    let validator = DagValidator::new();
    let result = validator.validate(&workflow);
    assert!(result.is_ok());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_simple_workflow() -> Workflow {
    Workflow {
        id: "workflow-1".to_string(),
        name: "test-workflow".to_string(),
        description: "A simple test workflow".to_string(),
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
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 1,
            timeout: Duration::from_secs(300),
            max_retries: 3,
        },
    }
}

fn create_workflow_with_cycle() -> Workflow {
    let mut workflow = create_simple_workflow();

    // Create cycle: task1 -> task2 -> task1
    workflow.dependencies.insert("task1".to_string(), vec!["task2".to_string()]);
    workflow.dependencies.insert("task2".to_string(), vec!["task1".to_string()]);

    workflow
}

fn create_complex_workflow() -> Workflow {
    let mut workflow = Workflow {
        id: "complex-workflow".to_string(),
        name: "Complex Workflow".to_string(),
        description: "Workflow with multiple dependencies".to_string(),
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
                task_type: TaskType::Review,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
            Task {
                id: "task4".to_string(),
                name: "Task 4".to_string(),
                task_type: TaskType::Testing,
                input: serde_json::json!({}),
                status: TaskStatus::Pending,
            },
        ],
        dependencies: HashMap::new(),
        metadata: WorkflowMetadata {
            created_at: Utc::now(),
            priority: 2,
            timeout: Duration::from_secs(600),
            max_retries: 5,
        },
    };

    // task3 depends on task1 and task2
    workflow.dependencies.insert("task3".to_string(), vec!["task1".to_string(), "task2".to_string()]);
    // task4 depends on task3
    workflow.dependencies.insert("task4".to_string(), vec!["task3".to_string()]);

    workflow
}
