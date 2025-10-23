//! Complete Migration Workflow End-to-End Tests
//!
//! This test suite validates the complete migration workflow from HNSW to Qdrant:
//! 1. Start with fully populated HNSW index
//! 2. Enable dual-write mode (write to both HNSW and Qdrant)
//! 3. Verify consistency between both stores
//! 4. Switch to Qdrant as primary (read from Qdrant, fallback to HNSW)
//! 5. Complete migration and verify no data loss
//! 6. Performance comparison before and after migration
//! 7. Rollback scenarios and recovery procedures
//!
//! These tests ensure zero-downtime migration capability.

use cortex_core::prelude::*;
use cortex_semantic::prelude::*;
use cortex_semantic::index::{HNSWIndex, VectorIndex, SearchResult as IndexSearchResult, IndexStats};
use cortex_semantic::{HybridVectorStore, MigrationMode, HybridMetrics};
use cortex_semantic::types::{SimilarityMetric, Vector, DocumentId};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// ============================================================================
// Test Configuration
// ============================================================================

const EMBEDDING_DIMENSION: usize = 384;
const MIGRATION_DATASET_SIZE: usize = 1000;
const SEARCH_SAMPLE_SIZE: usize = 50;
const CONSISTENCY_CHECK_INTERVAL_MS: u64 = 100;

fn create_qdrant_config() -> QdrantConfig {
    let mut config = QdrantConfig::default();
    config.url = std::env::var("QDRANT_URL")
        .unwrap_or_else(|_| "http://localhost:6333".to_string());
    config.collection_name = format!("migration_test_{}", Uuid::new_v4());
    config.enable_quantization = true;
    config.quantization_type = QuantizationType::Scalar;
    config.write_batch_size = 100;
    config
}

// ============================================================================
// Migration Test Context
// ============================================================================

struct MigrationContext {
    hnsw_store: Arc<HNSWIndex>,
    qdrant_store: Arc<QdrantVectorStore>,
    hybrid_store: Arc<Mutex<HybridVectorStore>>,
    test_vectors: Vec<(DocumentId, Vector)>,
    query_vectors: Vec<Vector>,
    metrics: Arc<Mutex<MigrationMetrics>>,
}

#[derive(Debug, Default)]
struct MigrationMetrics {
    migration_start: Option<Instant>,
    migration_end: Option<Instant>,
    vectors_migrated: usize,
    dual_write_successes: usize,
    dual_write_failures: usize,
    consistency_checks_passed: usize,
    consistency_checks_failed: usize,
    search_queries_before: usize,
    search_queries_after: usize,
    avg_latency_before_ms: f64,
    avg_latency_after_ms: f64,
}

impl MigrationContext {
    async fn new() -> Result<Self> {
        info!("Setting up migration test context");

        // Create HNSW index
        let hnsw_store = Arc::new(HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine));

        // Create Qdrant store
        let qdrant_config = create_qdrant_config();
        let qdrant_store = Arc::new(
            QdrantVectorStore::new(qdrant_config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                .await?,
        );

        // Create hybrid store (initially in SingleStore mode - HNSW only)
        let hybrid_store = Arc::new(Mutex::new(HybridVectorStore::new(
            hnsw_store.clone() as Arc<dyn VectorIndex>,
            qdrant_store.clone() as Arc<dyn VectorIndex>,
            MigrationMode::SingleStore,
        )));

        // Generate test data
        let test_vectors = Self::generate_test_vectors(MIGRATION_DATASET_SIZE, EMBEDDING_DIMENSION);
        let query_vectors = Self::generate_query_vectors(SEARCH_SAMPLE_SIZE, EMBEDDING_DIMENSION);

        Ok(Self {
            hnsw_store,
            qdrant_store,
            hybrid_store,
            test_vectors,
            query_vectors,
            metrics: Arc::new(Mutex::new(MigrationMetrics::default())),
        })
    }

    fn generate_test_vectors(count: usize, dimension: usize) -> Vec<(DocumentId, Vector)> {
        (0..count)
            .map(|i| {
                let id = format!("doc_{:06}", i);
                let vector = (0..dimension)
                    .map(|j| {
                        let val = ((i * 1000 + j * 137 + i * j) % 1000) as f32 / 1000.0;
                        val * 2.0 - 1.0 // Normalize to [-1, 1]
                    })
                    .collect();
                (id, vector)
            })
            .collect()
    }

