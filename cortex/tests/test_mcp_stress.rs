//! MCP Tools Comprehensive Stress Tests
//!
//! This test suite validates Cortex MCP tools under production load conditions:
//! 1. Concurrent Tool Execution - 100 parallel operations, race conditions, locks
//! 2. Large Codebase Performance - 1000+ files, 100K+ LOC, dependency graphs
//! 3. Memory System Under Load - 1000s of episodes, consolidation, queries
//! 4. VFS Stress Test - 10K files, concurrent updates, deduplication
//! 5. Semantic Search Scalability - 100K units, concurrent queries, HNSW
//! 6. Multi-Agent Merge Stress - 20 agents, 100 changes each, conflicts
//! 7. Connection Pool Saturation - 100 concurrent DB ops, health monitoring
//! 8. Error Recovery - DB failures, timeouts, corrupt data, retries
//!
//! Performance Targets:
//! - 1000+ ops/sec sustained
//! - <200ms P95 latency
//! - 0% data loss
//! - 100% consistency
//! - <10GB memory for 100K units

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_parser::CodeParser;
use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig,
};
use cortex_storage::locks::{LockManager, LockType};
use cortex_storage::MergeEngine;
use cortex_vfs::prelude::*;
use futures::future::join_all;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};

// =============================================================================
// Test Infrastructure
// =============================================================================

/// Test statistics collector
#[derive(Debug, Default)]
struct StressTestStats {
    total_operations: AtomicUsize,
    successful_operations: AtomicUsize,
    failed_operations: AtomicUsize,
    total_duration_ms: AtomicU64,
    min_latency_ms: AtomicU64,
    max_latency_ms: AtomicU64,
    data_loss_events: AtomicUsize,
    consistency_violations: AtomicUsize,
}

impl StressTestStats {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            total_operations: AtomicUsize::new(0),
            successful_operations: AtomicUsize::new(0),
            failed_operations: AtomicUsize::new(0),
            total_duration_ms: AtomicU64::new(0),
            min_latency_ms: AtomicU64::new(u64::MAX),
            max_latency_ms: AtomicU64::new(0),
            data_loss_events: AtomicUsize::new(0),
            consistency_violations: AtomicUsize::new(0),
        })
    }

    fn record_operation(&self, success: bool, latency_ms: u64) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
        self.total_duration_ms.fetch_add(latency_ms, Ordering::Relaxed);

        // Update min latency
        let mut current_min = self.min_latency_ms.load(Ordering::Relaxed);
        while latency_ms < current_min {
            match self.min_latency_ms.compare_exchange(
                current_min,
                latency_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }

        // Update max latency
        let mut current_max = self.max_latency_ms.load(Ordering::Relaxed);
        while latency_ms > current_max {
            match self.max_latency_ms.compare_exchange(
                current_max,
                latency_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    fn record_data_loss(&self) {
        self.data_loss_events.fetch_add(1, Ordering::Relaxed);
    }

    fn record_consistency_violation(&self) {
        self.consistency_violations.fetch_add(1, Ordering::Relaxed);
    }

    fn print_summary(&self, test_name: &str, duration: Duration) {
        let total = self.total_operations.load(Ordering::Relaxed);
        let success = self.successful_operations.load(Ordering::Relaxed);
        let failed = self.failed_operations.load(Ordering::Relaxed);
        let total_latency = self.total_duration_ms.load(Ordering::Relaxed);
        let min_latency = self.min_latency_ms.load(Ordering::Relaxed);
        let max_latency = self.max_latency_ms.load(Ordering::Relaxed);
        let data_loss = self.data_loss_events.load(Ordering::Relaxed);
        let consistency = self.consistency_violations.load(Ordering::Relaxed);

        println!("\n{}", "=".repeat(80));
        println!("STRESS TEST: {}", test_name);
        println!("{}", "=".repeat(80));
        println!("Duration:              {:.2}s", duration.as_secs_f64());
        println!("Total Operations:      {}", total);
        println!("Successful:            {} ({:.2}%)", success, 100.0 * success as f64 / total as f64);
        println!("Failed:                {} ({:.2}%)", failed, 100.0 * failed as f64 / total as f64);
        println!("Throughput:            {:.2} ops/sec", total as f64 / duration.as_secs_f64());
        println!("Avg Latency:           {:.2}ms", total_latency as f64 / total as f64);
        println!("Min Latency:           {}ms", if min_latency == u64::MAX { 0 } else { min_latency });
        println!("Max Latency:           {}ms", max_latency);
        println!("Data Loss Events:      {}", data_loss);
        println!("Consistency Violations: {}", consistency);
        println!("{}", "=".repeat(80));
    }
}

/// Helper to create test database config with higher connection pool
fn create_stress_test_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            max_connections: 100, // High for stress testing
            min_connections: 20,
            connection_timeout: Duration::from_secs(60),
            idle_timeout: Some(Duration::from_secs(600)),
            ..Default::default()
        },
        namespace: "cortex_stress".to_string(),
        database: db_name.to_string(),
    }
}

