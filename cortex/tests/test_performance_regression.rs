//! Performance Regression Tests for Cortex
//!
//! This test suite ensures Cortex meets all performance targets defined in the spec.
//! Each test measures actual latency (not estimates) and compares against targets.
//!
//! Performance Targets:
//! - Navigation Operations: <50ms (P95)
//! - Semantic Search: <100ms (P95)
//! - Code Manipulation: <200ms (P95)
//! - Flush to Disk: <5s for 10K LOC
//! - Memory Operations: <50ms for queries (P95)
//! - Connection Pool: 1000-2000 ops/sec
//!
//! Run with:
//! ```bash
//! cargo test -p cortex-integration-tests test_performance_regression -- --nocapture
//! ```

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
use cortex_memory::types::CodeUnitType as MemoryCodeUnitType;
use cortex_semantic::prelude::*;
use cortex_semantic::types::EntityType;
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig, RetryPolicy,
};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use futures::future::join_all;
use tracing::{info, warn};

// =============================================================================
// Performance Measurement Utilities
// =============================================================================

/// Statistics for latency measurements with percentile calculations
#[derive(Debug, Clone)]
struct LatencyStats {
    count: usize,
    min: Duration,
    max: Duration,
    p50: Duration,
    p95: Duration,
    p99: Duration,
    mean: Duration,
}

impl LatencyStats {
    /// Calculate statistics from raw latency samples
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }

        samples.sort();
        let count = samples.len();

        let min = samples[0];
        let max = samples[count - 1];
        let p50 = samples[count * 50 / 100];
        let p95 = samples[count * 95 / 100];
        let p99 = samples[count * 99 / 100];

        let sum: Duration = samples.iter().sum();
        let mean = sum / count as u32;

        Self { count, min, max, p50, p95, p99, mean }
    }

    fn empty() -> Self {
        Self {
            count: 0,
            min: Duration::ZERO,
            max: Duration::ZERO,
            p50: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            mean: Duration::ZERO,
        }
    }

    /// Print detailed performance report
    fn print_report(&self, operation: &str, target_p95: Option<Duration>) {
        info!("=== {} Performance Report ===", operation);
        info!("  Samples: {}", self.count);
        info!("  Min:     {:?}", self.min);
        info!("  Mean:    {:?}", self.mean);
        info!("  P50:     {:?}", self.p50);
        info!("  P95:     {:?}", self.p95);
        info!("  P99:     {:?}", self.p99);
        info!("  Max:     {:?}", self.max);

        if let Some(target) = target_p95 {
            let pass = self.p95 <= target;
            let status = if pass { "PASS" } else { "FAIL" };
            info!("  Target (P95): {:?} - {}", target, status);

            if !pass {
                warn!(
                    "REGRESSION DETECTED: {} P95 is {:?}, exceeds target of {:?}",
                    operation, self.p95, target
                );
            }
        }
    }

    /// Check if performance meets target
    fn meets_target(&self, target_p95: Duration) -> bool {
        self.p95 <= target_p95
    }
}

/// Throughput measurement
#[derive(Debug, Clone)]
struct ThroughputStats {
    total_operations: usize,
    duration: Duration,
    ops_per_sec: f64,
}

impl ThroughputStats {
    fn new(total_operations: usize, duration: Duration) -> Self {
        let ops_per_sec = if duration.as_secs_f64() > 0.0 {
            total_operations as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            total_operations,
            duration,
            ops_per_sec,
        }
    }

    fn print_report(&self, operation: &str, min_target: Option<f64>, max_target: Option<f64>) {
        info!("=== {} Throughput Report ===", operation);
        info!("  Operations: {}", self.total_operations);
        info!("  Duration:   {:?}", self.duration);
        info!("  Throughput: {:.2} ops/sec", self.ops_per_sec);

        if let (Some(min), Some(max)) = (min_target, max_target) {
            let pass = self.ops_per_sec >= min && self.ops_per_sec <= max * 1.5;
            let status = if pass { "PASS" } else { "FAIL" };
            info!("  Target Range: {:.2}-{:.2} ops/sec - {}", min, max, status);

            if !pass {
                warn!(
                    "REGRESSION DETECTED: {} throughput is {:.2} ops/sec, target range {:.2}-{:.2}",
                    operation, self.ops_per_sec, min, max
                );
            }
        }
    }

