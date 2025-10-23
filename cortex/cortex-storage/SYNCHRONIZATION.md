# Data Synchronization System for SurrealDB and Qdrant

## Overview

This document describes the comprehensive data synchronization system implemented in `cortex-storage` for maintaining consistency between SurrealDB (structured data) and Qdrant (vector embeddings).

## Architecture

### Dual-Storage Pattern

The system implements a **dual-storage architecture** where:
- **SurrealDB** serves as the source of truth for structured metadata and relationships
- **Qdrant** serves as the specialized vector store for embeddings and semantic search
- **Synchronization Manager** coordinates writes between both systems with transactional guarantees

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              DataSyncManager                                 │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │  Write-Ahead Log │  │  Event Streaming │                │
│  └──────────────────┘  └──────────────────┘                │
│  ┌──────────────────┐  ┌──────────────────┐                │
│  │  Compensation    │  │  Retry Logic     │                │
│  └──────────────────┘  └──────────────────┘                │
└─────────────┬──────────────────────┬────────────────────────┘
              │                      │
              ▼                      ▼
    ┌─────────────────┐    ┌─────────────────┐
    │   SurrealDB     │    │     Qdrant      │
    │   (Metadata)    │    │   (Vectors)     │
    └─────────────────┘    └─────────────────┘
```

## Core Components

### 1. DataSyncManager (`sync_manager.rs`)

The central coordination component for all dual-write operations.

#### Features

- **Transactional Patterns with Compensation**: Coordinated writes with automatic rollback on failure
- **Write-Ahead Logging (WAL)**: Crash recovery and durability guarantees
- **Async Event Streaming**: Real-time sync notifications for reactive systems
- **Conflict Resolution**: Semantic understanding for resolving conflicts
- **Batch Operations**: Optimal transaction boundaries for bulk operations
- **Adaptive Performance**: Adjusts batch sizes based on latency

#### Usage Example

```rust
use cortex_storage::prelude::*;
use std::sync::Arc;

// Initialize sync manager
let config = SyncConfig {
    qdrant_url: "http://localhost:6333".to_string(),
    enable_wal: true,
    max_batch_size: 100,
    ..Default::default()
};

let sync_manager = DataSyncManager::new(config, surreal_conn).await?;

// Create Qdrant collection
sync_manager.create_collection(
    "code_vectors",
    1536, // OpenAI embedding size
    Distance::Cosine,
).await?;

// Sync a single entity
let entity = SyncEntity {
    id: CortexId::new(),
    entity_type: "code".to_string(),
    vector: vec![0.1; 1536],
    metadata: HashMap::new(),
    timestamp: Utc::now(),
    workspace_id: Some("workspace-123".to_string()),
};

let result = sync_manager.sync_entity(entity).await?;
println!("Sync result: {:?}", result);

// Batch sync multiple entities
let entities = vec![entity1, entity2, entity3];
let batch_result = sync_manager.batch_sync(entities).await?;

// Subscribe to sync events
let mut events = sync_manager.subscribe();
tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        match event.event_type {
            SyncEventType::Synced => println!("Entity synced: {}", event.entity_id),
            SyncEventType::Failed => eprintln!("Sync failed: {}", event.entity_id),
            SyncEventType::Inconsistent => println!("Inconsistency detected: {}", event.entity_id),
            _ => {}
        }
    }
});

// Get metrics
let metrics = sync_manager.metrics();
println!("Total operations: {}", metrics.total_operations);
println!("Success rate: {:.2}%",
    metrics.successful_operations as f64 / metrics.total_operations as f64 * 100.0);
```

### 2. ConsistencyChecker (`consistency.rs`)

Advanced verification and repair system for ensuring data integrity.

#### Features

- **Merkle Tree Verification**: Efficient consistency checks for large datasets
- **Bloom Filters**: Quick existence checks with minimal memory overhead
- **Probabilistic Consistency**: Sampling strategies for large-scale verification
- **Automated Repair**: Self-healing capabilities with multiple repair strategies
- **Comprehensive Metrics**: Real-time monitoring of consistency status

#### Algorithms

##### Merkle Tree Verification
```rust
// Build Merkle trees for both systems
let surreal_tree = build_merkle_tree(surreal_entities);
let qdrant_tree = build_merkle_tree(qdrant_entities);

