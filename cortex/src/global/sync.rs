//! Synchronization manager for global-local coordination
//!
//! Implements the synchronization strategy from global-architecture-spec:
//! - Push sync: Local changes → Global DB
//! - Pull sync: Global DB → Local cache
//! - Periodic sync: Scheduled synchronization every 5 minutes
//! - Cache invalidation: On file changes, re-index, or TTL expiration

use super::registry::ProjectRegistryManager;
use super::storage::GlobalStorage;
use super::watcher::{FileChangeEvent, GlobalFileWatcher};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Synchronization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of items synced
    pub items_synced: usize,

    /// Number of items that failed to sync
    pub errors: usize,

    /// Duration of the sync operation
    pub duration_ms: u64,

    /// Sync direction
    pub direction: SyncDirection,

    /// Project ID (if specific to a project)
    pub project_id: Option<String>,

    /// Timestamp of the sync
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    Push, // Local → Global
    Pull, // Global → Local
    Bidirectional,
}

/// Pending change to be synced
#[derive(Debug, Clone)]
struct PendingChange {
    /// Project ID
    project_id: String,

    /// Changed file paths
    paths: Vec<PathBuf>,

    /// Timestamp of first change
    first_seen: DateTime<Utc>,

    /// Number of changes accumulated
    change_count: usize,
}

/// Synchronization manager
pub struct SyncManager {
    /// Registry manager
    registry: Arc<ProjectRegistryManager>,

    /// Global storage
    storage: Arc<GlobalStorage>,

    /// File watcher
    #[allow(dead_code)]
    file_watcher: Arc<GlobalFileWatcher>,

    /// Pending changes (debounced)
    pending_changes: Arc<RwLock<HashMap<String, PendingChange>>>,

    /// Periodic sync interval (default: 5 minutes)
    sync_interval: Duration,

    /// Debounce delay for file changes (default: 500ms)
    debounce_delay: Duration,

