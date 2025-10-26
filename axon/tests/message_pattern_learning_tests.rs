//! Tests for message pattern learning from episodic memory
//!
//! These tests verify that the system can learn from communication patterns
//! and extract reusable coordination strategies.

use axon::coordination::{
    Message, MessageBusConfig, MessageCoordinator, UnifiedMessageBus,
    AgentMessagingAdapterBuilder,
};
use axon::cortex_bridge::{CortexBridge, CortexConfig, Pattern, PatternType};
use axon::agents::AgentId;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

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

#[tokio::test]
async fn test_communication_pattern_extraction() {
    let cortex = create_test_cortex().await;

    let config = MessageBusConfig {
        persist_to_episodic: true,
        ..Default::default()
    };

    let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), config));
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("pattern-learning-workspace").await
        .expect("Failed to create workspace");

    // Create a common communication pattern: Request -> Progress -> Complete
    for iteration in 0..5 {
        let session = cortex.create_session(
            AgentId::from(format!("orchestrator-{}", iteration)),
            workspace_id.clone(),
            Default::default(),
        ).await.expect("Failed to create session");

        let adapter = AgentMessagingAdapterBuilder::new()
            .agent_id(AgentId::from(format!("orchestrator-{}", iteration)))
            .session_id(session.clone())
            .workspace_id(workspace_id.clone())
            .bus(bus.clone())
            .coordinator(coordinator.clone())
            .cortex(cortex.clone())
            .build()
            .await
            .expect("Failed to create adapter");

        // Simulate task workflow pattern
        let task_id = format!("task-{}", iteration);

        // 1. Task Assignment
        adapter.send_to_agent(
            AgentId::from("worker".to_string()),
            Message::TaskAssignment {
                task_id: task_id.clone(),
                task_description: "Process data".to_string(),
                context: serde_json::json!({"iteration": iteration}),
            },
        ).await.ok();

        sleep(Duration::from_millis(50)).await;

        // 2. Progress Updates
        for progress in &[0.25, 0.5, 0.75] {
            adapter.update_task_progress(
                task_id.clone(),
                *progress,
                "processing".to_string(),
                serde_json::json!({}),
                AgentId::from(format!("orchestrator-{}", iteration)),
            ).await.ok();

            sleep(Duration::from_millis(30)).await;
        }

        // 3. Completion
        adapter.complete_task(
            task_id.clone(),
            serde_json::json!({"result": "success"}),
            true,
            vec![],
            AgentId::from(format!("orchestrator-{}", iteration)),
        ).await.ok();

        sleep(Duration::from_millis(50)).await;

        cortex.close_session(&session, &AgentId::from(format!("orchestrator-{}", iteration)))
            .await.ok();
    }

    // Wait for episodic memory to persist
    sleep(Duration::from_secs(2)).await;

    // Extract patterns from episodes
    let patterns = cortex.extract_patterns(&workspace_id, 3)
        .await
        .expect("Failed to extract patterns");

    // Should have detected the task workflow pattern
    assert!(!patterns.is_empty(), "Should have extracted at least one pattern");

    // Verify pattern characteristics
    let task_patterns: Vec<_> = patterns.iter()
        .filter(|p| p.pattern_type == PatternType::Workflow)
        .collect();

    assert!(!task_patterns.is_empty(), "Should have detected workflow pattern");

    println!("Extracted {} communication patterns", patterns.len());
    for pattern in &patterns {
        println!("Pattern: {} (type: {:?}, confidence: {})",
                 pattern.name, pattern.pattern_type, pattern.confidence);
    }
}