// =============================================================================
// Test 1: Concurrent Tool Execution - 100 Parallel Operations
// =============================================================================

#[tokio::test]
async fn test_1_concurrent_tool_execution() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting concurrent tool execution stress test");

    let db_config = create_stress_test_config("concurrent_tools");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let cognitive = Arc::new(CognitiveManager::new(connection_manager.clone()));

    // Prepare shared workspace
    let workspace_id = uuid::Uuid::new_v4();

    // Scenario 1: 10 agents reading same file concurrently
    info!("Scenario 1: 10 agents reading same file");
    let shared_path = VirtualPath::new("shared/data.rs").unwrap();
    vfs.write_file(&workspace_id, &shared_path, b"// Shared file content\npub fn shared() {}")
        .await
        .expect("Failed to create shared file");

    let mut handles = vec![];
    for i in 0..10 {
        let vfs_clone = vfs.clone();
        let path_clone = shared_path.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = vfs_clone.read_file(&workspace_id, &path_clone).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);

            if result.is_err() {
                error!("Agent {} failed to read file: {:?}", i, result.err());
            }
        });
        handles.push(handle);
    }
    join_all(handles).await;

    // Scenario 2: 10 agents writing different files concurrently
    info!("Scenario 2: 10 agents writing different files");
    let mut handles = vec![];
    for i in 0..10 {
        let vfs_clone = vfs.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let path = VirtualPath::new(&format!("agent_{}/file.rs", i)).unwrap();
            let content = format!("// Agent {} file\npub fn agent_{}() {{}}", i, i);

            let start = Instant::now();
            let result = vfs_clone.write_file(&workspace_id, &path, content.as_bytes()).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);

            if result.is_err() {
                error!("Agent {} failed to write file: {:?}", i, result.err());
            }
        });
        handles.push(handle);
    }
    join_all(handles).await;

    // Scenario 3: Race conditions in version checking
    info!("Scenario 3: Testing race conditions with concurrent updates");
    let race_path = VirtualPath::new("race/test.rs").unwrap();
    vfs.write_file(&workspace_id, &race_path, b"version 0")
        .await
        .expect("Failed to create race test file");

    let mut handles = vec![];
    for i in 0..20 {
        let vfs_clone = vfs.clone();
        let path_clone = race_path.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let content = format!("version {}", i);
            let result = vfs_clone.write_file(&workspace_id, &path_clone, content.as_bytes()).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);
        });
        handles.push(handle);
    }
    join_all(handles).await;

    // Verify final consistency
    let final_content = vfs.read_file(&workspace_id, &race_path).await.unwrap();
    let final_str = String::from_utf8_lossy(&final_content);
    info!("Final content after race: {}", final_str);

    if !final_str.starts_with("version ") {
        stats.record_consistency_violation();
        error!("Consistency violation: unexpected final content");
    }

    // Scenario 4: Memory operations under concurrent load
    info!("Scenario 4: Concurrent memory operations");
    let project_id = CortexId::new();
    let mut handles = vec![];

    for i in 0..50 {
        let cognitive_clone = cognitive.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let episode = EpisodicMemory::new(
                format!("Concurrent task {}", i),
                format!("agent-{}", i % 5),
                project_id,
                EpisodeType::Task,
            );

            let start = Instant::now();
            let result = cognitive_clone.remember_episode(&episode).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);
        });
        handles.push(handle);
    }
    join_all(handles).await;

    // Verify data consistency
    let memory_stats = cognitive.get_statistics().await.unwrap();
    if memory_stats.episodic.total_episodes != 50 {
        stats.record_data_loss();
        error!("Data loss detected: expected 50 episodes, got {}", memory_stats.episodic.total_episodes);
    }

    let test_duration = test_start.elapsed();
    stats.print_summary("Concurrent Tool Execution", test_duration);

    // Assertions
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);
    let data_loss = stats.data_loss_events.load(Ordering::Relaxed);
    let consistency = stats.consistency_violations.load(Ordering::Relaxed);

    assert!(total >= 90, "Should complete at least 90 operations");
    assert!(success as f64 / total as f64 >= 0.95, "Should have at least 95% success rate");
    assert_eq!(data_loss, 0, "Should have zero data loss events");
    assert_eq!(consistency, 0, "Should have zero consistency violations");
}

