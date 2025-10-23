//! Example usage of the data synchronization system
//!
//! This example demonstrates:
//! - Setting up the sync manager
//! - Syncing entities between SurrealDB and Qdrant
//! - Running consistency checks
//! - Performing migrations
//! - Monitoring with metrics and events

use cortex_storage::prelude::*;
use cortex_core::id::CortexId;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Cortex Data Synchronization Example ===\n");

    // ============================================================================
    // 1. Setup - Initialize connections and managers
    // ============================================================================

    println!("1. Initializing connections...");

    // SurrealDB configuration
    let surreal_config = DatabaseConfig {
        namespace: "cortex".to_string(),
        database: "main".to_string(),
        credentials: Credentials::RootAuth {
            username: "root".to_string(),
            password: "root".to_string(),
        },
        ..Default::default()
    };

    let surreal_conn = Arc::new(
        ConnectionManager::new(surreal_config).await?
    );

    println!("✓ Connected to SurrealDB\n");

    // ============================================================================
    // 2. Initialize Sync Manager
    // ============================================================================

    println!("2. Initializing DataSyncManager...");

    let sync_config = SyncConfig {
        qdrant_url: "http://localhost:6333".to_string(),
        qdrant_api_key: None,
        enable_wal: true,
        wal_dir: "/tmp/cortex-wal-example".to_string(),
        max_batch_size: 100,
        sync_timeout_secs: 30,
        enable_retry: true,
        max_retries: 3,
        retry_backoff_ms: 100,
        enable_verification: true,
        verification_interval_secs: 300,
        max_concurrent_ops: 10,
    };

    let sync_manager = Arc::new(
        DataSyncManager::new(sync_config, surreal_conn.clone()).await?
    );

    println!("✓ DataSyncManager initialized\n");

    // ============================================================================
    // 3. Create Qdrant Collections
    // ============================================================================

    println!("3. Creating Qdrant collections...");

    // Create collection for code vectors
    sync_manager.create_collection(
        "code_vectors",
        1536, // OpenAI ada-002 embedding size
        qdrant_client::qdrant::Distance::Cosine,
    ).await?;

    // Create collection for documentation vectors
    sync_manager.create_collection(
        "doc_vectors",
        768, // Smaller embedding size for docs
        qdrant_client::qdrant::Distance::Cosine,
    ).await?;

    println!("✓ Collections created\n");

    // ============================================================================
    // 4. Subscribe to Sync Events
    // ============================================================================

    println!("4. Setting up event monitoring...");

    let mut event_receiver = sync_manager.subscribe();

    // Spawn task to monitor events
    tokio::spawn(async move {
        println!("   Event monitor started\n");
        while let Ok(event) = event_receiver.recv().await {
            match event.event_type {
                SyncEventType::Synced => {
                    println!("   [EVENT] ✓ Entity synced: {} ({})",
                        event.entity_id, event.entity_type);
                }
                SyncEventType::Failed => {
                    println!("   [EVENT] ✗ Sync failed: {} ({})",
                        event.entity_id, event.entity_type);
                }
                SyncEventType::Inconsistent => {
                    println!("   [EVENT] ⚠ Inconsistency detected: {} ({})",
                        event.entity_id, event.entity_type);
                }
                SyncEventType::Repaired => {
                    println!("   [EVENT] ✓ Repaired: {} ({})",
                        event.entity_id, event.entity_type);
                }
                _ => {}
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // ============================================================================
    // 5. Sync Single Entity
    // ============================================================================

    println!("5. Syncing single entity...");

    let code_entity = SyncEntity {
        id: CortexId::new(),
        entity_type: "code".to_string(),
        vector: vec![0.1; 1536], // Mock embedding vector
        metadata: HashMap::from([
            ("file_path".to_string(), json!("/src/main.rs")),
            ("language".to_string(), json!("rust")),
            ("function_name".to_string(), json!("main")),
            ("lines".to_string(), json!(42)),
        ]),
        timestamp: Utc::now(),
        workspace_id: Some("workspace-123".to_string()),
    };

    let sync_result = sync_manager.sync_entity(code_entity.clone()).await?;

    println!("   Result:");
    println!("   - Success: {}", sync_result.success);
    println!("   - Affected: {}", sync_result.affected_count);
    println!("   - Duration: {}ms\n", sync_result.duration_ms);

    // ============================================================================
    // 6. Batch Sync Multiple Entities
    // ============================================================================

    println!("6. Batch syncing multiple entities...");

    let mut entities = Vec::new();
    for i in 0..10 {
        entities.push(SyncEntity {
            id: CortexId::new(),
            entity_type: "code".to_string(),
            vector: vec![0.1 + (i as f32 * 0.01); 1536],
            metadata: HashMap::from([
                ("file_path".to_string(), json!(format!("/src/module_{}.rs", i))),
                ("language".to_string(), json!("rust")),
                ("function_name".to_string(), json!(format!("function_{}", i))),
            ]),
            timestamp: Utc::now(),
            workspace_id: Some("workspace-123".to_string()),
        });
    }

    let batch_result = sync_manager.batch_sync(entities).await?;

    println!("   Batch Result:");
    println!("   - Success: {}", batch_result.success);
    println!("   - Entities synced: {}", batch_result.affected_count);
    println!("   - Duration: {}ms\n", batch_result.duration_ms);

    // Wait for events to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // ============================================================================
    // 7. Get Sync Metrics
    // ============================================================================

    println!("7. Checking sync metrics...");

    let metrics = sync_manager.metrics();
    println!("   Sync Metrics:");
    println!("   - Total operations: {}", metrics.total_operations);
    println!("   - Successful: {}", metrics.successful_operations);
    println!("   - Failed: {}", metrics.failed_operations);
    println!("   - Success rate: {:.2}%",
        (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0);
    println!("   - Average latency: {:.2}ms\n", metrics.average_latency_ms);

    // ============================================================================
    // 8. Initialize Consistency Checker
    // ============================================================================

    println!("8. Initializing ConsistencyChecker...");

    let qdrant_client = qdrant_client::client::QdrantClient::from_url("http://localhost:6333")
        .build()?;

    let consistency_config = ConsistencyConfig {
        batch_size: 100,
        sample_rate: 1.0, // Check all entities
        enable_merkle: true,
        enable_bloom: true,
        bloom_fpr: 0.01,
        bloom_capacity: 100_000,
        enable_auto_repair: true,
        max_repair_batch: 50,
    };

    let consistency_checker = Arc::new(ConsistencyChecker::new(
        consistency_config,
        surreal_conn.clone(),
        Arc::new(qdrant_client),
    ));

    println!("✓ ConsistencyChecker initialized\n");

    // ============================================================================
    // 9. Verify Single Entity
    // ============================================================================

    println!("9. Verifying single entity consistency...");

    let status = consistency_checker.verify_entity(
        &code_entity.id,
        "code"
    ).await?;

    println!("   Status: {:?}\n", status);

    // ============================================================================
    // 10. Run Full Consistency Check
    // ============================================================================

    println!("10. Running full consistency check...");

    let consistency_report = consistency_checker.run_full_check("code").await?;

    println!("   Consistency Report:");
    println!("   - Total checked: {}", consistency_report.total_checked);
    println!("   - Consistent: {}", consistency_report.consistent);
    println!("   - Missing vectors: {}", consistency_report.missing_vectors);
    println!("   - Orphan vectors: {}", consistency_report.orphan_vectors);
    println!("   - Mismatches: {}", consistency_report.mismatches);
    println!("   - Duration: {}ms\n", consistency_report.duration_ms);

    // ============================================================================
    // 11. Repair Inconsistencies (if any)
    // ============================================================================

    if !consistency_report.inconsistent_ids.is_empty() {
        println!("11. Repairing inconsistencies...");

        let repair_result = consistency_checker.repair(
            "code",
            consistency_report.inconsistent_ids,
        ).await?;

        println!("   Repair Result:");
        println!("   - Attempted: {}", repair_result.attempted);
        println!("   - Successful: {}", repair_result.successful);
        println!("   - Failed: {}", repair_result.failed);
        println!("   - Duration: {}ms\n", repair_result.duration_ms);
    } else {
        println!("11. No inconsistencies found, skipping repair\n");
    }

    // ============================================================================
    // 12. Get Consistency Metrics
    // ============================================================================

    println!("12. Checking consistency metrics...");

    let consistency_metrics = consistency_checker.metrics().await;
    println!("   Consistency Metrics:");
    println!("   - Total checks: {}", consistency_metrics.total_checks);
    println!("   - Consistent: {}", consistency_metrics.consistent_checks);
    println!("   - Inconsistent: {}", consistency_metrics.inconsistent_checks);
    println!("   - Consistency rate: {:.2}%",
        (consistency_metrics.consistent_checks as f64 / consistency_metrics.total_checks as f64) * 100.0);
    println!("   - Total repairs: {}", consistency_metrics.total_repairs);
    println!("   - Successful repairs: {}", consistency_metrics.successful_repairs);
    println!("   - Failed repairs: {}\n", consistency_metrics.failed_repairs);

    // ============================================================================
    // 13. Initialize Migration Manager
    // ============================================================================

    println!("13. Initializing MigrationManager...");

    let migration_config = MigrationConfig {
        source_type: "surreal".to_string(),
        target_collection: "code_vectors".to_string(),
        batch_size: 50,
        parallel_workers: 2,
        adaptive_batch_size: true,
        target_latency_ms: 1000,
        enable_checkpointing: true,
        checkpoint_dir: "/tmp/cortex-migration-checkpoints-example".to_string(),
        checkpoint_interval: 5,
        verify_after_migration: true,
        dry_run: false,
        resume_from_checkpoint: None,
    };

    let migration_manager = Arc::new(
        MigrationManager::new(migration_config, surreal_conn.clone()).await?
    );

    println!("✓ MigrationManager initialized\n");

    // ============================================================================
    // 14. Monitor Migration Progress
    // ============================================================================

    println!("14. Starting migration with progress monitoring...");

    let migration_manager_clone = migration_manager.clone();

    // Spawn task to monitor progress
    let progress_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Some(progress) = migration_manager_clone.get_progress().await {
                if progress.status == MigrationStatus::InProgress {
                    let percent = (progress.migrated_entities as f64 / progress.total_entities as f64) * 100.0;
                    println!("   Progress: {}/{} entities ({:.1}%) - {:.2} entities/sec",
                        progress.migrated_entities,
                        progress.total_entities,
                        percent,
                        progress.throughput
                    );

                    if let Some(eta) = progress.estimated_completion {
                        println!("   ETA: {}", eta.format("%H:%M:%S"));
                    }
                } else {
                    break;
                }
            }
        }
    });

    // ============================================================================
    // 15. Run Migration
    // ============================================================================

    // Note: This would actually migrate real data in production
    // For this example, we'll skip the actual migration
    println!("   (Skipping actual migration in example)\n");

    // In production:
    // let migration_report = migration_manager.migrate("code").await?;
    // progress_handle.await?;

    // ============================================================================
    // Summary
    // ============================================================================

    println!("=== Example Complete ===\n");
    println!("This example demonstrated:");
    println!("✓ Setting up DataSyncManager");
    println!("✓ Creating Qdrant collections");
    println!("✓ Syncing single and batch entities");
    println!("✓ Monitoring sync events");
    println!("✓ Checking consistency");
    println!("✓ Repairing inconsistencies");
    println!("✓ Setting up migrations");
    println!("✓ Collecting metrics\n");

    println!("For production use:");
    println!("1. Configure proper Qdrant URLs and authentication");
    println!("2. Set up monitoring with Prometheus");
    println!("3. Configure appropriate batch sizes");
    println!("4. Enable WAL for durability");
    println!("5. Set up automated consistency checks");
    println!("6. Configure alerting for sync failures\n");

    Ok(())
}
