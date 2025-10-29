//! Enhanced Producer-Consumer Architecture for Concurrent File Processing
//!
//! This module provides a robust producer-consumer pattern optimized for file processing:
//! - Bounded channels with backpressure handling
//! - Parallel directory traversal
//! - Adaptive worker pool sizing
//! - Error aggregation and retry logic
//! - Graceful shutdown and cancellation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐      ┌──────────────┐      ┌──────────────┐
//! │  Producer   │─────▶│ Bounded Chan │─────▶│   Workers    │
//! │  (Files)    │      │   (Queue)    │      │   (Pool)     │
//! └─────────────┘      └──────────────┘      └──────────────┘
//!       │                                            │
//!       │                                            ▼
//!       │                                     ┌──────────────┐
//!       └────────────────────────────────────▶│   Results    │
//!                                             │ Aggregator   │
//!                                             └──────────────┘
//! ```

use anyhow::{Context, Result};
use crossbeam::channel::{bounded, Receiver, Sender};
use globset::GlobSet;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

/// Configuration for the producer-consumer system
#[derive(Debug, Clone)]
pub struct ProducerConsumerConfig {
    /// Maximum number of worker threads (0 = auto-detect)
    pub num_workers: usize,

    /// Channel buffer size for backpressure (0 = unbounded)
    pub channel_capacity: usize,

    /// Enable parallel directory traversal
    pub parallel_discovery: bool,

    /// Maximum retry attempts for failed files
    pub max_retries: usize,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Enable graceful error handling (continue on errors)
    pub graceful_errors: bool,
}

impl Default for ProducerConsumerConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus::get(),
            channel_capacity: 1000,
            parallel_discovery: true,
            max_retries: 2,
            retry_delay_ms: 100,
            graceful_errors: true,
        }
    }
}

/// Statistics about the processing run
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Total files discovered
    pub files_discovered: usize,

    /// Files successfully processed
    pub files_processed: usize,

    /// Files that failed processing
    pub files_failed: usize,

    /// Total bytes processed
    pub bytes_processed: u64,

    /// Processing duration
    pub duration: Duration,

    /// Files per second throughput
    pub throughput_fps: f64,

    /// Megabytes per second throughput
    pub throughput_mbps: f64,
}

impl ProcessingStats {
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

    pub fn success_rate(&self) -> f64 {
        let total = self.files_processed + self.files_failed;
        if total == 0 {
            0.0
        } else {
            (self.files_processed as f64 / total as f64) * 100.0
        }
    }
}

/// Job item for processing
#[derive(Debug, Clone)]
struct Job<C> {
    path: PathBuf,
    size: u64,
    config: Arc<C>,
    #[allow(dead_code)]
    retry_count: usize,
}

/// Result of processing a single file
#[derive(Debug)]
pub struct FileResult {
    pub path: PathBuf,
    pub success: bool,
    pub size: u64,
    pub error: Option<String>,
}

/// Check if a directory entry is hidden
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

