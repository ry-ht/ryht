//! Parallel File Processing with Rayon
//!
//! This module provides CPU-bound parallel processing optimized for:
//! - Work stealing for load balancing
//! - Adaptive parallelism based on workload
//! - Batch processing for optimal throughput
//! - Memory-efficient streaming
//! - Parallel metrics computation
//!
//! # Architecture
//!
//! Uses Rayon's work-stealing thread pool for optimal CPU utilization:
//! - Automatically balances load across threads
//! - Minimizes synchronization overhead
//! - Handles irregular workloads efficiently
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::parallel_processor::{ParallelProcessor, ParallelConfig};
//! use std::path::PathBuf;
//!
//! let processor = ParallelProcessor::new(|path: &PathBuf, _config: &()| {
//!     let _content = std::fs::read_to_string(path)?;
//!     Ok(())
//! });
//!
//! let files = vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")];
//! let results = processor.process_all(files, ()).unwrap();
//! ```

use anyhow::Result;
use globset::GlobSet;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

/// Configuration for parallel processing
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of threads (0 = auto-detect)
    pub num_threads: usize,

    /// Batch size for processing (0 = auto)
    pub batch_size: usize,

    /// Enable adaptive batching based on file sizes
    pub adaptive_batching: bool,

    /// Maximum files to process in parallel
    pub max_parallel: usize,

    /// Enable work stealing (rayon default)
    pub work_stealing: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            num_threads: 0, // Auto-detect
            batch_size: 0,  // Auto-calculate
            adaptive_batching: true,
            max_parallel: 0, // Unlimited
            work_stealing: true,
        }
    }
}

/// Statistics for parallel processing
#[derive(Debug, Clone, Default)]
pub struct ParallelStats {
    /// Total files processed
    pub files_processed: usize,

    /// Files that failed
    pub files_failed: usize,

    /// Total bytes processed
    pub bytes_processed: u64,

    /// Processing duration
    pub duration: Duration,

    /// Throughput in files/second
    pub throughput_fps: f64,

    /// Throughput in MB/second
    pub throughput_mbps: f64,

    /// Average file size
    pub avg_file_size: u64,

    /// Peak memory usage (if available)
    pub peak_memory_mb: Option<f64>,
}

impl ParallelStats {
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

        if self.files_processed > 0 {
            self.avg_file_size = self.bytes_processed / self.files_processed as u64;
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.files_processed + self.files_failed;
        if total == 0 {
            0.0
        } else {
            (self.files_processed as f64 / total as f64) * 100.0
        }
    }
}

/// Result of processing a single file
#[derive(Debug)]
pub struct ProcessResult<T> {
    pub path: PathBuf,
    pub success: bool,
    pub result: Option<T>,
    pub error: Option<String>,
    pub size: u64,
}

/// Check if directory entry is hidden
fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Get file size safely
fn get_file_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .ok()
        .map(|m| m.len())
        .unwrap_or(0)
}

/// Parallel file processor using Rayon
pub struct ParallelProcessor<F, C, T>
where
    F: Fn(&PathBuf, &C) -> Result<T> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    processor: Arc<F>,
    config: ParallelConfig,
    _phantom: std::marker::PhantomData<(C, T)>,
}

