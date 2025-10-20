//! Benchmarks for embedding generation performance.

use cortex_semantic::providers::{EmbeddingProvider, MockProvider};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn bench_single_embedding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let provider = MockProvider::new(384);

    c.bench_function("generate_single_embedding", |b| {
        b.iter(|| {
            rt.block_on(async {
                provider
                    .embed(black_box("This is a test sentence for embedding generation"))
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_batch_embeddings(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let provider = MockProvider::new(384);

    let mut group = c.benchmark_group("batch_embeddings");

    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), batch_size, |b, &size| {
            let texts: Vec<String> = (0..size)
                .map(|i| format!("Test sentence number {}", i))
                .collect();

            b.iter(|| {
                rt.block_on(async {
                    provider
                        .embed_batch(black_box(&texts))
                        .await
                        .unwrap()
                })
            })
        });
    }

    group.finish();
}

fn bench_embedding_dimensions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("embedding_dimensions");

    for dim in [128, 384, 768, 1536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(dim), dim, |b, &dimension| {
            let provider = MockProvider::new(dimension);

            b.iter(|| {
                rt.block_on(async {
                    provider
                        .embed(black_box("Test sentence"))
                        .await
                        .unwrap()
                })
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_embedding,
    bench_batch_embeddings,
    bench_embedding_dimensions,
);

criterion_main!(benches);
