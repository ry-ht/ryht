//! Memory System Performance Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Working memory (store <1ms, retrieve <1ms, eviction <5ms)
//! - Episodic memory (store episode <50ms, query recent <100ms, pattern extraction <500ms)
//! - Semantic memory (store code unit <50ms, find dependencies <100ms, search <50ms)
//! - Memory consolidation and cross-memory operations

use cortex_memory::{
    working::{WorkingMemory, WorkingMemoryItem, ItemPriority},
    episodic::{EpisodicMemory, Episode, EpisodeType},
    semantic::{SemanticMemory, CodeUnitMemory},
    consolidation::MemoryConsolidator,
};
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, PoolConfig,
    RetryPolicy, Credentials,
};
use cortex_core::types::CodeUnit;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;
use serde_json::json;

// ==============================================================================
// Benchmark Setup Helpers
// ==============================================================================

fn create_test_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "memory".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                backoff_multiplier: 2.0,
            },
            warm_connections: true,
            health_check_interval: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        },
        namespace: "memory_bench_ns".to_string(),
        database: "memory_bench_db".to_string(),
    }
}

async fn setup_memory_system() -> (
    Arc<ConnectionManager>,
    WorkingMemory,
    EpisodicMemory,
    SemanticMemory,
) {
    let config = create_test_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    let manager = Arc::new(manager);

    let working = WorkingMemory::new(manager.clone(), 1000); // 1000 item capacity
    let episodic = EpisodicMemory::new(manager.clone());
    let semantic = SemanticMemory::new(manager.clone());

    (manager, working, episodic, semantic)
}

fn create_test_working_item(key: &str, size: usize) -> WorkingMemoryItem {
    WorkingMemoryItem {
        key: key.to_string(),
        value: json!({
            "data": "x".repeat(size),
            "timestamp": chrono::Utc::now(),
        }),
        priority: ItemPriority::Normal,
        ttl: Some(Duration::from_secs(300)),
        access_count: 0,
        last_accessed: chrono::Utc::now(),
    }
}

fn create_test_episode(episode_type: EpisodeType, detail_level: usize) -> Episode {
    let mut actions = Vec::new();
    for i in 0..detail_level {
        actions.push(json!({
            "action": format!("action_{}", i),
            "result": format!("result_{}", i),
        }));
    }

    Episode {
        id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        episode_type,
        timestamp: chrono::Utc::now(),
        actions,
        outcome: json!({"success": true}),
        context: json!({"workspace": "test"}),
        duration: Duration::from_secs(1),
    }
}

fn create_test_code_unit(index: usize) -> CodeUnit {
    CodeUnit {
        id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        path: format!("/src/module_{}.rs", index),
        name: format!("function_{}", index),
        kind: "function".to_string(),
        content: format!(
            "pub fn function_{}(x: i32) -> i32 {{\n\
             \t// Function implementation\n\
             \tx * {}\n\
             }}",
            index, index
        ),
        start_line: 1,
        end_line: 4,
        language: "rust".to_string(),
        metadata: json!({
            "complexity": index % 10,
            "calls": vec![format!("other_fn_{}", index + 1)],
        }),
    }
}

// ==============================================================================
// Working Memory Benchmarks
// ==============================================================================

