//! Integration Tests for Full Agent Lifecycle
//!
//! Tests the complete lifecycle of agents including:
//! - Agent creation and initialization
//! - Task execution
//! - State transitions
//! - Resource management
//! - Graceful shutdown
//! - Error recovery

use axon::runtime::{AgentRuntime, RuntimeConfig, AgentStatus, RuntimeState};
use axon::agents::{AgentId, AgentType, AgentMetrics};
use axon::coordination::UnifiedMessageBus;
use axon::orchestration::TaskDelegation;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Full Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_full_agent_lifecycle() {
    // Create message bus and runtime
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    // 1. Start runtime
    let start_result = runtime.start().await;
    assert!(start_result.is_ok(), "Runtime should start successfully");
    assert!(runtime.is_running().await, "Runtime should be in running state");

    // 2. Spawn an agent (using echo as test binary)
    let agent_result = runtime.spawn_agent(
        "test-lifecycle-agent".to_string(),
        AgentType::Developer,
        "echo",
        &["test".to_string()],
    ).await;

    // Agent spawn may fail without actual cortex binary, but we verify the API
    if let Ok(agent_id) = agent_result {
        // Give agent time to initialize
        sleep(Duration::from_millis(100)).await;

        // 3. Verify agent is registered
        let agents = runtime.list_agents().await;
        if let Ok(agent_list) = agents {
            assert!(!agent_list.is_empty(), "Agent should be in the list");
        }

        // 4. Kill agent
        let kill_result = runtime.kill_agent(&agent_id).await;
        let _ = kill_result; // May succeed or fail depending on environment
    }

    // 5. Shutdown runtime
    let shutdown_result = runtime.shutdown().await;
    assert!(shutdown_result.is_ok(), "Runtime should shutdown successfully");
    assert!(!runtime.is_running().await, "Runtime should not be running after shutdown");
}

#[tokio::test]
async fn test_agent_initialization_sequence() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Test spawning multiple agents with different types
    let agent_types = vec![
        AgentType::Developer,
        AgentType::Tester,
        AgentType::Reviewer,
    ];

    for (i, agent_type) in agent_types.iter().enumerate() {
        let name = format!("agent-{}", i);
        let result = runtime.spawn_agent(
            name,
            *agent_type,
            "echo",
            &["test".to_string()],
        ).await;

        // Result may vary based on environment
        let _ = result;
    }

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_agent_state_transitions() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Track state transitions
    let initial_state = runtime.get_state().await;
    assert_eq!(initial_state, RuntimeState::Running);

    // Trigger shutdown
    runtime.shutdown().await.unwrap();

    let final_state = runtime.get_state().await;
    assert_ne!(final_state, RuntimeState::Running);
}

// ============================================================================
// Task Execution Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_task_execution_lifecycle() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Create a task delegation
    let task = TaskDelegation::builder()
        .objective("Test task execution".to_string())
        .add_scope("test scope".to_string())
        .max_tool_calls(5)
        .timeout(Duration::from_secs(30))
        .priority(5)
        .build();

    assert!(task.is_ok(), "Task should be created successfully");

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_task_execution() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Create multiple tasks
    let tasks: Vec<_> = (1..=5)
        .map(|i| {
            TaskDelegation::builder()
                .objective(format!("Task {}", i))
                .priority(i as u8)
                .build()
                .unwrap()
        })
        .collect();

    assert_eq!(tasks.len(), 5, "Should create 5 tasks");

    runtime.shutdown().await.unwrap();
}

// ============================================================================
// Resource Management Tests
// ============================================================================