    fn generate_query_vectors(count: usize, dimension: usize) -> Vec<Vector> {
        (0..count)
            .map(|i| {
                (0..dimension)
                    .map(|j| {
                        let val = ((i * 500 + j * 271) % 1000) as f32 / 1000.0;
                        val * 2.0 - 1.0
                    })
                    .collect()
            })
            .collect()
    }

    async fn populate_hnsw(&self) -> Result<()> {
        info!("Populating HNSW index with {} vectors", self.test_vectors.len());
        let start = Instant::now();

        self.hnsw_store.insert_batch(self.test_vectors.clone()).await?;

        let duration = start.elapsed();
        let throughput = self.test_vectors.len() as f64 / duration.as_secs_f64();
        info!(
            "HNSW population completed in {:?} ({:.2} vectors/sec)",
            duration, throughput
        );

        Ok(())
    }

    async fn measure_search_performance(&self, store: &Arc<dyn VectorIndex>) -> Result<f64> {
        let mut total_latency = 0u128;

        for query in &self.query_vectors {
            let start = Instant::now();
            let _ = store.search(query, 10).await?;
            total_latency += start.elapsed().as_millis();
        }

        let avg_latency = total_latency as f64 / self.query_vectors.len() as f64;
        Ok(avg_latency)
    }

    async fn verify_consistency(&self, sample_size: usize) -> Result<ConsistencyReport> {
        info!("Verifying consistency between HNSW and Qdrant");

        let mut report = ConsistencyReport {
            total_checked: 0,
            consistent: 0,
            inconsistent: 0,
            missing_in_qdrant: Vec::new(),
            score_differences: Vec::new(),
        };

        // Sample vectors to check
        let sample_indices: Vec<usize> = (0..sample_size)
            .map(|i| i * (self.test_vectors.len() / sample_size))
            .collect();

        for idx in sample_indices {
            if idx >= self.test_vectors.len() {
                break;
            }

            let (doc_id, vector) = &self.test_vectors[idx];
            report.total_checked += 1;

            // Search in both stores
            let hnsw_results = self.hnsw_store.search(vector, 1).await?;
            let qdrant_results = self.qdrant_store.search(vector, 1).await?;

            // Check if the document exists in Qdrant
            if qdrant_results.is_empty() {
                report.missing_in_qdrant.push(doc_id.clone());
                report.inconsistent += 1;
                continue;
            }

            // Compare top result
            if !hnsw_results.is_empty() && !qdrant_results.is_empty() {
                if hnsw_results[0].doc_id == qdrant_results[0].doc_id {
                    report.consistent += 1;

                    // Track score differences
                    let score_diff = (hnsw_results[0].score - qdrant_results[0].score).abs();
                    report.score_differences.push(score_diff);
                } else {
                    report.inconsistent += 1;
                    debug!(
                        "Inconsistent results for {}: HNSW={}, Qdrant={}",
                        doc_id, hnsw_results[0].doc_id, qdrant_results[0].doc_id
                    );
                }
            }
        }

        let consistency_rate = if report.total_checked > 0 {
            report.consistent as f64 / report.total_checked as f64
        } else {
            0.0
        };

        info!(
            "Consistency check: {}/{} consistent ({:.1}%)",
            report.consistent,
            report.total_checked,
            consistency_rate * 100.0
        );

        Ok(report)
    }
}

#[derive(Debug)]
struct ConsistencyReport {
    total_checked: usize,
    consistent: usize,
    inconsistent: usize,
    missing_in_qdrant: Vec<DocumentId>,
    score_differences: Vec<f32>,
}

