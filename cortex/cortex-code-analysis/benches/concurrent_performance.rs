//! Benchmark for concurrent file processing performance
//!
//! Compares different concurrent processing strategies:
//! - Basic sync runner
//! - Enhanced producer-consumer
//! - Parallel processor with Rayon
//! - Batch processor
//!
//! Run with: cargo bench --bench concurrent_performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use cortex_code_analysis::{
    ConcurrentRunner, FilesData,
    EnhancedProducerConsumer, ProducerConsumerConfig,
    ParallelProcessor, ParallelConfig,
    BatchProcessor, BatchStrategy,
    RustParser,
};
use globset::GlobSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create test files of various sizes
fn create_test_files(count: usize) -> (TempDir, Vec<PathBuf>) {
    let temp = TempDir::new().unwrap();
    let mut files = Vec::new();

    for i in 0..count {
        let file = temp.path().join(format!("test{}.rs", i));

        // Create files with varying complexity
        let content = if i % 3 == 0 {
            // Simple file
            format!(
                "fn test_{}() {{\n    println!(\"Hello\");\n}}\n",
                i
            )
        } else if i % 3 == 1 {
            // Medium file
            format!(
                "fn test_{}(a: i32, b: i32) -> i32 {{\n    if a > b {{\n        a + b\n    }} else {{\n        a - b\n    }}\n}}\n\nfn helper_{}() {{\n    let x = 42;\n}}\n",
                i, i
            )
        } else {
            // Complex file
            format!(
                "struct Data_{} {{\n    value: i32,\n}}\n\nimpl Data_{} {{\n    fn new(v: i32) -> Self {{\n        Self {{ value: v }}\n    }}\n\n    fn process(&self) -> i32 {{\n        match self.value {{\n            0..=10 => self.value * 2,\n            11..=20 => self.value * 3,\n            _ => self.value,\n        }}\n    }}\n}}\n",
                i, i
            )
        };

        fs::write(&file, content).unwrap();
        files.push(file);
    }

    (temp, files)
}

/// Benchmark basic sync runner
fn bench_sync_runner(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_runner");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_temp, files) = create_test_files(size);

            b.iter(|| {
                let runner = ConcurrentRunner::new(4, |path, _: &()| {
                    let _source = std::fs::read_to_string(&path)?;
                    Ok(())
                });

                let files_data = FilesData {
                    paths: files.clone(),
                    include: GlobSet::empty(),
                    exclude: GlobSet::empty(),
                };

                black_box(runner.run((), files_data).unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark enhanced producer-consumer
fn bench_producer_consumer(c: &mut Criterion) {
    let mut group = c.benchmark_group("producer_consumer");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_temp, files) = create_test_files(size);

            b.iter(|| {
                let config = ProducerConsumerConfig {
                    num_workers: 4,
                    channel_capacity: 100,
                    parallel_discovery: true,
                    max_retries: 0,
                    retry_delay_ms: 0,
                    graceful_errors: true,
                };

                let processor = EnhancedProducerConsumer::new(
                    |path, _: &()| {
                        let _source = std::fs::read_to_string(&path)?;
                        Ok(())
                    },
                    config,
                );

                black_box(
                    processor
                        .run((), files.clone(), GlobSet::empty(), GlobSet::empty())
                        .unwrap(),
                );
            });
        });
    }

    group.finish();
}

/// Benchmark parallel processor
fn bench_parallel_processor(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_processor");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_temp, files) = create_test_files(size);

            b.iter(|| {
                let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
                    let _source = std::fs::read_to_string(path)?;
                    Ok(())
                });

                black_box(processor.process_all(files.clone(), ()).unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark batch processor
fn bench_batch_processor(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_processor");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let (_temp, files) = create_test_files(size);

            b.iter(|| {
                let processor = BatchProcessor::new(|batch: Vec<PathBuf>, _: &()| {
                    let results: Vec<()> = batch
                        .into_iter()
                        .filter_map(|path| std::fs::read_to_string(&path).ok().map(|_| ()))
                        .collect();
                    Ok(results)
                })
                .with_strategy(BatchStrategy::Fixed(10));

                black_box(processor.process_batches(files.clone(), ()).unwrap());
            });
        });
    }

    group.finish();
}

/// Benchmark with actual parsing
fn bench_with_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_parsing");

    for size in [10, 50].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("parallel", size),
            size,
            |b, &size| {
                let (_temp, files) = create_test_files(size);

                b.iter(|| {
                    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
                        let source = std::fs::read_to_string(path)?;
                        let mut parser = RustParser::new()?;
                        let _parsed = parser.parse_file("test.rs", &source)?;
                        Ok(())
                    });

                    black_box(processor.process_all(files.clone(), ()).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark different thread counts
fn bench_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_scaling");
    let (_temp, files) = create_test_files(100);

    for threads in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(threads),
            threads,
            |b, &threads| {
                b.iter(|| {
                    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
                        let _source = std::fs::read_to_string(path)?;
                        Ok(())
                    })
                    .with_threads(threads);

                    black_box(processor.process_all(files.clone(), ()).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch size impact
fn bench_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_sizes");
    let (_temp, files) = create_test_files(100);

    for batch_size in [5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let processor = BatchProcessor::new(|batch: Vec<PathBuf>, _: &()| {
                        let results: Vec<()> = batch
                            .into_iter()
                            .filter_map(|path| std::fs::read_to_string(&path).ok().map(|_| ()))
                            .collect();
                        Ok(results)
                    })
                    .with_strategy(BatchStrategy::Fixed(batch_size));

                    black_box(processor.process_batches(files.clone(), ()).unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_sync_runner,
    bench_producer_consumer,
    bench_parallel_processor,
    bench_batch_processor,
    bench_with_parsing,
    bench_thread_scaling,
    bench_batch_sizes,
);

criterion_main!(benches);