#[tokio::test]
async fn test_collaborative_learning_across_agents() {
    let cortex = create_test_cortex().await;

    let config = MessageBusConfig {
        persist_to_episodic: true,
        ..Default::default()
    };

    let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), config));
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("collaborative-learning").await
        .expect("Failed to create workspace");

    // Multiple agents working on similar tasks and sharing knowledge
    let agent_ids = vec!["agent-alpha", "agent-beta", "agent-gamma"];

    for agent_name in &agent_ids {
        let session = cortex.create_session(
            AgentId::from(agent_name.to_string()),
            workspace_id.clone(),
            Default::default(),
        ).await.expect("Failed to create session");

        let adapter = AgentMessagingAdapterBuilder::new()
            .agent_id(AgentId::from(agent_name.to_string()))
            .session_id(session.clone())
            .workspace_id(workspace_id.clone())
            .bus(bus.clone())
            .coordinator(coordinator.clone())
            .cortex(cortex.clone())
            .build()
            .await
            .expect("Failed to create adapter");

        // Each agent discovers insights and shares them
        let episode_id = uuid::Uuid::new_v4().to_string();

        adapter.broadcast_knowledge(
            episode_id,
            format!("{} discovered optimization technique", agent_name),
            vec![
                "Cache frequently accessed data".to_string(),
                "Use lazy evaluation".to_string(),
            ],
            "team.learnings".to_string(),
        ).await.ok();

        sleep(Duration::from_millis(100)).await;

        cortex.close_session(&session, &AgentId::from(agent_name.to_string()))
            .await.ok();
    }

    sleep(Duration::from_secs(1)).await;

    // Get collaborative insights
    let insights = cortex.get_collaborative_insights(&workspace_id)
        .await
        .expect("Failed to get collaborative insights");

    assert!(!insights.is_empty(), "Should have collaborative insights from multiple agents");

    println!("Found {} collaborative insights", insights.len());
    for insight in &insights {
        println!("Insight from {} agents: {}",
                 insight.contributing_agents.len(), insight.summary);
    }
}