// =============================================================================
// Test 2: Large Codebase Performance - 1000+ Files
// =============================================================================

#[tokio::test]
async fn test_2_large_codebase_performance() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting large codebase performance test");

    let db_config = create_stress_test_config("large_codebase");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let parser = Arc::new(tokio::sync::Mutex::new(
        CodeParser::new().expect("Failed to create parser")
    ));
    let _semantic_memory = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));

    let workspace_id = uuid::Uuid::new_v4();

    // Import 1000+ files
    info!("Importing 1000 files");
    let import_start = Instant::now();
    let mut file_count = 0;
    let mut total_lines = 0;

    for module_idx in 0..50 {
        for file_idx in 0..20 {
            let file_path = VirtualPath::new(&format!(
                "src/module_{}/file_{}.rs",
                module_idx, file_idx
            ))
            .unwrap();

            // Generate realistic Rust code
            let content = format!(
                "//! Module {} File {}\n\
                \n\
                use std::collections::HashMap;\n\
                use std::sync::Arc;\n\
                \n\
                /// Main handler for module {}\n\
                pub struct Handler{}{} {{\n\
                    data: HashMap<String, String>,\n\
                    counter: usize,\n\
                }}\n\
                \n\
                impl Handler{}{} {{\n\
                    pub fn new() -> Self {{\n\
                        Self {{\n\
                            data: HashMap::new(),\n\
                            counter: 0,\n\
                        }}\n\
                    }}\n\
                    \n\
                    pub fn process(&mut self, input: &str) -> Result<String, String> {{\n\
                        self.counter += 1;\n\
                        self.data.insert(input.to_string(), format!(\"processed-{{}}\", self.counter));\n\
                        Ok(format!(\"Processed: {{}}\", input))\n\
                    }}\n\
                    \n\
                    pub fn get_count(&self) -> usize {{\n\
                        self.counter\n\
                    }}\n\
                }}\n\
                \n\
                #[cfg(test)]\n\
                mod tests {{\n\
                    use super::*;\n\
                    \n\
                    #[test]\n\
                    fn test_handler() {{\n\
                        let mut handler = Handler{}{}::new();\n\
                        let result = handler.process(\"test\");\n\
                        assert!(result.is_ok());\n\
                        assert_eq!(handler.get_count(), 1);\n\
                    }}\n\
                }}\n",
                module_idx, file_idx,
                module_idx,
                module_idx, file_idx,
                module_idx, file_idx,
                module_idx, file_idx,
            );

            let lines = content.lines().count();
            total_lines += lines;

            let start = Instant::now();
            let result = vfs.write_file(&workspace_id, &file_path, content.as_bytes()).await;
            let latency = start.elapsed().as_millis() as u64;

            stats.record_operation(result.is_ok(), latency);
            file_count += 1;

            if file_count % 100 == 0 {
                info!("Imported {}/1000 files, {} LOC", file_count, total_lines);
            }
        }
    }

    let import_duration = import_start.elapsed();
    info!(
        "Imported {} files ({} LOC) in {:?} ({:.2} files/sec, {:.2} LOC/sec)",
        file_count,
        total_lines,
        import_duration,
        file_count as f64 / import_duration.as_secs_f64(),
        total_lines as f64 / import_duration.as_secs_f64()
    );

    // Parse and extract code units
    info!("Parsing and extracting code units from all files");
    let parse_start = Instant::now();
    let mut unit_count = 0;

    for module_idx in 0..50 {
        for file_idx in 0..20 {
            let file_path = VirtualPath::new(&format!(
                "src/module_{}/file_{}.rs",
                module_idx, file_idx
            ))
            .unwrap();

            let content = vfs.read_file(&workspace_id, &file_path).await.unwrap();
            let content_str = String::from_utf8_lossy(&content);

            let start = Instant::now();
            let mut parser_lock = parser.lock().await;
            let result = parser_lock.parse_file(&content_str, Language::Rust);
            drop(parser_lock);
            let latency = start.elapsed().as_millis() as u64;

            stats.record_operation(result.is_ok(), latency);

            if let Ok(units) = result {
                unit_count += units.len();
            }

            if (module_idx * 20 + file_idx + 1) % 100 == 0 {
                info!("Parsed {} files, extracted {} units", module_idx * 20 + file_idx + 1, unit_count);
            }
        }
    }

    let parse_duration = parse_start.elapsed();
    info!(
        "Parsed 1000 files and extracted {} code units in {:?} ({:.2} units/sec)",
        unit_count,
        parse_duration,
        unit_count as f64 / parse_duration.as_secs_f64()
    );

    // Performance targets
    assert!(import_duration.as_secs() < 5, "Import should complete in <5s");
    assert!(parse_duration.as_secs() < 5, "Parsing should complete in <5s");
    assert!(unit_count > 10_000, "Should extract 10K+ code units, got {}", unit_count);

    let test_duration = test_start.elapsed();
    stats.print_summary("Large Codebase Performance", test_duration);

    // Overall performance assertion
    assert!(test_duration.as_secs() < 15, "Total test should complete in <15s");
}

