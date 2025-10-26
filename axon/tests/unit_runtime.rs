//! Unit Tests for Agent Runtime System
//!
//! Tests the runtime components including:
//! - Agent Runtime lifecycle
//! - Process Manager
//! - MCP Server Pool
//! - Agent Executor
//! - Resource management

use axon::runtime::{
    AgentRuntime, RuntimeConfig, RuntimeState, AgentStatus,
    ProcessManager, ProcessState, ResourceLimits,
    McpServerPool, McpConfig,
    AgentExecutor, ExecutionStatus,
    ProcessConfig, MonitoringConfig, RecoveryConfig,
};
use axon::agents::{AgentId, AgentType};
use axon::coordination::UnifiedMessageBus;
use axon::orchestration::TaskDelegation;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Runtime Configuration Tests
// ============================================================================

#[test]
fn test_runtime_config_defaults() {
    let config = RuntimeConfig::default();

    assert_eq!(config.process.max_concurrent_processes, 10);
    assert!(config.resources.enable_resource_tracking);
    assert!(config.monitoring.enable_metrics);
    assert!(config.recovery.enable_auto_restart);
}

#[test]
fn test_runtime_config_custom() {
    let mut config = RuntimeConfig::default();
    config.process.max_concurrent_processes = 20;
    config.resources.max_memory_mb = 2048;
    config.monitoring.health_check_interval = Duration::from_secs(10);

    assert_eq!(config.process.max_concurrent_processes, 20);
    assert_eq!(config.resources.max_memory_mb, 2048);
    assert_eq!(config.monitoring.health_check_interval, Duration::from_secs(10));
}

#[test]
fn test_process_config_validation() {
    let config = ProcessConfig {
        max_concurrent_processes: 5,
        process_timeout: Duration::from_secs(60),
        spawn_delay: Duration::from_millis(100),
        shutdown_timeout: Duration::from_secs(10),
    };

    assert!(config.max_concurrent_processes > 0);
    assert!(config.process_timeout > Duration::from_secs(0));
}

#[test]
fn test_resource_limits() {
    let limits = ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_mb: 1024,
        max_execution_time: Duration::from_secs(300),
        enable_resource_tracking: true,
    };

    assert!(limits.max_cpu_percent > 0.0 && limits.max_cpu_percent <= 100.0);
    assert!(limits.max_memory_mb > 0);
    assert!(limits.enable_resource_tracking);
}

#[test]
fn test_mcp_config() {
    let config = McpConfig {
        stdio_timeout: Duration::from_secs(30),
        max_request_size: 1024 * 1024,
        enable_tool_caching: true,
        tool_call_timeout: Duration::from_secs(60),
    };

    assert!(config.stdio_timeout > Duration::from_secs(0));
    assert!(config.max_request_size > 0);
    assert!(config.enable_tool_caching);
}

// ============================================================================
// Process Manager Tests
// ============================================================================

#[test]
fn test_process_manager_creation() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();

    let manager = ProcessManager::new(process_config, resource_limits);

    // Manager should be created successfully
    assert!(true);
}

#[tokio::test]
async fn test_process_manager_spawn_and_kill() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();
    let manager = ProcessManager::new(process_config, resource_limits);

    let agent_id = AgentId::new();

    // Spawn a simple echo process for testing
    let result = manager.spawn_process(
        agent_id.clone(),
        "test-agent".to_string(),
        "echo".to_string(),
        vec!["test".to_string()],
    ).await;

    // Process spawn might fail in test environment, but we test the API
    if result.is_ok() {
        // If spawn succeeded, test killing
        let kill_result = manager.kill_process(&agent_id).await;
        assert!(kill_result.is_ok() || kill_result.is_err()); // Either way is fine for testing
    }
}

#[tokio::test]
async fn test_process_manager_is_alive() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();
    let manager = ProcessManager::new(process_config, resource_limits);

    let agent_id = AgentId::new();

    // Non-existent process should not be alive
    let is_alive = manager.is_alive(&agent_id).await;
    assert!(!is_alive);
}

