# Cortex Bridge Module

The Cortex Bridge provides seamless integration between Axon's multi-agent orchestration system and Cortex's persistent memory and data layer.

## Overview

This module implements the complete API specified in `/docs/spec/multi-agent-system/10-cortex-integration.md`. It provides:

- **Session Management**: Isolated workspaces for concurrent agent execution
- **Episodic Memory**: Shared learning across all agents
- **Semantic Search**: Context-aware code discovery
- **Pattern Learning**: Automated extraction and application of successful patterns
- **Distributed Locks**: Safe coordination between multiple agents
- **Knowledge Graph**: Rich code relationship queries

## Architecture

```
┌─────────────────────────────────────────────┐
│            CortexBridge (mod.rs)            │
│  High-level API and coordination layer     │
├─────────────────────────────────────────────┤
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ Session  │  │  Memory  │  │  Search  │ │
│  │ Manager  │  │ Manager  │  │ Manager  │ │
│  └──────────┘  └──────────┘  └──────────┘ │
│                                             │
│  ┌──────────┐  ┌──────────┐               │
│  │   Lock   │  │  Client  │               │
│  │ Manager  │  │  (HTTP)  │               │
│  └──────────┘  └──────────┘               │
└─────────────────────────────────────────────┘
                    │
                    ▼
         Cortex REST API (v3)
```

## Module Structure

### Core Modules

- **`mod.rs`**: Main CortexBridge structure with high-level API
- **`client.rs`**: HTTP client with retry logic and error handling
- **`models.rs`**: Data structures matching Cortex API schema

### Functional Modules

- **`session.rs`**: Session lifecycle management
- **`memory.rs`**: Episodic memory and pattern storage
- **`search.rs`**: Semantic code search and knowledge graph queries
- **`locks.rs`**: Distributed locking for agent coordination

## Usage Examples

### Basic Connection

```rust
use axon::cortex_bridge::{CortexBridge, CortexConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create bridge with default config
    let config = CortexConfig::default();
    let bridge = CortexBridge::new(config).await?;

    // Verify connectivity
    let health = bridge.health_check().await?;
    println!("Connected to Cortex {}", health.version);

    Ok(())
}
```

### Agent Session Workflow

```rust
use axon::cortex_bridge::*;

async fn agent_task(bridge: &CortexBridge) -> Result<()> {
    let agent_id = AgentId::from("developer-001".to_string());
    let workspace_id = WorkspaceId::from("project-x".to_string());

    // 1. Create isolated session
    let session_id = bridge.create_session(
        agent_id.clone(),
        workspace_id.clone(),
        SessionScope {
            paths: vec!["src/".to_string()],
            read_only_paths: vec!["tests/".to_string()],
        },
    ).await?;

    // 2. Work with files
    let content = bridge.read_file(&session_id, "src/main.rs").await?;
    let modified = format!("// Modified\n{}", content);
    bridge.write_file(&session_id, "src/main.rs", &modified).await?;

    // 3. Merge changes back
    let report = bridge.merge_session(&session_id, MergeStrategy::Auto).await?;
    println!("Merged {} changes", report.changes_merged);

    // 4. Cleanup
    bridge.close_session(&session_id, &agent_id).await?;

    Ok(())
}
```

### Episodic Learning

```rust
use axon::cortex_bridge::*;
use chrono::Utc;

async fn store_learning(bridge: &CortexBridge) -> Result<()> {
    // Create episode from agent's work
    let episode = Episode {
        id: uuid::Uuid::new_v4().to_string(),
        episode_type: EpisodeType::Feature,
        task_description: "Add authentication to API".to_string(),
        agent_id: "developer-001".to_string(),
        session_id: Some("session-123".to_string()),
        workspace_id: "project-x".to_string(),
        entities_created: vec!["auth_middleware".to_string()],
        entities_modified: vec!["main.rs".to_string()],
        entities_deleted: vec![],
        files_touched: vec!["src/auth.rs".to_string()],
        queries_made: vec!["authentication patterns".to_string()],
        tools_used: vec![
            ToolUsage {
                tool_name: "semantic_search".to_string(),
                invocations: 3,
                success_rate: 1.0,
            }
        ],
        solution_summary: "Implemented JWT-based authentication".to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({"tests_passed": 15}),
        errors_encountered: vec![],
        lessons_learned: vec!["Use middleware pattern for auth".to_string()],
        duration_seconds: 1200,
        tokens_used: TokenUsage { input: 5000, output: 2000, total: 7000 },
        embedding: vec![],
        created_at: Utc::now(),
        completed_at: Some(Utc::now()),
    };

    let episode_id = bridge.store_episode(episode).await?;
    println!("Stored episode: {}", episode_id);

    Ok(())
}
```