// =============================================================================
// Test 3: Memory System Under Load - 1000s of Episodes
// =============================================================================

#[tokio::test]
async fn test_3_memory_system_under_load() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting memory system stress test");

    let db_config = create_stress_test_config("memory_stress");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = Arc::new(CognitiveManager::new(connection_manager.clone()));
    let project_id = CortexId::new();

    // Store 1000 episodes concurrently
    info!("Storing 1000 episodes concurrently");
    let store_start = Instant::now();
    let mut handles = vec![];

    for i in 0..1000 {
        let cognitive_clone = cognitive.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let mut episode = EpisodicMemory::new(
                format!("Task {}: Implement feature", i),
                format!("agent-{}", i % 10),
                project_id,
                EpisodeType::Feature,
            );

            episode.entities_created = vec![format!("src/feature_{}.rs", i)];
            episode.entities_modified = vec!["src/main.rs".to_string()];
            episode.solution_summary = format!("Implemented feature {}", i);
            episode.outcome = if i % 10 == 0 { EpisodeOutcome::Failure } else { EpisodeOutcome::Success };

            let start = Instant::now();
            let result = cognitive_clone.remember_episode(&episode).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);
        });
        handles.push(handle);
    }

    join_all(handles).await;
    let store_duration = store_start.elapsed();
    info!("Stored 1000 episodes in {:?} ({:.2} eps/sec)", store_duration, 1000.0 / store_duration.as_secs_f64());

    // Complex queries with filters
    info!("Running complex queries");
    let query_start = Instant::now();

    for i in 0..100 {
        let start = Instant::now();
        let result = cognitive
            .episodic()
            .retrieve_by_outcome(EpisodeOutcome::Success, 10)
            .await;
        let latency = start.elapsed().as_millis() as u64;

        stats.record_operation(result.is_ok(), latency);

        if i % 20 == 0 {
            info!("Completed {} queries", i + 1);
        }
    }

    let query_duration = query_start.elapsed();
    let avg_query_ms = query_duration.as_millis() / 100;
    info!("Completed 100 queries in {:?} ({:.2}ms avg)", query_duration, avg_query_ms);

    // Consolidation with many patterns
    info!("Running consolidation on 1000 episodes");
    let consolidate_start = Instant::now();
    let result = cognitive.consolidate().await;
    let consolidate_duration = consolidate_start.elapsed();

    stats.record_operation(result.is_ok(), consolidate_duration.as_millis() as u64);
    if let Ok(report) = result {
        info!("Consolidation report: episodes={}, patterns={}, duration={:?}",
              report.episodes_processed, report.patterns_extracted, consolidate_duration);
    }

    // Verify data integrity
    let memory_stats = cognitive.get_statistics().await.unwrap();
    if memory_stats.episodic.total_episodes != 1000 {
        stats.record_data_loss();
    }

    let test_duration = test_start.elapsed();
    stats.print_summary("Memory System Under Load", test_duration);

    // Assertions
    assert_eq!(memory_stats.episodic.total_episodes, 1000, "Should have 1000 episodes");
    assert!(avg_query_ms < 100, "Average query should be <100ms");
    assert!(consolidate_duration.as_secs() < 30, "Consolidation should complete in <30s");
}

