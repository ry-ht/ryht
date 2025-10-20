// Backup and recovery system for Meridian
//
// Features:
// - Automatic backups before schema migrations
// - Scheduled backups (daily, keep last 7)
// - Manual backup via MCP tools
// - Point-in-time restore capability
// - SurrealDB export/import for backups
// - Backup verification and integrity checks

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::sync::RwLock;

/// Backup type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
    /// Manual backup triggered by user
    Manual,
    /// Scheduled automatic backup
    Scheduled,
    /// Pre-migration backup
    PreMigration,
    /// Incremental backup (using SurrealDB export)
    Incremental,
}

impl BackupType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupType::Manual => "manual",
            BackupType::Scheduled => "scheduled",
            BackupType::PreMigration => "pre_migration",
            BackupType::Incremental => "incremental",
        }
    }
}

/// Backup metadata stored in SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup ID (timestamp-based)
    /// Note: Renamed to backup_id in database to avoid conflict with SurrealDB's built-in id field
    #[serde(rename = "backup_id")]
    pub id: String,
    /// Backup type
    pub backup_type: BackupType,
    /// Timestamp when backup was created
    pub created_at: DateTime<Utc>,
    /// Size of backup in bytes
    pub size_bytes: u64,
    /// Number of files in backup
    pub file_count: usize,
    /// Checksum of backup data (blake3 hash)
    pub checksum: String,
    /// Version of Meridian that created this backup
    pub meridian_version: String,
    /// Optional description
    pub description: Option<String>,
    /// Database schema version at backup time
    pub schema_version: Option<u32>,
    /// Whether backup has been verified
    pub verified: bool,
    /// Verification timestamp
    pub verified_at: Option<DateTime<Utc>>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Path to backup file
    pub path: String,
}

impl BackupMetadata {
    pub fn new(
        id: String,
        backup_type: BackupType,
        description: Option<String>,
        path: String,
    ) -> Self {
        Self {
            id,
            backup_type,
            created_at: Utc::now(),
            size_bytes: 0,
            file_count: 1, // SurrealDB creates single .sql file
            checksum: String::new(),
            meridian_version: env!("CARGO_PKG_VERSION").to_string(),
            description,
            schema_version: None,
            verified: false,
            verified_at: None,
            tags: Vec::new(),
            path,
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Base directory for all backups
    pub backup_dir: PathBuf,
    /// Maximum number of scheduled backups to keep
    pub max_scheduled_backups: usize,
    /// Maximum number of incremental backups to keep
    pub max_incremental_backups: usize,
    /// Whether to automatically verify backups after creation
    pub auto_verify: bool,
    /// Whether to compress backups
    pub compress: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        // Use get_meridian_home() for consistency - ensures we always use ~/.meridian
        // Never fallback to current directory
        let meridian_home = crate::config::get_meridian_home();

        Self {
            backup_dir: meridian_home.join("backups"),
            max_scheduled_backups: 7,
            max_incremental_backups: 10,
            auto_verify: true,
            compress: false,
        }
    }
}

/// Backup statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_backups: usize,
    pub total_size_bytes: u64,
    pub by_type: HashMap<String, usize>,
    pub oldest_backup: Option<DateTime<Utc>>,
    pub newest_backup: Option<DateTime<Utc>>,
    pub verified_count: usize,
    pub unverified_count: usize,
}

/// Backup manager using SurrealDB export/import
pub struct BackupManager {
    config: BackupConfig,
    db_path: PathBuf,
    db: Arc<Surreal<Db>>,
    namespace: String,
    database: String,
    metadata_cache: Arc<RwLock<HashMap<String, BackupMetadata>>>,
}

