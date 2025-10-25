//! Orchestration Engine
//!
//! DAG-based workflow orchestration with parallel execution, dependency management,
//! and integration with Cortex for state management.
//!
//! # Features
//!
//! - DAG validation and cycle detection
//! - Topological sorting for execution order
//! - Parallel task execution within levels
//! - Critical path analysis
//! - Resource allocation
//! - Error handling and retry logic
//! - Cortex integration for task tracking

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod workflow;
pub mod scheduler;
pub mod executor;
pub mod dag;

pub use workflow::*;
pub use scheduler::*;
pub use executor::*;
pub use dag::*;

/// Main orchestrator for coordinating agent workflows
pub struct Orchestrator {
    /// Workflow scheduler
    scheduler: Arc<TaskScheduler>,

    /// Workflow executor
    executor: Arc<WorkflowExecutor>,

    /// DAG validator
    validator: DagValidator,

    /// Active workflows
    active_workflows: Arc<RwLock<HashMap<String, WorkflowStatus>>>,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(scheduler: Arc<TaskScheduler>, executor: Arc<WorkflowExecutor>) -> Self {
        Self {
            scheduler,
            executor,
            validator: DagValidator::new(),
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a workflow
    pub async fn execute_workflow(&self, workflow: Workflow) -> Result<WorkflowResult> {
        // Validate workflow DAG
        self.validator.validate(&workflow)?;

        // Create execution schedule
        let schedule = self.scheduler.create_schedule(&workflow).await?;

        // Track workflow
        self.active_workflows
            .write()
            .await
            .insert(workflow.id.clone(), WorkflowStatus::Running);

        // Execute workflow
        let result = self.executor.execute(workflow, schedule).await?;

        // Update status
        let status = if result.success {
            WorkflowStatus::Completed
        } else {
            WorkflowStatus::Failed
        };

        self.active_workflows
            .write()
            .await
            .insert(result.workflow_id.clone(), status);

        Ok(result)
    }

    /// Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &str) -> Option<WorkflowStatus> {
        self.active_workflows.read().await.get(workflow_id).copied()
    }
}

/// Workflow execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Result type for orchestration operations
pub type Result<T> = std::result::Result<T, OrchestrationError>;

/// Orchestration errors
#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("Cycle detected in workflow DAG: {task_id}")]
    CycleDetected { task_id: String },

    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: String },

    #[error("Dependency not found: task {task_id}, dependency {dependency_id}")]
    DependencyNotFound {
        task_id: String,
        dependency_id: String,
    },

    #[error("Invalid DAG: {reason}")]
    InvalidDag { reason: String },

    #[error("No suitable agent for task {task_id}")]
    NoSuitableAgent { task_id: String },

    #[error("Timeout executing workflow")]
    Timeout,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_status() {
        let status = WorkflowStatus::Running;
        assert_eq!(status, WorkflowStatus::Running);
        assert_ne!(status, WorkflowStatus::Completed);
    }
}
