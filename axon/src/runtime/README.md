# Agent Runtime System

The Axon Agent Runtime provides a production-ready system for spawning, managing, and executing agents in isolated processes with comprehensive resource management and MCP integration.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        AgentRuntime                             │
│  (Main coordinator managing the entire runtime lifecycle)      │
└──────────┬──────────────────────────┬─────────────────┬────────┘
           │                          │                 │
           ▼                          ▼                 ▼
┌──────────────────┐     ┌──────────────────┐  ┌─────────────────┐
│ ProcessManager   │     │ McpServerPool    │  │ AgentExecutor   │
│                  │     │                  │  │                 │
│ • Spawn processes│     │ • MCP stdio      │  │ • Task execution│
│ • Monitor health │     │ • Tool calls     │  │ • Delegation    │
│ • Resource limits│     │ • Server pool    │  │ • Result agg    │
└──────────────────┘     └──────────────────┘  └─────────────────┘
```

## Key Components

### 1. AgentRuntime
Main orchestrator managing the entire runtime lifecycle.

**Features:**
- Agent spawning and termination
- Task execution coordination
- Health monitoring
- Statistics collection
- Graceful shutdown

**Example:**
```rust
use axon::runtime::{AgentRuntime, RuntimeConfig};
use axon::coordination::UnifiedMessageBus;
use std::sync::Arc;

let message_bus = Arc::new(UnifiedMessageBus::new());
let config = RuntimeConfig::default();
let runtime = AgentRuntime::new(config, message_bus);

runtime.start().await?;

// Spawn an agent
let agent_id = runtime.spawn_agent(
    "worker-1".to_string(),
    AgentType::Developer,
    "cortex",
    &["mcp".to_string(), "stdio".to_string()],
).await?;

// Execute a task
let result = runtime.execute_task(&agent_id, task_delegation).await?;

