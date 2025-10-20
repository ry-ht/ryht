// Comprehensive tests for the backup system
//
// These tests verify all backup functionality including:
// - Backup creation and metadata
// - Restore functionality
// - Verification logic
// - Cleanup/retention policies
// - Checksum calculation
// - Error handling

#[cfg(test)]
mod tests {
    use crate::storage::{BackupConfig, BackupManager, BackupType};
    use surrealdb::engine::local::RocksDb;
    use surrealdb::Surreal;
    use tempfile::TempDir;

    /// Helper: Create a test database with sample data
    async fn create_test_db(path: &std::path::Path, num_keys: usize) {
        {
            let db = Surreal::new::<RocksDb>(path).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();
            for i in 0..num_keys {
                let _ = db.query(format!("CREATE item:{} SET value = 'value{}'", i, i))
                    .await
                    .unwrap();
            }
            // Database closes when it goes out of scope
        }
        // Give the database time to release the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Create test database
        create_test_db(&db_path, 100).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            max_scheduled_backups: 3,
            max_incremental_backups: 5,
            auto_verify: true,
            compress: false,
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create a manual backup
        let metadata = manager.create_backup(
            BackupType::Manual,
            Some("Test backup".to_string()),
            vec!["test".to_string()],
        ).await.unwrap();

        assert_eq!(metadata.backup_type, BackupType::Manual);
        assert!(metadata.verified); // Auto-verify is enabled
        assert!(metadata.size_bytes > 0);
        assert_eq!(metadata.file_count, 1); // SurrealDB creates single .sql file
        assert_eq!(metadata.tags, vec!["test"]);
        assert_eq!(metadata.description, Some("Test backup".to_string()));
    }