// ============================================================================
// TEST 1: Complete Migration Workflow
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_1_complete_migration_workflow() -> Result<()> {
    info!("========================================");
    info!("TEST 1: Complete Migration Workflow");
    info!("========================================");

    let ctx = MigrationContext::new().await?;

    // Phase 1: Start with HNSW (baseline)
    info!("Phase 1: Establishing HNSW baseline");
    ctx.populate_hnsw().await?;

    let hnsw_count = ctx.hnsw_store.len().await;
    info!("HNSW index contains {} vectors", hnsw_count);
    assert_eq!(hnsw_count, MIGRATION_DATASET_SIZE);

    // Measure baseline search performance
    let baseline_latency = ctx
        .measure_search_performance(&(ctx.hnsw_store.clone() as Arc<dyn VectorIndex>))
        .await?;
    info!("Baseline search latency: {:.2}ms", baseline_latency);

    {
        let mut metrics = ctx.metrics.lock().await;
        metrics.avg_latency_before_ms = baseline_latency;
        metrics.search_queries_before = ctx.query_vectors.len();
    }

    // Phase 2: Enable dual-write mode
    info!("Phase 2: Enabling dual-write mode");
    {
        let mut metrics = ctx.metrics.lock().await;
        metrics.migration_start = Some(Instant::now());
    }

    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::DualWrite).await;
        let mode = hybrid.mode().await;
        info!("Migration mode set to: {:?}", mode);
        assert_eq!(mode, MigrationMode::DualWrite);
    }

    // Phase 3: Migrate existing data to Qdrant
    info!("Phase 3: Migrating {} vectors to Qdrant", ctx.test_vectors.len());
    let migration_start = Instant::now();

    // Batch migration
    const BATCH_SIZE: usize = 100;
    for (batch_num, chunk) in ctx.test_vectors.chunks(BATCH_SIZE).enumerate() {
        ctx.qdrant_store.insert_batch(chunk.to_vec()).await?;

        let migrated_so_far = (batch_num + 1) * BATCH_SIZE.min(chunk.len());
        if migrated_so_far % 500 == 0 {
            info!("Migrated {}/{} vectors", migrated_so_far, ctx.test_vectors.len());
        }
    }

    let migration_duration = migration_start.elapsed();
    let migration_throughput = ctx.test_vectors.len() as f64 / migration_duration.as_secs_f64();
    info!(
        "Migration completed in {:?} ({:.2} vectors/sec)",
        migration_duration, migration_throughput
    );

    // Wait for Qdrant to process
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Phase 4: Verify data consistency
    info!("Phase 4: Verifying data consistency");
    let consistency_report = ctx.verify_consistency(100).await?;

    assert!(
        consistency_report.missing_in_qdrant.is_empty(),
        "Found {} vectors missing in Qdrant",
        consistency_report.missing_in_qdrant.len()
    );

    let consistency_rate = consistency_report.consistent as f64 / consistency_report.total_checked as f64;
    assert!(
        consistency_rate >= 0.9,
        "Consistency rate too low: {:.1}%",
        consistency_rate * 100.0
    );

    // Analyze score differences
    if !consistency_report.score_differences.is_empty() {
        let avg_score_diff: f32 = consistency_report.score_differences.iter().sum::<f32>()
            / consistency_report.score_differences.len() as f32;
        let max_score_diff = consistency_report
            .score_differences
            .iter()
            .cloned()
            .fold(0.0f32, f32::max);

        info!("Score difference analysis:");
        info!("  - Average: {:.6}", avg_score_diff);
        info!("  - Maximum: {:.6}", max_score_diff);

        // Scores should be similar (allowing for algorithmic differences)
        assert!(
            avg_score_diff < 0.1,
            "Average score difference too high: {:.6}",
            avg_score_diff
        );
    }

    // Phase 5: Test dual-write with new data
    info!("Phase 5: Testing dual-write with new insertions");
    let new_vectors = MigrationContext::generate_test_vectors(50, EMBEDDING_DIMENSION);

    for (id, vector) in &new_vectors {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.insert(id.clone(), vector.clone()).await?;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify new data in both stores
    let hnsw_new_count = ctx.hnsw_store.len().await;
    let qdrant_new_count = ctx.qdrant_store.len().await;
    info!("After dual-write: HNSW={}, Qdrant={}", hnsw_new_count, qdrant_new_count);

    assert_eq!(hnsw_new_count, MIGRATION_DATASET_SIZE + new_vectors.len());
    assert_eq!(qdrant_new_count, MIGRATION_DATASET_SIZE + new_vectors.len());

    // Phase 6: Switch to Qdrant as primary
    info!("Phase 6: Switching to Qdrant as primary");
    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::NewPrimary).await;
        let mode = hybrid.mode().await;
        info!("Migration mode set to: {:?}", mode);
        assert_eq!(mode, MigrationMode::NewPrimary);
    }

    // Phase 7: Measure post-migration search performance
    info!("Phase 7: Measuring post-migration performance");
    let post_migration_latency = ctx
        .measure_search_performance(&(ctx.qdrant_store.clone() as Arc<dyn VectorIndex>))
        .await?;
    info!("Post-migration search latency: {:.2}ms", post_migration_latency);

    {
        let mut metrics = ctx.metrics.lock().await;
        metrics.avg_latency_after_ms = post_migration_latency;
        metrics.search_queries_after = ctx.query_vectors.len();
        metrics.vectors_migrated = ctx.test_vectors.len();
        metrics.migration_end = Some(Instant::now());
    }

    // Phase 8: Verify search results quality
    info!("Phase 8: Verifying search quality");
    let mut matching_results = 0;
    let mut total_comparisons = 0;

    for query in ctx.query_vectors.iter().take(20) {
        let hnsw_results = ctx.hnsw_store.search(query, 5).await?;
        let qdrant_results = ctx.qdrant_store.search(query, 5).await?;

        let hnsw_ids: HashSet<_> = hnsw_results.iter().map(|r| &r.doc_id).collect();
        let qdrant_ids: HashSet<_> = qdrant_results.iter().map(|r| &r.doc_id).collect();

        let overlap = hnsw_ids.intersection(&qdrant_ids).count();
        matching_results += overlap;
        total_comparisons += hnsw_results.len().min(qdrant_results.len());
    }

    let result_similarity = matching_results as f64 / total_comparisons as f64;
    info!("Search result similarity: {:.1}%", result_similarity * 100.0);

    assert!(
        result_similarity >= 0.6,
        "Search result similarity too low: {:.1}%",
        result_similarity * 100.0
    );

    // Phase 9: Final metrics report
    info!("Phase 9: Final migration report");
    let metrics = ctx.metrics.lock().await;
    let total_migration_time = metrics
        .migration_end
        .unwrap()
        .duration_since(metrics.migration_start.unwrap());

    info!("========================================");
    info!("Migration Complete!");
    info!("========================================");
    info!("Total migration time: {:?}", total_migration_time);
    info!("Vectors migrated: {}", metrics.vectors_migrated);
    info!("Search latency before: {:.2}ms", metrics.avg_latency_before_ms);
    info!("Search latency after: {:.2}ms", metrics.avg_latency_after_ms);
    info!(
        "Latency change: {:.1}%",
        ((metrics.avg_latency_after_ms - metrics.avg_latency_before_ms)
            / metrics.avg_latency_before_ms)
            * 100.0
    );
    info!("Consistency rate: {:.1}%", consistency_rate * 100.0);
    info!("Result similarity: {:.1}%", result_similarity * 100.0);

    info!("✅ TEST 1 PASSED");
    Ok(())
}

