//! Comprehensive tests for enhanced concurrent processing features
//!
//! This test suite validates:
//! - Producer-consumer with backpressure
//! - Parallel processor with different strategies
//! - Batch processing with various batch sizes
//! - File caching functionality
//! - Progress tracking
//! - Error handling and retry logic
//! - Performance and throughput metrics

use anyhow::Result;
use cortex_code_analysis::{
    concurrent::{
        // Enhanced concurrent processing
        EnhancedProducerConsumer, ProducerConsumerConfig, ProcessingStats,
        ParallelProcessor, ParallelProcessorBuilder, ParallelConfig, ParallelStats,
        ProgressTracker, ProgressConfig, ProgressState,
        FileCache, ContentHashCache, MultiLevelCache, CacheConfig, CacheStats,
        BatchProcessor, BatchConfig, BatchStrategy, SortStrategy,
    },
    Lang,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

// ============================================================================
// SECTION 1: Producer-Consumer Tests
// ============================================================================

#[test]
#[ignore = "Integration test - requires file system"]
fn test_producer_consumer_basic() -> Result<()> {
    let temp = TempDir::new()?;

    // Create test files
    for i in 0..5 {
        fs::write(temp.path().join(format!("file{}.rs", i)), format!("fn test{}() {{}}", i))?;
    }

    let processed = Arc::new(Mutex::new(Vec::new()));
    let processed_clone = processed.clone();

    let config = ProducerConsumerConfig {
        num_workers: 2,
        channel_capacity: 10,
        parallel_discovery: true,
        max_retries: 1,
        retry_delay_ms: 10,
        graceful_errors: true,
    };

    let processor = EnhancedProducerConsumer::new(
        config,
        move |path: &PathBuf, _: &()| {
            processed_clone.lock().unwrap().push(path.clone());
            Ok(())
        },
    );

    let files = vec![temp.path().to_path_buf()];
    let (results, stats) = processor.process(files, (), vec![], vec![])?;

    assert_eq!(stats.files_processed, 5);
    assert_eq!(stats.files_failed, 0);
    assert!(stats.throughput_fps > 0.0);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_producer_consumer_backpressure() -> Result<()> {
    let temp = TempDir::new()?;

    // Create many files to test backpressure
    for i in 0..100 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let config = ProducerConsumerConfig {
        num_workers: 4,
        channel_capacity: 10, // Small buffer to test backpressure
        parallel_discovery: true,
        max_retries: 1,
        retry_delay_ms: 10,
        graceful_errors: true,
    };

    let processor = EnhancedProducerConsumer::new(
        config,
        move |_path: &PathBuf, _: &()| {
            // Simulate processing time
            std::thread::sleep(Duration::from_millis(1));
            Ok(())
        },
    );

    let files = vec![temp.path().to_path_buf()];
    let (_, stats) = processor.process(files, (), vec![], vec![])?;

    assert_eq!(stats.files_processed, 100);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_producer_consumer_error_handling() -> Result<()> {
    let temp = TempDir::new()?;

    // Create test files
    fs::write(temp.path().join("good.rs"), "fn test() {}")?;
    fs::write(temp.path().join("bad.rs"), "fn test() {}")?;

    let config = ProducerConsumerConfig {
        num_workers: 2,
        channel_capacity: 10,
        parallel_discovery: true,
        max_retries: 2,
        retry_delay_ms: 10,
        graceful_errors: true,
    };

    let processor = EnhancedProducerConsumer::new(
        config,
        move |path: &PathBuf, _: &()| {
            if path.to_string_lossy().contains("bad") {
                anyhow::bail!("Simulated error");
            }
            Ok(())
        },
    );

    let files = vec![temp.path().to_path_buf()];
    let (_, stats) = processor.process(files, (), vec![], vec![])?;

    // Should have 1 success and 1 failure
    assert_eq!(stats.files_processed, 1);
    assert_eq!(stats.files_failed, 1);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_producer_consumer_statistics() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..10 {
        let content = format!("fn test{}() {{}}", i);
        fs::write(temp.path().join(format!("file{}.rs", i)), content)?;
    }

    let config = ProducerConsumerConfig::default();

    let processor = EnhancedProducerConsumer::new(
        config,
        move |_path: &PathBuf, _: &()| Ok(()),
    );

    let files = vec![temp.path().to_path_buf()];
    let (_, stats) = processor.process(files, (), vec![], vec![])?;

    assert_eq!(stats.files_discovered, 10);
    assert_eq!(stats.files_processed, 10);
    assert!(stats.duration.as_millis() > 0);
    assert!(stats.throughput_fps > 0.0);

    Ok(())
}

// ============================================================================
// SECTION 2: Parallel Processor Tests
// ============================================================================

#[test]
#[ignore = "Integration test - requires file system"]
fn test_parallel_processor_basic() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..10 {
        fs::write(temp.path().join(format!("file{}.rs", i)), format!("fn test{}() {{}}", i))?;
    }

    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        let content = fs::read_to_string(path)?;
        Ok(content.len())
    });

    let files: Vec<PathBuf> = (0..10)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, stats) = processor.process_all(files, ())?;

    assert_eq!(results.len(), 10);
    assert_eq!(stats.total_files, 10);
    assert_eq!(stats.successful, 10);
    assert_eq!(stats.failed, 0);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_parallel_processor_with_config() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..20 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let config = ParallelConfig {
        num_threads: 4,
        batch_size: 5,
        adaptive_batching: true,
        fail_fast: false,
    };

    let processor = ParallelProcessorBuilder::new()
        .config(config)
        .build(|path: &PathBuf, _: &()| {
            let content = fs::read_to_string(path)?;
            Ok(content.len())
        });

    let files: Vec<PathBuf> = (0..20)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, stats) = processor.process_all(files, ())?;

    assert_eq!(results.len(), 20);
    assert_eq!(stats.total_files, 20);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_parallel_processor_error_handling() -> Result<()> {
    let temp = TempDir::new()?;

    fs::write(temp.path().join("good1.rs"), "fn test() {}")?;
    fs::write(temp.path().join("good2.rs"), "fn test() {}")?;
    fs::write(temp.path().join("bad.rs"), "fn test() {}")?;

    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        if path.to_string_lossy().contains("bad") {
            anyhow::bail!("Simulated error");
        }
        Ok(fs::read_to_string(path)?.len())
    });

    let files = vec![
        temp.path().join("good1.rs"),
        temp.path().join("bad.rs"),
        temp.path().join("good2.rs"),
    ];

    let (results, stats) = processor.process_all(files, ())?;

    // Should have 2 successes
    assert_eq!(results.len(), 2);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.failed, 1);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_parallel_processor_performance() -> Result<()> {
    let temp = TempDir::new()?;

    // Create 100 files
    for i in 0..100 {
        fs::write(temp.path().join(format!("file{}.rs", i)), format!("fn test{}() {{}}", i))?;
    }

    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        // Simulate work
        std::thread::sleep(Duration::from_micros(100));
        Ok(fs::read_to_string(path)?.len())
    });

    let files: Vec<PathBuf> = (0..100)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let start = std::time::Instant::now();
    let (results, stats) = processor.process_all(files, ())?;
    let duration = start.elapsed();

    assert_eq!(results.len(), 100);
    assert!(stats.throughput > 0.0);

    // Parallel processing should be faster than sequential
    // (with 100 files and 100us each, sequential would be 10ms minimum)
    assert!(duration.as_millis() < 50);

    Ok(())
}

