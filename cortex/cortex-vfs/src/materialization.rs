//! Materialization engine for flushing VFS content to physical filesystem.

use crate::path::VirtualPath;
use crate::types::*;
use crate::virtual_filesystem::VirtualFileSystem;
use cortex_core::error::{CortexError, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

/// Engine for materializing virtual filesystem to physical disk.
///
/// Supports:
/// - Target path specification (materialize anywhere)
/// - Atomic operations with rollback
/// - Parallel materialization for performance
/// - Change tracking and incremental sync
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
        if options.parallel && creates_and_updates.len() > 1 {
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
}

use std::sync::Arc;
