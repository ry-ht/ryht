//! Async Concurrent File Processing
//!
//! This module provides async/await based concurrent file processing with:
//! - Tokio-based async runtime support
//! - Stream-based file processing
//! - Configurable concurrency limits
//! - Progress tracking and cancellation
//! - Error aggregation and handling
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::async_runner::{AsyncRunner, AsyncFilesData};
//! use cortex_code_analysis::Lang;
//! use globset::GlobSet;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     struct Config {
//!         language: Lang,
//!     }
//!
//!     let runner = AsyncRunner::new(4, |path, config: Config| async move {
//!         let source = tokio::fs::read_to_string(&path).await?;
//!         Ok(())
//!     });
//!
//!     let files_data = AsyncFilesData {
//!         paths: vec![PathBuf::from("src")],
//!         include: GlobSet::empty(),
//!         exclude: GlobSet::empty(),
//!     };
//!
//!     let config = Config { language: Lang::Rust };
//!     runner.run(config, files_data).await?;
//!     Ok(())
//! }
//! ```

use anyhow::{Context, Result};
use globset::GlobSet;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use walkdir::{DirEntry, WalkDir};

/// Type for async file processing functions
pub type AsyncProcFn<Config, Fut> = Arc<dyn Fn(PathBuf, Config) -> Fut + Send + Sync>;

/// Check if a directory entry is hidden
fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Data describing which files to process
#[derive(Debug)]
pub struct AsyncFilesData {
    /// Glob patterns for files to include
    pub include: GlobSet,

    /// Glob patterns for files to exclude
    pub exclude: GlobSet,

    /// List of file or directory paths to process
    pub paths: Vec<PathBuf>,
}

/// Progress information for async processing
#[derive(Debug, Clone)]
pub struct AsyncProgress {
    /// Total number of files discovered
    pub total_files: usize,

    /// Number of files processed so far
    pub processed: usize,

    /// Number of files that failed processing
    pub failed: usize,
}

impl AsyncProgress {
    pub fn new() -> Self {
        Self {
            total_files: 0,
            processed: 0,
            failed: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.processed + self.failed >= self.total_files
    }

    pub fn success_rate(&self) -> f64 {
        if self.processed + self.failed == 0 {
            0.0
        } else {
            (self.processed as f64 / (self.processed + self.failed) as f64) * 100.0
        }
    }
}

impl Default for AsyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Async concurrent file processor
///
/// Provides async/await based file processing with configurable concurrency.
pub struct AsyncRunner<Config, Fut>
where
    Config: Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    proc_files: AsyncProcFn<Config, Fut>,
    max_concurrent: usize,
}