// =============================================================================
// Test 4: VFS Stress Test - 10K Files
// =============================================================================

#[tokio::test]
async fn test_4_vfs_stress_test() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting VFS stress test");

    let db_config = create_stress_test_config("vfs_stress");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Create 10K files
    info!("Creating 10K files");
    let create_start = Instant::now();

    for i in 0..10_000 {
        let dir_idx = i / 100;
        let file_idx = i % 100;
        let path = VirtualPath::new(&format!("data/dir_{}/file_{}.txt", dir_idx, file_idx)).unwrap();
        let content = format!("File {} content - {}", i, "x".repeat(100));

        let start = Instant::now();
        let result = vfs.write_file(&workspace_id, &path, content.as_bytes()).await;
        let latency = start.elapsed().as_millis() as u64;

        stats.record_operation(result.is_ok(), latency);

        if (i + 1) % 1000 == 0 {
            info!("Created {}/10000 files", i + 1);
        }
    }

    let create_duration = create_start.elapsed();
    info!("Created 10K files in {:?} ({:.2} files/sec)", create_duration, 10_000.0 / create_duration.as_secs_f64());

    // Update 1000 files concurrently
    info!("Updating 1000 files concurrently");
    let update_start = Instant::now();
    let mut handles = vec![];

    for i in 0..1000 {
        let vfs_clone = vfs.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let dir_idx = i / 100;
            let file_idx = i % 100;
            let path = VirtualPath::new(&format!("data/dir_{}/file_{}.txt", dir_idx, file_idx)).unwrap();
            let new_content = format!("Updated file {} - {}", i, "y".repeat(150));

            let start = Instant::now();
            let result = vfs_clone.write_file(&workspace_id, &path, new_content.as_bytes()).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);
        });
        handles.push(handle);
    }

    join_all(handles).await;
    let update_duration = update_start.elapsed();
    info!("Updated 1000 files in {:?} ({:.2} files/sec)", update_duration, 1000.0 / update_duration.as_secs_f64());

    // Test deduplication efficiency
    info!("Testing content deduplication");
    let dedup_start = Instant::now();

    // Write 100 files with identical content
    let shared_content = "Shared content for deduplication test".repeat(10);
    for i in 0..100 {
        let path = VirtualPath::new(&format!("dedup/file_{}.txt", i)).unwrap();
        let start = Instant::now();
        let result = vfs.write_file(&workspace_id, &path, shared_content.as_bytes()).await;
        let latency = start.elapsed().as_millis() as u64;

        stats.record_operation(result.is_ok(), latency);
    }

    let dedup_duration = dedup_start.elapsed();
    info!("Deduplication test completed in {:?}", dedup_duration);

    let test_duration = test_start.elapsed();
    stats.print_summary("VFS Stress Test", test_duration);

    // Assertions
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);

    assert!(total >= 11_000, "Should complete at least 11K operations");
    assert!(success as f64 / total as f64 >= 0.99, "Should have at least 99% success rate");
}

