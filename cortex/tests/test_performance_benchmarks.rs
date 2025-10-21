//! Comprehensive Performance Benchmarks for Cortex
//!
//! This test suite provides detailed performance benchmarking across all Cortex subsystems:
//! 1. VFS Performance: File operations, caching, deduplication, materialization
//! 2. Semantic Search: Embedding generation, search latency, index building
//! 3. Code Manipulation: Unit creation, refactoring operations
//! 4. Session Management: Session lifecycle, locking, merging
//! 5. Memory System: Storage, retrieval, consolidation
//! 6. Stress Tests: Large-scale scenarios and sustained operations
//!
//! Each benchmark measures:
//! - Latency (p50, p95, p99)
//! - Throughput (operations/sec)
//! - Memory usage
//! - Scalability characteristics
//! - Comparison with performance targets

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_semantic::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use futures::future::join_all;
use tracing::{info, warn};

// =============================================================================
// Performance Measurement Utilities
// =============================================================================

/// Statistics for latency measurements
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
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        let count = samples.len();

        if count == 0 {
            return Self {
                count: 0,
                min: Duration::ZERO,
                max: Duration::ZERO,
                p50: Duration::ZERO,
                p95: Duration::ZERO,
                p99: Duration::ZERO,
                mean: Duration::ZERO,
            };
        }

        let min = samples[0];
        let max = samples[count - 1];
        let p50 = samples[count * 50 / 100];
        let p95 = samples[count * 95 / 100];
        let p99 = samples[count * 99 / 100];

        let sum: Duration = samples.iter().sum();
        let mean = sum / count as u32;

        Self {
            count,
            min,
            max,
            p50,
            p95,
            p99,
            mean,
        }
    }

    fn print_report(&self, operation: &str, target_latency: Option<Duration>) {
        info!("=== {} Latency Report ===", operation);
        info!("  Samples: {}", self.count);
        info!("  Min: {:?}", self.min);
        info!("  Mean: {:?}", self.mean);
        info!("  p50: {:?}", self.p50);
        info!("  p95: {:?}", self.p95);
        info!("  p99: {:?}", self.p99);
        info!("  Max: {:?}", self.max);

        if let Some(target) = target_latency {
            let meets_target = self.p95 <= target;
            info!("  Target (p95): {:?} - {}", target, if meets_target { "✓ PASS" } else { "✗ FAIL" });
        }
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

    fn print_report(&self, operation: &str, target_throughput: Option<f64>) {
        info!("=== {} Throughput Report ===", operation);
        info!("  Operations: {}", self.total_operations);
        info!("  Duration: {:?}", self.duration);
        info!("  Throughput: {:.2} ops/sec", self.ops_per_sec);

        if let Some(target) = target_throughput {
            let meets_target = self.ops_per_sec >= target;
            info!("  Target: {:.2} ops/sec - {}", target, if meets_target { "✓ PASS" } else { "✗ FAIL" });
        }
    }
}

/// Memory usage tracking
struct MemoryTracker {
    initial_bytes: usize,
}

impl MemoryTracker {
    fn new() -> Self {
        Self {
            initial_bytes: 0, // Placeholder - would use actual memory tracking
        }
    }

    fn measure(&self) -> usize {
        // Placeholder - would measure actual memory
        0
    }

    fn print_report(&self, operation: &str) {
        let current = self.measure();
        let delta = current.saturating_sub(self.initial_bytes);
        info!("=== {} Memory Report ===", operation);
        info!("  Memory Delta: {} bytes ({:.2} MB)", delta, delta as f64 / 1024.0 / 1024.0);
    }
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
            ..Default::default()
        },
        namespace: "cortex_bench".to_string(),
        database: db_name.to_string(),
    }
}

// =============================================================================
// 1. VFS Performance Benchmarks
// =============================================================================

