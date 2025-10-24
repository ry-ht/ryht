//! Workspace service layer
//!
//! Provides unified workspace management operations for both API and MCP modules.

use anyhow::Result;
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use cortex_vfs::{VirtualFileSystem, Workspace, WorkspaceType, SourceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Workspace service for managing workspaces
#[derive(Clone)]
pub struct WorkspaceService {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
}

impl WorkspaceService {
    /// Create a new workspace service
    pub fn new(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Self {
        Self { storage, vfs }
    }

    /// Create a new workspace
    pub async fn create_workspace(&self, request: CreateWorkspaceRequest) -> Result<WorkspaceDetails> {
        info!("Creating workspace: {}", request.name);

        // Parse workspace type
        let workspace_type = match request.workspace_type.to_lowercase().as_str() {
            "code" => WorkspaceType::Code,
            "documentation" => WorkspaceType::Documentation,
            "mixed" => WorkspaceType::Mixed,
            "external" => WorkspaceType::External,
            _ => anyhow::bail!("Invalid workspace type: {}", request.workspace_type),
        };

        // Create workspace entity
        let workspace_id = Uuid::new_v4();
        let namespace = format!("ws_{}", workspace_id.to_string().replace('-', "_"));
        let now = Utc::now();

        let workspace = Workspace {
            id: workspace_id,
            name: request.name.clone(),
            workspace_type,
            source_type: SourceType::Local,
            namespace: namespace.clone(),
            source_path: request.source_path.map(PathBuf::from),
            read_only: request.read_only.unwrap_or(false),
            parent_workspace: None,
            fork_metadata: None,
            created_at: now,
            updated_at: now,
        };

        // Save to database
        let conn = self.storage.acquire().await?;
        let workspace_json = serde_json::to_value(&workspace)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("workspace", workspace_id.to_string()))
            .content(workspace_json)
            .await?;

        info!("Created workspace: {} ({})", workspace.name, workspace_id);

        Ok(WorkspaceDetails::from_workspace(workspace))
    }

    /// Get workspace by ID
    pub async fn get_workspace(&self, workspace_id: &Uuid) -> Result<Option<WorkspaceDetails>> {
        debug!("Getting workspace: {}", workspace_id);

        let conn = self.storage.acquire().await?;

        let workspace: Option<Workspace> = conn
            .connection()
            .select(("workspace", workspace_id.to_string()))
            .await?;

        Ok(workspace.map(WorkspaceDetails::from_workspace))
    }

    /// List all workspaces
    pub async fn list_workspaces(&self, filters: ListWorkspaceFilters) -> Result<Vec<WorkspaceDetails>> {
        debug!("Listing workspaces with filters: {:?}", filters);

        let conn = self.storage.acquire().await?;

        let mut query = String::from("SELECT * FROM workspace WHERE 1=1");

        if let Some(ref workspace_type) = filters.workspace_type {
            query.push_str(&format!(" AND workspace_type = '{}'", workspace_type));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut response = conn.connection().query(&query).await?;
        let workspaces: Vec<Workspace> = response.take(0)?;

        Ok(workspaces
            .into_iter()
            .map(WorkspaceDetails::from_workspace)
            .collect())
    }

    /// Update workspace
    pub async fn update_workspace(&self, workspace_id: &Uuid, request: UpdateWorkspaceRequest) -> Result<WorkspaceDetails> {
        debug!("Updating workspace: {}", workspace_id);

        let conn = self.storage.acquire().await?;

        // Get existing workspace
        let workspace: Option<Workspace> = conn
            .connection()
            .select(("workspace", workspace_id.to_string()))
            .await?;

        let mut workspace = workspace
            .ok_or_else(|| anyhow::anyhow!("Workspace {} not found", workspace_id))?;

        // Update fields
        if let Some(name) = request.name {
            workspace.name = name;
        }

        if let Some(workspace_type_str) = request.workspace_type {
            workspace.workspace_type = match workspace_type_str.to_lowercase().as_str() {
                "code" => WorkspaceType::Code,
                "documentation" => WorkspaceType::Documentation,
                "mixed" => WorkspaceType::Mixed,
                "external" => WorkspaceType::External,
                _ => anyhow::bail!("Invalid workspace type: {}", workspace_type_str),
            };
        }

        if let Some(read_only) = request.read_only {
            workspace.read_only = read_only;
        }

        workspace.updated_at = Utc::now();

        // Save to database
        let workspace_json = serde_json::to_value(&workspace)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .update(("workspace", workspace_id.to_string()))
            .content(workspace_json)
            .await?;

        info!("Updated workspace: {}", workspace_id);

        Ok(WorkspaceDetails::from_workspace(workspace))
    }

    /// Delete workspace and all associated data
    pub async fn delete_workspace(&self, workspace_id: &Uuid) -> Result<()> {
        info!("Deleting workspace: {}", workspace_id);

        let conn = self.storage.acquire().await?;

        // Delete all vnodes in workspace
        let _: Vec<serde_json::Value> = conn
            .connection()
            .query("DELETE vnode WHERE workspace_id = $workspace_id")
            .bind(("workspace_id", workspace_id.to_string()))
            .await?
            .take(0)?;

        // Delete workspace
        let _: Option<Workspace> = conn
            .connection()
            .delete(("workspace", workspace_id.to_string()))
            .await?;

        info!("Deleted workspace: {}", workspace_id);

        Ok(())
    }

    /// Detect project type from path
    pub fn detect_project_type(path: &std::path::Path) -> WorkspaceType {
        // Check for various project indicators
        if path.join("Cargo.toml").exists() {
            return WorkspaceType::Code;
        }
        if path.join("package.json").exists() {
            return WorkspaceType::Code;
        }
        if path.join("pom.xml").exists() || path.join("build.gradle").exists() {
            return WorkspaceType::Code;
        }
        if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
            return WorkspaceType::Code;
        }
        if path.join("go.mod").exists() {
            return WorkspaceType::Code;
        }

        // Check for documentation-heavy projects
        if path.join("README.md").exists() && path.join("docs").is_dir() {
            // Count source files vs markdown files
            let mut src_count = 0;
            let mut doc_count = 0;

            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        match ext.to_str() {
                            Some("rs") | Some("js") | Some("ts") | Some("py") | Some("java") | Some("go") => {
                                src_count += 1;
                            }
                            Some("md") | Some("rst") | Some("txt") => {
                                doc_count += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if doc_count > src_count * 2 {
                return WorkspaceType::Documentation;
            }
        }

        // Default to mixed if we can't determine
        WorkspaceType::Mixed
    }

    /// Get workspace statistics
    pub async fn get_workspace_stats(&self, workspace_id: &Uuid) -> Result<WorkspaceStats> {
        debug!("Calculating stats for workspace: {}", workspace_id);

        let conn = self.storage.acquire().await?;

        // Count files
        let mut response = conn
            .connection()
            .query(
                "SELECT count() as total FROM vnode WHERE workspace_id = $workspace_id AND node_type = 'file'"
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?;

        let count_results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let total_files = count_results
            .first()
            .and_then(|v| v.get("total").and_then(|t| t.as_u64()))
            .unwrap_or(0) as usize;

        // Count directories
        let mut response = conn
            .connection()
            .query(
                "SELECT count() as total FROM vnode WHERE workspace_id = $workspace_id AND node_type = 'directory'"
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?;

        let count_results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let total_directories = count_results
            .first()
            .and_then(|v| v.get("total").and_then(|t| t.as_u64()))
            .unwrap_or(0) as usize;

        // Sum file sizes
        let mut response = conn
            .connection()
            .query(
                "SELECT math::sum(size_bytes) as total FROM vnode WHERE workspace_id = $workspace_id AND node_type = 'file'"
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?;

        let sum_results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let total_bytes = sum_results
            .first()
            .and_then(|v| v.get("total").and_then(|t| t.as_u64()))
            .unwrap_or(0);

        // Get language breakdown
        let mut response = conn
            .connection()
            .query(
                "SELECT language, count() as count FROM vnode
                 WHERE workspace_id = $workspace_id AND node_type = 'file' AND language IS NOT NULL
                 GROUP BY language"
            )
            .bind(("workspace_id", workspace_id.to_string()))
            .await?;

        let lang_results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let mut languages = HashMap::new();
        for result in lang_results {
            if let (Some(lang), Some(count)) = (result.get("language"), result.get("count")) {
                if let (Some(lang_str), Some(count_num)) = (lang.as_str(), count.as_u64()) {
                    languages.insert(lang_str.to_string(), count_num as usize);
                }
            }
        }

        // Count code units
        let mut response = conn
            .connection()
            .query("SELECT count() as total FROM code_unit")
            .await?;

        let count_results: Vec<serde_json::Value> = response.take(0).unwrap_or_default();
        let total_units = count_results
            .first()
            .and_then(|v| v.get("total").and_then(|t| t.as_u64()))
            .unwrap_or(0) as usize;

        Ok(WorkspaceStats {
            total_files,
            total_directories,
            total_units,
            total_bytes,
            languages,
        })
    }

    /// Sync workspace with changes
    pub async fn sync_workspace(&self, workspace_id: &Uuid, changes: Vec<FileChange>) -> Result<SyncResult> {
        info!("Syncing workspace {} with {} changes", workspace_id, changes.len());

        let mut files_created = 0;
        let mut files_updated = 0;
        let mut files_deleted = 0;
        let mut errors = Vec::new();

        for change in changes {
            let result = match change.change_type.as_str() {
                "created" => {
                    self.vfs.write_file(
                        workspace_id,
                        &cortex_vfs::VirtualPath::new(&change.path)?,
                        change.content.as_deref().unwrap_or("").as_bytes(),
                    ).await
                    .map(|_| { files_created += 1; })
                }
                "modified" => {
                    self.vfs.write_file(
                        workspace_id,
                        &cortex_vfs::VirtualPath::new(&change.path)?,
                        change.content.as_deref().unwrap_or("").as_bytes(),
                    ).await
                    .map(|_| { files_updated += 1; })
                }
                "deleted" => {
                    self.vfs.delete(
                        workspace_id,
                        &cortex_vfs::VirtualPath::new(&change.path)?,
                        false,
                    ).await
                    .map(|_| { files_deleted += 1; })
                }
                _ => {
                    errors.push(format!("Unknown change type: {}", change.change_type));
                    Ok(())
                }
            };

            if let Err(e) = result {
                errors.push(format!("Failed to sync {}: {}", change.path, e));
            }
        }

        Ok(SyncResult {
            files_created,
            files_updated,
            files_deleted,
            errors,
        })
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub workspace_type: String,
    pub source_path: Option<String>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub workspace_type: Option<String>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct ListWorkspaceFilters {
    pub workspace_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceDetails {
    pub id: String,
    pub name: String,
    pub workspace_type: String,
    pub source_type: String,
    pub namespace: String,
    pub source_path: Option<String>,
    pub read_only: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WorkspaceDetails {
    fn from_workspace(workspace: Workspace) -> Self {
        Self {
            id: workspace.id.to_string(),
            name: workspace.name,
            workspace_type: format!("{:?}", workspace.workspace_type).to_lowercase(),
            source_type: format!("{:?}", workspace.source_type).to_lowercase(),
            namespace: workspace.namespace,
            source_path: workspace.source_path.map(|p| p.to_string_lossy().to_string()),
            read_only: workspace.read_only,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceStats {
    pub total_files: usize,
    pub total_directories: usize,
    pub total_units: usize,
    pub total_bytes: u64,
    pub languages: HashMap<String, usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: String, // "created", "modified", "deleted"
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub files_created: usize,
    pub files_updated: usize,
    pub files_deleted: usize,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_details_serialization() {
        let details = WorkspaceDetails {
            id: Uuid::new_v4().to_string(),
            name: "test".to_string(),
            workspace_type: "code".to_string(),
            source_type: "local".to_string(),
            namespace: "ws_test".to_string(),
            source_path: Some("/path/to/workspace".to_string()),
            read_only: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("test"));
    }
}