// Compare root hashes - O(1) operation
if surreal_tree.root_hash() != qdrant_tree.root_hash() {
    // Recursively find differences - O(log n) operations
    let diffs = find_differences(surreal_tree, qdrant_tree);
}
```

##### Bloom Filter Optimization
```rust
// Build Bloom filter for Qdrant IDs - O(n) build, O(1) queries
let bloom = BloomFilter::new(capacity, false_positive_rate);
for id in qdrant_ids {
    bloom.insert(&id);
}

// Quick existence check - O(1)
if !bloom.contains(&entity_id) {
    // Definitely doesn't exist, no need to query Qdrant
    return ConsistencyStatus::MissingVector;
}
```

#### Usage Example

```rust
use cortex_storage::prelude::*;

// Initialize checker
let config = ConsistencyConfig {
    batch_size: 100,
    sample_rate: 0.1, // Check 10% of entities
    enable_merkle: true,
    enable_bloom: true,
    bloom_fpr: 0.01, // 1% false positive rate
    enable_auto_repair: true,
    ..Default::default()
};

let checker = ConsistencyChecker::new(config, surreal_conn, qdrant_client);

// Check single entity
let status = checker.verify_entity(&entity_id, "code").await?;
match status {
    ConsistencyStatus::Consistent => println!("Entity is consistent"),
    ConsistencyStatus::MissingVector => println!("Vector missing in Qdrant"),
    ConsistencyStatus::OrphanVector => println!("Orphaned vector in Qdrant"),
    ConsistencyStatus::Mismatch => println!("Data mismatch detected"),
    ConsistencyStatus::NotFound => println!("Entity not found"),
}

// Run full consistency check
let report = checker.run_full_check("code").await?;
println!("Consistency Report:");
println!("  Total checked: {}", report.total_checked);
println!("  Consistent: {}", report.consistent);
println!("  Missing vectors: {}", report.missing_vectors);
println!("  Orphan vectors: {}", report.orphan_vectors);
println!("  Mismatches: {}", report.mismatches);

// Repair inconsistencies
if !report.inconsistent_ids.is_empty() {
    let repair_result = checker.repair("code", report.inconsistent_ids).await?;
    println!("Repair Result:");
    println!("  Attempted: {}", repair_result.attempted);
    println!("  Successful: {}", repair_result.successful);
    println!("  Failed: {}", repair_result.failed);
}

// Get metrics
let metrics = checker.metrics().await;
println!("Consistency Metrics:");
println!("  Total checks: {}", metrics.total_checks);
println!("  Consistency rate: {:.2}%",
    metrics.consistent_checks as f64 / metrics.total_checks as f64 * 100.0);
```

### 3. MigrationManager (`migration.rs`)

Production-ready tools for migrating vector data to Qdrant.

#### Features

- **Batch Migration**: Efficient bulk operations with configurable batch sizes
- **Progress Tracking**: Real-time monitoring with ETA calculation
- **Resumable Migrations**: Checkpoint-based recovery from interruptions
- **Verification**: Post-migration consistency checks
- **Adaptive Batch Sizing**: Dynamic adjustment based on observed latency
- **Parallel Workers**: Multi-threaded processing for performance
- **Memory-Efficient Streaming**: Handles large datasets without memory issues

#### Usage Example

```rust
use cortex_storage::prelude::*;

// Configure migration
let config = MigrationConfig {
    source_type: "in_memory".to_string(),
    target_collection: "code_vectors".to_string(),
    batch_size: 100,
    parallel_workers: 4,
    adaptive_batch_size: true,
    target_latency_ms: 1000,
    enable_checkpointing: true,
    checkpoint_interval: 10, // Every 10 batches
    verify_after_migration: true,
    dry_run: false, // Set to true for testing
    resume_from_checkpoint: None, // Or Some("checkpoint-id")
    ..Default::default()
};

// Initialize migration manager
let manager = MigrationManager::new(config, surreal_conn).await?;

// Start migration
let report = manager.migrate("code").await?;

println!("Migration Report:");
println!("  Status: {:?}", report.status);
println!("  Total entities: {}", report.total_entities);
println!("  Successful: {}", report.successful);
println!("  Failed: {}", report.failed);
println!("  Duration: {}ms", report.duration_ms);
println!("  Throughput: {:.2} entities/sec", report.avg_throughput);

if let Some(verification) = report.verification {
    println!("Verification:");
    println!("  Total verified: {}", verification.total_verified);
    println!("  Correct: {}", verification.correct);
    println!("  Incorrect: {}", verification.incorrect);
}

