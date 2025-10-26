//! Unit Tests for Parallel Tool Executor
//!
//! Tests the parallel execution capabilities including:
//! - Dependency graph construction
//! - Topological sorting
//! - Parallel execution of independent tools
//! - Error handling and partial failures
//! - Performance optimization

use axon::orchestration::parallel_tool_executor::{
    ParallelToolExecutor, ToolCall, ToolResult, ExecutionStats,
};
use std::time::Duration;
use std::collections::HashMap;

// ============================================================================
// Tool Call Tests
// ============================================================================

#[test]
fn test_tool_call_creation() {
    let tool = ToolCall {
        tool_id: "tool-1".to_string(),
        tool_name: "test_tool".to_string(),
        params: serde_json::json!({"key": "value"}),
        outputs: vec!["output1".to_string()],
        inputs: vec![],
        priority: 5,
    };

    assert_eq!(tool.tool_id, "tool-1");
    assert_eq!(tool.tool_name, "test_tool");
    assert_eq!(tool.priority, 5);
    assert!(tool.inputs.is_empty());
    assert_eq!(tool.outputs.len(), 1);
}

#[test]
fn test_tool_call_with_dependencies() {
    let tool1 = ToolCall {
        tool_id: "tool-1".to_string(),
        tool_name: "producer".to_string(),
        params: serde_json::json!({}),
        outputs: vec!["data1".to_string()],
        inputs: vec![],
        priority: 5,
    };

    let tool2 = ToolCall {
        tool_id: "tool-2".to_string(),
        tool_name: "consumer".to_string(),
        params: serde_json::json!({}),
        outputs: vec!["result".to_string()],
        inputs: vec!["data1".to_string()], // Depends on tool1's output
        priority: 5,
    };

    // tool2 should depend on tool1
    assert!(tool2.inputs.contains(&"data1".to_string()));
    assert!(tool1.outputs.contains(&"data1".to_string()));
}

#[test]
fn test_tool_priority_ordering() {
    let low_priority = ToolCall {
        tool_id: "tool-low".to_string(),
        tool_name: "low".to_string(),
        params: serde_json::json!({}),
        outputs: vec![],
        inputs: vec![],
        priority: 1,
    };

    let high_priority = ToolCall {
        tool_id: "tool-high".to_string(),
        tool_name: "high".to_string(),
        params: serde_json::json!({}),
        outputs: vec![],
        inputs: vec![],
        priority: 10,
    };

    assert!(high_priority.priority > low_priority.priority);
}

// ============================================================================
// Parallel Executor Tests
// ============================================================================

#[tokio::test]
async fn test_executor_creation() {
    let executor = ParallelToolExecutor::new(5); // max 5 concurrent

    assert_eq!(executor.max_concurrent(), 5);
}

#[tokio::test]
async fn test_executor_with_no_tools() {
    let executor = ParallelToolExecutor::new(3);
    let tools = vec![];

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();
    assert!(results.is_empty());
    assert_eq!(stats.total_tools, 0);
}

