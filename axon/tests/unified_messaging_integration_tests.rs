//! Integration tests for unified messaging system with Cortex
//!
//! These tests verify the complete integration of the unified message bus,
//! message coordinator, episodic memory persistence, distributed locking,
//! and resilience patterns.

use axon::coordination::{
    Message, MessageBusConfig, MessageCoordinator, UnifiedMessageBus,
    AgentMessagingAdapter, AgentMessagingAdapterBuilder,
};
use axon::cortex_bridge::{CortexBridge, CortexConfig, LockType};
use axon::agents::AgentId;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ==============================================================================
// Test Utilities
// ==============================================================================

/// Create a test Cortex bridge (mock or real depending on environment)
async fn create_test_cortex() -> Arc<CortexBridge> {
    let config = CortexConfig {
        base_url: std::env::var("CORTEX_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string()),
        api_version: "v3".to_string(),
        request_timeout_secs: 10,
        max_retries: 3,
    };

    Arc::new(
        CortexBridge::new(config)
            .await
            .expect("Failed to create Cortex bridge")
    )
}

/// Create a test message bus with Cortex
async fn create_test_bus(cortex: Arc<CortexBridge>) -> Arc<UnifiedMessageBus> {
    let config = MessageBusConfig {
        max_history_size: 1000,
        max_dead_letters: 100,
        circuit_breaker_threshold: 3,
        circuit_breaker_timeout: Duration::from_secs(30),
        rate_limit_per_agent: 50,
        persist_to_episodic: true,
        broadcast_capacity: 100,
        default_message_ttl: Duration::from_secs(300),
    };

    Arc::new(UnifiedMessageBus::new(cortex, config))
}

// ==============================================================================
// Basic Messaging Tests
// ==============================================================================

#[tokio::test]
async fn test_direct_messaging_between_agents() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    // Create test workspace and sessions
    let workspace_id = cortex.create_workspace("test-workspace").await
        .expect("Failed to create workspace");

    let session1 = cortex.create_session(
        AgentId::from("agent-1".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session 1");

    let session2 = cortex.create_session(
        AgentId::from("agent-2".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session 2");

    // Create messaging adapters
    let mut adapter1 = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-1".to_string()))
        .session_id(session1.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter 1");

    let mut adapter2 = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-2".to_string()))
        .session_id(session2.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter 2");

    // Agent 1 sends message to Agent 2
    adapter1.send_to_agent(
        AgentId::from("agent-2".to_string()),
        Message::Custom {
            message_type: "greeting".to_string(),
            data: serde_json::json!({"text": "Hello, Agent 2!"}),
        },
    ).await.expect("Failed to send message");

    // Agent 2 receives message
    let received = tokio::time::timeout(Duration::from_secs(5), adapter2.receive())
        .await
        .expect("Timeout waiting for message")
        .expect("No message received");

    match received.payload {
        Message::Custom { message_type, data } => {
            assert_eq!(message_type, "greeting");
            assert_eq!(data["text"], "Hello, Agent 2!");
        }
        _ => panic!("Unexpected message type"),
    }

    // Cleanup
    cortex.close_session(&session1, &AgentId::from("agent-1".to_string())).await.ok();
    cortex.close_session(&session2, &AgentId::from("agent-2".to_string())).await.ok();
}

#[tokio::test]
async fn test_pub_sub_messaging() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("test-workspace-pubsub").await
        .expect("Failed to create workspace");

    let session1 = cortex.create_session(
        AgentId::from("publisher".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create publisher session");

    let session2 = cortex.create_session(
        AgentId::from("subscriber".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create subscriber session");

    // Create publisher adapter
    let publisher = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("publisher".to_string()))
        .session_id(session1.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create publisher");

    // Create subscriber adapter
    let subscriber = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("subscriber".to_string()))
        .session_id(session2.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create subscriber");

    // Subscribe to topic
    let mut topic_receiver = subscriber.subscribe("test.notifications".to_string()).await;

    // Give subscription time to register
    sleep(Duration::from_millis(100)).await;

    // Publish message
    let count = publisher.publish_to_topic(
        "test.notifications".to_string(),
        Message::SystemEvent {
            event_type: "test_event".to_string(),
            severity: axon::coordination::EventSeverity::Info,
            data: serde_json::json!({"message": "Test notification"}),
        },
    ).await.expect("Failed to publish");

    assert!(count > 0, "No subscribers received message");

    // Receive broadcast
    let received = tokio::time::timeout(Duration::from_secs(5), topic_receiver.recv())
        .await
        .expect("Timeout waiting for broadcast")
        .expect("No broadcast received");

    match received.payload {
        Message::SystemEvent { event_type, data, .. } => {
            assert_eq!(event_type, "test_event");
            assert_eq!(data["message"], "Test notification");
        }
        _ => panic!("Unexpected message type"),
    }

    // Cleanup
    cortex.close_session(&session1, &AgentId::from("publisher".to_string())).await.ok();
    cortex.close_session(&session2, &AgentId::from("subscriber".to_string())).await.ok();
}

// ==============================================================================
// Coordination Tests
// ==============================================================================

#[tokio::test]
async fn test_distributed_locking() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("test-workspace-locks").await
        .expect("Failed to create workspace");

    let session1 = cortex.create_session(
        AgentId::from("agent-lock-1".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session 1");

    let session2 = cortex.create_session(
        AgentId::from("agent-lock-2".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session 2");

    let adapter1 = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-lock-1".to_string()))
        .session_id(session1.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter 1");

    let adapter2 = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-lock-2".to_string()))
        .session_id(session2.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter 2");

    let entity_id = "test-file.rs".to_string();

    // Agent 1 acquires write lock
    let lock_id = adapter1.acquire_lock(entity_id.clone(), LockType::Write)
        .await
        .expect("Failed to acquire lock");

    // Verify entity is locked
    let is_locked = coordinator.is_entity_locked(&entity_id)
        .await
        .expect("Failed to check lock status");
    assert!(is_locked, "Entity should be locked");

    // Agent 2 tries to acquire lock (should fail or wait)
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        adapter2.acquire_lock(entity_id.clone(), LockType::Write)
    ).await;

    assert!(result.is_err(), "Second lock should timeout while first is held");

    // Agent 1 releases lock
    adapter1.release_lock(entity_id.clone(), lock_id)
        .await
        .expect("Failed to release lock");

    // Now Agent 2 can acquire lock
    let lock_id_2 = adapter2.acquire_lock(entity_id.clone(), LockType::Write)
        .await
        .expect("Failed to acquire lock after release");

    // Cleanup
    adapter2.release_lock(entity_id.clone(), lock_id_2).await.ok();
    cortex.close_session(&session1, &AgentId::from("agent-lock-1".to_string())).await.ok();
    cortex.close_session(&session2, &AgentId::from("agent-lock-2".to_string())).await.ok();
}

#[tokio::test]
async fn test_knowledge_sharing() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("test-workspace-knowledge").await
        .expect("Failed to create workspace");

    let session1 = cortex.create_session(
        AgentId::from("teacher".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create teacher session");

    let session2 = cortex.create_session(
        AgentId::from("learner".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create learner session");

    let teacher = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("teacher".to_string()))
        .session_id(session1.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create teacher");

    let mut learner = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("learner".to_string()))
        .session_id(session2.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create learner");

    // Teacher shares knowledge
    let episode_id = uuid::Uuid::new_v4().to_string();
    teacher.share_knowledge(
        episode_id.clone(),
        "Learned how to refactor efficiently".to_string(),
        vec![
            "Extract common patterns".to_string(),
            "Use type inference".to_string(),
        ],
        vec![AgentId::from("learner".to_string())],
    ).await.expect("Failed to share knowledge");

    // Learner receives knowledge
    let received = tokio::time::timeout(Duration::from_secs(5), learner.receive())
        .await
        .expect("Timeout waiting for knowledge")
        .expect("No knowledge received");

    match received.payload {
        Message::KnowledgeShare { episode_id: recv_id, summary, insights } => {
            assert_eq!(recv_id, episode_id);
            assert_eq!(summary, "Learned how to refactor efficiently");
            assert_eq!(insights.len(), 2);
        }
        _ => panic!("Unexpected message type"),
    }

    // Cleanup
    cortex.close_session(&session1, &AgentId::from("teacher".to_string())).await.ok();
    cortex.close_session(&session2, &AgentId::from("learner".to_string())).await.ok();
}

// ==============================================================================
// Resilience Tests
// ==============================================================================

#[tokio::test]
async fn test_circuit_breaker() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;

    let workspace_id = cortex.create_workspace("test-workspace-circuit").await
        .expect("Failed to create workspace");

    let session = cortex.create_session(
        AgentId::from("sender".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    // Don't register target agent - messages will fail
    let target = AgentId::from("non-existent-agent".to_string());

    // Send multiple messages that will fail
    for i in 0..5 {
        let envelope = axon::coordination::MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            causation_id: None,
            from: AgentId::from("sender".to_string()),
            to: Some(target.clone()),
            topic: None,
            session_id: session.clone(),
            workspace_id: workspace_id.clone(),
            payload: Message::Custom {
                message_type: "test".to_string(),
                data: serde_json::json!({"attempt": i}),
            },
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 5,
            attempt_count: 0,
            max_attempts: 1,
            metadata: std::collections::HashMap::new(),
        };

        let result = bus.send(envelope).await;
        assert!(result.is_err(), "Message should fail to non-existent agent");
    }

    // Check circuit breaker state
    let states = bus.get_circuit_states().await;
    let state = states.get(&target);

    // Circuit should be open after failures
    assert!(state.is_some(), "Circuit breaker should exist for failed agent");
    assert_eq!(
        *state.unwrap(),
        axon::coordination::CircuitState::Open,
        "Circuit should be open after multiple failures"
    );

    // Cleanup
    cortex.close_session(&session, &AgentId::from("sender".to_string())).await.ok();
}

#[tokio::test]
async fn test_message_persistence_and_replay() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("test-workspace-replay").await
        .expect("Failed to create workspace");

    let session = cortex.create_session(
        AgentId::from("agent-replay".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    let mut adapter = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-replay".to_string()))
        .session_id(session.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter");

    // Send some messages
    for i in 0..3 {
        adapter.publish_to_topic(
            "test.replay".to_string(),
            Message::Custom {
                message_type: "replay_test".to_string(),
                data: serde_json::json!({"index": i}),
            },
        ).await.expect("Failed to publish message");
    }

    // Give time for persistence
    sleep(Duration::from_millis(200)).await;

    // Replay from history
    let history = adapter.get_message_history()
        .await
        .expect("Failed to get message history");

    assert!(history.len() >= 3, "Should have at least 3 messages in history");

    // Verify messages
    let replay_messages: Vec<_> = history.iter()
        .filter(|m| matches!(m.payload, Message::Custom { ref message_type, .. } if message_type == "replay_test"))
        .collect();

    assert_eq!(replay_messages.len(), 3, "Should have 3 replay test messages");

    // Cleanup
    cortex.close_session(&session, &AgentId::from("agent-replay".to_string())).await.ok();
}

#[tokio::test]
async fn test_dead_letter_queue() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;

    let workspace_id = cortex.create_workspace("test-workspace-dlq").await
        .expect("Failed to create workspace");

    let session = cortex.create_session(
        AgentId::from("sender-dlq".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    // Send message to non-existent agent (will eventually go to DLQ)
    let mut envelope = axon::coordination::MessageEnvelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        correlation_id: None,
        causation_id: None,
        from: AgentId::from("sender-dlq".to_string()),
        to: Some(AgentId::from("non-existent".to_string())),
        topic: None,
        session_id: session.clone(),
        workspace_id: workspace_id.clone(),
        payload: Message::Custom {
            message_type: "dlq_test".to_string(),
            data: serde_json::json!({"test": "dlq"}),
        },
        timestamp: chrono::Utc::now(),
        expires_at: None,
        priority: 5,
        attempt_count: 0,
        max_attempts: 1, // Will fail immediately
        metadata: std::collections::HashMap::new(),
    };

    envelope.attempt_count = envelope.max_attempts; // Exhaust retries

    let result = bus.send(envelope).await;
    assert!(result.is_err(), "Message should fail");

    // Give time for DLQ processing
    sleep(Duration::from_millis(100)).await;

    // Check dead letter queue
    let dead_letters = bus.get_dead_letters().await;
    assert!(!dead_letters.is_empty(), "Should have messages in DLQ");

    // Cleanup
    cortex.close_session(&session, &AgentId::from("sender-dlq".to_string())).await.ok();
}

// ==============================================================================
// Statistics Tests
// ==============================================================================

#[tokio::test]
async fn test_message_bus_statistics() {
    let cortex = create_test_cortex().await;
    let bus = create_test_bus(cortex.clone()).await;
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("test-workspace-stats").await
        .expect("Failed to create workspace");

    let session = cortex.create_session(
        AgentId::from("agent-stats".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    let adapter = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("agent-stats".to_string()))
        .session_id(session.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter");

    // Reset stats
    bus.reset_stats().await;

    // Send some messages
    for _ in 0..5 {
        adapter.publish_to_topic(
            "stats.test".to_string(),
            Message::HealthPing,
        ).await.ok();
    }

    sleep(Duration::from_millis(100)).await;

    // Get statistics
    let stats = bus.get_stats().await;

    assert!(stats.total_sent >= 5, "Should have sent at least 5 messages");

    // Cleanup
    cortex.close_session(&session, &AgentId::from("agent-stats".to_string())).await.ok();
}
