//! Project registry for managing all projects across monorepos
//!
//! The registry maintains metadata about all registered projects,
//! tracks their locations (with history for relocations), and
//! provides search and query capabilities.

use super::identity::ProjectIdentity;
use super::storage::GlobalStorage;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Status of a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    /// Project is active and at current path
    Active,
    /// Project has been moved (path changed)
    Moved,
    /// Project has been deleted
    Deleted,
    /// Project metadata is stale (needs refresh)
    Stale,
}

/// Entry in the path history
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathHistoryEntry {
    /// Previous path
    pub path: String,

    /// When the change occurred
    pub timestamp: DateTime<Utc>,

    /// Reason for the path change
    pub reason: String,

    /// Who/what initiated the change
    pub initiated_by: Option<String>,
}

/// Monorepo context information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonorepoContext {
    /// Monorepo ID
    pub id: String,

    /// Path to monorepo root
    pub path: String,

    /// Relative path within monorepo
    pub relative_path: String,
}

/// Indexing state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexingState {
    /// Last indexed timestamp
    pub last_indexed: Option<DateTime<Utc>>,

    /// Version of indexer used
    pub index_version: String,

    /// Current status
    pub status: IndexingStatus,

    /// Error message if status is Error
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexingStatus {
    Indexed,
    Indexing,
    Error,
    Pending,
}

/// Complete project registry entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRegistry {
    /// Project identity
    pub identity: ProjectIdentity,

    /// Current absolute path
    pub current_path: PathBuf,

    /// Specs directory path (for specifications)
    pub specs_path: Option<PathBuf>,

    /// History of path changes
    pub path_history: Vec<PathHistoryEntry>,

    /// Monorepo context (if part of a monorepo)
    pub monorepo: Option<MonorepoContext>,

    /// Indexing state
    pub indexing: IndexingState,

    /// Project status
    pub status: ProjectStatus,

    /// When the registry entry was created
    pub created_at: DateTime<Utc>,

    /// When it was last updated
    pub updated_at: DateTime<Utc>,

    /// Last access time (for cache management)
    pub last_accessed_at: DateTime<Utc>,
}

impl ProjectRegistry {
    /// Create a new registry entry for a project
    pub fn new(identity: ProjectIdentity, path: PathBuf) -> Self {
        let now = Utc::now();
        let path_str = path.display().to_string();

        // Ensure path is absolute
        let absolute_path = if path.is_absolute() {
            path
        } else {
            // Try to canonicalize first (resolves symlinks and makes absolute)
            if let Ok(canonical) = path.canonicalize() {
                canonical
            } else {
                // Fallback: manually construct absolute path without canonicalize
                std::env::current_dir()
                    .map(|cwd| cwd.join(&path))
                    .unwrap_or(path)
            }
        };

        // Check for specs directory (must be absolute)
        let specs_path = absolute_path.join("specs");
        let specs_path = if specs_path.exists() && specs_path.is_dir() {
            Some(specs_path)
        } else {
            None
        };

        Self {
            identity,
            current_path: absolute_path,
            specs_path,
            path_history: vec![PathHistoryEntry {
                path: path_str,
                timestamp: now,
                reason: "discovered".to_string(),
                initiated_by: None,
            }],
            monorepo: None,
            indexing: IndexingState {
                last_indexed: None,
                index_version: env!("CARGO_PKG_VERSION").to_string(),
                status: IndexingStatus::Pending,
                error_message: None,
            },
            status: ProjectStatus::Active,
            created_at: now,
            updated_at: now,
            last_accessed_at: now,
        }
    }

    /// Update the path and add to history
    pub fn relocate(&mut self, new_path: PathBuf, reason: String) {
        let old_path = self.current_path.display().to_string();
        self.current_path = new_path.clone();

        // Update specs path if it exists
        let specs_path = new_path.join("specs");
        self.specs_path = if specs_path.exists() && specs_path.is_dir() {
            Some(specs_path)
        } else {
            None
        };

        self.path_history.push(PathHistoryEntry {
            path: old_path,
            timestamp: Utc::now(),
            reason,
            initiated_by: Some("user".to_string()),
        });
        self.updated_at = Utc::now();
    }

    /// Mark project as accessed
    pub fn touch(&mut self) {
        self.last_accessed_at = Utc::now();
    }
}

/// Manager for the project registry
pub struct ProjectRegistryManager {
    storage: Arc<GlobalStorage>,
}

const CURRENT_PROJECT_KEY: &str = "current_project";

impl ProjectRegistryManager {
    /// Create a new registry manager
    pub fn new(storage: Arc<GlobalStorage>) -> Self {
        Self { storage }
    }

