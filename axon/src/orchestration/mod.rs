//! Orchestration Engine
//!
//! Multi-agent orchestration with two complementary patterns:
//!
//! 1. **DAG-based Workflow Orchestration**: For predefined workflows with dependencies
//! 2. **Orchestrator-Worker Pattern**: For dynamic query-driven multi-agent coordination
//!
//! # DAG-based Features
//!
//! - DAG validation and cycle detection
//! - Topological sorting for execution order
//! - Parallel task execution within levels
//! - Critical path analysis
//! - Resource allocation
//! - Error handling and retry logic
//! - Cortex integration for task tracking
//!
//! # Orchestrator-Worker Pattern Features (Anthropic's Best Practices)
//!
//! - Query complexity analysis (Simple/Medium/Complex)
//! - Dynamic worker spawning based on complexity
//! - Parallel execution of independent workers
//! - Result synthesis from multiple workers
//! - Resource allocation rules (1 worker for simple, 4 for medium, 10+ for complex)
//! - Task delegation with explicit objectives and boundaries
//! - Strategy library for query pattern matching
//!
//! ## Performance Goals
//!
//! - 90% time reduction for complex queries through parallelization
//! - Intelligent resource allocation based on query complexity
//! - Worker pool management with capability matching
//!
//! ## Usage Example
//!
//! ```no_run
//! use axon::orchestration::{LeadAgent, StrategyLibrary, WorkerRegistry};
//!
//! // For dynamic query handling (Orchestrator-Worker Pattern)
//! let lead_agent = LeadAgent::new(...);
//! let result = lead_agent.handle_query(query, workspace_id, session_id).await?;
//!
//! // For predefined workflows (DAG-based)
//! let orchestrator = Orchestrator::new(...);
//! let result = orchestrator.execute_workflow(workflow).await?;
//! ```

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// DAG-based workflow modules
pub mod workflow;
pub mod scheduler;
pub mod executor;
pub mod dag;

// Orchestrator-Worker Pattern modules (Anthropic's pattern)
pub mod lead_agent;
pub mod strategy_library;
pub mod worker_registry;
pub mod task_delegation;
pub mod result_synthesizer;
pub mod execution_plan;
pub mod runtime_integration;
pub mod parallel_tool_executor;

// Re-export DAG-based types
pub use workflow::*;
pub use scheduler::*;
pub use executor::*;
pub use dag::*;

// Re-export Orchestrator-Worker types
pub use lead_agent::{LeadAgent, LeadAgentConfig, QueryComplexity, QueryAnalysis, ExecutionState, WorkerResult};
pub use strategy_library::{StrategyLibrary, ExecutionStrategy, PatternType, OutputFormat, SuccessCriteria};
pub use worker_registry::{WorkerRegistry, WorkerHandle, WorkerInfo, WorkerStatus};
pub use task_delegation::{TaskDelegation, TaskBoundaries, TaskTemplates};
pub use result_synthesizer::{ResultSynthesizer, SynthesizedResult, Finding, Recommendation, QualityMetrics};
pub use execution_plan::{ExecutionPlan, ResourceAllocation, ExecutionProgress};
pub use runtime_integration::{RuntimeIntegration, LeadAgentWithRuntime};
pub use parallel_tool_executor::{ParallelToolExecutor, ToolCall, ToolResult, ExecutionStats};

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

    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },

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