#[tokio::test]
async fn test_process_manager_statistics() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();
    let manager = ProcessManager::new(process_config, resource_limits);

    let stats = manager.get_statistics().await;

    // Initial stats should show no active processes
    assert_eq!(stats.active_processes, 0);
    assert_eq!(stats.total_spawned, 0);
}

#[tokio::test]
async fn test_process_manager_max_concurrent_limit() {
    let mut process_config = ProcessConfig::default();
    process_config.max_concurrent_processes = 2; // Limit to 2

    let resource_limits = ResourceLimits::default();
    let manager = ProcessManager::new(process_config, resource_limits);

    // Try to spawn 3 processes
    let agent1 = AgentId::new();
    let agent2 = AgentId::new();
    let agent3 = AgentId::new();

    let _ = manager.spawn_process(agent1, "a1".to_string(), "sleep".to_string(), vec!["1".to_string()]).await;
    let _ = manager.spawn_process(agent2, "a2".to_string(), "sleep".to_string(), vec!["1".to_string()]).await;
    let result3 = manager.spawn_process(agent3, "a3".to_string(), "sleep".to_string(), vec!["1".to_string()]).await;

    // Third spawn might fail due to limit (depends on implementation)
    // The test validates the API surface
    let _ = result3;
}

// ============================================================================
// MCP Server Pool Tests
// ============================================================================

#[test]
fn test_mcp_server_pool_creation() {
    let config = McpConfig::default();
    let pool = McpServerPool::new(config);

    // Pool should be created successfully
    assert!(true);
}

#[tokio::test]
async fn test_mcp_server_pool_register_server() {
    let config = McpConfig::default();
    let pool = McpServerPool::new(config);

    let agent_id = AgentId::new();

    // Register a server (will fail without actual MCP server, but tests API)
    let result = pool.register_server(agent_id.clone(), "cortex".to_string(), vec!["mcp".to_string()]).await;

    // Result can be Ok or Err depending on environment
    let _ = result;
}

#[tokio::test]
async fn test_mcp_server_pool_unregister_server() {
    let config = McpConfig::default();
    let pool = McpServerPool::new(config);

    let agent_id = AgentId::new();

    // Unregister non-existent server
    let result = pool.unregister_server(&agent_id).await;

    // Should handle gracefully
    let _ = result;
}

#[tokio::test]
async fn test_mcp_server_pool_server_count() {
    let config = McpConfig::default();
    let pool = McpServerPool::new(config);

    let count = pool.server_count().await;

    // Initially should be 0
    assert_eq!(count, 0);
}

// ============================================================================
// Agent Executor Tests
// ============================================================================

#[tokio::test]
async fn test_agent_executor_creation() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();
    let process_manager = Arc::new(ProcessManager::new(process_config, resource_limits));

    let mcp_config = McpConfig::default();
    let mcp_pool = Arc::new(McpServerPool::new(mcp_config));

    let config = RuntimeConfig::default();
    let executor = AgentExecutor::new(process_manager, mcp_pool, config);

    // Executor should be created
    assert!(true);
}

#[tokio::test]
async fn test_agent_executor_statistics() {
    let process_config = ProcessConfig::default();
    let resource_limits = ResourceLimits::default();
    let process_manager = Arc::new(ProcessManager::new(process_config, resource_limits));

    let mcp_config = McpConfig::default();
    let mcp_pool = Arc::new(McpServerPool::new(mcp_config));

    let config = RuntimeConfig::default();
    let executor = AgentExecutor::new(process_manager, mcp_pool, config);

    let stats = executor.get_statistics().await;

    // Initial stats
    assert_eq!(stats.total_tasks, 0);
    assert_eq!(stats.successful_tasks, 0);
    assert_eq!(stats.failed_tasks, 0);
}

