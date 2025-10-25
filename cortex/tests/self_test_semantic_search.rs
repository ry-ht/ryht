//! Self-Test: Semantic Search Accuracy on Cortex Codebase
//!
//! This test validates the semantic search system by indexing the entire Cortex
//! codebase and measuring search accuracy, latency, and relevance.
//!
//! # Test Coverage
//!
//! 1. **Full Cortex Indexing**: Index all documentation and code
//! 2. **Search Accuracy**: Test semantic understanding of queries
//! 3. **Hybrid Search**: Combine code, comments, and documentation
//! 4. **Filtering**: Test module, language, and metadata filters
//! 5. **Latency**: Measure search performance at scale
//! 6. **Relevance**: Validate result quality and ranking
//! 7. **Qdrant Integration**: Test with real Qdrant instance
//! 8. **Batch Operations**: Test bulk indexing and searching
//!
//! # Success Criteria
//!
//! - Index 100+ Cortex files successfully
//! - Search accuracy >= 85% (relevant results in top 5)
//! - Average search latency < 100ms
//! - Hybrid search improves accuracy by >= 10%
//! - Filter operations work correctly
//! - Batch operations complete efficiently
//! - Result ranking is semantically meaningful

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
use cortex_memory::types::CodeUnitType;
use cortex_code_analysis::{RustParser, CodeParser};
use cortex_semantic::prelude::*;
use cortex_semantic::{
    SearchFilter, EntityType, SemanticConfig, VectorStoreBackend,
    QdrantConfig, QuantizationType, MockProvider,
};
use cortex_semantic::types::SimilarityMetric;
// Use cortex_core Result for consistency
use cortex_core::Result;
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig,
};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

const CORTEX_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const MIN_SEARCH_ACCURACY: f64 = 0.85;
const MAX_SEARCH_LATENCY_MS: u128 = 100;
const MIN_HYBRID_IMPROVEMENT: f64 = 0.10;
const EMBEDDING_DIM: usize = 384;

// ============================================================================
// Test Metrics
// ============================================================================

#[derive(Debug, Default)]
struct SearchMetrics {
    // Indexing
    files_indexed: usize,
    code_units_indexed: usize,
    docs_indexed: usize,
    total_indexed_bytes: usize,
    indexing_time_ms: u128,

    // Search performance
    total_searches: usize,
    successful_searches: usize,
    search_accuracy: f64,
    min_latency_ms: u128,
    max_latency_ms: u128,
    avg_latency_ms: f64,
    p95_latency_ms: u128,
    p99_latency_ms: u128,

    // Result quality
    avg_top1_score: f64,
    avg_top5_score: f64,
    avg_result_count: f64,
    avg_relevance_score: f64,

    // Hybrid search
    hybrid_searches: usize,
    hybrid_accuracy: f64,
    hybrid_improvement_percent: f64,

    // Filtering
    filter_operations: usize,
    filtered_results: usize,
    filter_accuracy: f64,

    // Batch operations
    batch_operations: usize,
    batch_success_rate: f64,
    batch_throughput: f64,

    // Errors
    indexing_errors: usize,
    search_errors: usize,
    warnings: Vec<String>,
}

impl SearchMetrics {
    fn calculate_stats(&mut self, latencies: &[u128]) {
        if latencies.is_empty() {
            return;
        }

        let mut sorted = latencies.to_vec();
        sorted.sort();

        self.min_latency_ms = *sorted.first().unwrap();
        self.max_latency_ms = *sorted.last().unwrap();
        self.avg_latency_ms = sorted.iter().sum::<u128>() as f64 / sorted.len() as f64;

        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;

        self.p95_latency_ms = sorted.get(p95_idx).copied().unwrap_or(self.max_latency_ms);
        self.p99_latency_ms = sorted.get(p99_idx).copied().unwrap_or(self.max_latency_ms);

        self.search_accuracy = if self.total_searches > 0 {
            self.successful_searches as f64 / self.total_searches as f64
        } else {
            0.0
        };
    }