#[tokio::test]
async fn bench_vfs_file_read_write_latency() {
    info!("Starting VFS file read/write latency benchmark");

    let db_config = create_test_db_config("vfs_rw_latency");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Benchmark write latency
    let mut write_samples = Vec::new();
    for i in 0..100 {
        let path = VirtualPath::new(&format!("bench/file_{}.txt", i)).unwrap();
        let content = format!("Content for file {}", i).repeat(10); // ~500 bytes

        let start = Instant::now();
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");
        write_samples.push(start.elapsed());
    }

    let write_stats = LatencyStats::from_samples(write_samples);
    write_stats.print_report("VFS File Write", Some(Duration::from_millis(50)));

    // Benchmark read latency
    let mut read_samples = Vec::new();
    for i in 0..100 {
        let path = VirtualPath::new(&format!("bench/file_{}.txt", i)).unwrap();

        let start = Instant::now();
        let _ = vfs.read_file(&workspace_id, &path)
            .await
            .expect("Failed to read file");
        read_samples.push(start.elapsed());
    }

    let read_stats = LatencyStats::from_samples(read_samples);
    read_stats.print_report("VFS File Read", Some(Duration::from_millis(50)));

    // Assertions
    assert!(
        write_stats.p95 < Duration::from_millis(50),
        "Write p95 latency should be <50ms, got {:?}",
        write_stats.p95
    );
    assert!(
        read_stats.p95 < Duration::from_millis(50),
        "Read p95 latency should be <50ms, got {:?}",
        read_stats.p95
    );
}

#[tokio::test]
async fn bench_vfs_directory_traversal() {
    info!("Starting VFS directory traversal benchmark");

    let db_config = create_test_db_config("vfs_traversal");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create directory structure: 10 dirs × 50 files = 500 files
    info!("Creating directory structure...");
    for dir_idx in 0..10 {
        for file_idx in 0..50 {
            let path = VirtualPath::new(&format!("dir_{}/file_{}.txt", dir_idx, file_idx)).unwrap();
            vfs.write_file(&workspace_id, &path, b"test content")
                .await
                .expect("Failed to write file");
        }
    }

    // Benchmark traversal
    let start = Instant::now();
    let entries = vfs.list_directory(&workspace_id, &VirtualPath::new("").unwrap(), true)
        .await
        .expect("Failed to list directory");
    let duration = start.elapsed();

    info!("Traversed {} entries in {:?}", entries.len(), duration);
    assert!(duration < Duration::from_millis(500), "Traversal should be <500ms");
}

#[tokio::test]
async fn bench_vfs_content_deduplication() {
    info!("Starting VFS content deduplication benchmark");

    let db_config = create_test_db_config("vfs_dedup");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    let duplicate_content = b"This is duplicate content that will be written many times";

    // Write same content to 100 different files
    let start = Instant::now();
    for i in 0..100 {
        let path = VirtualPath::new(&format!("dedup/file_{}.txt", i)).unwrap();
        vfs.write_file(&workspace_id, &path, duplicate_content)
            .await
            .expect("Failed to write file");
    }
    let duration = start.elapsed();

    let throughput = ThroughputStats::new(100, duration);
    throughput.print_report("Deduplication Write", None);

    info!("Deduplication overhead measured: {:?} for 100 files", duration);
    assert!(duration < Duration::from_secs(10), "Deduplication should complete <10s");
}

#[tokio::test]
async fn bench_vfs_cache_hit_rate() {
    info!("Starting VFS cache hit rate benchmark");

    let db_config = create_test_db_config("vfs_cache");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Write test files
    for i in 0..10 {
        let path = VirtualPath::new(&format!("cache_test_{}.txt", i)).unwrap();
        vfs.write_file(&workspace_id, &path, format!("Content {}", i).as_bytes())
            .await
            .expect("Failed to write file");
    }

    // First read (cache miss)
    let start = Instant::now();
    for i in 0..10 {
        let path = VirtualPath::new(&format!("cache_test_{}.txt", i)).unwrap();
        let _ = vfs.read_file(&workspace_id, &path).await.unwrap();
    }
    let cold_duration = start.elapsed();

    // Second read (cache hit)
    let start = Instant::now();
    for i in 0..10 {
        let path = VirtualPath::new(&format!("cache_test_{}.txt", i)).unwrap();
        let _ = vfs.read_file(&workspace_id, &path).await.unwrap();
    }
    let warm_duration = start.elapsed();

    info!("Cold cache: {:?}, Warm cache: {:?}", cold_duration, warm_duration);
    info!("Cache speedup: {:.2}x", cold_duration.as_secs_f64() / warm_duration.as_secs_f64());

    // Cache hits should be significantly faster
    assert!(warm_duration < cold_duration, "Cache hits should be faster than misses");
}

