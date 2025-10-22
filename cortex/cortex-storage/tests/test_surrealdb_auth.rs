//! Comprehensive tests for SurrealDB authentication and lifecycle management

use cortex_storage::{
    ConnectionManager, Credentials, DatabaseConfig, PoolConfig, PoolConnectionMode,
    SurrealDBConfig, SurrealDBManager,
};
use tempfile::TempDir;

/// Helper to create a test configuration with default credentials
fn create_test_config_with_defaults() -> (SurrealDBConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:19001".to_string(),
        data_dir: base_path.join("data"),
        log_file: base_path.join("logs").join("test.log"),
        pid_file: base_path.join("test.pid"),
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

/// Helper to create a test configuration with custom credentials
fn create_test_config_with_custom_creds(username: &str, password: &str) -> (SurrealDBConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:19002".to_string(),
        data_dir: base_path.join("data"),
        log_file: base_path.join("logs").join("test.log"),
        pid_file: base_path.join("test.pid"),
        username: username.to_string(),
        password: password.to_string(),
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
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_default_credentials_are_root() {
    let config = SurrealDBConfig::default();

    assert_eq!(config.username, "root", "Default username should be 'root'");
    assert_eq!(config.password, "root", "Default password should be 'root'");
}

#[tokio::test]
async fn test_manager_respects_default_credentials() {
    let (config, _temp) = create_test_config_with_defaults();
    let manager = SurrealDBManager::new(config).await.unwrap();

    assert_eq!(manager.config().username, "root");
    assert_eq!(manager.config().password, "root");
}

#[tokio::test]
async fn test_credentials_match_between_manager_and_config() {
    let (config, _temp) = create_test_config_with_custom_creds("testuser", "testpass");
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    assert_eq!(
        manager.config().username,
        config.username,
        "Manager username should match config"
    );
    assert_eq!(
        manager.config().password,
        config.password,
        "Manager password should match config"
    );
}

#[tokio::test]
async fn test_custom_credentials_configuration() {
    let (config, _temp) = create_test_config_with_custom_creds("admin", "admin123");

    assert_eq!(config.username, "admin");
    assert_eq!(config.password, "admin123");

    let manager = SurrealDBManager::new(config).await.unwrap();
    assert_eq!(manager.config().username, "admin");
    assert_eq!(manager.config().password, "admin123");
}

#[tokio::test]
async fn test_credentials_builder_pattern() {
    let (config, _temp) = create_test_config_with_defaults();

    let config_with_auth = config.with_auth("custom_user".to_string(), "custom_pass".to_string());

    assert_eq!(config_with_auth.username, "custom_user");
    assert_eq!(config_with_auth.password, "custom_pass");
}

#[tokio::test]
async fn test_empty_credentials_validation_fails() {
    let (mut config, _temp) = create_test_config_with_defaults();

    // Empty username should fail validation
    config.username = String::new();
    assert!(config.validate().is_err(), "Empty username should fail validation");

    // Reset username, test empty password
    config.username = "root".to_string();
    config.password = String::new();
    assert!(config.validate().is_err(), "Empty password should fail validation");
}

#[tokio::test]
async fn test_manager_creation_fails_with_invalid_credentials() {
    let (mut config, _temp) = create_test_config_with_defaults();
    config.username = String::new();

    let result = SurrealDBManager::new(config).await;
    assert!(result.is_err(), "Manager creation should fail with empty username");
}

// ============================================================================
// Lifecycle Management Tests
// ============================================================================

#[tokio::test]
async fn test_manager_starts_in_stopped_state() {
    let (config, _temp) = create_test_config_with_defaults();
    let manager = SurrealDBManager::new(config).await.unwrap();

    assert_eq!(
        manager.status(),
        cortex_storage::ServerStatus::Stopped,
        "Manager should start in Stopped state"
    );
}

#[tokio::test]
async fn test_manager_is_not_running_initially() {
    let (config, _temp) = create_test_config_with_defaults();
    let manager = SurrealDBManager::new(config).await.unwrap();

    assert!(!manager.is_running().await, "Manager should not be running initially");
}

#[tokio::test]
async fn test_manager_graceful_stop_when_not_running() {
    let (config, _temp) = create_test_config_with_defaults();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Stopping a non-running server should succeed gracefully
    let result = manager.stop().await;
    assert!(result.is_ok(), "Stopping a non-running server should succeed gracefully");
}

#[tokio::test]
async fn test_manager_creates_required_directories() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:19003".to_string(),
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

    // Verify directories don't exist
    assert!(!config.data_dir.exists());
    assert!(!config.log_file.parent().unwrap().exists());
    assert!(!config.pid_file.parent().unwrap().exists());

    // Create manager
    let _manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Verify directories were created
    assert!(config.data_dir.exists(), "Data directory should be created");
    assert!(config.log_file.parent().unwrap().exists(), "Log directory should be created");
    assert!(config.pid_file.parent().unwrap().exists(), "PID directory should be created");
}

#[tokio::test]
async fn test_server_info_reflects_initial_state() {
    let (config, _temp) = create_test_config_with_defaults();
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    let info = manager.server_info().await;

    assert_eq!(info.status, cortex_storage::ServerStatus::Stopped);
    assert_eq!(info.bind_address, config.bind_address);
    assert_eq!(info.data_dir, config.data_dir);
    assert_eq!(info.storage_engine, config.storage_engine);
    assert!(!info.is_running);
    assert_eq!(info.restart_count, 0);
    assert!(info.pid.is_none());
}

#[tokio::test]
async fn test_connection_url_format() {
    let (config, _temp) = create_test_config_with_defaults();
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    let expected_url = format!("http://{}", config.bind_address);
    assert_eq!(manager.connection_url(), expected_url);
}

// ============================================================================
// Connection Tests with Authentication
// ============================================================================

#[tokio::test]
async fn test_database_config_with_correct_credentials() {
    let (config, _temp) = create_test_config_with_defaults();

    let db_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: format!("http://{}", config.bind_address),
        },
        credentials: Credentials {
            username: Some(config.username.clone()),
            password: Some(config.password.clone()),
        },
        pool_config: PoolConfig {
            max_connections: 5,
            ..Default::default()
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    // Verify credentials are properly configured
    assert_eq!(db_config.credentials.username, Some("root".to_string()));
    assert_eq!(db_config.credentials.password, Some("root".to_string()));
}

#[tokio::test]
async fn test_database_config_with_custom_credentials() {
    let username = "custom_user";
    let password = "secure_password";

    let credentials = Credentials {
        username: Some(username.to_string()),
        password: Some(password.to_string()),
    };

    assert_eq!(credentials.username, Some(username.to_string()));
    assert_eq!(credentials.password, Some(password.to_string()));
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed and running
async fn test_connection_with_correct_credentials_succeeds() {
    let (config, _temp) = create_test_config_with_defaults();
    let mut manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Start the server
    manager.start().await.unwrap();

    // Create connection with matching credentials
    let db_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: manager.connection_url(),
        },
        credentials: Credentials {
            username: Some(config.username.clone()),
            password: Some(config.password.clone()),
        },
        pool_config: PoolConfig {
            max_connections: 5,
            ..Default::default()
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    let result = ConnectionManager::new(db_config).await;

    // Clean up
    manager.stop().await.ok();

    assert!(result.is_ok(), "Connection with correct credentials should succeed");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed and running
async fn test_connection_with_wrong_credentials_fails() {
    let (config, _temp) = create_test_config_with_defaults();
    let mut manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Start the server with root/root credentials
    manager.start().await.unwrap();

    // Try to connect with wrong credentials
    let db_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: manager.connection_url(),
        },
        credentials: Credentials {
            username: Some("wrong_user".to_string()),
            password: Some("wrong_password".to_string()),
        },
        pool_config: PoolConfig {
            max_connections: 5,
            ..Default::default()
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    let result = ConnectionManager::new(db_config).await;

    // Clean up
    manager.stop().await.ok();

    assert!(result.is_err(), "Connection with wrong credentials should fail");
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[tokio::test]
async fn test_config_validation_with_valid_credentials() {
    let (config, _temp) = create_test_config_with_defaults();

    let result = config.validate();
    assert!(result.is_ok(), "Valid config should pass validation");
}

#[tokio::test]
async fn test_config_validation_comprehensive() {
    let (mut config, _temp) = create_test_config_with_defaults();

    // Test all validation scenarios

    // 1. Empty bind address
    config.bind_address = String::new();
    assert!(config.validate().is_err(), "Empty bind address should fail");
    config.bind_address = "127.0.0.1:19001".to_string();

    // 2. Empty username
    config.username = String::new();
    assert!(config.validate().is_err(), "Empty username should fail");
    config.username = "root".to_string();

    // 3. Empty password
    config.password = String::new();
    assert!(config.validate().is_err(), "Empty password should fail");
    config.password = "root".to_string();

    // 4. Zero max_retries
    config.max_retries = 0;
    assert!(config.validate().is_err(), "Zero max_retries should fail");
    config.max_retries = 3;

    // 5. Valid config
    assert!(config.validate().is_ok(), "Valid config should pass");
}

#[tokio::test]
async fn test_credentials_consistency_across_lifecycle() {
    let (config, _temp) = create_test_config_with_custom_creds("lifecycle_user", "lifecycle_pass");
    let manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Check credentials are preserved
    assert_eq!(manager.config().username, "lifecycle_user");
    assert_eq!(manager.config().password, "lifecycle_pass");

    // Get server info
    let _info = manager.server_info().await;

    // Credentials should match original config
    assert_eq!(manager.config().username, config.username);
    assert_eq!(manager.config().password, config.password);
}
