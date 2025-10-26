//! Execution Plan - Resource Allocation and Task Coordination
//!
//! This module defines execution plans that coordinate multiple workers with
//! appropriate resource allocation based on query complexity.
//!
//! # Resource Allocation Rules (from Anthropic's Research)
//!
//! - Simple queries: 1 agent, 3-10 tool calls, 30s timeout
//! - Medium queries: 2-4 subagents, 10-15 calls each, 2min timeout
//! - Complex queries: 10+ subagents, 20+ calls each, 5min timeout
//!
//! This prevents over-investment in simple problems while ensuring adequate
//! resources for complex ones.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use chrono::{DateTime, Utc};

use super::{
    strategy_library::ExecutionStrategy,
    task_delegation::TaskDelegation,
};

// ============================================================================
// Resource Allocation
// ============================================================================

/// Resource allocation specification
///
/// Defines computational and financial resources allocated to an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Number of workers to spawn
    pub num_workers: usize,

    /// Maximum tool calls per worker
    ///
    /// Prevents individual workers from running indefinitely
    pub max_tool_calls_per_worker: usize,

    /// Maximum workers to run in parallel
    ///
    /// Limits concurrent execution for resource management
    pub max_parallel_workers: usize,

    /// Total execution timeout
    ///
    /// Maximum duration before entire execution is terminated
    pub timeout: Duration,

    /// Token budget for all workers combined
    ///
    /// Cost control mechanism
    pub max_tokens_budget: usize,

    /// Maximum cost in cents
    ///
    /// Financial constraint
    pub max_cost_cents: u64,
}

impl ResourceAllocation {
    /// Create allocation for simple query
    pub fn simple() -> Self {
        Self {
            num_workers: 1,
            max_tool_calls_per_worker: 10,
            max_parallel_workers: 1,
            timeout: Duration::from_secs(30),
            max_tokens_budget: 10_000,
            max_cost_cents: 10,
        }
    }

    /// Create allocation for medium complexity query
    pub fn medium() -> Self {
        Self {
            num_workers: 4,
            max_tool_calls_per_worker: 15,
            max_parallel_workers: 4,
            timeout: Duration::from_secs(120),
            max_tokens_budget: 50_000,
            max_cost_cents: 50,
        }
    }

    /// Create allocation for complex query
    pub fn complex() -> Self {
        Self {
            num_workers: 10,
            max_tool_calls_per_worker: 20,
            max_parallel_workers: 10,
            timeout: Duration::from_secs(300),
            max_tokens_budget: 150_000,
            max_cost_cents: 200,
        }
    }

    /// Validate allocation parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.num_workers == 0 {
            return Err("Number of workers must be > 0".to_string());
        }

        if self.max_tool_calls_per_worker == 0 {
            return Err("Max tool calls must be > 0".to_string());
        }

        if self.max_parallel_workers == 0 {
            return Err("Max parallel workers must be > 0".to_string());
        }

        if self.max_parallel_workers > self.num_workers {
            return Err("Max parallel workers cannot exceed total workers".to_string());
        }

        if self.timeout.as_secs() == 0 {
            return Err("Timeout must be > 0".to_string());
        }

        Ok(())
    }

    /// Calculate estimated total tokens
    pub fn estimated_total_tokens(&self) -> usize {
        // Rough estimate: each tool call uses ~500 tokens
        self.num_workers * self.max_tool_calls_per_worker * 500
    }

    /// Check if allocation is within budget
    pub fn is_within_budget(&self, tokens_used: usize, cost_cents: u64) -> bool {
        tokens_used <= self.max_tokens_budget && cost_cents <= self.max_cost_cents
    }
}

// ============================================================================
// Execution Plan
// ============================================================================

/// Complete execution plan for orchestrated task
///
/// Combines strategy, resource allocation, and task delegations into a
/// coherent execution blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Unique plan identifier
    pub plan_id: String,

    /// Execution strategy being used
    pub strategy: ExecutionStrategy,

    /// Resource allocation
    pub resource_allocation: ResourceAllocation,

    /// Task delegations for workers
    pub task_delegations: Vec<TaskDelegation>,

    /// Whether tasks can be executed in parallel
    pub parallelizable: bool,

    /// Estimated execution duration
    pub estimated_duration: Duration,

    /// Plan creation timestamp
    pub created_at: DateTime<Utc>,
}