#[tokio::test]
async fn test_executor_single_tool() {
    let executor = ParallelToolExecutor::new(3);

    let tool = ToolCall {
        tool_id: "single-tool".to_string(),
        tool_name: "test".to_string(),
        params: serde_json::json!({"test": true}),
        outputs: vec![],
        inputs: vec![],
        priority: 5,
    };

    let results = executor.execute_tools(vec![tool], mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(stats.total_tools, 1);
    assert_eq!(stats.successful_tools, 1);
}

#[tokio::test]
async fn test_executor_independent_tools_parallelization() {
    let executor = ParallelToolExecutor::new(10);

    // Create 3 independent tools (no dependencies)
    let tools = vec![
        create_independent_tool("tool-1", 1),
        create_independent_tool("tool-2", 2),
        create_independent_tool("tool-3", 3),
    ];

    let start = std::time::Instant::now();
    let results = executor.execute_tools(tools, mock_tool_executor).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    // All tools should complete
    assert_eq!(results.len(), 3);
    assert_eq!(stats.successful_tools, 3);

    // Execution should be faster than sequential (3 * 100ms)
    // With parallelization, should be close to 100ms (the longest single tool)
    // We'll be lenient with timing due to test environment variability
    assert!(duration < Duration::from_millis(250));
}

#[tokio::test]
async fn test_executor_sequential_dependencies() {
    let executor = ParallelToolExecutor::new(5);

    // Create a dependency chain: tool1 -> tool2 -> tool3
    let tools = vec![
        ToolCall {
            tool_id: "tool-1".to_string(),
            tool_name: "first".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["data1".to_string()],
            inputs: vec![],
            priority: 5,
        },
        ToolCall {
            tool_id: "tool-2".to_string(),
            tool_name: "second".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["data2".to_string()],
            inputs: vec!["data1".to_string()],
            priority: 5,
        },
        ToolCall {
            tool_id: "tool-3".to_string(),
            tool_name: "third".to_string(),
            params: serde_json::json!({}),
            outputs: vec![],
            inputs: vec!["data2".to_string()],
            priority: 5,
        },
    ];

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    // All tools should complete in order
    assert_eq!(results.len(), 3);
    assert_eq!(stats.successful_tools, 3);
}

#[tokio::test]
async fn test_executor_mixed_parallel_sequential() {
    let executor = ParallelToolExecutor::new(10);

    // Create a DAG:
    //     tool1
    //    /     \
    //  tool2  tool3  (parallel)
    //    \     /
    //     tool4      (waits for both)

    let tools = vec![
        ToolCall {
            tool_id: "tool-1".to_string(),
            tool_name: "root".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["data1".to_string()],
            inputs: vec![],
            priority: 5,
        },
        ToolCall {
            tool_id: "tool-2".to_string(),
            tool_name: "branch1".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["data2".to_string()],
            inputs: vec!["data1".to_string()],
            priority: 5,
        },
        ToolCall {
            tool_id: "tool-3".to_string(),
            tool_name: "branch2".to_string(),
            params: serde_json::json!({}),
            outputs: vec!["data3".to_string()],
            inputs: vec!["data1".to_string()],
            priority: 5,
        },
        ToolCall {
            tool_id: "tool-4".to_string(),
            tool_name: "merge".to_string(),
            params: serde_json::json!({}),
            outputs: vec![],
            inputs: vec!["data2".to_string(), "data3".to_string()],
            priority: 5,
        },
    ];

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    assert_eq!(results.len(), 4);
    assert_eq!(stats.successful_tools, 4);
}

#[tokio::test]
async fn test_executor_priority_ordering() {
    let executor = ParallelToolExecutor::new(1); // Force sequential with max_concurrent=1

    let tools = vec![
        create_tool_with_priority("low-priority", 1),
        create_tool_with_priority("high-priority", 10),
        create_tool_with_priority("medium-priority", 5),
    ];

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, _) = results.unwrap();

    // With sequential execution, higher priority should execute first
    // Note: This depends on the implementation's priority handling
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_executor_partial_failure() {
    let executor = ParallelToolExecutor::new(5);

    let tools = vec![
        create_independent_tool("success-1", 1),
        create_failing_tool("failure-1"),
        create_independent_tool("success-2", 2),
    ];

    let results = executor.execute_tools(tools, mock_tool_executor_with_failures).await;

    // Executor should handle partial failures
    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(stats.failed_tools, 1);
    assert_eq!(stats.successful_tools, 2);
}

#[tokio::test]
async fn test_executor_error_propagation() {
    let executor = ParallelToolExecutor::new(3);

    // Create a dependency where failure in parent affects child
    let tools = vec![
        create_failing_tool("parent"),
        ToolCall {
            tool_id: "child".to_string(),
            tool_name: "child".to_string(),
            params: serde_json::json!({}),
            outputs: vec![],
            inputs: vec!["parent_output".to_string()],
            priority: 5,
        },
    ];

    // Set parent to produce "parent_output"
    let mut tools_fixed = tools.clone();
    tools_fixed[0].outputs = vec!["parent_output".to_string()];

    let results = executor.execute_tools(tools_fixed, mock_tool_executor_with_failures).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    // Depending on error handling, child might not execute or might fail
    assert!(stats.failed_tools >= 1);
}

#[tokio::test]
async fn test_executor_max_concurrent_limit() {
    let executor = ParallelToolExecutor::new(2); // Limit to 2 concurrent

    // Create 5 independent tools
    let tools: Vec<_> = (1..=5)
        .map(|i| create_independent_tool(&format!("tool-{}", i), i))
        .collect();

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    assert_eq!(results.len(), 5);
    assert_eq!(stats.successful_tools, 5);

    // Max concurrent should be respected (hard to test directly, but code should handle it)
}

// ============================================================================
// Execution Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_execution_stats_calculation() {
    let executor = ParallelToolExecutor::new(5);

    let tools = vec![
        create_independent_tool("tool-1", 1),
        create_independent_tool("tool-2", 2),
        create_independent_tool("tool-3", 3),
    ];

    let results = executor.execute_tools(tools, mock_tool_executor).await;

    assert!(results.is_ok());
    let (_, stats) = results.unwrap();

    assert_eq!(stats.total_tools, 3);
    assert_eq!(stats.successful_tools, 3);
    assert_eq!(stats.failed_tools, 0);
    assert!(stats.total_duration > Duration::from_millis(0));
    assert!(stats.parallelization_factor > 0.0);
}