// ============================================================================
// TEST 2: Dual-Verify Mode
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_2_dual_verify_mode() -> Result<()> {
    info!("========================================");
    info!("TEST 2: Dual-Verify Mode");
    info!("========================================");

    let ctx = MigrationContext::new().await?;

    // Setup: Populate both stores
    info!("Setup: Populating both stores");
    ctx.populate_hnsw().await?;

    let batch_vectors = ctx.test_vectors.clone();
    ctx.qdrant_store.insert_batch(batch_vectors).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Enable dual-verify mode
    info!("Enabling dual-verify mode");
    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::DualVerify).await;
    }

    // Perform searches and verify both stores are queried
    info!("Testing dual-verify searches");
    let mut successful_verifications = 0;

    for query in ctx.query_vectors.iter().take(20) {
        let hybrid = ctx.hybrid_store.lock().await;
        let results = hybrid.search(query, 10).await?;

        if !results.is_empty() {
            successful_verifications += 1;
        }
    }

    info!(
        "Dual-verify searches: {}/20 successful",
        successful_verifications
    );
    assert!(successful_verifications >= 18, "Too many verification failures");

    // Check metrics
    let hybrid = ctx.hybrid_store.lock().await;
    let metrics = hybrid.metrics();
    let consistency_checks = metrics.consistency_checks.load(std::sync::atomic::Ordering::Relaxed);
    info!("Consistency checks performed: {}", consistency_checks);
    assert!(consistency_checks > 0, "No consistency checks were performed");

    info!("✅ TEST 2 PASSED");
    Ok(())
}