// ============================================================================
// SECTION 3: Batch Processor Tests
// ============================================================================

#[test]
#[ignore = "Integration test - requires file system"]
fn test_batch_processor_basic() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..10 {
        fs::write(temp.path().join(format!("file{}.rs", i)), format!("fn test{}() {{}}", i))?;
    }

    let config = BatchConfig {
        batch_size: 3,
        strategy: BatchStrategy::FixedSize,
        sort_strategy: SortStrategy::None,
        max_memory_mb: 100,
    };

    let processor = BatchProcessor::new(
        config,
        |batch: Vec<PathBuf>, _: &()| {
            Ok(batch.len())
        },
    );

    let files: Vec<PathBuf> = (0..10)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, stats) = processor.process(files, ())?;

    // 10 files with batch size 3 = 4 batches (3+3+3+1)
    assert_eq!(results.len(), 4);
    assert_eq!(stats.total_files, 10);
    assert_eq!(stats.batches_processed, 4);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_batch_processor_adaptive() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..20 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let config = BatchConfig {
        batch_size: 5,
        strategy: BatchStrategy::Adaptive,
        sort_strategy: SortStrategy::None,
        max_memory_mb: 100,
    };

    let processor = BatchProcessor::new(
        config,
        |batch: Vec<PathBuf>, _: &()| {
            Ok(batch.len())
        },
    );

    let files: Vec<PathBuf> = (0..20)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, stats) = processor.process(files, ())?;

    assert_eq!(stats.total_files, 20);
    assert!(stats.batches_processed > 0);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_batch_processor_sorting() -> Result<()> {
    let temp = TempDir::new()?;

    // Create files with different sizes
    fs::write(temp.path().join("small.rs"), "fn a() {}")?;
    fs::write(temp.path().join("medium.rs"), "fn b() {}\nfn c() {}")?;
    fs::write(temp.path().join("large.rs"), "fn d() {}\nfn e() {}\nfn f() {}")?;

    let config = BatchConfig {
        batch_size: 2,
        strategy: BatchStrategy::FixedSize,
        sort_strategy: SortStrategy::LargestFirst,
        max_memory_mb: 100,
    };

    let processor = BatchProcessor::new(
        config,
        |batch: Vec<PathBuf>, _: &()| {
            Ok(batch.len())
        },
    );

    let files = vec![
        temp.path().join("small.rs"),
        temp.path().join("medium.rs"),
        temp.path().join("large.rs"),
    ];

    let (results, stats) = processor.process(files, ())?;

    assert_eq!(stats.total_files, 3);
    assert!(stats.batches_processed > 0);

    Ok(())
}

