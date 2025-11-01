//! File system watcher for detecting changes with debouncing and change coalescing.

use cortex_core::error::{CortexError, Result};
use dashmap::DashMap;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{debug, warn, info};
use uuid::Uuid;

// Optional notification service support
type NotificationCallback = Arc<dyn Fn(Uuid, Vec<String>, serde_json::Value) + Send + Sync>;

/// Events emitted by the file watcher
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Configuration for file watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce duration (wait this long after last event before emitting)
    pub debounce_duration: Duration,

    /// Batch interval (emit batched events at this interval)
    pub batch_interval: Duration,

    /// Maximum batch size before forcing emission
    pub max_batch_size: usize,

    /// Whether to coalesce multiple events for same path
    pub coalesce_events: bool,

    /// Whether to trigger auto-reparse on file changes
    pub enable_auto_reparse: bool,

    /// Whether to sync changes from disk to VFS automatically
    pub enable_auto_sync: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(100),
            batch_interval: Duration::from_millis(500),
            max_batch_size: 100,
            coalesce_events: true,
            enable_auto_reparse: false,
            enable_auto_sync: false,
        }
    }
}

/// Pending event for a path
#[derive(Debug, Clone)]
struct PendingEvent {
    event: FileEvent,
    last_updated: Instant,
}

/// File system watcher for monitoring changes
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::UnboundedReceiver<Vec<FileEvent>>,
    _coalescer_handle: JoinHandle<()>,
    config: WatcherConfig,
    workspace_id: Option<Uuid>,
    vfs: Option<Arc<crate::VirtualFileSystem>>,
    auto_reparse: Option<Arc<crate::auto_reparse::AutoReparseHandle>>,
    watched_path: PathBuf,
    /// Statistics for monitoring
    pub stats: Arc<DashMap<String, u64>>,
    /// Optional notification callback
    notification_callback: Option<NotificationCallback>,
}

impl FileWatcher {
    /// Create a new file watcher with default configuration
    pub fn new(path: &Path) -> Result<Self> {
        Self::with_config(path, WatcherConfig::default())
    }

