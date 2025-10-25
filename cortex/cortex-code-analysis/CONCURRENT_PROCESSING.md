# Concurrent File Processing Guide

This guide covers the advanced concurrent processing capabilities in cortex-code-analysis, designed for analyzing large codebases efficiently.

## Table of Contents

1. [Overview](#overview)
2. [Architecture Comparison](#architecture-comparison)
3. [Quick Start](#quick-start)
4. [Processing Strategies](#processing-strategies)
5. [Progress Tracking](#progress-tracking)
6. [Caching](#caching)
7. [Performance Tuning](#performance-tuning)
8. [Examples](#examples)
9. [Benchmarks](#benchmarks)

## Overview

The concurrent processing system provides multiple strategies for efficient file processing:

- **Producer-Consumer**: Classic pattern with bounded channels and backpressure
- **Parallel Processing**: Rayon-based work stealing for CPU-bound tasks
- **Batch Processing**: Memory-efficient processing with adaptive batching
- **Async Processing**: Tokio-based async I/O (with `async` feature)
- **File Caching**: LRU and content-hash based caching

## Architecture Comparison

### 1. Basic Sync Runner (`ConcurrentRunner`)

**Best for**: Small to medium projects (< 1000 files)

```rust
use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};

let runner = ConcurrentRunner::new(4, |path, config: &()| {
    // Process file
    Ok(())
});
```

**Pros**:
- Simple API
- Good for basic use cases
- Backward compatible

**Cons**:
- Unbounded channels (memory growth)
- Basic error handling
- Limited statistics

### 2. Enhanced Producer-Consumer (`EnhancedProducerConsumer`)

**Best for**: Large projects with mixed file sizes

```rust
use cortex_code_analysis::concurrent::{EnhancedProducerConsumer, ProducerConsumerConfig};

let config = ProducerConsumerConfig {
    num_workers: 8,
    channel_capacity: 1000,  // Bounded channel
    parallel_discovery: true,
    max_retries: 2,
    retry_delay_ms: 100,
    graceful_errors: true,
};

let processor = EnhancedProducerConsumer::new(
    |path, config: &MyConfig| {
        // Process with retry logic
        Ok(())
    },
    config,
);

let (stats, errors) = processor.run(config, files, include, exclude)?;
```

**Features**:
- ✅ Bounded channels with backpressure
- ✅ Automatic retry on transient errors
- ✅ Parallel file discovery
- ✅ Comprehensive statistics
- ✅ Error aggregation

### 3. Parallel Processor (`ParallelProcessor`)

**Best for**: CPU-bound processing, maximum throughput

```rust
use cortex_code_analysis::concurrent::{ParallelProcessor, ParallelProcessorBuilder};

let processor = ParallelProcessorBuilder::new(|path: &PathBuf, config: &()| {
    // CPU-intensive processing
    Ok(result)
})
.num_threads(16)
.batch_size(50)
.adaptive_batching(true)
.build();

let (results, stats) = processor.process_all(files, config)?;
```

**Features**:
- ✅ Rayon work-stealing thread pool
- ✅ Zero synchronization overhead
- ✅ Adaptive batching
- ✅ Efficient for uniform workloads
- ✅ Builder pattern for configuration

### 4. Batch Processor (`BatchProcessor`)

**Best for**: Memory-bounded processing, large files

```rust
use cortex_code_analysis::concurrent::{BatchProcessor, BatchStrategy};

let processor = BatchProcessor::new(|batch: Vec<PathBuf>, config: &()| {
    // Process batch of files
    Ok(results)
})
.with_strategy(BatchStrategy::Adaptive {
    small_threshold_kb: 100,
    batch_size: 50,
})
.with_sort(SortStrategy::SizeDescending);

let (results, stats) = processor.process_batches(files, config)?;
```

**Features**:
- ✅ Adaptive batch sizing
- ✅ Memory-efficient streaming
- ✅ Multiple batching strategies
- ✅ File sorting options
- ✅ Incremental results

### 5. Async Processor (`AsyncRunner`)

**Best for**: I/O-bound operations (requires `async` feature)

```rust
use cortex_code_analysis::concurrent::{AsyncRunner, AsyncFilesData};

let runner = AsyncRunner::new(10, |path, config| async move {
    let source = tokio::fs::read_to_string(&path).await?;
    Ok(())
});

let progress = runner.run(config, files_data).await?;
```

**Features**:
- ✅ Tokio-based async I/O
- ✅ Configurable concurrency
- ✅ Progress callbacks
- ✅ Efficient for network/disk I/O

## Processing Strategies

### Batch Strategies

```rust
// Fixed size batches
BatchStrategy::Fixed(100)

// Size-based batches (50 MB per batch)
BatchStrategy::SizeBytes(50 * 1024 * 1024)

// Adaptive: small files grouped, large files separate
BatchStrategy::Adaptive {
    small_threshold_kb: 100,
    batch_size: 50,
}

// Dynamic based on available resources
BatchStrategy::Dynamic
```

### Sort Strategies

```rust
// No sorting
SortStrategy::None

// Process large files first (better load balancing)
SortStrategy::SizeDescending

// Process small files first
SortStrategy::SizeAscending

// Alphabetical ordering
SortStrategy::PathAlphabetical

// Group by file extension
SortStrategy::Extension
```

## Progress Tracking

### Basic Progress

```rust
use cortex_code_analysis::concurrent::progress::{ProgressTracker, ProgressConfig};

let tracker = ProgressTracker::new(total_files, ProgressConfig::default());

// Update progress
tracker.inc(1);
tracker.set_message("Processing...");

// Get current state
let state = tracker.state();
println!("Progress: {:.1}%", state.percentage());
println!("ETA: {:?}", state.eta);

tracker.finish();
```

### Progress with Callbacks

```rust
use cortex_code_analysis::concurrent::progress::CallbackProgressTracker;

let tracker = CallbackProgressTracker::new(total_files, config);

tracker.add_callback(|state| {
    println!("Processed: {}/{}", state.processed, state.total);
    println!("Throughput: {:.1} files/s", state.fps);
});
```

### With indicatif (requires `progress` feature)

```toml
[dependencies]
cortex-code-analysis = { version = "*", features = ["progress"] }
```

```rust
// Automatically displays progress bar
let tracker = ProgressTracker::new(total_files, ProgressConfig::default());
```

## Caching

### File Cache (LRU)

```rust
use cortex_code_analysis::concurrent::file_cache::{FileCache, CacheConfig};

let cache = FileCache::<ParsedResult>::with_capacity(1000);

// Check cache first
if let Some(result) = cache.get(&path) {
    return Ok(result);
}

// Process and cache
let result = expensive_operation(&path)?;
cache.insert(path, result.clone());
```

### Content Hash Cache (Concurrent)

```rust
use cortex_code_analysis::concurrent::file_cache::ContentHashCache;

let cache = ContentHashCache::<ParsedResult>::new(config);

let content = std::fs::read(&path)?;
let hash = ContentHashCache::<_>::hash_content(&content);

if let Some(result) = cache.get(hash) {
    return Ok(result);
}

let result = parse(&content)?;
cache.insert(hash, result, content.len());
```

### Multi-Level Cache

```rust
use cortex_code_analysis::concurrent::file_cache::MultiLevelCache;

let cache = MultiLevelCache::<ParsedResult>::with_defaults();

// Checks path cache first, then content cache
if let Some(result) = cache.get(&path, Some(&content)) {
    return Ok(result);
}

// Stores in both caches
cache.insert(path, Some(&content), result, content.len());
```

## Performance Tuning

### Thread Count

```rust
// Auto-detect (recommended)
let processor = ParallelProcessor::new(processor_fn);

// Explicit thread count
let processor = ParallelProcessor::new(processor_fn)
    .with_threads(16);

// CPU count
let threads = num_cpus::get();
```

### Channel Capacity

```rust
let config = ProducerConsumerConfig {
    // Small capacity for backpressure
    channel_capacity: 100,  // Good for memory-constrained

    // Large capacity for throughput
    channel_capacity: 10000,  // Good for fast processing

    // Unbounded
    channel_capacity: 0,
    ..Default::default()
};
```

### Batch Size Tuning

```rust
// Rule of thumb: 2-4 batches per thread
let threads = num_cpus::get();
let batch_size = total_files / (threads * 3);

let processor = BatchProcessor::new(process_fn)
    .with_strategy(BatchStrategy::Fixed(batch_size));
```

### Memory Management

```rust
// Process in streaming mode for large datasets
processor.process_streaming(files, config, |batch_result| {
    // Process results incrementally
    save_to_disk(&batch_result)?;
    Ok(())
})?;
```

## Examples

### Complete Analysis Pipeline

```rust
use cortex_code_analysis::{
    ParallelProcessor, ProgressTracker, FileCache,
    RustParser, metrics::MetricsStrategy,
};
use std::sync::Arc;

// Setup cache
let cache = Arc::new(FileCache::with_capacity(1000));

// Setup progress
let progress = ProgressTracker::with_defaults(files.len());

// Configure processor
let processor = ParallelProcessor::new(|path: &PathBuf, cache: &Arc<FileCache<_>>| {
    // Check cache
    if let Some(metrics) = cache.get(path) {
        return Ok(metrics);
    }

    // Parse and analyze
    let source = std::fs::read_to_string(path)?;
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("file", &source)?;

    let strategy = MetricsStrategy::default();
    let metrics = strategy.calculate_all(&parsed.node, &source)?;

    // Cache result
    cache.insert(path.clone(), metrics.clone());

    Ok(metrics)
});

// Process with progress
let (results, stats) = processor.process_all(files, cache)?;

progress.finish();
```

### Adaptive Processing by File Type

```rust
use cortex_code_analysis::concurrent::{BatchProcessor, BatchStrategy};

// Separate strategies for different file types
let rust_files: Vec<_> = files.iter()
    .filter(|p| p.extension() == Some("rs"))
    .cloned()
    .collect();

let ts_files: Vec<_> = files.iter()
    .filter(|p| p.extension() == Some("ts"))
    .cloned()
    .collect();

// Heavy batching for small Rust files
let rust_processor = BatchProcessor::new(process_rust)
    .with_strategy(BatchStrategy::Fixed(100));

// Light batching for larger TS files
let ts_processor = BatchProcessor::new(process_ts)
    .with_strategy(BatchStrategy::Fixed(20));

let rust_results = rust_processor.process_batches(rust_files, ())?;
let ts_results = ts_processor.process_batches(ts_files, ())?;
```

## Benchmarks

Run benchmarks to compare strategies:

```bash
# All benchmarks
cargo bench --bench concurrent_performance

# Specific benchmark
cargo bench --bench concurrent_performance -- sync_runner

# Thread scaling
cargo bench --bench concurrent_performance -- thread_scaling
```

### Expected Performance

Based on benchmarks on a typical machine:

| Strategy | Small Files (10) | Medium (50) | Large (100) |
|----------|-----------------|-------------|-------------|
| Sync Runner | ~5ms | ~25ms | ~50ms |
| Producer-Consumer | ~4ms | ~20ms | ~40ms |
| Parallel Processor | ~3ms | ~15ms | ~30ms |
| Batch Processor | ~4ms | ~18ms | ~35ms |

*Note: Actual performance depends on hardware and workload*

### Scaling Characteristics

- **1-4 threads**: ~3x speedup
- **4-8 threads**: ~1.5x speedup
- **8-16 threads**: ~1.2x speedup (diminishing returns)

## Best Practices

1. **Choose the Right Strategy**
   - CPU-bound → Parallel Processor
   - I/O-bound → Async Runner
   - Memory-constrained → Batch Processor
   - Mixed workload → Producer-Consumer

2. **Enable Caching**
   - Use cache for repeated analysis
   - Content-hash cache for dedupe
   - Monitor hit rate (target >80%)

3. **Track Progress**
   - Always show progress for long operations
   - Monitor throughput to detect bottlenecks
   - Use ETA for user feedback

4. **Handle Errors Gracefully**
   - Enable retry for transient errors
   - Log errors for debugging
   - Continue on partial failures

5. **Tune for Your Hardware**
   - Profile with different thread counts
   - Adjust batch sizes based on file sizes
   - Monitor memory usage

## Troubleshooting

### High Memory Usage

```rust
// Reduce batch size
BatchStrategy::Fixed(10)

// Enable bounded channels
channel_capacity: 100

// Use streaming
processor.process_streaming(files, config, callback)?;
```

### Low Throughput

```rust
// Increase threads
.with_threads(num_cpus::get() * 2)

// Increase batch size
BatchStrategy::Fixed(100)

// Enable parallel discovery
parallel_discovery: true
```

### Uneven Load

```rust
// Sort by size descending (process large files first)
.with_sort(SortStrategy::SizeDescending)

// Use adaptive batching
BatchStrategy::Adaptive {
    small_threshold_kb: 100,
    batch_size: 50,
}
```

## License

Same as cortex-code-analysis main license.
