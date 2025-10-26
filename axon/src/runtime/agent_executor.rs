//! Agent Executor - Task Delegation Execution
//!
//! This module handles the execution of task delegations on actual agent processes,
//! coordinating between the orchestration layer and the runtime processes.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::agents::AgentId;
use crate::orchestration::task_delegation::TaskDelegation;
use crate::orchestration::lead_agent::WorkerResult;
use super::mcp_integration::{McpServerPool, ToolCall, McpError};
use super::agent_process::ProcessManager;
use super::runtime_config::RuntimeConfig;

/// Result type for executor operations
pub type Result<T> = std::result::Result<T, ExecutorError>;

/// Executor errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Task timeout: {0}")]
    Timeout(String),

    #[error("Agent not available: {0}")]
    AgentNotAvailable(String),

    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("Invalid delegation: {0}")]
    InvalidDelegation(String),

    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

/// Agent executor for running task delegations
pub struct AgentExecutor {
    /// Process manager
    process_manager: Arc<ProcessManager>,

    /// MCP server pool
    mcp_pool: Arc<McpServerPool>,

    /// Runtime configuration
    config: RuntimeConfig,

    /// Execution statistics
    stats: Arc<RwLock<ExecutorStatistics>>,
}

/// Executor statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutorStatistics {
    /// Total tasks executed
    pub total_tasks: u64,

    /// Successful tasks
    pub successful_tasks: u64,

    /// Failed tasks
    pub failed_tasks: u64,

    /// Total tool calls
    pub total_tool_calls: u64,

    /// Average execution time (ms)
    pub avg_execution_time_ms: u64,

    /// Total execution time (ms)
    pub total_execution_time_ms: u64,
}

/// Task execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Task delegation
    pub delegation: TaskDelegation,

    /// Agent ID executing the task
    pub agent_id: AgentId,

    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Tool calls made
    pub tool_calls_made: usize,

    /// Intermediate results
    pub intermediate_results: Vec<JsonValue>,

    /// Current status
    pub status: ExecutionStatus,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Task is queued
    Queued,

    /// Task is being executed
    Executing,

    /// Task completed successfully
    Completed,

    /// Task failed
    Failed,

    /// Task timed out
    TimedOut,

    /// Task was cancelled
    Cancelled,
}

impl AgentExecutor {
    /// Create a new agent executor
    pub fn new(
        process_manager: Arc<ProcessManager>,
        mcp_pool: Arc<McpServerPool>,
        config: RuntimeConfig,
    ) -> Self {
        info!("Initializing Agent Executor");

        Self {
            process_manager,
            mcp_pool,
            config,
            stats: Arc::new(RwLock::new(ExecutorStatistics::default())),
        }
    }