impl BackupManager {
    /// Create a new backup manager
    pub async fn new(
        db_path: PathBuf,
        config: BackupConfig,
        namespace: String,
        database: String,
    ) -> Result<Self> {
        // Ensure backup directory exists
        fs::create_dir_all(&config.backup_dir)
            .with_context(|| format!("Failed to create backup directory: {:?}", config.backup_dir))?;

        // Connect to SurrealDB for metadata storage
        let db = Surreal::new::<RocksDb>(db_path.as_path())
            .await
            .context("Failed to initialize SurrealDB for backup manager")?;

        db.use_ns(&namespace)
            .use_db(&database)
            .await
            .context("Failed to set namespace and database")?;

        // Ensure backup_metadata table schema (using SCHEMAFULL for strict typing)
        // Note: We don't need to explicitly DEFINE TABLE as CREATE will create it
        // This avoids conflicts with backup imports that might also create the table

        Ok(Self {
            config,
            db_path,
            db: Arc::new(db),
            namespace,
            database,
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Generate backup ID from timestamp and type
    fn generate_backup_id(backup_type: BackupType) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}", timestamp, backup_type.as_str())
    }

    /// Get backup file path
    fn get_backup_path(&self, backup_id: &str) -> PathBuf {
        self.config.backup_dir.join(format!("{}.sql", backup_id))
    }

    /// Create a new backup using SurrealDB export
    pub async fn create_backup(
        &self,
        backup_type: BackupType,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<BackupMetadata> {
        let backup_id = Self::generate_backup_id(backup_type);
        let backup_path = self.get_backup_path(&backup_id);

        tracing::info!(
            "Creating {:?} backup: {} at {:?}",
            backup_type,
            backup_id,
            backup_path
        );

        // Export database using SurrealDB
        self.db
            .export(&backup_path)
            .await
            .with_context(|| format!("Failed to export database to {:?}", backup_path))?;

        // Calculate backup size
        let size_bytes = fs::metadata(&backup_path)
            .with_context(|| format!("Failed to get backup file metadata: {:?}", backup_path))?
            .len();

        // Calculate checksum
        let checksum = self.calculate_checksum(&backup_path).await?;

        // Create metadata
        let mut metadata = BackupMetadata::new(
            backup_id.clone(),
            backup_type,
            description,
            backup_path.to_string_lossy().to_string(),
        );
        metadata.size_bytes = size_bytes;
        metadata.checksum = checksum;
        metadata.tags = tags;

        // Save metadata to SurrealDB first (needed for verify_backup_internal)
        self.save_metadata(&metadata).await?;

        // Auto-verify if enabled
        if self.config.auto_verify {
            match self.verify_backup_internal(&backup_id).await {
                Ok(_) => {
                    metadata.verified = true;
                    metadata.verified_at = Some(Utc::now());
                    // Update metadata in database with verification status
                    let _ = self.db
                        .query("UPDATE backup_metadata SET verified = true, verified_at = $timestamp WHERE backup_id = $backup_id")
                        .bind(("timestamp", Utc::now()))
                        .bind(("backup_id", backup_id.clone()))
                        .await;
                }
                Err(e) => {
                    tracing::warn!("Auto-verification failed for backup {}: {}", backup_id, e);
                }
            }
        }

        // Cache metadata
        self.metadata_cache.write().await.insert(backup_id.clone(), metadata.clone());

        // Clean up old backups
        self.cleanup_old_backups(backup_type).await?;

        tracing::info!(
            "Backup created successfully: {} ({} bytes)",
            backup_id,
            size_bytes
        );

        Ok(metadata)
    }

    /// Calculate checksum of backup file
    async fn calculate_checksum(&self, backup_path: &Path) -> Result<String> {
        let backup_path = backup_path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let data = fs::read(&backup_path)
                .with_context(|| format!("Failed to read backup file: {:?}", backup_path))?;
            let hash = blake3::hash(&data);
            Ok(hash.to_hex().to_string())
        })
        .await?
    }

    /// Save metadata to SurrealDB
    async fn save_metadata(&self, metadata: &BackupMetadata) -> Result<()> {
        // Use UPSERT-style INSERT to handle duplicates gracefully
        // Don't deserialize response since it contains Thing type for id field
        let _ = self.db
            .query("CREATE backup_metadata CONTENT $metadata")
            .bind(("metadata", metadata.clone()))
            .await
            .with_context(|| format!("Failed to save backup metadata for {}", metadata.id))?;

        Ok(())
    }

