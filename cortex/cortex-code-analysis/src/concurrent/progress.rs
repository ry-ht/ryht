//! Progress Tracking for Concurrent Processing
//!
//! Provides comprehensive progress tracking with:
//! - Real-time progress bars (with indicatif feature)
//! - ETA calculation
//! - Throughput metrics (files/sec, MB/sec)
//! - Cancellation support
//! - Custom progress callbacks
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::concurrent::progress::{ProgressTracker, ProgressConfig};
//!
//! let tracker = ProgressTracker::new(100, ProgressConfig::default());
//! tracker.inc(1);
//! tracker.set_message("Processing files...");
//! tracker.finish();
//! ```

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for progress tracking
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    /// Enable visual progress bar (requires indicatif feature)
    pub enable_bar: bool,

    /// Update interval in milliseconds
    pub update_interval_ms: u64,

    /// Show throughput metrics
    pub show_throughput: bool,

    /// Show ETA
    pub show_eta: bool,

    /// Template for progress bar message
    pub template: Option<String>,

    /// Enable cancellation support
    pub enable_cancellation: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enable_bar: true,
            update_interval_ms: 100,
            show_throughput: true,
            show_eta: true,
            template: None,
            enable_cancellation: false,
        }
    }
}

/// Progress tracking state
#[derive(Debug)]
pub struct ProgressState {
    /// Total items to process
    pub total: usize,

    /// Items processed so far
    pub processed: usize,

    /// Items that failed
    pub failed: usize,

    /// Total bytes processed
    pub bytes_processed: u64,

    /// Start time
    pub start_time: Instant,

    /// Elapsed time
    pub elapsed: Duration,

    /// Files per second
    pub fps: f64,

    /// Megabytes per second
    pub mbps: f64,

    /// Estimated time remaining
    pub eta: Option<Duration>,

    /// Current message
    pub message: String,

    /// Is complete
    pub complete: bool,

    /// Was cancelled
    pub cancelled: bool,
}

impl ProgressState {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            processed: 0,
            failed: 0,
            bytes_processed: 0,
            start_time: Instant::now(),
            elapsed: Duration::ZERO,
            fps: 0.0,
            mbps: 0.0,
            eta: None,
            message: String::new(),
            complete: false,
            cancelled: false,
        }
    }

    pub fn update(&mut self) {
        self.elapsed = self.start_time.elapsed();
        let secs = self.elapsed.as_secs_f64();

        if secs > 0.0 {
            self.fps = self.processed as f64 / secs;
            self.mbps = (self.bytes_processed as f64 / 1_048_576.0) / secs;

            // Calculate ETA
            if self.processed > 0 && self.processed < self.total {
                let rate = self.processed as f64 / secs;
                let remaining = self.total - self.processed;
                let eta_secs = remaining as f64 / rate;
                self.eta = Some(Duration::from_secs_f64(eta_secs));
            }
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.processed + self.failed;
        if total == 0 {
            100.0
        } else {
            (self.processed as f64 / total as f64) * 100.0
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            ((self.processed + self.failed) as f64 / self.total as f64) * 100.0
        }
    }
}

/// Progress tracker for concurrent operations
pub struct ProgressTracker {
    total: AtomicUsize,
    processed: AtomicUsize,
    failed: AtomicUsize,
    bytes_processed: AtomicU64,
    start_time: Instant,
    message: Arc<Mutex<String>>,
    cancelled: AtomicBool,
    config: ProgressConfig,

    #[cfg(feature = "progress")]
    bar: Arc<Mutex<Option<indicatif::ProgressBar>>>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(total: usize, config: ProgressConfig) -> Arc<Self> {
        let tracker = Arc::new(Self {
            total: AtomicUsize::new(total),
            processed: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            bytes_processed: AtomicU64::new(0),
            start_time: Instant::now(),
            message: Arc::new(Mutex::new(String::new())),
            cancelled: AtomicBool::new(false),
            config: config.clone(),

            #[cfg(feature = "progress")]
            bar: Arc::new(Mutex::new(None)),
        });

        #[cfg(feature = "progress")]
        if config.enable_bar {
            tracker.init_bar(total, &config);
        }

        tracker
    }

