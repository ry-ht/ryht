//! Comprehensive tests for database commands and configuration

use cortex_storage::{SurrealDBConfig, SurrealDBManager};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test SurrealDB configuration
fn create_test_db_config() -> (SurrealDBConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:29000".to_string(),
        data_dir: base_path.join("data"),
        log_file: base_path.join("logs").join("surreal.log"),
        pid_file: base_path.join("run").join("surreal.pid"),
        username: "root".to_string(),
        password: "root".to_string(),
        storage_engine: "memory".to_string(),
        allow_guests: false,
        max_retries: 3,
        startup_timeout_secs: 30,
        auto_restart: false,
        health_check_interval_secs: 30,
        max_restart_attempts: 5,
        start_on_boot: false,
    };

    (config, temp_dir)
}

// ============================================================================
// DB Command Setup Tests
// ============================================================================

#[tokio::test]
async fn test_db_start_command_setup() {
    let (config, _temp) = create_test_db_config();

    // Simulate db_start command setup
    let bind_address = Some(config.bind_address.clone());
    let data_dir = Some(config.data_dir.clone());

    let mut db_config = SurrealDBConfig::default();

    if let Some(addr) = bind_address {
        db_config.bind_address = addr;
    }

    if let Some(dir) = data_dir {
        db_config.data_dir = dir;
    }

    // Verify configuration was applied
    assert_eq!(db_config.bind_address, config.bind_address);
    assert_eq!(db_config.data_dir, config.data_dir);

    // Verify manager can be created with this config
    let manager_result = SurrealDBManager::new(db_config).await;
    assert!(manager_result.is_ok(), "Manager should be created successfully");
}

#[tokio::test]
async fn test_db_start_with_default_bind_address() {
    let (_config, _temp) = create_test_db_config();

    // Simulate db_start with no custom bind address
    let bind_address: Option<String> = None;
    let data_dir: Option<PathBuf> = None;

    let mut db_config = SurrealDBConfig::default();

    if let Some(addr) = bind_address {
        db_config.bind_address = addr;
    }

    if let Some(dir) = data_dir {
        db_config.data_dir = dir;
    }

    // Should use default bind address
    assert_eq!(db_config.bind_address, "127.0.0.1:8000");

    let manager_result = SurrealDBManager::new(db_config).await;
    assert!(manager_result.is_ok());
}

#[tokio::test]
async fn test_db_start_with_custom_data_dir() {
    let temp_dir = TempDir::new().unwrap();
    let custom_data_dir = temp_dir.path().join("custom_db_data");

    let bind_address: Option<String> = None;
    let data_dir = Some(custom_data_dir.clone());

    let mut db_config = SurrealDBConfig::default();

    if let Some(addr) = bind_address {
        db_config.bind_address = addr;
    }

    if let Some(dir) = data_dir {
        db_config.data_dir = dir;
    }

    assert_eq!(db_config.data_dir, custom_data_dir);

    let manager_result = SurrealDBManager::new(db_config).await;
    assert!(manager_result.is_ok());
    assert!(custom_data_dir.exists(), "Custom data directory should be created");
}

#[tokio::test]
async fn test_db_stop_command_setup() {
    let (_config, _temp) = create_test_db_config();

    // Simulate db_stop command - uses default config
    let db_config = SurrealDBConfig::default();
    let mut manager = SurrealDBManager::new(db_config).await.unwrap();

    // Stop should succeed even if not running
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stop command should succeed gracefully");
}

#[tokio::test]
async fn test_db_restart_command_setup() {
    let (_config, _temp) = create_test_db_config();

    // Simulate db_restart command - uses default config
    let db_config = SurrealDBConfig::default();
    let mut manager = SurrealDBManager::new(db_config).await.unwrap();

    // Note: We don't actually restart here as it would require SurrealDB to be installed
    // We just verify the manager can be created
    assert_eq!(manager.status(), cortex_storage::ServerStatus::Stopped);
}

#[tokio::test]
async fn test_db_status_command_setup() {
    let (config, _temp) = create_test_db_config();

    // Simulate db_status command
    let db_config = config.clone();
    let manager = SurrealDBManager::new(db_config).await.unwrap();

    // Verify we can get status information
    assert_eq!(manager.connection_url(), format!("http://{}", config.bind_address));
    assert_eq!(manager.config().data_dir, config.data_dir);
    assert_eq!(manager.config().log_file, config.log_file);
    assert_eq!(manager.config().pid_file, config.pid_file);

    let is_running = manager.is_running().await;
    assert!(!is_running, "Server should not be running initially");
}