impl ExecutionPlan {
    /// Create a new execution plan
    pub fn new(
        strategy: ExecutionStrategy,
        resource_allocation: ResourceAllocation,
        task_delegations: Vec<TaskDelegation>,
    ) -> Result<Self, String> {
        // Validate allocation
        resource_allocation.validate()?;

        // Validate task count matches allocation
        if task_delegations.len() > resource_allocation.num_workers {
            return Err(format!(
                "Task count ({}) exceeds allocated workers ({})",
                task_delegations.len(),
                resource_allocation.num_workers
            ));
        }

        // Validate all task delegations
        for delegation in &task_delegations {
            delegation.validate()?;
        }

        // Determine if parallelizable based on task dependencies
        let parallelizable = Self::check_parallelizable(&task_delegations);

        Ok(Self {
            plan_id: uuid::Uuid::new_v4().to_string(),
            strategy,
            estimated_duration: resource_allocation.timeout,
            resource_allocation,
            task_delegations,
            parallelizable,
            created_at: Utc::now(),
        })
    }

    /// Check if tasks can be executed in parallel
    fn check_parallelizable(tasks: &[TaskDelegation]) -> bool {
        // Simple heuristic: if tasks have non-overlapping scopes, they're parallelizable
        for i in 0..tasks.len() {
            for j in (i + 1)..tasks.len() {
                let task_i = &tasks[i];
                let task_j = &tasks[j];

                // Check for scope overlap
                for scope_i in &task_i.boundaries.scope {
                    for scope_j in &task_j.boundaries.scope {
                        if scope_i == scope_j {
                            // Overlapping scope, might have dependencies
                            return false;
                        }
                    }
                }
            }
        }

        true
    }

    /// Get high-priority tasks
    pub fn high_priority_tasks(&self) -> Vec<&TaskDelegation> {
        self.task_delegations
            .iter()
            .filter(|t| t.priority >= 7)
            .collect()
    }

    /// Get tasks sorted by priority (highest first)
    pub fn tasks_by_priority(&self) -> Vec<&TaskDelegation> {
        let mut tasks: Vec<_> = self.task_delegations.iter().collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }

    /// Estimate total cost in cents
    pub fn estimated_total_cost(&self) -> u64 {
        // Rough estimate: $0.01 per 1000 tokens
        let tokens = self.resource_allocation.estimated_total_tokens();
        (tokens as f64 / 1000.0 * 1.0) as u64
    }

    /// Create execution batches for parallel execution
    pub fn create_execution_batches(&self) -> Vec<Vec<&TaskDelegation>> {
        let mut batches = Vec::new();
        let batch_size = self.resource_allocation.max_parallel_workers;

        let mut current_batch = Vec::new();
        for task in &self.task_delegations {
            current_batch.push(task);

            if current_batch.len() >= batch_size {
                batches.push(current_batch);
                current_batch = Vec::new();
            }
        }

        if !current_batch.is_empty() {
            batches.push(current_batch);
        }

        batches
    }

    /// Get execution summary
    pub fn summary(&self) -> ExecutionPlanSummary {
        ExecutionPlanSummary {
            plan_id: self.plan_id.clone(),
            strategy_name: self.strategy.name.clone(),
            total_workers: self.resource_allocation.num_workers,
            parallel_workers: self.resource_allocation.max_parallel_workers,
            total_tasks: self.task_delegations.len(),
            high_priority_tasks: self.high_priority_tasks().len(),
            estimated_duration_secs: self.estimated_duration.as_secs(),
            estimated_cost_cents: self.estimated_total_cost(),
            parallelizable: self.parallelizable,
        }
    }
}

/// Execution plan summary for logging/display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlanSummary {
    pub plan_id: String,
    pub strategy_name: String,
    pub total_workers: usize,
    pub parallel_workers: usize,
    pub total_tasks: usize,
    pub high_priority_tasks: usize,
    pub estimated_duration_secs: u64,
    pub estimated_cost_cents: u64,
    pub parallelizable: bool,
}

// ============================================================================
// Execution Progress Tracking
// ============================================================================

