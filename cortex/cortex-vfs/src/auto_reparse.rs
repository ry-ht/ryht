//! Automatic file re-parsing system with debouncing.
//!
//! This module implements a background task that monitors file changes in VFS
//! and automatically triggers re-parsing through the ingestion pipeline.
//!
//! # Architecture
//!
//! 1. File changes are sent to a channel when `update_file()` is called
//! 2. A background task debounces changes (waits for `debounce_ms` after last change)
//! 3. After debounce period, files are re-parsed using FileIngestionPipeline
//! 4. Old CodeUnits are marked as replaced and new ones are stored
//!
//! # Example
//!
//! ```no_run
//! use cortex_vfs::{VirtualFileSystem, AutoReparseConfig};
//! use cortex_storage::ConnectionManager;
//! use std::sync::Arc;
//!
//! # async fn example() -> cortex_core::error::Result<()> {
//! let storage = Arc::new(ConnectionManager::default());
//! let config = AutoReparseConfig {
//!     enabled: true,
//!     debounce_ms: 500,
//!     ..Default::default()
//! };
//! let vfs = VirtualFileSystem::with_auto_reparse(storage, config);
//! # Ok(())
//! # }
//! ```

use crate::path::VirtualPath;
use crate::types::AutoReparseConfig;
use crate::ingestion::FileIngestionPipeline;
use cortex_core::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, Instant};
use tracing::{debug, info, warn, error};
use uuid::Uuid;

// Optional notification service support
// This is a soft dependency - if not provided, notifications are disabled
type NotificationCallback = Arc<dyn Fn(Uuid, &str, serde_json::Value) + Send + Sync>;

/// Message sent when a file is modified.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Workspace ID containing the file
    pub workspace_id: Uuid,

    /// Path to the modified file
    pub path: VirtualPath,

    /// Timestamp of the change
    pub timestamp: Instant,
}

/// Handle for controlling the auto-reparse system.
pub struct AutoReparseHandle {
    /// Sender for file change events
    tx: mpsc::UnboundedSender<FileChangeEvent>,

    /// Configuration
    config: AutoReparseConfig,

    /// Optional notification callback
    notification_callback: Option<NotificationCallback>,
}

impl AutoReparseHandle {
    /// Create a new auto-reparse system and spawn the background task.
    pub fn new(
        config: AutoReparseConfig,
        ingestion_pipeline: Option<Arc<FileIngestionPipeline>>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        if config.enabled {
            // Ensure pipeline is provided when enabled
            if let Some(pipeline) = ingestion_pipeline {
                // Spawn background task
                let config_clone = config.clone();
                tokio::spawn(async move {
                    Self::reparse_worker(rx, config_clone, pipeline, None).await;
                });
                info!("Auto-reparse system started with {}ms debounce", config.debounce_ms);
            } else {
                warn!("Auto-reparse enabled but no ingestion pipeline provided");
            }
        } else {
            debug!("Auto-reparse system disabled");
        }

        Self {
            tx,
            config,
            notification_callback: None,
        }
    }

    /// Create a new auto-reparse system with notification support.
    pub fn with_notifications(
        config: AutoReparseConfig,
        ingestion_pipeline: Option<Arc<FileIngestionPipeline>>,
        notification_callback: NotificationCallback,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        if config.enabled {
            // Ensure pipeline is provided when enabled
            if let Some(pipeline) = ingestion_pipeline {
                // Spawn background task with notification support
                let config_clone = config.clone();
                let callback_clone = Arc::clone(&notification_callback);
                tokio::spawn(async move {
                    Self::reparse_worker(rx, config_clone, pipeline, Some(callback_clone)).await;
                });
                info!("Auto-reparse system started with notifications and {}ms debounce", config.debounce_ms);
            } else {
                warn!("Auto-reparse enabled but no ingestion pipeline provided");
            }
        } else {
            debug!("Auto-reparse system disabled");
        }

        Self {
            tx,
            config,
            notification_callback: Some(notification_callback),
        }
    }

    /// Notify the system that a file has been modified.
    pub fn notify_file_changed(&self, workspace_id: Uuid, path: VirtualPath) {
        if !self.config.enabled {
            return;
        }

        let event = FileChangeEvent {
            workspace_id,
            path,
            timestamp: Instant::now(),
        };

        // Send is non-blocking and will drop the event if the receiver is gone
        if let Err(e) = self.tx.send(event) {
            warn!("Failed to send file change event: {}", e);
        }
    }