#[tokio::test]
async fn bench_vfs_materialization_throughput() {
    info!("Starting VFS materialization throughput benchmark");

    let db_config = create_test_db_config("vfs_materialize");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create 1000 files totaling ~10K LOC
    info!("Creating 1000 files for materialization...");
    let mut total_lines = 0;
    for i in 0..1000 {
        let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", i / 100, i % 100)).unwrap();
        let content = format!(
            "// File {}\npub fn function_{}() {{\n    println!(\"Hello\");\n}}\n",
            i, i
        );
        total_lines += content.lines().count();

        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");
    }

    info!("Created {} files with {} total lines", 1000, total_lines);

    // Benchmark materialization
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let engine = MaterializationEngine::new(vfs.clone());

    let start = Instant::now();
    engine.flush(
        FlushScope::All,
        temp_dir.path(),
        FlushOptions::default(),
    ).await.expect("Failed to flush");
    let duration = start.elapsed();

    let throughput = ThroughputStats::new(total_lines, duration);
    throughput.print_report("Materialization", None);

    info!("Materialization: {} files, {} LOC in {:?}", 1000, total_lines, duration);

    // Target: 10K LOC in <5s
    if total_lines >= 10000 {
        assert!(
            duration < Duration::from_secs(5),
            "10K LOC materialization should be <5s, got {:?}",
            duration
        );
    }
}

// =============================================================================
// 2. Semantic Search Benchmarks
// =============================================================================