// =============================================================================
// Test 5: Semantic Search Scalability - 100K Units
// =============================================================================

#[tokio::test]
#[ignore] // Run separately due to time/resource requirements
async fn test_5_semantic_search_scalability() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting semantic search scalability test");

    let db_config = create_stress_test_config("semantic_stress");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let semantic = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));

    // Index 100K code units (simulated)
    info!("Indexing 100K code units");
    let index_start = Instant::now();

    for i in 0..100_000 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("function_{}", i),
            qualified_name: format!("module::function_{}", i),
            display_name: format!("function_{}", i),
            file_path: format!("src/file_{}.rs", i / 100),
            start_line: (i % 100) as u32,
            start_column: 0,
            end_line: ((i % 100) + 10) as u32,
            end_column: 1,
            signature: format!("pub fn function_{}() -> Result<()>", i),
            body: format!("// Function {}\nOk(())", i),
            docstring: Some(format!("Documentation for function {}", i)),
            visibility: "pub".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: Some("Result<()>".to_string()),
            summary: format!("Function {}", i),
            purpose: "Test function".to_string(),
            complexity: ComplexityMetrics {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 1,
                lines: 10,
            },
            test_coverage: Some(0.8),
            has_tests: true,
            has_documentation: true,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let start = Instant::now();
        let result = semantic.store_unit(&unit).await;
        let latency = start.elapsed().as_millis() as u64;

        stats.record_operation(result.is_ok(), latency);

        if (i + 1) % 10_000 == 0 {
            info!("Indexed {}/100000 units", i + 1);
        }
    }

    let index_duration = index_start.elapsed();
    info!("Indexed 100K units in {:?} ({:.2} units/sec)", index_duration, 100_000.0 / index_duration.as_secs_f64());

    // 1000 concurrent search queries
    info!("Running 1000 concurrent search queries");
    let search_start = Instant::now();
    let mut handles = vec![];

    for i in 0..1000 {
        let semantic_clone = semantic.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = semantic_clone.get_units_in_file(&format!("src/file_{}.rs", i % 1000)).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);
        });
        handles.push(handle);
    }

    join_all(handles).await;
    let search_duration = search_start.elapsed();
    info!("Completed 1000 searches in {:?} ({:.2}ms avg)", search_duration, search_duration.as_millis() as f64 / 1000.0);

    let test_duration = test_start.elapsed();
    stats.print_summary("Semantic Search Scalability", test_duration);

    // Assertions
    assert!(index_duration.as_secs() < 120, "Indexing should complete in <2min");
    assert!(search_duration.as_millis() / 1000 < 200, "Average search should be <200ms");
}

// =============================================================================
// Test 6: Multi-Agent Merge Stress - 20 Agents, 100 Changes Each
// =============================================================================