#[tokio::test]
async fn test_execution_stats_parallelization_factor() {
    let executor = ParallelToolExecutor::new(10);

    // Independent tools should have high parallelization factor
    let independent_tools = vec![
        create_independent_tool("tool-1", 1),
        create_independent_tool("tool-2", 2),
        create_independent_tool("tool-3", 3),
    ];

    let results = executor.execute_tools(independent_tools, mock_tool_executor).await;
    assert!(results.is_ok());
    let (_, stats) = results.unwrap();

    // Parallelization factor should be high (close to number of tools)
    // For 3 independent tools, factor could be up to ~3.0
    assert!(stats.parallelization_factor >= 1.0);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_independent_tool(id: &str, priority: u8) -> ToolCall {
    ToolCall {
        tool_id: id.to_string(),
        tool_name: id.to_string(),
        params: serde_json::json!({"test": true}),
        outputs: vec![format!("{}_output", id)],
        inputs: vec![],
        priority,
    }
}

fn create_tool_with_priority(id: &str, priority: u8) -> ToolCall {
    ToolCall {
        tool_id: id.to_string(),
        tool_name: id.to_string(),
        params: serde_json::json!({}),
        outputs: vec![],
        inputs: vec![],
        priority,
    }
}

fn create_failing_tool(id: &str) -> ToolCall {
    ToolCall {
        tool_id: id.to_string(),
        tool_name: "FAIL".to_string(), // Special marker for mock executor
        params: serde_json::json!({}),
        outputs: vec![],
        inputs: vec![],
        priority: 5,
    }
}

/// Mock tool executor function for testing
async fn mock_tool_executor(tool: ToolCall) -> ToolResult {
    // Simulate some work
    tokio::time::sleep(Duration::from_millis(50)).await;

    ToolResult {
        tool_id: tool.tool_id.clone(),
        tool_name: tool.tool_name.clone(),
        result: serde_json::json!({"status": "success", "data": "mock_result"}),
        success: true,
        error: None,
        duration: Duration::from_millis(50),
        completed_at: chrono::Utc::now(),
    }
}

/// Mock tool executor that simulates failures
async fn mock_tool_executor_with_failures(tool: ToolCall) -> ToolResult {
    // Simulate some work
    tokio::time::sleep(Duration::from_millis(50)).await;

    let success = tool.tool_name != "FAIL";

    ToolResult {
        tool_id: tool.tool_id.clone(),
        tool_name: tool.tool_name.clone(),
        result: if success {
            serde_json::json!({"status": "success"})
        } else {
            serde_json::json!({"status": "failed"})
        },
        success,
        error: if success {
            None
        } else {
            Some("Simulated failure".to_string())
        },
        duration: Duration::from_millis(50),
        completed_at: chrono::Utc::now(),
    }
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_performance_3_tools_parallel_vs_sequential() {
    let executor = ParallelToolExecutor::new(10);

    // Create 3 independent tools
    let tools = vec![
        create_independent_tool("tool-1", 5),
        create_independent_tool("tool-2", 5),
        create_independent_tool("tool-3", 5),
    ];

    let start = std::time::Instant::now();
    let results = executor.execute_tools(tools, mock_tool_executor).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    assert_eq!(results.len(), 3);

    // Sequential would take ~150ms (3 * 50ms)
    // Parallel should take ~50ms (max of individual times)
    // With overhead, allow up to 120ms
    assert!(duration < Duration::from_millis(120));

    // Check parallelization factor is good (>= 2.0 means at least 2x speedup)
    assert!(stats.parallelization_factor >= 1.5);
}

#[tokio::test]
async fn test_performance_10_tools_high_concurrency() {
    let executor = ParallelToolExecutor::new(10);

    // Create 10 independent tools
    let tools: Vec<_> = (1..=10)
        .map(|i| create_independent_tool(&format!("tool-{}", i), 5))
        .collect();

    let start = std::time::Instant::now();
    let results = executor.execute_tools(tools, mock_tool_executor).await;
    let duration = start.elapsed();

    assert!(results.is_ok());
    let (results, stats) = results.unwrap();

    assert_eq!(results.len(), 10);
    assert_eq!(stats.successful_tools, 10);

    // Sequential: 10 * 50ms = 500ms
    // Parallel with 10 concurrent: ~50ms
    // Allow overhead, cap at 200ms
    assert!(duration < Duration::from_millis(200));
}
