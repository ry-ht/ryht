# Unified Messaging System - Integration Guide

## Quick Start

### 1. Setup Dependencies

Ensure your `Cargo.toml` includes:

```toml
[dependencies]
axon = { path = "../axon" }
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### 2. Initialize Infrastructure

```rust
use axon::coordination::{
    MessageBusConfig, MessageCoordinator, UnifiedMessageBus,
    AgentMessagingAdapterBuilder,
};
use axon::cortex_bridge::{CortexBridge, CortexConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Cortex
    let cortex_config = CortexConfig {
        base_url: "http://localhost:8081".to_string(),
        api_version: "v3".to_string(),
        request_timeout_secs: 30,
        max_retries: 3,
    };
    let cortex = Arc::new(CortexBridge::new(cortex_config).await?);

    // Configure message bus
    let bus_config = MessageBusConfig {
        persist_to_episodic: true,
        circuit_breaker_threshold: 5,
        rate_limit_per_agent: 100,
        ..Default::default()
    };

    // Create message bus and coordinator
    let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), bus_config));
    let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

    // Create workspace
    let workspace_id = cortex.create_workspace("my-project").await?;

    // Ready to create agents...

    Ok(())
}
```

### 3. Create Agent with Messaging

```rust
use axon::agents::AgentId;