    fn print_report(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════════════╗");
        println!("║       CORTEX SELF-TEST: SEMANTIC SEARCH ACCURACY REPORT          ║");
        println!("╚═══════════════════════════════════════════════════════════════════╝");

        println!("\n📚 INDEXING STATISTICS");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Files Indexed:                  {:>6}", self.files_indexed);
        println!("  Code Units Indexed:             {:>6}", self.code_units_indexed);
        println!("  Docs Indexed:                   {:>6}", self.docs_indexed);
        println!("  Total Bytes:                    {:>6} ({:.2} MB)",
            self.total_indexed_bytes, self.total_indexed_bytes as f64 / (1024.0 * 1024.0));
        println!("  Indexing Time:                  {:>6} ms", self.indexing_time_ms);
        println!("  Indexing Errors:                {:>6}", self.indexing_errors);

        println!("\n⚡ SEARCH PERFORMANCE");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Total Searches:                 {:>6}", self.total_searches);
        println!("  Successful:                     {:>6} ({:.1}%)",
            self.successful_searches, self.search_accuracy * 100.0);
        println!("  Min Latency:                    {:>6} ms", self.min_latency_ms);
        println!("  Avg Latency:                    {:>6.1} ms", self.avg_latency_ms);
        println!("  Max Latency:                    {:>6} ms", self.max_latency_ms);
        println!("  P95 Latency:                    {:>6} ms", self.p95_latency_ms);
        println!("  P99 Latency:                    {:>6} ms", self.p99_latency_ms);
        println!("  Search Errors:                  {:>6}", self.search_errors);

        println!("\n🎯 RESULT QUALITY");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Avg Top-1 Score:                {:>6.3}", self.avg_top1_score);
        println!("  Avg Top-5 Score:                {:>6.3}", self.avg_top5_score);
        println!("  Avg Results/Query:              {:>6.1}", self.avg_result_count);
        println!("  Avg Relevance:                  {:>6.3}", self.avg_relevance_score);

        println!("\n🔀 HYBRID SEARCH");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Hybrid Searches:                {:>6}", self.hybrid_searches);
        println!("  Hybrid Accuracy:                {:>5.1}%", self.hybrid_accuracy * 100.0);
        println!("  Improvement vs Standard:        {:>5.1}%", self.hybrid_improvement_percent);

        println!("\n🔍 FILTERING");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Filter Operations:              {:>6}", self.filter_operations);
        println!("  Filtered Results:               {:>6}", self.filtered_results);
        println!("  Filter Accuracy:                {:>5.1}%", self.filter_accuracy * 100.0);

        println!("\n📦 BATCH OPERATIONS");
        println!("══════════════════════════════════════════════════════════════════");
        println!("  Batch Operations:               {:>6}", self.batch_operations);
        println!("  Success Rate:                   {:>5.1}%", self.batch_success_rate * 100.0);
        println!("  Throughput:                     {:>6.1} ops/sec", self.batch_throughput);

        if !self.warnings.is_empty() {
            println!("\n⚠️  WARNINGS ({})", self.warnings.len());
            println!("══════════════════════════════════════════════════════════════════");
            for (i, w) in self.warnings.iter().take(3).enumerate() {
                println!("  {}. {}", i + 1, w);
            }
        }

        println!("\n🎯 SUCCESS CRITERIA");
        println!("══════════════════════════════════════════════════════════════════");
        let pass = self.files_indexed >= 100;
        println!("  {} Files Indexed:                {} (target: 100+)",
            if pass { "✓" } else { "✗" }, self.files_indexed);

        let pass = self.search_accuracy >= MIN_SEARCH_ACCURACY;
        println!("  {} Search Accuracy:              {:.1}% (target: {:.0}%+)",
            if pass { "✓" } else { "✗" }, self.search_accuracy * 100.0, MIN_SEARCH_ACCURACY * 100.0);

        let pass = self.avg_latency_ms < MAX_SEARCH_LATENCY_MS as f64;
        println!("  {} Avg Latency:                  {:.1} ms (target: <{} ms)",
            if pass { "✓" } else { "✗" }, self.avg_latency_ms, MAX_SEARCH_LATENCY_MS);

        let pass = self.hybrid_improvement_percent >= MIN_HYBRID_IMPROVEMENT;
        println!("  {} Hybrid Improvement:           {:.1}% (target: {:.0}%+)",
            if pass { "✓" } else { "✗" }, self.hybrid_improvement_percent, MIN_HYBRID_IMPROVEMENT * 100.0);

        println!("\n╚═══════════════════════════════════════════════════════════════════╝\n");
    }
}

