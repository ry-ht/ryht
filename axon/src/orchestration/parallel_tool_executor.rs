//! Parallel Tool Execution - Execute Independent Tools Concurrently
//!
//! This module implements parallel tool execution as described in Anthropic's research.
//! By executing 3+ tools concurrently, we can achieve up to 90% time reduction compared
//! to sequential execution.
//!
//! # Features
//!
//! - Dependency analysis to detect tool dependencies
//! - Topological sorting for execution order
//! - Concurrent execution of independent tools
//! - Semaphore-based concurrency control
//! - Error handling and partial failure recovery
//!
//! # Performance Goals
//!
//! - 70-90% time reduction for 3+ independent tools
//! - Automatic parallelization based on dependency analysis
//! - Respects max_concurrent_tools limit

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};
use serde::{Deserialize, Serialize};

use super::{OrchestrationError, Result};

// ============================================================================
// Tool Call Types
// ============================================================================

/// Tool call specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool identifier
    pub tool_id: String,

    /// Tool name
    pub tool_name: String,

    /// Tool parameters
    pub params: serde_json::Value,

    /// Expected output resources
    pub outputs: Vec<String>,

    /// Required input resources
    pub inputs: Vec<String>,

    /// Priority (1-10, higher is more important)
    pub priority: u8,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool ID
    pub tool_id: String,

    /// Tool name
    pub tool_name: String,

    /// Result data
    pub result: serde_json::Value,

    /// Success flag
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution duration
    pub duration: Duration,

    /// Timestamp
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Dependency graph for tools
struct DependencyGraph {
    /// Nodes: tool_id -> tool_call
    nodes: HashMap<String, ToolCall>,

    /// Edges: tool_id -> [dependent_tool_ids]
    edges: HashMap<String, Vec<String>>,

    /// In-degree: tool_id -> count of dependencies
    in_degree: HashMap<String, usize>,
}

impl DependencyGraph {
    /// Create a new dependency graph from tool calls
    fn new(tools: Vec<ToolCall>) -> Self {
        let mut graph = Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            in_degree: HashMap::new(),
        };

        // Add all nodes
        for tool in tools {
            graph.in_degree.insert(tool.tool_id.clone(), 0);
            graph.nodes.insert(tool.tool_id.clone(), tool);
        }

        // Build edges based on input/output dependencies
        let node_ids: Vec<_> = graph.nodes.keys().cloned().collect();

        for i in 0..node_ids.len() {
            for j in 0..node_ids.len() {
                if i == j {
                    continue;
                }

                let tool_i = &graph.nodes[&node_ids[i]];
                let tool_j = &graph.nodes[&node_ids[j]];

                // Check if tool_j depends on tool_i
                // (tool_j requires outputs from tool_i)
                for output in &tool_i.outputs {
                    if tool_j.inputs.contains(output) {
                        // tool_j depends on tool_i
                        graph.edges
                            .entry(node_ids[i].clone())
                            .or_default()
                            .push(node_ids[j].clone());

                        *graph.in_degree.get_mut(&node_ids[j]).unwrap() += 1;
                    }
                }
            }
        }

        graph
    }

    /// Perform topological sort to get execution stages
    /// Returns stages where tools in each stage can be executed in parallel
    fn topological_sort(&self) -> Vec<Vec<String>> {
        let mut stages = Vec::new();
        let mut in_degree = self.in_degree.clone();
        let mut processed = HashSet::new();

        while processed.len() < self.nodes.len() {
            // Find all nodes with in-degree 0 (no dependencies)
            let mut current_stage = Vec::new();

            for (node_id, &degree) in &in_degree {
                if degree == 0 && !processed.contains(node_id) {
                    current_stage.push(node_id.clone());
                }
            }

            if current_stage.is_empty() {
                // Cycle detected or graph issue
                warn!("Cycle detected in tool dependency graph or no more nodes available");
                break;
            }

            // Sort by priority (highest first)
            current_stage.sort_by(|a, b| {
                let pri_a = self.nodes[a].priority;
                let pri_b = self.nodes[b].priority;
                pri_b.cmp(&pri_a)
            });

            stages.push(current_stage.clone());

            // Mark as processed and update in-degrees
            for node_id in &current_stage {
                processed.insert(node_id.clone());

                // Reduce in-degree of dependent nodes
                if let Some(dependents) = self.edges.get(node_id) {
                    for dependent in dependents {
                        if let Some(degree) = in_degree.get_mut(dependent) {
                            *degree = degree.saturating_sub(1);
                        }
                    }
                }

                // Set in-degree to max to prevent reprocessing
                in_degree.insert(node_id.clone(), usize::MAX);
            }
        }

        stages
    }

    /// Get tool calls for a stage
    fn get_tools_for_stage(&self, stage: &[String]) -> Vec<ToolCall> {
        stage.iter()
            .filter_map(|id| self.nodes.get(id).cloned())
            .collect()
    }
}