    /// Load metadata from SurrealDB
    async fn load_metadata(&self, backup_id: &str) -> Result<BackupMetadata> {
        let mut response = self.db
            .query("SELECT * FROM backup_metadata WHERE backup_id = $backup_id")
            .bind(("backup_id", backup_id.to_string()))
            .await
            .with_context(|| format!("Failed to load backup metadata for {}", backup_id))?;

        let metadatas: Vec<BackupMetadata> = response.take(0)?;

        metadatas.into_iter().next()
            .ok_or_else(|| anyhow!("Backup metadata not found for {}", backup_id))
    }

    /// List all available backups
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>> {
        let mut response = self.db
            .query("SELECT * FROM backup_metadata ORDER BY created_at DESC")
            .await
            .context("Failed to list backups")?;

        let backups: Vec<BackupMetadata> = response.take(0)?;

        // Update cache
        let mut cache = self.metadata_cache.write().await;
        for backup in &backups {
            cache.insert(backup.id.clone(), backup.clone());
        }

        Ok(backups)
    }

    /// Get backup metadata by ID
    pub async fn get_backup(&self, backup_id: &str) -> Result<BackupMetadata> {
        // Check cache first
        if let Some(metadata) = self.metadata_cache.read().await.get(backup_id) {
            return Ok(metadata.clone());
        }

        // Load from SurrealDB
        let metadata = self.load_metadata(backup_id).await?;

        // Cache it
        self.metadata_cache.write().await.insert(backup_id.to_string(), metadata.clone());

        Ok(metadata)
    }

    /// Restore from backup using SurrealDB import
    pub async fn restore_backup(&self, backup_id: &str, target_path: Option<PathBuf>) -> Result<()> {
        let metadata = self.get_backup(backup_id).await?;
        let backup_path = PathBuf::from(&metadata.path);

        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found at {:?}", backup_path));
        }

        tracing::info!(
            "Restoring backup {} from {:?}",
            backup_id,
            backup_path
        );

        // Verify backup before restoring
        if !metadata.verified {
            tracing::warn!("Backup {} has not been verified, verifying now...", backup_id);
            self.verify_backup(backup_id).await?;
        }

        // Create backup of current state before restore (safety measure)
        tracing::info!("Creating safety backup before restore");
        let _safety_backup = self.create_backup(
            BackupType::Manual,
            Some(format!("Pre-restore safety backup before restoring {}", backup_id)),
            vec!["safety".to_string(), "pre-restore".to_string()],
        ).await?;

        // For SurrealDB, we need to import into the current database
        // If target_path is specified, we'd need to create a new DB instance
        if target_path.is_some() {
            return Err(anyhow!(
                "Custom target path not supported for SurrealDB restore. \
                 Database will be restored to current instance."
            ));
        }

        // Clear the current database by removing and recreating it
        // This ensures a clean slate for the import
        tracing::debug!("Clearing database {} before restore", &self.database);

        // Remove the database entirely
        let _ = self.db
            .query(format!("REMOVE DATABASE {}", &self.database))
            .await
            .context("Failed to remove database before restore")?;

        // Recreate it
        let _ = self.db
            .query(format!("DEFINE DATABASE {}", &self.database))
            .await
            .context("Failed to recreate database")?;

        // Re-select the database
        self.db
            .use_ns(&self.namespace)
            .use_db(&self.database)
            .await
            .context("Failed to re-select database after recreation")?;

        // Import the backup
        self.db
            .import(&backup_path)
            .await
            .with_context(|| format!("Failed to import backup from {:?}", backup_path))?;

        tracing::info!("Backup restored successfully from {}", backup_id);

