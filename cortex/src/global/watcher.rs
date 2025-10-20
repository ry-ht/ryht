//! File watcher for automatic project re-indexing
//!
//! Watches all registered monorepos and triggers incremental re-indexing
//! when source files change. Implements debouncing to avoid excessive
//! re-indexing on rapid file changes.

use super::registry::{ProjectRegistry, ProjectRegistryManager};
use anyhow::{Context, Result};
use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path that changed
    pub path: PathBuf,

    /// Type of change
    pub kind: FileChangeKind,

    /// Project ID that owns this file
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Created,
    Modified,
    Removed,
}

impl From<EventKind> for FileChangeKind {
    fn from(kind: EventKind) -> Self {
        match kind {
            EventKind::Create(_) => FileChangeKind::Created,
            EventKind::Modify(_) => FileChangeKind::Modified,
            EventKind::Remove(_) => FileChangeKind::Removed,
            _ => FileChangeKind::Modified,
        }
    }
}

/// File watcher configuration
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,

    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,

    /// Extensions to watch (empty = all)
    pub watch_extensions: Vec<String>,

    /// Maximum concurrent reindex operations
    pub max_concurrent_reindex: usize,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 500,
            ignore_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".git".to_string(),
                ".meridian".to_string(),
                "*.log".to_string(),
                "*.lock".to_string(),
            ],
            watch_extensions: vec![
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
                "rs".to_string(),
                "toml".to_string(),
                "json".to_string(),
            ],
            max_concurrent_reindex: 4,
        }
    }
}

/// Callback for handling file changes
pub type FileChangeCallback = Arc<dyn Fn(FileChangeEvent) + Send + Sync>;

/// Global file watcher
pub struct GlobalFileWatcher {
    config: WatcherConfig,
    registry_manager: Arc<ProjectRegistryManager>,
    watchers: Arc<RwLock<HashMap<String, RecommendedWatcher>>>,
    pending_changes: Arc<RwLock<HashMap<PathBuf, FileChangeEvent>>>,
    change_callback: Arc<RwLock<Option<FileChangeCallback>>>,
    shutdown_tx: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

impl GlobalFileWatcher {
    /// Create a new global file watcher
    pub fn new(
        config: WatcherConfig,
        registry_manager: Arc<ProjectRegistryManager>,
    ) -> Self {
        Self {
            config,
            registry_manager,
            watchers: Arc::new(RwLock::new(HashMap::new())),
            pending_changes: Arc::new(RwLock::new(HashMap::new())),
            change_callback: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the change callback
    pub async fn set_change_callback(&self, callback: FileChangeCallback) {
        let mut cb = self.change_callback.write().await;
        *cb = Some(callback);
    }

    /// Start watching all registered projects
    pub async fn start(&self) -> Result<()> {
        info!("Starting global file watcher");

        // Get all active projects
        let projects = self.registry_manager.list_all().await?;

        info!("Found {} projects to watch", projects.len());

        // Watch each project
        for project in projects {
            if let Err(e) = self.watch_project(&project).await {
                warn!(
                    "Failed to watch project {}: {}",
                    project.identity.full_id, e
                );
            }
        }

        // Start debounce processor
        self.start_debounce_processor().await?;

        info!("Global file watcher started");
        Ok(())
    }

    /// Stop the file watcher
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping global file watcher");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(()).await;
        }

        // Remove all watchers
        let mut watchers = self.watchers.write().await;
        watchers.clear();

        info!("Global file watcher stopped");
        Ok(())
    }

    /// Watch a specific project
    async fn watch_project(&self, project: &ProjectRegistry) -> Result<()> {
        let project_path = &project.current_path;

        debug!("Setting up watcher for project: {}", project.identity.full_id);

        // Create channel for file events
        let (tx, mut rx) = mpsc::channel(100);

        // Clone Arc for the closure
        let pending_changes = Arc::clone(&self.pending_changes);
        let project_id = project.identity.full_id.clone();
        let ignore_patterns = self.config.ignore_patterns.clone();
        let watch_extensions = self.config.watch_extensions.clone();

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Send event to processing channel
                        let tx = tx.clone();
                        let event = event;
                        tokio::spawn(async move {
                            let _ = tx.send(event).await;
                        });
                    }
                    Err(e) => {
                        error!("File watcher error: {}", e);
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_millis(100)),
        )
        .with_context(|| format!("Failed to create watcher for {:?}", project_path))?;

