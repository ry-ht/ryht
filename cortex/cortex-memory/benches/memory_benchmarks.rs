//! Performance benchmarks for cognitive memory system.

use cortex_core::id::CortexId;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use tokio::runtime::Runtime;

async fn create_test_manager() -> CognitiveManager {
    let config = ConnectionConfig::memory();
    let pool_config = PoolConfig::default();
    let manager = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    );
    CognitiveManager::new(manager)
}

fn bench_episodic_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    c.bench_function("episodic_store", |b| {
        b.iter(|| {
            let episode = EpisodicMemory::new(
                black_box("Test task".to_string()),
                black_box("agent-001".to_string()),
                black_box(CortexId::new()),
                black_box(EpisodeType::Task),
            );

            rt.block_on(async {
                manager
                    .remember_episode(&episode)
                    .await
                    .expect("Failed to store episode")
            })
        })
    });
}

fn bench_episodic_retrieval(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    // Pre-populate with episodes
    let mut episode_ids = Vec::new();
    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        let id = rt
            .block_on(async {
                manager
                    .remember_episode(&episode)
                    .await
                    .expect("Failed to store episode")
            });
        episode_ids.push(id);
    }

    c.bench_function("episodic_retrieve", |b| {
        b.iter(|| {
            let id = black_box(episode_ids[0]);
            rt.block_on(async {
                manager
                    .episodic()
                    .get_episode(id)
                    .await
                    .expect("Failed to retrieve episode")
            })
        })
    });
}

fn bench_semantic_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    c.bench_function("semantic_store", |b| {
        b.iter(|| {
            let unit = SemanticUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Function,
                name: black_box("test_function".to_string()),
                qualified_name: black_box("module::test_function".to_string()),
                display_name: black_box("test_function".to_string()),
                file_path: black_box("src/test.rs".to_string()),
                start_line: 10,
                start_column: 0,
                end_line: 20,
                end_column: 1,
                signature: black_box("fn test() -> Result<()>".to_string()),
                body: black_box("// test body".to_string()),
                docstring: Some("Test function".to_string()),
                visibility: "public".to_string(),
                modifiers: vec![],
                parameters: vec![],
                return_type: Some("Result<()>".to_string()),
                summary: "Test".to_string(),
                purpose: "Testing".to_string(),
                complexity: ComplexityMetrics::default(),
                test_coverage: Some(0.8),
                has_tests: true,
                has_documentation: true,
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            rt.block_on(async {
                manager
                    .remember_unit(&unit)
                    .await
                    .expect("Failed to store unit")
            })
        })
    });
}

fn bench_working_memory_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    let mut group = c.benchmark_group("working_memory");

    group.bench_function("store", |b| {
        b.iter(|| {
            manager.working().store(
                black_box("test_key".to_string()),
                black_box(vec![1, 2, 3, 4, 5]),
                black_box(Priority::Medium),
            )
        })
    });

    group.bench_function("retrieve", |b| {
        // Pre-populate
        manager.working().store(
            "benchmark_key".to_string(),
            vec![1, 2, 3, 4, 5],
            Priority::Medium,
        );

        b.iter(|| {
            manager.working().retrieve(black_box("benchmark_key"))
        })
    });

    group.finish();
}

fn bench_working_memory_eviction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("working_memory_eviction");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let manager = rt.block_on(create_test_manager());

                // Fill memory to trigger eviction
                for i in 0..size {
                    let priority = match i % 3 {
                        0 => Priority::Low,
                        1 => Priority::Medium,
                        _ => Priority::High,
                    };

                    manager.working().store(
                        format!("key_{}", i),
                        vec![i as u8; 1024],
                        priority,
                    );
                }

                // Trigger final eviction
                manager.working().store(
                    "final_key".to_string(),
                    vec![0; 1024],
                    Priority::Critical,
                );
            })
        });
    }

    group.finish();
}

fn bench_pattern_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    c.bench_function("pattern_store", |b| {
        b.iter(|| {
            let pattern = LearnedPattern::new(
                black_box(PatternType::Code),
                black_box("Test pattern".to_string()),
                black_box("Description".to_string()),
                black_box("Context".to_string()),
            );

            rt.block_on(async {
                manager
                    .remember_pattern(&pattern)
                    .await
                    .expect("Failed to store pattern")
            })
        })
    });
}

fn bench_consolidation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    // Pre-populate with episodes
    rt.block_on(async {
        for i in 0..20 {
            let mut episode = EpisodicMemory::new(
                format!("Task {}", i),
                "agent-001".to_string(),
                CortexId::new(),
                EpisodeType::Task,
            );
            episode.outcome = if i % 2 == 0 {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Partial
            };

            manager
                .remember_episode(&episode)
                .await
                .expect("Failed to store episode");
        }
    });

    c.bench_function("consolidation", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager.consolidate().await.expect("Failed to consolidate")
            })
        })
    });
}

fn bench_complexity_analysis(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    // Create test units with varying complexity
    rt.block_on(async {
        for i in 0..50 {
            let unit = SemanticUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Function,
                name: format!("function_{}", i),
                qualified_name: format!("module::function_{}", i),
                display_name: format!("function_{}", i),
                file_path: "src/test.rs".to_string(),
                start_line: i * 20,
                start_column: 0,
                end_line: (i + 1) * 20,
                end_column: 1,
                signature: format!("fn function_{}() -> Result<()>", i),
                body: "// function body".to_string(),
                docstring: None,
                visibility: "private".to_string(),
                modifiers: vec![],
                parameters: vec![],
                return_type: Some("Result<()>".to_string()),
                summary: "Function".to_string(),
                purpose: "Testing".to_string(),
                complexity: ComplexityMetrics {
                    cyclomatic: (i % 30) + 1,
                    cognitive: (i % 40) + 1,
                    nesting: i % 5,
                    lines: (i % 100) + 10,
                },
                test_coverage: None,
                has_tests: false,
                has_documentation: false,
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            manager
                .remember_unit(&unit)
                .await
                .expect("Failed to store unit");
        }
    });

    c.bench_function("find_complex_units", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager
                    .semantic()
                    .find_complex_units(black_box(15))
                    .await
                    .expect("Failed to find complex units")
            })
        })
    });
}

fn bench_dependency_tracking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(create_test_manager());

    // Create dependency graph
    let source_id = CortexId::new();
    rt.block_on(async {
        for i in 0..20 {
            let target_id = CortexId::new();
            manager
                .associate(source_id, target_id, DependencyType::Calls)
                .await
                .expect("Failed to create dependency");
        }
    });

    c.bench_function("get_dependencies", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager
                    .semantic()
                    .get_dependencies(black_box(source_id))
                    .await
                    .expect("Failed to get dependencies")
            })
        })
    });
}

criterion_group!(
    benches,
    bench_episodic_storage,
    bench_episodic_retrieval,
    bench_semantic_storage,
    bench_working_memory_operations,
    bench_working_memory_eviction,
    bench_pattern_storage,
    bench_consolidation,
    bench_complexity_analysis,
    bench_dependency_tracking
);

criterion_main!(benches);
