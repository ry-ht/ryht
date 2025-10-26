# Unified Messaging Architecture - Cortex-Integrated Multi-Agent Communication

## Overview

The Unified Messaging Architecture provides a comprehensive, persistent, and intelligent messaging system for multi-agent coordination in Axon. It deeply integrates with Cortex's capabilities to enable robust communication, learning from interaction patterns, and resilient distributed coordination.

## Architecture Components

### 1. Unified Message Bus (`UnifiedMessageBus`)

The core messaging infrastructure that provides:

- **Direct Agent-to-Agent Messaging**: Point-to-point communication with guaranteed delivery
- **Pub/Sub Broadcasting**: Topic-based message distribution
- **Message Persistence**: All messages stored in Cortex episodic memory
- **Message History & Replay**: Full audit trail and ability to replay conversations
- **Circuit Breakers**: Automatic failure detection and recovery
- **Rate Limiting**: Per-agent message rate control
- **Dead Letter Queue**: Failed message capture and analysis

#### Key Features

```rust
// Create unified message bus
let bus = UnifiedMessageBus::new(cortex_bridge, config);

// Register agent
let receiver = bus.register_agent(agent_id, session_id).await?;

// Send direct message
bus.send(envelope).await?;

// Publish to topic
bus.publish(envelope).await?;

// Replay session messages
let history = bus.replay_session(&session_id).await?;
```

### 2. Message Coordinator (`MessageCoordinator`)

High-level coordination patterns built on the message bus:

- **Request/Response Pattern**: Synchronous communication with timeout
- **Distributed Locking**: Cortex-backed coordination locks
- **Workflow Coordination**: Multi-agent task orchestration
- **Knowledge Sharing**: Episodic memory distribution
- **Health Monitoring**: Agent health checks and status

#### Coordination Patterns

```rust
// Request with response
let response = coordinator.request_response(request, timeout).await?;

// Acquire coordinated lock
let lock_id = coordinator.acquire_coordinated_lock(
    entity_id,
    LockType::Write,
    agent_id,
    session_id,
    workspace_id
).await?;

// Share knowledge
coordinator.share_knowledge(
    episode_id,
    summary,
    insights,
    source_agent,
    target_agents,
    session_id,
    workspace_id
).await?;
```

### 3. Agent Messaging Adapter (`AgentMessagingAdapter`)

Simplified interface for agents to interact with the messaging system:

```rust
// Create adapter for an agent
let adapter = AgentMessagingAdapterBuilder::new()
    .agent_id(agent_id)
    .session_id(session_id)
    .workspace_id(workspace_id)
    .bus(bus)
    .coordinator(coordinator)
    .cortex(cortex)
    .build()
    .await?;

// Simple API for agents
adapter.send_to_agent(target, message).await?;
adapter.publish_to_topic(topic, message).await?;
adapter.acquire_lock(entity_id, LockType::Write).await?;
adapter.share_knowledge(episode_id, summary, insights, targets).await?;
```

## Message Types

### Core Messages

1. **Task Messages**
   - `TaskAssignment`: Assign work to an agent
   - `TaskProgress`: Report progress updates
   - `TaskComplete`: Signal task completion
   - `TaskFailed`: Report task failure

2. **Coordination Messages**
   - `AssistanceRequest`: Request help from other agents
   - `AssistanceResponse`: Respond to assistance request
   - `LockRequest`: Request coordination lock
   - `LockGranted`: Lock acquisition confirmed
   - `LockDenied`: Lock denied

3. **Knowledge Messages**
   - `KnowledgeShare`: Share episodic memories
   - `PatternDiscovered`: Notify pattern learning

4. **System Messages**
   - `SystemEvent`: System-wide notifications
   - `HealthPing`/`HealthPong`: Health checks

5. **Custom Messages**
   - `Custom`: Flexible user-defined messages

### Message Envelope

All messages are wrapped in a comprehensive envelope:

```rust
pub struct MessageEnvelope {
    pub message_id: String,           // Unique identifier
    pub correlation_id: Option<String>, // For request/response
    pub causation_id: Option<String>,  // Event chain tracking
    pub from: AgentId,                 // Source agent
    pub to: Option<AgentId>,           // Target (None for broadcast)
    pub topic: Option<String>,         // Pub/sub topic
    pub session_id: SessionId,         // Cortex session context
    pub workspace_id: WorkspaceId,     // Workspace context
    pub payload: Message,              // Actual message
    pub timestamp: DateTime<Utc>,      // Creation time
    pub expires_at: Option<DateTime<Utc>>, // TTL
    pub priority: u8,                  // 0-10 priority
    pub attempt_count: u32,            // Retry tracking
    pub max_attempts: u32,             // Max retries
    pub metadata: HashMap<String, String>, // Custom metadata
}
```

## Cortex Integration

### Session Isolation

Each agent operates within a Cortex session, providing:
- **Isolated Message Context**: Messages scoped to sessions
- **Transactional Messaging**: Messages tied to session lifecycle
- **Session-Based Replay**: Replay all messages from a session

### Distributed Locking

Uses Cortex's lock system for coordination:
- **Entity Locks**: Lock files, resources, or abstract entities
- **Lock Types**: Read (shared) or Write (exclusive)
- **Deadlock Detection**: Automatic cycle detection
- **Automatic Expiration**: Locks expire if not renewed

### Episodic Memory Persistence

All messages are stored as episodes in Cortex:
- **Full Message History**: Complete communication audit trail
- **Semantic Search**: Query past conversations by meaning
- **Pattern Learning**: Extract communication patterns
- **Replay & Debug**: Reconstruct past interactions

### Event System Integration

Broadcasts leverage Cortex's event infrastructure:
- **Topic-Based Distribution**: Scalable pub/sub
- **Event Correlation**: Track event chains
- **Event Replay**: Reconstruct event sequences

## Resilience Patterns

### Circuit Breaker

Protects against cascading failures:

```rust
// Automatic circuit breaking
if circuit_breaker.should_attempt() {
    bus.send(message).await?;
    circuit_breaker.record_success();
} else {
    // Circuit is open, reject request
    return Err(CircuitOpen);
}
```

**States:**
- `Closed`: Normal operation
- `Open`: Failing, reject requests
- `HalfOpen`: Testing recovery

**Configuration:**
- `circuit_breaker_threshold`: Failures before opening
- `circuit_breaker_timeout`: Time before retry

### Dead Letter Queue

Captures failed messages for analysis:

```rust
// Messages that exceed retry attempts go to DLQ
let dead_letters = bus.get_dead_letters().await;

for (message, reason) in dead_letters {
    println!("Failed message: {} - {}", message.message_id, reason);
}
```

### Rate Limiting

Prevents message flooding:

```rust
// Per-agent rate limits
config.rate_limit_per_agent = 100; // messages per second
```

### Automatic Retry

Messages retry on transient failures:

```rust
envelope.max_attempts = 3; // Retry up to 3 times
```

## Pattern Learning

The system learns from communication patterns:

### Pattern Extraction

```rust
// Extract patterns from episodic memory
let patterns = cortex.extract_patterns(&workspace_id, min_occurrences).await?;

for pattern in patterns {
    println!("Discovered: {} (confidence: {})",
             pattern.name, pattern.confidence);
}
```

### Pattern Types

- **Workflow Patterns**: Common task coordination sequences
- **Error Handling Patterns**: Successful recovery strategies
- **Performance Patterns**: Efficient communication flows
- **Collaboration Patterns**: Multi-agent coordination strategies

### Pattern Application

```rust
// Apply learned pattern
let application = cortex.apply_pattern(
    &pattern_id,
    context,
).await?;

// Provide feedback
cortex.update_pattern_stats(
    &pattern_id,
    success,
    improvements,
).await?;
```

## Usage Examples