    /// Execute a task delegation on an agent
    pub async fn execute_task(
        &self,
        agent_id: &AgentId,
        delegation: TaskDelegation,
    ) -> Result<WorkerResult> {
        let start_time = Instant::now();

        info!(
            "Executing task {} on agent {} (objective: {})",
            delegation.task_id, agent_id, delegation.objective
        );

        // Validate delegation
        delegation.validate()
            .map_err(ExecutorError::InvalidDelegation)?;

        // Check if agent process is alive
        if !self.process_manager.is_alive(agent_id).await {
            return Err(ExecutorError::AgentNotAvailable(agent_id.to_string()));
        }

        // Ensure MCP server is running
        self.mcp_pool.get_or_create(agent_id).await?;

        // Create execution context
        let mut context = ExecutionContext {
            delegation: delegation.clone(),
            agent_id: agent_id.clone(),
            started_at: chrono::Utc::now(),
            tool_calls_made: 0,
            intermediate_results: Vec::new(),
            status: ExecutionStatus::Executing,
        };

        // Execute with timeout
        let timeout_duration = delegation.boundaries.timeout;

        let result = tokio::time::timeout(
            timeout_duration,
            self.execute_with_context(&mut context)
        ).await;

        let execution_result = match result {
            Ok(Ok(result)) => {
                context.status = ExecutionStatus::Completed;
                Ok(result)
            }
            Ok(Err(e)) => {
                context.status = ExecutionStatus::Failed;
                error!("Task execution failed: {}", e);
                Err(e)
            }
            Err(_) => {
                context.status = ExecutionStatus::TimedOut;
                Err(ExecutorError::Timeout(format!(
                    "Task {} timed out after {:?}",
                    delegation.task_id, timeout_duration
                )))
            }
        };

        let duration = start_time.elapsed();

        // Update statistics
        self.update_statistics(duration, context.tool_calls_made, execution_result.is_ok()).await;

        // Build worker result
        match execution_result {
            Ok(result_data) => {
                Ok(WorkerResult {
                    worker_id: agent_id.clone(),
                    task: delegation,
                    result: result_data,
                    success: true,
                    duration,
                    tokens_used: self.estimate_tokens(&context),
                    cost_cents: self.estimate_cost(&context),
                    completed_at: chrono::Utc::now(),
                })
            }
            Err(e) => {
                Ok(WorkerResult {
                    worker_id: agent_id.clone(),
                    task: delegation,
                    result: serde_json::json!({
                        "error": e.to_string(),
                        "status": "failed"
                    }),
                    success: false,
                    duration,
                    tokens_used: self.estimate_tokens(&context),
                    cost_cents: self.estimate_cost(&context),
                    completed_at: chrono::Utc::now(),
                })
            }
        }
    }

    /// Execute task with context
    async fn execute_with_context(&self, context: &mut ExecutionContext) -> Result<JsonValue> {
        debug!("Executing task with context: {}", context.delegation.task_id);

        // Strategy: Execute the task by making tool calls through MCP
        // This is a simplified implementation - in production, this would involve
        // more sophisticated orchestration of tool calls based on the task objective

        let mut results = Vec::new();

        // Execute task-specific logic based on required capabilities
        let required_capabilities = context.delegation.required_capabilities.clone();
        let agent_id = context.agent_id.clone();
        let delegation = context.delegation.clone();

        for capability in &required_capabilities {
            let tool_result = self.execute_capability_task(
                &agent_id,
                capability,
                &delegation,
                context
            ).await?;

            results.push(tool_result);
        }

        // Aggregate results
        let final_result = serde_json::json!({
            "task_id": context.delegation.task_id,
            "objective": context.delegation.objective,
            "status": "completed",
            "results": results,
            "tool_calls_made": context.tool_calls_made,
            "execution_time_ms": (chrono::Utc::now() - context.started_at).num_milliseconds(),
        });

        Ok(final_result)
    }

