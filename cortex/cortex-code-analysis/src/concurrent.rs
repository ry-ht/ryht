//! Concurrent file processing system for code analysis.
//!
//! This module provides a producer-consumer architecture for processing multiple
//! source code files concurrently. It supports:
//! - Multi-threaded file processing with configurable worker count
//! - File filtering via glob patterns (include/exclude)
//! - Hidden file exclusion
//! - Custom per-file processing callbacks
//! - Error handling and reporting
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
//! use cortex_code_analysis::Lang;
//! use globset::GlobSet;
//! use std::path::PathBuf;
//! use std::sync::{Arc, Mutex};
//!
//! // Configuration for processing files
//! struct Config {
//!     language: Lang,
//! }
//!
//! // Collect results
//! let results = Arc::new(Mutex::new(Vec::new()));
//! let results_clone = results.clone();
//!
//! // Create runner with 4 worker threads
//! let runner = ConcurrentRunner::new(4, move |path, config: &Config| {
//!     // Process each file
//!     let source = std::fs::read_to_string(&path)?;
//!     results_clone.lock().unwrap().push(path.clone());
//!     Ok(())
//! });
//!
//! // Configure files to process
//! let files_data = FilesData {
//!     paths: vec![PathBuf::from("src")],
//!     include: GlobSet::empty(),
//!     exclude: GlobSet::empty(),
//! };
//!
//! let config = Config { language: Lang::Rust };
//! runner.run(config, files_data)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result};
use crossbeam::channel::{Receiver, Sender, unbounded};
use globset::GlobSet;
use walkdir::{DirEntry, WalkDir};

/// Function type for processing individual files.
///
/// Takes a file path and configuration, returns a Result.
type ProcFilesFunction<Config> = dyn Fn(PathBuf, &Config) -> Result<()> + Send + Sync;

/// Function type for processing directory paths.
///
/// Called for each file found in a directory, allows collecting metadata.
type ProcDirPathsFunction<Config> =
    dyn Fn(&mut HashMap<String, Vec<PathBuf>>, &Path, &Config) + Send + Sync;

/// Function type for processing individual paths.
///
/// Called for each path before sending to workers.
type ProcPathFunction<Config> = dyn Fn(&Path, &Config) + Send + Sync;

/// Null directory paths processor (no-op, optimized away at compile time)
fn null_proc_dir_paths<Config>(_: &mut HashMap<String, Vec<PathBuf>>, _: &Path, _: &Config) {}

/// Null path processor (no-op, optimized away at compile time)
fn null_proc_path<Config>(_: &Path, _: &Config) {}

/// A job item representing a file to process.
#[derive(Debug)]
struct JobItem<Config> {
    path: PathBuf,
    cfg: Arc<Config>,
}

type JobReceiver<Config> = Receiver<Option<JobItem<Config>>>;
type JobSender<Config> = Sender<Option<JobItem<Config>>>;

/// Consumer worker that processes jobs from the queue.
///
/// Runs in a separate thread, pulling jobs from the receiver channel
/// and processing them with the provided function.
fn consumer<Config, ProcFiles>(receiver: JobReceiver<Config>, func: Arc<ProcFiles>)
where
    ProcFiles: Fn(PathBuf, &Config) -> Result<()> + Send + Sync,
{
    while let Ok(job) = receiver.recv() {
        // None is the poison pill to stop processing
        if job.is_none() {
            break;
        }

        // Safe to unwrap because of the check above
        let job = job.unwrap();
        let path = job.path.clone();

        if let Err(err) = func(job.path, &job.cfg) {
            eprintln!("Error processing file {path:?}: {err:?}");
        }
    }
}

/// Send a file job to the processing queue.
fn send_file<T>(
    path: PathBuf,
    cfg: &Arc<T>,
    sender: &JobSender<T>,
) -> Result<()> {
    sender
        .send(Some(JobItem {
            path,
            cfg: Arc::clone(cfg),
        }))
        .map_err(|e| anyhow::anyhow!("Failed to send job to queue: {:?}", e))
}

