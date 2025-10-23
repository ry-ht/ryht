//! Comprehensive Qdrant Performance Benchmarks
//!
//! This benchmark suite measures and compares:
//! 1. Insert performance (single, batch, streaming)
//! 2. Search performance (various k values, with/without filters)
//! 3. Memory usage comparison
//! 4. Quantization impact analysis
//! 5. HNSW vs Qdrant performance comparison
//!
//! Run with: cargo bench --bench qdrant_benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use cortex_semantic::prelude::*;
use cortex_semantic::index::{HNSWIndex, VectorIndex, SearchResult as IndexSearchResult};
use cortex_semantic::types::{SimilarityMetric, Vector, DocumentId};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

// ============================================================================
// Configuration and Constants
// ============================================================================

const EMBEDDING_DIMENSION: usize = 384;
const SMALL_DATASET: usize = 100;
const MEDIUM_DATASET: usize = 1_000;
const LARGE_DATASET: usize = 10_000;

/// Create Qdrant configuration for benchmarking
fn create_bench_qdrant_config() -> QdrantConfig {
    let mut config = QdrantConfig::default();
    config.url = std::env::var("QDRANT_URL")
        .unwrap_or_else(|_| "http://localhost:6333".to_string());
    config.collection_name = format!("bench_{}", Uuid::new_v4());
    config.enable_quantization = false; // Baseline without quantization
    config.write_batch_size = 100;
    config
}

/// Generate deterministic test vectors for reproducible benchmarks
fn generate_test_vectors(count: usize, dimension: usize) -> Vec<(DocumentId, Vector)> {
    (0..count)
        .map(|i| {
            let id = format!("doc_{}", i);
            let vector = (0..dimension)
                .map(|j| {
                    let val = ((i * 1000 + j * 137) % 1000) as f32 / 1000.0;
                    val
                })
                .collect();
            (id, vector)
        })
        .collect()
}

/// Generate query vectors
fn generate_query_vectors(count: usize, dimension: usize) -> Vec<Vector> {
    (0..count)
        .map(|i| {
            (0..dimension)
                .map(|j| {
                    let val = ((i * 500 + j * 271) % 1000) as f32 / 1000.0;
                    val
                })
                .collect()
        })
        .collect()
}

// ============================================================================
// Benchmark Group 1: Insert Performance
// ============================================================================