runtime.shutdown().await?;
```

### 2. ProcessManager
Spawns and monitors agent processes with resource limits.

**Features:**
- Process isolation
- Resource tracking (CPU, memory)
- Health checks
- Heartbeat monitoring
- Graceful termination

**Resource Limits:**
- Max memory per process
- CPU percentage limits
- File descriptor limits
- Task duration limits
- Tool call limits

### 3. McpServerPool
Manages MCP (Model Context Protocol) servers for agent communication with Cortex.

**Features:**
- MCP stdio server management
- Tool call routing via JSON-RPC 2.0
- Server lifecycle management
- Connection pooling
- Automatic Cortex binary discovery

**MCP Protocol:**
The runtime launches Cortex in MCP stdio mode using:
```bash
cortex mcp stdio
```

Communication happens via JSON-RPC 2.0 over stdin/stdout:
```json
// Request
{
  "jsonrpc": "2.0",
  "id": "uuid-1234",
  "method": "tools/call",
  "params": {
    "name": "cortex_search",
    "arguments": {"query": "test"}
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "uuid-1234",
  "result": {
    "success": true,
    "content": [{"type": "text", "text": "Found 5 memories..."}]
  }
}
```

**Available Tools:**
- `cortex_search` - Search cognitive memory
- `cortex_store` - Store new memories
- `cortex_retrieve` - Retrieve specific memories
- `cortex_consolidate` - Trigger memory consolidation

**Example:**
```rust
use axon::runtime::mcp_integration::{McpServerPool, ToolCall};

let mcp_pool = McpServerPool::new(mcp_config);

// Get or create server for agent (spawns cortex process)
mcp_pool.get_or_create(&agent_id).await?;

// Call a tool via MCP
let tool_call = ToolCall {
    name: "cortex_search".to_string(),
    arguments: serde_json::json!({
        "query": "authentication patterns",
        "limit": 10
    }),
};

let result = mcp_pool.call_tool(&agent_id, tool_call).await?;
println!("Tool result: {:?}", result.content);
```

### 4. AgentExecutor
Executes task delegations on actual agent processes.

**Features:**
- Task delegation execution
- Tool call management
- Timeout handling
- Result aggregation
- Statistics tracking

**Example:**
```rust
use axon::runtime::AgentExecutor;

let executor = AgentExecutor::new(
    process_manager,
    mcp_pool,
    runtime_config,
);

// Execute single task
let result = executor.execute_task(&agent_id, delegation).await?;

// Execute multiple tasks in parallel
let results = executor.execute_tasks_parallel(tasks, max_parallel).await;
```

## Configuration

### RuntimeConfig
Complete runtime configuration with sensible defaults.

```rust
use axon::runtime::RuntimeConfig;
use std::time::Duration;

let config = RuntimeConfig {
    process: ProcessConfig {
        max_concurrent_processes: 10,
        spawn_timeout: Duration::from_secs(30),
        shutdown_grace_period: Duration::from_secs(10),
        enable_isolation: true,
        ..Default::default()
    },
    resources: ResourceLimits {
        max_memory_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
        cpu_limit_percent: Some(80.0),
        max_task_duration: Duration::from_secs(300),
        max_tool_calls_per_task: 50,
        ..Default::default()
    },
    mcp: McpConfig {
        cortex_binary_path: None, // Auto-discover
        protocol_version: "2024-11-05".to_string(),
        request_timeout: Duration::from_secs(30),
        ..Default::default()
    },
    monitoring: MonitoringConfig {
        health_check_interval: Duration::from_secs(10),
        enable_metrics: true,
        enable_log_aggregation: true,
        ..Default::default()
    },
    recovery: RecoveryConfig {
        enable_auto_restart: true,
        max_restart_attempts: 3,
        enable_graceful_degradation: true,
        ..Default::default()
    },
};
```

## Process Lifecycle

```
┌─────────┐     spawn     ┌──────────┐    ready    ┌────────┐
│ Initial │──────────────>│ Starting │────────────>│ Ready  │
└─────────┘               └──────────┘             └────┬───┘
                                                        │
                          execute task                  │
                          ┌───────────────────────────┘
                          │
                          ▼
                    ┌──────────┐    complete    ┌────────┐
                    │   Busy   │────────────────>│  Idle  │
                    └──────────┘                 └────┬───┘
                          │                           │
                          │ terminate                 │
                          ▼                           │
                    ┌──────────────┐◄─────────────────┘
                    │ ShuttingDown │
                    └──────┬───────┘
                           │
                           ▼
                    ┌─────────────┐
                    │ Terminated  │
                    └─────────────┘
```

## Task Execution Flow

1. **Task Submission**: Lead agent creates TaskDelegation
2. **Worker Acquisition**: Runtime acquires available worker from pool
3. **Process Check**: Verify agent process is alive and healthy
4. **MCP Server**: Ensure MCP server is running for the agent
5. **Task Execution**: Execute task with timeout and resource limits
6. **Tool Calls**: Route tool calls through MCP to Cortex
7. **Result Collection**: Aggregate results and update statistics
8. **Worker Release**: Return worker to pool

## Monitoring & Metrics

### Runtime Statistics
```rust
let stats = runtime.get_statistics().await;

println!("Total agents spawned: {}", stats.total_agents_spawned);
println!("Active agents: {}", stats.active_agents);
println!("Total tasks executed: {}", stats.total_tasks_executed);
println!("Success rate: {:.2}%", stats.success_rate * 100.0);
```

### Process Statistics
```rust
let process_stats = process_manager.get_statistics().await;

println!("Active processes: {}", process_stats.active_processes);
println!("Total memory: {} MB", process_stats.total_memory_bytes / 1024 / 1024);
println!("Total CPU time: {}ms", process_stats.total_cpu_time_ms);
```

### Executor Statistics
```rust
let executor_stats = executor.get_statistics().await;

println!("Total tasks: {}", executor_stats.total_tasks);
println!("Successful: {}", executor_stats.successful_tasks);
println!("Failed: {}", executor_stats.failed_tasks);
println!("Avg execution time: {}ms", executor_stats.avg_execution_time_ms);
```

## Health Monitoring

The runtime includes automatic health monitoring:

- **Heartbeat Checks**: Regular heartbeat updates from agents
- **Process Monitoring**: Detect crashed or hung processes
- **Resource Tracking**: Monitor memory and CPU usage
- **Automatic Cleanup**: Remove dead processes
- **Failure Detection**: Mark failed agents and attempt recovery

## Error Handling

The runtime provides comprehensive error handling:

```rust
use axon::runtime::RuntimeError;

match runtime.execute_task(&agent_id, task).await {
    Ok(result) => println!("Task completed: {:?}", result),
    Err(RuntimeError::AgentNotFound(id)) => {
        eprintln!("Agent {} not found", id);
    }
    Err(RuntimeError::Timeout(msg)) => {
        eprintln!("Task timed out: {}", msg);
    }
    Err(RuntimeError::ResourceLimitExceeded(msg)) => {
        eprintln!("Resource limit exceeded: {}", msg);
    }
    Err(e) => eprintln!("Runtime error: {}", e),
}
```

## Integration with LeadAgent

The runtime integrates seamlessly with the orchestration layer:

```rust
use axon::orchestration::{LeadAgent, LeadAgentWithRuntime};

// Create LeadAgent
let lead_agent = LeadAgent::new(
    "Orchestrator".to_string(),
    cortex,
    strategy_library,
    worker_registry,
    result_synthesizer,
    message_bus,
    coordinator,
    config,
);

// Wrap with runtime integration
let lead_agent_with_runtime = LeadAgentWithRuntime::new(
    lead_agent,
    runtime.clone(),
);

// Handle query with actual agent execution
let result = lead_agent_with_runtime
    .handle_query(query, workspace_id, session_id)
    .await?;
```

## Production Considerations

### Resource Management
- Set appropriate memory limits per agent
- Configure CPU limits to prevent resource starvation
- Monitor and enforce timeout limits
- Track and limit tool calls per task

### Scalability
- Adjust `max_concurrent_processes` based on system capacity
- Use worker pools for efficient agent reuse
- Implement rate limiting for tool calls
- Consider horizontal scaling for large deployments

### Reliability
- Enable automatic restart for failed agents
- Configure graceful degradation
- Set up proper monitoring and alerting
- Implement checkpoint/resume for long-running tasks

### Security
- Enable process isolation
- Validate all input to agents
- Implement proper access controls
- Audit tool calls and agent actions

## Best Practices

1. **Always call `runtime.start()` before spawning agents**
2. **Always call `runtime.shutdown()` for graceful cleanup**
3. **Monitor resource usage and adjust limits accordingly**
4. **Use appropriate timeout values for your use case**
5. **Implement proper error handling for all runtime operations**
6. **Track metrics and statistics for performance tuning**
7. **Test agent processes in isolation before production**
8. **Use health checks to detect and recover from failures**

## Examples

See the `examples/` directory for complete examples:
- `multi_agent_runtime.rs` - Basic runtime usage with multiple agents
- More examples coming soon...

## Future Enhancements

- [ ] Container-based process isolation (Docker/Podman)
- [ ] Remote agent execution
- [ ] Advanced checkpoint/resume functionality
- [ ] Resource quota management
- [ ] Dynamic worker scaling
- [ ] Advanced metrics and telemetry
- [ ] Distributed agent execution
- [ ] Agent warm pooling for faster startup