// ============================================================================
// DB Credentials Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_db_credentials_default_configuration() {
    let db_config = SurrealDBConfig::default();

    assert_eq!(db_config.username, "root", "Default username should be 'root'");
    assert_eq!(db_config.password, "root", "Default password should be 'root'");
}

#[tokio::test]
async fn test_db_credentials_custom_configuration() {
    let (mut config, _temp) = create_test_db_config();

    config.username = "admin".to_string();
    config.password = "admin_password".to_string();

    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    assert_eq!(manager.config().username, "admin");
    assert_eq!(manager.config().password, "admin_password");
}

#[tokio::test]
async fn test_db_credentials_via_builder_pattern() {
    let (config, _temp) = create_test_db_config();

    let config_with_auth = config.with_auth("cli_user".to_string(), "cli_pass".to_string());

    assert_eq!(config_with_auth.username, "cli_user");
    assert_eq!(config_with_auth.password, "cli_pass");

    let manager = SurrealDBManager::new(config_with_auth).await.unwrap();
    assert_eq!(manager.config().username, "cli_user");
    assert_eq!(manager.config().password, "cli_pass");
}

#[tokio::test]
async fn test_db_credentials_validation() {
    let (mut config, _temp) = create_test_db_config();

    // Valid credentials
    assert!(config.validate().is_ok());

    // Invalid: empty username
    config.username = String::new();
    assert!(config.validate().is_err(), "Empty username should fail validation");

    // Reset and test empty password
    config.username = "root".to_string();
    config.password = String::new();
    assert!(config.validate().is_err(), "Empty password should fail validation");
}

// ============================================================================
// Config Loading Tests
// ============================================================================

#[tokio::test]
async fn test_config_loading_with_default_credentials() {
    let db_config = SurrealDBConfig::default();
    let manager = SurrealDBManager::new(db_config.clone()).await.unwrap();

    // Verify default credentials are loaded
    assert_eq!(manager.config().username, "root");
    assert_eq!(manager.config().password, "root");
}

#[tokio::test]
async fn test_config_from_defaults_has_correct_structure() {
    let config = SurrealDBConfig::default();

    // Verify all default fields
    assert!(!config.bind_address.is_empty());
    assert!(!config.username.is_empty());
    assert!(!config.password.is_empty());
    assert!(!config.storage_engine.is_empty());
    assert!(config.max_retries > 0);
    assert!(config.startup_timeout_secs > 0);

    // Verify credentials
    assert_eq!(config.username, "root");
    assert_eq!(config.password, "root");
}

#[tokio::test]
async fn test_config_data_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:29001".to_string(),
        data_dir: base_path.join("db_data"),
        log_file: base_path.join("logs").join("db.log"),
        pid_file: base_path.join("run").join("db.pid"),
        username: "root".to_string(),
        password: "root".to_string(),
        storage_engine: "memory".to_string(),
        allow_guests: false,
        max_retries: 3,
        startup_timeout_secs: 30,
        auto_restart: false,
        health_check_interval_secs: 30,
        max_restart_attempts: 5,
        start_on_boot: false,
    };

    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Verify directory structure is created
    assert!(config.data_dir.exists());
    assert!(config.log_file.parent().unwrap().exists());
    assert!(config.pid_file.parent().unwrap().exists());
}

// ============================================================================
// Credentials Consistency Tests
// ============================================================================

#[tokio::test]
async fn test_credentials_consistency_between_config_and_manager() {
    let (config, _temp) = create_test_db_config();
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Credentials should match exactly
    assert_eq!(
        manager.config().username,
        config.username,
        "Username should match between config and manager"
    );
    assert_eq!(
        manager.config().password,
        config.password,
        "Password should match between config and manager"
    );
}

#[tokio::test]
async fn test_credentials_preserved_through_manager_lifecycle() {
    let (config, _temp) = create_test_db_config();
    let original_username = config.username.clone();
    let original_password = config.password.clone();

    let manager = SurrealDBManager::new(config).await.unwrap();

    // Get server info
    let info = manager.server_info().await;

    // Verify credentials are preserved
    assert_eq!(manager.config().username, original_username);
    assert_eq!(manager.config().password, original_password);
}