    /// Shutdown signal
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new(
        registry: Arc<ProjectRegistryManager>,
        storage: Arc<GlobalStorage>,
        file_watcher: Arc<GlobalFileWatcher>,
    ) -> Self {
        Self {
            registry,
            storage,
            file_watcher,
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
            sync_interval: Duration::from_secs(300), // 5 minutes
            debounce_delay: Duration::from_millis(500),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Push sync: local changes → global
    pub async fn push_sync(&self, project_id: &str) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        info!("Starting push sync for project: {}", project_id);

        let mut items_synced = 0;
        let mut errors = 0;

        // Get project registry
        let project = self
            .registry
            .get(project_id)
            .await?
            .context("Project not found")?;

        // Store updated project in global storage
        match self.storage.put_project(&project).await {
            Ok(_) => {
                items_synced += 1;
                debug!("Pushed project metadata to global storage: {}", project_id);
            }
            Err(e) => {
                warn!("Failed to push project {}: {}", project_id, e);
                errors += 1;
            }
        }

        // TODO: Push code symbols, documentation, etc.
        // This will be implemented when we integrate with the indexer

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = SyncResult {
            items_synced,
            errors,
            duration_ms,
            direction: SyncDirection::Push,
            project_id: Some(project_id.to_string()),
            timestamp: Utc::now(),
        };

        info!(
            "Push sync completed for {}: {} items, {} errors, {}ms",
            project_id, items_synced, errors, duration_ms
        );

        Ok(result)
    }

    /// Pull sync: global → local cache
    pub async fn pull_sync(&self, project_id: &str) -> Result<SyncResult> {
        let start = std::time::Instant::now();
        info!("Starting pull sync for project: {}", project_id);

        let mut items_synced = 0;
        let mut errors = 0;

        // Get project from global storage
        match self.storage.get_project(project_id).await {
            Ok(Some(_project)) => {
                items_synced += 1;
                debug!("Pulled project metadata from global storage: {}", project_id);

                // TODO: Update local cache
                // This will be integrated with the LocalCache in a future step
            }
            Ok(None) => {
                warn!("Project not found in global storage: {}", project_id);
                errors += 1;
            }
            Err(e) => {
                warn!("Failed to pull project {}: {}", project_id, e);
                errors += 1;
            }
        }

        // TODO: Pull code symbols, documentation, etc.

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = SyncResult {
            items_synced,
            errors,
            duration_ms,
            direction: SyncDirection::Pull,
            project_id: Some(project_id.to_string()),
            timestamp: Utc::now(),
        };

        info!(
            "Pull sync completed for {}: {} items, {} errors, {}ms",
            project_id, items_synced, errors, duration_ms
        );

        Ok(result)
    }

    /// Start periodic synchronization
    pub async fn start_periodic_sync(&self) -> Result<()> {
        info!(
            "Starting periodic sync with interval: {:?}",
            self.sync_interval
        );

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        {
            let mut tx = self.shutdown_tx.write().await;
            *tx = Some(shutdown_tx);
        }

        let registry = Arc::clone(&self.registry);
        let storage = Arc::clone(&self.storage);
        let pending_changes = Arc::clone(&self.pending_changes);
        let sync_interval = self.sync_interval;
        let debounce_delay = self.debounce_delay;

        tokio::spawn(async move {
            let mut sync_timer = tokio::time::interval(sync_interval);
            let mut debounce_timer = tokio::time::interval(debounce_delay);

            loop {
                tokio::select! {
                    _ = sync_timer.tick() => {
                        debug!("Periodic sync triggered");

                        // Sync all active projects
                        match registry.list_all().await {
                            Ok(projects) => {
                                for project in projects {
                                    // Push each project to global storage
                                    if let Err(e) = storage.put_project(&project).await {
                                        warn!(
                                            "Failed to sync project {} during periodic sync: {}",
                                            project.identity.full_id, e
                                        );
                                    } else {
                                        debug!("Synced project: {}", project.identity.full_id);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to list projects during periodic sync: {}", e);
                            }
                        }
                    }

                    _ = debounce_timer.tick() => {
                        // Process pending changes
                        let changes_to_sync = {
                            let mut pending = pending_changes.write().await;
                            let ready: Vec<PendingChange> = pending
                                .values()
                                .filter(|change| {
                                    let age = Utc::now()
                                        .signed_duration_since(change.first_seen)
                                        .to_std()
                                        .unwrap_or_default();
                                    age >= debounce_delay
                                })
                                .cloned()
                                .collect();

                            // Remove the changes we're about to process
                            for change in &ready {
                                pending.remove(&change.project_id);
                            }

                            ready
                        };

                        if !changes_to_sync.is_empty() {
                            debug!("Processing {} pending changes", changes_to_sync.len());

                            for change in changes_to_sync {
                                // Push sync for the changed project
                                if let Ok(Some(project)) = registry.get(&change.project_id).await {
                                    if let Err(e) = storage.put_project(&project).await {
                                        warn!(
                                            "Failed to sync changed project {}: {}",
                                            change.project_id, e
                                        );
                                    } else {
                                        debug!(
                                            "Synced changed project {} ({} changes)",
                                            change.project_id, change.change_count
                                        );
                                    }
                                }
                            }
                        }
                    }

                    _ = shutdown_rx.recv() => {
                        info!("Periodic sync shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop periodic synchronization
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping sync manager");

        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
        }

        Ok(())
    }

    /// Invalidate cache for a project
    pub async fn invalidate_cache(
        &self,
        project_id: &str,
        paths: Vec<PathBuf>,
    ) -> Result<()> {
        debug!(
            "Invalidating cache for project {} ({} paths)",
            project_id,
            paths.len()
        );

        // Add to pending changes
        let mut pending = self.pending_changes.write().await;

        if let Some(existing) = pending.get_mut(project_id) {
            // Add new paths
            for path in paths {
                if !existing.paths.contains(&path) {
                    existing.paths.push(path);
                }
            }
            existing.change_count += 1;
        } else {
            // Create new pending change
            pending.insert(
                project_id.to_string(),
                PendingChange {
                    project_id: project_id.to_string(),
                    paths,
                    first_seen: Utc::now(),
                    change_count: 1,
                },
            );
        }

        Ok(())
    }

    /// Handle file change event from watcher
    pub async fn handle_file_change(&self, event: FileChangeEvent) -> Result<()> {
        if let Some(project_id) = event.project_id {
            debug!(
                "Handling file change for project {}: {:?}",
                project_id, event.path
            );

            self.invalidate_cache(&project_id, vec![event.path])
                .await?;
        }

        Ok(())
    }

    /// Get synchronization statistics
    pub async fn get_stats(&self) -> SyncStats {
        let pending = self.pending_changes.read().await;

        SyncStats {
            pending_changes: pending.len(),
            total_pending_paths: pending.values().map(|p| p.paths.len()).sum(),
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    /// Number of projects with pending changes
    pub pending_changes: usize,

    /// Total number of pending file paths
    pub total_pending_paths: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::watcher::WatcherConfig;
    use tempfile::TempDir;

    async fn create_test_sync_manager() -> (SyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(GlobalStorage::new(temp_dir.path()).await.unwrap());
        let registry = Arc::new(ProjectRegistryManager::new(Arc::clone(&storage)));
        let watcher_config = WatcherConfig::default();
        let watcher = Arc::new(GlobalFileWatcher::new(
            watcher_config,
            Arc::clone(&registry),
        ));

        let sync_manager = SyncManager::new(registry, storage, watcher);

        (sync_manager, temp_dir)
    }

    #[tokio::test]
    async fn test_sync_manager_creation() {
        let (_sync_manager, _temp) = create_test_sync_manager().await;
        // Should create successfully
    }

    #[tokio::test]
    async fn test_invalidate_cache() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let project_id = "test-project";
        let paths = vec![PathBuf::from("src/main.rs")];

        sync_manager
            .invalidate_cache(project_id, paths.clone())
            .await
            .unwrap();

        let stats = sync_manager.get_stats().await;
        assert_eq!(stats.pending_changes, 1);
        assert_eq!(stats.total_pending_paths, 1);
    }

    #[tokio::test]
    async fn test_invalidate_cache_accumulation() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let project_id = "test-project";

        // Add multiple changes
        sync_manager
            .invalidate_cache(project_id, vec![PathBuf::from("src/main.rs")])
            .await
            .unwrap();

        sync_manager
            .invalidate_cache(project_id, vec![PathBuf::from("src/lib.rs")])
            .await
            .unwrap();

        let stats = sync_manager.get_stats().await;
        assert_eq!(stats.pending_changes, 1);
        assert_eq!(stats.total_pending_paths, 2);
    }

    #[tokio::test]
    async fn test_handle_file_change() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let event = FileChangeEvent {
            path: PathBuf::from("src/main.rs"),
            kind: crate::global::watcher::FileChangeKind::Modified,
            project_id: Some("test-project".to_string()),
        };

        sync_manager.handle_file_change(event).await.unwrap();

        let stats = sync_manager.get_stats().await;
        assert_eq!(stats.pending_changes, 1);
    }

    #[tokio::test]
    async fn test_periodic_sync_lifecycle() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        // Start periodic sync
        sync_manager.start_periodic_sync().await.unwrap();

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stop it
        sync_manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_push_sync_nonexistent_project() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let result = sync_manager.push_sync("nonexistent-project").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pull_sync_nonexistent_project() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let result = sync_manager.pull_sync("nonexistent-project").await;
        assert!(result.is_ok());

        let sync_result = result.unwrap();
        assert_eq!(sync_result.errors, 1);
        assert_eq!(sync_result.items_synced, 0);
    }

    #[tokio::test]
    async fn test_sync_stats() {
        let (sync_manager, _temp) = create_test_sync_manager().await;

        let stats = sync_manager.get_stats().await;
        assert_eq!(stats.pending_changes, 0);
        assert_eq!(stats.total_pending_paths, 0);

        // Add some pending changes
        sync_manager
            .invalidate_cache(
                "project1",
                vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
            )
            .await
            .unwrap();

        sync_manager
            .invalidate_cache("project2", vec![PathBuf::from("c.rs")])
            .await
            .unwrap();

        let stats = sync_manager.get_stats().await;
        assert_eq!(stats.pending_changes, 2);
        assert_eq!(stats.total_pending_paths, 3);
    }
}