### Example 1: Simple Agent Communication

```rust
// Create messaging infrastructure
let cortex = CortexBridge::new(config).await?;
let bus = Arc::new(UnifiedMessageBus::new(cortex.clone(), bus_config));
let coordinator = Arc::new(MessageCoordinator::new(bus.clone(), cortex.clone()));

// Create workspace and sessions
let workspace_id = cortex.create_workspace("my-project").await?;
let session1 = cortex.create_session(agent1_id, workspace_id, scope).await?;
let session2 = cortex.create_session(agent2_id, workspace_id, scope).await?;

// Create adapters
let mut agent1 = AgentMessagingAdapterBuilder::new()
    .agent_id(agent1_id)
    .session_id(session1)
    .workspace_id(workspace_id)
    .bus(bus.clone())
    .coordinator(coordinator.clone())
    .cortex(cortex.clone())
    .build()
    .await?;

let mut agent2 = AgentMessagingAdapterBuilder::new()
    .agent_id(agent2_id)
    .session_id(session2)
    .workspace_id(workspace_id)
    .bus(bus)
    .coordinator(coordinator)
    .cortex(cortex)
    .build()
    .await?;

// Agent 1 sends message to Agent 2
agent1.send_to_agent(
    agent2_id,
    Message::Custom {
        message_type: "hello".to_string(),
        data: serde_json::json!({"greeting": "Hi!"}),
    },
).await?;

// Agent 2 receives message
if let Some(envelope) = agent2.receive().await {
    println!("Received: {:?}", envelope.payload);
}
```

### Example 2: Coordinated Workflow

```rust
// Coordinator assigns tasks
let workflow = coordinator.coordinate_workflow(
    workflow_id,
    vec![
        WorkflowTask {
            task_id: "task-1".to_string(),
            description: "Parse files".to_string(),
            assigned_agent: developer_id,
            context: serde_json::json!({}),
            priority: 8,
            deadline: None,
            dependencies: vec![],
        },
        WorkflowTask {
            task_id: "task-2".to_string(),
            description: "Run tests".to_string(),
            assigned_agent: tester_id,
            context: serde_json::json!({}),
            priority: 7,
            deadline: None,
            dependencies: vec!["task-1".to_string()],
        },
    ],
    orchestrator_id,
    session_id,
    workspace_id,
).await?;

// Agents report progress
developer.update_task_progress(
    "task-1".to_string(),
    0.5,
    "parsing".to_string(),
    serde_json::json!({"files_processed": 10}),
    orchestrator_id,
).await?;
```

### Example 3: Knowledge Sharing

```rust
// Agent discovers something useful
let episode_id = /* from completed task */;

// Share with specific agents
agent.share_knowledge(
    episode_id,
    "Efficient refactoring technique".to_string(),
    vec![
        "Use extract method pattern".to_string(),
        "Leverage type inference".to_string(),
    ],
    vec![other_developer_id, reviewer_id],
).await?;

// Or broadcast to all team members
agent.broadcast_knowledge(
    episode_id,
    "New optimization discovered".to_string(),
    vec!["Cache results", "Use lazy evaluation"],
    "team.learnings".to_string(),
).await?;
```

### Example 4: Distributed Locking

```rust
// Agent acquires lock on a file
let lock_id = agent.acquire_lock(
    "src/main.rs".to_string(),
    LockType::Write,
).await?;

// Perform exclusive operation
// ...

// Release lock
agent.release_lock(
    "src/main.rs".to_string(),
    lock_id,
).await?;
```

### Example 5: Message Replay for Debugging

```rust
// Get message history for a session
let history = agent.get_message_history().await?;

println!("Session had {} messages", history.len());

for message in history {
    println!("{}: {:?} -> {:?}",
             message.timestamp,
             message.from,
             message.payload);
}

// Replay from episodic memory for deeper analysis
let episodic_replay = agent.replay_from_memory(100).await?;

// Analyze patterns
let error_messages: Vec<_> = episodic_replay.iter()
    .filter(|m| matches!(m.payload, Message::SystemEvent {
        severity: EventSeverity::Error, ..
    }))
    .collect();

println!("Found {} error events in replay", error_messages.len());
```