    #[tokio::test]
    async fn test_backup_list() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 50).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create multiple backups
        manager.create_backup(BackupType::Manual, None, vec![]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        manager.create_backup(BackupType::Scheduled, None, vec![]).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        manager.create_backup(BackupType::PreMigration, None, vec![]).await.unwrap();

        // List all backups
        let backups = manager.list_backups().await.unwrap();
        assert_eq!(backups.len(), 3);

        // Verify sorted by newest first
        assert!(backups[0].created_at >= backups[1].created_at);
        assert!(backups[1].created_at >= backups[2].created_at);
    }

    #[tokio::test]
    async fn test_backup_restore() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Create test database with initial data
        create_test_db(&db_path, 50).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let backup_id = {
            let manager = BackupManager::new(
                db_path.clone(),
                config.clone(),
                "test".to_string(),
                "test".to_string()
            ).await.unwrap();

            // Create a backup
            let metadata = manager.create_backup(
                BackupType::Manual,
                Some("Test restore".to_string()),
                vec![],
            ).await.unwrap();

            metadata.id.clone()
            // Manager dropped here
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Restore with new manager instance
        {
            let manager = BackupManager::new(
                db_path.clone(),
                config,
                "test".to_string(),
                "test".to_string()
            ).await.unwrap();

            manager.restore_backup(&backup_id, None).await.unwrap();
            // Manager dropped here
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify data still exists with new connection
        let db = Surreal::new::<RocksDb>(db_path.as_path()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        // Use count to avoid serialization issues
        let mut response = db.query("SELECT count() FROM item GROUP ALL").await.unwrap();
        let count_result: Vec<serde_json::Value> = response.take(0).unwrap();
        assert!(!count_result.is_empty(), "Expected items to exist after restore");
    }

    #[tokio::test]
    async fn test_backup_verification() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 30).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            auto_verify: false, // Disable auto-verify to test manual verification
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create a backup (unverified)
        let metadata = manager.create_backup(
            BackupType::Manual,
            None,
            vec![],
        ).await.unwrap();

        assert!(!metadata.verified);

        // Manually verify
        manager.verify_backup(&metadata.id).await.unwrap();

        // Check verification status
        let updated_metadata = manager.get_backup(&metadata.id).await.unwrap();
        assert!(updated_metadata.verified);
        assert!(updated_metadata.verified_at.is_some());
    }

    #[tokio::test]
    async fn test_backup_cleanup_scheduled() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 20).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            max_scheduled_backups: 2, // Keep only 2 scheduled backups
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create 4 scheduled backups
        for i in 0..4 {
            manager.create_backup(
                BackupType::Scheduled,
                Some(format!("Backup {}", i)),
                vec![],
            ).await.unwrap();

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Should only have 2 backups (max_scheduled_backups)
        let backups = manager.list_backups().await.unwrap();
        let scheduled_backups: Vec<_> = backups
            .into_iter()
            .filter(|b| b.backup_type == BackupType::Scheduled)
            .collect();

        assert_eq!(scheduled_backups.len(), 2);

        // Verify newest backups are kept
        assert!(scheduled_backups[0].description.as_ref().unwrap().contains("3"));
        assert!(scheduled_backups[1].description.as_ref().unwrap().contains("2"));
    }

    #[tokio::test]
    async fn test_backup_delete() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 25).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create a backup
        let metadata = manager.create_backup(
            BackupType::Manual,
            None,
            vec![],
        ).await.unwrap();

        // Verify it exists
        let backups_before = manager.list_backups().await.unwrap();
        assert_eq!(backups_before.len(), 1);

        // Delete the backup
        manager.delete_backup(&metadata.id).await.unwrap();

        // Verify it's gone
        let backups_after = manager.list_backups().await.unwrap();
        assert_eq!(backups_after.len(), 0);
    }

    #[tokio::test]
    async fn test_backup_stats() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 40).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create various backups
        manager.create_backup(BackupType::Manual, None, vec![]).await.unwrap();
        manager.create_backup(BackupType::Scheduled, None, vec![]).await.unwrap();
        manager.create_backup(BackupType::PreMigration, None, vec![]).await.unwrap();

        let stats = manager.get_stats().await.unwrap();

        assert_eq!(stats.total_backups, 3);
        assert!(stats.total_size_bytes > 0);
        assert_eq!(stats.by_type.get("manual").unwrap(), &1);
        assert_eq!(stats.by_type.get("scheduled").unwrap(), &1);
        assert_eq!(stats.by_type.get("pre_migration").unwrap(), &1);
        assert_eq!(stats.verified_count, 3); // Auto-verify is on by default
        assert_eq!(stats.unverified_count, 0);
        assert!(stats.oldest_backup.is_some());
        assert!(stats.newest_backup.is_some());
    }

    #[tokio::test]
    async fn test_checksum_validation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 15).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create a backup
        let metadata = manager.create_backup(
            BackupType::Manual,
            None,
            vec![],
        ).await.unwrap();

        // Checksum should be non-empty
        assert!(!metadata.checksum.is_empty());

        // Verification should succeed (checksum matches)
        manager.verify_backup(&metadata.id).await.unwrap();
    }

    #[tokio::test]
    async fn test_pre_migration_backup() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 35).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create pre-migration backup
        let metadata = manager.create_pre_migration_backup(
            2,
            Some("Migration v2 -> v3".to_string()),
        ).await.unwrap();

        assert_eq!(metadata.backup_type, BackupType::PreMigration);
        assert_eq!(metadata.schema_version, Some(2));
        assert!(metadata.description.as_ref().unwrap().contains("Migration"));
        assert!(metadata.tags.contains(&"migration".to_string()));
    }

    #[tokio::test]
    async fn test_scheduled_backup() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 28).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create scheduled backup
        let metadata = manager.create_scheduled_backup().await.unwrap();

        assert_eq!(metadata.backup_type, BackupType::Scheduled);
        assert!(metadata.description.as_ref().unwrap().contains("daily"));
        assert!(metadata.tags.contains(&"scheduled".to_string()));
    }

    #[tokio::test]
    async fn test_get_backup_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 22).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Create a backup
        let original_metadata = manager.create_backup(
            BackupType::Manual,
            Some("Test metadata".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
        ).await.unwrap();

        // Retrieve metadata
        let retrieved_metadata = manager.get_backup(&original_metadata.id).await.unwrap();

        assert_eq!(retrieved_metadata.id, original_metadata.id);
        assert_eq!(retrieved_metadata.backup_type, BackupType::Manual);
        assert_eq!(retrieved_metadata.description, Some("Test metadata".to_string()));
        assert_eq!(retrieved_metadata.tags, vec!["tag1", "tag2"]);
    }

    #[tokio::test]
    async fn test_backup_with_empty_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        // Create empty database
        {
            let db = Surreal::new::<RocksDb>(db_path.as_path()).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();
            // Close database
        }

        // Give the database time to release the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Should still create a valid backup
        let metadata = manager.create_backup(
            BackupType::Manual,
            None,
            vec![],
        ).await.unwrap();

        assert!(metadata.size_bytes > 0); // SurrealDB has metadata even when empty
        assert!(metadata.verified);
    }

    #[tokio::test]
    async fn test_error_invalid_backup_id() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("db");
        let backup_dir = temp_dir.path().join("backups");

        create_test_db(&db_path, 12).await;

        let config = BackupConfig {
            backup_dir: backup_dir.clone(),
            ..Default::default()
        };

        let manager = BackupManager::new(
            db_path.clone(),
            config,
            "test".to_string(),
            "test".to_string()
        ).await.unwrap();

        // Try to verify non-existent backup
        let result = manager.verify_backup("nonexistent_backup").await;
        assert!(result.is_err());

        // Try to restore non-existent backup
        let result = manager.restore_backup("nonexistent_backup", None).await;
        assert!(result.is_err());

        // Try to get non-existent backup
        let result = manager.get_backup("nonexistent_backup").await;
        assert!(result.is_err());
    }
}