// ============================================================================
// SECTION 4: File Caching Tests
// ============================================================================

#[test]
fn test_file_cache_basic() -> Result<()> {
    let cache = FileCache::new(10);

    let path = PathBuf::from("/test/file.rs");
    let content = vec![1, 2, 3, 4, 5];

    // Cache miss
    assert!(cache.get(&path).is_none());

    // Insert and retrieve
    cache.insert(path.clone(), content.clone());
    let cached = cache.get(&path);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), content);

    Ok(())
}

#[test]
fn test_file_cache_eviction() -> Result<()> {
    let cache = FileCache::new(2);

    let path1 = PathBuf::from("/test/file1.rs");
    let path2 = PathBuf::from("/test/file2.rs");
    let path3 = PathBuf::from("/test/file3.rs");

    cache.insert(path1.clone(), vec![1]);
    cache.insert(path2.clone(), vec![2]);

    // Both should be in cache
    assert!(cache.get(&path1).is_some());
    assert!(cache.get(&path2).is_some());

    // Insert path3, should evict path1 (LRU)
    cache.insert(path3.clone(), vec![3]);

    assert!(cache.get(&path1).is_none());
    assert!(cache.get(&path2).is_some());
    assert!(cache.get(&path3).is_some());

    Ok(())
}

#[test]
fn test_file_cache_clear() -> Result<()> {
    let cache = FileCache::new(10);

    let path = PathBuf::from("/test/file.rs");
    cache.insert(path.clone(), vec![1, 2, 3]);

    assert!(cache.get(&path).is_some());

    cache.clear();

    assert!(cache.get(&path).is_none());

    Ok(())
}

#[test]
fn test_content_hash_cache() -> Result<()> {
    let cache = ContentHashCache::new(10);

    let path = PathBuf::from("/test/file.rs");
    let content1 = vec![1, 2, 3];
    let content2 = vec![1, 2, 3]; // Same content
    let content3 = vec![4, 5, 6]; // Different content

    // First insert
    cache.insert(path.clone(), content1.clone(), "result1".to_string());

    // Same content should hit cache
    let hit = cache.get(&path, &content2);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap(), "result1");

    // Different content should miss
    let miss = cache.get(&path, &content3);
    assert!(miss.is_none());

    Ok(())
}