// ============================================================================
// Parallel Tool Executor
// ============================================================================

/// Executor for running tools in parallel
pub struct ParallelToolExecutor {
    /// Maximum concurrent tools
    max_concurrent: usize,

    /// Timeout for individual tools
    tool_timeout: Duration,

    /// Enable partial failure recovery
    allow_partial_failure: bool,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total tools executed
    pub total_tools: usize,

    /// Successful executions
    pub successful: usize,

    /// Failed executions
    pub failed: usize,

    /// Total execution time (parallel)
    pub total_duration: Duration,

    /// Sequential time (sum of all tool durations)
    pub sequential_duration: Duration,

    /// Time saved percentage
    pub time_saved_percent: f32,

    /// Parallelization efficiency (0.0 - 1.0)
    pub parallel_efficiency: f32,
}

impl Default for ParallelToolExecutor {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            tool_timeout: Duration::from_secs(60),
            allow_partial_failure: true,
        }
    }
}

impl ParallelToolExecutor {
    /// Create a new parallel tool executor
    pub fn new(max_concurrent: usize, tool_timeout: Duration) -> Self {
        info!("Initializing Parallel Tool Executor with max_concurrent={}", max_concurrent);

        Self {
            max_concurrent,
            tool_timeout,
            allow_partial_failure: true,
        }
    }

    /// Execute tools in parallel with dependency management
    pub async fn execute_tools(
        &self,
        tools: Vec<ToolCall>,
    ) -> Result<(Vec<ToolResult>, ExecutionStats)> {
        let start_time = Instant::now();

        info!("Executing {} tools with dependency-aware parallelization", tools.len());

        if tools.is_empty() {
            return Ok((Vec::new(), ExecutionStats {
                total_tools: 0,
                successful: 0,
                failed: 0,
                total_duration: Duration::from_secs(0),
                sequential_duration: Duration::from_secs(0),
                time_saved_percent: 0.0,
                parallel_efficiency: 0.0,
            }));
        }

        // Step 1: Build dependency graph
        let graph = DependencyGraph::new(tools);

        // Step 2: Get execution stages via topological sort
        let stages = graph.topological_sort();

        info!("Tool execution planned in {} stages", stages.len());
        for (idx, stage) in stages.iter().enumerate() {
            debug!("Stage {}: {} tools", idx + 1, stage.len());
        }

        // Step 3: Execute each stage in parallel
        let mut all_results = Vec::new();
        let mut sequential_time = Duration::from_secs(0);

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        for (stage_idx, stage) in stages.iter().enumerate() {
            debug!("Executing stage {} with {} tools", stage_idx + 1, stage.len());

            let stage_tools = graph.get_tools_for_stage(stage);
            let stage_results = self.execute_stage(stage_tools, semaphore.clone()).await?;

            // Update sequential time
            for result in &stage_results {
                sequential_time += result.duration;
            }

            all_results.extend(stage_results);
        }

        let total_duration = start_time.elapsed();

        // Step 4: Calculate statistics
        let stats = self.calculate_stats(&all_results, total_duration, sequential_time);

        info!(
            "Parallel execution complete: {}/{} successful, {:.1}% time saved",
            stats.successful,
            stats.total_tools,
            stats.time_saved_percent
        );

        Ok((all_results, stats))
    }

    /// Execute a single stage of tools in parallel
    async fn execute_stage(
        &self,
        tools: Vec<ToolCall>,
        semaphore: Arc<Semaphore>,
    ) -> Result<Vec<ToolResult>> {
        let mut handles = Vec::new();

        for tool in tools {
            let semaphore = semaphore.clone();
            let timeout = self.tool_timeout;

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.unwrap();

                // Execute tool with timeout
                tokio::time::timeout(timeout, Self::execute_single_tool(tool))
                    .await
                    .unwrap_or_else(|_| {
                        // Timeout occurred
                        let tool_id = uuid::Uuid::new_v4().to_string();
                        ToolResult {
                            tool_id,
                            tool_name: "timeout".to_string(),
                            result: serde_json::json!({}),
                            success: false,
                            error: Some("Tool execution timed out".to_string()),
                            duration: timeout,
                            completed_at: chrono::Utc::now(),
                        }
                    })
            });