    /// Create with default configuration
    pub fn with_defaults(total: usize) -> Arc<Self> {
        Self::new(total, ProgressConfig::default())
    }

    /// Create without visual progress bar
    pub fn without_bar(total: usize) -> Arc<Self> {
        let mut config = ProgressConfig::default();
        config.enable_bar = false;
        Self::new(total, config)
    }

    #[cfg(feature = "progress")]
    fn init_bar(&self, total: usize, config: &ProgressConfig) {
        use indicatif::{ProgressBar, ProgressStyle};

        let bar = ProgressBar::new(total as u64);

        let template = config.template.as_deref().unwrap_or(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        );

        if let Ok(style) = ProgressStyle::with_template(template) {
            bar.set_style(
                style
                    .progress_chars("#>-")
                    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
            );
        }

        *self.bar.lock() = Some(bar);
    }

    /// Increment processed count
    pub fn inc(&self, n: usize) {
        self.processed.fetch_add(n, Ordering::Relaxed);

        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.inc(n as u64);
        }
    }

    /// Increment failed count
    pub fn inc_failed(&self, n: usize) {
        self.failed.fetch_add(n, Ordering::Relaxed);

        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.inc(n as u64);
        }
    }

    /// Add bytes processed
    pub fn add_bytes(&self, bytes: u64) {
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Set progress message
    pub fn set_message(&self, msg: impl Into<String>) {
        let msg = msg.into();
        *self.message.lock() = msg.clone();

        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.set_message(msg);
        }
    }

    /// Set total count (for dynamic totals)
    pub fn set_total(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);

        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.set_length(total as u64);
        }
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Request cancellation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);

        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.abandon_with_message("Cancelled");
        }
    }

    /// Get current state
    pub fn state(&self) -> ProgressState {
        let mut state = ProgressState::new(self.total.load(Ordering::Relaxed));
        state.processed = self.processed.load(Ordering::Relaxed);
        state.failed = self.failed.load(Ordering::Relaxed);
        state.bytes_processed = self.bytes_processed.load(Ordering::Relaxed);
        state.message = self.message.lock().clone();
        state.cancelled = self.cancelled.load(Ordering::Relaxed);
        state.start_time = self.start_time;
        state.update();
        state
    }

    /// Finish progress tracking
    pub fn finish(&self) {
        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            let state = self.state();
            let msg = format!(
                "Complete! {}/{} files ({:.1}% success, {:.1} files/s, {:.2} MB/s)",
                state.processed,
                state.total,
                state.success_rate(),
                state.fps,
                state.mbps
            );
            bar.finish_with_message(msg);
        }
    }

    /// Finish with error message
    pub fn finish_with_error(&self, error: &str) {
        #[cfg(feature = "progress")]
        if let Some(ref bar) = *self.bar.lock() {
            bar.abandon_with_message(format!("Error: {}", error));
        }
    }

    /// Get throughput in files per second
    pub fn throughput_fps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.processed.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get throughput in MB per second
    pub fn throughput_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            (self.bytes_processed.load(Ordering::Relaxed) as f64 / 1_048_576.0) / elapsed
        } else {
            0.0
        }
    }
}

/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(&ProgressState) + Send + Sync>;

/// Progress tracker with custom callbacks
pub struct CallbackProgressTracker {
    tracker: Arc<ProgressTracker>,
    callbacks: Arc<Mutex<Vec<ProgressCallback>>>,
    last_update: Arc<Mutex<Instant>>,
}