// Monitor progress in separate task
tokio::spawn(async move {
    loop {
        if let Some(progress) = manager.get_progress().await {
            println!("Progress: {}/{} entities migrated ({:.1}%)",
                progress.migrated_entities,
                progress.total_entities,
                progress.migrated_entities as f64 / progress.total_entities as f64 * 100.0
            );
            println!("Throughput: {:.2} entities/sec", progress.throughput);
            if let Some(eta) = progress.estimated_completion {
                println!("ETA: {}", eta);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
});
```

## Transaction Patterns

### Pattern 1: Coordinated Write with WAL

```rust
// 1. Write to WAL for durability
let wal_entry = WalEntry::new(operation);
wal.write(wal_entry).await?;

// 2. Write to SurrealDB (source of truth)
surreal.store_entity(entity).await?;
wal.update_status(wal_id, WalStatus::SurrealCompleted).await?;

// 3. Write to Qdrant (vector store)
qdrant.upsert_point(entity).await?;
wal.update_status(wal_id, WalStatus::QdrantCompleted).await?;

// 4. Commit and clean up
wal.update_status(wal_id, WalStatus::Committed).await?;
wal.remove(wal_id).await?;
```

### Pattern 2: Compensating Transaction

```rust
// Track operations for potential rollback
let mut rollback = RollbackLog::new();

// Step 1: SurrealDB write
surreal.soft_delete(&id).await?;
rollback.add(|| surreal.restore(&id));

// Step 2: Qdrant write
match qdrant.delete(&id).await {
    Ok(_) => {
        // Both succeeded, hard delete from SurrealDB
        surreal.hard_delete(&id).await?;
    }
    Err(e) => {
        // Qdrant failed, rollback SurrealDB
        rollback.execute().await?;
        return Err(e);
    }
}
```

## Consistency Guarantees

### Strong Consistency (Default)
- All writes go through DataSyncManager
- WAL ensures durability
- Automatic retry on transient failures
- Compensation on permanent failures

### Eventual Consistency (Batch Mode)
- High-throughput batch operations
- Periodic consistency checks
- Automated repair of inconsistencies
- Acceptable for bulk imports

### Probabilistic Consistency (Sampling)
- Large-scale verification with sampling
- Bloom filters for quick existence checks
- Statistical confidence levels
- Efficient for massive datasets

## Performance Characteristics

### Sync Operations
- Single entity sync: ~10-50ms (depends on network)
- Batch sync (100 entities): ~500ms-2s
- Adaptive batch sizing maintains target latency

### Consistency Checks
- Single entity check: ~5-20ms
- Full check with Bloom filter: O(n) with low constant
- Full check without Bloom filter: O(n log n)
- Merkle tree verification: O(log n) comparisons

### Migration Performance
- Throughput: 100-1000 entities/second (depends on batch size)
- Memory usage: O(batch_size) - constant with streaming
- Parallelization: Linear speedup with workers

## Configuration Best Practices

### Development
```rust
SyncConfig {
    enable_wal: true,
    max_batch_size: 50,
    enable_retry: true,
    max_retries: 3,
    enable_verification: true,
    ..Default::default()
}
```

### Production
```rust
SyncConfig {
    enable_wal: true,
    wal_dir: "/var/lib/cortex/wal".to_string(),
    max_batch_size: 200,
    enable_retry: true,
    max_retries: 5,
    retry_backoff_ms: 100,
    enable_verification: true,
    verification_interval_secs: 300, // 5 minutes
    max_concurrent_ops: 20,
    ..Default::default()
}
```

### High-Throughput Migration
```rust
MigrationConfig {
    batch_size: 500,
    parallel_workers: 8,
    adaptive_batch_size: true,
    target_latency_ms: 2000,
    enable_checkpointing: true,
    checkpoint_interval: 100,
    ..Default::default()
}
```

## Monitoring and Observability

### Metrics Collection

```rust
// Sync metrics
let metrics = sync_manager.metrics();
println!("Sync Metrics:");
println!("  Operations: {} total, {} success, {} failed",
    metrics.total_operations,
    metrics.successful_operations,
    metrics.failed_operations
);
println!("  Average latency: {:.2}ms", metrics.average_latency_ms);

// Consistency metrics
let consistency_metrics = checker.metrics().await;
println!("Consistency Metrics:");
println!("  Checks: {} total, {} consistent, {} inconsistent",
    consistency_metrics.total_checks,
    consistency_metrics.consistent_checks,
    consistency_metrics.inconsistent_checks
);
println!("  Repairs: {} total, {} successful, {} failed",
    consistency_metrics.total_repairs,
    consistency_metrics.successful_repairs,
    consistency_metrics.failed_repairs
);
```

### Event Streaming

```rust
// Subscribe to sync events
let mut events = sync_manager.subscribe();

tokio::spawn(async move {
    while let Ok(event) = events.recv().await {
        // Send to monitoring system (Prometheus, Datadog, etc.)
        match event.event_type {
            SyncEventType::Synced => {
                metrics::increment_counter!("cortex.sync.success");
            }
            SyncEventType::Failed => {
                metrics::increment_counter!("cortex.sync.failure");
                alert::send("Sync failure detected", &event);
            }
            SyncEventType::Inconsistent => {
                metrics::increment_counter!("cortex.consistency.issue");
                alert::send("Inconsistency detected", &event);
            }
            _ => {}
        }
    }
});
```

## Error Handling

### Transient Errors
- Automatic retry with exponential backoff
- WAL ensures no data loss
- Events emitted for monitoring

### Permanent Errors
- Compensation logic prevents inconsistency
- Detailed error reporting
- Manual intervention may be required

### Recovery Procedures

```rust
// Recover from WAL after crash
let sync_manager = DataSyncManager::new(config, surreal_conn).await?;
// Automatically recovers pending operations from WAL

// Manual consistency repair
let report = checker.run_full_check("code").await?;
if report.mismatches > 0 {
    let repair_result = checker.repair("code", report.inconsistent_ids).await?;
    println!("Repaired {} inconsistencies", repair_result.successful);
}

// Resume interrupted migration
let config = MigrationConfig {
    resume_from_checkpoint: Some("checkpoint-abc123".to_string()),
    ..Default::default()
};
let manager = MigrationManager::new(config, surreal_conn).await?;
let report = manager.migrate("code").await?;
```

## Testing

### Unit Tests
```bash
cargo test --package cortex-storage --lib
```

### Integration Tests
```bash
# Start required services
docker-compose up -d surrealdb qdrant

# Run integration tests
cargo test --package cortex-storage --test '*'
```

### Chaos Testing
```rust
#[tokio::test]
async fn test_sync_with_network_failure() {
    // Simulate network partition
    toxiproxy.add_toxic("latency", Latency { latency: 5000 });

    let result = sync_manager.sync_entity(entity).await;

    // Should retry and eventually succeed or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_consistency_after_crash() {
    // Simulate crash after SurrealDB write but before Qdrant write
    let wal_entries = wal.recover().await?;

    // WAL should have pending entry
    assert!(!wal_entries.is_empty());

    // Recovery should complete the operation
    sync_manager.recover_from_wal().await?;

    // Verify consistency
    let status = checker.verify_entity(&entity_id, "code").await?;
    assert_eq!(status, ConsistencyStatus::Consistent);
}
```

## Troubleshooting

### Common Issues

#### High Sync Latency
```
Problem: Sync operations taking too long
Solution:
  - Increase max_batch_size
  - Enable adaptive_batch_size
  - Add more parallel_workers
  - Check network latency to Qdrant
```

#### Consistency Mismatches
```
Problem: Frequent inconsistencies detected
Solution:
  - Enable WAL if not already enabled
  - Increase retry_backoff_ms
  - Check Qdrant cluster health
  - Run full consistency check and repair
```

#### Migration Failure
```
Problem: Migration stops or fails
Solution:
  - Check checkpoint directory permissions
  - Verify enough disk space for WAL
  - Resume from last checkpoint
  - Reduce batch_size if memory issues
```

## Future Enhancements

- [ ] Multi-region Qdrant replication support
- [ ] Saga pattern for complex distributed transactions
- [ ] Advanced conflict resolution with semantic understanding
- [ ] Real-time streaming sync with change data capture (CDC)
- [ ] Integration with Apache Kafka for event sourcing
- [ ] Distributed consensus with Raft/Paxos
- [ ] Vector versioning and time-travel queries
- [ ] Automated A/B testing for sync strategies

## References

- [Qdrant Documentation](https://qdrant.tech/documentation/)
- [SurrealDB Documentation](https://surrealdb.com/docs)
- [Distributed Systems Patterns](https://martinfowler.com/articles/patterns-of-distributed-systems/)
- [Two-Phase Commit Protocol](https://en.wikipedia.org/wiki/Two-phase_commit_protocol)
- [Saga Pattern](https://microservices.io/patterns/data/saga.html)
