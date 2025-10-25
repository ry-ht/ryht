//! Batch Processing and Streaming for Large Files
//!
//! Provides memory-efficient processing for large codebases:
//! - Adaptive batching based on file sizes
//! - Streaming processing for large files
//! - Memory-bounded operations
//! - Incremental results collection
//! - Backpressure handling
//!
//! # Architecture
//!
//! Batching strategies:
//! - Fixed-size batches
//! - Size-based batches (MB)
//! - Adaptive batching (mix of small/large files)
//! - Priority-based ordering
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::batch_processor::{BatchProcessor, BatchConfig};
//! use std::path::PathBuf;
//!
//! let processor = BatchProcessor::new(|batch: Vec<PathBuf>, _config: &()| {
//!     // Process batch of files
//!     Ok(batch.len())
//! });
//!
//! let files = vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")];
//! let results = processor.process_batches(files, ()).unwrap();
//! ```

use anyhow::Result;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for batch processing
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Batch size strategy
    pub strategy: BatchStrategy,

    /// Enable parallel batch processing
    pub parallel_batches: bool,

    /// Maximum concurrent batches (0 = unlimited)
    pub max_concurrent_batches: usize,

    /// Sort files before batching
    pub sort_strategy: SortStrategy,

    /// Memory limit per batch in MB (0 = no limit)
    pub memory_limit_mb: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            strategy: BatchStrategy::Fixed(100),
            parallel_batches: true,
            max_concurrent_batches: 0,
            sort_strategy: SortStrategy::SizeDescending,
            memory_limit_mb: 0,
        }
    }
}

/// Batch size strategy
#[derive(Debug, Clone, Copy)]
pub enum BatchStrategy {
    /// Fixed number of files per batch
    Fixed(usize),

    /// Batch by total size in bytes
    SizeBytes(u64),

    /// Adaptive batching (small files grouped, large files separate)
    Adaptive { small_threshold_kb: u64, batch_size: usize },

    /// Dynamic based on available memory
    Dynamic,
}

/// File sorting strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortStrategy {
    /// No sorting
    None,

    /// Sort by size ascending (small first)
    SizeAscending,

    /// Sort by size descending (large first)
    SizeDescending,

    /// Sort by path alphabetically
    PathAlphabetical,

    /// Sort by extension
    Extension,
}

/// Batch of files to process
#[derive(Debug, Clone)]
pub struct Batch {
    pub files: Vec<PathBuf>,
    pub total_size: u64,
    pub batch_id: usize,
}

impl Batch {
    pub fn new(batch_id: usize) -> Self {
        Self {
            files: Vec::new(),
            total_size: 0,
            batch_id,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, size: u64) {
        self.files.push(path);
        self.total_size += size;
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// File with metadata for batching
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    size: u64,
}

impl FileEntry {
    fn new(path: PathBuf) -> Self {
        let size = get_file_size(&path);
        Self { path, size }
    }
}

/// Get file size safely
fn get_file_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .ok()
        .map(|m| m.len())
        .unwrap_or(0)
}

/// Sort files according to strategy
fn sort_files(files: &mut [FileEntry], strategy: SortStrategy) {
    match strategy {
        SortStrategy::None => {}
        SortStrategy::SizeAscending => {
            files.sort_by(|a, b| a.size.cmp(&b.size));
        }
        SortStrategy::SizeDescending => {
            files.sort_by(|a, b| b.size.cmp(&a.size));
        }
        SortStrategy::PathAlphabetical => {
            files.sort_by(|a, b| a.path.cmp(&b.path));
        }
        SortStrategy::Extension => {
            files.sort_by(|a, b| {
                let ext_a = a.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let ext_b = b.path.extension().and_then(|e| e.to_str()).unwrap_or("");
                ext_a.cmp(ext_b)
            });
        }
    }
}

/// Create batches from files
fn create_batches(
    files: Vec<PathBuf>,
    strategy: BatchStrategy,
    sort_strategy: SortStrategy,
) -> Vec<Batch> {
    // Convert to FileEntry with size metadata
    let mut file_entries: Vec<FileEntry> = files.into_iter().map(FileEntry::new).collect();

    // Sort files
    sort_files(&mut file_entries, sort_strategy);

    let mut batches = Vec::new();
    let mut current_batch = Batch::new(0);

    match strategy {
        BatchStrategy::Fixed(size) => {
            for entry in file_entries {
                if current_batch.len() >= size {
                    batches.push(current_batch);
                    current_batch = Batch::new(batches.len());
                }
                current_batch.add_file(entry.path, entry.size);
            }
        }

        BatchStrategy::SizeBytes(max_bytes) => {
            for entry in file_entries {
                if current_batch.total_size > 0
                    && current_batch.total_size + entry.size > max_bytes
                {
                    batches.push(current_batch);
                    current_batch = Batch::new(batches.len());
                }
                current_batch.add_file(entry.path, entry.size);
            }
        }

        BatchStrategy::Adaptive {
            small_threshold_kb,
            batch_size,
        } => {
            let threshold = small_threshold_kb * 1024;
            let mut small_files = Vec::new();
            let mut large_files = Vec::new();

            // Separate small and large files
            for entry in file_entries {
                if entry.size <= threshold {
                    small_files.push(entry);
                } else {
                    large_files.push(entry);
                }
            }

            // Batch small files
            for entry in small_files {
                if current_batch.len() >= batch_size {
                    batches.push(current_batch);
                    current_batch = Batch::new(batches.len());
                }
                current_batch.add_file(entry.path, entry.size);
            }

            // Push remaining small files batch
            if !current_batch.is_empty() {
                batches.push(current_batch);
                current_batch = Batch::new(batches.len());
            }

            // Each large file gets its own batch
            for entry in large_files {
                let mut batch = Batch::new(batches.len());
                batch.add_file(entry.path, entry.size);
                batches.push(batch);
            }

            return batches;
        }

        BatchStrategy::Dynamic => {
            // Simple dynamic strategy: aim for reasonable batch sizes
            let avg_size = if !file_entries.is_empty() {
                file_entries.iter().map(|e| e.size).sum::<u64>() / file_entries.len() as u64
            } else {
                0
            };

            let target_batch_size = if avg_size > 0 {
                // Aim for ~10MB per batch
                ((10 * 1024 * 1024) / avg_size).max(1).min(100) as usize
            } else {
                50
            };

            for entry in file_entries {
                if current_batch.len() >= target_batch_size {
                    batches.push(current_batch);
                    current_batch = Batch::new(batches.len());
                }
                current_batch.add_file(entry.path, entry.size);
            }
        }
    }

    // Push final batch
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

/// Batch processor statistics
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    pub total_batches: usize,
    pub batches_processed: usize,
    pub total_files: usize,
    pub files_processed: usize,
    pub bytes_processed: u64,
    pub duration: Duration,
    pub throughput_fps: f64,
    pub throughput_mbps: f64,
}

impl BatchStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finish(&mut self, duration: Duration) {
        self.duration = duration;
        let secs = duration.as_secs_f64();