impl CallbackProgressTracker {
    pub fn new(total: usize, config: ProgressConfig) -> Self {
        Self {
            tracker: ProgressTracker::new(total, config),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn add_callback<F>(&self, callback: F)
    where
        F: Fn(&ProgressState) + Send + Sync + 'static,
    {
        self.callbacks.lock().push(Arc::new(callback));
    }

    pub fn inc(&self, n: usize) {
        self.tracker.inc(n);
        self.maybe_trigger_callbacks();
    }

    pub fn inc_failed(&self, n: usize) {
        self.tracker.inc_failed(n);
        self.maybe_trigger_callbacks();
    }

    pub fn add_bytes(&self, bytes: u64) {
        self.tracker.add_bytes(bytes);
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        self.tracker.set_message(msg);
    }

    pub fn is_cancelled(&self) -> bool {
        self.tracker.is_cancelled()
    }

    pub fn cancel(&self) {
        self.tracker.cancel();
    }

    pub fn state(&self) -> ProgressState {
        self.tracker.state()
    }

    pub fn finish(&self) {
        self.tracker.finish();
        self.trigger_callbacks();
    }

    fn maybe_trigger_callbacks(&self) {
        let mut last_update = self.last_update.lock();
        let now = Instant::now();

        if now.duration_since(*last_update).as_millis() >= self.tracker.config.update_interval_ms as u128 {
            *last_update = now;
            drop(last_update);
            self.trigger_callbacks();
        }
    }

    fn trigger_callbacks(&self) {
        let state = self.tracker.state();
        let callbacks = self.callbacks.lock();

        for callback in callbacks.iter() {
            callback(&state);
        }
    }
}

/// Simplified progress reporter without visual bar
#[derive(Debug)]
pub struct SimpleProgressReporter {
    total: AtomicUsize,
    processed: AtomicUsize,
    failed: AtomicUsize,
    start_time: Instant,
}

impl SimpleProgressReporter {
    pub fn new(total: usize) -> Arc<Self> {
        Arc::new(Self {
            total: AtomicUsize::new(total),
            processed: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            start_time: Instant::now(),
        })
    }

    pub fn inc(&self) {
        self.processed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_failed(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn report(&self) -> String {
        let total = self.total.load(Ordering::Relaxed);
        let processed = self.processed.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        let fps = if elapsed.as_secs_f64() > 0.0 {
            processed as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        format!(
            "{}/{} files processed ({} failed, {:.1} files/s, {:.1}s elapsed)",
            processed,
            total,
            failed,
            fps,
            elapsed.as_secs_f64()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_state() {
        let mut state = ProgressState::new(100);
        state.processed = 50;
        state.failed = 5;
        state.update();

        assert_eq!(state.total, 100);
        assert_eq!(state.processed, 50);
        assert_eq!(state.failed, 5);
        assert!(state.percentage() > 0.0);
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::without_bar(100);
        tracker.inc(10);
        tracker.inc_failed(2);
        tracker.add_bytes(1024);

        let state = tracker.state();
        assert_eq!(state.processed, 10);
        assert_eq!(state.failed, 2);
        assert_eq!(state.bytes_processed, 1024);
    }

    #[test]
    fn test_progress_cancellation() {
        let tracker = ProgressTracker::without_bar(100);
        assert!(!tracker.is_cancelled());

        tracker.cancel();
        assert!(tracker.is_cancelled());
    }

    #[test]
    fn test_simple_reporter() {
        let reporter = SimpleProgressReporter::new(100);
        reporter.inc();
        reporter.inc();
        reporter.inc_failed();

        let report = reporter.report();
        assert!(report.contains("2/100"));
        assert!(report.contains("1 failed"));
    }

    #[test]
    fn test_callback_tracker() {
        let tracker = CallbackProgressTracker::new(10, ProgressConfig::default());
        let called = Arc::new(AtomicUsize::new(0));
        let called_clone = Arc::clone(&called);

        tracker.add_callback(move |_state| {
            called_clone.fetch_add(1, Ordering::Relaxed);
        });

        tracker.inc(5);
        tracker.finish();

        // Callback should be called at least once
        assert!(called.load(Ordering::Relaxed) > 0);
    }
}