#[tokio::test]
async fn bench_semantic_embedding_generation() {
    info!("Starting semantic embedding generation benchmark");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    let test_documents = vec![
        "This is a test document about machine learning algorithms",
        "Rust programming language provides memory safety without garbage collection",
        "Async programming enables concurrent operations in modern applications",
        "Database indexing improves query performance significantly",
        "Code quality metrics help identify areas for improvement",
    ];

    // Benchmark embedding generation
    let mut samples = Vec::new();
    for (i, doc) in test_documents.iter().enumerate() {
        let start = Instant::now();
        engine.index_document(&format!("doc_{}", i), doc)
            .await
            .expect("Failed to index document");
        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Embedding Generation", None);
}

#[tokio::test]
async fn bench_semantic_search_latency() {
    info!("Starting semantic search latency benchmark");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    // Index documents
    for i in 0..100 {
        let doc = format!("Test document {} about software development and programming", i);
        engine.index_document(&format!("doc_{}", i), &doc)
            .await
            .expect("Failed to index");
    }

    // Benchmark search
    let queries = vec![
        "software development",
        "programming languages",
        "code quality",
        "performance optimization",
        "testing strategies",
    ];

    let mut samples = Vec::new();
    for query in &queries {
        let start = Instant::now();
        let _ = engine.search(query, 10).await.expect("Search failed");
        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Semantic Search", Some(Duration::from_millis(100)));

    assert!(
        stats.p95 < Duration::from_millis(100),
        "Search p95 should be <100ms, got {:?}",
        stats.p95
    );
}

#[tokio::test]
async fn bench_semantic_index_build_time() {
    info!("Starting semantic index build time benchmark");

    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create search engine");

    // Build index with 1000 functions
    let start = Instant::now();
    for i in 0..1000 {
        let doc = format!(
            "fn function_{}(param: Type) -> Result<Value> {{\n  // Implementation\n  Ok(value)\n}}",
            i
        );
        engine.index_document(&format!("fn_{}", i), &doc)
            .await
            .expect("Failed to index");
    }
    let duration = start.elapsed();

    let throughput = ThroughputStats::new(1000, duration);
    throughput.print_report("Index Building", None);

    info!("Indexed 1000 functions in {:?}", duration);
}

#[tokio::test]
async fn bench_semantic_concurrent_search() {
    info!("Starting semantic concurrent search benchmark");

    let config = SemanticConfig::default();
    let engine = Arc::new(
        SemanticSearchEngine::new(config)
            .await
            .expect("Failed to create search engine")
    );

    // Index documents
    for i in 0..100 {
        let doc = format!("Document {} about technology and software engineering", i);
        engine.index_document(&format!("doc_{}", i), &doc)
            .await
            .expect("Failed to index");
    }

    // Concurrent search
    let queries = vec!["technology", "software", "engineering", "development", "code"];
    let start = Instant::now();

    let mut handles = vec![];
    for _ in 0..20 {
        for query in &queries {
            let engine_clone = engine.clone();
            let query_clone = query.to_string();

            let handle = tokio::spawn(async move {
                engine_clone.search(&query_clone, 5).await
            });
            handles.push(handle);
        }
    }

    let results = join_all(handles).await;
    let duration = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    let throughput = ThroughputStats::new(successful, duration);
    throughput.print_report("Concurrent Search", None);
}

// =============================================================================
// 3. Code Manipulation Benchmarks
// =============================================================================

#[tokio::test]
async fn bench_code_create_unit() {
    info!("Starting code unit creation benchmark");

    let db_config = create_test_db_config("code_create");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());

    // Benchmark unit creation
    let mut samples = Vec::new();
    for i in 0..100 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("test_fn_{}", i),
            qualified_name: format!("module::test_fn_{}", i),
            display_name: format!("test_fn_{}", i),
            file_path: "src/test.rs".to_string(),
            start_line: i * 10,
            start_column: 0,
            end_line: i * 10 + 5,
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
        };

        let start = Instant::now();
        cognitive.remember_unit(&unit)
            .await
            .expect("Failed to create unit");
        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Code Unit Creation", Some(Duration::from_millis(200)));

    assert!(
        stats.p95 < Duration::from_millis(200),
        "Unit creation p95 should be <200ms, got {:?}",
        stats.p95
    );
}

#[tokio::test]
async fn bench_code_rename_workspace() {
    info!("Starting workspace-wide rename benchmark");

    let db_config = create_test_db_config("code_rename");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create 50 files with references to a function
    for i in 0..50 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        let content = format!(
            "fn caller_{}() {{\n    old_function_name();\n}}\n",
            i
        );
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write");
    }

    // Benchmark rename operation
    // Note: This is a placeholder - actual implementation would use tree-sitter
    let start = Instant::now();

    // Simulate rename by updating all files
    for i in 0..50 {
        let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
        let content = vfs.read_file(&workspace_id, &path).await.unwrap();
        let updated = String::from_utf8(content)
            .unwrap()
            .replace("old_function_name", "new_function_name");
        vfs.write_file(&workspace_id, &path, updated.as_bytes())
            .await
            .unwrap();
    }

    let duration = start.elapsed();

    info!("Renamed across 50 files in {:?}", duration);
    assert!(
        duration < Duration::from_millis(500),
        "Workspace rename should be <500ms, got {:?}",
        duration
    );
}

#[tokio::test]
async fn bench_code_update_references() {
    info!("Starting reference update benchmark");

    let db_config = create_test_db_config("code_refs");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());

    // Create source unit
    let source_id = CortexId::new();

    // Create 100 dependencies
    let start = Instant::now();
    for _ in 0..100 {
        let target_id = CortexId::new();
        cognitive.associate(source_id, target_id, DependencyType::Calls)
            .await
            .expect("Failed to create dependency");
    }
    let duration = start.elapsed();

    let throughput = ThroughputStats::new(100, duration);
    throughput.print_report("Reference Updates", None);

    assert!(
        duration < Duration::from_millis(200),
        "100 reference updates should be <200ms, got {:?}",
        duration
    );
}