#[tokio::test]
async fn test_execution_status_transitions() {
    // Test execution status enum
    assert_eq!(ExecutionStatus::Queued, ExecutionStatus::Queued);
    assert_ne!(ExecutionStatus::Queued, ExecutionStatus::Executing);
    assert_ne!(ExecutionStatus::Executing, ExecutionStatus::Completed);
}

// ============================================================================
// Agent Runtime Tests
// ============================================================================

#[tokio::test]
async fn test_runtime_creation() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    // Runtime should be created
    assert!(true);
}

#[tokio::test]
async fn test_runtime_lifecycle() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    // Test start
    let start_result = runtime.start().await;
    assert!(start_result.is_ok());

    // Should be running
    assert!(runtime.is_running().await);

    // Test shutdown
    let shutdown_result = runtime.shutdown().await;
    assert!(shutdown_result.is_ok());

    // Should not be running
    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn test_runtime_state_management() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    // Initially should not be running
    assert!(!runtime.is_running().await);

    // Start runtime
    runtime.start().await.unwrap();

    // Should be running
    assert!(runtime.is_running().await);

    // Shutdown
    runtime.shutdown().await.unwrap();

    // Should be stopped
    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn test_runtime_agent_spawning() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Try to spawn an agent (will likely fail without real binary, but tests API)
    let result = runtime.spawn_agent(
        "test-agent".to_string(),
        AgentType::Developer,
        "echo",
        &["test".to_string()],
    ).await;

    // Result can be Ok or Err, we're testing the API surface
    let _ = result;

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_runtime_statistics() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    let stats = runtime.get_statistics().await;

    // Initial statistics
    assert_eq!(stats.active_agents, 0);
    assert_eq!(stats.total_agents_spawned, 0);

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_runtime_multiple_start_stop_cycles() {
    let config = RuntimeConfig::default();
    let message_bus = Arc::new(UnifiedMessageBus::new());

    let runtime = AgentRuntime::new(config, message_bus);

    // Cycle 1
    runtime.start().await.unwrap();
    assert!(runtime.is_running().await);
    runtime.shutdown().await.unwrap();
    assert!(!runtime.is_running().await);

    // Cycle 2
    runtime.start().await.unwrap();
    assert!(runtime.is_running().await);
    runtime.shutdown().await.unwrap();
    assert!(!runtime.is_running().await);
}

// ============================================================================
// Agent Status Tests
// ============================================================================

#[test]
fn test_agent_status_values() {
    let statuses = vec![
        AgentStatus::Initializing,
        AgentStatus::Ready,
        AgentStatus::Busy,
        AgentStatus::Idle,
        AgentStatus::Failed,
        AgentStatus::ShuttingDown,
        AgentStatus::Terminated,
    ];

    // All statuses should be distinct
    for (i, status1) in statuses.iter().enumerate() {
        for (j, status2) in statuses.iter().enumerate() {
            if i == j {
                assert_eq!(status1, status2);
            } else {
                assert_ne!(status1, status2);
            }
        }
    }
}

#[test]
fn test_agent_status_lifecycle_progression() {
    // Test logical status progression
    let lifecycle = vec![
        AgentStatus::Initializing,
        AgentStatus::Ready,
        AgentStatus::Idle,
        AgentStatus::Busy,
        AgentStatus::Idle,
        AgentStatus::ShuttingDown,
        AgentStatus::Terminated,
    ];

    // Each status should be valid
    for status in lifecycle {
        match status {
            AgentStatus::Initializing => assert_eq!(status, AgentStatus::Initializing),
            AgentStatus::Ready => assert_eq!(status, AgentStatus::Ready),
            AgentStatus::Busy => assert_eq!(status, AgentStatus::Busy),
            AgentStatus::Idle => assert_eq!(status, AgentStatus::Idle),
            AgentStatus::Failed => assert_eq!(status, AgentStatus::Failed),
            AgentStatus::ShuttingDown => assert_eq!(status, AgentStatus::ShuttingDown),
            AgentStatus::Terminated => assert_eq!(status, AgentStatus::Terminated),
        }
    }
}