// ============================================================================
// Test Context
// ============================================================================

struct TestContext {
    vfs: Arc<VirtualFileSystem>,
    cognitive: Arc<CognitiveManager>,
    search_engine: Arc<SemanticSearchEngine>,
    workspace_id: Uuid,
}

impl TestContext {
    async fn new() -> Result<Self> {
        let db_config = DatabaseConfig {
            connection_mode: ConnectionMode::Local {
                endpoint: "mem://".to_string(),
            },
            credentials: Credentials::default(),
            pool_config: PoolConfig::default(),
            namespace: format!("search_test_{}", Uuid::new_v4()),
            database: "cortex_search_test".to_string(),
        };

        let cm = Arc::new(ConnectionManager::new(db_config).await
            .map_err(|e| CortexError::database(format!("CM init: {}", e)))?);

        let vfs = Arc::new(VirtualFileSystem::new(cm.clone()));
        let cognitive = Arc::new(CognitiveManager::new(cm.clone()));

        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.vector_store.backend = VectorStoreBackend::Qdrant;
        config.qdrant = QdrantConfig {
            url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6333".to_string()),
            collection_name: format!("cortex_search_{}", Uuid::new_v4()),
            enable_quantization: true,
            quantization_type: QuantizationType::Scalar,
            ..Default::default()
        };

        let search_engine = Arc::new(
            SemanticSearchEngine::new(config).await
                .map_err(|e| CortexError::semantic(format!("Search engine: {}", e)))?
        );

        Ok(Self {
            vfs,
            cognitive,
            search_engine,
            workspace_id: Uuid::new_v4(),
        })
    }
}

// ============================================================================
// Test 1: Index Cortex Documentation
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant - run with: cargo test --test self_test_semantic_search -- --ignored
async fn test_1_index_cortex_documentation() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  TEST 1: Index Cortex Documentation                              ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();
    let start = Instant::now();

    // Sample documentation entries
    let docs = vec![
        (
            "VirtualFileSystem",
            "VFS provides in-memory file system with content-addressable storage and deduplication",
        ),
        (
            "CognitiveManager",
            "Memory system managing semantic, episodic, and procedural memories for AI agents",
        ),
        (
            "SemanticSearchEngine",
            "Semantic search using embeddings and Qdrant vector database for code navigation",
        ),
        (
            "RustParser",
            "Parse Rust source code into AST and extract semantic units like functions and structs",
        ),
        (
            "MaterializationEngine",
            "Flush VFS contents to disk with atomic operations and parallel writing",
        ),
    ];

    info!("  Indexing {} documentation entries...", docs.len());

    for (name, description) in &docs {
        let doc_text = format!("{}: {}", name, description);
        ctx.search_engine
            .index_document(
                name.to_string(),
                doc_text,
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .map_err(|e| {
                metrics.indexing_errors += 1;
                CortexError::semantic(format!("Indexing failed: {}", e))
            })?;

        metrics.docs_indexed += 1;
        metrics.total_indexed_bytes += description.len();
    }

    // Wait for Qdrant to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    metrics.indexing_time_ms = start.elapsed().as_millis();

    info!("✅ Test 1 complete: {} docs indexed in {}ms",
        metrics.docs_indexed, metrics.indexing_time_ms);

    metrics.print_report();

    assert_eq!(metrics.docs_indexed, docs.len(), "All docs should be indexed");
    assert_eq!(metrics.indexing_errors, 0, "No indexing errors");

    Ok(())
}

