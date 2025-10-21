//! Semantic Search Performance Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Vector search (target: <100ms for 1M vectors)
//! - Hybrid search (keyword + semantic)
//! - Index building and updates
//! - Re-ranking performance

use cortex_semantic::{
    search::{SemanticSearch, SearchConfig, SearchResult},
    providers::{EmbeddingProvider, MockEmbeddingProvider},
    cache::EmbeddingCache,
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
        namespace: "semantic_bench_ns".to_string(),
        database: "semantic_bench_db".to_string(),
    }
}

async fn setup_semantic_search() -> (Arc<ConnectionManager>, SemanticSearch) {
    let config = create_test_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    let manager = Arc::new(manager);

    // Create semantic search with mock provider for benchmarking
    let provider = Arc::new(MockEmbeddingProvider::new(384)); // Standard embedding dimension
    let cache = EmbeddingCache::new(1000);

    let search_config = SearchConfig {
        max_results: 10,
        min_similarity: 0.7,
        use_hybrid_search: false,
        keyword_weight: 0.3,
        semantic_weight: 0.7,
    };

    let search = SemanticSearch::new(
        manager.clone(),
        provider,
        cache,
        search_config,
    );

    (manager, search)
}

fn generate_mock_embedding(dimension: usize, seed: u64) -> Vec<f32> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let mut hasher = RandomState::new().build_hasher();
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    let mut embedding = Vec::with_capacity(dimension);
    let mut rng_state = hash;

    for _ in 0..dimension {
        // Simple LCG for reproducible random numbers
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let val = ((rng_state >> 16) & 0x7fff) as f32 / 32768.0;
        embedding.push(val * 2.0 - 1.0); // Scale to [-1, 1]
    }

    // Normalize
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in &mut embedding {
            *val /= magnitude;
        }
    }

    embedding
}

async fn populate_index(
    search: &SemanticSearch,
    count: usize,
) {
    let workspace_id = Uuid::new_v4();

    for i in 0..count {
        let code_unit = CodeUnit {
            id: Uuid::new_v4(),
            workspace_id,
            path: format!("/src/module_{}.rs", i),
            name: format!("function_{}", i),
            kind: "function".to_string(),
            content: format!(
                "pub fn function_{}(x: i32) -> i32 {{\n\
                 \t// This is function number {}\n\
                 \tx * 2 + {}\n\
                 }}",
                i, i, i
            ),
            start_line: 1,
            end_line: 4,
            language: "rust".to_string(),
            metadata: serde_json::Value::Null,
        };

        let embedding = generate_mock_embedding(384, i as u64);

        search.index_code_unit(&code_unit, embedding)
            .await
            .expect("Failed to index code unit");
    }
}

// ==============================================================================
// Vector Search Benchmarks
// ==============================================================================

fn bench_vector_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, search) = rt.block_on(setup_semantic_search());

    let mut group = c.benchmark_group("vector_search");
    group.significance_level(0.05).sample_size(50);

    // Test with different index sizes
    for size in [100, 1_000, 10_000].iter() {
        rt.block_on(populate_index(&search, *size));

        // Vector search - Target: <100ms for all sizes
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("search_top_10", size),
            size,
            |b, _size| {
                let query_embedding = generate_mock_embedding(384, 999999);
                b.to_async(&rt).iter(|| async {
                    let results = search
                        .search_by_embedding(&query_embedding, 10, 0.0)
                        .await
                        .unwrap();
                    black_box(results);
                });
            },
        );

        // Top-K with different K values
        for k in [1, 5, 10, 20, 50].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("search_top_{}_in_{}", k, size), k),
                k,
                |b, &k_val| {
                    let query_embedding = generate_mock_embedding(384, 888888);
                    b.to_async(&rt).iter(|| async {
                        let results = search
                            .search_by_embedding(&query_embedding, k_val, 0.0)
                            .await
                            .unwrap();
                        black_box(results);
                    });
                },
            );
        }
    }

    group.finish();
}

// ==============================================================================
// Hybrid Search Benchmarks
// ==============================================================================