    /// Execute a capability-specific task
    async fn execute_capability_task(
        &self,
        agent_id: &AgentId,
        capability: &str,
        delegation: &TaskDelegation,
        context: &mut ExecutionContext,
    ) -> Result<JsonValue> {
        debug!("Executing capability task: {}", capability);

        // Check tool call limit
        if context.tool_calls_made >= delegation.boundaries.max_tool_calls {
            return Err(ExecutorError::ResourceLimitExceeded(
                format!("Tool call limit reached: {}", delegation.boundaries.max_tool_calls)
            ));
        }

        // Map capability to tool calls
        let tool_calls = self.map_capability_to_tools(capability, delegation);

        let mut capability_results = Vec::new();

        for tool_call in tool_calls {
            // Check if tool is allowed
            if !delegation.is_tool_allowed(&tool_call.name) {
                warn!("Tool {} not allowed for this task", tool_call.name);
                continue;
            }

            // Execute tool call
            let tool_result = self.mcp_pool.call_tool(agent_id, tool_call.clone()).await?;

            context.tool_calls_made += 1;

            // Extract result content
            let result_text = tool_result.content.iter()
                .filter_map(|item| {
                    if let super::mcp_integration::ContentItem::Text { text } = item {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            capability_results.push(serde_json::json!({
                "tool": tool_call.name,
                "success": tool_result.success,
                "result": result_text,
            }));

            // Store intermediate result
            context.intermediate_results.push(serde_json::json!({
                "tool": tool_call.name,
                "result": result_text,
            }));
        }

        Ok(serde_json::json!({
            "capability": capability,
            "results": capability_results,
        }))
    }

    /// Map capability to tool calls
    fn map_capability_to_tools(&self, capability: &str, delegation: &TaskDelegation) -> Vec<ToolCall> {
        let mut tools = Vec::new();

        match capability {
            "CodeGeneration" => {
                // For code generation, we might use search and write tools
                tools.push(ToolCall {
                    name: "cortex_search".to_string(),
                    arguments: serde_json::json!({
                        "query": delegation.objective,
                        "limit": 5,
                    }),
                });
            }
            "CodeReview" => {
                // For code review, search relevant code
                tools.push(ToolCall {
                    name: "cortex_search".to_string(),
                    arguments: serde_json::json!({
                        "query": delegation.objective,
                        "limit": 10,
                    }),
                });
            }
            "Testing" => {
                // For testing, search test patterns
                tools.push(ToolCall {
                    name: "cortex_search".to_string(),
                    arguments: serde_json::json!({
                        "query": format!("test {}", delegation.objective),
                        "limit": 5,
                    }),
                });
            }
            "InformationRetrieval" => {
                // For research, use search extensively
                tools.push(ToolCall {
                    name: "cortex_search".to_string(),
                    arguments: serde_json::json!({
                        "query": delegation.objective,
                        "limit": 20,
                    }),
                });
            }
            _ => {
                // Default: generic search
                tools.push(ToolCall {
                    name: "cortex_search".to_string(),
                    arguments: serde_json::json!({
                        "query": delegation.objective,
                        "limit": 10,
                    }),
                });
            }
        }

        tools
    }

    /// Estimate tokens used
    fn estimate_tokens(&self, context: &ExecutionContext) -> u64 {
        // Rough estimation based on tool calls and results
        let base_tokens = 100u64;
        let per_tool_call = 500u64;
        let result_tokens = context.intermediate_results.len() as u64 * 200;

        base_tokens + (context.tool_calls_made as u64 * per_tool_call) + result_tokens
    }

    /// Estimate cost
    fn estimate_cost(&self, context: &ExecutionContext) -> u64 {
        // Rough cost estimation in cents
        let tokens = self.estimate_tokens(context);
        // Assume $0.01 per 1000 tokens
        (tokens / 1000).max(1)
    }

    /// Update statistics
    async fn update_statistics(&self, duration: Duration, tool_calls: usize, success: bool) {
        let mut stats = self.stats.write().await;

        stats.total_tasks += 1;
        if success {
            stats.successful_tasks += 1;
        } else {
            stats.failed_tasks += 1;
        }

        stats.total_tool_calls += tool_calls as u64;

        let duration_ms = duration.as_millis() as u64;
        stats.total_execution_time_ms += duration_ms;

        // Update average
        if stats.total_tasks > 0 {
            stats.avg_execution_time_ms = stats.total_execution_time_ms / stats.total_tasks;
        }
    }

    /// Get executor statistics
    pub async fn get_statistics(&self) -> ExecutorStatistics {
        self.stats.read().await.clone()
    }

    /// Execute multiple tasks in parallel
    pub async fn execute_tasks_parallel(
        &self,
        tasks: Vec<(AgentId, TaskDelegation)>,
        max_parallel: usize,
    ) -> Vec<Result<WorkerResult>> {
        info!("Executing {} tasks in parallel (max: {})", tasks.len(), max_parallel);

        let mut results = Vec::new();

        // Execute in batches
        for chunk in tasks.chunks(max_parallel) {
            let batch_futures: Vec<_> = chunk
                .iter()
                .map(|(agent_id, delegation)| {
                    self.execute_task(agent_id, delegation.clone())
                })
                .collect();

            let batch_results = futures::future::join_all(batch_futures).await;
            results.extend(batch_results);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status() {
        let status = ExecutionStatus::Executing;
        assert_eq!(status, ExecutionStatus::Executing);
    }

    #[test]
    fn test_executor_statistics() {
        let stats = ExecutorStatistics::default();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.successful_tasks, 0);
    }
}