fn bench_working_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, working, _, _) = rt.block_on(setup_memory_system());

    let mut group = c.benchmark_group("working_memory");
    group.significance_level(0.05).sample_size(200);

    // Store item - Target: <1ms
    group.bench_function("store_item", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let item = create_test_working_item(&format!("key_{}", counter), 100);
            working.store(item).await.unwrap();
        });
    });

    // Retrieve item - Target: <1ms
    // Pre-populate some items
    rt.block_on(async {
        for i in 0..100 {
            let item = create_test_working_item(&format!("retrieve_key_{}", i), 100);
            working.store(item).await.unwrap();
        }
    });

    group.bench_function("retrieve_item", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter = (counter + 1) % 100;
            let item = working.get(&format!("retrieve_key_{}", counter)).await.unwrap();
            black_box(item);
        });
    });

    // Update item - Target: <2ms
    group.bench_function("update_item", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter = (counter + 1) % 100;
            let key = format!("retrieve_key_{}", counter);
            working.update(&key, json!({"updated": true})).await.unwrap();
        });
    });

    // Delete item - Target: <2ms
    group.bench_function("delete_item", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            // Store and delete
            let item = create_test_working_item(&format!("delete_key_{}", counter), 100);
            working.store(item).await.unwrap();
            working.delete(&format!("delete_key_{}", counter)).await.unwrap();
        });
    });

    // Eviction (LRU) - Target: <5ms
    group.bench_function("evict_lru", |b| {
        b.to_async(&rt).iter(|| async {
            working.evict_lru(10).await.unwrap();
        });
    });

    // Batch store - Target: <10ms for 100 items
    group.throughput(Throughput::Elements(100));
    group.bench_function("batch_store_100", |b| {
        let mut batch_counter = 0;
        b.to_async(&rt).iter(|| async {
            batch_counter += 1;
            for i in 0..100 {
                let item = create_test_working_item(
                    &format!("batch_{}_{}", batch_counter, i),
                    100,
                );
                working.store(item).await.unwrap();
            }
        });
    });

    // Query by priority - Target: <10ms
    group.bench_function("query_by_priority", |b| {
        b.to_async(&rt).iter(|| async {
            let items = working.get_by_priority(ItemPriority::High).await.unwrap();
            black_box(items);
        });
    });

    // Get all items - Target: <20ms for 1000 items
    group.bench_function("get_all_items", |b| {
        b.to_async(&rt).iter(|| async {
            let items = working.get_all().await.unwrap();
            black_box(items);
        });
    });

    group.finish();
}

// ==============================================================================
// Episodic Memory Benchmarks
// ==============================================================================

fn bench_episodic_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, _, episodic, _) = rt.block_on(setup_memory_system());

    let mut group = c.benchmark_group("episodic_memory");
    group.significance_level(0.05).sample_size(100);

    // Store episode - Target: <50ms
    group.bench_function("store_episode", |b| {
        b.to_async(&rt).iter(|| async {
            let episode = create_test_episode(EpisodeType::CodeModification, 5);
            episodic.store(episode).await.unwrap();
        });
    });

    // Store complex episode with many actions - Target: <100ms
    group.bench_function("store_complex_episode", |b| {
        b.to_async(&rt).iter(|| async {
            let episode = create_test_episode(EpisodeType::Debugging, 20);
            episodic.store(episode).await.unwrap();
        });
    });

    // Pre-populate with episodes
    let agent_id = Uuid::new_v4();
    rt.block_on(async {
        for i in 0..100 {
            let episode_type = match i % 4 {
                0 => EpisodeType::CodeModification,
                1 => EpisodeType::Debugging,
                2 => EpisodeType::Testing,
                _ => EpisodeType::Refactoring,
            };
            let mut episode = create_test_episode(episode_type, 5);
            episode.agent_id = agent_id;
            episodic.store(episode).await.unwrap();
        }
    });

    // Query recent episodes - Target: <100ms
    group.bench_function("query_recent_10", |b| {
        b.to_async(&rt).iter(|| async {
            let episodes = episodic.get_recent(&agent_id, 10).await.unwrap();
            black_box(episodes);
        });
    });

    // Query by type - Target: <100ms
    group.bench_function("query_by_type", |b| {
        b.to_async(&rt).iter(|| async {
            let episodes = episodic
                .get_by_type(&agent_id, EpisodeType::CodeModification, 20)
                .await
                .unwrap();
            black_box(episodes);
        });
    });

    // Query by time range - Target: <150ms
    group.bench_function("query_time_range", |b| {
        b.to_async(&rt).iter(|| async {
            let start = chrono::Utc::now() - chrono::Duration::hours(1);
            let end = chrono::Utc::now();
            let episodes = episodic
                .get_in_range(&agent_id, start, end)
                .await
                .unwrap();
            black_box(episodes);
        });
    });

    // Find similar episodes - Target: <200ms
    group.bench_function("find_similar_episodes", |b| {
        b.to_async(&rt).iter(|| async {
            let query_episode = create_test_episode(EpisodeType::CodeModification, 5);
            let similar = episodic
                .find_similar(&query_episode, 10)
                .await
                .unwrap();
            black_box(similar);
        });
    });

    // Pattern extraction - Target: <500ms
    group.bench_function("extract_patterns", |b| {
        b.to_async(&rt).iter(|| async {
            let patterns = episodic
                .extract_patterns(&agent_id, 100)
                .await
                .unwrap();
            black_box(patterns);
        });
    });

    // Episode statistics - Target: <100ms
    group.bench_function("episode_statistics", |b| {
        b.to_async(&rt).iter(|| async {
            let stats = episodic.get_statistics(&agent_id).await.unwrap();
            black_box(stats);
        });
    });

    group.finish();
}