    /// Create a new file watcher with custom configuration
    pub fn with_config(path: &Path, config: WatcherConfig) -> Result<Self> {
        let (raw_tx, raw_rx) = mpsc::unbounded_channel();
        let (coalesced_tx, coalesced_rx) = mpsc::unbounded_channel();

        // Create the notify watcher
        let tx_clone = raw_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if let Some(file_event) = Self::convert_event(event) {
                    let _ = tx_clone.send(file_event);
                }
            }
        })
        .map_err(|e| CortexError::vfs(format!("Failed to create watcher: {}", e)))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| CortexError::vfs(format!("Failed to watch path: {}", e)))?;

        // Spawn coalescing task
        let coalescer_handle = tokio::spawn(Self::coalesce_events(
            raw_rx,
            coalesced_tx,
            config.clone(),
        ));

        let stats = Arc::new(DashMap::new());
        stats.insert("events_received".to_string(), 0);
        stats.insert("events_processed".to_string(), 0);
        stats.insert("files_synced".to_string(), 0);
        stats.insert("files_reparsed".to_string(), 0);
        stats.insert("errors".to_string(), 0);

        Ok(Self {
            _watcher: watcher,
            receiver: coalesced_rx,
            _coalescer_handle: coalescer_handle,
            config,
            workspace_id: None,
            vfs: None,
            auto_reparse: None,
            watched_path: path.to_path_buf(),
            stats,
            notification_callback: None,
        })
    }

    /// Create a file watcher with VFS integration for automatic sync and reparse.
    ///
    /// # Arguments
    ///
    /// * `path` - Physical filesystem path to watch
    /// * `workspace_id` - Workspace ID for VFS operations
    /// * `config` - Watcher configuration
    /// * `vfs` - Virtual filesystem instance
    /// * `auto_reparse` - Optional auto-reparse handle for triggering re-parsing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cortex_vfs::{FileWatcher, WatcherConfig, VirtualFileSystem, AutoReparseHandle};
    /// use std::path::Path;
    /// use std::sync::Arc;
    /// use uuid::Uuid;
    ///
    /// # async fn example() -> cortex_core::error::Result<()> {
    /// let workspace_id = Uuid::new_v4();
    /// let vfs = Arc::new(VirtualFileSystem::new(Arc::new(/* storage */)));
    /// let auto_reparse = Arc::new(/* auto_reparse_handle */);
    ///
    /// let mut config = WatcherConfig::default();
    /// config.enable_auto_sync = true;
    /// config.enable_auto_reparse = true;
    ///
    /// let mut watcher = FileWatcher::with_integration(
    ///     Path::new("/path/to/project"),
    ///     workspace_id,
    ///     config,
    ///     vfs,
    ///     Some(auto_reparse),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_integration(
        path: &Path,
        workspace_id: Uuid,
        config: WatcherConfig,
        vfs: Arc<crate::VirtualFileSystem>,
        auto_reparse: Option<Arc<crate::auto_reparse::AutoReparseHandle>>,
    ) -> Result<Self> {
        let (raw_tx, raw_rx) = mpsc::unbounded_channel();
        let (coalesced_tx, coalesced_rx) = mpsc::unbounded_channel();

        // Create the notify watcher
        let tx_clone = raw_tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if let Some(file_event) = Self::convert_event(event) {
                    let _ = tx_clone.send(file_event);
                }
            }
        })
        .map_err(|e| CortexError::vfs(format!("Failed to create watcher: {}", e)))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| CortexError::vfs(format!("Failed to watch path: {}", e)))?;

        info!(
            "FileWatcher started for path: {} (workspace: {}, auto_sync: {}, auto_reparse: {})",
            path.display(),
            workspace_id,
            config.enable_auto_sync,
            config.enable_auto_reparse
        );

        // Spawn coalescing task
        let coalescer_handle = tokio::spawn(Self::coalesce_events(
            raw_rx,
            coalesced_tx,
            config.clone(),
        ));

        let stats = Arc::new(DashMap::new());
        stats.insert("events_received".to_string(), 0);
        stats.insert("events_processed".to_string(), 0);
        stats.insert("files_synced".to_string(), 0);
        stats.insert("files_reparsed".to_string(), 0);
        stats.insert("errors".to_string(), 0);

        Ok(Self {
            _watcher: watcher,
            receiver: coalesced_rx,
            _coalescer_handle: coalescer_handle,
            config,
            workspace_id: Some(workspace_id),
            vfs: Some(vfs),
            auto_reparse,
            watched_path: path.to_path_buf(),
            stats,
            notification_callback: None,
        })
    }

    /// Set notification callback for file change events
    pub fn set_notification_callback(&mut self, callback: NotificationCallback) {
        self.notification_callback = Some(callback);
    }

    /// Coalesce and debounce events
    async fn coalesce_events(
        mut raw_rx: mpsc::UnboundedReceiver<FileEvent>,
        coalesced_tx: mpsc::UnboundedSender<Vec<FileEvent>>,
        config: WatcherConfig,
    ) {
        let pending: Arc<DashMap<PathBuf, PendingEvent>> = Arc::new(DashMap::new());
        let pending_clone = Arc::clone(&pending);

        // Spawn batch emission task
        tokio::spawn(async move {
            let mut interval_timer = interval(config.batch_interval);
            loop {
                interval_timer.tick().await;

                let now = Instant::now();
                let mut to_emit = Vec::new();

                // Find events ready to emit (past debounce duration)
                pending_clone.retain(|path, event| {
                    if now.duration_since(event.last_updated) >= config.debounce_duration {
                        to_emit.push(event.event.clone());
                        debug!("Emitting debounced event for: {}", path.display());
                        false // Remove from pending
                    } else {
                        true // Keep in pending
                    }
                });

                // Emit batch if we have events
                if !to_emit.is_empty() {
                    if coalesced_tx.send(to_emit).is_err() {
                        // Receiver dropped, stop task
                        break;
                    }
                }

                // Force emit if batch size exceeded
                if pending_clone.len() >= config.max_batch_size {
                    warn!("Max batch size reached, forcing emission");
                    let mut force_emit = Vec::new();
                    pending_clone.retain(|_, event| {
                        force_emit.push(event.event.clone());
                        false
                    });
                    if !force_emit.is_empty() {
                        let _ = coalesced_tx.send(force_emit);
                    }
                }
            }
        });

        // Process raw events
        while let Some(event) = raw_rx.recv().await {
            let path = match &event {
                FileEvent::Created(p) | FileEvent::Modified(p) | FileEvent::Deleted(p) => p.clone(),
                FileEvent::Renamed { to, .. } => to.clone(),
            };

            if config.coalesce_events {
                // Coalesce: update existing event or insert new one
                if let Some(mut existing) = pending.get_mut(&path) {
                    // Merge events intelligently
                    let merged = Self::merge_events(&existing.event, &event);
                    existing.event = merged;
                    existing.last_updated = Instant::now();
                    debug!("Coalesced event for: {}", path.display());
                } else {
                    pending.insert(
                        path.clone(),
                        PendingEvent {
                            event: event.clone(),
                            last_updated: Instant::now(),
                        },
                    );
                    debug!("Queued event for: {}", path.display());
                }
            } else {
                // No coalescing, just debounce
                pending.insert(
                    path,
                    PendingEvent {
                        event,
                        last_updated: Instant::now(),
                    },
                );
            }
        }
    }

    /// Merge two events for the same path intelligently
    fn merge_events(old: &FileEvent, new: &FileEvent) -> FileEvent {
        match (old, new) {
            // Multiple modifications = one modification
            (FileEvent::Modified(_), FileEvent::Modified(p)) => FileEvent::Modified(p.clone()),

            // Created then modified = created
            (FileEvent::Created(p), FileEvent::Modified(_)) => FileEvent::Created(p.clone()),

            // Created then deleted = no-op (but we'll emit delete)
            (FileEvent::Created(_), FileEvent::Deleted(p)) => FileEvent::Deleted(p.clone()),

            // Modified then deleted = deleted
            (FileEvent::Modified(_), FileEvent::Deleted(p)) => FileEvent::Deleted(p.clone()),

            // Deleted then created = modified
            (FileEvent::Deleted(_), FileEvent::Created(p)) => FileEvent::Modified(p.clone()),

            // Any other combination = use new event
            _ => new.clone(),
        }
    }

    /// Convert notify event to FileEvent
    fn convert_event(event: Event) -> Option<FileEvent> {
        if event.paths.is_empty() {
            return None;
        }

        match event.kind {
            EventKind::Create(_) => {
                let path = event.paths[0].clone();
                Some(FileEvent::Created(path))
            }
            EventKind::Modify(_) => {
                let path = event.paths[0].clone();
                Some(FileEvent::Modified(path))
            }
            EventKind::Remove(_) => {
                let path = event.paths[0].clone();
                Some(FileEvent::Deleted(path))
            }
            EventKind::Any if event.paths.len() >= 2 => {
                // Rename detected
                Some(FileEvent::Renamed {
                    from: event.paths[0].clone(),
                    to: event.paths[1].clone(),
                })
            }
            _ => None,
        }
    }

    /// Receive the next batch of file events
    pub async fn recv(&mut self) -> Option<Vec<FileEvent>> {
        self.receiver.recv().await
    }

    /// Try to receive a batch of file events without blocking
    pub fn try_recv(&mut self) -> Option<Vec<FileEvent>> {
        self.receiver.try_recv().ok()
    }

    /// Receive a single event (waits for next batch and returns first event)
    pub async fn recv_one(&mut self) -> Option<FileEvent> {
        self.receiver.recv().await.and_then(|mut batch| {
            if batch.is_empty() {
                None
            } else {
                Some(batch.remove(0))
            }
        })
    }

    /// Process events with automatic VFS sync and reparse integration.
    ///
    /// This method should be called in a loop to continuously process file events.
    /// When events are received:
    /// 1. Updates statistics
    /// 2. Syncs changed files to VFS (if enable_auto_sync is true)
    /// 3. Triggers auto-reparse (if enable_auto_reparse is true)
    ///
    /// # Returns
    ///
    /// Returns `Some(events)` if events were processed, or `None` if the watcher was closed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use cortex_vfs::FileWatcher;
    /// # async fn example(mut watcher: FileWatcher) {
    /// loop {
    ///     if let Some(events) = watcher.process_events().await {
    ///         println!("Processed {} events", events.len());
    ///     } else {
    ///         break; // Watcher closed
    ///     }
    /// }
    /// # }
    /// ```
    pub async fn process_events(&mut self) -> Option<Vec<FileEvent>> {
        let events = self.receiver.recv().await?;

        if events.is_empty() {
            return Some(events);
        }

        // Update statistics
        if let Some(mut count) = self.stats.get_mut("events_received") {
            *count += events.len() as u64;
        }

        // Process events if integration is enabled
        if self.config.enable_auto_sync || self.config.enable_auto_reparse {
            if let (Some(workspace_id), Some(vfs)) = (self.workspace_id, &self.vfs) {
                for event in &events {
                    if let Err(e) = self.process_event(event, workspace_id, vfs).await {
                        warn!("Failed to process event {:?}: {} (Error details: {:?})", event, e, e);
                        if let Some(mut count) = self.stats.get_mut("errors") {
                            *count += 1;
                        }
                    }
                }

                // Send notification about file changes if callback is set
                if let Some(ref callback) = self.notification_callback {
                    let file_paths: Vec<String> = events
                        .iter()
                        .filter_map(|e| match e {
                            FileEvent::Created(p) | FileEvent::Modified(p) | FileEvent::Deleted(p) => {
                                p.to_str().map(|s| s.to_string())
                            }
                            FileEvent::Renamed { to, .. } => to.to_str().map(|s| s.to_string()),
                        })
                        .collect();

                    if !file_paths.is_empty() {
                        let data = serde_json::json!({
                            "event_count": events.len(),
                            "files": file_paths,
                            "watched_path": self.watched_path.to_string_lossy(),
                        });
                        callback(workspace_id, file_paths, data);
                    }
                }
            }
        }

        // Update processed count
        if let Some(mut count) = self.stats.get_mut("events_processed") {
            *count += events.len() as u64;
        }

        Some(events)
    }

    /// Process a single file event with VFS sync and auto-reparse.
    async fn process_event(
        &self,
        event: &FileEvent,
        workspace_id: Uuid,
        vfs: &Arc<crate::VirtualFileSystem>,
    ) -> Result<()> {
        let path = match event {
            FileEvent::Created(p) | FileEvent::Modified(p) => p,
            FileEvent::Deleted(p) => {
                // Sync deletion to VFS if enabled
                if self.config.enable_auto_sync {
                    if let Err(e) = self.sync_deletion_to_vfs(p, workspace_id, vfs).await {
                        warn!("Failed to sync deletion of {} to VFS: {}", p.display(), e);
                        return Err(e);
                    }
                }
                return Ok(());
            }
            FileEvent::Renamed { from, to } => {
                debug!("File renamed: {} -> {}", from.display(), to.display());
                // Use the new path for processing
                to
            }
        };

        // Sync to VFS if enabled
        if self.config.enable_auto_sync {
            if let Err(e) = self.sync_file_to_vfs(path, workspace_id, vfs).await {
                warn!("Failed to sync {} to VFS: {}", path.display(), e);
                return Err(e);
            }
        }

        // Trigger auto-reparse if enabled
        if self.config.enable_auto_reparse {
            if let Some(ref auto_reparse) = self.auto_reparse {
                if let Err(e) = self.trigger_reparse(path, workspace_id, auto_reparse).await {
                    warn!("Failed to trigger reparse for {}: {}", path.display(), e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Sync a file from disk to VFS.
    async fn sync_file_to_vfs(
        &self,
        path: &Path,
        workspace_id: Uuid,
        vfs: &Arc<crate::VirtualFileSystem>,
    ) -> Result<()> {
        // Check if file exists first
        if !path.exists() {
            debug!("Skipping sync for non-existent file: {}", path.display());
            return Ok(());
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            debug!("Skipping sync for non-file: {}", path.display());
            return Ok(());
        }

        // Read file content from disk
        let content = tokio::fs::read(path).await.map_err(|e| {
            CortexError::vfs(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        // Convert physical path to virtual path (relative to watched path)
        let relative_path = path.strip_prefix(&self.watched_path).map_err(|e| {
            CortexError::vfs(format!("Path {} not under watched path: {}", path.display(), e))
        })?;

        let virtual_path = crate::VirtualPath::new(relative_path.to_string_lossy().as_ref())?;

        // Write to VFS
        debug!(
            "Syncing file to VFS: {} -> {} (workspace: {})",
            path.display(),
            virtual_path,
            workspace_id
        );

        vfs.write_file(&workspace_id, &virtual_path, &content).await?;

        // Update statistics
        if let Some(mut count) = self.stats.get_mut("files_synced") {
            *count += 1;
        }

        info!("Synced file to VFS: {} ({} bytes)", virtual_path, content.len());
        Ok(())
    }

    /// Sync file deletion from disk to VFS.
    async fn sync_deletion_to_vfs(
        &self,
        path: &Path,
        workspace_id: Uuid,
        vfs: &Arc<crate::VirtualFileSystem>,
    ) -> Result<()> {
        // Convert physical path to virtual path
        let relative_path = path.strip_prefix(&self.watched_path).map_err(|e| {
            CortexError::vfs(format!("Path {} not under watched path: {}", path.display(), e))
        })?;

        let virtual_path = crate::VirtualPath::new(relative_path.to_string_lossy().as_ref())?;

        debug!("Deleting from VFS: {} (workspace: {})", virtual_path, workspace_id);

        // Delete from VFS (recursive=true to handle both files and directories)
        vfs.delete(&workspace_id, &virtual_path, true).await?;

        // Update statistics
        if let Some(mut count) = self.stats.get_mut("files_synced") {
            *count += 1;
        }

        info!("Deleted from VFS: {}", virtual_path);
        Ok(())
    }

    /// Trigger auto-reparse for a file.
    async fn trigger_reparse(
        &self,
        path: &Path,
        workspace_id: Uuid,
        auto_reparse: &Arc<crate::auto_reparse::AutoReparseHandle>,
    ) -> Result<()> {
        // Convert physical path to virtual path
        let relative_path = path.strip_prefix(&self.watched_path).map_err(|e| {
            CortexError::vfs(format!("Path {} not under watched path: {}", path.display(), e))
        })?;

        let virtual_path = crate::VirtualPath::new(relative_path.to_string_lossy().as_ref())?;

        // Only trigger reparse for code files
        if let Some(ext) = virtual_path.extension() {
            let language = crate::types::Language::from_extension(ext);
            if !matches!(language, crate::types::Language::Unknown) {
                debug!(
                    "Triggering auto-reparse: {} (workspace: {})",
                    virtual_path, workspace_id
                );

                auto_reparse.notify_file_changed(workspace_id, virtual_path);

                // Update statistics
                if let Some(mut count) = self.stats.get_mut("files_reparsed") {
                    *count += 1;
                }
            }
        }

        Ok(())
    }

    /// Get current statistics.
    pub fn get_stats(&self) -> std::collections::HashMap<String, u64> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        for mut entry in self.stats.iter_mut() {
            *entry.value_mut() = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_conversion() {
        let path = PathBuf::from("/test/file.txt");
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let file_event = FileWatcher::convert_event(event);
        assert!(matches!(file_event, Some(FileEvent::Created(_))));
    }

    #[test]
    fn test_event_merging() {
        let path = PathBuf::from("/test/file.txt");

        // Multiple modifications should coalesce
        let old = FileEvent::Modified(path.clone());
        let new = FileEvent::Modified(path.clone());
        let merged = FileWatcher::merge_events(&old, &new);
        assert!(matches!(merged, FileEvent::Modified(_)));

        // Created then modified = created
        let old = FileEvent::Created(path.clone());
        let new = FileEvent::Modified(path.clone());
        let merged = FileWatcher::merge_events(&old, &new);
        assert!(matches!(merged, FileEvent::Created(_)));

        // Created then deleted = deleted
        let old = FileEvent::Created(path.clone());
        let new = FileEvent::Deleted(path.clone());
        let merged = FileWatcher::merge_events(&old, &new);
        assert!(matches!(merged, FileEvent::Deleted(_)));

        // Deleted then created = modified
        let old = FileEvent::Deleted(path.clone());
        let new = FileEvent::Created(path.clone());
        let merged = FileWatcher::merge_events(&old, &new);
        assert!(matches!(merged, FileEvent::Modified(_)));
    }

    #[tokio::test]
    async fn test_watcher_config() {
        let config = WatcherConfig {
            debounce_duration: Duration::from_millis(50),
            batch_interval: Duration::from_millis(200),
            max_batch_size: 10,
            coalesce_events: true,
            enable_auto_reparse: true,
            enable_auto_sync: true,
        };

        assert_eq!(config.debounce_duration, Duration::from_millis(50));
        assert_eq!(config.max_batch_size, 10);
        assert!(config.coalesce_events);
        assert!(config.enable_auto_reparse);
        assert!(config.enable_auto_sync);
    }
}