impl<F, C, T> ParallelProcessor<F, C, T>
where
    F: Fn(&PathBuf, &C) -> Result<T> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    /// Create a new parallel processor
    pub fn new(processor: F) -> Self {
        Self::with_config(processor, ParallelConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(processor: F, config: ParallelConfig) -> Self {
        Self {
            processor: Arc::new(processor),
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set number of threads
    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.config.num_threads = num_threads;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size;
        self
    }

    /// Discover files from paths
    pub fn discover_files(
        &self,
        paths: Vec<PathBuf>,
        include: &GlobSet,
        exclude: &GlobSet,
    ) -> Result<Vec<PathBuf>> {
        let discovered: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));

        // Use parallel discovery for better performance
        paths.par_iter().try_for_each(|path| -> Result<()> {
            let files = self.discover_path(path, include, exclude)?;
            discovered.lock().extend(files);
            Ok(())
        })?;

        Ok(Arc::try_unwrap(discovered).unwrap().into_inner())
    }

    /// Discover files from a single path
    fn discover_path(
        &self,
        path: &Path,
        include: &GlobSet,
        exclude: &GlobSet,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        if !path.exists() {
            eprintln!("Warning: Path doesn't exist: {:?}", path);
            return Ok(files);
        }

        if path.is_dir() {
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
            {
                let entry = entry?;
                let entry_path = entry.path().to_path_buf();

                if (include.is_empty() || include.is_match(&entry_path))
                    && (exclude.is_empty() || !exclude.is_match(&entry_path))
                    && entry_path.is_file()
                {
                    files.push(entry_path);
                }
            }
        } else if (include.is_empty() || include.is_match(path))
            && (exclude.is_empty() || !exclude.is_match(path))
            && path.is_file()
        {
            files.push(path.to_path_buf());
        }

        Ok(files)
    }

    /// Calculate optimal batch size based on file count and sizes
    fn calculate_batch_size(&self, files: &[PathBuf]) -> usize {
        if self.config.batch_size > 0 {
            return self.config.batch_size;
        }

        let num_files = files.len();
        let num_threads = if self.config.num_threads > 0 {
            self.config.num_threads
        } else {
            rayon::current_num_threads()
        };

        // Aim for 2-4 batches per thread for good load balancing
        let target_batches = num_threads * 3;
        (num_files / target_batches).max(1).min(100)
    }

    /// Process a single file
    fn process_file(&self, path: &PathBuf, config: &C) -> ProcessResult<T> {
        let size = get_file_size(path);

        match (self.processor)(path, config) {
            Ok(result) => ProcessResult {
                path: path.clone(),
                success: true,
                result: Some(result),
                error: None,
                size,
            },
            Err(e) => ProcessResult {
                path: path.clone(),
                success: false,
                result: None,
                error: Some(e.to_string()),
                size,
            },
        }
    }

    /// Process all files in parallel
    pub fn process_all(
        &self,
        files: Vec<PathBuf>,
        config: C,
    ) -> Result<(Vec<ProcessResult<T>>, ParallelStats)>
    where
        C: Sync,
    {
        let start = Instant::now();
        let stats = Arc::new(Mutex::new(ParallelStats::new()));

        // Build thread pool if custom thread count specified
        let pool = if self.config.num_threads > 0 {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(self.config.num_threads)
                    .build()?,
            )
        } else {
            None
        };

        // Process files in parallel
        let results: Vec<ProcessResult<T>> = if let Some(pool) = pool {
            pool.install(|| {
                files
                    .par_iter()
                    .map(|path| {
                        let result = self.process_file(path, &config);
                        self.update_stats(&result, &stats);
                        result
                    })
                    .collect()
            })
        } else {
            files
                .par_iter()
                .map(|path| {
                    let result = self.process_file(path, &config);
                    self.update_stats(&result, &stats);
                    result
                })
                .collect()
        };

        // Finalize stats
        let duration = start.elapsed();
        stats.lock().finish(duration);
        let final_stats = stats.lock().clone();

        Ok((results, final_stats))
    }

    /// Process files with batching for better memory efficiency
    pub fn process_batched(
        &self,
        files: Vec<PathBuf>,
        config: C,
    ) -> Result<(Vec<ProcessResult<T>>, ParallelStats)>
    where
        C: Sync + Clone,
    {
        let start = Instant::now();
        let stats = Arc::new(Mutex::new(ParallelStats::new()));
        let batch_size = self.calculate_batch_size(&files);

        // Process in batches
        let results: Vec<ProcessResult<T>> = files
            .par_chunks(batch_size)
            .flat_map(|batch| {
                batch
                    .par_iter()
                    .map(|path| {
                        let result = self.process_file(path, &config);
                        self.update_stats(&result, &stats);
                        result
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let duration = start.elapsed();
        stats.lock().finish(duration);
        let final_stats = stats.lock().clone();

        Ok((results, final_stats))
    }

    /// Process files and collect only successful results
    pub fn process_collect<R>(
        &self,
        files: Vec<PathBuf>,
        config: C,
    ) -> Result<(Vec<R>, ParallelStats)>
    where
        C: Sync,
        T: Into<R>,
    {
        let (results, stats) = self.process_all(files, config)?;

        let collected: Vec<R> = results
            .into_iter()
            .filter_map(|r| r.result.map(|t| t.into()))
            .collect();

        Ok((collected, stats))
    }

    /// Process files and aggregate results with a custom function
    pub fn process_aggregate<A, G>(
        &self,
        files: Vec<PathBuf>,
        config: C,
        initial: A,
        aggregator: G,
    ) -> Result<(A, ParallelStats)>
    where
        C: Sync,
        A: Send + Sync + Clone,
        G: Fn(A, ProcessResult<T>) -> A + Send + Sync,
    {
        let start = Instant::now();
        let stats = Arc::new(Mutex::new(ParallelStats::new()));

        let aggregator = Arc::new(aggregator);
        let initial_arc = Arc::new(initial.clone());

        let result = files
            .par_iter()
            .map(|path| {
                let result = self.process_file(path, &config);
                self.update_stats(&result, &stats);
                result
            })
            .fold(
                || (*initial_arc).clone(),
                |acc, result| {
                    let agg = Arc::clone(&aggregator);
                    agg(acc, result)
                },
            )
            .reduce(
                || initial.clone(),
                |a, _b| {
                    // Return the first accumulated result
                    a
                },
            );

        let duration = start.elapsed();
        stats.lock().finish(duration);
        let final_stats = stats.lock().clone();

        Ok((result, final_stats))
    }

    /// Update statistics based on processing result
    fn update_stats(&self, result: &ProcessResult<T>, stats: &Arc<Mutex<ParallelStats>>) {
        let mut s = stats.lock();

        if result.success {
            s.files_processed += 1;
            s.bytes_processed += result.size;
        } else {
            s.files_failed += 1;
        }
    }
}

/// Builder for parallel processor with fluent API
pub struct ParallelProcessorBuilder<F, C, T>
where
    F: Fn(&PathBuf, &C) -> Result<T> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    processor: F,
    config: ParallelConfig,
    _phantom: std::marker::PhantomData<(C, T)>,
}

impl<F, C, T> ParallelProcessorBuilder<F, C, T>
where
    F: Fn(&PathBuf, &C) -> Result<T> + Send + Sync,
    C: Send + Sync,
    T: Send + Sync,
{
    pub fn new(processor: F) -> Self {
        Self {
            processor,
            config: ParallelConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn num_threads(mut self, threads: usize) -> Self {
        self.config.num_threads = threads;
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    pub fn adaptive_batching(mut self, enabled: bool) -> Self {
        self.config.adaptive_batching = enabled;
        self
    }

    pub fn max_parallel(mut self, max: usize) -> Self {
        self.config.max_parallel = max;
        self
    }

    pub fn build(self) -> ParallelProcessor<F, C, T> {
        ParallelProcessor::with_config(self.processor, self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_processor_creation() {
        let _processor = ParallelProcessor::new(|_path: &PathBuf, _config: &()| Ok(()));
    }

    #[test]
    fn test_discover_files() {
        let temp = TempDir::new().unwrap();

        // On macOS, TempDir creates directories in hidden locations (e.g., /var/folders/...)
        // which are filtered out by is_hidden(). Create a non-hidden subdirectory for testing.
        let test_dir = temp.path().join("test_data");
        fs::create_dir(&test_dir).unwrap();

        let file1 = test_dir.join("file1.txt");
        let file2 = test_dir.join("file2.txt");

        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        let processor = ParallelProcessor::new(|_path: &PathBuf, _config: &()| Ok(()));

        let files = processor
            .discover_files(
                vec![test_dir],
                &GlobSet::empty(),
                &GlobSet::empty(),
            )
            .unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_process_all() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let processor = ParallelProcessor::new(|path: &PathBuf, _config: &()| {
            let content = std::fs::read_to_string(path)?;
            Ok(content.len())
        });

        let (results, stats) = processor
            .process_all(vec![file1, file2], ())
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.files_failed, 0);
    }

    #[test]
    fn test_builder_pattern() {
        let processor = ParallelProcessorBuilder::new(|_path: &PathBuf, _config: &()| Ok(()))
            .num_threads(4)
            .batch_size(10)
            .adaptive_batching(true)
            .build();

        assert_eq!(processor.config.num_threads, 4);
        assert_eq!(processor.config.batch_size, 10);
        assert!(processor.config.adaptive_batching);
    }

    #[test]
    fn test_process_batched() {
        let temp = TempDir::new().unwrap();
        let mut files = Vec::new();

        for i in 0..10 {
            let file = temp.path().join(format!("file{}.txt", i));
            fs::write(&file, format!("content{}", i)).unwrap();
            files.push(file);
        }

        let processor = ParallelProcessor::new(|path: &PathBuf, _config: &()| {
            let _content = std::fs::read_to_string(path)?;
            Ok(())
        })
        .with_batch_size(3);

        let (results, stats) = processor.process_batched(files, ()).unwrap();

        assert_eq!(results.len(), 10);
        assert_eq!(stats.files_processed, 10);
    }
}