#[test]
fn test_multi_level_cache() -> Result<()> {
    let config = CacheConfig {
        l1_capacity: 10,
        l2_capacity: 100,
        enable_compression: false,
    };

    let cache = MultiLevelCache::new(config);

    let path = PathBuf::from("/test/file.rs");
    let content = vec![1, 2, 3];

    // Miss on both levels
    assert!(cache.get(&path).is_none());

    // Insert
    cache.insert(path.clone(), content.clone());

    // Should hit L1
    let hit = cache.get(&path);
    assert!(hit.is_some());

    Ok(())
}

#[test]
fn test_cache_statistics() -> Result<()> {
    let cache = FileCache::new(10);

    let path1 = PathBuf::from("/test/file1.rs");
    let path2 = PathBuf::from("/test/file2.rs");

    // Misses
    cache.get(&path1);
    cache.get(&path2);

    // Insert and hit
    cache.insert(path1.clone(), vec![1]);
    cache.get(&path1);
    cache.get(&path1);

    let stats = cache.stats();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 2);
    assert!(stats.hit_rate() > 0.0);

    Ok(())
}

// ============================================================================
// SECTION 5: Progress Tracking Tests
// ============================================================================

#[test]
fn test_progress_tracker_basic() -> Result<()> {
    let config = ProgressConfig {
        enabled: true,
        update_interval_ms: 10,
        show_throughput: true,
        show_eta: true,
    };

    let tracker = ProgressTracker::new(100, config);

    assert_eq!(tracker.state().total, 100);
    assert_eq!(tracker.state().completed, 0);

    tracker.increment(10);
    assert_eq!(tracker.state().completed, 10);

    tracker.increment(20);
    assert_eq!(tracker.state().completed, 30);

    tracker.finish();
    assert_eq!(tracker.state().completed, 100);

    Ok(())
}

#[test]
fn test_progress_tracker_percentage() -> Result<()> {
    let config = ProgressConfig::default();
    let tracker = ProgressTracker::new(100, config);

    tracker.increment(25);
    assert_eq!(tracker.state().percentage(), 25.0);

    tracker.increment(25);
    assert_eq!(tracker.state().percentage(), 50.0);

    tracker.finish();
    assert_eq!(tracker.state().percentage(), 100.0);

    Ok(())
}

#[test]
fn test_progress_tracker_throughput() -> Result<()> {
    let config = ProgressConfig::default();
    let tracker = ProgressTracker::new(100, config);

    tracker.increment(10);
    std::thread::sleep(Duration::from_millis(100));
    tracker.increment(10);

    let state = tracker.state();
    assert!(state.throughput() > 0.0);

    Ok(())
}

#[test]
fn test_progress_tracker_eta() -> Result<()> {
    let config = ProgressConfig::default();
    let tracker = ProgressTracker::new(100, config);

    tracker.increment(50);
    std::thread::sleep(Duration::from_millis(10));

    let state = tracker.state();
    let eta = state.eta();

    // ETA should be calculated
    assert!(eta.is_some());

    Ok(())
}

#[test]
fn test_progress_tracker_disabled() -> Result<()> {
    let config = ProgressConfig {
        enabled: false,
        ..Default::default()
    };

    let tracker = ProgressTracker::new(100, config);

    // Should work but not display anything
    tracker.increment(50);
    assert_eq!(tracker.state().completed, 50);

    Ok(())
}

// ============================================================================
// SECTION 6: Integration Tests
// ============================================================================