/// Tracks execution progress for a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    /// Plan ID
    pub plan_id: String,

    /// Tasks completed
    pub tasks_completed: usize,

    /// Tasks in progress
    pub tasks_in_progress: usize,

    /// Tasks pending
    pub tasks_pending: usize,

    /// Tasks failed
    pub tasks_failed: usize,

    /// Overall progress (0.0 - 1.0)
    pub overall_progress: f32,

    /// Tokens used so far
    pub tokens_used: usize,

    /// Cost incurred so far (cents)
    pub cost_cents: u64,

    /// Elapsed time
    pub elapsed: Duration,

    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl ExecutionProgress {
    /// Create new progress tracker for a plan
    pub fn new(plan: &ExecutionPlan) -> Self {
        Self {
            plan_id: plan.plan_id.clone(),
            tasks_completed: 0,
            tasks_in_progress: 0,
            tasks_pending: plan.task_delegations.len(),
            tasks_failed: 0,
            overall_progress: 0.0,
            tokens_used: 0,
            cost_cents: 0,
            elapsed: Duration::from_secs(0),
            updated_at: Utc::now(),
        }
    }

    /// Update progress with completed task
    pub fn complete_task(&mut self, tokens: usize, cost: u64) {
        self.tasks_completed += 1;
        if self.tasks_in_progress > 0 {
            self.tasks_in_progress -= 1;
        }
        self.tokens_used += tokens;
        self.cost_cents += cost;
        self.update_overall_progress();
    }

    /// Update progress with failed task
    pub fn fail_task(&mut self) {
        self.tasks_failed += 1;
        if self.tasks_in_progress > 0 {
            self.tasks_in_progress -= 1;
        }
        self.update_overall_progress();
    }

    /// Start a task
    pub fn start_task(&mut self) {
        self.tasks_in_progress += 1;
        if self.tasks_pending > 0 {
            self.tasks_pending -= 1;
        }
        self.update_overall_progress();
    }

    /// Calculate overall progress
    fn update_overall_progress(&mut self) {
        let total = self.tasks_completed + self.tasks_in_progress + self.tasks_pending + self.tasks_failed;
        if total > 0 {
            self.overall_progress = self.tasks_completed as f32 / total as f32;
        }
        self.updated_at = Utc::now();
    }

    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.tasks_pending == 0 && self.tasks_in_progress == 0
    }

    /// Check if execution is successful (all tasks completed)
    pub fn is_successful(&self) -> bool {
        self.is_complete() && self.tasks_failed == 0
    }

    /// Get estimated time remaining
    pub fn estimated_time_remaining(&self) -> Option<Duration> {
        if self.tasks_completed == 0 {
            return None;
        }

        let avg_time_per_task = self.elapsed.as_secs_f64() / self.tasks_completed as f64;
        let remaining_tasks = self.tasks_pending + self.tasks_in_progress;

        Some(Duration::from_secs_f64(avg_time_per_task * remaining_tasks as f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_allocation_simple() {
        let allocation = ResourceAllocation::simple();
        assert_eq!(allocation.num_workers, 1);
        assert!(allocation.validate().is_ok());
    }

    #[test]
    fn test_resource_allocation_validation() {
        let mut allocation = ResourceAllocation::simple();
        allocation.num_workers = 0;
        assert!(allocation.validate().is_err());
    }

    #[test]
    fn test_execution_progress() {
        let allocation = ResourceAllocation::simple();
        let plan = ExecutionPlan {
            plan_id: "test".to_string(),
            strategy: ExecutionStrategy {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test strategy".to_string(),
                patterns: vec![],
                recommended_workers: 1,
                max_parallel: 1,
                allowed_tools: vec![],
                output_format: super::super::strategy_library::OutputFormat::default(),
                success_criteria: super::super::strategy_library::SuccessCriteria::default(),
                times_applied: 0,
                success_rate: 0.0,
                avg_time_saved_percent: 0.0,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            resource_allocation: allocation,
            task_delegations: vec![],
            parallelizable: true,
            estimated_duration: Duration::from_secs(30),
            created_at: Utc::now(),
        };

        let mut progress = ExecutionProgress::new(&plan);
        assert_eq!(progress.overall_progress, 0.0);

        progress.start_task();
        progress.complete_task(1000, 5);
        assert!(progress.overall_progress > 0.0);
    }
}