fn bench_insert_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("insert_single");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    // Benchmark Qdrant single insert
    group.bench_function("qdrant", |b| {
        b.iter_batched(
            || {
                let config = create_bench_qdrant_config();
                let store = rt.block_on(async {
                    QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap()
                });
                let vectors = generate_test_vectors(10, EMBEDDING_DIMENSION);
                (store, vectors)
            },
            |(store, vectors)| {
                rt.block_on(async {
                    for (id, vector) in vectors {
                        store.insert(id, vector).await.unwrap();
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark HNSW single insert
    group.bench_function("hnsw", |b| {
        b.iter_batched(
            || {
                let store = HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine);
                let vectors = generate_test_vectors(10, EMBEDDING_DIMENSION);
                (store, vectors)
            },
            |(store, vectors)| {
                rt.block_on(async {
                    for (id, vector) in vectors {
                        store.insert(id, vector).await.unwrap();
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_insert_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("insert_batch");
    group.sample_size(10);

    for size in [SMALL_DATASET, MEDIUM_DATASET].iter() {
        let vectors = generate_test_vectors(*size, EMBEDDING_DIMENSION);

        // Benchmark Qdrant batch insert
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("qdrant", size), size, |b, _| {
            b.iter_batched(
                || {
                    let config = create_bench_qdrant_config();
                    let store = rt.block_on(async {
                        QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                            .await
                            .unwrap()
                    });
                    (store, vectors.clone())
                },
                |(store, vectors)| {
                    rt.block_on(async {
                        store.insert_batch(vectors).await.unwrap();
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });

        // Benchmark HNSW batch insert
        group.bench_with_input(BenchmarkId::new("hnsw", size), size, |b, _| {
            b.iter_batched(
                || {
                    let store = HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine);
                    (store, vectors.clone())
                },
                |(store, vectors)| {
                    rt.block_on(async {
                        store.insert_batch(vectors).await.unwrap();
                    });
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 2: Search Performance
// ============================================================================

fn bench_search_varying_k(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("search_varying_k");
    group.sample_size(50);

    // Setup: Pre-populate stores
    let vectors = generate_test_vectors(MEDIUM_DATASET, EMBEDDING_DIMENSION);
    let queries = generate_query_vectors(10, EMBEDDING_DIMENSION);

    let qdrant_store = rt.block_on(async {
        let config = create_bench_qdrant_config();
        let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
            .await
            .unwrap();
        store.insert_batch(vectors.clone()).await.unwrap();
        // Wait for indexing
        tokio::time::sleep(Duration::from_millis(500)).await;
        Arc::new(store)
    });

    let hnsw_store = rt.block_on(async {
        let store = HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine);
        store.insert_batch(vectors.clone()).await.unwrap();
        Arc::new(store)
    });

    for k in [1, 5, 10, 20, 50].iter() {
        // Benchmark Qdrant search
        group.bench_with_input(BenchmarkId::new("qdrant", k), k, |b, k| {
            let store = qdrant_store.clone();
            let queries = queries.clone();
            b.iter(|| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, *k).await.unwrap());
                    }
                });
            });
        });

        // Benchmark HNSW search
        group.bench_with_input(BenchmarkId::new("hnsw", k), k, |b, k| {
            let store = hnsw_store.clone();
            let queries = queries.clone();
            b.iter(|| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, *k).await.unwrap());
                    }
                });
            });
        });
    }

    group.finish();
}

fn bench_search_dataset_size(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("search_dataset_size");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(15));

    for size in [SMALL_DATASET, MEDIUM_DATASET, LARGE_DATASET].iter() {
        let vectors = generate_test_vectors(*size, EMBEDDING_DIMENSION);
        let queries = generate_query_vectors(5, EMBEDDING_DIMENSION);

        // Benchmark Qdrant
        group.throughput(Throughput::Elements(queries.len() as u64));
        group.bench_with_input(BenchmarkId::new("qdrant", size), size, |b, _| {
            let store = rt.block_on(async {
                let config = create_bench_qdrant_config();
                let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                    .await
                    .unwrap();
                store.insert_batch(vectors.clone()).await.unwrap();
                tokio::time::sleep(Duration::from_millis(1000)).await;
                store
            });

            let queries = queries.clone();
            b.iter(|| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, 10).await.unwrap());
                    }
                });
            });
        });

        // Benchmark HNSW
        group.bench_with_input(BenchmarkId::new("hnsw", size), size, |b, _| {
            let store = rt.block_on(async {
                let store = HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine);
                store.insert_batch(vectors.clone()).await.unwrap();
                store
            });

            let queries = queries.clone();
            b.iter(|| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, 10).await.unwrap());
                    }
                });
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 3: Quantization Impact
// ============================================================================

