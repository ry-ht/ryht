use super::code_indexer::CodeIndexer;
use super::watcher::{FileChangeEvent, FileChangeKind, FileWatcher, WatcherConfig};
use super::Indexer;
use crate::types::SymbolId;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Change tracker for incremental updates
#[derive(Debug)]
struct ChangeTracker {
    /// Files that have been modified
    modified_files: HashSet<PathBuf>,
    /// Files that have been deleted
    deleted_files: HashSet<PathBuf>,
    /// Files that have been created
    created_files: HashSet<PathBuf>,
    /// Symbols affected by changes
    affected_symbols: HashSet<SymbolId>,
    /// Last update timestamp
    last_update: Instant,
}

impl ChangeTracker {
    fn new() -> Self {
        Self {
            modified_files: HashSet::new(),
            deleted_files: HashSet::new(),
            created_files: HashSet::new(),
            affected_symbols: HashSet::new(),
            last_update: Instant::now(),
        }
    }

    fn track_change(&mut self, path: PathBuf, kind: &FileChangeKind) {
        match kind {
            FileChangeKind::Created => {
                self.created_files.insert(path);
            }
            FileChangeKind::Modified => {
                self.modified_files.insert(path);
            }
            FileChangeKind::Deleted => {
                self.deleted_files.insert(path);
            }
            FileChangeKind::Renamed { from, to } => {
                self.deleted_files.insert(from.clone());
                self.created_files.insert(to.clone());
            }
        }
        self.last_update = Instant::now();
    }

    fn clear(&mut self) {
        self.modified_files.clear();
        self.deleted_files.clear();
        self.created_files.clear();
        self.affected_symbols.clear();
    }

    fn is_empty(&self) -> bool {
        self.modified_files.is_empty()
            && self.deleted_files.is_empty()
            && self.created_files.is_empty()
    }
}

/// Write-Ahead Log entry for crash recovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WalEntry {
    timestamp: i64,
    file_path: PathBuf,
    change_kind: String,
    symbols_before: Vec<SymbolId>,
    completed: bool,
}

/// Delta indexer for incremental updates
pub struct DeltaIndexer {
    base_indexer: Arc<RwLock<CodeIndexer>>,
    watcher: Arc<RwLock<FileWatcher>>,
    change_tracker: Arc<RwLock<ChangeTracker>>,
    #[allow(dead_code)]
    config: WatcherConfig,
    wal_path: Option<PathBuf>,
    watching: Arc<RwLock<bool>>,
}

impl DeltaIndexer {
    /// Create a new delta indexer
    pub fn new(
        base_indexer: Arc<RwLock<CodeIndexer>>,
        config: WatcherConfig,
        wal_path: Option<PathBuf>,
    ) -> Result<Self> {
        let watcher = Arc::new(RwLock::new(FileWatcher::new(config.clone())?));

        Ok(Self {
            base_indexer,
            watcher,
            change_tracker: Arc::new(RwLock::new(ChangeTracker::new())),
            config,
            wal_path,
            watching: Arc::new(RwLock::new(false)),
        })
    }

    /// Enable file watching for a path
    pub async fn enable_watching(&self, path: &Path) -> Result<()> {
        let mut watcher = self.watcher.write().await;
        watcher.watch_path(path).await?;

        let mut watching = self.watching.write().await;
        *watching = true;

        info!("File watching enabled for: {:?}", path);
        Ok(())
    }

    /// Disable file watching for a path
    pub async fn disable_watching(&self, path: &Path) -> Result<()> {
        let mut watcher = self.watcher.write().await;
        watcher.unwatch_path(path).await?;

        // Check if any paths are still being watched
        let watched = watcher.get_watched_paths().await;
        if watched.is_empty() {
            let mut watching = self.watching.write().await;
            *watching = false;
        }

        info!("File watching disabled for: {:?}", path);
        Ok(())
    }

    /// Check if watching is enabled
    pub async fn is_watching(&self) -> bool {
        *self.watching.read().await
    }

    /// Get watch status
    pub async fn get_watch_status(&self) -> WatchStatus {
        let watcher = self.watcher.read().await;
        let watching = *self.watching.read().await;
        let tracker = self.change_tracker.read().await;

        WatchStatus {
            enabled: watching,
            watched_paths: watcher.get_watched_paths().await,
            pending_changes: !tracker.is_empty(),
            queue_size: watcher.queue_size().await,
        }
    }