fn bench_hybrid_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, search) = rt.block_on(setup_semantic_search());

    // Populate with 10K vectors
    rt.block_on(populate_index(&search, 10_000));

    let mut group = c.benchmark_group("hybrid_search");
    group.significance_level(0.05).sample_size(50);

    // Hybrid search (keyword + semantic) - Target: <150ms
    group.bench_function("hybrid_keyword_semantic", |b| {
        let query = "function that multiplies by 2";
        let query_embedding = generate_mock_embedding(384, 777777);

        b.to_async(&rt).iter(|| async {
            let results = search
                .hybrid_search(query, &query_embedding, 10, 0.3, 0.7)
                .await
                .unwrap();
            black_box(results);
        });
    });

    // Pure keyword search - Target: <50ms
    group.bench_function("keyword_only", |b| {
        let query = "function multiply";

        b.to_async(&rt).iter(|| async {
            let results = search
                .keyword_search(query, 10)
                .await
                .unwrap();
            black_box(results);
        });
    });

    // Re-ranking top results - Target: <20ms
    group.bench_function("rerank_top_100", |b| {
        let query_embedding = generate_mock_embedding(384, 666666);

        b.to_async(&rt).iter(|| async {
            // First get top 100 candidates
            let candidates = search
                .search_by_embedding(&query_embedding, 100, 0.0)
                .await
                .unwrap();

            // Re-rank to top 10
            let reranked = search
                .rerank_results(&candidates, &query_embedding, 10)
                .await
                .unwrap();
            black_box(reranked);
        });
    });

    group.finish();
}

// ==============================================================================
// Index Building Benchmarks
// ==============================================================================

fn bench_index_building(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("index_building");
    group.significance_level(0.05).sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // Build index for various sizes
    for size in [100, 1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("build_hnsw_index", size),
            size,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let (manager, search) = setup_semantic_search().await;
                    populate_index(&search, count).await;
                });
            },
        );
    }

    group.finish();
}

// ==============================================================================
// Incremental Index Updates Benchmarks
// ==============================================================================

fn bench_incremental_updates(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, search) = rt.block_on(setup_semantic_search());

    // Start with 10K vectors
    rt.block_on(populate_index(&search, 10_000));

    let mut group = c.benchmark_group("incremental_updates");
    group.significance_level(0.05).sample_size(50);

    // Single vector insertion - Target: <10ms
    group.bench_function("insert_single_vector", |b| {
        let mut counter = 10_000;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let workspace_id = Uuid::new_v4();

            let code_unit = CodeUnit {
                id: Uuid::new_v4(),
                workspace_id,
                path: format!("/src/new_{}.rs", counter),
                name: format!("new_function_{}", counter),
                kind: "function".to_string(),
                content: format!("pub fn new_function_{}() {{}}", counter),
                start_line: 1,
                end_line: 1,
                language: "rust".to_string(),
                metadata: serde_json::Value::Null,
            };

            let embedding = generate_mock_embedding(384, counter);

            search.index_code_unit(&code_unit, embedding)
                .await
                .unwrap();
        });
    });

    // Batch insertion - Target: <100ms for 100 vectors
    group.throughput(Throughput::Elements(100));
    group.bench_function("insert_batch_100_vectors", |b| {
        let mut batch_counter = 0;
        b.to_async(&rt).iter(|| async {
            batch_counter += 1;
            let workspace_id = Uuid::new_v4();

            for i in 0..100 {
                let idx = batch_counter * 100 + i;
                let code_unit = CodeUnit {
                    id: Uuid::new_v4(),
                    workspace_id,
                    path: format!("/src/batch_{}.rs", idx),
                    name: format!("batch_function_{}", idx),
                    kind: "function".to_string(),
                    content: format!("pub fn batch_function_{}() {{}}", idx),
                    start_line: 1,
                    end_line: 1,
                    language: "rust".to_string(),
                    metadata: serde_json::Value::Null,
                };

                let embedding = generate_mock_embedding(384, idx);

                search.index_code_unit(&code_unit, embedding)
                    .await
                    .unwrap();
            }
        });
    });

    // Vector deletion - Target: <10ms
    group.bench_function("delete_single_vector", |b| {
        b.to_async(&rt).iter(|| async {
            let id = Uuid::new_v4();
            search.delete_vector(&id)
                .await
                .unwrap();
        });
    });

    // Vector update - Target: <20ms
    group.bench_function("update_single_vector", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let workspace_id = Uuid::new_v4();

            let code_unit = CodeUnit {
                id: Uuid::new_v4(),
                workspace_id,
                path: format!("/src/update_{}.rs", counter),
                name: format!("update_function_{}", counter),
                kind: "function".to_string(),
                content: format!("pub fn updated_function_{}() {{}}", counter),
                start_line: 1,
                end_line: 1,
                language: "rust".to_string(),
                metadata: serde_json::Value::Null,
            };

            let new_embedding = generate_mock_embedding(384, counter + 1_000_000);

            search.update_vector(&code_unit.id, new_embedding)
                .await
                .unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Similarity Computation Benchmarks
// ==============================================================================

fn bench_similarity_computation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("similarity_computation");
    group.significance_level(0.05).sample_size(200);

    let embedding1 = generate_mock_embedding(384, 12345);
    let embedding2 = generate_mock_embedding(384, 67890);

    // Cosine similarity - Target: <1μs
    group.bench_function("cosine_similarity", |b| {
        b.iter(|| {
            let similarity = cosine_similarity(&embedding1, &embedding2);
            black_box(similarity);
        });
    });

    // Euclidean distance - Target: <1μs
    group.bench_function("euclidean_distance", |b| {
        b.iter(|| {
            let distance = euclidean_distance(&embedding1, &embedding2);
            black_box(distance);
        });
    });

    // Batch similarity (1 query vs 100 vectors) - Target: <100μs
    let vectors: Vec<Vec<f32>> = (0..100)
        .map(|i| generate_mock_embedding(384, i))
        .collect();

    group.bench_function("batch_similarity_100", |b| {
        b.iter(|| {
            let similarities: Vec<f32> = vectors
                .iter()
                .map(|v| cosine_similarity(&embedding1, v))
                .collect();
            black_box(similarities);
        });
    });

    group.finish();
}