// ==============================================================================
// Semantic Memory Benchmarks
// ==============================================================================

fn bench_semantic_memory(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, _, _, semantic) = rt.block_on(setup_memory_system());

    let mut group = c.benchmark_group("semantic_memory");
    group.significance_level(0.05).sample_size(100);

    // Store code unit - Target: <50ms
    group.bench_function("store_code_unit", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let code_unit = create_test_code_unit(counter);
            semantic.store_code_unit(&code_unit).await.unwrap();
        });
    });

    // Pre-populate semantic memory
    let workspace_id = Uuid::new_v4();
    rt.block_on(async {
        for i in 0..1000 {
            let mut code_unit = create_test_code_unit(i);
            code_unit.workspace_id = workspace_id;
            semantic.store_code_unit(&code_unit).await.unwrap();
        }
    });

    // Retrieve code unit by ID - Target: <10ms
    group.bench_function("retrieve_by_id", |b| {
        b.to_async(&rt).iter(|| async {
            let code_unit = semantic
                .get_code_unit(&workspace_id, &format!("function_100"))
                .await
                .unwrap();
            black_box(code_unit);
        });
    });

    // Search by name - Target: <50ms
    group.bench_function("search_by_name", |b| {
        b.to_async(&rt).iter(|| async {
            let results = semantic
                .search_by_name(&workspace_id, "function_5")
                .await
                .unwrap();
            black_box(results);
        });
    });

    // Search by kind - Target: <50ms
    group.bench_function("search_by_kind", |b| {
        b.to_async(&rt).iter(|| async {
            let results = semantic
                .search_by_kind(&workspace_id, "function", 20)
                .await
                .unwrap();
            black_box(results);
        });
    });

    // Find dependencies - Target: <100ms
    group.bench_function("find_dependencies", |b| {
        b.to_async(&rt).iter(|| async {
            let deps = semantic
                .find_dependencies(&workspace_id, "function_100")
                .await
                .unwrap();
            black_box(deps);
        });
    });

    // Find dependents (reverse dependencies) - Target: <100ms
    group.bench_function("find_dependents", |b| {
        b.to_async(&rt).iter(|| async {
            let dependents = semantic
                .find_dependents(&workspace_id, "function_100")
                .await
                .unwrap();
            black_box(dependents);
        });
    });

    // Build dependency graph - Target: <200ms for 100 units
    group.bench_function("build_dependency_graph_100", |b| {
        b.to_async(&rt).iter(|| async {
            let graph = semantic
                .build_dependency_graph(&workspace_id, 100)
                .await
                .unwrap();
            black_box(graph);
        });
    });

    // Get all in path - Target: <100ms
    group.bench_function("get_all_in_path", |b| {
        b.to_async(&rt).iter(|| async {
            let units = semantic
                .get_all_in_path(&workspace_id, "/src")
                .await
                .unwrap();
            black_box(units);
        });
    });

    // Update code unit - Target: <50ms
    group.bench_function("update_code_unit", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter = (counter + 1) % 1000;
            let mut code_unit = create_test_code_unit(counter);
            code_unit.workspace_id = workspace_id;
            code_unit.content = "// Updated content".to_string();
            semantic.update_code_unit(&code_unit).await.unwrap();
        });
    });

    // Delete code unit - Target: <30ms
    group.bench_function("delete_code_unit", |b| {
        let mut counter = 1000;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            // Create and delete
            let code_unit = create_test_code_unit(counter);
            semantic.store_code_unit(&code_unit).await.unwrap();
            semantic
                .delete_code_unit(&code_unit.workspace_id, &code_unit.name)
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Memory Consolidation Benchmarks
// ==============================================================================

fn bench_memory_consolidation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, working, episodic, semantic) = rt.block_on(setup_memory_system());

    let consolidator = MemoryConsolidator::new(
        working.clone(),
        episodic.clone(),
        semantic.clone(),
    );

    let mut group = c.benchmark_group("memory_consolidation");
    group.significance_level(0.05).sample_size(20);
    group.measurement_time(Duration::from_secs(15));

    // Consolidate working to episodic - Target: <200ms
    group.bench_function("consolidate_working_to_episodic", |b| {
        b.to_async(&rt).iter(|| async {
            consolidator
                .consolidate_working_to_episodic()
                .await
                .unwrap();
        });
    });

    // Consolidate episodic to semantic - Target: <500ms
    group.bench_function("consolidate_episodic_to_semantic", |b| {
        b.to_async(&rt).iter(|| async {
            consolidator
                .consolidate_episodic_to_semantic()
                .await
                .unwrap();
        });
    });

    // Full consolidation cycle - Target: <1s
    group.bench_function("full_consolidation_cycle", |b| {
        b.to_async(&rt).iter(|| async {
            consolidator.run_full_cycle().await.unwrap();
        });
    });

    // Selective consolidation - Target: <300ms
    group.bench_function("selective_consolidation", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = Uuid::new_v4();
            consolidator
                .consolidate_for_agent(&agent_id)
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Cross-Memory Operations Benchmarks
// ==============================================================================

fn bench_cross_memory_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, working, episodic, semantic) = rt.block_on(setup_memory_system());

    // Pre-populate all memory systems
    let agent_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();

    rt.block_on(async {
        // Working memory
        for i in 0..100 {
            let item = create_test_working_item(&format!("cross_key_{}", i), 100);
            working.store(item).await.unwrap();
        }

        // Episodic memory
        for i in 0..100 {
            let mut episode = create_test_episode(EpisodeType::CodeModification, 5);
            episode.agent_id = agent_id;
            episodic.store(episode).await.unwrap();
        }

        // Semantic memory
        for i in 0..100 {
            let mut code_unit = create_test_code_unit(i);
            code_unit.workspace_id = workspace_id;
            semantic.store_code_unit(&code_unit).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("cross_memory_operations");
    group.significance_level(0.05).sample_size(50);

    // Query across all memory systems - Target: <200ms
    group.bench_function("query_all_memories", |b| {
        b.to_async(&rt).iter(|| async {
            let working_items = working.get_all().await.unwrap();
            let episodes = episodic.get_recent(&agent_id, 10).await.unwrap();
            let code_units = semantic
                .search_by_kind(&workspace_id, "function", 10)
                .await
                .unwrap();
            black_box((working_items, episodes, code_units));
        });
    });

    // Context reconstruction - Target: <300ms
    group.bench_function("reconstruct_context", |b| {
        b.to_async(&rt).iter(|| async {
            // Get recent working memory
            let recent_working = working.get_recent(20).await.unwrap();

            // Get recent episodes
            let recent_episodes = episodic.get_recent(&agent_id, 10).await.unwrap();

            // Get relevant code units
            let code_context = semantic
                .get_all_in_path(&workspace_id, "/src")
                .await
                .unwrap();

            black_box((recent_working, recent_episodes, code_context));
        });
    });

    // Memory statistics across all systems - Target: <150ms
    group.bench_function("global_memory_stats", |b| {
        b.to_async(&rt).iter(|| async {
            let working_count = working.count().await.unwrap();
            let episodic_count = episodic.count(&agent_id).await.unwrap();
            let semantic_count = semantic.count(&workspace_id).await.unwrap();
            black_box((working_count, episodic_count, semantic_count));
        });
    });

    group.finish();
}

// ==============================================================================
// Memory Cleanup and Maintenance Benchmarks
// ==============================================================================

fn bench_memory_maintenance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, working, episodic, semantic) = rt.block_on(setup_memory_system());

    let mut group = c.benchmark_group("memory_maintenance");
    group.significance_level(0.05).sample_size(20);

    // Cleanup expired items - Target: <100ms
    group.bench_function("cleanup_expired_working", |b| {
        b.to_async(&rt).iter(|| async {
            working.cleanup_expired().await.unwrap();
        });
    });

    // Archive old episodes - Target: <500ms
    group.bench_function("archive_old_episodes", |b| {
        b.to_async(&rt).iter(|| async {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
            episodic.archive_before(cutoff).await.unwrap();
        });
    });

    // Compact semantic memory - Target: <1s
    group.bench_function("compact_semantic_memory", |b| {
        b.to_async(&rt).iter(|| async {
            let workspace_id = Uuid::new_v4();
            semantic.compact(&workspace_id).await.unwrap();
        });
    });

    // Vacuum all memory systems - Target: <2s
    group.bench_function("vacuum_all_memory", |b| {
        b.to_async(&rt).iter(|| async {
            working.vacuum().await.unwrap();
            episodic.vacuum().await.unwrap();
            semantic.vacuum().await.unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Main Benchmark Configuration
// ==============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_working_memory,
        bench_episodic_memory,
        bench_semantic_memory,
        bench_memory_consolidation,
        bench_cross_memory_operations,
        bench_memory_maintenance,
);

criterion_main!(benches);
