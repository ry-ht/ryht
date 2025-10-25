//! Advanced Concurrent Processing Example
//!
//! Demonstrates all concurrent processing features:
//! - Producer-consumer with backpressure
//! - Parallel processing with work stealing
//! - Progress tracking
//! - File caching
//! - Batch processing
//! - Comprehensive metrics

use anyhow::Result;
use cortex_code_analysis::{
    Lang, RustParser,
    metrics::{CodeMetrics, MetricsStrategy},
    // Enhanced concurrent types
    ParallelProcessor, ParallelProcessorBuilder, ParallelConfig,
    BatchProcessor, BatchStrategy, SortStrategy,
    ProgressTracker, ProgressConfig,
    FileCache, CacheConfig,
    EnhancedProducerConsumer, ProducerConsumerConfig,
};
use globset::{Glob, GlobSetBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Configuration for code analysis
#[derive(Clone)]
struct AnalysisConfig {
    language: Lang,
    verbose: bool,
    cache: Arc<FileCache<CodeMetrics>>,
}

/// Results from analyzing a file
#[derive(Debug, Clone)]
struct FileMetrics {
    path: PathBuf,
    metrics: CodeMetrics,
    cached: bool,
}

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   Advanced Concurrent Code Analysis - Feature Demonstration  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Discover files to analyze
    let files = discover_rust_files("src")?;
    println!("📁 Discovered {} Rust files\n", files.len());

    if files.is_empty() {
        println!("No Rust files found in src/. Exiting.");
        return Ok(());
    }

    // Initialize shared cache
    let cache = Arc::new(FileCache::with_capacity(1000));

    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Demo 1: Parallel Processing with Rayon");
    println!("───────────────────────────────────────────────────────────────");
    demo_parallel_processing(&files, Arc::clone(&cache))?;

    println!("\n═══════════════════════════════════════════════════════════════\n");
    println!("Demo 2: Producer-Consumer with Backpressure");
    println!("───────────────────────────────────────────────────────────────");
    demo_producer_consumer(&files, Arc::clone(&cache))?;

    println!("\n═══════════════════════════════════════════════════════════════\n");
    println!("Demo 3: Batch Processing with Adaptive Sizing");
    println!("───────────────────────────────────────────────────────────────");
    demo_batch_processing(&files, Arc::clone(&cache))?;

    println!("\n═══════════════════════════════════════════════════════════════\n");
    println!("Demo 4: Caching Performance Comparison");
    println!("───────────────────────────────────────────────────────────────");
    demo_caching_comparison(&files)?;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                    All Demos Complete!                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Demo 1: Parallel processing with Rayon
fn demo_parallel_processing(files: &[PathBuf], cache: Arc<FileCache<CodeMetrics>>) -> Result<()> {
    let config = AnalysisConfig {
        language: Lang::Rust,
        verbose: false,
        cache: Arc::clone(&cache),
    };

    // Create parallel processor with custom configuration
    let processor = ParallelProcessorBuilder::new(|path: &PathBuf, config: &AnalysisConfig| {
        analyze_file_with_cache(path, config)
    })
    .num_threads(4)
    .adaptive_batching(true)
    .build();

    println!("🚀 Processing with 4 threads (work-stealing enabled)...");
    let start = Instant::now();

    let (results, stats) = processor.process_all(files.to_vec(), config)?;

    let duration = start.elapsed();

    println!("\n✅ Results:");
    println!("   Files processed: {}", stats.files_processed);
    println!("   Files failed:    {}", stats.files_failed);
    println!("   Success rate:    {:.1}%", stats.success_rate());
    println!("   Duration:        {:.2}s", duration.as_secs_f64());
    println!("   Throughput:      {:.1} files/s", stats.throughput_fps);
    println!("   Data processed:  {:.2} MB/s", stats.throughput_mbps);

    // Show top complex files
    let mut file_metrics: Vec<_> = results
        .into_iter()
        .filter_map(|r| r.result)
        .collect();

    file_metrics.sort_by(|a, b| {
        b.metrics.cyclomatic.average()
            .partial_cmp(&a.metrics.cyclomatic.average())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("\n📊 Top 3 Most Complex Files:");
    for (i, fm) in file_metrics.iter().take(3).enumerate() {
        println!("   {}. {:?}", i + 1, fm.path.file_name().unwrap_or_default());
        println!("      Cyclomatic: {:.1}", fm.metrics.cyclomatic.average());
        println!("      LOC:        {}", fm.metrics.loc.sloc());
    }

    Ok(())
}

/// Demo 2: Producer-consumer with backpressure
fn demo_producer_consumer(files: &[PathBuf], cache: Arc<FileCache<CodeMetrics>>) -> Result<()> {
    let config = AnalysisConfig {
        language: Lang::Rust,
        verbose: false,
        cache: Arc::clone(&cache),
    };

    let pc_config = ProducerConsumerConfig {
        num_workers: 4,
        channel_capacity: 100, // Bounded channel for backpressure
        parallel_discovery: true,
        max_retries: 2,
        retry_delay_ms: 50,
        graceful_errors: true,
    };

    let processor = EnhancedProducerConsumer::new(
        move |path: PathBuf, config: &AnalysisConfig| {
            analyze_file_with_cache(&path, config)?;
            Ok(())
        },
        pc_config,
    );

    println!("🔄 Processing with producer-consumer (bounded channel: 100)...");
    let start = Instant::now();

    let (stats, errors) = processor.run(
        config,
        files.to_vec(),
        globset::GlobSet::empty(),
        globset::GlobSet::empty(),
    )?;

    let duration = start.elapsed();

    println!("\n✅ Results:");
    println!("   Files discovered: {}", stats.files_discovered);
    println!("   Files processed:  {}", stats.files_processed);
    println!("   Files failed:     {}", stats.files_failed);
    println!("   Success rate:     {:.1}%", stats.success_rate());
    println!("   Duration:         {:.2}s", duration.as_secs_f64());
    println!("   Throughput:       {:.1} files/s", stats.throughput_fps);

    if !errors.is_empty() {
        println!("\n⚠️  Errors encountered: {}", errors.len());
        for (path, error) in errors.iter().take(3) {
            println!("   {:?}: {}", path.file_name().unwrap_or_default(), error);
        }
    }

    Ok(())
}

/// Demo 3: Batch processing with adaptive sizing
fn demo_batch_processing(files: &[PathBuf], cache: Arc<FileCache<CodeMetrics>>) -> Result<()> {
    let config = AnalysisConfig {
        language: Lang::Rust,
        verbose: false,
        cache: Arc::clone(&cache),
    };

    let processor = BatchProcessor::new(|batch: Vec<PathBuf>, config: &AnalysisConfig| {
        let results: Vec<FileMetrics> = batch
            .into_iter()
            .filter_map(|path| analyze_file_with_cache(&path, config).ok())
            .collect();
        Ok(results)
    })
    .with_strategy(BatchStrategy::Adaptive {
        small_threshold_kb: 50,
        batch_size: 20,
    })
    .with_sort(SortStrategy::SizeDescending);

    println!("📦 Processing with adaptive batching...");
    println!("   Small files (<50KB): batched in groups of 20");
    println!("   Large files (≥50KB): processed individually");

    let start = Instant::now();
    let (results, stats) = processor.process_batches(files.to_vec(), config)?;
    let duration = start.elapsed();

    println!("\n✅ Results:");
    println!("   Total batches:    {}", stats.total_batches);
    println!("   Batches done:     {}", stats.batches_processed);
    println!("   Files processed:  {}", stats.files_processed);
    println!("   Duration:         {:.2}s", duration.as_secs_f64());
    println!("   Throughput:       {:.1} files/s", stats.throughput_fps);

    // Aggregate metrics
    let total_loc: usize = results.iter().map(|fm| fm.metrics.loc.sloc()).sum();
    let avg_complexity: f64 = results
        .iter()
        .map(|fm| fm.metrics.cyclomatic.average())
        .sum::<f64>()
        / results.len() as f64;

    println!("\n📈 Aggregated Metrics:");
    println!("   Total LOC:         {}", total_loc);
    println!("   Avg Complexity:    {:.2}", avg_complexity);
    println!("   Files with issues: {}",
        results.iter().filter(|fm| fm.metrics.cyclomatic.average() > 10.0).count());

    Ok(())
}

/// Demo 4: Caching performance comparison
fn demo_caching_comparison(files: &[PathBuf]) -> Result<()> {
    let cache = Arc::new(FileCache::with_capacity(1000));

    let config_cached = AnalysisConfig {
        language: Lang::Rust,
        verbose: false,
        cache: Arc::clone(&cache),
    };

    let config_uncached = AnalysisConfig {
        language: Lang::Rust,
        verbose: false,
        cache: Arc::new(FileCache::with_capacity(0)), // Disabled cache
    };

    // Take a small subset for this demo
    let sample_files: Vec<_> = files.iter().take(10).cloned().collect();

    println!("🔍 Comparing cached vs uncached performance...");
    println!("   Sample size: {} files", sample_files.len());

    // First run - populate cache
    println!("\n1️⃣  First run (populating cache)...");
    let processor = ParallelProcessor::new(|path: &PathBuf, config: &AnalysisConfig| {
        analyze_file_with_cache(path, config)
    });

    let start = Instant::now();
    let _ = processor.process_all(sample_files.clone(), config_cached.clone())?;
    let first_duration = start.elapsed();
    println!("   Time: {:.3}s", first_duration.as_secs_f64());

    // Second run - using cache
    println!("\n2️⃣  Second run (using cache)...");
    let start = Instant::now();
    let _ = processor.process_all(sample_files.clone(), config_cached.clone())?;
    let cached_duration = start.elapsed();
    println!("   Time: {:.3}s", cached_duration.as_secs_f64());

    // Third run - no cache
    println!("\n3️⃣  Third run (no cache)...");
    let start = Instant::now();
    let _ = processor.process_all(sample_files.clone(), config_uncached)?;
    let uncached_duration = start.elapsed();
    println!("   Time: {:.3}s", uncached_duration.as_secs_f64());

    // Cache statistics
    let cache_stats = cache.stats();
    println!("\n💾 Cache Statistics:");
    println!("   Hits:       {}", cache_stats.hits);
    println!("   Misses:     {}", cache_stats.misses);
    println!("   Hit rate:   {:.1}%", cache_stats.hit_rate());
    println!("   Entries:    {}", cache_stats.current_size);

    let speedup = uncached_duration.as_secs_f64() / cached_duration.as_secs_f64();
    println!("\n⚡ Speedup with caching: {:.2}x faster", speedup);

    Ok(())
}

/// Analyze a file with caching support
fn analyze_file_with_cache(
    path: &Path,
    config: &AnalysisConfig,
) -> Result<FileMetrics> {
    // Check cache first
    if let Some(metrics) = config.cache.get(path) {
        return Ok(FileMetrics {
            path: path.to_path_buf(),
            metrics,
            cached: true,
        });
    }

    // Read and parse file
    let source = std::fs::read_to_string(path)?;
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file(path.to_str().unwrap_or("unknown"), &source)?;

    // Calculate metrics
    let strategy = MetricsStrategy::default();
    let metrics = strategy.calculate_all(&parsed.node, &source)?;

    // Store in cache
    let file_size = std::fs::metadata(path)?.len() as usize;
    config.cache.insert_with_size(path.to_path_buf(), metrics.clone(), file_size);

    Ok(FileMetrics {
        path: path.to_path_buf(),
        metrics,
        cached: false,
    })
}

/// Discover Rust files in a directory
fn discover_rust_files(dir: &str) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