## Configuration

### Message Bus Configuration

```rust
pub struct MessageBusConfig {
    // Message history per session
    pub max_history_size: usize,          // Default: 10000

    // Dead letter queue size
    pub max_dead_letters: usize,          // Default: 1000

    // Circuit breaker settings
    pub circuit_breaker_threshold: u32,   // Default: 5 failures
    pub circuit_breaker_timeout: Duration, // Default: 60 seconds

    // Rate limiting
    pub rate_limit_per_agent: usize,      // Default: 100 msg/sec

    // Episodic memory integration
    pub persist_to_episodic: bool,        // Default: true

    // Pub/sub settings
    pub broadcast_capacity: usize,        // Default: 1000

    // Message TTL
    pub default_message_ttl: Duration,    // Default: 3600 seconds
}
```

## Performance Considerations

### Message Throughput

- Direct messaging: ~10,000 msg/sec per agent
- Broadcast: ~50,000 msg/sec across all subscribers
- Persistence overhead: ~10% latency increase

### Memory Usage

- Message history: ~1KB per message
- Circuit breakers: ~100 bytes per agent
- Rate limiters: ~50 bytes per agent

### Scalability

- Horizontal: Multiple bus instances per workspace
- Vertical: Thread-safe, lock-free where possible
- Persistence: Async writes to Cortex

## Monitoring & Observability

### Statistics

```rust
let stats = bus.get_stats().await;

println!("Total sent: {}", stats.total_sent);
println!("Total delivered: {}", stats.total_delivered);
println!("Failed: {}", stats.total_failed);
println!("Dead letters: {}", stats.total_dead_letters);
println!("Circuit breaker trips: {}", stats.circuit_breaker_trips);
println!("Average latency: {:.2}ms", stats.average_latency_ms);
```

### Circuit Breaker States

```rust
let states = bus.get_circuit_states().await;

for (agent_id, state) in states {
    println!("Agent {}: {:?}", agent_id, state);
}
```

## Testing

Comprehensive integration tests cover:

1. **Basic Messaging**: Direct and broadcast messaging
2. **Coordination**: Distributed locking, workflows
3. **Resilience**: Circuit breakers, retries, DLQ
4. **Persistence**: Message history, replay
5. **Pattern Learning**: Pattern extraction and application

Run tests:

```bash
cargo test --test unified_messaging_integration_tests
cargo test --test message_pattern_learning_tests
```

## Migration Guide

### From Old Message Bus

**Before:**
```rust
let bus = MessageBus::new();
bus.register_agent(agent_id).await?;
bus.send(target, message).await?;
```

**After:**
```rust
let cortex = CortexBridge::new(config).await?;
let bus = UnifiedMessageBus::new(cortex, config);
let adapter = AgentMessagingAdapterBuilder::new()
    .agent_id(agent_id)
    .session_id(session_id)
    .workspace_id(workspace_id)
    .bus(bus)
    .coordinator(coordinator)
    .cortex(cortex)
    .build()
    .await?;

adapter.send_to_agent(target, message).await?;
```

## Future Enhancements

1. **Priority Queues**: Priority-based message ordering
2. **Message Batching**: Automatic batching of similar messages
3. **Compression**: Message compression for large payloads
4. **Encryption**: End-to-end message encryption
5. **Multi-Workspace**: Cross-workspace messaging
6. **WebSocket Support**: Real-time message streaming
7. **GraphQL Integration**: Query message history via GraphQL

## Conclusion

The Unified Messaging Architecture provides a robust, intelligent, and scalable foundation for multi-agent coordination in Axon. By deeply integrating with Cortex, it enables persistent communication, pattern learning, and resilient distributed systems that continuously improve through experience.