/// Check if a directory entry is hidden (starts with dot).
fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Explore directories and files, sending them to the processing queue.
///
/// This is the producer side of the producer-consumer pattern.
fn explore<Config, ProcDirPaths, ProcPath>(
    files_data: FilesData,
    cfg: &Arc<Config>,
    proc_dir_paths: ProcDirPaths,
    proc_path: ProcPath,
    sender: &JobSender<Config>,
) -> Result<HashMap<String, Vec<PathBuf>>>
where
    ProcDirPaths: Fn(&mut HashMap<String, Vec<PathBuf>>, &Path, &Config) + Send + Sync,
    ProcPath: Fn(&Path, &Config) + Send + Sync,
{
    let FilesData {
        mut paths,
        ref include,
        ref exclude,
    } = files_data;

    let mut all_files: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for path in paths.drain(..) {
        if !path.exists() {
            eprintln!("Warning: Path doesn't exist: {path:?}");
            continue;
        }

        if path.is_dir() {
            // Walk directory tree
            for entry in WalkDir::new(path)
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
            {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path().to_path_buf();

                // Apply include/exclude filters
                if (include.is_empty() || include.is_match(&path))
                    && (exclude.is_empty() || !exclude.is_match(&path))
                    && path.is_file()
                {
                    proc_dir_paths(&mut all_files, &path, cfg);
                    send_file(path, cfg, sender)?;
                }
            }
        } else if (include.is_empty() || include.is_match(&path))
            && (exclude.is_empty() || !exclude.is_match(&path))
            && path.is_file()
        {
            // Single file
            proc_path(&path, cfg);
            send_file(path, cfg, sender)?;
        }
    }

    Ok(all_files)
}

/// Data describing which files to process.
#[derive(Debug)]
pub struct FilesData {
    /// Glob patterns for files to include in processing.
    pub include: GlobSet,

    /// Glob patterns for files to exclude from processing.
    pub exclude: GlobSet,

    /// List of file or directory paths to process.
    pub paths: Vec<PathBuf>,
}

/// A concurrent file processor using a producer-consumer pattern.
///
/// This runner spawns multiple worker threads that process files in parallel.
/// A producer thread walks the file system and sends files to workers via a channel.
///
/// # Type Parameters
///
/// * `Config` - Configuration type passed to processing functions. Must be `Send + Sync`.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
/// use globset::GlobSet;
/// use std::path::PathBuf;
///
/// struct MyConfig {
///     verbose: bool,
/// }
///
/// let runner = ConcurrentRunner::new(4, |path, config: &MyConfig| {
///     if config.verbose {
///         println!("Processing: {:?}", path);
///     }
///     Ok(())
/// });
///
/// let files = FilesData {
///     paths: vec![PathBuf::from("src")],
///     include: GlobSet::empty(),
///     exclude: GlobSet::empty(),
/// };
///
/// let config = MyConfig { verbose: true };
/// runner.run(config, files)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct ConcurrentRunner<Config> {
    proc_files: Box<ProcFilesFunction<Config>>,
    proc_dir_paths: Box<ProcDirPathsFunction<Config>>,
    proc_path: Box<ProcPathFunction<Config>>,
    num_jobs: usize,
}