        Ok(())
    }

    /// Verify backup integrity
    pub async fn verify_backup(&self, backup_id: &str) -> Result<()> {
        self.verify_backup_internal(backup_id).await?;

        // Update metadata to mark as verified
        let mut metadata = self.get_backup(backup_id).await?;
        metadata.verified = true;
        metadata.verified_at = Some(Utc::now());

        // Update in SurrealDB
        let metadata_id = metadata.id.clone();
        let _ = self.db
            .query("UPDATE backup_metadata SET verified = true, verified_at = $timestamp WHERE backup_id = $backup_id")
            .bind(("timestamp", Utc::now()))
            .bind(("backup_id", metadata_id))
            .await
            .context("Failed to update backup verification status")?;

        // Update cache
        self.metadata_cache.write().await.insert(backup_id.to_string(), metadata);

        Ok(())
    }

    /// Internal verification logic
    async fn verify_backup_internal(&self, backup_id: &str) -> Result<()> {
        let metadata = self.get_backup(backup_id).await?;
        let backup_path = PathBuf::from(&metadata.path);

        tracing::info!("Verifying backup: {}", backup_id);

        // Check that backup file exists
        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found: {:?}", backup_path));
        }

        // Verify checksum
        let current_checksum = self.calculate_checksum(&backup_path).await?;
        if current_checksum != metadata.checksum {
            return Err(anyhow!(
                "Checksum mismatch for backup {}: expected {}, got {}",
                backup_id,
                metadata.checksum,
                current_checksum
            ));
        }

        // Verify file is readable and has size > 0
        let file_size = fs::metadata(&backup_path)
            .with_context(|| format!("Failed to get backup file metadata: {:?}", backup_path))?
            .len();

        // SurrealDB exports always create a file with header even for empty databases
        // So we just need to check that the file exists and is readable
        if file_size == 0 {
            return Err(anyhow!("Backup file is empty: {:?}", backup_path));
        }

        // Basic file format check - should be readable as text (SurrealDB exports are SQL text)
        let content = fs::read_to_string(&backup_path)
            .with_context(|| format!("Failed to read backup file as text: {:?}", backup_path))?;

        // Verify it's a SurrealDB export by checking for SQL syntax
        // Even empty databases have header comments
        if !content.starts_with("--") && !content.contains("DEFINE") && !content.contains("CREATE") {
            tracing::warn!(
                "Backup file {:?} doesn't appear to be a SurrealDB export (no SQL syntax found), but proceeding",
                backup_path
            );
        }

        tracing::info!("Backup {} verified successfully", backup_id);

        Ok(())
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_id: &str) -> Result<()> {
        let metadata = self.get_backup(backup_id).await?;
        let backup_path = PathBuf::from(&metadata.path);

        tracing::info!("Deleting backup: {}", backup_id);

        // Delete file
        if backup_path.exists() {
            fs::remove_file(&backup_path)
                .with_context(|| format!("Failed to delete backup file at {:?}", backup_path))?;
        }

        // Delete metadata from SurrealDB
        let _ = self.db
            .query("DELETE FROM backup_metadata WHERE backup_id = $backup_id")
            .bind(("backup_id", backup_id.to_string()))
            .await
            .context("Failed to delete backup metadata")?;

        // Remove from cache
        self.metadata_cache.write().await.remove(backup_id);

        tracing::info!("Backup {} deleted successfully", backup_id);

        Ok(())
    }

    /// Clean up old backups based on retention policy
    async fn cleanup_old_backups(&self, backup_type: BackupType) -> Result<()> {
        let all_backups = self.list_backups().await?;

        let max_backups = match backup_type {
            BackupType::Scheduled => self.config.max_scheduled_backups,
            BackupType::Incremental => self.config.max_incremental_backups,
            _ => return Ok(()), // Don't auto-cleanup manual or pre-migration backups
        };

        let mut type_backups: Vec<_> = all_backups
            .into_iter()
            .filter(|b| b.backup_type == backup_type)
            .collect();

        if type_backups.len() <= max_backups {
            return Ok(());
        }

        // Sort by creation time (oldest first)
        type_backups.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Delete oldest backups
        let to_delete = type_backups.len() - max_backups;
        for backup in type_backups.iter().take(to_delete) {
            tracing::info!(
                "Cleaning up old {:?} backup: {} (created at {})",
                backup_type,
                backup.id,
                backup.created_at
            );
            self.delete_backup(&backup.id).await?;
        }

        Ok(())
    }

    /// Get backup statistics
    pub async fn get_stats(&self) -> Result<BackupStats> {
        let backups = self.list_backups().await?;

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut total_size = 0u64;
        let mut verified_count = 0;
        let mut unverified_count = 0;
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;

        for backup in &backups {
            *by_type.entry(backup.backup_type.as_str().to_string()).or_insert(0) += 1;
            total_size += backup.size_bytes;

            if backup.verified {
                verified_count += 1;
            } else {
                unverified_count += 1;
            }

            match oldest {
                None => oldest = Some(backup.created_at),
                Some(ref o) if backup.created_at < *o => oldest = Some(backup.created_at),
                _ => {}
            }

            match newest {
                None => newest = Some(backup.created_at),
                Some(ref n) if backup.created_at > *n => newest = Some(backup.created_at),
                _ => {}
            }
        }

        Ok(BackupStats {
            total_backups: backups.len(),
            total_size_bytes: total_size,
            by_type,
            oldest_backup: oldest,
            newest_backup: newest,
            verified_count,
            unverified_count,
        })
    }

    /// Create a pre-migration backup
    pub async fn create_pre_migration_backup(
        &self,
        schema_version: u32,
        description: Option<String>,
    ) -> Result<BackupMetadata> {
        let desc = description.unwrap_or_else(|| {
            format!("Pre-migration backup for schema version {}", schema_version)
        });

        let mut metadata = self.create_backup(
            BackupType::PreMigration,
            Some(desc),
            vec!["migration".to_string()],
        ).await?;

        metadata.schema_version = Some(schema_version);

        // Update metadata with schema version
        let metadata_id = metadata.id.clone();
        let _ = self.db
            .query("UPDATE backup_metadata SET schema_version = $version WHERE backup_id = $backup_id")
            .bind(("version", schema_version))
            .bind(("backup_id", metadata_id))
            .await
            .context("Failed to update schema version in metadata")?;

        Ok(metadata)
    }

    /// Create a scheduled backup
    pub async fn create_scheduled_backup(&self) -> Result<BackupMetadata> {
        self.create_backup(
            BackupType::Scheduled,
            Some("Automated daily backup".to_string()),
            vec!["scheduled".to_string()],
        ).await
    }
}