// ============================================================================
// TEST 3: Rollback Scenario
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_3_rollback_scenario() -> Result<()> {
    info!("========================================");
    info!("TEST 3: Rollback Scenario");
    info!("========================================");

    let ctx = MigrationContext::new().await?;

    // Setup: Start migration
    info!("Phase 1: Starting migration");
    ctx.populate_hnsw().await?;

    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::DualWrite).await;
    }

    // Partial migration
    info!("Phase 2: Performing partial migration (50%)");
    let half_point = ctx.test_vectors.len() / 2;
    let partial_vectors = ctx.test_vectors[..half_point].to_vec();
    ctx.qdrant_store.insert_batch(partial_vectors).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Simulate issue - rollback to HNSW only
    info!("Phase 3: Simulating issue - rolling back to HNSW");
    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::SingleStore).await;
        let mode = hybrid.mode().await;
        assert_eq!(mode, MigrationMode::SingleStore);
    }

    // Verify HNSW still works correctly
    info!("Phase 4: Verifying HNSW functionality after rollback");
    let test_query = &ctx.query_vectors[0];
    let hybrid = ctx.hybrid_store.lock().await;
    let results = hybrid.search(test_query, 10).await?;

    info!("Rollback search returned {} results", results.len());
    assert!(!results.is_empty(), "HNSW should still work after rollback");

    // Test new insertions go only to HNSW
    info!("Phase 5: Testing post-rollback insertions");
    let new_vectors = MigrationContext::generate_test_vectors(10, EMBEDDING_DIMENSION);

    for (id, vector) in &new_vectors {
        hybrid.insert(id.clone(), vector.clone()).await?;
    }

    let hnsw_final_count = ctx.hnsw_store.len().await;
    info!("HNSW final count: {}", hnsw_final_count);
    assert_eq!(
        hnsw_final_count,
        MIGRATION_DATASET_SIZE + new_vectors.len(),
        "HNSW should contain all vectors"
    );

    info!("✅ TEST 3 PASSED - Rollback successful");
    Ok(())
}

