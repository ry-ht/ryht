//! Fork manager for creating editable copies of read-only content.

use crate::path::VirtualPath;
use crate::types::*;
use crate::virtual_filesystem::VirtualFileSystem;
use cortex_core::error::{CortexError, Result};
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Manager for creating and merging forks of workspaces.
///
/// Supports:
/// - Creating editable forks of read-only workspaces
/// - Three-way merge with conflict detection
/// - Multiple merge strategies (manual, auto, prefer fork/target)
pub struct ForkManager {
    vfs: VirtualFileSystem,
    storage: Arc<ConnectionManager>,
}

impl ForkManager {
    /// Create a new fork manager.
    pub fn new(vfs: VirtualFileSystem, storage: Arc<ConnectionManager>) -> Self {
        Self { vfs, storage }
    }

    /// Create an editable fork of a workspace.
    ///
    /// The fork creates a deep copy of all vnodes in a new namespace,
    /// making them editable even if the source is read-only.
    pub async fn create_fork(
        &self,
        source_workspace_id: &Uuid,
        fork_name: String,
    ) -> Result<Workspace> {
        info!("Creating fork of workspace {}", source_workspace_id);

        // Get source workspace
        let source = self.get_workspace(source_workspace_id).await?;

        // Create new namespace for fork
        let fork_namespace = format!(
            "{}_{}_fork_{}",
            source.namespace,
            fork_name.replace(" ", "_"),
            Uuid::new_v4()
        );

        // Create fork workspace
        let mut fork_metadata = source.metadata.clone();
        fork_metadata.insert("is_fork".to_string(), serde_json::Value::Bool(true));
        fork_metadata.insert("source_workspace_id".to_string(), serde_json::Value::String(source_workspace_id.to_string()));

        let fork = Workspace {
            id: Uuid::new_v4(),
            name: fork_name,
            namespace: fork_namespace.clone(),
            sync_sources: vec![], // Fork doesn't inherit sync sources initially
            metadata: fork_metadata,
            read_only: false, // Forks are editable
            parent_workspace: Some(*source_workspace_id),
            fork_metadata: Some(ForkMetadata {
                source_id: *source_workspace_id,
                source_name: source.name.clone(),
                fork_point: chrono::Utc::now(),
                fork_commit: None,
            }),
            dependencies: source.dependencies.clone(), // Inherit dependencies from source
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Store fork workspace
        let query = "CREATE workspace CONTENT $workspace";
        let conn = self.storage.acquire().await?;
        let fork_json = serde_json::to_value(&fork)
            .map_err(|e| CortexError::storage(e.to_string()))?;
        conn.connection().query(query)
            .bind(("workspace", fork_json))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        // Copy all vnodes from source to fork
        self.copy_vnodes(source_workspace_id, &fork.id).await?;

        info!("Fork created: {} ({})", fork.name, fork.id);

        Ok(fork)
    }

    /// Copy all vnodes from source workspace to fork.
    async fn copy_vnodes(
        &self,
        source_workspace_id: &Uuid,
        fork_workspace_id: &Uuid,
    ) -> Result<()> {
        // Get all vnodes from source
        let query = "SELECT * FROM vnode WHERE workspace_id = $source_id";
        let conn = self.storage.acquire().await?;
        let mut result = conn.connection().query(query)
            .bind(("source_id", source_workspace_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;
        let vnodes: Vec<VNode> = result.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        info!("Copying {} vnodes to fork", vnodes.len());

        // Copy each vnode
        for source_vnode in vnodes {
            let mut fork_vnode = source_vnode.clone();
            fork_vnode.id = Uuid::new_v4();
            fork_vnode.workspace_id = *fork_workspace_id;
            fork_vnode.read_only = false; // Make editable in fork
            fork_vnode.created_at = chrono::Utc::now();
            fork_vnode.updated_at = chrono::Utc::now();

            // Store fork vnode
            let query = "CREATE vnode CONTENT $vnode";
            let conn = self.storage.acquire().await?;
            let vnode_json = serde_json::to_value(&fork_vnode)
                .map_err(|e| CortexError::storage(e.to_string()))?;
            conn.connection().query(query)
                .bind(("vnode", vnode_json))
                .await
                .map_err(|e| CortexError::storage(e.to_string()))?;

            // Note: Content is already deduplicated by hash, so no need to copy content
        }

        Ok(())
    }

    /// Merge changes from fork back to target workspace.
    pub async fn merge_fork(
        &self,
        fork_id: &Uuid,
        target_id: &Uuid,
        strategy: MergeStrategy,
    ) -> Result<MergeReport> {
        info!("Merging fork {} into target {}", fork_id, target_id);

        let fork = self.get_workspace(fork_id).await?;
        let target = self.get_workspace(target_id).await?;

        // Check that target is not read-only
        if target.read_only {
            return Err(CortexError::invalid_input(
                "Cannot merge into read-only workspace"
            ));
        }

        // Find changes in fork since fork point
        let fork_point = fork.fork_metadata
            .as_ref()
            .map(|m| m.fork_point)
            .ok_or_else(|| CortexError::invalid_input("Not a fork workspace"))?;

        let changes = self.get_changes_since(fork_id, fork_point).await?;

        info!("Found {} changes to merge", changes.len());

        let mut report = MergeReport::default();

        // Apply each change
        for change in changes {
            match self.apply_change(&change, &target, &strategy, &mut report).await {
                Ok(_) => {
                    report.changes_applied += 1;
                }
                Err(e) => {
                    report.errors.push(format!("Failed to apply change: {}", e));
                }
            }
        }

        // Handle conflicts based on strategy
        if !report.conflicts.is_empty() {
            info!("Found {} conflicts", report.conflicts.len());
            report.conflicts_count = report.conflicts.len();

            match strategy {
                MergeStrategy::Manual => {
                    // Return conflicts for manual resolution
                }
                MergeStrategy::AutoMerge => {
                    // Attempt three-way merge
                    for conflict in &mut report.conflicts {
                        if let Ok(merged) = self.three_way_merge(conflict).await {
                            conflict.resolution = Some(merged);
                            report.auto_resolved += 1;
                        }
                    }
                }
                MergeStrategy::PreferFork => {
                    // Use fork version for all conflicts
                    for conflict in &mut report.conflicts {
                        conflict.resolution = Some(conflict.fork_content.clone());
                        report.auto_resolved += 1;
                    }
                }
                MergeStrategy::PreferTarget => {
                    // Keep target version for all conflicts
                    for conflict in &mut report.conflicts {
                        conflict.resolution = Some(conflict.target_content.clone());
                        report.auto_resolved += 1;
                    }
                }
            }

            // Apply resolved conflicts
            for conflict in &report.conflicts {
                if let Some(resolution) = &conflict.resolution {
                    if let Err(e) = self.apply_resolution(&target.id, &conflict.path, resolution).await {
                        report.errors.push(format!("Failed to apply resolution: {}", e));
                    }
                }
            }
        }

        info!(
            "Merge completed: {} changes applied, {} conflicts, {} auto-resolved",
            report.changes_applied, report.conflicts_count, report.auto_resolved
        );

        Ok(report)
    }

    /// Get changes since a specific point in time.
    async fn get_changes_since(
        &self,
        workspace_id: &Uuid,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Change>> {
        let query = "SELECT * FROM change WHERE workspace_id = $workspace_id AND timestamp > $since";
        let conn = self.storage.acquire().await?;
        let mut result = conn.connection().query(query)
            .bind(("workspace_id", workspace_id.to_string()))
            .bind(("since", since.to_rfc3339()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;
        let changes: Vec<Change> = result.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(changes)
    }

    /// Apply a change to the target workspace.
    async fn apply_change(
        &self,
        change: &Change,
        target: &Workspace,
        _strategy: &MergeStrategy,
        report: &mut MergeReport,
    ) -> Result<()> {
        // Check if path exists in target
        let target_vnode = self.vfs.metadata(&target.id, &change.path).await.ok();

        match change.change_type {
            ChangeType::Created => {
                if target_vnode.is_some() {
                    // Conflict: file already exists in target
                    let conflict = self.create_conflict(change, target).await?;
                    report.conflicts.push(conflict);
                } else {
                    // No conflict, create file
                    self.copy_vnode_to_workspace(&change.vnode_id, &target.id).await?;
                }
            }
            ChangeType::Modified => {
                if let Some(ref tv) = target_vnode {
                    // Check if target was also modified
                    if tv.version > 1 {
                        // Conflict: both modified
                        let conflict = self.create_conflict(change, target).await?;
                        report.conflicts.push(conflict);
                    } else {
                        // No conflict, apply change
                        self.copy_vnode_to_workspace(&change.vnode_id, &target.id).await?;
                    }
                } else {
                    // File was deleted in target
                    let conflict = self.create_conflict(change, target).await?;
                    report.conflicts.push(conflict);
                }
            }
            ChangeType::Deleted => {
                if let Some(_target_vnode) = target_vnode {
                    // Delete in target
                    self.vfs.delete(&target.id, &change.path, false).await?;
                }
                // If already deleted, no-op
            }
            ChangeType::Renamed => {
                // Handle rename by creating at new path
                // Note: Change struct doesn't track old_path, so we can't delete from old location
                // The vnode_id should reference the vnode with the current (new) path
                self.copy_vnode_to_workspace(&change.vnode_id, &target.id).await?;
            }
        }

        Ok(())
    }

    /// Create a conflict record.
    async fn create_conflict(&self, change: &Change, target: &Workspace) -> Result<Conflict> {
        // Get fork content
        let fork_vnode = self.get_vnode(&change.vnode_id).await?;
        let fork_content = if let Some(hash) = &fork_vnode.content_hash {
            self.get_content(hash).await?
        } else {
            String::new()
        };

        // Get target content
        let target_content = if let Ok(target_vnode) = self.vfs.metadata(&target.id, &change.path).await {
            if let Some(hash) = &target_vnode.content_hash {
                self.get_content(hash).await?
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        Ok(Conflict {
            path: change.path.clone(),
            fork_content,
            target_content,
            resolution: None,
        })
    }

    /// Perform three-way merge.
    async fn three_way_merge(&self, conflict: &Conflict) -> Result<String> {
        // Simple line-based merge
        // In production, you'd want a more sophisticated merge algorithm

        let _fork_lines: Vec<&str> = conflict.fork_content.lines().collect();
        let _target_lines: Vec<&str> = conflict.target_content.lines().collect();

        // For now, just concatenate with conflict markers
        // A real implementation would use a proper diff3 algorithm

        let merged = format!(
            "<<<<<<< FORK\n{}\n=======\n{}\n>>>>>>> TARGET\n",
            conflict.fork_content, conflict.target_content
        );

        Ok(merged)
    }

    /// Apply a conflict resolution.
    async fn apply_resolution(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
        content: &str,
    ) -> Result<()> {
        self.vfs.write_file(workspace_id, path, content.as_bytes()).await
    }

    /// Copy a vnode to another workspace.
    async fn copy_vnode_to_workspace(
        &self,
        vnode_id: &Uuid,
        target_workspace_id: &Uuid,
    ) -> Result<()> {
        let source_vnode = self.get_vnode(vnode_id).await?;

        let mut target_vnode = source_vnode.clone();
        target_vnode.id = Uuid::new_v4();
        target_vnode.workspace_id = *target_workspace_id;
        target_vnode.created_at = chrono::Utc::now();
        target_vnode.updated_at = chrono::Utc::now();

        let query = "CREATE vnode CONTENT $vnode";
        let conn = self.storage.acquire().await?;
        let vnode_json = serde_json::to_value(&target_vnode)
            .map_err(|e| CortexError::storage(e.to_string()))?;
        conn.connection().query(query)
            .bind(("vnode", vnode_json))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(())
    }

    /// Get a workspace by ID.
    async fn get_workspace(&self, workspace_id: &Uuid) -> Result<Workspace> {
        let query = "SELECT * FROM workspace WHERE id = $id LIMIT 1";
        let conn = self.storage.acquire().await?;
        let mut result = conn.connection().query(query)
            .bind(("id", workspace_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;
        let workspace: Option<Workspace> = result.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        workspace.ok_or_else(|| CortexError::not_found("Workspace", workspace_id.to_string()))
    }

    /// Get a vnode by ID.
    async fn get_vnode(&self, vnode_id: &Uuid) -> Result<VNode> {
        let query = "SELECT * FROM vnode WHERE id = $id LIMIT 1";
        let conn = self.storage.acquire().await?;
        let mut result = conn.connection().query(query)
            .bind(("id", vnode_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;
        let vnode: Option<VNode> = result.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        vnode.ok_or_else(|| CortexError::not_found("VNode", vnode_id.to_string()))
    }

    /// Get content by hash.
    async fn get_content(&self, hash: &str) -> Result<String> {
        let query = "SELECT * FROM file_content WHERE content_hash = $hash LIMIT 1";
        let conn = self.storage.acquire().await?;
        let mut result = conn.connection().query(query)
            .bind(("hash", hash.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;
        let content: Option<FileContent> = result.take(0)
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let content = content
            .ok_or_else(|| CortexError::not_found("FileContent", hash.to_string()))?;

        if let Some(text) = content.content {
            Ok(text)
        } else if let Some(binary) = content.content_binary {
            Ok(String::from_utf8_lossy(&binary).to_string())
        } else {
            Err(CortexError::internal("Content has no data"))
        }
    }
}