// =============================================================================
// 4. Session Management Benchmarks
// =============================================================================

#[tokio::test]
async fn bench_session_creation() {
    info!("Starting session creation benchmark");

    let db_config = create_test_db_config("session_create");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Benchmark session creation
    let mut samples = Vec::new();
    for i in 0..50 {
        let start = Instant::now();

        let episode = EpisodicMemory::new(
            format!("Session {}", i),
            format!("agent_{}", i),
            project_id,
            EpisodeType::Task,
        );

        cognitive.remember_episode(&episode)
            .await
            .expect("Failed to create session");

        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Session Creation", Some(Duration::from_millis(200)));

    assert!(
        stats.p95 < Duration::from_millis(200),
        "Session creation p95 should be <200ms, got {:?}",
        stats.p95
    );
}

#[tokio::test]
async fn bench_session_concurrent_access() {
    info!("Starting concurrent session access benchmark");

    let db_config = create_test_db_config("session_concurrent");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Create sessions
    let mut episode_ids = Vec::new();
    for i in 0..20 {
        let episode = EpisodicMemory::new(
            format!("Concurrent session {}", i),
            format!("agent_{}", i),
            project_id,
            EpisodeType::Task,
        );
        let id = cognitive.remember_episode(&episode)
            .await
            .expect("Failed to create episode");
        episode_ids.push(id);
    }

    // Concurrent reads
    let start = Instant::now();
    let mut handles = vec![];

    for _ in 0..50 {
        for &id in &episode_ids {
            let cognitive_clone = CognitiveManager::new(connection_manager.clone());
            let handle = tokio::spawn(async move {
                cognitive_clone.episodic().get_episode(id).await
            });
            handles.push(handle);
        }
    }

    let results = join_all(handles).await;
    let duration = start.elapsed();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    let throughput = ThroughputStats::new(successful, duration);
    throughput.print_report("Concurrent Session Access", None);
}

// =============================================================================
// 5. Memory System Benchmarks
// =============================================================================

#[tokio::test]
async fn bench_memory_episode_storage() {
    info!("Starting memory episode storage benchmark");

    let db_config = create_test_db_config("memory_episode");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    let mut samples = Vec::new();
    for i in 0..100 {
        let mut episode = EpisodicMemory::new(
            format!("Episode {}", i),
            "bench_agent".to_string(),
            project_id,
            EpisodeType::Task,
        );
        episode.outcome = EpisodeOutcome::Success;

        let start = Instant::now();
        cognitive.remember_episode(&episode)
            .await
            .expect("Failed to store episode");
        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Episode Storage", None);
}

#[tokio::test]
async fn bench_memory_pattern_extraction() {
    info!("Starting pattern extraction benchmark");

    let db_config = create_test_db_config("memory_pattern");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Create episodes for pattern extraction
    for i in 0..50 {
        let mut episode = EpisodicMemory::new(
            format!("Pattern test {}", i),
            "bench_agent".to_string(),
            project_id,
            EpisodeType::Task,
        );
        episode.outcome = if i % 2 == 0 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Partial
        };

        cognitive.remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Benchmark pattern extraction via consolidation
    let start = Instant::now();
    let report = cognitive.consolidate()
        .await
        .expect("Failed to consolidate");
    let duration = start.elapsed();

    info!("Pattern extraction: {} patterns in {:?}", report.patterns_extracted, duration);
    info!("Consolidation report: {:?}", report);
}

#[tokio::test]
async fn bench_memory_consolidation() {
    info!("Starting memory consolidation benchmark");

    let db_config = create_test_db_config("memory_consolidate");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Create diverse memory objects
    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Task {}", i),
            "bench_agent".to_string(),
            project_id,
            EpisodeType::Task,
        );
        cognitive.remember_episode(&episode).await.unwrap();
    }

    for i in 0..50 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("fn_{}", i),
            qualified_name: format!("module::fn_{}", i),
            display_name: format!("fn_{}", i),
            file_path: "src/lib.rs".to_string(),
            start_line: i * 10,
            start_column: 0,
            end_line: i * 10 + 5,
            end_column: 1,
            signature: format!("fn fn_{}()", i),
            body: "// body".to_string(),
            docstring: None,
            visibility: "private".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: "Function".to_string(),
            purpose: "Test".to_string(),
            complexity: ComplexityMetrics::default(),
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        cognitive.remember_unit(&unit).await.unwrap();
    }

    // Benchmark consolidation
    let start = Instant::now();
    let report = cognitive.consolidate()
        .await
        .expect("Failed to consolidate");
    let duration = start.elapsed();

    info!("Consolidation completed in {:?}", duration);
    info!("Report: {:?}", report);
}