    /// Register a new project
    pub async fn register(&self, path: PathBuf) -> Result<ProjectRegistry> {
        // Generate identity
        let identity = ProjectIdentity::from_path(&path)
            .with_context(|| format!("Failed to create identity for path {:?}", path))?;

        // Check if already exists
        if let Some(existing) = self.get(&identity.full_id).await? {
            // Update path if different
            if existing.current_path != path {
                let mut updated = existing;
                updated.relocate(path, "auto-detected".to_string());
                self.update(updated.clone()).await?;
                return Ok(updated);
            }
            return Ok(existing);
        }

        // Create new registry entry
        let registry = ProjectRegistry::new(identity, path);
        self.update(registry.clone()).await?;

        Ok(registry)
    }

    /// Get a project by its full ID
    pub async fn get(&self, project_id: &str) -> Result<Option<ProjectRegistry>> {
        self.storage.get_project(project_id).await
    }

    /// Update a project registry entry
    pub async fn update(&self, mut registry: ProjectRegistry) -> Result<()> {
        registry.updated_at = Utc::now();
        self.storage.put_project(&registry).await
    }

    /// Delete a project from the registry
    pub async fn delete(&self, project_id: &str) -> Result<()> {
        // Mark as deleted rather than actually removing
        if let Some(mut registry) = self.get(project_id).await? {
            registry.status = ProjectStatus::Deleted;
            registry.updated_at = Utc::now();
            self.update(registry).await?;
        }
        Ok(())
    }

    /// Find project by path
    pub async fn find_by_path(&self, path: &Path) -> Result<Option<ProjectRegistry>> {
        self.storage.find_project_by_path(path).await
    }

    /// Find projects by name (supports partial matching)
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<ProjectRegistry>> {
        let all_projects = self.list_all().await?;
        Ok(all_projects
            .into_iter()
            .filter(|p| p.identity.id.contains(name))
            .collect())
    }

    /// List all active projects
    pub async fn list_all(&self) -> Result<Vec<ProjectRegistry>> {
        self.storage.list_all_projects().await
    }

    /// Relocate a project to a new path
    pub async fn relocate_project(
        &self,
        project_id: &str,
        new_path: PathBuf,
        reason: String,
    ) -> Result<()> {
        let mut registry = self
            .get(project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

        registry.relocate(new_path, reason);
        self.update(registry).await
    }

    /// Set the current project (used by MCP server)
    pub async fn set_current_project(&self, project_id: &str) -> Result<()> {
        // Verify project exists
        if self.get(project_id).await?.is_none() {
            anyhow::bail!("Project not found: {}", project_id);
        }

        // Store in a simple key-value manner
        self.storage.put_raw(CURRENT_PROJECT_KEY, project_id.as_bytes()).await
    }

    /// Get the current project ID
    pub async fn get_current_project(&self) -> Result<Option<String>> {
        match self.storage.get_raw(CURRENT_PROJECT_KEY).await? {
            Some(bytes) => Ok(Some(String::from_utf8(bytes)?)),
            None => Ok(None),
        }
    }

    /// Get the current project registry
    pub async fn get_current_project_registry(&self) -> Result<Option<ProjectRegistry>> {
        if let Some(project_id) = self.get_current_project().await? {
            self.get(&project_id).await
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_storage() -> Arc<GlobalStorage> {
        let temp_dir = TempDir::new().unwrap();
        Arc::new(GlobalStorage::new(temp_dir.path()).await.unwrap())
    }

    #[tokio::test]
    async fn test_register_project() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();

        assert_eq!(registry.identity.id, "test-project");
        assert_eq!(registry.status, ProjectStatus::Active);
        assert_eq!(registry.path_history.len(), 1);
    }

    #[tokio::test]
    async fn test_get_project() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test-get", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        let retrieved = manager.get(&registry.identity.full_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().identity.id, "test-get");
    }

    #[tokio::test]
    async fn test_relocate_project() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "test-relocate", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        let project_id = registry.identity.full_id.clone();

        let temp_dir2 = TempDir::new().unwrap();
        manager
            .relocate_project(&project_id, temp_dir2.path().to_path_buf(), "testing".to_string())
            .await
            .unwrap();

        let updated = manager.get(&project_id).await.unwrap().unwrap();
        assert_eq!(updated.current_path, temp_dir2.path());
        assert_eq!(updated.path_history.len(), 2);
        assert_eq!(updated.path_history[1].reason, "testing");
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "my-awesome-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        let temp_dir2 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir2.path().join("package.json"),
            r#"{"name": "other-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        manager.register(temp_dir2.path().to_path_buf()).await.unwrap();

        let results = manager.find_by_name("awesome").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].identity.id, "my-awesome-project");
    }

    #[tokio::test]
    async fn test_list_all() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "project1", "version": "1.0.0"}"#,
        )
        .unwrap();

        let temp_dir2 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir2.path().join("package.json"),
            r#"{"name": "project2", "version": "1.0.0"}"#,
        )
        .unwrap();

        manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        manager.register(temp_dir2.path().to_path_buf()).await.unwrap();

        let all = manager.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_project() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "test-delete", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        let project_id = registry.identity.full_id.clone();

        manager.delete(&project_id).await.unwrap();

        let deleted = manager.get(&project_id).await.unwrap().unwrap();
        assert_eq!(deleted.status, ProjectStatus::Deleted);
    }

    // Comprehensive project registry tests
    #[tokio::test]
    async fn test_register_with_all_metadata() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();

        // Create project with specs directory
        let specs_dir = temp_dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        std::fs::write(
            specs_dir.join("test-spec.md"),
            "# Test Specification",
        )
        .unwrap();

        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "full-metadata-project", "version": "1.5.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();

        // Verify all metadata
        assert_eq!(registry.identity.id, "full-metadata-project");
        assert_eq!(registry.identity.version, "1.5.0");
        assert_eq!(registry.status, ProjectStatus::Active);
        assert!(registry.specs_path.is_some());
        assert_eq!(registry.path_history.len(), 1);
        assert_eq!(registry.path_history[0].reason, "discovered");
        assert!(registry.monorepo.is_none());
        assert_eq!(registry.indexing.status, IndexingStatus::Pending);
    }

    #[tokio::test]
    async fn test_duplicate_registration_detection() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "duplicate-test", "version": "1.0.0"}"#,
        )
        .unwrap();

        // Register first time
        let registry1 = manager.register(temp_dir.path().to_path_buf()).await.unwrap();

        // Register again - should return existing
        let registry2 = manager.register(temp_dir.path().to_path_buf()).await.unwrap();

        assert_eq!(registry1.identity.full_id, registry2.identity.full_id);
        assert_eq!(registry1.created_at, registry2.created_at);
    }

    #[tokio::test]
    async fn test_path_relocation_workflow() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir1.path().join("Cargo.toml"),
            r#"[package]