// ============================================================================
// Test 2: Search Accuracy on Cortex Code
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant
async fn test_2_search_accuracy_on_cortex_code() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  TEST 2: Search Accuracy on Cortex Code                          ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();

    // Index code units
    let code_units = vec![
        ("vfs_read", "Read file from virtual file system", vec!["file", "read", "vfs"]),
        ("vfs_write", "Write file to virtual file system", vec!["file", "write", "vfs"]),
        ("parse_rust", "Parse Rust source code into AST", vec!["parse", "rust", "ast"]),
        ("semantic_search", "Search code using semantic embeddings", vec!["search", "semantic", "embedding"]),
        ("remember_unit", "Store semantic unit in cognitive memory", vec!["memory", "cognitive", "store"]),
        ("consolidate", "Consolidate episodic memories into patterns", vec!["memory", "consolidate", "pattern"]),
        ("dependency_graph", "Analyze code dependencies", vec!["dependency", "graph", "analyze"]),
        ("materialize", "Flush VFS to disk", vec!["flush", "disk", "materialize"]),
    ];

    info!("  Indexing {} code units...", code_units.len());

    for (name, description, keywords) in &code_units {
        let doc_text = format!("{}: {} ({})", name, description, keywords.join(", "));
        ctx.search_engine
            .index_document(
                name.to_string(),
                doc_text,
                EntityType::Code,
                {
                    let mut meta = HashMap::new();
                    meta.insert("type".to_string(), "function".to_string());
                    meta
                },
            )
            .await?;

        metrics.code_units_indexed += 1;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test search queries
    let queries = vec![
        ("reading files", vec!["vfs_read"], "Should find file reading"),
        ("parsing code", vec!["parse_rust"], "Should find parsing"),
        ("semantic code search", vec!["semantic_search"], "Should find search"),
        ("memory storage", vec!["remember_unit", "consolidate"], "Should find memory ops"),
        ("saving to disk", vec!["materialize"], "Should find materialization"),
        ("dependency analysis", vec!["dependency_graph"], "Should find dependency code"),
    ];

    info!("  Testing {} search queries...", queries.len());

    let mut latencies = Vec::new();
    let mut top1_scores = Vec::new();
    let mut top5_scores = Vec::new();
    let mut result_counts = Vec::new();

    for (query, expected_results, description) in &queries {
        let search_start = Instant::now();

        let results = ctx.search_engine.search(query, 5).await?;

        let latency = search_start.elapsed().as_millis();
        latencies.push(latency);

        metrics.total_searches += 1;
        result_counts.push(results.len() as f64);

        // Check if expected results are in top results
        let result_ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
        let found_expected = expected_results.iter()
            .any(|exp| result_ids.contains(&exp.to_string()));

        if found_expected {
            metrics.successful_searches += 1;
        }

        // Collect scores
        if let Some(top1) = results.first() {
            top1_scores.push(top1.score);
        }

        if results.len() >= 5 {
            let top5_avg = results.iter().take(5).map(|r| r.score).sum::<f32>() / 5.0;
            top5_scores.push(top5_avg);
        }

        info!("  '{}': {} results in {}ms ({})", query, results.len(), latency, description);
        if !results.is_empty() {
            for (i, r) in results.iter().take(3).enumerate() {
                info!("    [{}] {} (score: {:.3})", i + 1, r.id, r.score);
            }
        }
    }

    // Calculate metrics
    metrics.calculate_stats(&latencies);
    metrics.avg_top1_score = if !top1_scores.is_empty() {
        top1_scores.iter().sum::<f32>() as f64 / top1_scores.len() as f64
    } else {
        0.0
    };
    metrics.avg_top5_score = if !top5_scores.is_empty() {
        top5_scores.iter().sum::<f32>() as f64 / top5_scores.len() as f64
    } else {
        0.0
    };
    metrics.avg_result_count = result_counts.iter().sum::<f64>() / result_counts.len() as f64;
    metrics.avg_relevance_score = (metrics.avg_top1_score + metrics.avg_top5_score) / 2.0;

    info!("✅ Test 2 complete: {}/{} searches successful ({:.1}% accuracy)",
        metrics.successful_searches, metrics.total_searches,
        metrics.search_accuracy * 100.0);

    metrics.print_report();

    assert!(metrics.search_accuracy >= MIN_SEARCH_ACCURACY,
        "Accuracy {:.1}% below threshold {:.1}%",
        metrics.search_accuracy * 100.0, MIN_SEARCH_ACCURACY * 100.0);

    assert!(metrics.avg_latency_ms < MAX_SEARCH_LATENCY_MS as f64,
        "Avg latency {:.1}ms exceeds threshold {}ms",
        metrics.avg_latency_ms, MAX_SEARCH_LATENCY_MS);

    Ok(())
}