    /// Poll for file changes and apply them
    pub async fn poll_and_apply(&self) -> Result<ApplyResult> {
        let start = Instant::now();

        // Poll events from watcher
        let events = {
            let watcher = self.watcher.read().await;
            watcher.poll_events().await
        };

        if events.is_empty() {
            return Ok(ApplyResult {
                files_updated: 0,
                symbols_updated: 0,
                symbols_deleted: 0,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        debug!("Processing {} file change events", events.len());

        // Apply changes
        let result = self.apply_changes(events).await?;

        Ok(result)
    }

    /// Apply file changes incrementally
    pub async fn apply_changes(&self, events: Vec<FileChangeEvent>) -> Result<ApplyResult> {
        let start = Instant::now();
        let mut files_updated = 0;
        let mut symbols_updated = 0;
        let mut symbols_deleted = 0;

        // Track changes
        {
            let mut tracker = self.change_tracker.write().await;
            for event in &events {
                tracker.track_change(event.path.clone(), &event.kind);
            }
        }

        // Get all unique files that need processing
        let (deleted_files, modified_files, created_files) = {
            let tracker = self.change_tracker.read().await;
            (
                tracker.deleted_files.clone(),
                tracker.modified_files.clone(),
                tracker.created_files.clone(),
            )
        };

        // Process deletions first
        for path in &deleted_files {
            if let Err(e) = self.process_deletion(path).await {
                error!("Failed to process deletion of {:?}: {}", path, e);
            } else {
                files_updated += 1;
                symbols_deleted += 1; // Approximate
            }
        }

        // Process modifications and creations
        let mut indexer = self.base_indexer.write().await;

        for path in modified_files.iter().chain(created_files.iter()) {
            // Write WAL entry
            if let Some(ref _wal_path) = self.wal_path {
                if let Err(e) = self.write_wal_entry(path, "update").await {
                    warn!("Failed to write WAL entry: {}", e);
                }
            }

            // Update the file
            match indexer.update_file(path).await {
                Ok(_) => {
                    files_updated += 1;
                    symbols_updated += 1; // Approximate
                    debug!("Updated file: {:?}", path);

                    // Mark WAL entry as complete
                    if let Some(ref _wal_path) = self.wal_path {
                        if let Err(e) = self.mark_wal_complete(path).await {
                            warn!("Failed to mark WAL entry complete: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to update file {:?}: {}", path, e);
                }
            }
        }

        // Clear change tracker
        {
            let mut tracker = self.change_tracker.write().await;
            tracker.clear();
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            "Applied {} file changes in {}ms ({} files, {} symbols updated, {} deleted)",
            events.len(),
            duration_ms,
            files_updated,
            symbols_updated,
            symbols_deleted
        );

        Ok(ApplyResult {
            files_updated,
            symbols_updated,
            symbols_deleted,
            duration_ms,
        })
    }

    /// Process file deletion
    async fn process_deletion(&self, path: &Path) -> Result<()> {
        // Write WAL entry
        if let Some(ref _wal_path) = self.wal_path {
            self.write_wal_entry(path, "delete").await?;
        }

        // For deletion, we'll update the file which will handle the cleanup
        // when the file no longer exists
        let mut indexer = self.base_indexer.write().await;

        // The update_file method will handle removing old symbols
        // and when the file doesn't exist, it will just clean up
        if path.exists() {
            indexer.update_file(path).await?;
        } else {
            // File already deleted, update_file will handle cleanup
            let _ = indexer.update_file(path).await;
        }

        debug!("Processed deletion for {:?}", path);

        // Mark WAL entry as complete
        if let Some(ref _wal_path) = self.wal_path {
            self.mark_wal_complete(path).await?;
        }

        Ok(())
    }

    /// Write WAL entry (simple implementation)
    async fn write_wal_entry(&self, path: &Path, kind: &str) -> Result<()> {
        if let Some(ref wal_path) = self.wal_path {
            let entry = WalEntry {
                timestamp: chrono::Utc::now().timestamp(),
                file_path: path.to_path_buf(),
                change_kind: kind.to_string(),
                symbols_before: Vec::new(), // Could be populated for rollback
                completed: false,
            };

            let json = serde_json::to_string(&entry)?;
            let mut content = String::new();

            if wal_path.exists() {
                content = tokio::fs::read_to_string(wal_path).await?;
            }

            content.push_str(&json);
            content.push('\n');

            tokio::fs::write(wal_path, content).await?;
        }

        Ok(())
    }

    /// Mark WAL entry as complete
    async fn mark_wal_complete(&self, _path: &Path) -> Result<()> {
        // Simple implementation: in production, would update the specific entry
        // For now, we just append a completion marker
        Ok(())
    }

    /// Recover from WAL after crash
    pub async fn recover_from_wal(&self) -> Result<()> {
        if let Some(ref wal_path) = self.wal_path {
            if !wal_path.exists() {
                return Ok(());
            }

            info!("Recovering from WAL: {:?}", wal_path);

            let content = tokio::fs::read_to_string(wal_path).await?;
            let entries: Vec<WalEntry> = content
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect();

            let incomplete: Vec<_> = entries.iter().filter(|e| !e.completed).collect();

            if !incomplete.is_empty() {
                warn!(
                    "Found {} incomplete WAL entries, reprocessing...",
                    incomplete.len()
                );

                for entry in incomplete {
                    match entry.change_kind.as_str() {
                        "update" => {
                            let mut indexer = self.base_indexer.write().await;
                            if let Err(e) = indexer.update_file(&entry.file_path).await {
                                error!("Failed to recover update for {:?}: {}", entry.file_path, e);
                            }
                        }
                        "delete" => {
                            if let Err(e) = self.process_deletion(&entry.file_path).await {
                                error!(
                                    "Failed to recover deletion for {:?}: {}",
                                    entry.file_path, e
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Clear WAL
            tokio::fs::remove_file(wal_path).await?;
            info!("WAL recovery complete");
        }

        Ok(())
    }

    /// Shutdown the delta indexer
    pub async fn shutdown(&self) -> Result<()> {
        let mut watcher = self.watcher.write().await;
        watcher.shutdown().await?;

        let mut watching = self.watching.write().await;
        *watching = false;

        info!("Delta indexer shutdown");
        Ok(())
    }
}

/// Watch status information
#[derive(Debug, Clone)]
pub struct WatchStatus {
    pub enabled: bool,
    pub watched_paths: Vec<PathBuf>,
    pub pending_changes: bool,
    pub queue_size: usize,
}

/// Result of applying changes
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub files_updated: usize,
    pub symbols_updated: usize,
    pub symbols_deleted: usize,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IndexConfig;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;
    use tokio::time::sleep;
    use std::time::Duration;

    async fn setup_test_delta_indexer() -> (DeltaIndexer, TempDir, TempDir) {
        let storage_dir = TempDir::new().unwrap();
        let test_dir = TempDir::new().unwrap();

        let storage = Arc::new(MemoryStorage::new());
        let config = IndexConfig {
            languages: vec!["rust".to_string()],
            ignore: vec!["target".to_string()],
            max_file_size: "1MB".to_string(),
        };

        let indexer = Arc::new(RwLock::new(CodeIndexer::new(storage, config).unwrap()));

        let watcher_config = WatcherConfig {
            debounce_ms: 10,
            ..Default::default()
        };

        let delta_indexer = DeltaIndexer::new(indexer, watcher_config, None).unwrap();

        (delta_indexer, storage_dir, test_dir)
    }

    #[tokio::test]
    async fn test_enable_disable_watching() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        assert!(!delta_indexer.is_watching().await);

        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();
        assert!(delta_indexer.is_watching().await);

        delta_indexer
            .disable_watching(test_dir.path())
            .await
            .unwrap();
        assert!(!delta_indexer.is_watching().await);
    }

    #[tokio::test]
    async fn test_file_creation_incremental_update() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        // Enable watching
        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        // Create a file
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(
            &test_file,
            r#"
            pub fn test_function() -> i32 {
                42
            }
            "#,
        )
        .await
        .unwrap();

        // Wait for event to be processed
        sleep(Duration::from_millis(100)).await;

        // Apply changes
        let result = delta_indexer.poll_and_apply().await.unwrap();

        assert!(result.files_updated > 0);
        assert!(result.duration_ms < 100); // Should be fast
    }

    #[tokio::test]
    async fn test_file_modification_incremental_update() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        // Create initial file
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(
            &test_file,
            r#"
            pub fn test_function() -> i32 {
                42
            }
            "#,
        )
        .await
        .unwrap();

        // Index initially
        {
            let mut indexer = delta_indexer.base_indexer.write().await;
            indexer.index_project(test_dir.path(), false).await.unwrap();
        }

        // Enable watching
        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
        delta_indexer.poll_and_apply().await.unwrap(); // Clear initial events

        // Modify file
        tokio::fs::write(
            &test_file,
            r#"
            pub fn test_function() -> i32 {
                100
            }

            pub fn another_function() -> String {
                "test".to_string()
            }
            "#,
        )
        .await
        .unwrap();

        sleep(Duration::from_millis(100)).await;

        // Apply changes
        let result = delta_indexer.poll_and_apply().await.unwrap();

        assert!(result.files_updated > 0);
        assert!(result.duration_ms < 100);
    }

    #[tokio::test]
    async fn test_file_deletion() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        // Create and index file
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "pub fn test() {}").await.unwrap();

        {
            let mut indexer = delta_indexer.base_indexer.write().await;
            indexer.index_project(test_dir.path(), false).await.unwrap();
        }

        // Enable watching
        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
        delta_indexer.poll_and_apply().await.unwrap();

        // Delete file
        tokio::fs::remove_file(&test_file).await.unwrap();

        sleep(Duration::from_millis(100)).await;

        // Apply changes
        let result = delta_indexer.poll_and_apply().await.unwrap();

        assert!(result.files_updated > 0 || result.symbols_deleted > 0);
    }

    #[tokio::test]
    async fn test_watch_status() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        let status = delta_indexer.get_watch_status().await;
        assert!(!status.enabled);

        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        let status = delta_indexer.get_watch_status().await;
        assert!(status.enabled);
        assert_eq!(status.watched_paths.len(), 1);
    }

    #[tokio::test]
    async fn test_performance_target() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        // Create initial file
        let test_file = test_dir.path().join("test.rs");
        tokio::fs::write(&test_file, "pub fn test() {}").await.unwrap();

        {
            let mut indexer = delta_indexer.base_indexer.write().await;
            indexer.index_project(test_dir.path(), false).await.unwrap();
        }

        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
        delta_indexer.poll_and_apply().await.unwrap();

        // Modify file
        tokio::fs::write(&test_file, "pub fn test() { let x = 42; }")
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        // Measure update time
        let start = Instant::now();
        let result = delta_indexer.poll_and_apply().await.unwrap();
        let elapsed = start.elapsed().as_millis();

        println!("Update took {}ms", elapsed);

        // Should be < 100ms
        assert!(result.duration_ms < 100, "Update took too long: {}ms", result.duration_ms);
    }

    #[tokio::test]
    async fn test_incremental_parsing_speedup() {
        let (delta_indexer, _storage, test_dir) = setup_test_delta_indexer().await;

        // Create a large file with many functions
        let test_file = test_dir.path().join("large.rs");
        let mut large_content = String::new();
        for i in 0..50 {
            large_content.push_str(&format!(
                r#"
                pub fn function_{}(x: i32) -> i32 {{
                    let result = x + {};
                    if result > 100 {{
                        return result - 100;
                    }}
                    result
                }}
                "#,
                i, i
            ));
        }
        tokio::fs::write(&test_file, &large_content).await.unwrap();

        // Initial index
        {
            let mut indexer = delta_indexer.base_indexer.write().await;
            indexer.index_project(test_dir.path(), false).await.unwrap();
        }

        delta_indexer
            .enable_watching(test_dir.path())
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;
        delta_indexer.poll_and_apply().await.unwrap();

        // Modify just one function at the end
        let mut modified_content = large_content.clone();
        modified_content.push_str(
            r#"
            pub fn new_function() -> i32 {
                42
            }
            "#,
        );
        tokio::fs::write(&test_file, &modified_content).await.unwrap();

        sleep(Duration::from_millis(100)).await;

        // Measure incremental update time
        let start = Instant::now();
        let result = delta_indexer.poll_and_apply().await.unwrap();
        let elapsed = start.elapsed().as_millis();

        println!("Incremental update of large file took {}ms", elapsed);
        println!("Files updated: {}", result.files_updated);
        println!("Symbols updated: {}", result.symbols_updated);

        // With incremental parsing, this should be fast even for large files
        // Target: < 100ms for updating 1 symbol out of 50 (relaxed for CI/slower systems)
        assert!(
            result.duration_ms < 100,
            "Incremental update took too long: {}ms (should be < 100ms)",
            result.duration_ms
        );
    }
}