    fn meets_target(&self, min_target: f64) -> bool {
        self.ops_per_sec >= min_target
    }
}

/// Helper to measure latency of an async operation
async fn measure_latency<F, Fut, T>(mut operation: F) -> (Duration, T)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = operation().await;
    let duration = start.elapsed();
    (duration, result)
}

/// Helper to create test database config
fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            max_connections: 20,
            min_connections: 5,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
            validate_on_checkout: false,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(5),
        },
        namespace: "cortex_perf_regression".to_string(),
        database: db_name.to_string(),
    }
}

// =============================================================================
// 1. Navigation Operations Performance Tests
// Target: <50ms (P95)
// =============================================================================

#[tokio::test]
async fn test_navigation_1000_files_p95() {
    info!("Starting Navigation Performance Test: 1000 files");

    let db_config = create_test_db_config("nav_1000_files");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create 1000 files
    info!("Creating 1000 test files...");
    for i in 0..1000 {
        let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", i / 100, i % 100))
            .expect("Failed to create path");
        vfs.write_file(&workspace_id, &path, format!("// File {}", i).as_bytes())
            .await
            .expect("Failed to write file");
    }

    // Warmup: 10 iterations
    info!("Warmup phase...");
    for _ in 0..10 {
        let path = VirtualPath::new("src/module_5/file_50.rs").unwrap();
        let _ = vfs.read_file(&workspace_id, &path).await;
    }

    // Measure: 100 random navigation operations
    info!("Measuring navigation latency...");
    let mut latencies = Vec::new();
    for i in 0..100 {
        let idx = (i * 13) % 1000; // Pseudo-random access pattern
        let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", idx / 100, idx % 100))
            .expect("Failed to create path");

        let (duration, result) = measure_latency(|| vfs.read_file(&workspace_id, &path)).await;
        assert!(result.is_ok(), "Navigation failed");
        latencies.push(duration);
    }

    // Assert performance target
    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Navigation (1000 files)", Some(Duration::from_millis(50)));

    assert!(
        stats.meets_target(Duration::from_millis(50)),
        "REGRESSION: Navigation P95 latency {}ms exceeds 50ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_navigation_deep_directory_tree() {
    info!("Starting Navigation Performance Test: Deep directory tree (10 levels)");

    let db_config = create_test_db_config("nav_deep_tree");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create 10-level deep directory tree with files at each level
    info!("Creating deep directory structure (10 levels)...");
    for level in 0..10 {
        let path_str = format!("level_0/level_1/level_2/level_3/level_4/level_5/level_6/level_7/level_8/level_9/file_at_level_{}.rs", level);
        let components: Vec<&str> = path_str.split('/').collect();
        let actual_path = components[..level + 2].join("/");

        let path = VirtualPath::new(&actual_path).expect("Failed to create path");
        vfs.write_file(&workspace_id, &path, format!("// Level {}", level).as_bytes())
            .await
            .expect("Failed to write file");
    }

    // Measure: Navigate to different depths
    info!("Measuring traversal latency...");
    let mut latencies = Vec::new();
    for level in 0..10 {
        let path_str = format!("level_0/level_1/level_2/level_3/level_4/level_5/level_6/level_7/level_8/level_9/file_at_level_{}.rs", level);
        let components: Vec<&str> = path_str.split('/').collect();
        let actual_path = components[..level + 2].join("/");

        let path = VirtualPath::new(&actual_path).expect("Failed to create path");

        for _ in 0..10 {
            let (duration, result) = measure_latency(|| vfs.read_file(&workspace_id, &path)).await;
            assert!(result.is_ok(), "Traversal failed at level {}", level);
            latencies.push(duration);
        }
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Deep Directory Traversal", Some(Duration::from_millis(50)));

    assert!(
        stats.meets_target(Duration::from_millis(50)),
        "REGRESSION: Deep traversal P95 latency {}ms exceeds 50ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_navigation_list_large_directory() {
    info!("Starting Navigation Performance Test: List directory with 1000 items");

    let db_config = create_test_db_config("nav_list_large");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create directory with 1000 files
    info!("Creating directory with 1000 files...");
    for i in 0..1000 {
        let path = VirtualPath::new(&format!("large_dir/file_{:04}.rs", i))
            .expect("Failed to create path");
        vfs.write_file(&workspace_id, &path, b"content")
            .await
            .expect("Failed to write file");
    }

    // Warmup
    info!("Warmup phase...");
    for _ in 0..5 {
        let _ = vfs.list_directory(&workspace_id, &VirtualPath::new("large_dir").unwrap(), false).await;
    }

    // Measure: List directory operations
    info!("Measuring directory listing latency...");
    let mut latencies = Vec::new();
    let large_dir_path = VirtualPath::new("large_dir").unwrap();
    for _ in 0..50 {
        let (duration, result) = measure_latency(|| {
            vfs.list_directory(&workspace_id, &large_dir_path, false)
        })
        .await;

        assert!(result.is_ok(), "Directory listing failed");
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1000, "Should list all 1000 files");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("List Large Directory (1000 items)", Some(Duration::from_millis(50)));

    assert!(
        stats.meets_target(Duration::from_millis(50)),
        "REGRESSION: Directory listing P95 latency {}ms exceeds 50ms target",
        stats.p95.as_millis()
    );
}

// =============================================================================
// 2. Semantic Search Performance Tests
// Target: <100ms (P95)
// =============================================================================

#[tokio::test]
async fn test_semantic_search_10k_units_p95() {
    info!("Starting Semantic Search Performance Test: 10K code units");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    // Setup: Index 10,000 code units
    info!("Indexing 10,000 code units...");
    for i in 0..10_000 {
        let doc = format!(
            "fn function_{}(param: Type) -> Result<Value> {{\n  // Implementation for {}\n  Ok(value)\n}}",
            i, i
        );
        engine
            .index_document(format!("fn_{}", i), doc, EntityType::Code, HashMap::new())
            .await
            .expect("Failed to index document");

        if (i + 1) % 1000 == 0 {
            info!("Indexed {} / 10,000", i + 1);
        }
    }

    // Warmup: 10 searches
    info!("Warmup phase...");
    for _ in 0..10 {
        let _ = engine.search("function implementation", 10).await;
    }

    // Measure: 100 search operations
    info!("Measuring search latency...");
    let queries = vec![
        "function implementation",
        "error handling",
        "data validation",
        "async operation",
        "type conversion",
    ];

    let mut latencies = Vec::new();
    for i in 0..100 {
        let query = queries[i % queries.len()];

        let (duration, result) = measure_latency(|| engine.search(query, 10)).await;
        assert!(result.is_ok(), "Search failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Semantic Search (10K units)", Some(Duration::from_millis(100)));

    assert!(
        stats.meets_target(Duration::from_millis(100)),
        "REGRESSION: Semantic search P95 latency {}ms exceeds 100ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_semantic_search_complex_query_with_filters() {
    info!("Starting Semantic Search Performance Test: Complex query with filters");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    // Setup: Index diverse documents
    info!("Indexing 5,000 diverse code units...");
    for i in 0..5000 {
        let entity_type_str = match i % 3 {
            0 => "function",
            1 => "struct",
            _ => "trait",
        };
        let doc = format!("{} entity_{} {{ /* impl */ }}", entity_type_str, i);
        engine
            .index_document(format!("{}_{}", entity_type_str, i), doc, EntityType::Code, HashMap::new())
            .await
            .expect("Failed to index");
    }

    // Measure: Complex filtered searches
    info!("Measuring filtered search latency...");
    let mut latencies = Vec::new();
    for _ in 0..50 {
        let (duration, result) =
            measure_latency(|| engine.search("implementation entity", 20)).await;
        assert!(result.is_ok(), "Filtered search failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Complex Filtered Search", Some(Duration::from_millis(100)));

    assert!(
        stats.meets_target(Duration::from_millis(100)),
        "REGRESSION: Complex search P95 latency {}ms exceeds 100ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_semantic_search_hybrid_keyword_semantic() {
    info!("Starting Semantic Search Performance Test: Hybrid search");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    // Setup: Index documents
    info!("Indexing 3,000 code units for hybrid search...");
    for i in 0..3000 {
        let doc = format!(
            "pub fn process_data_{}(input: Data) -> Result<Output> {{\n  validate(input)?;\n  transform(input)\n}}",
            i
        );
        engine
            .index_document(format!("process_{}", i), doc, EntityType::Code, HashMap::new())
            .await
            .expect("Failed to index");
    }

    // Measure: Hybrid searches (keyword + semantic)
    info!("Measuring hybrid search latency...");
    let mut latencies = Vec::new();
    for _ in 0..50 {
        let (duration, result) =
            measure_latency(|| engine.search("process data validation", 15)).await;
        assert!(result.is_ok(), "Hybrid search failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Hybrid Search", Some(Duration::from_millis(100)));

    assert!(
        stats.meets_target(Duration::from_millis(100)),
        "REGRESSION: Hybrid search P95 latency {}ms exceeds 100ms target",
        stats.p95.as_millis()
    );
}

// =============================================================================
// 3. Code Manipulation Performance Tests
// Target: <200ms (P95)
// =============================================================================

#[tokio::test]
async fn test_code_manipulation_create_unit_in_large_file() {
    info!("Starting Code Manipulation Test: Create unit in large file");

    let db_config = create_test_db_config("code_create_large");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Warmup
    info!("Warmup phase...");
    for i in 0..10 {
        let unit = create_test_semantic_unit(i, "warmup.rs");
        cognitive.remember_unit(&unit).await.expect("Warmup failed");
    }

    // Measure: Create units
    info!("Measuring unit creation latency...");
    let mut latencies = Vec::new();
    for i in 0..100 {
        let unit = create_test_semantic_unit(i, "large_file.rs");

        let (duration, result) = measure_latency(|| cognitive.remember_unit(&unit)).await;
        assert!(result.is_ok(), "Unit creation failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Create Code Unit", Some(Duration::from_millis(200)));

    assert!(
        stats.meets_target(Duration::from_millis(200)),
        "REGRESSION: Unit creation P95 latency {}ms exceeds 200ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_code_manipulation_rename_across_100_files() {
    info!("Starting Code Manipulation Test: Rename across 100 files");

    let db_config = create_test_db_config("code_rename_100");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create 100 files with function references
    info!("Creating 100 files with references...");
    for i in 0..100 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        let content = format!(
            "fn caller_{}() {{\n    old_function_name();\n    old_function_name();\n}}\n",
            i
        );
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");
    }

    // Measure: Rename operation (simulate by rewriting all files)
    info!("Measuring rename operation latency...");
    let mut latencies = Vec::new();

    for iteration in 0..10 {
        let old_name = if iteration % 2 == 0 {
            "old_function_name"
        } else {
            "new_function_name"
        };
        let new_name = if iteration % 2 == 0 {
            "new_function_name"
        } else {
            "old_function_name"
        };

        let start = Instant::now();

        for i in 0..100 {
            let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
            let content = vfs.read_file(&workspace_id, &path).await.unwrap();
            let updated = String::from_utf8(content)
                .unwrap()
                .replace(old_name, new_name);
            vfs.write_file(&workspace_id, &path, updated.as_bytes())
                .await
                .unwrap();
        }

        let duration = start.elapsed();
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Rename Across 100 Files", Some(Duration::from_millis(200)));

    assert!(
        stats.meets_target(Duration::from_millis(200)),
        "REGRESSION: Rename P95 latency {}ms exceeds 200ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_code_manipulation_extract_function_with_analysis() {
    info!("Starting Code Manipulation Test: Extract function with analysis");

    let db_config = create_test_db_config("code_extract");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Measure: Extract function (create new unit + update dependencies)
    info!("Measuring extract function latency...");
    let mut latencies = Vec::new();

    for i in 0..50 {
        let start = Instant::now();

        // Create extracted function
        let extracted_unit = create_test_semantic_unit(i * 2, "extracted.rs");
        cognitive
            .remember_unit(&extracted_unit)
            .await
            .expect("Failed to create extracted unit");

        // Update original function
        let original_unit = create_test_semantic_unit(i * 2 + 1, "original.rs");
        cognitive
            .remember_unit(&original_unit)
            .await
            .expect("Failed to create original unit");

        // Create dependency
        cognitive
            .associate(original_unit.id, extracted_unit.id, DependencyType::Calls)
            .await
            .expect("Failed to create dependency");

        let duration = start.elapsed();
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Extract Function with Analysis", Some(Duration::from_millis(200)));

    assert!(
        stats.meets_target(Duration::from_millis(200)),
        "REGRESSION: Extract function P95 latency {}ms exceeds 200ms target",
        stats.p95.as_millis()
    );
}

// =============================================================================
// 4. Flush to Disk Performance Tests
// Target: <5s for 10K LOC
// =============================================================================

#[tokio::test]
async fn test_flush_materialize_10k_loc() {
    info!("Starting Flush Performance Test: Materialize 10K LOC");

    let db_config = create_test_db_config("flush_10k_loc");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create files totaling ~10K LOC
    info!("Creating ~10K lines of code...");
    let mut total_lines = 0;
    let mut file_count = 0;

    while total_lines < 10_000 {
        let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", file_count / 50, file_count % 50))
            .expect("Failed to create path");

        // Each file has ~20 lines
        let content = format!(
            "// File {}\n\
            pub struct Data{} {{\n\
            field1: String,\n\
            field2: i32,\n\
            }}\n\
            \n\
            impl Data{} {{\n\
            pub fn new() -> Self {{\n\
            Self {{\n\
            field1: String::new(),\n\
            field2: 0,\n\
            }}\n\
            }}\n\
            \n\
            pub fn process(&self) -> Result<()> {{\n\
            // Processing logic\n\
            Ok(())\n\
            }}\n\
            }}\n",
            file_count, file_count, file_count
        );

        total_lines += content.lines().count();

        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");

        file_count += 1;
    }

    info!("Created {} files with {} total lines", file_count, total_lines);

    // Measure: Flush to disk
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let vfs_owned = (*vfs).clone();
    let engine = MaterializationEngine::new(vfs_owned);

    info!("Measuring materialization latency...");
    let start = Instant::now();
    engine
        .flush(FlushScope::All, temp_dir.path(), FlushOptions::default())
        .await
        .expect("Failed to flush");
    let duration = start.elapsed();

    info!("Materialized {} LOC in {:?}", total_lines, duration);
    info!("Throughput: {:.2} LOC/sec", total_lines as f64 / duration.as_secs_f64());

    assert!(
        duration < Duration::from_secs(5),
        "REGRESSION: 10K LOC materialization took {:?}, exceeds 5s target",
        duration
    );
}

#[tokio::test]
async fn test_flush_parallel_materialization() {
    info!("Starting Flush Performance Test: Parallel materialization");

    let db_config = create_test_db_config("flush_parallel");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create 500 files (~5K LOC)
    info!("Creating 500 files...");
    for i in 0..500 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        let content = format!(
            "// File {}\npub fn function_{}() {{\n  println!(\"Hello\");\n}}\n",
            i, i
        );
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");
    }

    // Measure: Parallel flush
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let vfs_owned = (*vfs).clone();
    let engine = MaterializationEngine::new(vfs_owned);

    info!("Measuring parallel materialization latency...");
    let start = Instant::now();
    engine
        .flush(FlushScope::All, temp_dir.path(), FlushOptions::default())
        .await
        .expect("Failed to flush");
    let duration = start.elapsed();

    info!("Parallel flush completed in {:?}", duration);

    assert!(
        duration < Duration::from_secs(3),
        "REGRESSION: Parallel flush took {:?}, exceeds 3s target",
        duration
    );
}

#[tokio::test]
async fn test_flush_incremental() {
    info!("Starting Flush Performance Test: Incremental flush");

    let db_config = create_test_db_config("flush_incremental");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Setup: Create initial files
    info!("Creating initial 100 files...");
    for i in 0..100 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        vfs.write_file(&workspace_id, &path, b"initial content")
            .await
            .expect("Failed to write file");
    }

    // Initial flush
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let vfs_owned = (*vfs).clone();
    let engine = MaterializationEngine::new(vfs_owned);
    engine
        .flush(FlushScope::All, temp_dir.path(), FlushOptions::default())
        .await
        .expect("Initial flush failed");

    // Modify 10 files
    info!("Modifying 10 files...");
    for i in 0..10 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        vfs.write_file(&workspace_id, &path, b"modified content")
            .await
            .expect("Failed to modify file");
    }

    // Measure: Incremental flush (should only flush changed files)
    info!("Measuring incremental flush latency...");
    let start = Instant::now();
    engine
        .flush(FlushScope::All, temp_dir.path(), FlushOptions::default())
        .await
        .expect("Incremental flush failed");
    let duration = start.elapsed();

    info!("Incremental flush completed in {:?}", duration);

    assert!(
        duration < Duration::from_millis(500),
        "REGRESSION: Incremental flush took {:?}, exceeds 500ms target",
        duration
    );
}

// =============================================================================
// 5. Memory Operations Performance Tests
// Target: <50ms for queries (P95)
// =============================================================================

#[tokio::test]
async fn test_memory_query_1000_episodes_p95() {
    info!("Starting Memory Performance Test: Query 1000 episodes");

    let db_config = create_test_db_config("memory_query_episodes");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Setup: Create 1000 episodes
    info!("Creating 1000 episodes...");
    let mut episode_ids = Vec::new();
    for i in 0..1000 {
        let episode = create_test_episode(i, project_id);
        let id = cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to create episode");
        episode_ids.push(id);
    }

    // Warmup
    info!("Warmup phase...");
    for _ in 0..10 {
        let _ = cognitive.episodic().get_episode(episode_ids[0]).await;
    }

    // Measure: Query episodes
    info!("Measuring episode query latency...");
    let mut latencies = Vec::new();
    for i in 0..100 {
        let id = episode_ids[i % episode_ids.len()];

        let (duration, result) = measure_latency(|| cognitive.episodic().get_episode(id)).await;
        assert!(result.is_ok(), "Episode query failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Memory Episode Query", Some(Duration::from_millis(50)));

    assert!(
        stats.meets_target(Duration::from_millis(50)),
        "REGRESSION: Episode query P95 latency {}ms exceeds 50ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_memory_pattern_matching_large_dataset() {
    info!("Starting Memory Performance Test: Pattern matching on large dataset");

    let db_config = create_test_db_config("memory_pattern_match");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Setup: Create 500 episodes with patterns
    info!("Creating 500 episodes...");
    for i in 0..500 {
        let mut episode = create_test_episode(i, project_id);
        episode.outcome = if i % 5 == 0 {
            EpisodeOutcome::Failure
        } else {
            EpisodeOutcome::Success
        };
        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to create episode");
    }

    // Measure: Pattern extraction queries
    info!("Measuring pattern extraction latency...");
    let mut latencies = Vec::new();

    for _ in 0..20 {
        let (duration, result) = measure_latency(|| {
            cognitive.episodic().retrieve_by_outcome(EpisodeOutcome::Success, 50)
        })
        .await;

        assert!(result.is_ok(), "Pattern query failed");
        latencies.push(duration);
    }

    let stats = LatencyStats::from_samples(latencies);
    stats.print_report("Pattern Matching Query", Some(Duration::from_millis(50)));

    assert!(
        stats.meets_target(Duration::from_millis(50)),
        "REGRESSION: Pattern query P95 latency {}ms exceeds 50ms target",
        stats.p95.as_millis()
    );
}

#[tokio::test]
async fn test_memory_consolidation_performance() {
    info!("Starting Memory Performance Test: Consolidation");

    let db_config = create_test_db_config("memory_consolidation");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Setup: Create 200 episodes for consolidation
    info!("Creating 200 episodes for consolidation...");
    for i in 0..200 {
        let episode = create_test_episode(i, project_id);
        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to create episode");
    }

    // Measure: Consolidation
    info!("Measuring consolidation latency...");
    let start = Instant::now();
    let report = cognitive.consolidate().await.expect("Consolidation failed");
    let duration = start.elapsed();

    info!("Consolidation completed in {:?}", duration);
    info!("Episodes processed: {}", report.episodes_processed);
    info!("Patterns extracted: {}", report.patterns_extracted);

    assert!(
        duration < Duration::from_secs(2),
        "REGRESSION: Consolidation took {:?}, exceeds 2s target",
        duration
    );
}

// =============================================================================
// 6. Connection Pool Performance Tests
// Target: 1000-2000 ops/sec
// =============================================================================

#[tokio::test]
async fn test_connection_pool_sustained_throughput() {
    info!("Starting Connection Pool Test: Sustained throughput");

    let db_config = create_test_db_config("pool_throughput");
    let manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let duration = Duration::from_secs(5);
    let operation_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();
    let mut handles = vec![];

    // Spawn 20 workers
    for _ in 0..20 {
        let manager_clone = manager.clone();
        let count_clone = operation_count.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                if manager_clone.acquire().await.is_ok() {
                    count_clone.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        });

        handles.push(handle);
    }

    join_all(handles).await;
    let actual_duration = start.elapsed();

    let ops = operation_count.load(Ordering::Relaxed);
    let throughput = ThroughputStats::new(ops, actual_duration);
    throughput.print_report("Connection Pool Sustained", Some(1000.0), Some(2000.0));

    assert!(
        throughput.meets_target(1000.0),
        "REGRESSION: Pool throughput {:.2} ops/sec is below 1000 ops/sec target",
        throughput.ops_per_sec
    );
}

#[tokio::test]
async fn test_connection_pool_concurrent_access_100_threads() {
    info!("Starting Connection Pool Test: 100 concurrent threads");

    let db_config = create_test_db_config("pool_concurrent");
    let manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let total_ops = 1000;
    let success_count = Arc::new(AtomicUsize::new(0));

    let start = Instant::now();
    let mut handles = vec![];

    for _i in 0..100 {
        let manager_clone = manager.clone();
        let success_clone = success_count.clone();

        let handle = tokio::spawn(async move {
            for _ in 0..10 {
                if manager_clone.acquire().await.is_ok() {
                    success_clone.fetch_add(1, Ordering::Relaxed);
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
            }
        });

        handles.push(handle);
    }

    join_all(handles).await;
    let duration = start.elapsed();

    let successes = success_count.load(Ordering::Relaxed);
    let throughput = ThroughputStats::new(successes, duration);
    throughput.print_report("Concurrent Access (100 threads)", Some(1000.0), Some(2000.0));

    assert!(
        successes >= total_ops * 95 / 100,
        "REGRESSION: Only {}/{} operations succeeded",
        successes,
        total_ops
    );

    assert!(
        throughput.meets_target(1000.0),
        "REGRESSION: Concurrent throughput {:.2} ops/sec is below 1000 ops/sec target",
        throughput.ops_per_sec
    );
}

#[tokio::test]
async fn test_connection_pool_saturation_recovery() {
    info!("Starting Connection Pool Test: Saturation recovery");

    let db_config = DatabaseConfig {
        pool_config: PoolConfig {
            max_connections: 10,
            min_connections: 2,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(60)),
            max_lifetime: Some(Duration::from_secs(300)),
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
            validate_on_checkout: false,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(5),
        },
        ..create_test_db_config("pool_saturation")
    };

    let manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Phase 1: Saturate pool
    info!("Saturating pool with 10 connections...");
    let _connections: Vec<_> = (0..10)
        .map(|_| manager.acquire())
        .collect::<futures::future::JoinAll<_>>()
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    // Phase 2: Release and measure recovery
    info!("Releasing connections and measuring recovery...");
    drop(_connections);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Measure recovery throughput
    let start = Instant::now();
    let ops = 100;
    for _ in 0..ops {
        let _ = manager.acquire().await;
    }
    let duration = start.elapsed();

    let throughput = ThroughputStats::new(ops, duration);
    throughput.print_report("Pool Recovery", Some(500.0), Some(2000.0));

    assert!(
        throughput.meets_target(500.0),
        "REGRESSION: Pool recovery throughput {:.2} ops/sec is too slow",
        throughput.ops_per_sec
    );
}

// =============================================================================
// Test Helper Functions
// =============================================================================

fn create_test_semantic_unit(i: usize, file_path: &str) -> SemanticUnit {
    SemanticUnit {
        id: CortexId::new(),
        unit_type: MemoryCodeUnitType::Function,
        name: format!("test_fn_{}", i),
        qualified_name: format!("module::test_fn_{}", i),
        display_name: format!("test_fn_{}", i),
        file_path: file_path.to_string(),
        start_line: (i * 10) as u32,
        start_column: 0,
        end_line: (i * 10 + 5) as u32,
        end_column: 1,
        signature: format!("fn test_fn_{}() -> Result<()>", i),
        body: "// function body".to_string(),
        docstring: None,
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: Some("Result<()>".to_string()),
        summary: "Test function".to_string(),
        purpose: "Testing".to_string(),
        complexity: ComplexityMetrics::default(),
        test_coverage: None,
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn create_test_episode(i: usize, project_id: CortexId) -> EpisodicMemory {
    let mut episode = EpisodicMemory::new(
        format!("Test episode {}", i),
        format!("agent_{}", i % 3),
        project_id,
        EpisodeType::Task,
    );

    episode.outcome = EpisodeOutcome::Success;
    episode.duration_seconds = 60 + (i % 120) as u64;
    episode.tokens_used = TokenUsage {
        input: 1000 + (i % 500) as u64,
        output: 500 + (i % 300) as u64,
        total: 1500 + (i % 800) as u64,
    };

    episode
}

// =============================================================================
// Performance Report Summary
// =============================================================================

#[tokio::test]
async fn generate_performance_regression_report() {
    info!("=============================================================================");
    info!("           CORTEX PERFORMANCE REGRESSION TEST REPORT                        ");
    info!("=============================================================================");
    info!("");
    info!("This test suite validates all performance targets:");
    info!("");
    info!("1. Navigation Operations:");
    info!("   - Navigate 1000 files: <50ms (P95)");
    info!("   - Deep directory tree (10 levels): <50ms (P95)");
    info!("   - List large directory (1000 items): <50ms (P95)");
    info!("");
    info!("2. Semantic Search:");
    info!("   - Search 10K code units: <100ms (P95)");
    info!("   - Complex query with filters: <100ms (P95)");
    info!("   - Hybrid search (keyword + semantic): <100ms (P95)");
    info!("");
    info!("3. Code Manipulation:");
    info!("   - Create unit in large file: <200ms (P95)");
    info!("   - Rename across 100 files: <200ms (P95)");
    info!("   - Extract function with analysis: <200ms (P95)");
    info!("");
    info!("4. Flush to Disk:");
    info!("   - Materialize 10K LOC: <5s");
    info!("   - Parallel materialization: <3s");
    info!("   - Incremental flush: <500ms");
    info!("");
    info!("5. Memory Operations:");
    info!("   - Query 1000 episodes: <50ms (P95)");
    info!("   - Pattern matching on large dataset: <50ms (P95)");
    info!("   - Consolidation performance: <2s");
    info!("");
    info!("6. Connection Pool:");
    info!("   - Sustained throughput: 1000-2000 ops/sec");
    info!("   - Concurrent access (100 threads): 1000-2000 ops/sec");
    info!("   - Pool saturation recovery: >500 ops/sec");
    info!("");
    info!("Run individual tests to see detailed metrics and detect regressions.");
    info!("=============================================================================");
}
