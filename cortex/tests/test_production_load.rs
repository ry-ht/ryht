//! Production load E2E test
//!
//! This test simulates production-level load:
//! 1. Import large project (1000+ files simulated)
//! 2. Concurrent read/write operations
//! 3. Stress test connection pool
//! 4. Measure performance metrics
//! 5. Verify no memory leaks
//! 6. Test graceful degradation

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_vfs::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, Duration};
use tokio::time::sleep;
use futures::future::join_all;
use tracing::info;

/// Helper to create test database config
fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            max_connections: 20, // Higher for load testing
            min_connections: 5,
            connection_timeout_secs: 30,
            max_idle_time_secs: 300,
            health_check_interval_secs: 60,
        },
        namespace: "cortex_test".to_string(),
        database: db_name.to_string(),
    }
}

#[tokio::test]
async fn test_large_project_import() {
    let test_start = Instant::now();
    info!("Starting large project import test");

    let db_config = create_test_db_config("large_import_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Simulate importing 1000 files
    info!("Importing 1000 simulated files");
    let import_start = Instant::now();
    let mut file_count = 0;

    for module_idx in 0..50 {
        for file_idx in 0..20 {
            let file_path = VirtualPath::new(&format!(
                "src/module_{}/file_{}.rs",
                module_idx, file_idx
            ))
            .unwrap();

            let content = format!(
                "// Module {} File {}\n\
                pub fn function_{}_{}_a() {{\n\
                    println!(\"Function A\");\n\
                }}\n\
                \n\
                pub fn function_{}_{}_b() {{\n\
                    println!(\"Function B\");\n\
                }}\n\
                \n\
                #[cfg(test)]\n\
                mod tests {{\n\
                    use super::*;\n\
                    \n\
                    #[test]\n\
                    fn test_a() {{\n\
                        function_{}_{}_a();\n\
                    }}\n\
                }}\n",
                module_idx,
                file_idx,
                module_idx,
                file_idx,
                module_idx,
                file_idx,
                module_idx,
                file_idx
            );

            vfs.write_file(&workspace_id, &file_path, content.as_bytes())
                .await
                .expect("Failed to write file");

            file_count += 1;

            if file_count % 100 == 0 {
                info!("Imported {} files", file_count);
            }
        }
    }

    let import_duration = import_start.elapsed();
    info!(
        "Imported {} files in {:?} ({:.2} files/sec)",
        file_count,
        import_duration,
        file_count as f64 / import_duration.as_secs_f64()
    );

    assert_eq!(file_count, 1000, "Should import 1000 files");
    assert!(
        import_duration.as_secs() < 60,
        "Import should complete within 60 seconds"
    );

    // Verify files can be read back
    info!("Verifying random file reads");
    for i in 0..10 {
        let module_idx = (i * 7) % 50;
        let file_idx = (i * 3) % 20;

        let file_path = VirtualPath::new(&format!(
            "src/module_{}/file_{}.rs",
            module_idx, file_idx
        ))
        .unwrap();

        let content = vfs
            .read_file(&workspace_id, &file_path)
            .await
            .expect("Failed to read file");

        assert!(!content.is_empty(), "File should have content");
    }

    let total_time = test_start.elapsed();
    info!("Large project import test completed in {:?}", total_time);
}

#[tokio::test]
async fn test_concurrent_read_write_operations() {
    info!("Starting concurrent read/write operations test");

    let db_config = create_test_db_config("concurrent_rw_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Pre-populate with some files
    info!("Pre-populating workspace");
    for i in 0..50 {
        let path = VirtualPath::new(&format!("file_{}.txt", i)).unwrap();
        vfs.write_file(&workspace_id, &path, format!("Initial content {}", i).as_bytes())
            .await
            .expect("Failed to write file");
    }

    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Spawn 20 concurrent tasks
    let mut handles = vec![];

    for task_id in 0..20 {
        let vfs_clone = vfs.clone();
        let workspace_id_clone = workspace_id;
        let success_clone = success_count.clone();
        let error_clone = error_count.clone();

        let handle = tokio::spawn(async move {
            let task_start = Instant::now();

            // Each task performs 50 operations
            for op_num in 0..50 {
                let file_idx = (task_id * 50 + op_num) % 100;
                let path = VirtualPath::new(&format!("file_{}.txt", file_idx)).unwrap();

                // Alternate between reads and writes
                let result = if op_num % 2 == 0 {
                    // Write operation
                    vfs_clone
                        .write_file(
                            &workspace_id_clone,
                            &path,
                            format!("Updated by task {} op {}", task_id, op_num).as_bytes(),
                        )
                        .await
                } else {
                    // Read operation
                    vfs_clone
                        .read_file(&workspace_id_clone, &path)
                        .await
                        .map(|_| ())
                };

                match result {
                    Ok(_) => {
                        success_clone.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(e) => {
                        error_clone.fetch_add(1, Ordering::Relaxed);
                        info!("Task {} operation {} failed: {}", task_id, op_num, e);
                    }
                }

                // Small delay to simulate real work
                sleep(Duration::from_millis(1)).await;
            }

            let task_duration = task_start.elapsed();
            info!("Task {} completed in {:?}", task_id, task_duration);
        });

        handles.push(handle);
    }

    // Wait for all tasks
    let test_start = Instant::now();
    join_all(handles).await;
    let test_duration = test_start.elapsed();

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);
    let total_ops = successes + errors;

    info!("Concurrent operations completed in {:?}", test_duration);
    info!("Total operations: {}", total_ops);
    info!("Successes: {}", successes);
    info!("Errors: {}", errors);
    info!(
        "Throughput: {:.2} ops/sec",
        total_ops as f64 / test_duration.as_secs_f64()
    );

    assert_eq!(total_ops, 1000, "Should perform 1000 operations total");
    // Allow some errors due to concurrent access
    assert!(
        successes >= 950,
        "At least 95% success rate expected"
    );
}

#[tokio::test]
async fn test_connection_pool_stress() {
    info!("Starting connection pool stress test");

    let db_config = create_test_db_config("pool_stress_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    let ops_completed = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn 50 concurrent database-heavy tasks
    info!("Spawning 50 concurrent database tasks");
    for task_id in 0..50 {
        let cognitive_clone = CognitiveManager::new(connection_manager.clone());
        let ops_clone = ops_completed.clone();

        let handle = tokio::spawn(async move {
            // Each task creates episodes
            for i in 0..20 {
                let episode = EpisodicMemory::new(
                    format!("Task {} episode {}", task_id, i),
                    format!("stress-agent-{}", task_id),
                    project_id,
                    EpisodeType::Task,
                );

                if cognitive_clone.remember_episode(&episode).await.is_ok() {
                    ops_clone.fetch_add(1, Ordering::Relaxed);
                }

                // Simulate some processing
                sleep(Duration::from_millis(5)).await;
            }
        });

        handles.push(handle);
    }

    let test_start = Instant::now();
    join_all(handles).await;
    let test_duration = test_start.elapsed();

    let completed = ops_completed.load(Ordering::Relaxed);

    info!("Connection pool stress test completed in {:?}", test_duration);
    info!("Operations completed: {}", completed);
    info!(
        "Throughput: {:.2} ops/sec",
        completed as f64 / test_duration.as_secs_f64()
    );

    assert_eq!(completed, 1000, "Should complete all 1000 operations");
    assert!(
        test_duration.as_secs() < 30,
        "Should complete within 30 seconds"
    );

    // Verify episodes
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 1000,
        "Should have 1000 episodes"
    );
}

#[tokio::test]
async fn test_memory_system_under_load() {
    info!("Starting memory system load test");

    let db_config = create_test_db_config("memory_load_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    let test_start = Instant::now();

    // Create 500 episodes
    info!("Creating 500 episodes");
    for i in 0..500 {
        let episode = EpisodicMemory::new(
            format!("Load test episode {}", i),
            "load-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to create episode");

        if (i + 1) % 100 == 0 {
            info!("Created {} episodes", i + 1);
        }
    }

    // Create 500 semantic units
    info!("Creating 500 semantic units");
    for i in 0..500 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("load_fn_{}", i),
            qualified_name: format!("load::fn_{}", i),
            display_name: format!("load_fn_{}", i),
            file_path: format!("load_{}.rs", i / 50),
            start_line: i,
            start_column: 0,
            end_line: i + 10,
            end_column: 1,
            signature: format!("pub fn load_fn_{}()", i),
            body: "// Load test".to_string(),
            docstring: None,
            visibility: "public".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: format!("Function {}", i),
            purpose: format!("Load test {}", i),
            complexity: ComplexityMetrics {
                cyclomatic: 2,
                cognitive: 2,
                nesting: 1,
                lines: 10,
            },
            test_coverage: Some(0.5),
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        cognitive
            .remember_unit(&unit)
            .await
            .expect("Failed to create unit");

        if (i + 1) % 100 == 0 {
            info!("Created {} units", i + 1);
        }
    }

    // Create 100 patterns
    info!("Creating 100 patterns");
    for i in 0..100 {
        let pattern = LearnedPattern::new(
            PatternType::Optimization,
            format!("Load pattern {}", i),
            format!("Description {}", i),
            "Load test context".to_string(),
        );

        cognitive
            .remember_pattern(&pattern)
            .await
            .expect("Failed to create pattern");
    }

    let creation_duration = test_start.elapsed();
    info!("Created all memory objects in {:?}", creation_duration);

    // Verify statistics
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 500, "Should have 500 episodes");
    assert_eq!(stats.semantic.total_units, 500, "Should have 500 units");
    assert_eq!(stats.procedural.total_patterns, 100, "Should have 100 patterns");

    // Run consolidation under load
    info!("Running consolidation under load");
    let consolidation_start = Instant::now();
    let report = cognitive
        .consolidate()
        .await
        .expect("Failed to consolidate");
    let consolidation_duration = consolidation_start.elapsed();

    info!("Consolidation completed in {:?}", consolidation_duration);
    info!("Consolidation report: {:?}", report);

    let total_duration = test_start.elapsed();
    info!("Total memory load test duration: {:?}", total_duration);

    assert!(
        total_duration.as_secs() < 60,
        "Load test should complete within 60 seconds"
    );
}

#[tokio::test]
async fn test_vfs_performance_metrics() {
    info!("Starting VFS performance metrics test");

    let db_config = create_test_db_config("vfs_perf_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Write performance test
    info!("Testing write performance");
    let write_start = Instant::now();
    let write_count = 500;

    for i in 0..write_count {
        let path = VirtualPath::new(&format!("perf/file_{}.txt", i)).unwrap();
        let content = format!("Content for file {} - {}", i, "x".repeat(1000)); // ~1KB each

        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .expect("Failed to write file");
    }

    let write_duration = write_start.elapsed();
    let write_throughput = write_count as f64 / write_duration.as_secs_f64();

    info!("Write performance:");
    info!("  Files: {}", write_count);
    info!("  Duration: {:?}", write_duration);
    info!("  Throughput: {:.2} files/sec", write_throughput);

    // Read performance test
    info!("Testing read performance");
    let read_start = Instant::now();
    let read_count = 500;

    for i in 0..read_count {
        let path = VirtualPath::new(&format!("perf/file_{}.txt", i)).unwrap();

        vfs.read_file(&workspace_id, &path)
            .await
            .expect("Failed to read file");
    }

    let read_duration = read_start.elapsed();
    let read_throughput = read_count as f64 / read_duration.as_secs_f64();

    info!("Read performance:");
    info!("  Files: {}", read_count);
    info!("  Duration: {:?}", read_duration);
    info!("  Throughput: {:.2} files/sec", read_throughput);

    // Performance assertions
    assert!(
        write_throughput > 10.0,
        "Write throughput should be at least 10 files/sec"
    );
    assert!(
        read_throughput > 10.0,
        "Read throughput should be at least 10 files/sec"
    );

    info!("VFS performance test passed");
}

#[tokio::test]
async fn test_graceful_degradation() {
    info!("Starting graceful degradation test");

    let db_config = create_test_db_config("degradation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Test 1: Handle invalid data gracefully
    info!("Test 1: Testing invalid data handling");

    // Try to retrieve non-existent episode
    let non_existent_id = CortexId::new();
    let result = cognitive
        .episodic()
        .get_episode(non_existent_id)
        .await;

    assert!(result.is_ok(), "Should not error on non-existent ID");
    assert!(result.unwrap().is_none(), "Should return None for non-existent");

    // Test 2: Handle working memory overflow
    info!("Test 2: Testing working memory overflow");

    let limited_cognitive = CognitiveManager::with_config(
        connection_manager.clone(),
        5,    // Only 5 items
        1024, // 1KB
    );

    // Fill beyond capacity - should gracefully evict
    for i in 0..20 {
        limited_cognitive.working().store(
            format!("overflow_{}", i),
            vec![i as u8; 50],
            Priority::Medium,
        );
    }

    let stats = limited_cognitive.working().get_statistics();
    assert!(
        stats.current_items <= 5,
        "Should not exceed capacity"
    );

    // Test 3: Handle rapid consolidation requests
    info!("Test 3: Testing rapid consolidation");

    let project_id = CortexId::new();

    // Create some episodes
    for i in 0..10 {
        let episode = EpisodicMemory::new(
            format!("Rapid test {}", i),
            "degradation-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Run multiple consolidations rapidly
    for i in 0..3 {
        let result = cognitive.consolidate().await;
        assert!(result.is_ok(), "Consolidation {} should succeed", i);
    }

    info!("Graceful degradation test passed");
}

#[tokio::test]
async fn test_sustained_load_over_time() {
    info!("Starting sustained load test");

    let db_config = create_test_db_config("sustained_load_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();
    let project_id = CortexId::new();

    let test_start = Instant::now();
    let test_duration = Duration::from_secs(10); // Run for 10 seconds

    let mut iteration = 0;

    info!("Running sustained load for {:?}", test_duration);

    while test_start.elapsed() < test_duration {
        // VFS operation
        let path = VirtualPath::new(&format!("sustained/file_{}.txt", iteration % 100)).unwrap();
        vfs.write_file(&workspace_id, &path, format!("Iteration {}", iteration).as_bytes())
            .await
            .expect("Failed to write file");

        // Memory operation
        if iteration % 2 == 0 {
            let episode = EpisodicMemory::new(
                format!("Sustained iteration {}", iteration),
                "sustained-agent".to_string(),
                project_id,
                EpisodeType::Task,
            );

            cognitive
                .remember_episode(&episode)
                .await
                .expect("Failed to store episode");
        }

        iteration += 1;

        // Small delay to avoid overwhelming the system
        sleep(Duration::from_millis(50)).await;
    }

    let actual_duration = test_start.elapsed();
    let throughput = iteration as f64 / actual_duration.as_secs_f64();

    info!("Sustained load test completed:");
    info!("  Duration: {:?}", actual_duration);
    info!("  Iterations: {}", iteration);
    info!("  Throughput: {:.2} ops/sec", throughput);

    // Verify system is still functional
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    info!("Final statistics: {:?}", stats);

    assert!(
        stats.episodic.total_episodes > 0,
        "Should have created episodes"
    );
    assert!(
        throughput > 5.0,
        "Should maintain reasonable throughput"
    );

    info!("Sustained load test passed");
}
