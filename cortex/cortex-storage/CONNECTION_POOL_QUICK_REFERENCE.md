# Connection Pool Quick Reference Guide

## Table of Contents
1. [Quick Start](#quick-start)
2. [Configuration Options](#configuration-options)
3. [Common Use Cases](#common-use-cases)
4. [Best Practices](#best-practices)
5. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Basic Setup
```rust
use cortex_storage::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = DatabaseConfig::default();

    // Initialize connection manager
    let manager = Arc::new(ConnectionManager::new(config).await?);

    // Acquire connection
    let conn = manager.acquire().await?;

    // Use connection
    let result = conn.connection()
        .query("SELECT * FROM users")
        .await?;

    // Graceful shutdown
    manager.shutdown().await?;

    Ok(())
}
```

---

## Configuration Options

### Connection Modes

#### Local Development
```rust
ConnectionMode::Local {
    endpoint: "ws://localhost:8000".to_string()
}
```

#### Remote Production
```rust
ConnectionMode::Remote {
    endpoints: vec![
        "wss://db1.example.com:8000".to_string(),
        "wss://db2.example.com:8000".to_string(),
    ],
    load_balancing: LoadBalancingStrategy::HealthBased,
}
```

#### Hybrid (Local + Remote)
```rust
ConnectionMode::Hybrid {
    local_cache: "ws://localhost:8000".to_string(),
    remote_sync: vec!["wss://db.example.com:8000".to_string()],
    sync_interval: Duration::from_secs(60),
}
```

### Pool Configuration

```rust
PoolConfig {
    // Basic sizing
    min_connections: 5,
    max_connections: 20,

    // Timeouts
    connection_timeout: Duration::from_secs(30),
    idle_timeout: Some(Duration::from_secs(300)),
    max_lifetime: Some(Duration::from_secs(1800)),

    // Enterprise features
    validate_on_checkout: true,
    recycle_after_uses: Some(1000),
    shutdown_grace_period: Duration::from_secs(60),

    // Retry behavior
    retry_policy: RetryPolicy {
        max_attempts: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(10),
        multiplier: 2.0,
    },

    warm_connections: true,
}
```

### Load Balancing Strategies

| Strategy | Best For | Description |
|----------|----------|-------------|
| `RoundRobin` | Even distribution | Cycles through endpoints in order |
| `LeastConnections` | Variable load | Routes to endpoint with fewest active connections |
| `Random` | Simple scenarios | Randomly selects endpoint |
| `HealthBased` | High availability | Routes to healthiest endpoints |

---

## Common Use Cases

### 1. Simple Query
```rust
let conn = manager.acquire().await?;
let result: Vec<User> = conn.connection()
    .query("SELECT * FROM users WHERE active = true")
    .await?
    .take(0)?;
```

### 2. Transaction with Automatic Rollback
```rust
let conn = manager.acquire().await?;

let user_id = conn.with_transaction(|c| {
    Box::pin(async move {
        // Create user
        let user: User = c.connection()
            .query("CREATE user SET name = 'Alice', email = 'alice@example.com'")
            .await?
            .take(0)?;

        // Create profile
        c.connection()
            .query(&format!("CREATE profile SET user_id = {}", user.id))
            .await?;

        Ok(user.id)
    })
}).await?;
```

### 3. Manual Transaction Control
```rust
let conn = manager.acquire().await?;

conn.begin_transaction().await?;

match perform_complex_operation(&conn).await {
    Ok(result) => {
        conn.commit_transaction().await?;
        println!("Success: {:?}", result);
    }
    Err(e) => {
        conn.rollback_transaction().await?;
        eprintln!("Error: {}, transaction rolled back", e);
    }
}
```

### 4. Savepoints for Partial Rollback
```rust
let conn = manager.acquire().await?;

conn.begin_transaction().await?;

// Operation 1
conn.connection().query("CREATE user:1").await?;

// Create savepoint before risky operation
conn.savepoint("before_risky_op").await?;

// Risky operation
if let Err(e) = risky_operation(&conn).await {
    // Rollback to savepoint only
    conn.rollback_to_savepoint("before_risky_op").await?;
} else {
    conn.commit_transaction().await?;
}
```

### 5. Agent Sessions with Resource Limits
```rust
let limits = ResourceLimits {
    max_concurrent_connections: 5,
    max_operations: 10000,
    max_transaction_log_size: 1000,
};

let session = AgentSession::create_with_limits(
    "data-processor-01".to_string(),
    manager.clone(),
    "processing_namespace".to_string(),
    limits,
).await?;

// Session enforces limits automatically
let conn = session.acquire().await?;

// Record transaction
let txn_id = session.record_transaction(TransactionOperation::Write {
    path: "/data/output.json".to_string(),
    content_hash: "sha256:abc...".to_string(),
});

// Perform work...

session.commit_transaction(txn_id);

// Monitor session
let stats = session.session_stats();
println!("Operations: {}/{}", stats.total_operations,
    stats.resource_limits.max_operations);
```

### 6. Health Monitoring
```rust
// Periodic health check
tokio::spawn({
    let manager = manager.clone();
    async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let health = manager.health_status();
            let stats = manager.pool_stats();

            if !health.healthy {
                eprintln!("Pool unhealthy! Circuit breaker: {:?}",
                    health.circuit_breaker_state);
            }

            println!("Pool: {}/{} connections, {:.1}% health pass rate",
                stats.in_use_connections,
                stats.total_connections,
                stats.health_check_pass_rate
            );
        }
    }
});
```

### 7. Graceful Shutdown
```rust
// Register shutdown handler
let shutdown_manager = manager.clone();

tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();

    println!("Received shutdown signal...");

    match shutdown_manager.shutdown().await {
        Ok(_) => println!("Shutdown complete"),
        Err(e) => eprintln!("Shutdown error: {}", e),
    }

    std::process::exit(0);
});
```

### 8. Connection Recycling
```rust
let config = DatabaseConfig {
    pool_config: PoolConfig {
        // Recycle connections after 5000 uses
        recycle_after_uses: Some(5000),
        ..Default::default()
    },
    ..Default::default()
};

let manager = ConnectionManager::new(config).await?;

// Connections automatically recycled when limit reached
```

### 9. Retry with Exponential Backoff
```rust
let result = manager.execute_with_retry(|| {
    Box::pin(async {
        let conn = manager.acquire().await?;

        // Operation that might fail transiently
        conn.connection()
            .query("SELECT * FROM remote_table")
            .await?
            .take(0)
    })
}).await?;
```

---

## Best Practices

### 1. Connection Management

**✅ DO:**
- Always use `Arc<ConnectionManager>` for sharing across tasks
- Return connections promptly (they auto-return on drop)
- Enable `validate_on_checkout` for production
- Use appropriate pool sizing (start small, scale up)

**❌ DON'T:**
- Hold connections across await points unnecessarily
- Create multiple managers for same database
- Disable health checks in production
- Use unbounded max_connections

### 2. Transaction Management

**✅ DO:**
- Use `with_transaction()` for automatic cleanup
- Keep transactions short and focused
- Use savepoints for complex multi-step operations
- Handle rollback errors gracefully

**❌ DON'T:**
- Start transaction without committing/rolling back
- Perform long-running operations inside transactions
- Ignore transaction errors
- Nest transactions without savepoints

### 3. Resource Limits

**✅ DO:**
- Set realistic limits based on workload
- Monitor session statistics regularly
- Use different limits for different agent types
- Plan for limit exhaustion scenarios

**❌ DON'T:**
- Use same limits for all sessions
- Set limits too low (causes frequent failures)
- Ignore limit violation errors
- Forget to clean up old sessions

### 4. Error Handling

**✅ DO:**
- Always check `Result` types
- Log connection failures
- Implement retry logic for transient failures
- Monitor circuit breaker state

**❌ DON'T:**
- Unwrap connection acquisitions
- Ignore health check failures
- Retry forever without backoff
- Suppress error logging

### 5. Monitoring

**✅ DO:**
- Regularly call `pool_stats()` and `health_status()`
- Set up alerts for circuit breaker opens
- Track connection reuse ratio
- Monitor per-session resource usage

**❌ DON'T:**
- Only check health when problems occur
- Ignore degraded performance warnings
- Run production without monitoring
- Forget to export metrics

---

## Troubleshooting

### Connection Acquisition Timeouts

**Symptoms:**
```
Error: Database error: Connection acquisition timeout
```

**Solutions:**
1. Increase `max_connections` in pool config
2. Reduce connection hold time in application
3. Check for connection leaks (held but not used)
4. Review `connection_timeout` setting

**Debug:**
```rust
let stats = manager.pool_stats();
println!("Available: {}/{}", stats.available_connections, stats.total_connections);
if stats.available_connections == 0 {
    println!("Pool exhausted! All connections in use.");
}
```

### Circuit Breaker Open

**Symptoms:**
```
Error: Circuit breaker open - too many failures
```

**Solutions:**
1. Check database connectivity
2. Review recent error logs
3. Verify credentials are correct
4. Wait for circuit breaker timeout (60s default)

**Debug:**
```rust
let health = manager.health_status();
println!("Circuit breaker: {:?}", health.circuit_breaker_state);

let metrics = manager.metrics().snapshot();
println!("Recent failures: {}", metrics.errors);
```

### Health Check Failures

**Symptoms:**
- `validate_on_checkout` retries
- High health check failure rate

**Solutions:**
1. Verify database is running
2. Check network connectivity
3. Review idle/max lifetime settings
4. Check database logs for errors

**Debug:**
```rust
let stats = manager.pool_stats();
println!("Health pass rate: {:.1}%", stats.health_check_pass_rate);

if stats.health_check_pass_rate < 90.0 {
    eprintln!("Health checks failing frequently!");
}
```

### Transaction Rollback Failures

**Symptoms:**
```
WARN: Failed to rollback transaction: ...
```

**Solutions:**
1. Check connection is still healthy
2. Verify transaction was actually started
3. Review SurrealDB logs
4. Ensure database supports transactions

**Debug:**
```rust
let conn = manager.acquire().await?;

conn.begin_transaction().await?;

// Perform operations...

if let Err(e) = conn.commit_transaction().await {
    eprintln!("Commit failed: {}", e);

    // Try rollback
    match conn.rollback_transaction().await {
        Ok(_) => println!("Rollback successful"),
        Err(rb_err) => eprintln!("Rollback also failed: {}", rb_err),
    }
}
```

### Resource Limit Exceeded

**Symptoms:**
```
Error: Concurrency error: Session xyz exceeded max concurrent connections limit
```

**Solutions:**
1. Increase limits for the session
2. Release connections sooner
3. Implement connection pooling within session
4. Review session workload patterns

**Debug:**
```rust
let stats = session.session_stats();
println!("Connections: {}/{}",
    stats.active_connections,
    stats.resource_limits.max_concurrent_connections);

println!("Operations: {}/{}",
    stats.total_operations,
    stats.resource_limits.max_operations);

if !session.is_within_limits() {
    println!("Session at or near limits!");
}
```

### Shutdown Hangs

**Symptoms:**
- `shutdown()` takes longer than expected
- Grace period timeout reached

**Solutions:**
1. Increase `shutdown_grace_period`
2. Check for stuck connections
3. Review in-flight operations
4. Force kill if necessary (last resort)

**Debug:**
```rust
println!("Starting shutdown...");
let start = std::time::Instant::now();

match tokio::time::timeout(
    Duration::from_secs(120),
    manager.shutdown()
).await {
    Ok(Ok(_)) => println!("Shutdown completed in {:?}", start.elapsed()),
    Ok(Err(e)) => eprintln!("Shutdown error: {}", e),
    Err(_) => eprintln!("Shutdown timed out after 120s"),
}
```

---

## Performance Tuning

### Pool Sizing

**Guidelines:**
- Start with: `min = 2-5`, `max = 10-20`
- High-load: `min = 10-20`, `max = 50-100`
- Monitor and adjust based on `pool_stats()`

**Formula:**
```
max_connections = (expected_concurrent_operations * avg_operation_time) / connection_utilization_target
```

Example: 100 ops/sec * 0.1s/op / 0.8 = ~13 connections

### Timeout Settings

| Timeout | Development | Production |
|---------|-------------|------------|
| `connection_timeout` | 5s | 30s |
| `idle_timeout` | 60s | 300s (5min) |
| `max_lifetime` | 300s (5min) | 1800s (30min) |
| `shutdown_grace_period` | 5s | 60s |

### Retry Policy

**Conservative (high reliability):**
```rust
RetryPolicy {
    max_attempts: 5,
    initial_backoff: Duration::from_millis(100),
    max_backoff: Duration::from_secs(30),
    multiplier: 2.0,
}
```

**Aggressive (low latency):**
```rust
RetryPolicy {
    max_attempts: 2,
    initial_backoff: Duration::from_millis(50),
    max_backoff: Duration::from_secs(1),
    multiplier: 2.0,
}
```

---

## Testing

### Unit Tests
```bash
cargo test --lib connection_pool
```

### Integration Tests
```bash
cargo test --test connection_pool_integration_tests
```

### Load Tests
```bash
cargo test --test connection_pool_load_tests --release -- --nocapture
```

### With Real SurrealDB
```bash
# Terminal 1: Start SurrealDB
surreal start --bind 0.0.0.0:8000 memory

# Terminal 2: Run tests
cargo test --test connection_pool_integration_tests -- --test-threads=1
```

---

## Additional Resources

- [Full Enhancement Report](./CONNECTION_POOL_ENHANCEMENTS.md)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [Rust Async Book](https://rust-lang.github.io/async-book/)

---

**Last Updated:** 2025-10-20