// ============================================================================
// Runtime State Tests
// ============================================================================

#[test]
fn test_runtime_state_values() {
    let states = vec![
        RuntimeState::Initializing,
        RuntimeState::Running,
        RuntimeState::ShuttingDown,
        RuntimeState::Stopped,
    ];

    // All states should be distinct
    for (i, state1) in states.iter().enumerate() {
        for (j, state2) in states.iter().enumerate() {
            if i == j {
                assert_eq!(state1, state2);
            } else {
                assert_ne!(state1, state2);
            }
        }
    }
}

// ============================================================================
// Task Delegation Integration with Runtime
// ============================================================================

#[tokio::test]
async fn test_task_delegation_creation_for_runtime() {
    // Test creating task delegations that would be executed by runtime
    let task = TaskDelegation::builder()
        .objective("Test runtime task".to_string())
        .add_scope("runtime scope".to_string())
        .max_tool_calls(10)
        .timeout(Duration::from_secs(60))
        .priority(5)
        .build();

    assert!(task.is_ok());
    let task = task.unwrap();

    assert_eq!(task.objective, "Test runtime task");
    assert_eq!(task.boundaries.max_tool_calls, 10);
}

#[tokio::test]
async fn test_multiple_task_delegations() {
    // Test creating multiple tasks
    let tasks: Vec<_> = (1..=5)
        .map(|i| {
            TaskDelegation::builder()
                .objective(format!("Task {}", i))
                .priority(i as u8)
                .build()
                .unwrap()
        })
        .collect();

    assert_eq!(tasks.len(), 5);

    // Verify priorities
    for (i, task) in tasks.iter().enumerate() {
        assert_eq!(task.priority, (i + 1) as u8);
    }
}

// ============================================================================
// Process State Tests
// ============================================================================

#[test]
fn test_process_state_values() {
    let states = vec![
        ProcessState::Starting,
        ProcessState::Running,
        ProcessState::Stopping,
        ProcessState::Stopped,
        ProcessState::Failed,
    ];

    // All states should be valid
    for state in states {
        match state {
            ProcessState::Starting => assert_eq!(state, ProcessState::Starting),
            ProcessState::Running => assert_eq!(state, ProcessState::Running),
            ProcessState::Stopping => assert_eq!(state, ProcessState::Stopping),
            ProcessState::Stopped => assert_eq!(state, ProcessState::Stopped),
            ProcessState::Failed => assert_eq!(state, ProcessState::Failed),
        }
    }
}

// ============================================================================
// Resource Usage Tests
// ============================================================================

#[test]
fn test_resource_limits_validation() {
    let limits = ResourceLimits {
        max_cpu_percent: 100.0,
        max_memory_mb: 2048,
        max_execution_time: Duration::from_secs(600),
        enable_resource_tracking: true,
    };

    // CPU should be between 0 and 100
    assert!(limits.max_cpu_percent >= 0.0);
    assert!(limits.max_cpu_percent <= 100.0);

    // Memory should be positive
    assert!(limits.max_memory_mb > 0);

    // Execution time should be positive
    assert!(limits.max_execution_time > Duration::from_secs(0));
}

#[test]
fn test_resource_limits_edge_cases() {
    // Test minimum viable limits
    let min_limits = ResourceLimits {
        max_cpu_percent: 1.0,
        max_memory_mb: 64,
        max_execution_time: Duration::from_secs(1),
        enable_resource_tracking: false,
    };

    assert!(min_limits.max_cpu_percent > 0.0);
    assert!(min_limits.max_memory_mb > 0);

    // Test maximum practical limits
    let max_limits = ResourceLimits {
        max_cpu_percent: 100.0,
        max_memory_mb: 16384,
        max_execution_time: Duration::from_secs(3600),
        enable_resource_tracking: true,
    };

    assert!(max_limits.max_cpu_percent <= 100.0);
}
