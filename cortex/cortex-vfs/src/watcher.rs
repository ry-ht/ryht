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
use tracing::{debug, warn};

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
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(100),
            batch_interval: Duration::from_millis(500),
            max_batch_size: 100,
            coalesce_events: true,
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
            config,
        ));

        Ok(Self {
            _watcher: watcher,
            receiver: coalesced_rx,
            _coalescer_handle: coalescer_handle,
        })
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
        };

        assert_eq!(config.debounce_duration, Duration::from_millis(50));
        assert_eq!(config.max_batch_size, 10);
        assert!(config.coalesce_events);
    }
}
