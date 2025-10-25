//! Concurrent Processing Module
//!
//! This module provides comprehensive concurrent file processing with:
//! - Producer-consumer architecture with backpressure
//! - Rayon-based parallel processing with work stealing
//! - Progress tracking with throughput metrics
//! - File caching and memory pools
//! - Batch processing for large codebases
//! - Async/await support (with async feature)
//!
//! # Architecture Comparison
//!
//! ## sync_runner (Basic)
//! Simple producer-consumer with unbounded channels. Good for small to medium projects.
//!
//! ## producer_consumer (Enhanced)
//! Advanced producer-consumer with:
//! - Bounded channels with backpressure
//! - Error retry logic
//! - Comprehensive statistics
//! - Parallel file discovery
//!
//! ## parallel_processor (CPU-Bound)
//! Rayon-based parallel processing with:
//! - Work stealing for load balancing
//! - Adaptive batching
//! - Zero synchronization overhead
//!
//! ## batch_processor (Memory-Efficient)
//! Batch processing for large codebases:
//! - Adaptive batch sizing
//! - Streaming results
//! - Memory-bounded operations
//!
//! ## async_runner (I/O-Bound)
//! Tokio-based async processing with:
//! - Concurrent I/O operations
//! - Configurable concurrency limits
//! - Progress callbacks
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::{
//!     parallel_processor::{ParallelProcessor, ParallelConfig},
//!     progress::ProgressTracker,
//! };
//! use std::path::PathBuf;
//!
//! // High-performance parallel processing
//! let processor = ParallelProcessor::new(|path: &PathBuf, _config: &()| {
//!     let content = std::fs::read_to_string(path)?;
//!     Ok(content.len())
//! });
//!
//! let files = vec![PathBuf::from("src/lib.rs")];
//! let (results, stats) = processor.process_all(files, ()).unwrap();
//! ```

// Core modules
pub mod sync_runner;
pub mod producer_consumer;
pub mod parallel_processor;
pub mod progress;
pub mod file_cache;
pub mod batch_processor;

#[cfg(feature = "async")]
pub mod async_runner;

// Re-export basic sync types (backward compatible)
pub use sync_runner::{ConcurrentRunner, FilesData};

// Re-export enhanced concurrent types
pub use producer_consumer::{
    EnhancedProducerConsumer, ProducerConsumerConfig, ProcessingStats, FileResult,
};
pub use parallel_processor::{
    ParallelProcessor, ParallelProcessorBuilder, ParallelConfig, ParallelStats,
    ProcessResult,
};
pub use progress::{
    ProgressTracker, ProgressConfig, ProgressState, CallbackProgressTracker,
    SimpleProgressReporter, ProgressCallback,
};
pub use file_cache::{
    FileCache, ContentHashCache, MultiLevelCache, CacheConfig, CacheStats,
};
pub use batch_processor::{
    BatchProcessor, BatchConfig, BatchStrategy, SortStrategy, Batch, BatchStats, BatchResult,
};

// Re-export async types
#[cfg(feature = "async")]
pub use async_runner::{AsyncRunner, AsyncFilesData, AsyncProgress};