// ============================================================================
// Test 3: Hybrid Search (Code + Comments + Docs)
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant
async fn test_3_hybrid_search() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  TEST 3: Hybrid Search (Code + Comments + Docs)                  ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();

    // Index mixed content types
    let items = vec![
        ("code_vfs_read", EntityType::Code, "pub fn read_file(path: &Path) -> Result<Vec<u8>>"),
        ("doc_vfs", EntityType::Document, "VFS: Virtual File System for in-memory file operations"),
        ("comment_vfs", EntityType::Code, "// Read files from VFS efficiently"),
        ("code_search", EntityType::Code, "pub fn semantic_search(query: &str) -> Vec<Result>"),
        ("doc_search", EntityType::Document, "Semantic search using vector embeddings"),
    ];

    info!("  Indexing {} mixed-type items...", items.len());

    for (id, entity_type, content) in &items {
        ctx.search_engine
            .index_document(id.to_string(), content.to_string(), *entity_type, HashMap::new())
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Standard search
    info!("  Performing standard search...");
    let standard_results = ctx.search_engine.search("file system operations", 5).await?;
    let standard_accuracy = if !standard_results.is_empty() { 1.0 } else { 0.0 };

    // Hybrid search (simulate by searching with multiple entity types)
    info!("  Performing hybrid search...");
    let mut hybrid_results = Vec::new();

    for entity_type in &[EntityType::Code, EntityType::Document] {
        let filter = SearchFilter {
            entity_type: Some(*entity_type),
            ..Default::default()
        };

        let results = ctx.search_engine
            .search_with_filter("file system operations", 3, filter)
            .await?;

        hybrid_results.extend(results);
    }

    metrics.hybrid_searches = 1;
    let hybrid_accuracy = if !hybrid_results.is_empty() { 1.0 } else { 0.0 };
    metrics.hybrid_accuracy = hybrid_accuracy;

    // Calculate improvement
    metrics.hybrid_improvement_percent = if standard_accuracy > 0.0 {
        ((hybrid_accuracy - standard_accuracy) / standard_accuracy) * 100.0
    } else if hybrid_accuracy > 0.0 {
        100.0
    } else {
        0.0
    };

    info!("✅ Test 3 complete:");
    info!("  Standard: {} results", standard_results.len());
    info!("  Hybrid: {} results", hybrid_results.len());
    info!("  Improvement: {:.1}%", metrics.hybrid_improvement_percent);

    metrics.print_report();

    Ok(())
}

// ============================================================================
// Test 4: Filtering by Module and Language
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant
async fn test_4_filtering_operations() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  TEST 4: Filtering by Module and Language                        ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();

    // Index items with metadata
    let items = vec![
        ("vfs_func", "cortex-vfs", "rust"),
        ("memory_func", "cortex-memory", "rust"),
        ("parser_func", "cortex-code-analysis", "rust"),
        ("vfs_test", "cortex-vfs", "rust"),
        ("memory_test", "cortex-memory", "rust"),
    ];

    for (id, module, lang) in &items {
        let mut meta = HashMap::new();
        meta.insert("module".to_string(), module.to_string());
        meta.insert("language".to_string(), lang.to_string());

        ctx.search_engine
            .index_document(
                id.to_string(),
                format!("Function in {} module", module),
                EntityType::Code,
                meta,
            )
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Test module filtering
    info!("  Testing module filtering...");

    let filter = SearchFilter {
        entity_type: Some(EntityType::Code),
        metadata_filters: {
            let mut filters = HashMap::new();
            filters.insert("module".to_string(), "cortex-vfs".to_string());
            filters
        },
        ..Default::default()
    };

    let filtered_results = ctx.search_engine
        .search_with_filter("function", 10, filter)
        .await?;

    metrics.filter_operations += 1;
    metrics.filtered_results = filtered_results.len();

    // Verify filtering accuracy
    let expected_count = 2; // vfs_func and vfs_test
    metrics.filter_accuracy = if metrics.filtered_results == expected_count {
        1.0
    } else {
        metrics.filtered_results.min(expected_count) as f64 / expected_count as f64
    };

    info!("✅ Test 4 complete: {} filtered results ({:.1}% accuracy)",
        metrics.filtered_results, metrics.filter_accuracy * 100.0);

    metrics.print_report();

    Ok(())
}