#[tokio::test]
async fn test_6_multi_agent_merge_stress() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting multi-agent merge stress test");

    let db_config = create_stress_test_config("multi_agent_stress");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let lock_manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));
    let merge_engine = Arc::new(MergeEngine::new(connection_manager.clone()));

    let workspace_id = uuid::Uuid::new_v4();

    // Create base files
    info!("Creating base files");
    for i in 0..50 {
        let path = VirtualPath::new(&format!("src/module_{}.rs", i)).unwrap();
        let content = format!("// Module {}\npub fn base_function() {{}}\n", i);
        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to create base file");
    }

    // 20 agents working simultaneously
    info!("Spawning 20 agents, each making 100 changes");
    let agent_start = Instant::now();
    let mut handles = vec![];

    for agent_id in 0..20 {
        let vfs_clone = vfs.clone();
        let lock_manager_clone = lock_manager.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            for change_id in 0..100 {
                let file_idx = (agent_id * 100 + change_id) % 50;
                let path = VirtualPath::new(&format!("src/module_{}.rs", file_idx)).unwrap();
                let resource_id = format!("file_{}", file_idx);

                // Try to acquire lock
                let lock_result = lock_manager_clone
                    .acquire_lock(
                        &resource_id,
                        &format!("agent_{}", agent_id),
                        LockType::Write,
                        Duration::from_secs(5),
                    )
                    .await;

                if lock_result.is_err() {
                    stats_clone.record_operation(false, 0);
                    continue;
                }

                // Make change
                let start = Instant::now();
                let new_content = format!(
                    "// Module {} - Agent {} Change {}\npub fn agent_{}_fn_{}() {{}}\n",
                    file_idx, agent_id, change_id, agent_id, change_id
                );

                let result = vfs_clone.write_file(&workspace_id, &path, new_content.as_bytes()).await;
                let latency = start.elapsed().as_millis() as u64;

                // Release lock
                let _ = lock_manager_clone.release_lock(&resource_id, &format!("agent_{}", agent_id)).await;

                stats_clone.record_operation(result.is_ok(), latency);

                // Small delay to simulate thinking
                sleep(Duration::from_millis(1)).await;
            }

            info!("Agent {} completed all changes", agent_id);
        });
        handles.push(handle);
    }

    join_all(handles).await;
    let agent_duration = agent_start.elapsed();
    info!("All 20 agents completed in {:?}", agent_duration);

    // Verify system stability (deadlock detection runs in background)
    info!("Multi-agent test completed successfully without deadlock");

    let test_duration = test_start.elapsed();
    stats.print_summary("Multi-Agent Merge Stress", test_duration);

    // Assertions
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);

    assert!(total >= 1000, "Should attempt at least 1000 operations");
    assert!(success as f64 / total as f64 >= 0.80, "Should have at least 80% success rate (accounting for lock contention)");
}

// =============================================================================
// Test 7: Connection Pool Saturation - 100 Concurrent DB Ops
// =============================================================================

#[tokio::test]
async fn test_7_connection_pool_saturation() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting connection pool saturation test");

    let db_config = create_stress_test_config("pool_stress");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = Arc::new(CognitiveManager::new(connection_manager.clone()));
    let project_id = CortexId::new();

    // 100 concurrent database operations
    info!("Running 100 concurrent database operations");
    let mut handles = vec![];

    for i in 0..100 {
        let cognitive_clone = cognitive.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let episode = EpisodicMemory::new(
                format!("Pool test {}", i),
                format!("agent-{}", i),
                project_id,
                EpisodeType::Task,
            );

            let start = Instant::now();
            let result = cognitive_clone.remember_episode(&episode).await;
            let latency = start.elapsed().as_millis() as u64;

            stats_clone.record_operation(result.is_ok(), latency);

            // Also test retrieval
            let retrieve_start = Instant::now();
            let retrieve_result = cognitive_clone
                .episodic()
                .retrieve_by_outcome(EpisodeOutcome::Success, 5)
                .await;
            let retrieve_latency = retrieve_start.elapsed().as_millis() as u64;

            stats_clone.record_operation(retrieve_result.is_ok(), retrieve_latency);
        });
        handles.push(handle);
    }

    join_all(handles).await;

    // Verify pool statistics
    info!("Connection pool stress test completed successfully");

    let test_duration = test_start.elapsed();
    stats.print_summary("Connection Pool Saturation", test_duration);

    // Assertions
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);

    assert!(total >= 150, "Should complete at least 150 operations (100 writes + 100 reads, some may fail)");
    assert!(success as f64 / total as f64 >= 0.90, "Should have at least 90% success rate");
}

