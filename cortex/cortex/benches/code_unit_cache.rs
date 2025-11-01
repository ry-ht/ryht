//! Benchmarks for CodeUnitService cache performance

use cortex::services::{CacheConfig, CodeUnitService};
use cortex_core::types::{CodeUnit, CodeUnitType, Complexity, Language, Visibility};
use cortex_storage::ConnectionManager;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use tokio::runtime::Runtime;
use uuid::Uuid;

fn create_test_unit(id: &str, name: &str, qname: &str) -> CodeUnit {
    CodeUnit {
        id: id.to_string(),
        unit_type: CodeUnitType::Function,
        name: name.to_string(),
        qualified_name: qname.to_string(),
        display_name: name.to_string(),
        file_path: format!("/test/{}.rs", name),
        language: Language::Rust,
        start_line: 1,
        end_line: 10,
        start_column: 0,
        end_column: 0,
        signature: format!("fn {}()", name),
        body: Some("{ }".to_string()),
        docstring: Some(format!("Test {}", name)),
        visibility: Visibility::Public,
        is_async: false,
        is_exported: true,
        complexity: Complexity {
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            lines: 10,
        },
        has_tests: false,
        has_documentation: true,
        dependencies: vec![],
        version: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

async fn setup_service_with_units(count: usize) -> (Arc<ConnectionManager>, CodeUnitService) {
    let storage = Arc::new(ConnectionManager::new_memory().await.unwrap());
    let service = CodeUnitService::new(storage.clone());

    // Insert test units
    for i in 0..count {
        let unit = create_test_unit(
            &format!("bench:{}", i),
            &format!("func_{}", i),
            &format!("module::func_{}", i),
        );

        let pooled = storage.acquire().await.unwrap();
        let conn = pooled.connection();
        let query = "CREATE code_unit CONTENT $unit";
        conn.query(query).bind(("unit", &unit)).await.unwrap();
    }

    (storage, service)
}

fn bench_cache_hit_vs_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_hit_vs_miss");

    // Setup: service with 100 units
    let (_storage, service) = rt.block_on(setup_service_with_units(100));

    // Warm up cache for unit 0
    rt.block_on(async { service.get_code_unit("bench:0").await.unwrap() });

    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(service.get_code_unit("bench:0").await.unwrap())
        });
    });

    group.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            // Always miss cache by accessing random units
            let id = format!("bench:{}", fastrand::usize(1..100));
            // Clear cache before each iteration to ensure miss
            service.clear_cache().await;
            black_box(service.get_code_unit(&id).await.unwrap())
        });
    });

    group.finish();
}

fn bench_different_cache_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_sizes");

    let cache_sizes = vec![100, 1000, 10000];

    for size in cache_sizes {
        let storage = rt.block_on(async {
            Arc::new(ConnectionManager::new_memory().await.unwrap())
        });

        let config = CacheConfig {
            max_capacity: size,
            ttl_seconds: 300,
            tti_seconds: 60,
        };
        let service = CodeUnitService::with_cache_config(storage.clone(), config);

        // Insert 100 units
        rt.block_on(async {
            for i in 0..100 {
                let unit = create_test_unit(
                    &format!("size_bench:{}", i),
                    &format!("func_{}", i),
                    &format!("module::func_{}", i),
                );
                let pooled = storage.acquire().await.unwrap();
                let conn = pooled.connection();
                let query = "CREATE code_unit CONTENT $unit";
                conn.query(query).bind(("unit", &unit)).await.unwrap();
            }
        });

        group.bench_with_input(
            BenchmarkId::new("cache_size", size),
            &size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let id = format!("size_bench:{}", fastrand::usize(0..100));
                    black_box(service.get_code_unit(&id).await.unwrap())
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_reads(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_reads");

    let (_storage, service) = rt.block_on(setup_service_with_units(100));
    let service = Arc::new(service);

    // Warm cache
    rt.block_on(async {
        for i in 0..10 {
            service.get_code_unit(&format!("bench:{}", i)).await.unwrap();
        }
    });

    let concurrency_levels = vec![1, 10, 50, 100];

    for level in concurrency_levels {
        group.throughput(Throughput::Elements(level as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent", level),
            &level,
            |b, &level| {
                b.to_async(&rt).iter(|| {
                    let service = service.clone();
                    async move {
                        let mut handles = vec![];
                        for _ in 0..level {
                            let service = service.clone();
                            let handle = tokio::spawn(async move {
                                let id = format!("bench:{}", fastrand::usize(0..10));
                                service.get_code_unit(&id).await
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            black_box(handle.await.unwrap().unwrap());
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_invalidation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_invalidation");

    let (_storage, service) = rt.block_on(setup_service_with_units(100));

    // Warm cache
    rt.block_on(async {
        service.get_code_unit("bench:0").await.unwrap();
    });

    group.bench_function("invalidate_single", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(
                service
                    .update_code_unit(
                        "bench:0",
                        Some("{ updated }".to_string()),
                        None,
                        None,
                    )
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("mixed_workload");

    let (_storage, service) = rt.block_on(setup_service_with_units(1000));
    let service = Arc::new(service);

    // Simulate realistic workload: 80% reads (hot set), 15% reads (cold set), 5% writes
    group.bench_function("realistic_80_15_5", |b| {
        b.to_async(&rt).iter(|| {
            let service = service.clone();
            async move {
                let rand = fastrand::u8(0..100);

                if rand < 80 {
                    // Hot reads (cached)
                    let id = format!("bench:{}", fastrand::usize(0..100));
                    black_box(service.get_code_unit(&id).await.unwrap())
                } else if rand < 95 {
                    // Cold reads (likely not cached)
                    let id = format!("bench:{}", fastrand::usize(100..1000));
                    black_box(service.get_code_unit(&id).await.unwrap())
                } else {
                    // Writes (invalidate cache)
                    let id = format!("bench:{}", fastrand::usize(0..100));
                    black_box(
                        service
                            .update_code_unit(&id, Some("{ }".to_string()), None, None)
                            .await
                            .unwrap(),
                    )
                }
            }
        });
    });

    group.finish();
}

fn bench_qualified_name_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("qualified_name_lookup");

    let (_storage, service) = rt.block_on(setup_service_with_units(100));

    // Warm cache via ID lookup
    rt.block_on(async {
        service.get_code_unit("bench:0").await.unwrap();
    });

    group.bench_function("by_id_cached", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(service.get_code_unit("bench:0").await.unwrap())
        });
    });

    group.bench_function("by_qualified_name_cached", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(
                service
                    .get_by_qualified_name("module::func_0")
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

fn bench_cache_stats_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_stats");

    let (_storage, service) = rt.block_on(setup_service_with_units(10));

    group.bench_function("get_stats", |b| {
        b.iter(|| {
            black_box(service.cache_stats());
        });
    });

    group.bench_function("with_reads", |b| {
        b.to_async(&rt).iter(|| async {
            service.get_code_unit("bench:0").await.unwrap();
            black_box(service.cache_stats())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_hit_vs_miss,
    bench_different_cache_sizes,
    bench_concurrent_reads,
    bench_cache_invalidation,
    bench_mixed_workload,
    bench_qualified_name_lookup,
    bench_cache_stats_overhead
);

criterion_main!(benches);