#[tokio::test]
async fn test_multiple_managers_with_different_credentials() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();

    let config1 = SurrealDBConfig {
        bind_address: "127.0.0.1:29010".to_string(),
        data_dir: temp_dir1.path().join("data"),
        log_file: temp_dir1.path().join("logs").join("db.log"),
        pid_file: temp_dir1.path().join("run").join("db.pid"),
        username: "user1".to_string(),
        password: "pass1".to_string(),
        storage_engine: "memory".to_string(),
        allow_guests: false,
        max_retries: 3,
        startup_timeout_secs: 30,
        auto_restart: false,
        health_check_interval_secs: 30,
        max_restart_attempts: 5,
        start_on_boot: false,
    };

    let config2 = SurrealDBConfig {
        bind_address: "127.0.0.1:29011".to_string(),
        data_dir: temp_dir2.path().join("data"),
        log_file: temp_dir2.path().join("logs").join("db.log"),
        pid_file: temp_dir2.path().join("run").join("db.pid"),
        username: "user2".to_string(),
        password: "pass2".to_string(),
        storage_engine: "memory".to_string(),
        allow_guests: false,
        max_retries: 3,
        startup_timeout_secs: 30,
        auto_restart: false,
        health_check_interval_secs: 30,
        max_restart_attempts: 5,
        start_on_boot: false,
    };

    let manager1 = SurrealDBManager::new(config1).await.unwrap();
    let manager2 = SurrealDBManager::new(config2).await.unwrap();

    // Each manager should have its own credentials
    assert_eq!(manager1.config().username, "user1");
    assert_eq!(manager1.config().password, "pass1");

    assert_eq!(manager2.config().username, "user2");
    assert_eq!(manager2.config().password, "pass2");

    // They should not interfere with each other
    assert_ne!(manager1.config().username, manager2.config().username);
    assert_ne!(manager1.config().password, manager2.config().password);
}

// ============================================================================
// Configuration Options Tests
// ============================================================================

#[tokio::test]
async fn test_db_config_with_all_options() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:29020".to_string(),
        data_dir: base_path.join("data"),
        log_file: base_path.join("logs").join("surreal.log"),
        pid_file: base_path.join("run").join("surreal.pid"),
        username: "test_user".to_string(),
        password: "test_password".to_string(),
        storage_engine: "rocksdb".to_string(),
        allow_guests: true,
        max_retries: 5,
        startup_timeout_secs: 60,
        auto_restart: true,
        health_check_interval_secs: 15,
        max_restart_attempts: 10,
        start_on_boot: true,
    };

    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Verify all options are preserved
    assert_eq!(manager.config().bind_address, config.bind_address);
    assert_eq!(manager.config().username, config.username);
    assert_eq!(manager.config().password, config.password);
    assert_eq!(manager.config().storage_engine, config.storage_engine);
    assert_eq!(manager.config().allow_guests, config.allow_guests);
    assert_eq!(manager.config().max_retries, config.max_retries);
    assert_eq!(manager.config().startup_timeout_secs, config.startup_timeout_secs);
    assert_eq!(manager.config().auto_restart, config.auto_restart);
    assert_eq!(manager.config().health_check_interval_secs, config.health_check_interval_secs);
    assert_eq!(manager.config().max_restart_attempts, config.max_restart_attempts);
    assert_eq!(manager.config().start_on_boot, config.start_on_boot);
}

#[tokio::test]
async fn test_db_config_storage_engine_options() {
    let (mut config, _temp) = create_test_db_config();

    // Test memory engine
    config.storage_engine = "memory".to_string();
    assert!(SurrealDBManager::new(config.clone()).await.is_ok());

    // Test rocksdb engine
    config.storage_engine = "rocksdb".to_string();
    assert!(SurrealDBManager::new(config.clone()).await.is_ok());
}

#[tokio::test]
async fn test_db_connection_url_construction() {
    let (config, _temp) = create_test_db_config();
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    let url = manager.connection_url();
    assert_eq!(url, format!("http://{}", config.bind_address));
    assert!(url.starts_with("http://"));
    assert!(url.contains(&config.bind_address));
}

#[tokio::test]
async fn test_db_manager_initial_state() {
    let (config, _temp) = create_test_db_config();
    let manager = SurrealDBManager::new(config).await.unwrap();

    // Verify initial state
    assert_eq!(manager.status(), cortex_storage::ServerStatus::Stopped);
    assert!(!manager.is_running().await);
    assert_eq!(manager.restart_count(), 0);
    assert!(manager.binary_path().is_none());
}

#[tokio::test]
async fn test_db_server_info_initial_values() {
    let (config, _temp) = create_test_db_config();
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    let info = manager.server_info().await;

    assert_eq!(info.status, cortex_storage::ServerStatus::Stopped);
    assert_eq!(info.bind_address, config.bind_address);
    assert_eq!(info.data_dir, config.data_dir);
    assert_eq!(info.storage_engine, config.storage_engine);
    assert!(!info.is_running);
    assert_eq!(info.restart_count, 0);
    assert!(info.binary_path.is_none());
    assert!(info.pid.is_none());
}