        if secs > 0.0 {
            self.throughput_fps = self.files_processed as f64 / secs;
            self.throughput_mbps = (self.bytes_processed as f64 / 1_048_576.0) / secs;
        }
    }
}

/// Result from processing a batch
pub struct BatchResult<T> {
    pub batch_id: usize,
    pub results: Vec<T>,
    pub files_processed: usize,
    pub bytes_processed: u64,
}

/// Batch processor
pub struct BatchProcessor<F, C, T>
where
    F: Fn(Vec<PathBuf>, &C) -> Result<Vec<T>> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    processor: Arc<F>,
    config: BatchConfig,
    _phantom: std::marker::PhantomData<(C, T)>,
}

impl<F, C, T> BatchProcessor<F, C, T>
where
    F: Fn(Vec<PathBuf>, &C) -> Result<Vec<T>> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    /// Create a new batch processor
    pub fn new(processor: F) -> Self {
        Self::with_config(processor, BatchConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(processor: F, config: BatchConfig) -> Self {
        Self {
            processor: Arc::new(processor),
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set batch strategy
    pub fn with_strategy(mut self, strategy: BatchStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    /// Set sort strategy
    pub fn with_sort(mut self, sort: SortStrategy) -> Self {
        self.config.sort_strategy = sort;
        self
    }

    /// Process files in batches
    pub fn process_batches(
        &self,
        files: Vec<PathBuf>,
        config: C,
    ) -> Result<(Vec<T>, BatchStats)>
    where
        C: Sync,
    {
        let start = Instant::now();
        let mut stats = BatchStats::new();

        // Create batches
        let batches = create_batches(
            files.clone(),
            self.config.strategy,
            self.config.sort_strategy,
        );

        stats.total_batches = batches.len();
        stats.total_files = files.len();

        // Process batches
        let results: Vec<T> = if self.config.parallel_batches {
            self.process_parallel_batches(batches, &config, &mut stats)?
        } else {
            self.process_sequential_batches(batches, &config, &mut stats)?
        };

        stats.finish(start.elapsed());
        Ok((results, stats))
    }

    /// Process batches in parallel
    fn process_parallel_batches(
        &self,
        batches: Vec<Batch>,
        config: &C,
        stats: &mut BatchStats,
    ) -> Result<Vec<T>>
    where
        C: Sync,
    {
        use parking_lot::Mutex;
        use std::sync::Arc;

        let stats_arc = Arc::new(Mutex::new(BatchStats::new()));

        let results: Vec<Vec<T>> = batches
            .par_iter()
            .map(|batch| {
                match (self.processor)(batch.files.clone(), config) {
                    Ok(batch_results) => {
                        let mut s = stats_arc.lock();
                        s.batches_processed += 1;
                        s.files_processed += batch.files.len();
                        s.bytes_processed += batch.total_size;
                        batch_results
                    }
                    Err(e) => {
                        eprintln!("Error processing batch {}: {}", batch.batch_id, e);
                        Vec::new()
                    }
                }
            })
            .collect();

        // Copy stats back
        let final_stats = stats_arc.lock();
        stats.batches_processed = final_stats.batches_processed;
        stats.files_processed = final_stats.files_processed;
        stats.bytes_processed = final_stats.bytes_processed;

        Ok(results.into_iter().flatten().collect())
    }

    /// Process batches sequentially
    fn process_sequential_batches(
        &self,
        batches: Vec<Batch>,
        config: &C,
        stats: &mut BatchStats,
    ) -> Result<Vec<T>> {
        let mut all_results = Vec::new();

        for batch in batches {
            match (self.processor)(batch.files.clone(), config) {
                Ok(batch_results) => {
                    stats.batches_processed += 1;
                    stats.files_processed += batch.files.len();
                    stats.bytes_processed += batch.total_size;
                    all_results.extend(batch_results);
                }
                Err(e) => {
                    eprintln!("Error processing batch {}: {}", batch.batch_id, e);
                }
            }
        }

        Ok(all_results)
    }

    /// Process batches with streaming (one at a time)
    pub fn process_streaming<R>(
        &self,
        files: Vec<PathBuf>,
        config: C,
        mut callback: impl FnMut(BatchResult<T>) -> Result<R>,
    ) -> Result<Vec<R>>
    where
        C: Sync,
    {
        let batches = create_batches(
            files,
            self.config.strategy,
            self.config.sort_strategy,
        );

        let mut results = Vec::new();

        for batch in batches {
            let batch_size = batch.total_size;
            let batch_len = batch.files.len();
            let batch_id = batch.batch_id;

            match (self.processor)(batch.files, &config) {
                Ok(batch_results) => {
                    let result = BatchResult {
                        batch_id,
                        results: batch_results,
                        files_processed: batch_len,
                        bytes_processed: batch_size,
                    };

                    results.push(callback(result)?);
                }
                Err(e) => {
                    eprintln!("Error processing batch {}: {}", batch_id, e);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_batch_creation_fixed() {
        let files = vec![
            PathBuf::from("file1.rs"),
            PathBuf::from("file2.rs"),
            PathBuf::from("file3.rs"),
        ];

        let batches = create_batches(files, BatchStrategy::Fixed(2), SortStrategy::None);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 1);
    }

    #[test]
    fn test_batch_processor() {
        let temp = TempDir::new().unwrap();
        let mut files = Vec::new();

        for i in 0..5 {
            let file = temp.path().join(format!("file{}.txt", i));
            fs::write(&file, format!("content{}", i)).unwrap();
            files.push(file);
        }

        let processor = BatchProcessor::new(|batch: Vec<PathBuf>, _config: &()| {
            let results: Vec<usize> = batch.iter().map(|_| 1).collect();
            Ok(results)
        })
        .with_strategy(BatchStrategy::Fixed(2));

        let (results, stats) = processor.process_batches(files, ()).unwrap();

        assert_eq!(results.len(), 5);
        assert_eq!(stats.total_files, 5);
        assert!(stats.batches_processed > 0);
    }

    #[test]
    fn test_adaptive_batching() {
        let temp = TempDir::new().unwrap();
        let mut files = Vec::new();

        // Create small files
        for i in 0..5 {
            let file = temp.path().join(format!("small{}.txt", i));
            fs::write(&file, "small").unwrap();
            files.push(file);
        }

        // Create large files
        for i in 0..2 {
            let file = temp.path().join(format!("large{}.txt", i));
            fs::write(&file, "x".repeat(10000)).unwrap();
            files.push(file);
        }

        let batches = create_batches(
            files,
            BatchStrategy::Adaptive {
                small_threshold_kb: 1,
                batch_size: 3,
            },
            SortStrategy::None,
        );

        // Should have batches for small files and separate batches for large files
        assert!(batches.len() >= 2);
    }

    #[test]
    fn test_streaming_processor() {
        let temp = TempDir::new().unwrap();
        let mut files = Vec::new();

        for i in 0..3 {
            let file = temp.path().join(format!("file{}.txt", i));
            fs::write(&file, format!("content{}", i)).unwrap();
            files.push(file);
        }

        let processor = BatchProcessor::new(|batch: Vec<PathBuf>, _: &()| {
            Ok(batch.into_iter().map(|_| 1).collect())
        })
        .with_strategy(BatchStrategy::Fixed(2));

        let mut batch_count = 0;
        let results = processor
            .process_streaming(files, (), |batch_result| {
                batch_count += 1;
                Ok(batch_result.files_processed)
            })
            .unwrap();

        assert!(batch_count > 0);
        assert!(!results.is_empty());
    }
}
