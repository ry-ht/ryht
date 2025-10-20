//! Benchmarks for semantic search performance.

use cortex_semantic::prelude::*;
use cortex_semantic::EntityType;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use tokio::runtime::Runtime;

fn create_test_engine(rt: &Runtime) -> SemanticSearchEngine {
    rt.block_on(async {
        let mut config = cortex_semantic::config::SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.embedding.fallback_providers = vec![];
        SemanticSearchEngine::new(config).await.unwrap()
    })
}

fn bench_single_document_indexing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = create_test_engine(&rt);

    c.bench_function("index_single_document", |b| {
        b.iter(|| {
            rt.block_on(async {
                engine
                    .index_document(
                        black_box("doc1".to_string()),
                        black_box("This is a test document for benchmarking".to_string()),
                        EntityType::Document,
                        HashMap::new(),
                    )
                    .await
                    .unwrap();
            })
        })
    });
}

fn bench_batch_indexing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_indexing");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let engine = create_test_engine(&rt);
                let documents: Vec<_> = (0..size)
                    .map(|i| {
                        (
                            format!("doc{}", i),
                            format!("Document content number {}", i),
                            EntityType::Document,
                            HashMap::new(),
                        )
                    })
                    .collect();

                rt.block_on(async {
                    engine
                        .index_batch(black_box(documents))
                        .await
                        .unwrap();
                })
            })
        });
    }

    group.finish();
}

fn bench_search_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = create_test_engine(&rt);

    // Pre-index documents
    rt.block_on(async {
        let documents: Vec<_> = (0..100)
            .map(|i| {
                (
                    format!("doc{}", i),
                    format!("Document about topic {} with various keywords", i),
                    EntityType::Document,
                    HashMap::new(),
                )
            })
            .collect();

        engine.index_batch(documents).await.unwrap();
    });

    c.bench_function("search_100_docs", |b| {
        b.iter(|| {
            rt.block_on(async {
                engine
                    .search(black_box("topic keywords"), black_box(10))
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_search_with_varying_index_size(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("search_scaling");

    for size in [100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let engine = create_test_engine(&rt);

            // Pre-index documents
            rt.block_on(async {
                let documents: Vec<_> = (0..size)
                    .map(|i| {
                        (
                            format!("doc{}", i),
                            format!("Document content {}", i),
                            EntityType::Document,
                            HashMap::new(),
                        )
                    })
                    .collect();

                engine.index_batch(documents).await.unwrap();
            });

            b.iter(|| {
                rt.block_on(async {
                    engine
                        .search(black_box("content"), black_box(10))
                        .await
                        .unwrap()
                })
            })
        });
    }

    group.finish();
}

fn bench_concurrent_searches(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = std::sync::Arc::new(create_test_engine(&rt));

    // Pre-index documents
    rt.block_on(async {
        let documents: Vec<_> = (0..100)
            .map(|i| {
                (
                    format!("doc{}", i),
                    format!("Document {}", i),
                    EntityType::Document,
                    HashMap::new(),
                )
            })
            .collect();

        engine.index_batch(documents).await.unwrap();
    });

    c.bench_function("concurrent_10_searches", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::new();

                for i in 0..10 {
                    let engine_clone = engine.clone();
                    let handle = tokio::spawn(async move {
                        engine_clone
                            .search(&format!("query {}", i), 5)
                            .await
                            .unwrap()
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
}

criterion_group!(
    benches,
    bench_single_document_indexing,
    bench_batch_indexing,
    bench_search_performance,
    bench_search_with_varying_index_size,
    bench_concurrent_searches,
);

criterion_main!(benches);