impl<Config: 'static + Send + Sync> ConcurrentRunner<Config> {
    /// Creates a new concurrent file processor.
    ///
    /// # Parameters
    ///
    /// * `num_jobs` - Number of worker threads. Minimum is 2 (1 producer + 1 consumer).
    /// * `proc_files` - Function called for each file. Should return `Ok(())` on success.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::concurrent::ConcurrentRunner;
    ///
    /// let runner = ConcurrentRunner::new(4, |path, _config: &()| {
    ///     println!("Processing: {:?}", path);
    ///     Ok(())
    /// });
    /// ```
    pub fn new<ProcFiles>(num_jobs: usize, proc_files: ProcFiles) -> Self
    where
        ProcFiles: 'static + Fn(PathBuf, &Config) -> Result<()> + Send + Sync,
    {
        // Reserve one job for the producer thread
        let num_jobs = std::cmp::max(2, num_jobs) - 1;

        Self {
            proc_files: Box::new(proc_files),
            proc_dir_paths: Box::new(null_proc_dir_paths),
            proc_path: Box::new(null_proc_path),
            num_jobs,
        }
    }

    /// Sets a function to process directory paths.
    ///
    /// This function is called for each file found within a directory,
    /// allowing you to collect metadata or build file lists.
    ///
    /// # Parameters
    ///
    /// * `proc_dir_paths` - Function that receives a mutable HashMap, file path, and config
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::concurrent::ConcurrentRunner;
    /// use std::collections::HashMap;
    /// use std::path::{Path, PathBuf};
    ///
    /// let runner = ConcurrentRunner::new(4, |path, _: &()| Ok(()))
    ///     .set_proc_dir_paths(|files: &mut HashMap<String, Vec<PathBuf>>, path: &Path, _: &()| {
    ///         if let Some(ext) = path.extension() {
    ///             files.entry(ext.to_string_lossy().to_string())
    ///                 .or_insert_with(Vec::new)
    ///                 .push(path.to_path_buf());
    ///         }
    ///     });
    /// ```
    pub fn set_proc_dir_paths<ProcDirPaths>(mut self, proc_dir_paths: ProcDirPaths) -> Self
    where
        ProcDirPaths:
            'static + Fn(&mut HashMap<String, Vec<PathBuf>>, &Path, &Config) + Send + Sync,
    {
        self.proc_dir_paths = Box::new(proc_dir_paths);
        self
    }

    /// Sets a function to process individual paths.
    ///
    /// This function is called for each path before it's sent to workers.
    /// Useful for logging or validation.
    ///
    /// # Parameters
    ///
    /// * `proc_path` - Function that receives a path and config
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::concurrent::ConcurrentRunner;
    /// use std::path::Path;
    ///
    /// let runner = ConcurrentRunner::new(4, |path, _: &()| Ok(()))
    ///     .set_proc_path(|path: &Path, _: &()| {
    ///         println!("Found file: {:?}", path);
    ///     });
    /// ```
    pub fn set_proc_path<ProcPath>(mut self, proc_path: ProcPath) -> Self
    where
        ProcPath: 'static + Fn(&Path, &Config) + Send + Sync,
    {
        self.proc_path = Box::new(proc_path);
        self
    }

    /// Runs the concurrent file processor.
    ///
    /// This method:
    /// 1. Spawns a producer thread to walk the file system
    /// 2. Spawns worker threads to process files
    /// 3. Waits for all processing to complete
    /// 4. Returns collected metadata from directory processing
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration passed to all processing functions
    /// * `files_data` - Describes which files to process
    ///
    /// # Returns
    ///
    /// Returns a HashMap of metadata collected by `proc_dir_paths`, or an error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Thread spawning fails
    /// - File system walking fails
    /// - Channel communication fails
    /// - A worker thread panics
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cortex_code_analysis::concurrent::{ConcurrentRunner, FilesData};
    /// use globset::GlobSet;
    /// use std::path::PathBuf;
    ///
    /// let runner = ConcurrentRunner::new(4, |path, _: &()| {
    ///     let _content = std::fs::read_to_string(&path)?;
    ///     Ok(())
    /// });
    ///
    /// let files = FilesData {
    ///     paths: vec![PathBuf::from("src")],
    ///     include: GlobSet::empty(),
    ///     exclude: GlobSet::empty(),
    /// };
    ///
    /// let results = runner.run((), files)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn run(
        self,
        config: Config,
        files_data: FilesData,
    ) -> Result<HashMap<String, Vec<PathBuf>>> {
        let cfg = Arc::new(config);

        // Create unbounded channel for job queue
        let (sender, receiver) = unbounded();

        // Spawn producer thread
        let producer = {
            let sender = sender.clone();

            thread::Builder::new()
                .name(String::from("FileProducer"))
                .spawn(move || {
                    explore(
                        files_data,
                        &cfg,
                        self.proc_dir_paths,
                        self.proc_path,
                        &sender,
                    )
                })
                .context("Failed to spawn producer thread")?
        };

        // Spawn consumer worker threads
        let mut receivers = Vec::with_capacity(self.num_jobs);
        let proc_files = Arc::new(self.proc_files);

        for i in 0..self.num_jobs {
            let receiver = receiver.clone();
            let proc_files = proc_files.clone();

            let worker = thread::Builder::new()
                .name(format!("FileWorker-{i}"))
                .spawn(move || {
                    consumer(receiver, proc_files);
                })
                .context("Failed to spawn worker thread")?;

            receivers.push(worker);
        }

        // Wait for producer to finish
        let all_files = match producer.join() {
            Ok(res) => res?,
            Err(_) => {
                anyhow::bail!("Producer thread panicked");
            }
        };

        // Send poison pills to stop workers
        for _ in 0..self.num_jobs {
            sender
                .send(None)
                .context("Failed to send termination signal to workers")?;
        }

        // Wait for all workers to finish
        for worker in receivers {
            if worker.join().is_err() {
                anyhow::bail!("Worker thread panicked");
            }
        }

        Ok(all_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_concurrent_runner_creation() {
        let _runner = ConcurrentRunner::new(4, |_path, _config: &()| Ok(()));
    }

    #[test]
    fn test_is_hidden() {
        let temp = TempDir::new().unwrap();
        let hidden_file = temp.path().join(".hidden");
        let normal_file = temp.path().join("normal");

        fs::write(&hidden_file, "test").unwrap();
        fs::write(&normal_file, "test").unwrap();

        for entry in WalkDir::new(temp.path()) {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy();

            if name == ".hidden" {
                assert!(is_hidden(&entry));
            } else if name == "normal" {
                assert!(!is_hidden(&entry));
            }
        }
    }

    #[test]
    fn test_process_files() {
        let temp = TempDir::new().unwrap();
        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        let processed = Arc::new(Mutex::new(Vec::new()));
        let processed_clone = processed.clone();

        let runner = ConcurrentRunner::new(2, move |path, _: &()| {
            processed_clone.lock().unwrap().push(path);
            Ok(())
        });

        let files = FilesData {
            paths: vec![file1, file2],  // Use file paths directly
            include: GlobSet::empty(),
            exclude: GlobSet::empty(),
        };

        runner.run((), files).unwrap();

        let processed = processed.lock().unwrap();
        assert_eq!(processed.len(), 2);
    }

    #[test]
    fn test_proc_dir_paths() {
        let temp = TempDir::new().unwrap();
        // Create a subdirectory to test directory traversal
        let src_dir = temp.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let file1 = src_dir.join("file1.rs");
        let file2 = src_dir.join("file2.rs");

        fs::write(&file1, "fn main() {}").unwrap();
        fs::write(&file2, "fn test() {}").unwrap();

        let runner = ConcurrentRunner::new(2, |_path, _: &()| Ok(()))
            .set_proc_dir_paths(|files, path, _: &()| {
                if let Some(ext) = path.extension() {
                    files
                        .entry(ext.to_string_lossy().to_string())
                        .or_insert_with(Vec::new)
                        .push(path.to_path_buf());
                }
            });

        let files = FilesData {
            paths: vec![src_dir],  // Use directory path to trigger directory traversal
            include: GlobSet::empty(),
            exclude: GlobSet::empty(),
        };

        let results = runner.run((), files).unwrap();
        assert_eq!(results.get("rs").map(|v| v.len()), Some(2));
    }
}