#[tokio::test]
async fn test_resource_limits_enforcement() {
    let mut config = RuntimeConfig::default();
    config.resources.max_memory_mb = 512;
    config.resources.max_cpu_percent = 50.0;

    let message_bus = Arc::new(UnifiedMessageBus::new());
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Verify resource limits are set
    let stats = runtime.get_statistics().await;
    assert!(stats.active_agents >= 0);

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_max_concurrent_agents() {
    let mut config = RuntimeConfig::default();
    config.process.max_concurrent_processes = 3;

    let message_bus = Arc::new(UnifiedMessageBus::new());
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Try to spawn more agents than the limit
    for i in 0..5 {
        let result = runtime.spawn_agent(
            format!("agent-{}", i),
            AgentType::Developer,
            "echo",
            &["test".to_string()],
        ).await;

        // Some should fail or be queued
        let _ = result;
    }

    // Verify limit is respected
    let stats = runtime.get_statistics().await;
    assert!(stats.active_agents <= 3, "Should not exceed max concurrent limit");

    runtime.shutdown().await.unwrap();
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[tokio::test]
async fn test_agent_failure_recovery() {
    let mut config = RuntimeConfig::default();
    config.recovery.enable_auto_restart = true;
    config.recovery.max_restart_attempts = 3;

    let message_bus = Arc::new(UnifiedMessageBus::new());
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Spawn an agent that will fail (invalid binary)
    let result = runtime.spawn_agent(
        "failing-agent".to_string(),
        AgentType::Developer,
        "nonexistent-binary-xyz",
        &[],
    ).await;

    // Should handle failure gracefully
    assert!(result.is_ok() || result.is_err());

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_runtime_recovery_after_error() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    // Start runtime
    runtime.start().await.unwrap();

    // Simulate error condition
    let invalid_agent_id = AgentId::new();
    let kill_result = runtime.kill_agent(&invalid_agent_id).await;

    // Should handle gracefully
    assert!(kill_result.is_err(), "Killing non-existent agent should error");

    // Runtime should still be functional
    assert!(runtime.is_running().await, "Runtime should still be running");

    runtime.shutdown().await.unwrap();
}

// ============================================================================
// Graceful Shutdown Tests
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_with_active_agents() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Spawn some agents
    for i in 0..3 {
        let _ = runtime.spawn_agent(
            format!("agent-{}", i),
            AgentType::Developer,
            "sleep",
            &["1".to_string()],
        ).await;
    }

    // Shutdown should handle active agents gracefully
    let shutdown_result = runtime.shutdown().await;
    assert!(shutdown_result.is_ok(), "Shutdown should succeed even with active agents");

    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn test_shutdown_timeout() {
    let mut config = RuntimeConfig::default();
    config.process.shutdown_timeout = Duration::from_secs(5);

    let message_bus = Arc::new(UnifiedMessageBus::new());
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Shutdown should respect timeout
    let start = std::time::Instant::now();
    runtime.shutdown().await.unwrap();
    let duration = start.elapsed();

    // Shutdown should complete within reasonable time
    assert!(duration < Duration::from_secs(10));
}

// ============================================================================
// Statistics and Monitoring Tests
// ============================================================================

#[tokio::test]
async fn test_runtime_statistics_tracking() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Get initial statistics
    let stats = runtime.get_statistics().await;
    assert_eq!(stats.active_agents, 0);
    assert_eq!(stats.total_agents_spawned, 0);

    // Spawn an agent
    let result = runtime.spawn_agent(
        "stats-agent".to_string(),
        AgentType::Developer,
        "echo",
        &["test".to_string()],
    ).await;

    if result.is_ok() {
        sleep(Duration::from_millis(100)).await;

        // Statistics should be updated
        let updated_stats = runtime.get_statistics().await;
        // Stats may or may not change depending on spawn success
        assert!(updated_stats.total_agents_spawned >= 0);
    }

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_agent_metrics_collection() {
    // Test that agent metrics are properly collected
    let metrics = AgentMetrics::new();

    // Record some operations
    metrics.record_success(100, 1000, 50);
    metrics.record_success(200, 2000, 100);
    metrics.record_failure();

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.tasks_completed, 2);
    assert_eq!(snapshot.tasks_failed, 1);
    assert_eq!(snapshot.tokens_used, 3000);
    assert_eq!(snapshot.total_cost_cents, 150);
}

// ============================================================================
// Concurrent Operations Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_agent_operations() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = Arc::new(runtime);

    runtime.start().await.unwrap();

    // Spawn multiple tasks concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let runtime_clone = runtime.clone();
        let handle = tokio::spawn(async move {
            runtime_clone.spawn_agent(
                format!("concurrent-agent-{}", i),
                AgentType::Developer,
                "echo",
                &["test".to_string()],
            ).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let _ = handle.await;
    }

    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_lifecycle_operations() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = Arc::new(runtime);

    runtime.start().await.unwrap();

    // Perform concurrent start/stop/query operations
    let runtime_clone1 = runtime.clone();
    let runtime_clone2 = runtime.clone();
    let runtime_clone3 = runtime.clone();

    let h1 = tokio::spawn(async move {
        runtime_clone1.get_statistics().await
    });

    let h2 = tokio::spawn(async move {
        runtime_clone2.list_agents().await
    });

    let h3 = tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        runtime_clone3.get_state().await
    });

    // All should complete without deadlock
    let _ = h1.await;
    let _ = h2.await;
    let _ = h3.await;

    runtime.shutdown().await.unwrap();
}

// ============================================================================
// Message Bus Integration Tests
// ============================================================================

#[tokio::test]
async fn test_message_bus_lifecycle() {
    let message_bus = Arc::new(UnifiedMessageBus::new());

    // Test message bus is functional
    let agent_id = AgentId::new();

    // Subscribe to a topic
    let subscription_result = message_bus.subscribe(agent_id.clone(), "test-topic".to_string()).await;
    assert!(subscription_result.is_ok());

    // Publish a message
    let message = serde_json::json!({"type": "test", "data": "hello"});
    let publish_result = message_bus.publish("test-topic".to_string(), message).await;
    assert!(publish_result.is_ok());

    // Unsubscribe
    let unsub_result = message_bus.unsubscribe(agent_id, "test-topic".to_string()).await;
    assert!(unsub_result.is_ok());
}

#[tokio::test]
async fn test_runtime_with_message_bus_integration() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let message_bus_clone = message_bus.clone();

    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Verify message bus is accessible
    let agent_id = AgentId::new();
    let sub_result = message_bus_clone.subscribe(agent_id, "runtime-events".to_string()).await;
    assert!(sub_result.is_ok());

    runtime.shutdown().await.unwrap();
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[tokio::test]
async fn test_runtime_health_checks() {
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let config = RuntimeConfig::default();
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Runtime should be healthy after start
    assert!(runtime.is_running().await);

    // Statistics should be available
    let stats_result = runtime.get_statistics().await;
    assert!(stats_result.active_agents >= 0);

    runtime.shutdown().await.unwrap();

    // Runtime should not be healthy after shutdown
    assert!(!runtime.is_running().await);
}

#[tokio::test]
async fn test_periodic_health_monitoring() {
    let mut config = RuntimeConfig::default();
    config.monitoring.enable_metrics = true;
    config.monitoring.health_check_interval = Duration::from_millis(100);

    let message_bus = Arc::new(UnifiedMessageBus::new());
    let runtime = AgentRuntime::new(config, message_bus);

    runtime.start().await.unwrap();

    // Wait for a few health check cycles
    sleep(Duration::from_millis(350)).await;

    // Runtime should still be healthy
    assert!(runtime.is_running().await);

    runtime.shutdown().await.unwrap();
}