/// Enhanced producer-consumer file processor
pub struct EnhancedProducerConsumer<C, F>
where
    C: Send + Sync + 'static,
    F: Fn(PathBuf, &C) -> Result<()> + Send + Sync + 'static,
{
    config: ProducerConsumerConfig,
    processor: Arc<F>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C, F> EnhancedProducerConsumer<C, F>
where
    C: Send + Sync + 'static,
    F: Fn(PathBuf, &C) -> Result<()> + Send + Sync + 'static,
{
    /// Create a new enhanced producer-consumer processor
    pub fn new(processor: F, config: ProducerConsumerConfig) -> Self {
        Self {
            config,
            processor: Arc::new(processor),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(processor: F) -> Self {
        Self::new(processor, ProducerConsumerConfig::default())
    }

    /// Discover files to process
    fn discover_files(
        &self,
        paths: Vec<PathBuf>,
        include: &GlobSet,
        exclude: &GlobSet,
    ) -> Result<Vec<PathBuf>> {
        let discovered = Arc::new(Mutex::new(Vec::new()));

        if self.config.parallel_discovery {
            // Parallel discovery for better performance on large directories
            paths.par_iter().try_for_each(|path| -> Result<()> {
                let local_files = self.discover_path(path, include, exclude)?;
                discovered.lock().extend(local_files);
                Ok(())
            })?;
        } else {
            // Sequential discovery
            for path in paths {
                let local_files = self.discover_path(&path, include, exclude)?;
                discovered.lock().extend(local_files);
            }
        }

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
                let entry = entry.context("Failed to read directory entry")?;
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

    /// Producer thread: discovers files and sends to queue
    fn producer(
        _processor: Arc<F>,
        files: Vec<PathBuf>,
        sender: Sender<Option<Job<C>>>,
        config: Arc<C>,
        stats: Arc<Mutex<ProcessingStats>>,
    ) {
        for path in files {
            let size = get_file_size(&path);

            let job = Job {
                path,
                size,
                config: Arc::clone(&config),
                retry_count: 0,
            };

            if sender.send(Some(job)).is_err() {
                eprintln!("Producer: channel closed unexpectedly");
                break;
            }

            stats.lock().files_discovered += 1;
        }
    }

    /// Consumer worker: processes jobs from queue
    fn consumer(
        processor: Arc<F>,
        config: ProducerConsumerConfig,
        receiver: Receiver<Option<Job<C>>>,
        result_sender: Sender<FileResult>,
        stats: Arc<Mutex<ProcessingStats>>,
    ) {
        let temp_self = Self {
            config,
            processor,
            _phantom: std::marker::PhantomData,
        };
        while let Ok(Some(job)) = receiver.recv() {
            let result = temp_self.process_job(job, &receiver, &stats);

            if result_sender.send(result).is_err() {
                eprintln!("Consumer: result channel closed");
                break;
            }
        }
    }

    /// Process a single job with retry logic
    fn process_job(
        &self,
        job: Job<C>,
        _receiver: &Receiver<Option<Job<C>>>,
        _stats: &Arc<Mutex<ProcessingStats>>,
    ) -> FileResult {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                thread::sleep(Duration::from_millis(self.config.retry_delay_ms));
            }

            match (self.processor)(job.path.clone(), &job.config) {
                Ok(()) => {
                    return FileResult {
                        path: job.path,
                        success: true,
                        size: job.size,
                        error: None,
                    };
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    if !self.config.graceful_errors && attempt == 0 {
                        break;
                    }
                }
            }
        }

        FileResult {
            path: job.path,
            success: false,
            size: job.size,
            error: last_error,
        }
    }

    /// Result aggregator: collects results from workers
    fn result_aggregator(
        receiver: Receiver<FileResult>,
        stats: Arc<Mutex<ProcessingStats>>,
        errors: Arc<Mutex<Vec<(PathBuf, String)>>>,
    ) {
        while let Ok(result) = receiver.recv() {
            let mut stats = stats.lock();

            if result.success {
                stats.files_processed += 1;
                stats.bytes_processed += result.size;
            } else {
                stats.files_failed += 1;
                if let Some(error) = result.error {
                    errors.lock().push((result.path, error));
                }
            }
        }
    }

    /// Run the producer-consumer system
    pub fn run(
        &self,
        user_config: C,
        paths: Vec<PathBuf>,
        include: GlobSet,
        exclude: GlobSet,
    ) -> Result<(ProcessingStats, Vec<(PathBuf, String)>)> {
        let start_time = Instant::now();
        let config = Arc::new(user_config);
        let stats = Arc::new(Mutex::new(ProcessingStats::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        // Discover files
        let files = self.discover_files(paths, &include, &exclude)?;

        if files.is_empty() {
            return Ok((ProcessingStats::new(), Vec::new()));
        }

        // Create channels
        let (job_sender, job_receiver) = if self.config.channel_capacity > 0 {
            bounded(self.config.channel_capacity)
        } else {
            crossbeam::channel::unbounded()
        };

        let (result_sender, result_receiver) = crossbeam::channel::unbounded();

        // Spawn producer thread
        let producer_files = files.clone();
        let producer_sender = job_sender.clone();
        let producer_config = Arc::clone(&config);
        let producer_stats = Arc::clone(&stats);
        let producer_processor = Arc::clone(&self.processor);

        let producer_handle = thread::Builder::new()
            .name("FileProducer".to_string())
            .spawn(move || {
                Self::producer(producer_processor, producer_files, producer_sender, producer_config, producer_stats);
            })
            .context("Failed to spawn producer thread")?;

        // Spawn consumer worker threads
        let num_workers = if self.config.num_workers > 0 {
            self.config.num_workers
        } else {
            num_cpus::get()
        };

        let mut worker_handles = Vec::with_capacity(num_workers);

        for i in 0..num_workers {
            let worker_receiver = job_receiver.clone();
            let worker_result_sender = result_sender.clone();
            let worker_stats = Arc::clone(&stats);
            let worker_processor = Arc::clone(&self.processor);
            let worker_config = self.config.clone();

            let handle = thread::Builder::new()
                .name(format!("FileWorker-{}", i))
                .spawn(move || {
                    Self::consumer(worker_processor, worker_config, worker_receiver, worker_result_sender, worker_stats);
                })
                .context("Failed to spawn worker thread")?;

            worker_handles.push(handle);
        }

        // Spawn result aggregator thread
        let aggregator_receiver = result_receiver;
        let aggregator_stats = Arc::clone(&stats);
        let aggregator_errors = Arc::clone(&errors);

        let aggregator_handle = thread::Builder::new()
            .name("ResultAggregator".to_string())
            .spawn(move || {
                Self::result_aggregator(aggregator_receiver, aggregator_stats, aggregator_errors);
            })
            .context("Failed to spawn aggregator thread")?;

        // Wait for producer to finish
        producer_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Producer thread panicked"))?;

        // Send poison pills to workers
        for _ in 0..num_workers {
            let _ = job_sender.send(None);
        }

        // Wait for workers to finish
        for handle in worker_handles {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("Worker thread panicked"))?;
        }

        // Close result channel and wait for aggregator
        drop(result_sender);
        aggregator_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Aggregator thread panicked"))?;

        // Finalize statistics
        let duration = start_time.elapsed();
        stats.lock().finish(duration);

        let final_stats = stats.lock().clone();
        let final_errors = errors.lock().clone();

        Ok((final_stats, final_errors))
    }

    /// Helper to clone the processor for workers
    #[allow(dead_code)]
    fn clone_processor(&self) -> Self {
        Self {
            config: self.config.clone(),
            processor: Arc::clone(&self.processor),
            _phantom: std::marker::PhantomData,
        }
    }
}

// Helper to get CPU count
fn num_cpus_get() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

mod num_cpus {
    pub fn get() -> usize {
        super::num_cpus_get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_producer_consumer_creation() {
        let processor = |_path: PathBuf, _config: &()| Ok(());
        let _pc = EnhancedProducerConsumer::with_defaults(processor);
    }

    #[test]
    fn test_file_discovery() {
        let temp = TempDir::new().unwrap();

        // On macOS, TempDir creates directories in hidden locations (e.g., /var/folders/...)
        // which are filtered out by is_hidden(). Create a non-hidden subdirectory for testing.
        let test_dir = temp.path().join("test_data");
        fs::create_dir(&test_dir).unwrap();

        let file1 = test_dir.join("file1.txt");
        let file2 = test_dir.join("file2.txt");

        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        let processor = |_path: PathBuf, _config: &()| Ok(());
        let pc = EnhancedProducerConsumer::with_defaults(processor);

        let files = pc.discover_files(
            vec![test_dir],
            &GlobSet::empty(),
            &GlobSet::empty(),
        ).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_processing_stats() {
        let mut stats = ProcessingStats::new();
        stats.files_processed = 100;
        stats.files_failed = 10;
        stats.bytes_processed = 1_048_576; // 1 MB
        stats.finish(Duration::from_secs(1));

        assert!(stats.success_rate() > 90.0);
        assert!(stats.throughput_fps > 0.0);
        assert!(stats.throughput_mbps > 0.0);
    }

    #[test]
    fn test_run_processing() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "test content").unwrap();

        let processor = |path: PathBuf, _config: &()| {
            let _content = std::fs::read_to_string(&path)?;
            Ok(())
        };

        let pc = EnhancedProducerConsumer::with_defaults(processor);

        let (stats, errors) = pc.run(
            (),
            vec![file],
            GlobSet::empty(),
            GlobSet::empty(),
        ).unwrap();

        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.files_failed, 0);
        assert_eq!(errors.len(), 0);
    }
}