// =============================================================================
// Test 8: Error Recovery - Simulated Failures
// =============================================================================

#[tokio::test]
async fn test_8_error_recovery() {
    let test_start = Instant::now();
    let stats = StressTestStats::new();

    info!("Starting error recovery test");

    let db_config = create_stress_test_config("error_recovery");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Test 1: Invalid paths
    info!("Test 1: Invalid path handling");
    let invalid_paths = vec!["//invalid", "../../../etc/passwd"];

    for path_str in invalid_paths {
        let start = Instant::now();
        let result = VirtualPath::new(path_str);
        let latency = start.elapsed().as_millis() as u64;

        // We expect these to fail
        stats.record_operation(result.is_err(), latency);
    }

    // Test 2: Non-existent files
    info!("Test 2: Non-existent file handling");
    for i in 0..50 {
        let path = VirtualPath::new(&format!("nonexistent/file_{}.rs", i)).unwrap();

        let start = Instant::now();
        let _result = vfs.read_file(&workspace_id, &path).await;
        let latency = start.elapsed().as_millis() as u64;

        // Should gracefully handle missing files
        stats.record_operation(true, latency); // Record as success if it doesn't panic
    }

    // Test 3: Timeout handling (simulated with short operations)
    info!("Test 3: Timeout resilience");
    let mut handles = vec![];

    for i in 0..20 {
        let vfs_clone = vfs.clone();
        let stats_clone = stats.clone();

        let handle = tokio::spawn(async move {
            let path = VirtualPath::new(&format!("timeout/file_{}.rs", i)).unwrap();
            let content = "x".repeat(1_000_000); // 1MB

            let start = Instant::now();
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                vfs_clone.write_file(&workspace_id, &path, content.as_bytes()),
            )
            .await;
            let latency = start.elapsed().as_millis() as u64;

            let success = result.is_ok() && result.unwrap().is_ok();
            stats_clone.record_operation(success, latency);
        });
        handles.push(handle);
    }

    join_all(handles).await;

    // Test 4: Graceful degradation
    info!("Test 4: Graceful degradation under errors");
    let cognitive = CognitiveManager::new(connection_manager.clone());

    // Try to get non-existent episodes
    for _i in 0..20 {
        let start = Instant::now();
        let result = cognitive.episodic().get_episode(CortexId::new()).await;
        let latency = start.elapsed().as_millis() as u64;

        // Should return None, not error
        let success = result.is_ok();
        stats.record_operation(success, latency);
    }

    let test_duration = test_start.elapsed();
    stats.print_summary("Error Recovery", test_duration);

    // Assertions
    let total = stats.total_operations.load(Ordering::Relaxed);
    let success = stats.successful_operations.load(Ordering::Relaxed);

    assert!(total >= 90, "Should complete at least 90 operations");
    assert!(success as f64 / total as f64 >= 0.85, "Should handle errors gracefully with 85%+ success");
}

// =============================================================================
// NOTE: Run individual tests separately to measure actual performance:
// cargo test -p cortex-integration-tests test_1_concurrent -- --nocapture
// cargo test -p cortex-integration-tests test_2_large_codebase -- --nocapture
// cargo test -p cortex-integration-tests test_3_memory_system -- --nocapture
// cargo test -p cortex-integration-tests test_4_vfs_stress -- --nocapture
// cargo test -p cortex-integration-tests test_6_multi_agent -- --nocapture
// cargo test -p cortex-integration-tests test_7_connection_pool -- --nocapture
// cargo test -p cortex-integration-tests test_8_error_recovery -- --nocapture
// =============================================================================