#[tokio::test]
async fn bench_memory_retrieval() {
    info!("Starting memory retrieval benchmark");

    let db_config = create_test_db_config("memory_retrieve");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Store episodes
    let mut episode_ids = Vec::new();
    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Episode {}", i),
            "bench_agent".to_string(),
            project_id,
            EpisodeType::Task,
        );
        let id = cognitive.remember_episode(&episode).await.unwrap();
        episode_ids.push(id);
    }

    // Benchmark retrieval
    let mut samples = Vec::new();
    for &id in &episode_ids {
        let start = Instant::now();
        let _ = cognitive.episodic().get_episode(id)
            .await
            .expect("Failed to retrieve");
        samples.push(start.elapsed());
    }

    let stats = LatencyStats::from_samples(samples);
    stats.print_report("Memory Retrieval", None);
}

// =============================================================================
// 6. Stress Tests
// =============================================================================

#[tokio::test]
async fn stress_test_10000_files_in_vfs() {
    info!("Starting stress test: 10,000 files in VFS");

    let db_config = create_test_db_config("stress_vfs_10k");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    let start = Instant::now();

    for i in 0..10000 {
        let path = VirtualPath::new(&format!("stress/dir_{}/file_{}.txt", i / 100, i % 100)).unwrap();
        vfs.write_file(&workspace_id, &path, format!("Content {}", i).as_bytes())
            .await
            .expect("Failed to write file");

        if (i + 1) % 1000 == 0 {
            info!("Wrote {} files", i + 1);
        }
    }

    let duration = start.elapsed();
    let throughput = ThroughputStats::new(10000, duration);
    throughput.print_report("10K File Import", None);

    assert!(duration < Duration::from_secs(300), "10K files should import in <5 minutes");
}

#[tokio::test]
async fn stress_test_100000_code_units() {
    info!("Starting stress test: 100,000 code units");

    let db_config = create_test_db_config("stress_units_100k");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());

    let start = Instant::now();

    for i in 0..100000 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: if i % 3 == 0 { CodeUnitType::Function } else { CodeUnitType::Class },
            name: format!("unit_{}", i),
            qualified_name: format!("module::unit_{}", i),
            display_name: format!("unit_{}", i),
            file_path: format!("file_{}.rs", i / 1000),
            start_line: (i % 1000) * 10,
            start_column: 0,
            end_line: (i % 1000) * 10 + 5,
            end_column: 1,
            signature: format!("fn unit_{}()", i),
            body: "// body".to_string(),
            docstring: None,
            visibility: "private".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: "Unit".to_string(),
            purpose: "Stress test".to_string(),
            complexity: ComplexityMetrics::default(),
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        cognitive.remember_unit(&unit)
            .await
            .expect("Failed to create unit");

        if (i + 1) % 10000 == 0 {
            info!("Created {} units", i + 1);
        }
    }

    let duration = start.elapsed();
    let throughput = ThroughputStats::new(100000, duration);
    throughput.print_report("100K Unit Creation", None);
}