    /// Background worker that debounces changes and triggers re-parsing.
    async fn reparse_worker(
        mut rx: mpsc::UnboundedReceiver<FileChangeEvent>,
        config: AutoReparseConfig,
        ingestion_pipeline: Arc<FileIngestionPipeline>,
        notification_callback: Option<NotificationCallback>,
    ) {
        // Map of (workspace_id, path) -> last change timestamp
        let mut pending_changes: HashMap<(Uuid, String), Instant> = HashMap::new();

        // Debounce timer
        let debounce_duration = Duration::from_millis(config.debounce_ms);

        loop {
            tokio::select! {
                // Receive new file change events
                Some(event) = rx.recv() => {
                    let key = (event.workspace_id, event.path.to_string());
                    debug!("File change detected: {} in workspace {}", event.path, event.workspace_id);
                    pending_changes.insert(key, event.timestamp);

                    // If we have too many pending changes, force a parse
                    if pending_changes.len() >= config.max_pending_changes {
                        debug!("Max pending changes reached ({}), forcing parse", config.max_pending_changes);
                        Self::process_pending_changes(
                            &mut pending_changes,
                            &ingestion_pipeline,
                            &notification_callback,
                        ).await;
                    }
                }

                // Periodic check for debounced changes
                _ = sleep(Duration::from_millis(100)) => {
                    if pending_changes.is_empty() {
                        continue;
                    }

                    let now = Instant::now();
                    let mut to_process = Vec::new();

                    // Find changes that have passed the debounce period
                    for ((workspace_id, path), timestamp) in &pending_changes {
                        if now.duration_since(*timestamp) >= debounce_duration {
                            to_process.push((*workspace_id, path.clone()));
                        }
                    }

                    // Process debounced changes
                    if !to_process.is_empty() {
                        debug!("Processing {} debounced file changes", to_process.len());

                        for (workspace_id, path_str) in to_process {
                            match VirtualPath::new(&path_str) {
                                Ok(path) => {
                                    // Remove from pending before processing
                                    pending_changes.remove(&(workspace_id, path_str.clone()));

                                    // Trigger re-parse
                                    if let Err(e) = Self::reparse_file(
                                        &workspace_id,
                                        &path,
                                        &ingestion_pipeline,
                                        &notification_callback,
                                    ).await {
                                        error!("Failed to reparse {}: {}", path, e);
                                    }
                                }
                                Err(e) => {
                                    error!("Invalid path {}: {}", path_str, e);
                                    pending_changes.remove(&(workspace_id, path_str));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Process all pending changes immediately.
    async fn process_pending_changes(
        pending_changes: &mut HashMap<(Uuid, String), Instant>,
        ingestion_pipeline: &Arc<FileIngestionPipeline>,
        notification_callback: &Option<NotificationCallback>,
    ) {
        let changes: Vec<_> = pending_changes.keys().cloned().collect();

        for (workspace_id, path_str) in changes {
            match VirtualPath::new(&path_str) {
                Ok(path) => {
                    if let Err(e) = Self::reparse_file(
                        &workspace_id,
                        &path,
                        ingestion_pipeline,
                        notification_callback,
                    ).await {
                        error!("Failed to reparse {}: {}", path, e);
                    }
                }
                Err(e) => {
                    error!("Invalid path {}: {}", path_str, e);
                }
            }
            pending_changes.remove(&(workspace_id, path_str));
        }
    }

    /// Re-parse a single file.
    async fn reparse_file(
        workspace_id: &Uuid,
        path: &VirtualPath,
        ingestion_pipeline: &Arc<FileIngestionPipeline>,
        notification_callback: &Option<NotificationCallback>,
    ) -> Result<()> {
        debug!("Re-parsing file: {} in workspace {}", path, workspace_id);

        // Mark old code units as replaced before parsing
        // The ingestion pipeline will handle this by querying existing units
        // and marking them as CodeUnitStatus::Replaced
        match ingestion_pipeline.mark_old_units_replaced(workspace_id, path).await {
            Ok(count) => {
                debug!("Marked {} old code units as replaced for {}", count, path);
            }
            Err(e) => {
                warn!("Failed to mark old units as replaced: {}", e);
                // Continue with re-parsing anyway
            }
        }

        // Trigger re-parse
        match ingestion_pipeline.ingest_file(workspace_id, path).await {
            Ok(result) => {
                info!(
                    "Successfully re-parsed {} in {}ms: {} units stored",
                    path,
                    result.duration_ms,
                    result.units_stored
                );

                // Send notification if callback is provided
                if let Some(callback) = notification_callback {
                    let data = serde_json::json!({
                        "file_path": path.to_string(),
                        "units_stored": result.units_stored,
                        "duration_ms": result.duration_ms,
                        "errors": result.errors.len(),
                        "status": "success"
                    });
                    callback(*workspace_id, &path.to_string(), data);
                }

                if !result.errors.is_empty() {
                    warn!(
                        "Re-parse completed with {} errors: {:?}",
                        result.errors.len(),
                        result.errors
                    );
                }

                Ok(())
            }
            Err(e) => {
                error!("Failed to re-parse {}: {}", path, e);

                // Send error notification if callback is provided
                if let Some(callback) = notification_callback {
                    let data = serde_json::json!({
                        "file_path": path.to_string(),
                        "error": e.to_string(),
                        "status": "error"
                    });
                    callback(*workspace_id, &path.to_string(), data);
                }

                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_filesystem::VirtualFileSystem;
    use cortex_storage::ConnectionManager;
    use cortex_storage::connection_pool::{ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy};
    use cortex_code_analysis::CodeParser;
    use cortex_memory::SemanticMemorySystem;

    async fn create_test_setup() -> (AutoReparseHandle, Arc<FileIngestionPipeline>, Arc<VirtualFileSystem>, Uuid) {
        let config = DatabaseConfig {
            connection_mode: ConnectionMode::InMemory,
            credentials: Credentials {
                username: None,
                password: None,
            },
            pool_config: PoolConfig {
                min_connections: 0,
                max_connections: 5,
                connection_timeout: Duration::from_secs(5),
                idle_timeout: Some(Duration::from_secs(30)),
                max_lifetime: Some(Duration::from_secs(60)),
                retry_policy: RetryPolicy::default(),
                warm_connections: false,
                validate_on_checkout: false,
                recycle_after_uses: Some(10000),
                shutdown_grace_period: Duration::from_secs(30),
            },
            namespace: "test".to_string(),
            database: "test".to_string(),
        };
        let storage = Arc::new(ConnectionManager::new(config).await.unwrap());

        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new().unwrap()));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage));

        let pipeline = Arc::new(FileIngestionPipeline::new(parser, vfs.clone(), semantic_memory));

        let auto_reparse_config = AutoReparseConfig {
            enabled: true,
            debounce_ms: 100, // Short for tests
            max_pending_changes: 5,
            background_parsing: true,
        };

        let handle = AutoReparseHandle::new(auto_reparse_config, Some(pipeline.clone()));
        let workspace_id = Uuid::new_v4();

        (handle, pipeline, vfs, workspace_id)
    }

    #[tokio::test]
    async fn test_auto_reparse_disabled() {
        let config = AutoReparseConfig {
            enabled: false,
            ..Default::default()
        };

        // Just ensure it doesn't panic when disabled
        let handle = AutoReparseHandle::new(
            config,
            None, // No pipeline needed when disabled
        );

        let workspace_id = Uuid::new_v4();
        let path = VirtualPath::new("test.rs").unwrap();

        // Should not panic
        handle.notify_file_changed(workspace_id, path);
    }

    #[tokio::test]
    async fn test_file_change_notification() {
        let (handle, _pipeline, vfs, workspace_id) = create_test_setup().await;

        // Create a test document (VFS now only allows documents, not code files)
        let path = VirtualPath::new("docs/test.md").unwrap();
        let content = b"# Test Document\n\nThis is a test.";

        vfs.write_file(&workspace_id, &path, content).await.unwrap();

        // Notify of change
        handle.notify_file_changed(workspace_id, path.clone());

        // Wait for debounce + processing
        tokio::time::sleep(Duration::from_millis(300)).await;

        // If no panic, test passes
    }
}