impl<Config, Fut> AsyncRunner<Config, Fut>
where
    Config: Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    /// Create a new async runner
    ///
    /// # Parameters
    ///
    /// * `max_concurrent` - Maximum number of concurrent operations
    /// * `proc_files` - Async function to process each file
    pub fn new<F>(max_concurrent: usize, proc_files: F) -> Self
    where
        F: Fn(PathBuf, Config) -> Fut + Send + Sync + 'static,
    {
        Self {
            proc_files: Arc::new(proc_files),
            max_concurrent: max_concurrent.max(1),
        }
    }

    /// Discover files to process
    async fn discover_files(
        &self,
        files_data: AsyncFilesData,
    ) -> Result<Vec<PathBuf>> {
        let mut discovered = Vec::new();

        for path in files_data.paths {
            if !path.exists() {
                eprintln!("Warning: Path doesn't exist: {:?}", path);
                continue;
            }

            if path.is_dir() {
                // Walk directory tree
                for entry in WalkDir::new(&path)
                    .into_iter()
                    .filter_entry(|e| !is_hidden(e))
                {
                    let entry = entry.context("Failed to read directory entry")?;
                    let entry_path = entry.path().to_path_buf();

                    // Apply filters
                    if (files_data.include.is_empty() || files_data.include.is_match(&entry_path))
                        && (files_data.exclude.is_empty() || !files_data.exclude.is_match(&entry_path))
                        && entry_path.is_file()
                    {
                        discovered.push(entry_path);
                    }
                }
            } else if (files_data.include.is_empty() || files_data.include.is_match(&path))
                && (files_data.exclude.is_empty() || !files_data.exclude.is_match(&path))
                && path.is_file()
            {
                discovered.push(path);
            }
        }

        Ok(discovered)
    }

    /// Run the async file processor
    ///
    /// Processes files concurrently with the configured limit.
    pub async fn run(
        &self,
        config: Config,
        files_data: AsyncFilesData,
    ) -> Result<AsyncProgress> {
        // Discover files
        let files = self.discover_files(files_data).await?;
        let total_files = files.len();

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        // Create progress tracker
        let (progress_tx, mut progress_rx) = mpsc::channel(100);

        // Spawn progress aggregator task
        let progress_handle = tokio::spawn(async move {
            let mut progress = AsyncProgress::new();
            progress.total_files = total_files;

            while let Some(result) = progress_rx.recv().await {
                match result {
                    Ok(()) => progress.processed += 1,
                    Err(_) => progress.failed += 1,
                }
            }

            progress
        });

        // Process files concurrently
        let proc_files = Arc::clone(&self.proc_files);
        let tasks: Vec<_> = files
            .into_iter()
            .map(|path| {
                let config = config.clone();
                let proc_files = Arc::clone(&proc_files);
                let semaphore = Arc::clone(&semaphore);
                let progress_tx = progress_tx.clone();

                tokio::spawn(async move {
                    // Acquire semaphore permit
                    let _permit = semaphore.acquire().await.unwrap();

                    // Process file
                    let result = proc_files(path.clone(), config).await;

                    // Report progress
                    let _ = progress_tx.send(result).await;
                })
            })
            .collect();

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        // Close progress channel
        drop(progress_tx);

        // Get final progress
        let progress = progress_handle.await.context("Progress aggregator failed")?;

        Ok(progress)
    }

    /// Run with progress callback
    ///
    /// Calls the callback periodically with progress updates.
    pub async fn run_with_progress<F>(
        &self,
        config: Config,
        files_data: AsyncFilesData,
        mut progress_callback: F,
    ) -> Result<AsyncProgress>
    where
        F: FnMut(AsyncProgress) + Send + 'static,
    {
        // Discover files
        let files = self.discover_files(files_data).await?;
        let total_files = files.len();

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        // Create progress tracker
        let (progress_tx, mut progress_rx) = mpsc::channel(100);

        // Spawn progress aggregator task
        let progress_handle = tokio::spawn(async move {
            let mut progress = AsyncProgress::new();
            progress.total_files = total_files;

            while let Some(result) = progress_rx.recv().await {
                match result {
                    Ok(()) => progress.processed += 1,
                    Err(_) => progress.failed += 1,
                }

                // Call progress callback
                progress_callback(progress.clone());
            }

            progress
        });

        // Process files concurrently
        let proc_files = Arc::clone(&self.proc_files);
        let tasks: Vec<_> = files
            .into_iter()
            .map(|path| {
                let config = config.clone();
                let proc_files = Arc::clone(&proc_files);
                let semaphore = Arc::clone(&semaphore);
                let progress_tx = progress_tx.clone();

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = proc_files(path.clone(), config).await;
                    let _ = progress_tx.send(result).await;
                })
            })
            .collect();

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        drop(progress_tx);

        let progress = progress_handle.await.context("Progress aggregator failed")?;

        Ok(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_async_runner_creation() {
        let _runner = AsyncRunner::new(4, |_path, _config: ()| async move { Ok(()) });
    }

    #[tokio::test]
    async fn test_discover_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        let runner = AsyncRunner::new(2, |_path, _: ()| async move { Ok(()) });

        let files_data = AsyncFilesData {
            paths: vec![temp.path().to_path_buf()],
            include: GlobSet::empty(),
            exclude: GlobSet::empty(),
        };

        let files = runner.discover_files(files_data).await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_async_processing() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "test1").unwrap();
        fs::write(&file2, "test2").unwrap();

        let runner = AsyncRunner::new(2, |_path, _: ()| async move {
            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        });

        let files_data = AsyncFilesData {
            paths: vec![file1, file2],
            include: GlobSet::empty(),
            exclude: GlobSet::empty(),
        };

        let progress = runner.run((), files_data).await.unwrap();
        assert_eq!(progress.total_files, 2);
        assert_eq!(progress.processed, 2);
        assert_eq!(progress.failed, 0);
    }

    #[tokio::test]
    async fn test_progress_tracking() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("file.txt");
        fs::write(&file, "test").unwrap();

        let runner = AsyncRunner::new(1, |_path, _: ()| async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        });

        let files_data = AsyncFilesData {
            paths: vec![file],
            include: GlobSet::empty(),
            exclude: GlobSet::empty(),
        };

        let mut progress_updates = Vec::new();
        let progress = runner
            .run_with_progress((), files_data, |p| {
                progress_updates.push(p.clone());
            })
            .await
            .unwrap();

        assert!(progress_updates.len() > 0);
        assert_eq!(progress.processed, 1);
    }
}