#[tokio::test]
async fn test_pattern_application_and_feedback() {
    let cortex = create_test_cortex().await;

    let workspace_id = cortex.create_workspace("pattern-application").await
        .expect("Failed to create workspace");

    // Create a pattern for testing
    let pattern = Pattern {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Error Recovery Pattern".to_string(),
        pattern_type: PatternType::ErrorHandling,
        description: "Retry with exponential backoff".to_string(),
        context: serde_json::json!({
            "applicable_to": ["network_calls", "database_operations"],
            "conditions": ["transient_error", "recoverable_failure"]
        }),
        solution: serde_json::json!({
            "strategy": "exponential_backoff",
            "max_retries": 3,
            "base_delay_ms": 100
        }),
        examples: vec![],
        prerequisites: vec![],
        success_rate: 0.85,
        usage_count: 10,
        confidence: 0.9,
        tags: vec!["resilience".to_string(), "retry".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: 1,
    };

    let pattern_id = cortex.store_pattern(pattern)
        .await
        .expect("Failed to store pattern");

    // Apply the pattern
    let application = cortex.apply_pattern(
        &pattern_id,
        serde_json::json!({
            "operation": "fetch_data",
            "error_type": "timeout"
        }),
    ).await.expect("Failed to apply pattern");

    assert_eq!(application.pattern_id, pattern_id);
    assert!(application.applied_successfully, "Pattern should apply successfully");

    // Update pattern statistics based on outcome
    cortex.update_pattern_stats(
        &pattern_id,
        true, // success
        serde_json::json!({"improved_latency": true}),
    ).await.expect("Failed to update pattern stats");

    // Retrieve updated pattern
    let updated_pattern = cortex.get_pattern(&pattern_id)
        .await
        .expect("Failed to get updated pattern");

    assert!(updated_pattern.usage_count > 10, "Usage count should have increased");

    println!("Pattern '{}' now has success rate: {:.2}%",
             updated_pattern.name, updated_pattern.success_rate * 100.0);
}

#[tokio::test]
async fn test_message_flow_optimization() {
    let cortex = create_test_cortex().await;

    let config = MessageBusConfig {
        persist_to_episodic: true,
        ..Default::default()
    };

    let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), config));
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("flow-optimization").await
        .expect("Failed to create workspace");

    // Create inefficient message flow (too many messages)
    let session1 = cortex.create_session(
        AgentId::from("chatty-agent".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    let adapter = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("chatty-agent".to_string()))
        .session_id(session1.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter");

    // Send many small messages (inefficient)
    for i in 0..20 {
        adapter.publish_to_topic(
            "updates".to_string(),
            Message::Custom {
                message_type: "small_update".to_string(),
                data: serde_json::json!({"value": i}),
            },
        ).await.ok();

        sleep(Duration::from_millis(10)).await;
    }

    cortex.close_session(&session1, &AgentId::from("chatty-agent".to_string()))
        .await.ok();

    sleep(Duration::from_millis(500)).await;

    // Get message statistics
    let stats = bus.get_stats().await;

    println!("Message bus stats:");
    println!("  Total sent: {}", stats.total_sent);
    println!("  Total delivered: {}", stats.total_delivered);
    println!("  Average latency: {:.2}ms", stats.average_latency_ms);

    // System should learn to batch messages or reduce frequency
    // This would be implemented by analyzing the pattern and suggesting optimizations
    let patterns = cortex.search_patterns(
        "message frequency optimization",
        Some(PatternType::Performance),
        5,
    ).await.expect("Failed to search patterns");

    // In a real implementation, the system would detect the chatty pattern
    // and suggest batching or aggregation
    println!("Found {} optimization patterns", patterns.len());
}

#[tokio::test]
async fn test_episodic_memory_replay_for_debugging() {
    let cortex = create_test_cortex().await;

    let config = MessageBusConfig {
        persist_to_episodic: true,
        ..Default::default()
    };

    let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), config));
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    let workspace_id = cortex.create_workspace("replay-debugging").await
        .expect("Failed to create workspace");

    let session = cortex.create_session(
        AgentId::from("debugger".to_string()),
        workspace_id.clone(),
        Default::default(),
    ).await.expect("Failed to create session");

    let adapter = AgentMessagingAdapterBuilder::new()
        .agent_id(AgentId::from("debugger".to_string()))
        .session_id(session.clone())
        .workspace_id(workspace_id.clone())
        .bus(bus.clone())
        .coordinator(coordinator.clone())
        .cortex(cortex.clone())
        .build()
        .await
        .expect("Failed to create adapter");

    // Simulate a sequence of events that led to an error
    let events = vec![
        ("start", "Task started"),
        ("progress", "Processing data"),
        ("warning", "High memory usage detected"),
        ("error", "Out of memory"),
    ];

    for (event_type, message) in &events {
        adapter.publish_to_topic(
            "system.events".to_string(),
            Message::SystemEvent {
                event_type: event_type.to_string(),
                severity: if *event_type == "error" {
                    axon::coordination::EventSeverity::Error
                } else if *event_type == "warning" {
                    axon::coordination::EventSeverity::Warning
                } else {
                    axon::coordination::EventSeverity::Info
                },
                data: serde_json::json!({"message": message}),
            },
        ).await.ok();

        sleep(Duration::from_millis(100)).await;
    }

    sleep(Duration::from_millis(500)).await;

    // Replay messages to understand what happened
    let history = adapter.get_message_history()
        .await
        .expect("Failed to get message history");

    assert!(!history.is_empty(), "Should have message history");

    // Replay from episodic memory for deeper analysis
    let episodic_replay = adapter.replay_from_memory(100)
        .await
        .expect("Failed to replay from episodic memory");

    println!("Replayed {} messages from episodic memory", episodic_replay.len());

    // Analyze the sequence to find root cause
    let error_messages: Vec<_> = history.iter()
        .filter(|m| matches!(m.payload, Message::SystemEvent { severity: axon::coordination::EventSeverity::Error, .. }))
        .collect();

    assert!(!error_messages.is_empty(), "Should have captured error event");

    println!("Debug replay found {} error events", error_messages.len());

    // System could learn from this failure pattern
    // and store it as a "what-went-wrong" pattern

    cortex.close_session(&session, &AgentId::from("debugger".to_string()))
        .await.ok();
}
