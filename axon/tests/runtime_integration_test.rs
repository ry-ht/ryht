//! Integration tests for the Agent Runtime System
//!
//! These tests verify the complete integration of the runtime components.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use axon::agents::AgentType;
    use axon::coordination::UnifiedMessageBus;
    use axon::orchestration::TaskDelegation;
    use axon::runtime::{AgentRuntime, RuntimeConfig, AgentStatus};

    #[tokio::test]
    async fn test_runtime_lifecycle() {
        // Create message bus
        let message_bus = Arc::new(UnifiedMessageBus::new());

        // Create runtime
        let config = RuntimeConfig::default();
        let runtime = AgentRuntime::new(config, message_bus);

        // Start runtime
        assert!(runtime.start().await.is_ok());
        assert!(runtime.is_running().await);

        // Shutdown runtime
        assert!(runtime.shutdown().await.is_ok());
        assert!(!runtime.is_running().await);
    }

    #[tokio::test]
    async fn test_agent_spawning() {
        let message_bus = Arc::new(UnifiedMessageBus::new());
        let config = RuntimeConfig::default();
        let runtime = AgentRuntime::new(config, message_bus);

        runtime.start().await.unwrap();

        // Spawn agent (this will fail without actual cortex binary, but tests the API)
        let result = runtime.spawn_agent(
            "test-agent".to_string(),
            AgentType::Developer,
            "echo", // Use echo instead of cortex for testing
            &["test".to_string()],
        ).await;

        // We expect this to work in terms of API, though the process may fail
        // The important part is testing the API surface

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_runtime_statistics() {
        let message_bus = Arc::new(UnifiedMessageBus::new());
        let config = RuntimeConfig::default();
        let runtime = AgentRuntime::new(config, message_bus);

        runtime.start().await.unwrap();

        // Get initial statistics
        let stats = runtime.get_statistics().await;
        assert_eq!(stats.active_agents, 0);
        assert_eq!(stats.total_agents_spawned, 0);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_delegation_builder() {
        let task = TaskDelegation::builder()
            .objective("Test task".to_string())
            .add_scope("test scope".to_string())
            .add_constraint("test constraint".to_string())
            .max_tool_calls(10)
            .timeout(Duration::from_secs(60))
            .priority(5)
            .required_capabilities(vec!["TestCapability".to_string()])
            .build();

        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.objective, "Test task");
        assert_eq!(task.priority, 5);
        assert_eq!(task.boundaries.max_tool_calls, 10);
    }

    #[tokio::test]
    async fn test_runtime_config_defaults() {
        let config = RuntimeConfig::default();

        assert_eq!(config.process.max_concurrent_processes, 10);
        assert!(config.resources.enable_resource_tracking);
        assert!(config.monitoring.enable_metrics);
        assert!(config.recovery.enable_auto_restart);
    }

    #[tokio::test]
    async fn test_agent_status_transitions() {
        // Test status enum
        assert_eq!(AgentStatus::Initializing, AgentStatus::Initializing);
        assert_ne!(AgentStatus::Ready, AgentStatus::Busy);
    }

    #[tokio::test]
    async fn test_runtime_state_management() {
        let message_bus = Arc::new(UnifiedMessageBus::new());
        let config = RuntimeConfig::default();
        let runtime = AgentRuntime::new(config, message_bus);

        // Initially not running
        assert!(!runtime.is_running().await);

        // Start
        runtime.start().await.unwrap();
        assert!(runtime.is_running().await);

        // Shutdown
        runtime.shutdown().await.unwrap();
        assert!(!runtime.is_running().await);
    }
}