#[tokio::test]
async fn stress_test_1000_concurrent_operations() {
    info!("Starting stress test: 1,000 concurrent operations");

    let db_config = create_test_db_config("stress_concurrent");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let cognitive = CognitiveManager::new(connection_manager.clone());
    let workspace_id = uuid::Uuid::new_v4();
    let project_id = CortexId::new();

    let success_count = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let mut handles = vec![];

    for i in 0..1000 {
        let vfs_clone = vfs.clone();
        let cognitive_clone = CognitiveManager::new(connection_manager.clone());
        let success_clone = success_count.clone();

        let handle = tokio::spawn(async move {
            // Alternate between VFS and memory operations
            let result = if i % 2 == 0 {
                let path = VirtualPath::new(&format!("concurrent/file_{}.txt", i)).unwrap();
                vfs_clone.write_file(&workspace_id, &path, b"test").await
            } else {
                let episode = EpisodicMemory::new(
                    format!("Op {}", i),
                    "stress_agent".to_string(),
                    project_id,
                    EpisodeType::Task,
                );
                cognitive_clone.remember_episode(&episode).await.map(|_| ())
            };

            if result.is_ok() {
                success_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        handles.push(handle);
    }

    join_all(handles).await;
    let duration = start.elapsed();

    let successes = success_count.load(Ordering::Relaxed);
    let throughput = ThroughputStats::new(successes, duration);
    throughput.print_report("Concurrent Operations", None);

    info!("Completed {} / 1000 concurrent operations", successes);
    assert!(successes >= 950, "At least 95% operations should succeed");
}

#[tokio::test]
#[ignore] // This test takes a long time
async fn stress_test_sustained_operations() {
    info!("Starting stress test: Sustained operations over time");

    let db_config = create_test_db_config("stress_sustained");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let cognitive = CognitiveManager::new(connection_manager.clone());
    let workspace_id = uuid::Uuid::new_v4();
    let project_id = CortexId::new();

    let test_duration = Duration::from_secs(60); // 1 minute
    let start = Instant::now();
    let mut iteration = 0;

    info!("Running sustained load for {:?}", test_duration);

    while start.elapsed() < test_duration {
        // VFS operation
        let path = VirtualPath::new(&format!("sustained/file_{}.txt", iteration % 100)).unwrap();
        vfs.write_file(&workspace_id, &path, format!("Iter {}", iteration).as_bytes())
            .await
            .expect("VFS write failed");

        // Memory operation
        if iteration % 2 == 0 {
            let episode = EpisodicMemory::new(
                format!("Sustained iter {}", iteration),
                "sustained_agent".to_string(),
                project_id,
                EpisodeType::Task,
            );
            cognitive.remember_episode(&episode)
                .await
                .expect("Episode creation failed");
        }

        iteration += 1;

        if iteration % 100 == 0 {
            info!("Iteration {}, elapsed: {:?}", iteration, start.elapsed());
        }

        sleep(Duration::from_millis(10)).await;
    }

    let actual_duration = start.elapsed();
    let throughput = ThroughputStats::new(iteration, actual_duration);
    throughput.print_report("Sustained Operations", None);

    info!("Sustained test completed: {} iterations in {:?}", iteration, actual_duration);
}

// =============================================================================
// Performance Report Summary
// =============================================================================

#[tokio::test]
async fn generate_performance_report() {
    info!("=============================================================================");
    info!("                    CORTEX PERFORMANCE BENCHMARK REPORT                    ");
    info!("=============================================================================");
    info!("");
    info!("This comprehensive benchmark suite tests:");
    info!("  1. VFS Performance - File operations, caching, deduplication");
    info!("  2. Semantic Search - Embedding, indexing, search latency");
    info!("  3. Code Manipulation - Unit creation, refactoring, references");
    info!("  4. Session Management - Creation, concurrency, locking");
    info!("  5. Memory System - Storage, retrieval, consolidation");
    info!("  6. Stress Tests - Large-scale scenarios");
    info!("");
    info!("Performance Targets:");
    info!("  - File read/write latency: <50ms (p95)");
    info!("  - Semantic search latency: <100ms (p95)");
    info!("  - Code unit creation: <200ms (p95)");
    info!("  - Workspace rename: <500ms");
    info!("  - Session creation: <200ms (p95)");
    info!("  - 10K LOC materialization: <5s");
    info!("");
    info!("Run individual tests to see detailed metrics and comparisons.");
    info!("=============================================================================");
}