// ============================================================================
// TEST 4: Incremental Migration with Live Traffic
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_4_incremental_migration_with_traffic() -> Result<()> {
    info!("========================================");
    info!("TEST 4: Incremental Migration with Live Traffic");
    info!("========================================");

    let ctx = MigrationContext::new().await?;

    // Setup
    info!("Setup: Initializing with HNSW data");
    ctx.populate_hnsw().await?;

    {
        let hybrid = ctx.hybrid_store.lock().await;
        hybrid.set_mode(MigrationMode::DualWrite).await;
    }

    // Simulate incremental migration with concurrent traffic
    info!("Starting incremental migration with simulated traffic");

    let migration_ctx = Arc::new(ctx);
    let mut handles = vec![];

    // Migration thread
    let migration_handle = {
        let ctx = migration_ctx.clone();
        tokio::spawn(async move {
            info!("Migration thread: Starting batch migration");
            const BATCH_SIZE: usize = 50;

            for (batch_num, chunk) in ctx.test_vectors.chunks(BATCH_SIZE).enumerate() {
                ctx.qdrant_store.insert_batch(chunk.to_vec()).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await; // Simulate slower migration

                if batch_num % 5 == 0 {
                    info!("Migration: Completed batch {}", batch_num);
                }
            }

            info!("Migration thread: Completed");
        })
    };
    handles.push(migration_handle);

    // Concurrent read traffic (5 threads)
    for thread_id in 0..5 {
        let ctx = migration_ctx.clone();
        let handle = tokio::spawn(async move {
            for i in 0..20 {
                let query = &ctx.query_vectors[i % ctx.query_vectors.len()];
                let hybrid = ctx.hybrid_store.lock().await;
                let results = hybrid.search(query, 10).await;

                if results.is_err() || results.as_ref().unwrap().is_empty() {
                    warn!("Thread {}: Search {} returned no results", thread_id, i);
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
    }

    // Concurrent write traffic (2 threads)
    for thread_id in 0..2 {
        let ctx = migration_ctx.clone();
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                let id = format!("live_{}_{}", thread_id, i);
                let vector = MigrationContext::generate_test_vectors(1, EMBEDDING_DIMENSION)[0]
                    .1
                    .clone();

                let hybrid = ctx.hybrid_store.lock().await;
                let _ = hybrid.insert(id, vector).await;

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all operations
    for handle in handles {
        handle.await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify final state
    info!("Verifying final state after concurrent operations");
    let hnsw_count = migration_ctx.hnsw_store.len().await;
    let qdrant_count = migration_ctx.qdrant_store.len().await;

    info!("Final counts: HNSW={}, Qdrant={}", hnsw_count, qdrant_count);

    // Both should have the original data plus new live traffic writes
    assert!(hnsw_count >= MIGRATION_DATASET_SIZE);
    assert!(qdrant_count >= MIGRATION_DATASET_SIZE);

    info!("✅ TEST 4 PASSED");
    Ok(())
}

// ============================================================================
// TEST 5: Performance Regression Detection
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_5_performance_regression_detection() -> Result<()> {
    info!("========================================");
    info!("TEST 5: Performance Regression Detection");
    info!("========================================");

    let ctx = MigrationContext::new().await?;

    // Populate and measure baseline
    ctx.populate_hnsw().await?;

    info!("Measuring baseline performance (HNSW)");
    let baseline_measurements = measure_performance_detailed(
        &(ctx.hnsw_store.clone() as Arc<dyn VectorIndex>),
        &ctx.query_vectors,
    )
    .await?;

    // Migrate to Qdrant
    info!("Migrating to Qdrant");
    ctx.qdrant_store.insert_batch(ctx.test_vectors.clone()).await?;
    tokio::time::sleep(Duration::from_millis(1000)).await;

    info!("Measuring post-migration performance (Qdrant)");
    let post_migration_measurements = measure_performance_detailed(
        &(ctx.qdrant_store.clone() as Arc<dyn VectorIndex>),
        &ctx.query_vectors,
    )
    .await?;

    // Compare and detect regressions
    info!("Performance comparison:");
    info!("  HNSW:");
    info!("    - Avg latency: {:.2}ms", baseline_measurements.avg_latency_ms);
    info!("    - P50 latency: {:.2}ms", baseline_measurements.p50_latency_ms);
    info!("    - P95 latency: {:.2}ms", baseline_measurements.p95_latency_ms);
    info!("    - P99 latency: {:.2}ms", baseline_measurements.p99_latency_ms);

    info!("  Qdrant:");
    info!("    - Avg latency: {:.2}ms", post_migration_measurements.avg_latency_ms);
    info!("    - P50 latency: {:.2}ms", post_migration_measurements.p50_latency_ms);
    info!("    - P95 latency: {:.2}ms", post_migration_measurements.p95_latency_ms);
    info!("    - P99 latency: {:.2}ms", post_migration_measurements.p99_latency_ms);

    // Calculate regression
    let avg_regression = (post_migration_measurements.avg_latency_ms - baseline_measurements.avg_latency_ms)
        / baseline_measurements.avg_latency_ms
        * 100.0;

    info!("Average latency change: {:.1}%", avg_regression);

    // Allow for reasonable variance (within 50% regression is acceptable for migration)
    assert!(
        avg_regression < 50.0,
        "Performance regression too high: {:.1}%",
        avg_regression
    );

    info!("✅ TEST 5 PASSED - No significant regression detected");
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

#[derive(Debug)]
struct PerformanceMeasurements {
    avg_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
}

async fn measure_performance_detailed(
    store: &Arc<dyn VectorIndex>,
    queries: &[Vector],
) -> Result<PerformanceMeasurements> {
    let mut latencies = Vec::new();

    for query in queries {
        let start = Instant::now();
        let _ = store.search(query, 10).await?;
        let latency = start.elapsed().as_micros() as f64 / 1000.0; // Convert to ms
        latencies.push(latency);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg_latency_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p50_latency_ms = latencies[latencies.len() / 2];
    let p95_latency_ms = latencies[latencies.len() * 95 / 100];
    let p99_latency_ms = latencies[latencies.len() * 99 / 100];
    let min_latency_ms = latencies[0];
    let max_latency_ms = latencies[latencies.len() - 1];

    Ok(PerformanceMeasurements {
        avg_latency_ms,
        p50_latency_ms,
        p95_latency_ms,
        p99_latency_ms,
        min_latency_ms,
        max_latency_ms,
    })
}