        // Watch the project directory
        watcher
            .watch(project_path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch path {:?}", project_path))?;

        // Store watcher
        {
            let mut watchers = self.watchers.write().await;
            watchers.insert(project.identity.full_id.clone(), watcher);
        }

        // Process events
        let pending_changes_clone = Arc::clone(&pending_changes);
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // Process each path in the event
                for path in event.paths {
                    // Check if we should ignore this file
                    if should_ignore(&path, &ignore_patterns, &watch_extensions) {
                        continue;
                    }

                    // Create change event
                    let change_event = FileChangeEvent {
                        path: path.clone(),
                        kind: event.kind.into(),
                        project_id: Some(project_id.clone()),
                    };

                    // Add to pending changes
                    let mut pending = pending_changes_clone.write().await;
                    pending.insert(path, change_event);
                }
            }
        });

        debug!("Watcher set up for project: {}", project.identity.full_id);
        Ok(())
    }

    /// Start the debounce processor
    async fn start_debounce_processor(&self) -> Result<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        {
            let mut tx = self.shutdown_tx.write().await;
            *tx = Some(shutdown_tx);
        }

        let pending_changes = Arc::clone(&self.pending_changes);
        let change_callback = Arc::clone(&self.change_callback);
        let debounce_ms = self.config.debounce_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(debounce_ms));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process pending changes
                        let changes = {
                            let mut pending = pending_changes.write().await;
                            let changes: Vec<FileChangeEvent> = pending.values().cloned().collect();
                            pending.clear();
                            changes
                        };

                        if !changes.is_empty() {
                            debug!("Processing {} pending changes", changes.len());

                            // Get callback
                            let callback = change_callback.read().await;
                            if let Some(ref cb) = *callback {
                                for change in changes {
                                    cb(change);
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Debounce processor shutting down");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Add a new project to watch
    pub async fn add_project(&self, project: &ProjectRegistry) -> Result<()> {
        info!("Adding project to watcher: {}", project.identity.full_id);
        self.watch_project(project).await
    }

    /// Remove a project from watching
    pub async fn remove_project(&self, project_id: &str) -> Result<()> {
        info!("Removing project from watcher: {}", project_id);

        let mut watchers = self.watchers.write().await;
        watchers.remove(project_id);

        Ok(())
    }

    /// Update a project path (e.g., after relocation)
    pub async fn update_project(&self, project: &ProjectRegistry) -> Result<()> {
        // Remove old watcher
        self.remove_project(&project.identity.full_id).await?;

        // Add new watcher with updated path
        self.add_project(project).await?;

        Ok(())
    }

    /// Get statistics about watched projects
    pub async fn get_stats(&self) -> WatcherStats {
        let watchers = self.watchers.read().await;
        let pending = self.pending_changes.read().await;

        WatcherStats {
            watched_projects: watchers.len(),
            pending_changes: pending.len(),
        }
    }
}

/// Watcher statistics
#[derive(Debug, Clone)]
pub struct WatcherStats {
    pub watched_projects: usize,
    pub pending_changes: usize,
}

/// Check if a path should be ignored
fn should_ignore(
    path: &Path,
    ignore_patterns: &[String],
    watch_extensions: &[String],
) -> bool {
    let path_str = path.to_string_lossy();

    // Check ignore patterns
    for pattern in ignore_patterns {
        if pattern.contains('*') {
            // Glob pattern
            if path_str.contains(&pattern.replace("*", "")) {
                return true;
            }
        } else if path_str.contains(pattern) {
            return true;
        }
    }

    // Check extensions if specified
    if !watch_extensions.is_empty() {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if !watch_extensions.contains(&ext_str.to_string()) {
                return true;
            }
        } else {
            // No extension - ignore
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::storage::GlobalStorage;
    use tempfile::TempDir;
    use std::fs;

    async fn create_test_watcher() -> (GlobalFileWatcher, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(GlobalStorage::new(temp_dir.path()).await.unwrap());
        let manager = Arc::new(ProjectRegistryManager::new(storage));

        let config = WatcherConfig::default();
        let watcher = GlobalFileWatcher::new(config, manager);

        (watcher, temp_dir)
    }

    #[test]
    fn test_should_ignore() {
        let ignore_patterns = vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "*.log".to_string(),
        ];
        let watch_extensions = vec!["ts".to_string(), "rs".to_string()];

        // Should ignore
        assert!(should_ignore(
            Path::new("node_modules/foo/bar.ts"),
            &ignore_patterns,
            &watch_extensions
        ));
        assert!(should_ignore(
            Path::new(".git/config"),
            &ignore_patterns,
            &watch_extensions
        ));
        assert!(should_ignore(
            Path::new("foo.log"),
            &ignore_patterns,
            &watch_extensions
        ));
        assert!(should_ignore(
            Path::new("foo.js"),
            &[],
            &watch_extensions
        ));

        // Should not ignore
        assert!(!should_ignore(
            Path::new("src/main.ts"),
            &ignore_patterns,
            &watch_extensions
        ));
        assert!(!should_ignore(
            Path::new("lib.rs"),
            &ignore_patterns,
            &watch_extensions
        ));
    }

    #[tokio::test]
    async fn test_watcher_lifecycle() {
        let (watcher, _temp_dir) = create_test_watcher().await;

        // Start watcher
        watcher.start().await.unwrap();

        // Get stats
        let stats = watcher.get_stats().await;
        assert_eq!(stats.watched_projects, 0); // No projects registered

        // Stop watcher
        watcher.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_add_remove_project() {
        let (watcher, _temp_dir) = create_test_watcher().await;

        // Create a test project
        let project_dir = TempDir::new().unwrap();
        fs::write(
            project_dir.path().join("package.json"),
            r#"{"name": "test-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Register project
        let registry = watcher
            .registry_manager
            .register(project_dir.path().to_path_buf())
            .await
            .unwrap();

        // Add to watcher
        watcher.add_project(&registry).await.unwrap();

        let stats = watcher.get_stats().await;
        assert_eq!(stats.watched_projects, 1);

        // Remove from watcher
        watcher.remove_project(&registry.identity.full_id).await.unwrap();

        let stats = watcher.get_stats().await;
        assert_eq!(stats.watched_projects, 0);
    }

    #[tokio::test]
    async fn test_change_callback() {
        let (watcher, _temp_dir) = create_test_watcher().await;

        // Set up a callback
        let changes = Arc::new(RwLock::new(Vec::new()));
        let changes_clone = Arc::clone(&changes);

        watcher
            .set_change_callback(Arc::new(move |event| {
                let changes = Arc::clone(&changes_clone);
                tokio::spawn(async move {
                    let mut list = changes.write().await;
                    list.push(event);
                });
            }))
            .await;

        // Callback is set (we can't easily test actual file events in unit tests)
        assert!(watcher.change_callback.read().await.is_some());
    }
}