fn bench_quantization_impact(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("quantization_impact");
    group.sample_size(10);

    let vectors = generate_test_vectors(MEDIUM_DATASET, EMBEDDING_DIMENSION);
    let queries = generate_query_vectors(20, EMBEDDING_DIMENSION);

    // Without quantization
    group.bench_function("no_quantization", |b| {
        b.iter_batched(
            || {
                let mut config = create_bench_qdrant_config();
                config.enable_quantization = false;
                let store = rt.block_on(async {
                    let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap();
                    store.insert_batch(vectors.clone()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    store
                });
                (store, queries.clone())
            },
            |(store, queries)| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, 10).await.unwrap());
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // With scalar quantization
    group.bench_function("scalar_quantization", |b| {
        b.iter_batched(
            || {
                let mut config = create_bench_qdrant_config();
                config.enable_quantization = true;
                config.quantization_type = QuantizationType::Scalar;
                let store = rt.block_on(async {
                    let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap();
                    store.insert_batch(vectors.clone()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    store
                });
                (store, queries.clone())
            },
            |(store, queries)| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, 10).await.unwrap());
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // With product quantization
    group.bench_function("product_quantization", |b| {
        b.iter_batched(
            || {
                let mut config = create_bench_qdrant_config();
                config.enable_quantization = true;
                config.quantization_type = QuantizationType::Product;
                let store = rt.block_on(async {
                    let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap();
                    store.insert_batch(vectors.clone()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    store
                });
                (store, queries.clone())
            },
            |(store, queries)| {
                rt.block_on(async {
                    for query in &queries {
                        black_box(store.search(query, 10).await.unwrap());
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark Group 4: Concurrent Operations
// ============================================================================

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_operations");
    group.sample_size(10);

    let vectors = generate_test_vectors(MEDIUM_DATASET, EMBEDDING_DIMENSION);
    let queries = generate_query_vectors(100, EMBEDDING_DIMENSION);

    // Sequential baseline
    group.bench_function("sequential_search", |b| {
        let store = rt.block_on(async {
            let config = create_bench_qdrant_config();
            let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                .await
                .unwrap();
            store.insert_batch(vectors.clone()).await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
            Arc::new(store)
        });

        let queries = queries.clone();
        b.iter(|| {
            rt.block_on(async {
                for query in &queries {
                    black_box(store.search(query, 10).await.unwrap());
                }
            });
        });
    });

    // Concurrent searches
    for concurrency in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_search", concurrency),
            concurrency,
            |b, &concurrency| {
                let store = rt.block_on(async {
                    let config = create_bench_qdrant_config();
                    let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap();
                    store.insert_batch(vectors.clone()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    Arc::new(store)
                });

                let queries = queries.clone();
                b.iter(|| {
                    rt.block_on(async {
                        let mut handles = vec![];
                        let queries_per_task = queries.len() / concurrency;

                        for chunk_id in 0..concurrency {
                            let store_clone = store.clone();
                            let start = chunk_id * queries_per_task;
                            let end = (chunk_id + 1) * queries_per_task;
                            let chunk_queries: Vec<_> = queries[start..end].to_vec();

                            let handle = tokio::spawn(async move {
                                for query in chunk_queries {
                                    black_box(store_clone.search(&query, 10).await.unwrap());
                                }
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark Group 5: Mixed Workload
// ============================================================================

fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("mixed_workload");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let initial_vectors = generate_test_vectors(MEDIUM_DATASET, EMBEDDING_DIMENSION);
    let insert_vectors = generate_test_vectors(100, EMBEDDING_DIMENSION);
    let queries = generate_query_vectors(50, EMBEDDING_DIMENSION);

    group.bench_function("qdrant_mixed", |b| {
        b.iter_batched(
            || {
                let config = create_bench_qdrant_config();
                let store = rt.block_on(async {
                    let store = QdrantVectorStore::new(config, EMBEDDING_DIMENSION, SimilarityMetric::Cosine)
                        .await
                        .unwrap();
                    store.insert_batch(initial_vectors.clone()).await.unwrap();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    Arc::new(store)
                });
                (store, insert_vectors.clone(), queries.clone())
            },
            |(store, insert_vectors, queries)| {
                rt.block_on(async {
                    // Interleave inserts and searches
                    for i in 0..10 {
                        // Insert batch
                        let batch = insert_vectors[i * 10..(i + 1) * 10].to_vec();
                        store.insert_batch(batch).await.unwrap();

                        // Perform searches
                        for query in &queries[i * 5..(i + 1) * 5] {
                            black_box(store.search(query, 10).await.unwrap());
                        }
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("hnsw_mixed", |b| {
        b.iter_batched(
            || {
                let store = rt.block_on(async {
                    let store = HNSWIndex::new(EMBEDDING_DIMENSION, SimilarityMetric::Cosine);
                    store.insert_batch(initial_vectors.clone()).await.unwrap();
                    Arc::new(store)
                });
                (store, insert_vectors.clone(), queries.clone())
            },
            |(store, insert_vectors, queries)| {
                rt.block_on(async {
                    for i in 0..10 {
                        let batch = insert_vectors[i * 10..(i + 1) * 10].to_vec();
                        store.insert_batch(batch).await.unwrap();

                        for query in &queries[i * 5..(i + 1) * 5] {
                            black_box(store.search(query, 10).await.unwrap());
                        }
                    }
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(2));
    targets =
        bench_insert_single,
        bench_insert_batch,
        bench_search_varying_k,
        bench_search_dataset_size,
        bench_quantization_impact,
        bench_concurrent_operations,
        bench_mixed_workload
}

criterion_main!(benches);