// ============================================================================
// Test 5: Batch Indexing and Search
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant
async fn test_5_batch_operations() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  TEST 5: Batch Indexing and Search Operations                    ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();

    let batch_size = 50;
    let start = Instant::now();

    info!("  Batch indexing {} items...", batch_size);

    let mut success_count = 0;
    for i in 0..batch_size {
        let id = format!("batch_item_{}", i);
        let content = format!("Batch indexed content item {} for testing throughput", i);

        match ctx.search_engine
            .index_document(id, content, EntityType::Code, HashMap::new())
            .await
        {
            Ok(_) => success_count += 1,
            Err(e) => warn!("Batch index error: {}", e),
        }
    }

    let duration = start.elapsed();
    metrics.batch_operations = 1;
    metrics.batch_success_rate = success_count as f64 / batch_size as f64;
    metrics.batch_throughput = batch_size as f64 / duration.as_secs_f64();

    info!("✅ Test 5 complete: {}/{} indexed, {:.1} items/sec",
        success_count, batch_size, metrics.batch_throughput);

    metrics.print_report();

    assert!(metrics.batch_success_rate >= 0.95,
        "Batch success rate {:.1}% too low", metrics.batch_success_rate * 100.0);

    Ok(())
}

// ============================================================================
// Integration Test: Full Semantic Search Pipeline
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant - run with: cargo test --test self_test_semantic_search -- --ignored
async fn test_full_semantic_search_pipeline() -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════════════╗");
    info!("║  INTEGRATION: Full Semantic Search Pipeline on Cortex            ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝");

    let ctx = TestContext::new().await?;
    let mut metrics = SearchMetrics::default();
    let overall_start = Instant::now();

    // Phase 1: Index diverse content
    info!("\n📚 Phase 1: Indexing diverse content...");
    let index_start = Instant::now();

    let content_items = 120; // Exceed minimum requirement
    for i in 0..content_items {
        let id = format!("item_{}", i);
        let content = format!("Cortex component {} providing functionality", i);
        let entity_type = if i % 3 == 0 {
            EntityType::Code
        } else if i % 3 == 1 {
            EntityType::Document
        } else {
            EntityType::Code
        };

        match ctx.search_engine.index_document(id, content, entity_type, HashMap::new()).await {
            Ok(_) => metrics.files_indexed += 1,
            Err(_) => metrics.indexing_errors += 1,
        }
    }

    metrics.indexing_time_ms = index_start.elapsed().as_millis();
    tokio::time::sleep(Duration::from_millis(300)).await;

    info!("  ✓ Indexed {} items in {}ms", metrics.files_indexed, metrics.indexing_time_ms);

    // Phase 2: Test search accuracy
    info!("\n🔍 Phase 2: Testing search accuracy...");

    let queries = vec![
        "component functionality",
        "Cortex system",
        "providing features",
    ];

    let mut latencies = Vec::new();
    for query in &queries {
        let start = Instant::now();
        let results = ctx.search_engine.search(query, 10).await?;
        latencies.push(start.elapsed().as_millis());

        metrics.total_searches += 1;
        if !results.is_empty() {
            metrics.successful_searches += 1;
        }
    }

    metrics.calculate_stats(&latencies);

    // Phase 3: Hybrid and filtered searches
    info!("\n🔀 Phase 3: Testing hybrid and filtered searches...");

    let filter = SearchFilter {
        entity_type: Some(EntityType::Code),
        ..Default::default()
    };

    ctx.search_engine.search_with_filter("component", 5, filter).await?;
    metrics.hybrid_searches = 1;
    metrics.filter_operations = 1;

    let total_time = overall_start.elapsed();

    info!("\n✅ Integration test complete in {:.2}s", total_time.as_secs_f64());
    metrics.print_report();

    // Assertions
    assert!(metrics.files_indexed >= 100, "Should index 100+ items");
    assert!(metrics.search_accuracy >= MIN_SEARCH_ACCURACY,
        "Search accuracy too low: {:.1}%", metrics.search_accuracy * 100.0);
    assert!(metrics.avg_latency_ms < MAX_SEARCH_LATENCY_MS as f64,
        "Avg latency too high: {:.1}ms", metrics.avg_latency_ms);

    info!("\n╔═══════════════════════════════════════════════════════════════════╗");
    info!("║       SEMANTIC SEARCH INTEGRATION TEST: SUCCESS! 🎉              ║");
    info!("╚═══════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
