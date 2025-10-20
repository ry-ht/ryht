//! Global SurrealDB storage wrapper
//!
//! Implements the global storage schema as defined in global-architecture-spec.md.
//! This is the global database that stores all projects across all monorepos.
//!
//! Migration from RocksDB to SurrealDB provides:
//! - Graph query capabilities for cross-project relationships
//! - Built-in indexing and query optimization
//! - Unified storage with main Meridian database

use super::registry::ProjectRegistry;
use crate::storage::{SurrealDBStorage, SurrealDBConfig, Storage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// SurrealDB record for project registry
#[derive(Debug, Serialize, Deserialize)]
struct ProjectRecord {
    /// Project full ID (used as record ID)
    id: String,
    /// The actual project registry data
    #[serde(flatten)]
    registry: ProjectRegistry,
}

/// SurrealDB record for name index
#[derive(Debug, Serialize, Deserialize)]
struct NameIndexRecord {
    /// Project name (used as record ID)
    name: String,
    /// Full project ID
    full_id: String,
}

/// SurrealDB record for path index
#[derive(Debug, Serialize, Deserialize)]
struct PathIndexRecord {
    /// Path hash (used as record ID)
    path_hash: String,
    /// Full project ID
    full_id: String,
}

/// SurrealDB record for monorepo index
#[derive(Debug, Serialize, Deserialize)]
struct MonorepoIndexRecord {
    /// Monorepo ID (used as record ID)
    monorepo_id: String,
    /// List of full project IDs
    project_ids: Vec<String>,
}

/// Global storage using SurrealDB
///
/// Schema (as per global-architecture-spec.md):
/// ```
/// Table: global_projects           → ProjectRegistry (full data)
/// Table: global_name_index         → name -> full_id mapping
/// Table: global_path_index         → path_hash -> full_id mapping
/// Table: global_monorepo_index     → monorepo_id -> [full_ids]
/// ```
pub struct GlobalStorage {
    storage: Arc<SurrealDBStorage>,
}

impl GlobalStorage {
    /// Create a new global storage with automatic SurrealDB configuration
    pub async fn new(path: &Path) -> Result<Self> {
        let config = SurrealDBConfig {
            namespace: "meridian_global".to_string(),
            database: "registry".to_string(),
        };
        Self::new_with_config(path, config).await
    }

    /// Create a new global storage with custom configuration
    pub async fn new_with_config(path: &Path, config: SurrealDBConfig) -> Result<Self> {
        tracing::info!(
            path = ?path,
            namespace = %config.namespace,
            database = %config.database,
            "Initializing global SurrealDB storage"
        );

        let storage = SurrealDBStorage::new_with_config(path, config)
            .await
            .context("Failed to create SurrealDB storage")?;

        let global_storage = Self {
            storage: Arc::new(storage),
        };

        // Initialize schema
        global_storage.initialize_schema().await?;

        tracing::info!("Global SurrealDB storage initialized successfully");

        Ok(global_storage)
    }

    /// Initialize the database schema
    async fn initialize_schema(&self) -> Result<()> {
        tracing::debug!("Initializing global storage schema");

        let db = self.storage.db();

        // Define tables and indexes
        let schema = r#"
            -- Projects table: stores complete project registry data (SCHEMALESS for enum compatibility)
            DEFINE TABLE IF NOT EXISTS global_projects SCHEMALESS;
            DEFINE INDEX IF NOT EXISTS idx_project_id ON TABLE global_projects COLUMNS full_id UNIQUE;

            -- Name index: project name -> full_id
            DEFINE TABLE IF NOT EXISTS global_name_index SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS name ON TABLE global_name_index TYPE string;
            DEFINE FIELD IF NOT EXISTS full_id ON TABLE global_name_index TYPE string;
            DEFINE INDEX IF NOT EXISTS idx_name ON TABLE global_name_index COLUMNS name UNIQUE;

            -- Path index: path_hash -> full_id
            DEFINE TABLE IF NOT EXISTS global_path_index SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS path_hash ON TABLE global_path_index TYPE string;
            DEFINE FIELD IF NOT EXISTS full_id ON TABLE global_path_index TYPE string;
            DEFINE INDEX IF NOT EXISTS idx_path_hash ON TABLE global_path_index COLUMNS path_hash UNIQUE;

            -- Monorepo index: monorepo_id -> [project_ids]
            DEFINE TABLE IF NOT EXISTS global_monorepo_index SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS monorepo_id ON TABLE global_monorepo_index TYPE string;
            DEFINE FIELD IF NOT EXISTS project_ids ON TABLE global_monorepo_index TYPE array;
            DEFINE INDEX IF NOT EXISTS idx_monorepo_id ON TABLE global_monorepo_index COLUMNS monorepo_id UNIQUE;
        "#;

        db.query(schema)
            .await
            .context("Failed to initialize global storage schema")?;

        tracing::debug!("Global storage schema initialized successfully");

        Ok(())
    }

    /// Put a project into the registry
    pub async fn put_project(&self, registry: &ProjectRegistry) -> Result<()> {
        let db = self.storage.db();
        let full_id = registry.identity.full_id.clone();

        // Store project data using delete+create pattern for upsert
        // First try to delete any existing record
        let san_id = sanitize_record_id(&full_id);
        let _: Option<serde_json::Value> = db.delete(("global_projects", san_id.as_str())).await.ok().flatten();

        // Serialize registry to JSON string to avoid enum serialization issues
        let registry_json_str = serde_json::to_string(registry)
            .context("Failed to serialize project registry")?;

        #[derive(Serialize)]
        struct StoredProject {
            full_id: String,
            registry: String,  // Store as JSON string
        }

        let wrapped = StoredProject {
            full_id: full_id.clone(),
            registry: registry_json_str,
        };

        let _: Option<serde_json::Value> = db
            .create(("global_projects", san_id.as_str()))
            .content(wrapped)
            .await
            .context("Failed to store project in SurrealDB")?;

        // Update indexes
        self.update_indexes(registry).await?;

        tracing::debug!(project_id = %full_id, "Project stored successfully");

        Ok(())
    }

    /// Get a project by its full ID
    pub async fn get_project(&self, full_id: &str) -> Result<Option<ProjectRegistry>> {
        let db = self.storage.db();

        let query = format!(
            "SELECT registry FROM global_projects:`{}` LIMIT 1",
            sanitize_record_id(full_id)
        );

        let mut response = db.query(query).await.context("Failed to query project")?;

        #[derive(Deserialize)]
        struct ProjectResult {
            registry: String,  // Stored as JSON string
        }

        let results: Vec<ProjectResult> = response.take(0).unwrap_or_default();

        // Deserialize from JSON string
        if let Some(result) = results.into_iter().next() {
            let registry = serde_json::from_str(&result.registry)
                .context("Failed to deserialize project registry")?;
            Ok(Some(registry))
        } else {
            Ok(None)
        }
    }

    /// Find project by path
    pub async fn find_project_by_path(&self, path: &Path) -> Result<Option<ProjectRegistry>> {
        let path_hash = Self::hash_path(path);
        let db = self.storage.db();

        // Query path index to get full_id
        let query = format!(
            "SELECT full_id FROM global_path_index:`{}` LIMIT 1",
            sanitize_record_id(&path_hash)
        );

        let mut response = db
            .query(query)
            .await
            .context("Failed to query path index")?;

        #[derive(Deserialize)]
        struct PathResult {
            full_id: String,
        }

        let results: Vec<PathResult> = response.take(0).unwrap_or_default();

        if let Some(result) = results.into_iter().next() {
            self.get_project(&result.full_id).await
        } else {
            Ok(None)
        }
    }

    /// List all projects
    pub async fn list_all_projects(&self) -> Result<Vec<ProjectRegistry>> {
        let db = self.storage.db();

        let query = "SELECT registry FROM global_projects";

        let mut response = db
            .query(query)
            .await
            .context("Failed to query all projects")?;

        #[derive(Deserialize)]
        struct ProjectResult {
            registry: String,  // Stored as JSON string
        }

        let results: Vec<ProjectResult> = response.take(0).unwrap_or_default();

        // Deserialize from JSON strings
        results.into_iter()
            .map(|r| serde_json::from_str(&r.registry)
                .context("Failed to deserialize project registry"))
            .collect()
    }

    /// Update all indexes for a project
    async fn update_indexes(&self, registry: &ProjectRegistry) -> Result<()> {
        let db = self.storage.db();
        let full_id = registry.identity.full_id.clone();
        let name = registry.identity.id.clone();

        // Name index
        let name_san = sanitize_record_id(&name);
        let name_query = format!(
            "DELETE global_name_index:`{}`; CREATE global_name_index:`{}` CONTENT {{ name: $name, full_id: $full_id }}",
            name_san, name_san
        );

        db.query(name_query)
            .bind(("name", name))
            .bind(("full_id", full_id.clone()))
            .await
            .context("Failed to update name index")?;

        // Path index
        let path_hash = Self::hash_path(&registry.current_path);
        let path_san = sanitize_record_id(&path_hash);
        let path_query = format!(
            "DELETE global_path_index:`{}`; CREATE global_path_index:`{}` CONTENT {{ path_hash: $path_hash, full_id: $full_id }}",
            path_san, path_san
        );

        db.query(path_query)
            .bind(("path_hash", path_hash))
            .bind(("full_id", full_id.clone()))
            .await
            .context("Failed to update path index")?;

        // Monorepo index (if applicable)
        if let Some(ref monorepo) = registry.monorepo {
            let monorepo_id = monorepo.id.clone();
            let monorepo_rec_id = sanitize_record_id(&monorepo_id);

            // Get existing project IDs
            let get_query = format!(
                "SELECT project_ids FROM global_monorepo_index:`{}` LIMIT 1",
                monorepo_rec_id
            );

            let mut response = db
                .query(&get_query)
                .await
                .context("Failed to query monorepo index")?;

            #[derive(Deserialize)]
            struct MonorepoResult {
                project_ids: Vec<String>,
            }

            let mut project_ids: Vec<String> = response
                .take::<Vec<MonorepoResult>>(0)
                .unwrap_or_default()
                .into_iter()
                .next()
                .map(|r| r.project_ids)
                .unwrap_or_default();

            // Add this project if not already there
            if !project_ids.contains(&full_id) {
                project_ids.push(full_id);

                let update_query = format!(
                    "DELETE global_monorepo_index:`{}`; CREATE global_monorepo_index:`{}` CONTENT {{ monorepo_id: $monorepo_id, project_ids: $project_ids }}",
                    monorepo_rec_id, monorepo_rec_id
                );

                db.query(update_query)
                    .bind(("monorepo_id", monorepo_id))
                    .bind(("project_ids", project_ids))
                    .await
                    .context("Failed to update monorepo index")?;
            }
        }

        Ok(())
    }

    /// Hash a path for indexing
    fn hash_path(path: &Path) -> String {
        let path_str = path.display().to_string();
        let hash = blake3::hash(path_str.as_bytes());
        hash.to_hex().to_string()
    }

    /// Get the underlying storage (for advanced operations)
    pub fn storage(&self) -> Arc<SurrealDBStorage> {
        Arc::clone(&self.storage)
    }

    /// Put raw key-value pair (for backwards compatibility)
    pub async fn put_raw(&self, key: &str, value: &[u8]) -> Result<()> {
        self.storage
            .put(key.as_bytes(), value)
            .await
            .with_context(|| format!("Failed to store raw key {}", key))
    }

    /// Get raw value by key (for backwards compatibility)
    pub async fn get_raw(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.storage
            .get(key.as_bytes())
            .await
            .with_context(|| format!("Failed to get raw key {}", key))
    }
}

/// Sanitize a string to be used as a SurrealDB record ID
/// SurrealDB record IDs have restrictions on special characters
fn sanitize_record_id(id: &str) -> String {
    // Replace characters that are problematic in SurrealDB record IDs
    id.replace('@', "_at_")
        .replace('/', "_slash_")
        .replace(':', "_colon_")
        .replace('.', "_dot_")
        .replace('-', "_dash_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::identity::ProjectIdentity;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_put_and_get_project() {
        let temp_dir = TempDir::new().unwrap();
        let storage = GlobalStorage::new(temp_dir.path()).await.unwrap();

        let project_dir = TempDir::new().unwrap();
        std::fs::write(
            project_dir.path().join("package.json"),
            r#"{"name": "test", "version": "1.0.0"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(project_dir.path()).unwrap();
        let registry = ProjectRegistry::new(identity.clone(), project_dir.path().to_path_buf());

        storage.put_project(&registry).await.unwrap();

        let retrieved = storage.get_project(&identity.full_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().identity.id, "test");
    }

    #[tokio::test]
    async fn test_find_by_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = GlobalStorage::new(temp_dir.path()).await.unwrap();

        let project_dir = TempDir::new().unwrap();
        std::fs::write(
            project_dir.path().join("package.json"),
            r#"{"name": "test-path", "version": "1.0.0"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(project_dir.path()).unwrap();
        let registry = ProjectRegistry::new(identity, project_dir.path().to_path_buf());

        storage.put_project(&registry).await.unwrap();

        let found = storage
            .find_project_by_path(project_dir.path())
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().identity.id, "test-path");
    }

    #[tokio::test]
    async fn test_list_all_projects() {
        let temp_dir = TempDir::new().unwrap();
        let storage = GlobalStorage::new(temp_dir.path()).await.unwrap();

        // Create two projects
        let project1 = TempDir::new().unwrap();
        std::fs::write(
            project1.path().join("package.json"),
            r#"{"name": "project1", "version": "1.0.0"}"#,
        )
        .unwrap();

        let project2 = TempDir::new().unwrap();
        std::fs::write(
            project2.path().join("package.json"),
            r#"{"name": "project2", "version": "1.0.0"}"#,
        )
        .unwrap();

        let identity1 = ProjectIdentity::from_npm(project1.path()).unwrap();
        let registry1 = ProjectRegistry::new(identity1, project1.path().to_path_buf());

        let identity2 = ProjectIdentity::from_npm(project2.path()).unwrap();
        let registry2 = ProjectRegistry::new(identity2, project2.path().to_path_buf());

        storage.put_project(&registry1).await.unwrap();
        storage.put_project(&registry2).await.unwrap();

        let all = storage.list_all_projects().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_path_hash_stability() {
        let path = Path::new("/some/test/path");
        let hash1 = GlobalStorage::hash_path(path);
        let hash2 = GlobalStorage::hash_path(path);
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_update_after_insert() {
        let temp_dir = TempDir::new().unwrap();
        let storage = GlobalStorage::new(temp_dir.path()).await.unwrap();

        let project_dir = TempDir::new().unwrap();
        std::fs::write(
            project_dir.path().join("package.json"),
            r#"{"name": "test-update", "version": "1.0.0"}"#,
        )
        .unwrap();

        let identity = ProjectIdentity::from_npm(project_dir.path()).unwrap();
        let mut registry = ProjectRegistry::new(identity.clone(), project_dir.path().to_path_buf());

        storage.put_project(&registry).await.unwrap();

        // Update the registry
        let new_path = TempDir::new().unwrap();
        registry.relocate(new_path.path().to_path_buf(), "test".to_string());
        storage.put_project(&registry).await.unwrap();

        let updated = storage.get_project(&identity.full_id).await.unwrap().unwrap();
        assert_eq!(updated.path_history.len(), 2);
    }

    #[test]
    fn test_sanitize_record_id() {
        assert_eq!(sanitize_record_id("simple"), "simple");
        assert_eq!(sanitize_record_id("@scope/package"), "_at_scope_slash_package");
        assert_eq!(sanitize_record_id("test@1.0.0"), "test_at_1_dot_0_dot_0");
        assert_eq!(sanitize_record_id("my-package"), "my_dash_package");
        assert_eq!(sanitize_record_id("ns:key"), "ns_colon_key");
    }

    #[tokio::test]
    async fn test_raw_key_value() {
        let temp_dir = TempDir::new().unwrap();
        let storage = GlobalStorage::new(temp_dir.path()).await.unwrap();

        storage.put_raw("test_key", b"test_value").await.unwrap();
        let value = storage.get_raw("test_key").await.unwrap();

        assert!(value.is_some());
        assert_eq!(value.unwrap(), b"test_value");
    }
}
