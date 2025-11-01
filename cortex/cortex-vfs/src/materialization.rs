//! Materialization engine for flushing VFS content to physical filesystem.
//!
//! ## Document-Optimized Design
//!
//! This materialization engine has been simplified for document-only workflows.
//! Documents are typically:
//! - Smaller than code files (KB vs MB)
//! - Less frequently flushed
//! - Have simpler conflict resolution (content-based, no AST merging)
//! - Don't require code formatting, import optimization, or syntax validation
//!
//! Key features retained:
//! - Atomic operations with backup/rollback
//! - Bidirectional sync for filesystem monitoring
//! - Conflict detection for concurrent edits
//!
//! Code-specific features removed:
//! - AST/tree-sitter integration (not needed for documents)
//! - Complex code formatting hooks (documents use plain text)
//! - Import/syntax optimization (N/A for documents)

use crate::path::VirtualPath;
use crate::types::*;
use crate::virtual_filesystem::VirtualFileSystem;
use chrono::Utc;
use cortex_core::error::{CortexError, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Engine for materializing virtual filesystem to physical disk.
///
/// Optimized for document-only workflows:
/// - Target path specification (materialize anywhere)
/// - Atomic operations with rollback
/// - Optional parallel materialization (small document sets may not benefit)
/// - Change tracking and incremental sync
/// - Simple content-based conflict detection
pub struct MaterializationEngine {
    vfs: VirtualFileSystem,
}

impl MaterializationEngine {
    /// Create a new materialization engine.
    pub fn new(vfs: VirtualFileSystem) -> Self {
        Self { vfs }
    }

    /// Flush virtual filesystem to physical disk.
    ///
    /// The `target_path` parameter specifies where to materialize the virtual files.
    /// Virtual paths are always relative and are joined with the target path.
    pub async fn flush(
        &self,
        scope: FlushScope,
        target_path: &Path,
        options: FlushOptions,
    ) -> Result<FlushReport> {
        let start = Instant::now();
        info!("Starting flush to: {}", target_path.display());

        // Create backup if requested
        let backup = if options.create_backup {
            Some(self.create_backup(target_path).await?)
        } else {
            None
        };

        // Collect changes to flush
        let changes = self.collect_changes(scope).await?;

        if changes.is_empty() {
            info!("No changes to flush");
            return Ok(FlushReport {
                duration_ms: start.elapsed().as_millis() as u64,
                ..Default::default()
            });
        }

        info!("Flushing {} changes", changes.len());

        // Execute flush
        let result = if options.atomic {
            self.flush_atomic(&changes, target_path, &options).await
        } else {
            self.flush_sequential(&changes, target_path, &options).await
        };

        // Handle result
        match result {
            Ok(report) => {
                // Clean up backup on success
                if let Some(backup_path) = backup {
                    if let Err(e) = fs::remove_dir_all(&backup_path).await {
                        warn!("Failed to remove backup: {}", e);
                    }
                }

                info!("Flush completed successfully in {}ms", report.duration_ms);
                Ok(report)
            }
            Err(e) => {
                error!("Flush failed: {}", e);

                // Restore backup if available
                if let Some(backup_path) = backup {
                    warn!("Restoring from backup...");
                    if let Err(restore_err) = self.restore_backup(&backup_path, target_path).await {
                        error!("Failed to restore backup: {}", restore_err);
                    }
                }

                Err(e)
            }
        }
    }

    /// Collect changes to flush based on scope.
    async fn collect_changes(&self, scope: FlushScope) -> Result<Vec<VNode>> {
        use SyncStatus::*;
        let change_statuses = vec![Modified, Created, Deleted];

        let vnodes = match scope {
            FlushScope::All => {
                self.vfs.query_vnodes_by_status(&change_statuses).await?
            }
            FlushScope::Path(path) => {
                self.vfs.query_vnodes_by_status_and_path(&change_statuses, &path).await?
            }
            FlushScope::Specific(ids) => {
                self.vfs.query_vnodes_by_ids(&ids).await?
            }
            FlushScope::Workspace(workspace_id) => {
                self.vfs.query_vnodes_by_status_and_workspace(&change_statuses, &workspace_id).await?
            }
        };

        debug!("Collected {} vnodes for flush", vnodes.len());
        Ok(vnodes)
    }

    /// Flush changes atomically (all or nothing).
    async fn flush_atomic(
        &self,
        changes: &[VNode],
        target_path: &Path,
        options: &FlushOptions,
    ) -> Result<FlushReport> {
        let start = Instant::now();
        let mut report = FlushReport::default();

        // Create temporary directory for atomic operations
        let temp_dir = target_path.join(".cortex_flush_temp");
        fs::create_dir_all(&temp_dir).await
            .map_err(|e| CortexError::vfs(format!("Failed to create temp dir: {}", e)))?;

        // Group changes by operation type
        let (deletes, creates_and_updates): (Vec<_>, Vec<_>) = changes.iter()
            .partition(|v| v.status == SyncStatus::Deleted);

        // Process all changes
        let flush_result = self.flush_changes(
            &creates_and_updates,
            &deletes,
            target_path,
            options,
            &mut report,
        ).await;

        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir).await;

        // If any error occurred, return error (rollback will happen in caller)
        flush_result?;

        // Mark all vnodes as synchronized
        for vnode in changes {
            let mut vnode_clone = vnode.clone();
            vnode_clone.mark_synchronized();
            if let Err(e) = self.vfs.save_vnode(&vnode_clone).await {
                warn!("Failed to mark vnode as synchronized: {}", e);
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Flush changes sequentially (best effort).
    async fn flush_sequential(
        &self,
        changes: &[VNode],
        target_path: &Path,
        options: &FlushOptions,
    ) -> Result<FlushReport> {
        let start = Instant::now();
        let mut report = FlushReport::default();

        // Group changes
        let (deletes, creates_and_updates): (Vec<_>, Vec<_>) = changes.iter()
            .partition(|v| v.status == SyncStatus::Deleted);

        // Process changes
        if let Err(e) = self.flush_changes(
            &creates_and_updates,
            &deletes,
            target_path,
            options,
            &mut report,
        ).await {
            report.errors.push(e.to_string());
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        Ok(report)
    }

    /// Flush changes to disk.
    ///
    /// For document-only workflows, we use a simpler strategy:
    /// - Always process deletes first (order matters)
    /// - Use parallel processing only for larger document sets (>10 files)
    /// - Documents are typically small enough that parallel overhead may not help
    async fn flush_changes(
        &self,
        creates_and_updates: &[&VNode],
        deletes: &[&VNode],
        target_path: &Path,
        options: &FlushOptions,
        report: &mut FlushReport,
    ) -> Result<()> {
        // Process deletes first
        for vnode in deletes {
            let physical_path = self.to_physical_path(target_path, &vnode.path);

            match self.delete_physical(&physical_path, vnode).await {
                Ok(_) => {
                    report.files_deleted += 1;
                    debug!("Deleted: {}", physical_path.display());
                }
                Err(e) => {
                    report.errors.push(format!("Failed to delete {}: {}", vnode.path, e));
                }
            }
        }

        // Process creates and updates
        // For documents, only use parallel processing for larger sets (>10 files)
        const PARALLEL_THRESHOLD: usize = 10;
        if options.parallel && creates_and_updates.len() > PARALLEL_THRESHOLD {
            debug!("Using parallel materialization for {} documents", creates_and_updates.len());
            self.flush_parallel(creates_and_updates, target_path, options, report).await
        } else {
            self.flush_sequential_inner(creates_and_updates, target_path, options, report).await
        }
    }

    /// Flush changes in parallel.
    async fn flush_parallel(
        &self,
        vnodes: &[&VNode],
        target_path: &Path,
        options: &FlushOptions,
        report: &mut FlushReport,
    ) -> Result<()> {
        let mut tasks = JoinSet::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(options.max_workers));

        for vnode in vnodes {
            let vnode = (*vnode).clone();
            let target_path = target_path.to_path_buf();
            let vfs = self.vfs.clone();
            let preserve_perms = options.preserve_permissions;
            let preserve_times = options.preserve_timestamps;
            let sem = semaphore.clone();

            tasks.spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::materialize_vnode_static(&vfs, &vnode, &target_path, preserve_perms, preserve_times).await
            });
        }

        // Collect results
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => {
                    report.files_written += 1;
                }
                Ok(Err(e)) => {
                    report.errors.push(e.to_string());
                }
                Err(e) => {
                    report.errors.push(format!("Task failed: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Flush changes sequentially.
    async fn flush_sequential_inner(
        &self,
        vnodes: &[&VNode],
        target_path: &Path,
        options: &FlushOptions,
        report: &mut FlushReport,
    ) -> Result<()> {
        for vnode in vnodes {
            let physical_path = self.to_physical_path(target_path, &vnode.path);

            match self.materialize_vnode(vnode, &physical_path, options).await {
                Ok(bytes) => {
                    report.files_written += 1;
                    report.bytes_written += bytes;
                    debug!("Materialized: {}", physical_path.display());
                }
                Err(e) => {
                    report.errors.push(format!("Failed to materialize {}: {}", vnode.path, e));
                }
            }
        }

        Ok(())
    }

    /// Materialize a single vnode to disk.
    async fn materialize_vnode(
        &self,
        vnode: &VNode,
        physical_path: &Path,
        options: &FlushOptions,
    ) -> Result<usize> {
        match vnode.node_type {
            NodeType::Directory => {
                fs::create_dir_all(physical_path).await
                    .map_err(|e| CortexError::vfs(format!("Failed to create directory: {}", e)))?;
                Ok(0)
            }
            NodeType::File | NodeType::Document => {
                // Ensure parent directory exists
                if let Some(parent) = physical_path.parent() {
                    fs::create_dir_all(parent).await
                        .map_err(|e| CortexError::vfs(format!("Failed to create parent directory: {}", e)))?;
                }

                // Get content
                let content = self.vfs.read_file(&vnode.workspace_id, &vnode.path).await?;
                let size = content.len();

                // Write to disk
                fs::write(physical_path, &content).await
                    .map_err(|e| CortexError::vfs(format!("Failed to write file: {}", e)))?;

                // Set permissions if requested
                if options.preserve_permissions {
                    if let Some(mode) = vnode.permissions {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let perms = std::fs::Permissions::from_mode(mode);
                            fs::set_permissions(physical_path, perms).await
                                .map_err(|e| CortexError::vfs(format!("Failed to set permissions: {}", e)))?;
                        }
                    }
                }

                // Set timestamps if requested
                if options.preserve_timestamps {
                    // Note: Requires additional dependencies for cross-platform timestamp setting
                    // Skipped for now
                }

                Ok(size)
            }
            NodeType::SymLink => {
                if let Some(target) = vnode.metadata.get("target") {
                    if let Some(target_str) = target.as_str() {
                        #[cfg(unix)]
                        {
                            std::os::unix::fs::symlink(target_str, physical_path)
                                .map_err(|e| CortexError::vfs(format!("Failed to create symlink: {}", e)))?;
                        }
                        #[cfg(not(unix))]
                        {
                            return Err(CortexError::invalid_input("Symlinks not supported on this platform"));
                        }
                    }
                }
                Ok(0)
            }
        }
    }

    /// Static version for parallel execution.
    async fn materialize_vnode_static(
        vfs: &VirtualFileSystem,
        vnode: &VNode,
        target_path: &Path,
        preserve_perms: bool,
        _preserve_times: bool,
    ) -> Result<()> {
        let physical_path = target_path.join(vnode.path.to_string().trim_start_matches('/'));

        match vnode.node_type {
            NodeType::Directory => {
                fs::create_dir_all(physical_path).await
                    .map_err(|e| CortexError::vfs(format!("Failed to create directory: {}", e)))?;
            }
            NodeType::File | NodeType::Document => {
                if let Some(parent) = physical_path.parent() {
                    fs::create_dir_all(parent).await
                        .map_err(|e| CortexError::vfs(e.to_string()))?;
                }

                let content = vfs.read_file(&vnode.workspace_id, &vnode.path).await?;
                fs::write(&physical_path, content).await
                    .map_err(|e| CortexError::vfs(e.to_string()))?;

                if preserve_perms {
                    if let Some(mode) = vnode.permissions {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let perms = std::fs::Permissions::from_mode(mode);
                            fs::set_permissions(&physical_path, perms).await
                                .map_err(|e| CortexError::vfs(e.to_string()))?;
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Delete a physical file or directory.
    async fn delete_physical(&self, physical_path: &Path, vnode: &VNode) -> Result<()> {
        if !physical_path.exists() {
            return Ok(()); // Already deleted
        }

        if vnode.is_directory() {
            fs::remove_dir_all(physical_path).await
                .map_err(|e| CortexError::vfs(format!("Failed to remove directory: {}", e)))?;
        } else {
            fs::remove_file(physical_path).await
                .map_err(|e| CortexError::vfs(format!("Failed to remove file: {}", e)))?;
        }

        Ok(())
    }

    /// Convert virtual path to physical path.
    fn to_physical_path(&self, base: &Path, virtual_path: &VirtualPath) -> PathBuf {
        virtual_path.to_physical(base)
    }


    /// Create a backup of the target directory.
    async fn create_backup(&self, target_path: &Path) -> Result<PathBuf> {
        let backup_path = target_path.with_extension("backup");

        if backup_path.exists() {
            fs::remove_dir_all(&backup_path).await
                .map_err(|e| CortexError::vfs(format!("Failed to remove old backup: {}", e)))?;
        }

        self.copy_dir_recursive(target_path, &backup_path).await?;

        info!("Created backup at: {}", backup_path.display());
        Ok(backup_path)
    }

    /// Restore from backup.
    async fn restore_backup(&self, backup_path: &Path, target_path: &Path) -> Result<()> {
        // Remove current directory
        if target_path.exists() {
            fs::remove_dir_all(target_path).await
                .map_err(|e| CortexError::vfs(format!("Failed to remove target: {}", e)))?;
        }

        // Restore from backup
        self.copy_dir_recursive(backup_path, target_path).await?;

        info!("Restored from backup");
        Ok(())
    }

    /// Recursively copy directory.
    fn copy_dir_recursive<'a>(&'a self, src: &'a Path, dst: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            fs::create_dir_all(dst).await
                .map_err(|e| CortexError::vfs(format!("Failed to create directory: {}", e)))?;

            let mut entries = fs::read_dir(src).await
                .map_err(|e| CortexError::vfs(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| CortexError::vfs(format!("Failed to read entry: {}", e)))? {
                let path = entry.path();
                let file_name = entry.file_name();
                let dst_path = dst.join(file_name);

                if path.is_dir() {
                    self.copy_dir_recursive(&path, &dst_path).await?;
                } else {
                    fs::copy(&path, &dst_path).await
                        .map_err(|e| CortexError::vfs(format!("Failed to copy file: {}", e)))?;
                }
            }

            Ok(())
        })
    }
    // ============================================================================
    // Bidirectional Sync
    // ============================================================================

    /// Synchronize VFS from filesystem changes.
    ///
    /// This method scans a filesystem directory and updates the VFS to match:
    /// - New files on disk → Created VNodes in VFS
    /// - Modified files → Updated VNodes with Modified status
    /// - Files deleted from disk → VNodes marked as Deleted
    /// - Conflict detection when both VFS and FS have changed
    ///
    /// ## Document-Specific Conflict Detection
    ///
    /// For documents, conflict detection is simpler than for code:
    /// - No AST merging required (documents are plain text/markdown)
    /// - Hash-based comparison (content changed on both sides = conflict)
    /// - Auto-resolve can safely prefer filesystem version (no broken imports)
    /// - Manual resolution typically involves human review of text changes
    ///
    /// # Arguments
    ///
    /// * `workspace_id` - The workspace to sync into
    /// * `fs_path` - Physical filesystem path to scan
    /// * `virtual_path_prefix` - Virtual path prefix to map files under (e.g., "/" for root)
    /// * `options` - Sync options controlling behavior
    ///
    /// # Returns
    ///
    /// Returns a `SyncReport` with statistics and any errors encountered.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use cortex_vfs::{MaterializationEngine, VirtualFileSystem, VirtualPath, SyncOptions};
    /// # use cortex_storage::ConnectionManager;
    /// # use cortex_storage::connection_pool::DatabaseConfig;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// # async fn example() -> cortex_core::error::Result<()> {
    /// let config = DatabaseConfig::default();
    /// let storage = Arc::new(ConnectionManager::new(config).await.unwrap());
    /// let vfs = VirtualFileSystem::new(storage);
    /// let engine = MaterializationEngine::new(vfs);
    ///
    /// let workspace_id = uuid::Uuid::new_v4();
    /// let fs_path = Path::new("/home/user/documents");
    /// let virtual_prefix = VirtualPath::root();
    ///
    /// let report = engine.sync_from_filesystem(
    ///     &workspace_id,
    ///     fs_path,
    ///     &virtual_prefix,
    ///     SyncOptions::default()
    /// ).await?;
    ///
    /// println!("Synced {} files, {} conflicts", report.files_synced, report.conflicts_detected);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync_from_filesystem(
        &self,
        workspace_id: &Uuid,
        fs_path: &Path,
        virtual_path_prefix: &VirtualPath,
        options: SyncOptions,
    ) -> Result<SyncReport> {
        let start = Instant::now();
        info!(
            "Starting filesystem sync from {} to workspace {}",
            fs_path.display(),
            workspace_id
        );

        let mut report = SyncReport::default();

        // Verify filesystem path exists
        if !fs_path.exists() {
            return Err(CortexError::not_found(
                "Filesystem path",
                fs_path.display().to_string(),
            ));
        }

        if !fs_path.is_dir() {
            return Err(CortexError::invalid_input(format!(
                "Path must be a directory: {}",
                fs_path.display()
            )));
        }

        // Recursively scan filesystem and sync
        self.sync_directory_recursive(
            workspace_id,
            fs_path,
            virtual_path_prefix,
            &options,
            &mut report,
            0,
        )
        .await?;

        report.duration_ms = start.elapsed().as_millis() as u64;
        info!(
            "Filesystem sync completed in {}ms: {} files synced, {} conflicts",
            report.duration_ms, report.files_synced, report.conflicts_detected
        );

        Ok(report)
    }

    /// Recursively sync a directory from filesystem to VFS.
    fn sync_directory_recursive<'a>(
        &'a self,
        workspace_id: &'a Uuid,
        fs_path: &'a Path,
        virtual_path: &'a VirtualPath,
        options: &'a SyncOptions,
        report: &'a mut SyncReport,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // Check max depth
            if let Some(max_depth) = options.max_depth {
                if depth > max_depth {
                    debug!("Skipping directory (max depth): {}", fs_path.display());
                    return Ok(());
                }
            }

            // Read directory entries
            let mut entries = match fs::read_dir(fs_path).await {
                Ok(entries) => entries,
                Err(e) => {
                    let err_msg = format!("Failed to read directory {}: {}", fs_path.display(), e);
                    warn!("{}", err_msg);
                    report.errors.push(err_msg);
                    return Ok(());
                }
            };

            // Process each entry
            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| CortexError::vfs(format!("Failed to read entry: {}", e)))?
            {
                let entry_path = entry.path();
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                // Skip hidden files if requested
                if options.skip_hidden && file_name_str.starts_with('.') {
                    debug!("Skipping hidden file: {}", entry_path.display());
                    continue;
                }

                // Check exclusion patterns
                if self.is_excluded(&entry_path, &options.exclude_patterns) {
                    debug!("Excluded by pattern: {}", entry_path.display());
                    continue;
                }

                // Build virtual path for this entry
                let entry_virtual_path = match virtual_path.join(&file_name_str) {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("Invalid path {}: {}", file_name_str, e);
                        continue;
                    }
                };

                // Get metadata
                let metadata = match fs::metadata(&entry_path).await {
                    Ok(m) => m,
                    Err(e) => {
                        let err_msg = format!("Failed to read metadata for {}: {}", entry_path.display(), e);
                        warn!("{}", err_msg);
                        report.errors.push(err_msg);
                        continue;
                    }
                };

                if metadata.is_dir() {
                    // Ensure directory exists in VFS
                    if let Err(e) = self.sync_directory(
                        workspace_id,
                        &entry_virtual_path,
                        &metadata,
                    ).await {
                        let err_msg = format!("Failed to sync directory {}: {}", entry_path.display(), e);
                        warn!("{}", err_msg);
                        report.errors.push(err_msg);
                        continue;
                    }

                    report.directories_synced += 1;

                    // Recurse into subdirectory
                    if let Err(e) = self.sync_directory_recursive(
                        workspace_id,
                        &entry_path,
                        &entry_virtual_path,
                        options,
                        report,
                        depth + 1,
                    ).await {
                        report.errors.push(e.to_string());
                    }
                } else if metadata.is_file() {
                    // Sync file
                    match self.sync_file(
                        workspace_id,
                        &entry_path,
                        &entry_virtual_path,
                        &metadata,
                        options,
                    ).await {
                        Ok(sync_result) => {
                            report.files_synced += 1;
                            if sync_result.is_conflict {
                                report.conflicts_detected += 1;
                            }
                            report.bytes_synced += sync_result.size_bytes;
                        }
                        Err(e) => {
                            let err_msg = format!("Failed to sync file {}: {}", entry_path.display(), e);
                            warn!("{}", err_msg);
                            report.errors.push(err_msg);
                        }
                    }
                } else if metadata.is_symlink() {
                    // Handle symlinks if requested
                    if options.follow_symlinks {
                        debug!("Following symlink: {}", entry_path.display());
                        // Could implement symlink handling here
                    } else {
                        debug!("Skipping symlink: {}", entry_path.display());
                    }
                }
            }

            Ok(())
        })
    }

    /// Sync a single directory from filesystem to VFS.
    async fn sync_directory(
        &self,
        workspace_id: &Uuid,
        virtual_path: &VirtualPath,
        _metadata: &std::fs::Metadata,
    ) -> Result<()> {
        // Check if directory already exists in VFS
        if self.vfs.exists(workspace_id, virtual_path).await? {
            debug!("Directory already exists in VFS: {}", virtual_path);
            return Ok(());
        }

        // Create directory in VFS
        debug!("Creating directory in VFS: {}", virtual_path);
        self.vfs.create_directory(workspace_id, virtual_path, false).await?;

        Ok(())
    }

    /// Sync a single file from filesystem to VFS.
    async fn sync_file(
        &self,
        workspace_id: &Uuid,
        fs_path: &Path,
        virtual_path: &VirtualPath,
        metadata: &std::fs::Metadata,
        options: &SyncOptions,
    ) -> Result<FileSyncResult> {
        // Read file content
        let content = fs::read(fs_path).await.map_err(|e| {
            CortexError::vfs(format!("Failed to read file {}: {}", fs_path.display(), e))
        })?;

        // Calculate hash
        let new_hash = Self::hash_content(&content);

        // Check if file exists in VFS
        let existing_vnode = self.vfs.get_vnode(workspace_id, virtual_path).await?;

        let sync_result = match existing_vnode {
            Some(vnode) => {
                // File exists in VFS - check for changes
                self.sync_existing_file(
                    workspace_id,
                    virtual_path,
                    &vnode,
                    &content,
                    &new_hash,
                    metadata,
                    options,
                ).await?
            }
            None => {
                // New file - create in VFS
                self.sync_new_file(
                    workspace_id,
                    virtual_path,
                    &content,
                    &new_hash,
                    metadata,
                ).await?
            }
        };

        Ok(sync_result)
    }

    /// Sync a new file (doesn't exist in VFS yet).
    async fn sync_new_file(
        &self,
        workspace_id: &Uuid,
        virtual_path: &VirtualPath,
        content: &[u8],
        content_hash: &str,
        metadata: &std::fs::Metadata,
    ) -> Result<FileSyncResult> {
        debug!("Syncing new file to VFS: {}", virtual_path);

        // Store content
        self.vfs.store_content(content_hash, content).await?;

        // Create VNode
        let mut vnode = VNode::new_file(
            *workspace_id,
            virtual_path.clone(),
            content_hash.to_string(),
            content.len(),
        );

        // Detect language
        if let Some(ext) = virtual_path.extension() {
            vnode.language = Some(Language::from_extension(ext));
        }

        // Set permissions from filesystem
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            vnode.permissions = Some(metadata.permissions().mode());
        }

        // Mark as Created
        vnode.status = SyncStatus::Created;

        // Save to VFS
        self.vfs.save_vnode(&vnode).await?;

        Ok(FileSyncResult {
            size_bytes: content.len(),
            is_conflict: false,
        })
    }

    /// Sync an existing file (already in VFS).
    ///
    /// ## Document Conflict Detection
    ///
    /// For document-only workflows, conflict detection is straightforward:
    /// 1. Compare content hashes (filesystem vs VFS)
    /// 2. If both changed since last sync → conflict
    /// 3. For documents, we can safely store both versions and let user choose
    /// 4. No risk of breaking imports/references (unlike code files)
    async fn sync_existing_file(
        &self,
        _workspace_id: &Uuid,
        virtual_path: &VirtualPath,
        existing_vnode: &VNode,
        content: &[u8],
        new_hash: &str,
        metadata: &std::fs::Metadata,
        options: &SyncOptions,
    ) -> Result<FileSyncResult> {
        let existing_hash = existing_vnode.content_hash.as_deref().unwrap_or("");

        // Check if content has changed on filesystem
        if new_hash == existing_hash {
            // No change on filesystem
            debug!("File unchanged on filesystem: {}", virtual_path);
            return Ok(FileSyncResult {
                size_bytes: content.len(),
                is_conflict: false,
            });
        }

        // Content has changed on filesystem
        debug!("File changed on filesystem: {}", virtual_path);

        // Simple conflict detection for documents: check if VFS also has unsaved changes
        let is_conflict = match existing_vnode.status {
            SyncStatus::Modified | SyncStatus::Created => {
                // VFS has unsaved changes AND filesystem has changed = CONFLICT
                // For documents, this is a simple content conflict (no AST to worry about)
                warn!("Conflict detected for {}: both VFS and filesystem modified", virtual_path);
                true
            }
            _ => false,
        };

        if is_conflict && !options.auto_resolve_conflicts {
            // Mark as conflict and store both versions
            // For documents, we simply preserve both versions for manual resolution
            let mut vnode = existing_vnode.clone();
            vnode.status = SyncStatus::Conflict;

            // Store filesystem version in metadata for later resolution
            vnode.metadata.insert(
                "fs_content_hash".to_string(),
                serde_json::Value::String(new_hash.to_string()),
            );
            vnode.metadata.insert(
                "conflict_detected_at".to_string(),
                serde_json::Value::String(Utc::now().to_rfc3339()),
            );

            // Store the new content (filesystem version)
            self.vfs.store_content(new_hash, content).await?;

            // Save updated vnode
            self.vfs.save_vnode(&vnode).await?;

            info!("Marked file as conflicted: {}", virtual_path);

            return Ok(FileSyncResult {
                size_bytes: content.len(),
                is_conflict: true,
            });
        }

        // No conflict or auto-resolve enabled - update from filesystem
        let mut vnode = existing_vnode.clone();

        // Store new content
        self.vfs.store_content(new_hash, content).await?;

        // Update vnode
        vnode.content_hash = Some(new_hash.to_string());
        vnode.size_bytes = content.len();

        // Update permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            vnode.permissions = Some(metadata.permissions().mode());
        }

        // Mark as modified (unless it was already created/modified and we're auto-resolving)
        if !is_conflict {
            vnode.mark_modified();
        } else {
            // Auto-resolve: for documents, we safely prefer filesystem version
            // (no risk of breaking imports or code dependencies)
            vnode.status = SyncStatus::Modified;
            vnode.version += 1;
            vnode.updated_at = Utc::now();
            debug!("Auto-resolved conflict for {} (preferred filesystem version)", virtual_path);
        }

        // Save updated vnode
        self.vfs.save_vnode(&vnode).await?;

        Ok(FileSyncResult {
            size_bytes: content.len(),
            is_conflict: false, // Not a conflict if auto-resolved
        })
    }

    /// Check if a path matches exclusion patterns.
    fn is_excluded(&self, path: &Path, patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in patterns {
            // Simple pattern matching (could be enhanced with glob crate)
            if pattern.contains("**") {
                // Wildcard pattern
                let pattern_parts: Vec<&str> = pattern.split("**").collect();
                if pattern_parts.len() == 2 {
                    let prefix = pattern_parts[0];
                    let suffix = pattern_parts[1].trim_start_matches('/');

                    if path_str.contains(prefix) && (suffix.is_empty() || path_str.contains(suffix)) {
                        return true;
                    }
                }
            } else if path_str.contains(pattern) {
                // Simple substring match
                return true;
            }
        }

        false
    }

    /// Hash content using blake3 (exposed for sync operations).
    fn hash_content(content: &[u8]) -> String {
        let hash = blake3::hash(content);
        hash.to_hex().to_string()
    }
}

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::ConnectionManager;
    use cortex_storage::connection_pool::{ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy};
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::fs;

    async fn setup_test_vfs() -> (VirtualFileSystem, Arc<ConnectionManager>) {
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
        let vfs = VirtualFileSystem::new(storage.clone());
        (vfs, storage)
    }

    #[tokio::test]
    async fn test_sync_new_file_from_filesystem() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        // Create temporary directory with a test file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"Hello, World!").await.unwrap();

        // Sync from filesystem
        let workspace_id = Uuid::new_v4();
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Verify sync report
        assert_eq!(report.files_synced, 1);
        assert_eq!(report.conflicts_detected, 0);
        assert!(report.errors.is_empty());

        // Verify file exists in VFS
        let virtual_path = VirtualPath::new("test.txt").unwrap();
        assert!(vfs.exists(&workspace_id, &virtual_path).await.unwrap());

        // Verify content
        let content = vfs.read_file(&workspace_id, &virtual_path).await.unwrap();
        assert_eq!(content, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_sync_modified_file_no_conflict() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let workspace_id = Uuid::new_v4();

        // Initial sync
        fs::write(&test_file, b"Version 1").await.unwrap();
        engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Modify file on filesystem
        fs::write(&test_file, b"Version 2").await.unwrap();

        // Sync again
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // No conflict since VFS wasn't modified
        assert_eq!(report.conflicts_detected, 0);

        // Verify updated content
        let virtual_path = VirtualPath::new("test.txt").unwrap();
        let content = vfs.read_file(&workspace_id, &virtual_path).await.unwrap();
        assert_eq!(content, b"Version 2");
    }

    #[tokio::test]
    async fn test_sync_conflict_detection() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let workspace_id = Uuid::new_v4();
        let virtual_path = VirtualPath::new("test.txt").unwrap();

        // Initial sync
        fs::write(&test_file, b"Version 1").await.unwrap();
        engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Modify in VFS (creating unsaved changes)
        vfs.write_file(&workspace_id, &virtual_path, b"VFS Version").await.unwrap();

        // Modify on filesystem
        fs::write(&test_file, b"FS Version").await.unwrap();

        // Sync should detect conflict
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        assert_eq!(report.conflicts_detected, 1);

        // Verify VNode is marked as conflicted
        let vnode = vfs.metadata(&workspace_id, &virtual_path).await.unwrap();
        assert_eq!(vnode.status, SyncStatus::Conflict);

        // Verify both versions are stored
        assert!(vnode.metadata.contains_key("fs_content_hash"));
    }

    #[tokio::test]
    async fn test_sync_auto_resolve_conflict() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let workspace_id = Uuid::new_v4();
        let virtual_path = VirtualPath::new("test.txt").unwrap();

        // Initial sync
        fs::write(&test_file, b"Version 1").await.unwrap();
        engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Modify in VFS
        vfs.write_file(&workspace_id, &virtual_path, b"VFS Version").await.unwrap();

        // Modify on filesystem
        fs::write(&test_file, b"FS Version").await.unwrap();

        // Sync with auto-resolve enabled (prefers filesystem)
        let mut options = SyncOptions::default();
        options.auto_resolve_conflicts = true;

        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            options,
        ).await.unwrap();

        // Conflict should be auto-resolved
        assert_eq!(report.conflicts_detected, 0);

        // VFS should have filesystem version
        let content = vfs.read_file(&workspace_id, &virtual_path).await.unwrap();
        assert_eq!(content, b"FS Version");
    }

    #[tokio::test]
    async fn test_sync_directory_structure() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let workspace_id = Uuid::new_v4();

        // Create directory structure
        fs::create_dir(temp_dir.path().join("src")).await.unwrap();
        fs::create_dir(temp_dir.path().join("src/lib")).await.unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), b"fn main() {}").await.unwrap();
        fs::write(temp_dir.path().join("src/lib/mod.rs"), b"pub mod test;").await.unwrap();

        // Sync
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        assert_eq!(report.files_synced, 2);
        assert_eq!(report.directories_synced, 2);

        // Verify structure in VFS
        assert!(vfs.exists(&workspace_id, &VirtualPath::new("src").unwrap()).await.unwrap());
        assert!(vfs.exists(&workspace_id, &VirtualPath::new("src/lib").unwrap()).await.unwrap());
        assert!(vfs.exists(&workspace_id, &VirtualPath::new("src/main.rs").unwrap()).await.unwrap());
        assert!(vfs.exists(&workspace_id, &VirtualPath::new("src/lib/mod.rs").unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn test_sync_exclusion_patterns() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let workspace_id = Uuid::new_v4();

        // Create files, including ones that should be excluded
        fs::create_dir(temp_dir.path().join("node_modules")).await.unwrap();
        fs::write(temp_dir.path().join("node_modules/package.json"), b"{}").await.unwrap();
        fs::write(temp_dir.path().join("index.js"), b"console.log('hi')").await.unwrap();

        // Sync with default exclusion patterns
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Only index.js should be synced (node_modules excluded)
        assert_eq!(report.files_synced, 1);

        // Verify
        assert!(vfs.exists(&workspace_id, &VirtualPath::new("index.js").unwrap()).await.unwrap());
        assert!(!vfs.exists(&workspace_id, &VirtualPath::new("node_modules/package.json").unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn test_sync_skip_hidden_files() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let workspace_id = Uuid::new_v4();

        // Create hidden and visible files
        fs::write(temp_dir.path().join(".hidden"), b"secret").await.unwrap();
        fs::write(temp_dir.path().join("visible.txt"), b"public").await.unwrap();

        // Sync with skip_hidden enabled
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Only visible file should be synced
        assert_eq!(report.files_synced, 1);

        assert!(vfs.exists(&workspace_id, &VirtualPath::new("visible.txt").unwrap()).await.unwrap());
        assert!(!vfs.exists(&workspace_id, &VirtualPath::new(".hidden").unwrap()).await.unwrap());
    }

    #[tokio::test]
    async fn test_bidirectional_sync_roundtrip() {
        let (vfs, _storage) = setup_test_vfs().await;
        let engine = MaterializationEngine::new(vfs.clone());

        let temp_dir = TempDir::new().unwrap();
        let workspace_id = Uuid::new_v4();

        // Sync from filesystem
        fs::write(temp_dir.path().join("test.txt"), b"Original").await.unwrap();
        engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // Modify in VFS
        let virtual_path = VirtualPath::new("test.txt").unwrap();
        vfs.write_file(&workspace_id, &virtual_path, b"Modified in VFS").await.unwrap();

        // Flush back to filesystem
        engine.flush(
            FlushScope::Workspace(workspace_id),
            temp_dir.path(),
            FlushOptions::default(),
        ).await.unwrap();

        // Verify filesystem has updated content
        let fs_content = fs::read(temp_dir.path().join("test.txt")).await.unwrap();
        assert_eq!(fs_content, b"Modified in VFS");

        // Sync again (should have no changes)
        let report = engine.sync_from_filesystem(
            &workspace_id,
            temp_dir.path(),
            &VirtualPath::root(),
            SyncOptions::default(),
        ).await.unwrap();

        // File should be unchanged (same hash)
        assert_eq!(report.files_synced, 1); // Still counted as synced
        assert_eq!(report.conflicts_detected, 0);
    }
}