name = "relocatable-crate"
version = "3.0.0"
"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        let project_id = registry.identity.full_id.clone();
        let old_path = registry.current_path.clone();

        let temp_dir2 = TempDir::new().unwrap();

        // Relocate with custom reason
        manager
            .relocate_project(&project_id, temp_dir2.path().to_path_buf(), "moved to new server".to_string())
            .await
            .unwrap();

        let relocated = manager.get(&project_id).await.unwrap().unwrap();

        assert_eq!(relocated.current_path, temp_dir2.path());
        assert_eq!(relocated.path_history.len(), 2);
        assert_eq!(relocated.path_history[0].path, old_path.display().to_string());
        assert_eq!(relocated.path_history[1].reason, "moved to new server");
        assert!(relocated.path_history[1].initiated_by.is_some());
    }

    #[tokio::test]
    async fn test_registry_persistence_across_restarts() {
        let temp_storage_dir = TempDir::new().unwrap();

        // Create and register project
        {
            let storage = Arc::new(GlobalStorage::new(temp_storage_dir.path()).await.unwrap());
            let manager = ProjectRegistryManager::new(storage);

            let temp_project = TempDir::new().unwrap();
            std::fs::write(
                temp_project.path().join("package.json"),
                r#"{"name": "persistent-project", "version": "1.0.0"}"#,
            )
            .unwrap();

            manager.register(temp_project.path().to_path_buf()).await.unwrap();
            std::mem::forget(temp_project); // Keep directory alive
        }

        // Restart storage and verify project still exists
        {
            let storage = Arc::new(GlobalStorage::new(temp_storage_dir.path()).await.unwrap());
            let manager = ProjectRegistryManager::new(storage);

            let projects = manager.list_all().await.unwrap();
            assert_eq!(projects.len(), 1);
            assert_eq!(projects[0].identity.id, "persistent-project");
        }
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use tokio::task;

        let storage = create_test_storage().await;
        let manager = Arc::new(ProjectRegistryManager::new(storage));

        // Create multiple projects concurrently
        let mut handles = vec![];

        for i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let handle = task::spawn(async move {
                let temp_dir = TempDir::new().unwrap();
                std::fs::write(
                    temp_dir.path().join("package.json"),
                    format!(r#"{{"name": "concurrent-{}", "version": "1.0.0"}}"#, i),
                )
                .unwrap();

                let result = manager_clone.register(temp_dir.path().to_path_buf()).await;
                std::mem::forget(temp_dir);
                result
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all projects registered
        let all_projects = manager.list_all().await.unwrap();
        assert_eq!(all_projects.len(), 5);
    }

    #[tokio::test]
    async fn test_find_by_path() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "find-by-path", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();

        // Find by exact path
        let found = manager.find_by_path(temp_dir.path()).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().identity.full_id, registry.identity.full_id);

        // Find by non-existent path
        let temp_dir2 = TempDir::new().unwrap();
        let not_found = manager.find_by_path(temp_dir2.path()).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_current_project_management() {
        let storage = create_test_storage().await;
        let manager = Arc::new(ProjectRegistryManager::new(storage));

        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "project-one", "version": "1.0.0"}"#,
        )
        .unwrap();

        std::fs::write(
            temp_dir2.path().join("package.json"),
            r#"{"name": "project-two", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry1 = manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        let registry2 = manager.register(temp_dir2.path().to_path_buf()).await.unwrap();

        // Set first as current
        manager.set_current_project(&registry1.identity.full_id).await.unwrap();
        let current = manager.get_current_project().await.unwrap();
        assert_eq!(current.unwrap(), registry1.identity.full_id);

        // Switch to second
        manager.set_current_project(&registry2.identity.full_id).await.unwrap();
        let current = manager.get_current_project().await.unwrap();
        assert_eq!(current.unwrap(), registry2.identity.full_id);

        // Get current registry
        let current_reg = manager.get_current_project_registry().await.unwrap().unwrap();
        assert_eq!(current_reg.identity.id, "project-two");
    }

    #[tokio::test]
    async fn test_set_current_project_nonexistent() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let result = manager.set_current_project("nonexistent@1.0.0").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_touch_updates_access_time() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "touch-test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let mut registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        let initial_access = registry.last_accessed_at;

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Touch the registry
        registry.touch();
        manager.update(registry.clone()).await.unwrap();

        let updated = manager.get(&registry.identity.full_id).await.unwrap().unwrap();
        assert!(updated.last_accessed_at > initial_access);
    }

    #[tokio::test]
    async fn test_registry_with_specs_directory() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        let specs_dir = temp_dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "with-specs", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        assert!(registry.specs_path.is_some());
        assert_eq!(registry.specs_path.unwrap(), specs_dir);
    }

    #[tokio::test]
    async fn test_registry_without_specs_directory() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "no-specs", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        assert!(registry.specs_path.is_none());
    }

    #[tokio::test]
    async fn test_multiple_relocations() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "multi-relocate", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        let project_id = registry.identity.full_id.clone();

        // Relocate multiple times
        for i in 0..3 {
            let temp_dir = TempDir::new().unwrap();
            manager
                .relocate_project(
                    &project_id,
                    temp_dir.path().to_path_buf(),
                    format!("relocation {}", i + 1),
                )
                .await
                .unwrap();
            std::mem::forget(temp_dir);
        }

        let final_registry = manager.get(&project_id).await.unwrap().unwrap();
        assert_eq!(final_registry.path_history.len(), 4); // Initial + 3 relocations
        assert_eq!(final_registry.path_history[3].reason, "relocation 3");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_project() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        // Deleting non-existent project should succeed silently
        let result = manager.delete("nonexistent@1.0.0").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_update_timestamps() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("package.json"),
            r#"{"name": "timestamp-test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let registry = manager.register(temp_dir.path().to_path_buf()).await.unwrap();
        let created_at = registry.created_at;
        let updated_at = registry.updated_at;

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update should change updated_at but not created_at
        manager.update(registry.clone()).await.unwrap();
        let updated = manager.get(&registry.identity.full_id).await.unwrap().unwrap();

        assert_eq!(updated.created_at, created_at);
        assert!(updated.updated_at > updated_at);
    }

    #[tokio::test]
    async fn test_find_by_name_partial_match() {
        let storage = create_test_storage().await;
        let manager = ProjectRegistryManager::new(storage);

        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        let temp_dir3 = TempDir::new().unwrap();

        std::fs::write(
            temp_dir1.path().join("package.json"),
            r#"{"name": "awesome-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        std::fs::write(
            temp_dir2.path().join("package.json"),
            r#"{"name": "another-awesome-lib", "version": "1.0.0"}"#,
        )
        .unwrap();

        std::fs::write(
            temp_dir3.path().join("package.json"),
            r#"{"name": "unrelated-project", "version": "1.0.0"}"#,
        )
        .unwrap();

        manager.register(temp_dir1.path().to_path_buf()).await.unwrap();
        manager.register(temp_dir2.path().to_path_buf()).await.unwrap();
        manager.register(temp_dir3.path().to_path_buf()).await.unwrap();

        let results = manager.find_by_name("awesome").await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