// Helper functions for similarity computation
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot_product / (mag_a * mag_b)
    }
}

fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

// ==============================================================================
// Cross-language Search Benchmarks
// ==============================================================================

fn bench_cross_language_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (manager, search) = rt.block_on(setup_semantic_search());

    // Populate with multi-language code units
    rt.block_on(async {
        let workspace_id = Uuid::new_v4();

        for i in 0..1000 {
            let lang = match i % 3 {
                0 => "rust",
                1 => "typescript",
                _ => "python",
            };

            let code_unit = CodeUnit {
                id: Uuid::new_v4(),
                workspace_id,
                path: format!("/src/file_{}.{}", i, lang),
                name: format!("function_{}", i),
                kind: "function".to_string(),
                content: format!("function_{} implementation", i),
                start_line: 1,
                end_line: 1,
                language: lang.to_string(),
                metadata: serde_json::Value::Null,
            };

            let embedding = generate_mock_embedding(384, i as u64);

            search.index_code_unit(&code_unit, embedding)
                .await
                .expect("Failed to index");
        }
    });

    let mut group = c.benchmark_group("cross_language_search");
    group.significance_level(0.05).sample_size(50);

    // Search across all languages - Target: <100ms
    group.bench_function("search_all_languages", |b| {
        let query_embedding = generate_mock_embedding(384, 555555);

        b.to_async(&rt).iter(|| async {
            let results = search
                .search_by_embedding(&query_embedding, 10, 0.0)
                .await
                .unwrap();
            black_box(results);
        });
    });

    // Language-filtered search - Target: <80ms
    group.bench_function("search_rust_only", |b| {
        let query_embedding = generate_mock_embedding(384, 444444);

        b.to_async(&rt).iter(|| async {
            let results = search
                .search_filtered(&query_embedding, 10, 0.0, |unit| {
                    unit.language == "rust"
                })
                .await
                .unwrap();
            black_box(results);
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
        bench_vector_search,
        bench_hybrid_search,
        bench_index_building,
        bench_incremental_updates,
        bench_similarity_computation,
        bench_cross_language_search,
);

criterion_main!(benches);