### Semantic Search

```rust
use axon::cortex_bridge::*;

async fn find_similar_code(bridge: &CortexBridge) -> Result<()> {
    let workspace_id = WorkspaceId::from("project-x".to_string());

    // Search for authentication-related code
    let results = bridge.semantic_search(
        "authentication middleware",
        &workspace_id,
        SearchFilters {
            types: vec!["function".to_string()],
            languages: vec!["rust".to_string()],
            visibility: Some("public".to_string()),
            min_relevance: 0.7,
        },
    ).await?;

    for result in results {
        println!("Found: {} (relevance: {:.2})",
            result.qualified_name, result.relevance_score);
    }

    Ok(())
}
```

### Multi-Agent Coordination

```rust
use axon::cortex_bridge::*;

async fn coordinated_work(bridge: &CortexBridge) -> Result<()> {
    let agent_id = AgentId::from("developer-001".to_string());
    let session_id = SessionId::from("session-123".to_string());

    // Acquire exclusive lock on file
    let lock_id = bridge.acquire_lock(
        "src/config.rs",
        LockType::Exclusive,
        &agent_id,
        &session_id,
    ).await?;

    // Do work...
    let content = bridge.read_file(&session_id, "src/config.rs").await?;
    bridge.write_file(&session_id, "src/config.rs", &content).await?;

    // Release lock
    bridge.release_lock(&lock_id).await?;

    Ok(())
}
```

## Configuration

### Default Configuration

```rust
CortexConfig {
    base_url: "http://localhost:8080",
    api_version: "v3",
    auth_token: None,
    cache_size_mb: 100,
    cache_ttl_seconds: 3600,
    connection_pool_size: 10,
    request_timeout_secs: 30,
    max_retries: 3,
    retry_delay_ms: 1000,
    enable_websocket: true,
    reconnect_websocket: true,
}
```

### Custom Configuration

```rust
let config = CortexConfig {
    base_url: "https://cortex.example.com".to_string(),
    api_version: "v3".to_string(),
    auth_token: Some("your-auth-token".to_string()),
    request_timeout_secs: 60,
    max_retries: 5,
    ..Default::default()
};

let bridge = CortexBridge::new(config).await?;
```

## Error Handling

All operations return `Result<T, CortexError>` with the following error types:

- `NetworkError`: Network communication failures
- `CortexUnavailable`: Cortex service is down
- `CortexError`: API-level errors
- `Timeout`: Request timeout
- `SessionNotFound`: Invalid session ID
- `LockFailed`: Unable to acquire lock
- `InvalidResponse`: Malformed API response
- `SerializationError`: JSON serialization issues

## Best Practices

### Session Management

1. **Always close sessions**: Use `close_session()` or call `shutdown()` on bridge
2. **Handle merge conflicts**: Check `MergeReport.conflicts_resolved`
3. **Set appropriate TTL**: Sessions expire automatically after TTL

### Memory & Learning

1. **Store all episodes**: Even failures provide learning opportunities
2. **Search before implementing**: Check for similar past solutions
3. **Update pattern statistics**: Track pattern success rates

### Coordination

1. **Use appropriate lock types**: Shared for reads, Exclusive for writes
2. **Set reasonable timeouts**: Don't block other agents indefinitely
3. **Release locks explicitly**: Don't rely on timeout expiration

### Performance

1. **Connection pooling**: Reuse the same CortexBridge instance
2. **Batch operations**: Multiple reads/writes in one session
3. **Cache awareness**: Repeated queries are cached automatically

## Testing

The module includes comprehensive unit tests:

```bash
cargo test --lib cortex_bridge
```

## Dependencies

- **reqwest**: HTTP client with TLS support
- **tokio**: Async runtime
- **serde**: JSON serialization
- **chrono**: Timestamp handling
- **tracing**: Structured logging
- **urlencoding**: URL-safe path encoding

## Future Enhancements

- [ ] WebSocket support for real-time events
- [ ] Connection pool metrics and monitoring
- [ ] Advanced caching with LRU eviction
- [ ] GraphQL query support
- [ ] Batch API operations
- [ ] Transaction support for multi-file operations

## Related Documentation

- Specification: `/docs/spec/multi-agent-system/10-cortex-integration.md`
- Cortex API: Cortex REST API documentation
- Agent System: `/docs/spec/multi-agent-system/`

## License

Part of the Axon multi-agent system framework.