async fn create_agent(
    agent_name: &str,
    workspace_id: WorkspaceId,
    bus: Arc<UnifiedMessageBus>,
    coordinator: Arc<MessageCoordinator>,
    cortex: Arc<CortexBridge>,
) -> Result<AgentMessagingAdapter, Box<dyn std::error::Error>> {
    let agent_id = AgentId::from(agent_name.to_string());

    // Create session
    let session_id = cortex.create_session(
        agent_id.clone(),
        workspace_id.clone(),
        Default::default(),
    ).await?;

    // Create messaging adapter
    let adapter = AgentMessagingAdapterBuilder::new()
        .agent_id(agent_id)
        .session_id(session_id)
        .workspace_id(workspace_id)
        .bus(bus)
        .coordinator(coordinator)
        .cortex(cortex)
        .build()
        .await?;

    Ok(adapter)
}
```

## Common Patterns

### Pattern 1: Direct Agent Communication

```rust
async fn agent_to_agent_communication() -> Result<()> {
    // Agent 1 sends to Agent 2
    agent1.send_to_agent(
        agent2_id,
        Message::Custom {
            message_type: "request".to_string(),
            data: serde_json::json!({
                "action": "analyze_file",
                "path": "src/main.rs"
            }),
        },
    ).await?;

    // Agent 2 receives
    while let Some(envelope) = agent2.receive().await {
        match envelope.payload {
            Message::Custom { message_type, data } => {
                println!("Received {}: {:?}", message_type, data);

                // Process and respond
                agent2.send_to_agent(
                    envelope.from,
                    Message::Custom {
                        message_type: "response".to_string(),
                        data: serde_json::json!({"status": "complete"}),
                    },
                ).await?;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Pattern 2: Request/Response with Timeout

```rust
async fn request_response_pattern(
    coordinator: &MessageCoordinator,
    agent_id: AgentId,
    target_id: AgentId,
) -> Result<serde_json::Value> {
    use std::time::Duration;

    // Create request envelope
    let envelope = MessageEnvelope {
        message_id: uuid::Uuid::new_v4().to_string(),
        correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        from: agent_id,
        to: Some(target_id),
        payload: Message::Custom {
            message_type: "query".to_string(),
            data: serde_json::json!({"query": "get_status"}),
        },
        // ... other fields
    };

    // Send and wait for response (5 second timeout)
    let response = coordinator.request_response(
        envelope,
        Duration::from_secs(5),
    ).await?;

    // Extract response data
    match response.payload {
        Message::Custom { data, .. } => Ok(data),
        _ => Err("Unexpected response type".into()),
    }
}
```

### Pattern 3: Broadcast to Multiple Agents

```rust
async fn broadcast_pattern(agent: &AgentMessagingAdapter) -> Result<()> {
    // Subscribe all agents to a topic
    let mut receiver = agent.subscribe("team.notifications".to_string()).await;

    // Spawn listener task
    tokio::spawn(async move {
        while let Ok(envelope) = receiver.recv().await {
            println!("Broadcast received: {:?}", envelope.payload);
        }
    });

    // Broadcast a message
    agent.publish_to_topic(
        "team.notifications".to_string(),
        Message::SystemEvent {
            event_type: "deployment_ready".to_string(),
            severity: EventSeverity::Info,
            data: serde_json::json!({"version": "1.2.3"}),
        },
    ).await?;

    Ok(())
}
```

### Pattern 4: Coordinated Locking

```rust
async fn coordinated_file_edit(
    agent: &AgentMessagingAdapter,
    file_path: &str,
) -> Result<()> {
    use axon::cortex_bridge::LockType;

    // Acquire exclusive write lock
    let lock_id = agent.acquire_lock(
        file_path.to_string(),
        LockType::Write,
    ).await?;

    // Critical section - modify file
    println!("Lock acquired, modifying {}...", file_path);

    // Simulate work
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Release lock
    agent.release_lock(
        file_path.to_string(),
        lock_id,
    ).await?;

    println!("Lock released");
    Ok(())
}
```

### Pattern 5: Knowledge Sharing

```rust
async fn share_learning(
    agent: &AgentMessagingAdapter,
    episode_id: String,
) -> Result<()> {
    // Share with specific agents
    agent.share_knowledge(
        episode_id.clone(),
        "Discovered efficient refactoring pattern".to_string(),
        vec![
            "Use extract method for duplicated code".to_string(),
            "Apply single responsibility principle".to_string(),
            "Leverage type inference where possible".to_string(),
        ],
        vec![
            AgentId::from("developer-2".to_string()),
            AgentId::from("reviewer".to_string()),
        ],
    ).await?;

    // Or broadcast to all team members
    agent.broadcast_knowledge(
        episode_id,
        "Best practice for error handling".to_string(),
        vec![
            "Always use Result types".to_string(),
            "Provide context with error messages".to_string(),
        ],
        "team.best_practices".to_string(),
    ).await?;

    Ok(())
}
```

### Pattern 6: Task Workflow Coordination

```rust
async fn coordinate_workflow(
    coordinator: &MessageCoordinator,
    orchestrator_id: AgentId,
    session_id: SessionId,
    workspace_id: WorkspaceId,
) -> Result<()> {
    use axon::coordination::WorkflowTask;

    // Define workflow
    let workflow_id = uuid::Uuid::new_v4().to_string();

    let tasks = vec![
        WorkflowTask {
            task_id: "parse-files".to_string(),
            description: "Parse all source files".to_string(),
            assigned_agent: AgentId::from("parser".to_string()),
            context: serde_json::json!({"path": "src/"}),
            priority: 8,
            deadline: None,
            dependencies: vec![],
        },
        WorkflowTask {
            task_id: "analyze-code".to_string(),
            description: "Analyze code structure".to_string(),
            assigned_agent: AgentId::from("analyzer".to_string()),
            context: serde_json::json!({}),
            priority: 7,
            deadline: None,
            dependencies: vec!["parse-files".to_string()],
        },
        WorkflowTask {
            task_id: "generate-report".to_string(),
            description: "Generate analysis report".to_string(),
            assigned_agent: AgentId::from("reporter".to_string()),
            context: serde_json::json!({}),
            priority: 6,
            deadline: None,
            dependencies: vec!["analyze-code".to_string()],
        },
    ];

    // Start workflow
    let execution = coordinator.coordinate_workflow(
        workflow_id,
        tasks,
        orchestrator_id,
        session_id,
        workspace_id,
    ).await?;

    println!("Workflow started: {:?}", execution.workflow_id);
    Ok(())
}
```

### Pattern 7: Health Monitoring

```rust
async fn monitor_agent_health(
    coordinator: &MessageCoordinator,
    monitor_id: AgentId,
    target_id: AgentId,
    session_id: SessionId,
    workspace_id: WorkspaceId,
) -> Result<()> {
    // Ping agent
    match coordinator.ping_agent(
        target_id.clone(),
        monitor_id,
        session_id,
        workspace_id,
    ).await {
        Ok((status, load)) => {
            println!("Agent {} is {}, load: {:.2}", target_id, status, load);
        }
        Err(e) => {
            println!("Agent {} appears down: {}", target_id, e);
            // Handle failure...
        }
    }

    Ok(())
}
```

### Pattern 8: Message Replay for Debugging

```rust
async fn debug_session(agent: &AgentMessagingAdapter) -> Result<()> {
    // Get message history
    let history = agent.get_message_history().await?;

    println!("=== Session Message History ===");
    for (i, envelope) in history.iter().enumerate() {
        println!("{}. {} -> {:?}: {:?}",
                 i + 1,
                 envelope.from,
                 envelope.to,
                 envelope.payload);
    }

    // Replay from episodic memory for deeper analysis
    let episodic_replay = agent.replay_from_memory(100).await?;

    // Find error messages
    let errors: Vec<_> = episodic_replay.iter()
        .filter(|m| matches!(m.payload, Message::SystemEvent {
            severity: EventSeverity::Error, ..
        }))
        .collect();

    println!("\n=== Error Events ===");
    for error in errors {
        println!("Error at {}: {:?}", error.timestamp, error.payload);
    }

    Ok(())
}
```

## Advanced Usage

### Custom Message Types

```rust
// Define custom message
let custom_msg = Message::Custom {
    message_type: "code_analysis".to_string(),
    data: serde_json::json!({
        "analysis_type": "complexity",
        "threshold": 10,
        "files": ["main.rs", "lib.rs"]
    }),
};

// Send with custom metadata
let mut envelope = agent.create_envelope(
    Some(target_id),
    None,
    custom_msg,
    7, // priority
);

envelope.metadata.insert("project".to_string(), "my-app".to_string());
envelope.metadata.insert("version".to_string(), "1.0.0".to_string());
envelope.expires_at = Some(Utc::now() + Duration::from_secs(300));

bus.send(envelope).await?;
```

### Circuit Breaker Monitoring

```rust
async fn monitor_circuit_breakers(bus: &UnifiedMessageBus) {
    let states = bus.get_circuit_states().await;

    for (agent_id, state) in states {
        match state {
            CircuitState::Open => {
                println!("WARNING: Circuit open for agent {}", agent_id);
                // Alert or take action
            }
            CircuitState::HalfOpen => {
                println!("INFO: Circuit testing recovery for {}", agent_id);
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
    }
}
```

### Dead Letter Queue Processing

```rust
async fn process_dead_letters(bus: &UnifiedMessageBus) {
    let dead_letters = bus.get_dead_letters().await;

    for (envelope, reason) in dead_letters {
        println!("Failed message: {} - {}", envelope.message_id, reason);

        // Analyze failure
        if reason.contains("Circuit breaker") {
            // Wait and retry
        } else if reason.contains("Rate limit") {
            // Backoff and retry
        } else {
            // Log for manual investigation
        }
    }

    // Clear processed messages
    bus.clear_dead_letters().await;
}
```

### Statistics Collection

```rust
async fn collect_metrics(bus: &UnifiedMessageBus) {
    let stats = bus.get_stats().await;

    // Log or export to monitoring system
    println!("Message Bus Metrics:");
    println!("  Total sent: {}", stats.total_sent);
    println!("  Delivered: {}", stats.total_delivered);
    println!("  Failed: {}", stats.total_failed);
    println!("  Dead letters: {}", stats.total_dead_letters);
    println!("  Circuit trips: {}", stats.circuit_breaker_trips);
    println!("  Rate limit hits: {}", stats.rate_limit_hits);
    println!("  Avg latency: {:.2}ms", stats.average_latency_ms);

    // Export to Prometheus, etc.
}
```

## Error Handling

### Graceful Degradation

```rust
async fn send_with_fallback(
    agent: &AgentMessagingAdapter,
    target: AgentId,
    message: Message,
) -> Result<()> {
    // Try direct send
    match agent.send_to_agent(target.clone(), message.clone()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("Direct send failed: {}, trying broadcast", e);

            // Fallback to broadcast
            agent.publish_to_topic(
                "fallback.messages".to_string(),
                message,
            ).await?;

            Ok(())
        }
    }
}
```

### Timeout Handling

```rust
async fn send_with_timeout(
    agent: &AgentMessagingAdapter,
    target: AgentId,
    message: Message,
) -> Result<()> {
    use tokio::time::timeout;

    match timeout(
        Duration::from_secs(5),
        agent.send_to_agent(target, message.clone())
    ).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            println!("Send timeout, message queued for retry");
            // Queue for later retry
            Ok(())
        }
    }
}
```

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_communication() {
        let cortex = create_test_cortex().await;
        let bus = create_test_bus(cortex.clone()).await;
        let coordinator = Arc::new(MessageCoordinator::new(
            bus.clone(),
            cortex.clone()
        ));

        let workspace_id = cortex.create_workspace("test").await.unwrap();

        let agent1 = create_agent(
            "test-agent-1",
            workspace_id.clone(),
            bus.clone(),
            coordinator.clone(),
            cortex.clone(),
        ).await.unwrap();

        let mut agent2 = create_agent(
            "test-agent-2",
            workspace_id,
            bus,
            coordinator,
            cortex,
        ).await.unwrap();

        // Send message
        agent1.send_to_agent(
            AgentId::from("test-agent-2".to_string()),
            Message::HealthPing,
        ).await.unwrap();

        // Receive message
        let received = tokio::time::timeout(
            Duration::from_secs(1),
            agent2.receive()
        ).await.unwrap().unwrap();

        assert!(matches!(received.payload, Message::HealthPing));
    }
}
```

## Best Practices

1. **Always use sessions**: Create a Cortex session for each agent
2. **Set appropriate priorities**: Use 0-10 scale, reserve 9-10 for urgent
3. **Enable persistence**: Keep `persist_to_episodic: true` for learning
4. **Handle timeouts**: Always use timeouts for request/response
5. **Monitor circuit breakers**: Check states periodically
6. **Process dead letters**: Review and handle failed messages
7. **Use correlation IDs**: Track request/response chains
8. **Leverage patterns**: Apply learned patterns when available
9. **Share knowledge**: Broadcast important discoveries
10. **Clean up**: Close sessions when agents complete

## Troubleshooting

### Messages Not Being Delivered

```rust
// Check if agent is registered
let channels = bus.get_circuit_states().await;
if !channels.contains_key(&agent_id) {
    println!("Agent not registered!");
}

// Check circuit breaker
let state = channels.get(&agent_id);
if matches!(state, Some(CircuitState::Open)) {
    println!("Circuit breaker is open!");
}
```

### High Message Latency

```rust
// Check stats
let stats = bus.get_stats().await;
if stats.average_latency_ms > 100.0 {
    println!("High latency detected");
    // Consider:
    // - Reducing persistence overhead
    // - Increasing rate limits
    // - Scaling message bus
}
```

### Circuit Breakers Tripping

```rust
// Adjust thresholds
let config = MessageBusConfig {
    circuit_breaker_threshold: 10, // Increase threshold
    circuit_breaker_timeout: Duration::from_secs(120), // Longer timeout
    ..Default::default()
};
```

## Migration Checklist

- [ ] Initialize Cortex bridge
- [ ] Create unified message bus
- [ ] Create message coordinator
- [ ] Update agent initialization to use adapters
- [ ] Replace old bus.send() with adapter.send_to_agent()
- [ ] Replace old bus.publish() with adapter.publish_to_topic()
- [ ] Add lock coordination where needed
- [ ] Enable episodic persistence
- [ ] Add error handling and retries
- [ ] Update tests
- [ ] Monitor statistics
- [ ] Remove old message bus

## Support

For issues or questions:
- Check documentation: `/docs/UNIFIED_MESSAGING_ARCHITECTURE.md`
- Review tests: `/tests/unified_messaging_integration_tests.rs`
- See examples: This guide's patterns section
