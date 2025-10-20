use anyhow::{Context, Result};
use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// File change event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub kind: FileChangeKind,
    pub timestamp: Instant,
}

impl FileChangeEvent {
    pub fn new(path: PathBuf, kind: FileChangeKind) -> Self {
        Self {
            path,
            kind,
            timestamp: Instant::now(),
        }
    }
}

/// File watcher configuration
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration in milliseconds (coalesce rapid changes)
    pub debounce_ms: u64,
    /// Maximum queue size before dropping events
    pub max_queue_size: usize,
    /// Extensions to watch (e.g., ["rs", "ts", "js"])
    pub watched_extensions: Vec<String>,
    /// Paths to ignore (e.g., ["target", "node_modules"])
    pub ignored_paths: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 50,
            max_queue_size: 10000,
            watched_extensions: vec![
                "rs".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
                "py".to_string(),
                "go".to_string(),
            ],
            ignored_paths: vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        }
    }
}

/// File watcher for real-time change detection
pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    event_queue: Arc<StdMutex<VecDeque<FileChangeEvent>>>,
    config: WatcherConfig,
    watched_paths: Arc<StdMutex<Vec<PathBuf>>>,
    // Debounce state: path -> last event time
    #[allow(dead_code)]
    debounce_map: Arc<StdMutex<std::collections::HashMap<PathBuf, Instant>>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatcherConfig) -> Result<Self> {
        let event_queue = Arc::new(StdMutex::new(VecDeque::new()));
        let debounce_map = Arc::new(StdMutex::new(std::collections::HashMap::new()));
        let queue_clone = event_queue.clone();
        let debounce_clone = debounce_map.clone();
        let config_clone = config.clone();

        // Create the watcher with event handler (synchronous)
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = Self::handle_event_sync(&event, &queue_clone, &debounce_clone, &config_clone)
                        {
                            warn!("Error handling file event: {}", e);
                        }
                    }
                    Err(e) => warn!("File watcher error: {}", e),
                }
            },
            Config::default(),
        )
        .context("Failed to create file watcher")?;

        Ok(Self {
            watcher: Some(watcher),
            event_queue,
            config,
            watched_paths: Arc::new(StdMutex::new(Vec::new())),
            debounce_map,
        })
    }

    /// Watch a path recursively
    pub async fn watch_path(&mut self, path: &Path) -> Result<()> {
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch path: {:?}", path))?;

            self.watched_paths.lock().unwrap().push(path.to_path_buf());
            debug!("Started watching path: {:?}", path);
        }
        Ok(())
    }

    /// Stop watching a path
    pub async fn unwatch_path(&mut self, path: &Path) -> Result<()> {
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .unwatch(path)
                .with_context(|| format!("Failed to unwatch path: {:?}", path))?;

            self.watched_paths.lock().unwrap().retain(|p| p != path);
            debug!("Stopped watching path: {:?}", path);
        }
        Ok(())
    }

    /// Get all watched paths
    pub async fn get_watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().unwrap().clone()
    }

    /// Poll events from the queue (non-blocking)
    pub async fn poll_events(&self) -> Vec<FileChangeEvent> {
        let mut queue = self.event_queue.lock().unwrap();
        let events: Vec<_> = queue.drain(..).collect();
        events
    }

    /// Get the number of events in the queue
    pub async fn queue_size(&self) -> usize {
        self.event_queue.lock().unwrap().len()
    }

    /// Clear the event queue
    pub async fn clear_queue(&self) {
        self.event_queue.lock().unwrap().clear();
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check ignored paths
        for ignored in &self.config.ignored_paths {
            if path_str.contains(ignored) {
                return true;
            }
        }

        // Check if extension is watched
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if !self.config.watched_extensions.is_empty()
                && !self.config.watched_extensions.contains(&ext.to_string())
            {
                return true;
            }
        } else {
            // No extension, ignore
            return true;
        }

        false
    }

    /// Handle a file system event (synchronous to avoid tokio runtime issues)
    fn handle_event_sync(
        event: &Event,
        queue: &Arc<StdMutex<VecDeque<FileChangeEvent>>>,
        debounce: &Arc<StdMutex<std::collections::HashMap<PathBuf, Instant>>>,
        config: &WatcherConfig,
    ) -> Result<()> {
        // Extract paths from event
        let paths: Vec<PathBuf> = event.paths.clone();

        for path in paths {
            // Create temporary watcher to check if path should be ignored
            let temp_config = config.clone();
            let temp_watcher = Self {
                watcher: None,
                event_queue: queue.clone(),
                config: temp_config,
                watched_paths: Arc::new(StdMutex::new(Vec::new())),
                debounce_map: debounce.clone(),
            };

            if temp_watcher.should_ignore(&path) {
                continue;
            }

            // Check debounce
            let now = Instant::now();
            let mut debounce_map = debounce.lock().unwrap();

            if let Some(last_time) = debounce_map.get(&path) {
                if now.duration_since(*last_time) < Duration::from_millis(config.debounce_ms) {
                    // Too soon, skip this event
                    continue;
                }
            }

            debounce_map.insert(path.clone(), now);
            drop(debounce_map); // Release lock

            // Determine event kind
            let change_kind = match event.kind {
                EventKind::Create(_) => FileChangeKind::Created,
                EventKind::Modify(_) => FileChangeKind::Modified,
                EventKind::Remove(_) => FileChangeKind::Deleted,
                _ => continue, // Ignore other event types
            };

            // Create change event
            let change_event = FileChangeEvent::new(path.clone(), change_kind);

            // Add to queue
            let mut q = queue.lock().unwrap();
            if q.len() >= config.max_queue_size {
                // Queue is full, drop oldest event
                q.pop_front();
                warn!("Event queue full, dropping oldest event");
            }
            q.push_back(change_event);
            debug!("Queued file change event: {:?}", path);
        }

        Ok(())
    }

    /// Stop watching all paths and shutdown
    pub async fn shutdown(&mut self) -> Result<()> {
        let paths = self.watched_paths.lock().unwrap().clone();
        for path in paths {
            let _ = self.unwatch_path(&path).await;
        }

        self.watcher = None;
        debug!("File watcher shutdown");
        Ok(())
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        // Watcher will automatically stop when dropped
        debug!("FileWatcher dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_file_watcher_create() {
        let config = WatcherConfig::default();
        let watcher = FileWatcher::new(config);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_watch_path() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig::default();
        let mut watcher = FileWatcher::new(config).unwrap();

        let result = watcher.watch_path(temp_dir.path()).await;
        assert!(result.is_ok());

        let watched = watcher.get_watched_paths().await;
        assert_eq!(watched.len(), 1);
    }

    #[tokio::test]
    async fn test_file_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            debounce_ms: 10,
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(config).unwrap();

        watcher.watch_path(temp_dir.path()).await.unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "// test").await.unwrap();

        // Wait for event to be processed
        sleep(Duration::from_millis(100)).await;

        // Check events
        let events = watcher.poll_events().await;
        assert!(!events.is_empty());

        // Find create event
        let has_create = events
            .iter()
            .any(|e| matches!(e.kind, FileChangeKind::Created));
        assert!(has_create);

        // Explicit shutdown to avoid destructor panics
        watcher.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_modification_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            debounce_ms: 10,
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(config).unwrap();

        // Create file first
        let test_file = temp_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "// initial").await.unwrap();

        watcher.watch_path(temp_dir.path()).await.unwrap();

        // Clear initial events
        sleep(Duration::from_millis(100)).await;
        watcher.clear_queue().await;

        // Modify file
        tokio::fs::write(&test_file, "// modified").await.unwrap();

        // Wait for event
        sleep(Duration::from_millis(100)).await;

        let events = watcher.poll_events().await;
        assert!(!events.is_empty());

        let has_modify = events
            .iter()
            .any(|e| matches!(e.kind, FileChangeKind::Modified));
        assert!(has_modify);

        watcher.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_debouncing() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            debounce_ms: 200, // 200ms debounce
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(config).unwrap();

        watcher.watch_path(temp_dir.path()).await.unwrap();

        let test_file = temp_dir.path().join("test.rs");

        // Create file
        tokio::fs::write(&test_file, "// v1").await.unwrap();
        sleep(Duration::from_millis(50)).await;

        // Modify quickly (should be debounced)
        tokio::fs::write(&test_file, "// v2").await.unwrap();
        sleep(Duration::from_millis(50)).await;

        tokio::fs::write(&test_file, "// v3").await.unwrap();
        sleep(Duration::from_millis(50)).await;

        // Wait for debounce window
        sleep(Duration::from_millis(250)).await;

        let events = watcher.poll_events().await;

        // Should have fewer events due to debouncing
        // Exact count depends on timing, but should be < 3
        assert!(events.len() < 3);

        watcher.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            debounce_ms: 10,
            ignored_paths: vec!["ignored".to_string()],
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(config).unwrap();

        watcher.watch_path(temp_dir.path()).await.unwrap();

        // Create ignored directory
        let ignored_dir = temp_dir.path().join("ignored");
        tokio::fs::create_dir(&ignored_dir).await.unwrap();

        // Create file in ignored directory
        let ignored_file = ignored_dir.join("test.rs");
        tokio::fs::write(&ignored_file, "// ignored").await.unwrap();

        // Create normal file
        let normal_file = temp_dir.path().join("normal.rs");
        tokio::fs::write(&normal_file, "// normal").await.unwrap();

        sleep(Duration::from_millis(100)).await;

        let events = watcher.poll_events().await;

        // Should only have event for normal file
        let has_ignored = events
            .iter()
            .any(|e| e.path.to_string_lossy().contains("ignored"));
        assert!(!has_ignored);

        watcher.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_extension_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            debounce_ms: 10,
            watched_extensions: vec!["rs".to_string()],
            ..Default::default()
        };
        let mut watcher = FileWatcher::new(config).unwrap();

        watcher.watch_path(temp_dir.path()).await.unwrap();

        // Create rs file (should be watched)
        let rs_file = temp_dir.path().join("test.rs");
        tokio::fs::write(&rs_file, "// rust").await.unwrap();

        // Create txt file (should be ignored)
        let txt_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&txt_file, "text").await.unwrap();

        sleep(Duration::from_millis(100)).await;

        let events = watcher.poll_events().await;

        // Should only have event for .rs file
        let all_rs = events.iter().all(|e| {
            e.path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s == "rs")
                .unwrap_or(false)
        });
        assert!(all_rs);

        watcher.shutdown().await.unwrap();
    }
}