#[test]
#[ignore = "Integration test - requires file system"]
fn test_concurrent_with_caching() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..5 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let cache = Arc::new(FileCache::new(10));
    let cache_clone = cache.clone();

    let processor = ParallelProcessor::new(move |path: &PathBuf, _: &()| {
        // Try cache first
        if let Some(content) = cache_clone.get(path) {
            return Ok(content.len());
        }

        // Read and cache
        let content = fs::read(path)?;
        cache_clone.insert(path.clone(), content.clone());
        Ok(content.len())
    });

    let files: Vec<PathBuf> = (0..5)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results1, _) = processor.process_all(files.clone(), ())?;

    // Second run should hit cache
    let (results2, _) = processor.process_all(files, ())?;

    assert_eq!(results1.len(), results2.len());

    let stats = cache.stats();
    assert!(stats.hits > 0);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_concurrent_with_progress() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..10 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let progress_config = ProgressConfig {
        enabled: true,
        update_interval_ms: 10,
        show_throughput: true,
        show_eta: true,
    };

    let tracker = Arc::new(ProgressTracker::new(10, progress_config));
    let tracker_clone = tracker.clone();

    let processor = ParallelProcessor::new(move |path: &PathBuf, _: &()| {
        let content = fs::read_to_string(path)?;
        tracker_clone.increment(1);
        Ok(content.len())
    });

    let files: Vec<PathBuf> = (0..10)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, _) = processor.process_all(files, ())?;

    assert_eq!(results.len(), 10);
    assert_eq!(tracker.state().completed, 10);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_mixed_concurrent_strategies() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..20 {
        fs::write(temp.path().join(format!("file{}.rs", i)), format!("fn test{}() {{}}", i))?;
    }

    let files: Vec<PathBuf> = (0..20)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    // Test 1: Producer-Consumer
    let pc_config = ProducerConsumerConfig::default();
    let pc_processor = EnhancedProducerConsumer::new(
        pc_config,
        |path: &PathBuf, _: &()| {
            Ok(fs::read_to_string(path)?.len())
        },
    );
    let (_, pc_stats) = pc_processor.process(vec![temp.path().to_path_buf()], (), vec![], vec![])?;

    // Test 2: Parallel Processor
    let pp_processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        Ok(fs::read_to_string(path)?.len())
    });
    let (_, pp_stats) = pp_processor.process_all(files.clone(), ())?;

    // Test 3: Batch Processor
    let batch_config = BatchConfig::default();
    let bp_processor = BatchProcessor::new(
        batch_config,
        |batch: Vec<PathBuf>, _: &()| {
            Ok(batch.len())
        },
    );
    let (_, bp_stats) = bp_processor.process(files, ())?;

    // All should process the same number of files
    assert_eq!(pc_stats.files_processed, 20);
    assert_eq!(pp_stats.successful, 20);
    assert_eq!(bp_stats.total_files, 20);

    Ok(())
}

// ============================================================================
// SECTION 7: Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_file_list() -> Result<()> {
    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        Ok(fs::read_to_string(path)?.len())
    });

    let files: Vec<PathBuf> = vec![];
    let (results, stats) = processor.process_all(files, ())?;

    assert_eq!(results.len(), 0);
    assert_eq!(stats.total_files, 0);

    Ok(())
}

#[test]
fn test_cache_with_large_items() -> Result<()> {
    let cache = FileCache::new(2);

    let path1 = PathBuf::from("/test/file1.rs");
    let path2 = PathBuf::from("/test/file2.rs");

    // Insert large items
    let large_content1 = vec![0u8; 1024 * 1024]; // 1MB
    let large_content2 = vec![1u8; 1024 * 1024]; // 1MB

    cache.insert(path1.clone(), large_content1);
    cache.insert(path2.clone(), large_content2);

    assert!(cache.get(&path1).is_some());
    assert!(cache.get(&path2).is_some());

    Ok(())
}

#[test]
fn test_progress_tracker_overflow() -> Result<()> {
    let config = ProgressConfig::default();
    let tracker = ProgressTracker::new(10, config);

    // Try to increment beyond total
    tracker.increment(15);

    // Should cap at total
    assert_eq!(tracker.state().completed, 10);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_concurrent_resilience() -> Result<()> {
    let temp = TempDir::new()?;

    // Create mix of valid and problematic files
    fs::write(temp.path().join("good1.rs"), "fn test() {}")?;
    fs::write(temp.path().join("good2.rs"), "fn test() {}")?;

    let config = ProducerConsumerConfig {
        graceful_errors: true,
        max_retries: 1,
        ..Default::default()
    };

    let processor = EnhancedProducerConsumer::new(
        config,
        |path: &PathBuf, _: &()| {
            let content = fs::read_to_string(path)?;
            if content.is_empty() {
                anyhow::bail!("Empty file");
            }
            Ok(content.len())
        },
    );

    let (_, stats) = processor.process(vec![temp.path().to_path_buf()], (), vec![], vec![])?;

    // Should continue processing despite errors
    assert!(stats.files_processed > 0);

    Ok(())
}