            handles.push(handle);
        }

        // Wait for all tools to complete
        let results = futures::future::join_all(handles).await;

        // Unwrap join results
        let tool_results: Vec<ToolResult> = results
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        // Check if any critical failures
        if !self.allow_partial_failure {
            let failed_count = tool_results.iter().filter(|r| !r.success).count();
            if failed_count > 0 {
                return Err(OrchestrationError::ExecutionFailed {
                    reason: format!("{} tool(s) failed in stage", failed_count),
                });
            }
        }

        Ok(tool_results)
    }

    /// Execute a single tool (stub - would integrate with actual tool execution)
    async fn execute_single_tool(tool: ToolCall) -> ToolResult {
        let start_time = Instant::now();

        debug!("Executing tool: {} ({})", tool.tool_name, tool.tool_id);

        // Simulate tool execution
        // In real implementation, this would call the actual tool via MCP or other interface
        tokio::time::sleep(Duration::from_millis(100)).await;

        let duration = start_time.elapsed();

        ToolResult {
            tool_id: tool.tool_id.clone(),
            tool_name: tool.tool_name.clone(),
            result: serde_json::json!({
                "status": "completed",
                "tool": tool.tool_name,
                "params": tool.params,
            }),
            success: true,
            error: None,
            duration,
            completed_at: chrono::Utc::now(),
        }
    }

    /// Calculate execution statistics
    fn calculate_stats(
        &self,
        results: &[ToolResult],
        total_duration: Duration,
        sequential_duration: Duration,
    ) -> ExecutionStats {
        let total_tools = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total_tools - successful;

        // Calculate time saved
        let time_saved = if sequential_duration > total_duration {
            sequential_duration - total_duration
        } else {
            Duration::from_secs(0)
        };

        let time_saved_percent = if sequential_duration.as_secs_f64() > 0.0 {
            (time_saved.as_secs_f64() / sequential_duration.as_secs_f64()) * 100.0
        } else {
            0.0
        };

        // Calculate parallel efficiency
        let parallel_efficiency = if total_duration.as_secs_f64() > 0.0 {
            sequential_duration.as_secs_f64() / (total_duration.as_secs_f64() * total_tools as f64)
        } else {
            0.0
        };

        ExecutionStats {
            total_tools,
            successful,
            failed,
            total_duration,
            sequential_duration,
            time_saved_percent: time_saved_percent as f32,
            parallel_efficiency: parallel_efficiency as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_no_deps() {
        let tools = vec![
            ToolCall {
                tool_id: "t1".to_string(),
                tool_name: "tool1".to_string(),
                params: serde_json::json!({}),
                outputs: vec!["out1".to_string()],
                inputs: vec![],
                priority: 5,
            },
            ToolCall {
                tool_id: "t2".to_string(),
                tool_name: "tool2".to_string(),
                params: serde_json::json!({}),
                outputs: vec!["out2".to_string()],
                inputs: vec![],
                priority: 5,
            },
        ];

        let graph = DependencyGraph::new(tools);
        let stages = graph.topological_sort();

        // Should have 1 stage with both tools (no dependencies)
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].len(), 2);
    }

    #[test]
    fn test_dependency_graph_with_deps() {
        let tools = vec![
            ToolCall {
                tool_id: "t1".to_string(),
                tool_name: "tool1".to_string(),
                params: serde_json::json!({}),
                outputs: vec!["out1".to_string()],
                inputs: vec![],
                priority: 5,
            },
            ToolCall {
                tool_id: "t2".to_string(),
                tool_name: "tool2".to_string(),
                params: serde_json::json!({}),
                outputs: vec!["out2".to_string()],
                inputs: vec!["out1".to_string()], // Depends on t1
                priority: 5,
            },
        ];

        let graph = DependencyGraph::new(tools);
        let stages = graph.topological_sort();

        // Should have 2 stages (t1 then t2)
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].len(), 1);
        assert_eq!(stages[1].len(), 1);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let executor = ParallelToolExecutor::default();

        let tools = vec![
            ToolCall {
                tool_id: "t1".to_string(),
                tool_name: "tool1".to_string(),
                params: serde_json::json!({}),
                outputs: vec![],
                inputs: vec![],
                priority: 5,
            },
            ToolCall {
                tool_id: "t2".to_string(),
                tool_name: "tool2".to_string(),
                params: serde_json::json!({}),
                outputs: vec![],
                inputs: vec![],
                priority: 5,
            },
        ];

        let result = executor.execute_tools(tools).await;
        assert!(result.is_ok());

        let (results, stats) = result.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(stats.successful, 2);
    }
}