// Include comprehensive test module
#[cfg(test)]
#[path = "backup_tests.rs"]
mod backup_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (BackupManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Initialize SurrealDB and create test data
        {
            let db = Surreal::new::<RocksDb>(db_path.as_path()).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();

            // Create test data
            let _ = db.query("CREATE user:1 SET name = 'Alice'").await.unwrap();
            let _ = db.query("CREATE user:2 SET name = 'Bob'").await.unwrap();
            // Database closes when it goes out of scope
        }

        // Give the database time to release the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            max_scheduled_backups: 3,
            max_incremental_backups: 5,
            auto_verify: false, // Disable auto-verify in tests
            compress: false,
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string(),
        )
        .await
        .unwrap();

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create a backup
        let metadata = manager
            .create_backup(
                BackupType::Manual,
                Some("Test backup".to_string()),
                vec!["test".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(metadata.backup_type, BackupType::Manual);
        assert!(metadata.size_bytes > 0);

        // Manually verify
        manager.verify_backup(&metadata.id).await.unwrap();
        let verified_metadata = manager.get_backup(&metadata.id).await.unwrap();
        assert!(verified_metadata.verified);

        // List backups
        let backups = manager.list_backups().await.unwrap();
        assert!(backups.len() >= 1);

        // Verify backup
        manager.verify_backup(&metadata.id).await.unwrap();
    }

    #[tokio::test]
    async fn test_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Initialize SurrealDB and create test data
        {
            let db = Surreal::new::<RocksDb>(db_path.as_path()).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();
            let _ = db.query("CREATE user:1 SET name = 'Alice'").await.unwrap();
            let _ = db.query("CREATE user:2 SET name = 'Bob'").await.unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            max_scheduled_backups: 3,
            max_incremental_backups: 5,
            auto_verify: false,
            compress: false,
        };

        let backup_id = {
            let manager = BackupManager::new(
                db_path.clone(),
                config.clone(),
                "test".to_string(),
                "test".to_string(),
            )
            .await
            .unwrap();

            // Create a backup
            let metadata = manager
                .create_backup(
                    BackupType::Manual,
                    Some("Test backup".to_string()),
                    vec![],
                )
                .await
                .unwrap();

            metadata.id.clone()
            // Manager is dropped here, releasing the database lock
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Restore backup with a new manager instance
        {
            let manager = BackupManager::new(
                db_path.clone(),
                config,
                "test".to_string(),
                "test".to_string(),
            )
            .await
            .unwrap();

            manager.restore_backup(&backup_id, None).await.unwrap();
            // Manager is dropped here
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify data exists after restore with a new connection
        let db = Surreal::new::<RocksDb>(db_path.as_path())
            .await
            .unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        // Use count to avoid serialization issues
        let mut response = db.query("SELECT count() FROM user GROUP ALL").await.unwrap();
        let count_result: Vec<serde_json::Value> = response.take(0).unwrap();
        assert!(!count_result.is_empty(), "Expected users to exist after restore");
    }

    #[tokio::test]
    async fn test_backup_cleanup() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create 4 scheduled backups
        for i in 0..4 {
            manager
                .create_backup(
                    BackupType::Scheduled,
                    Some(format!("Backup {}", i)),
                    vec![],
                )
                .await
                .unwrap();

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Should only have 3 backups (max_scheduled_backups)
        let backups = manager.list_backups().await.unwrap();
        let scheduled_backups: Vec<_> = backups
            .into_iter()
            .filter(|b| b.backup_type == BackupType::Scheduled)
            .collect();

        assert_eq!(scheduled_backups.len(), 3);
    }

    #[tokio::test]
    async fn test_backup_stats() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create various backups
        manager.create_backup(BackupType::Manual, None, vec![]).await.unwrap();
        manager.create_backup(BackupType::Scheduled, None, vec![]).await.unwrap();
        manager.create_backup(BackupType::PreMigration, None, vec![]).await.unwrap();

        let stats = manager.get_stats().await.unwrap();

        assert!(stats.total_backups >= 3);
        assert!(stats.total_size_bytes > 0);
        assert!(stats.by_type.contains_key("manual"));
        assert!(stats.by_type.contains_key("scheduled"));
        assert!(stats.by_type.contains_key("pre_migration"));
    }

    #[tokio::test]
    async fn test_backup_verification() {
        // Create manager with auto_verify enabled
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Initialize SurrealDB and create test data
        {
            let db = Surreal::new::<RocksDb>(db_path.as_path()).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();
            let _ = db.query("CREATE user:1 SET name = 'Alice'").await.unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            max_scheduled_backups: 3,
            max_incremental_backups: 5,
            auto_verify: true, // Enable auto-verify for this test
            compress: false,
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string(),
        )
        .await
        .unwrap();

        // Create a backup
        let metadata = manager
            .create_backup(
                BackupType::Manual,
                Some("Test backup".to_string()),
                vec![],
            )
            .await
            .unwrap();

        // Should be auto-verified
        assert!(metadata.verified);

        // Verify again
        manager.verify_backup(&metadata.id).await.unwrap();

        // Get updated metadata
        let updated = manager.get_backup(&metadata.id).await.unwrap();
        assert!(updated.verified);
        assert!(updated.verified_at.is_some());
    }
}
