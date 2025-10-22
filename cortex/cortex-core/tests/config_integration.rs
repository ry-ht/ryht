//! Integration tests for the configuration system
//!
//! These tests verify:
//! - File I/O operations
//! - Directory creation and management
//! - Environment variable handling in real scenarios
//! - Configuration migration and updates
//! - Error handling and recovery

use cortex_core::config::{
    GlobalConfig, ENV_CACHE_SIZE_MB, ENV_CONFIG_PATH, ENV_DB_MODE, ENV_DB_URL, ENV_LOG_LEVEL,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to set up a temporary configuration environment
fn setup_temp_env() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    (temp_dir, config_path)
}

/// Helper to set up environment variable and restore it after test
struct EnvGuard {
    key: String,
    old_value: Option<String>,
}

impl EnvGuard {
    fn new(key: &str, value: &str) -> Self {
        let old_value = env::var(key).ok();
        env::set_var(key, value);
        Self {
            key: key.to_string(),
            old_value,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.old_value {
                Some(val) => env::set_var(&self.key, val),
                None => env::remove_var(&self.key),
            }
        }
    }
}

#[tokio::test]
async fn test_create_default_config_file() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Override config path
    let _guard = EnvGuard::new(ENV_CONFIG_PATH, config_path.to_str().unwrap());

    // Load or create default
    let config = GlobalConfig::load_or_create_default().await.unwrap();

    // Verify file was created
    assert!(config_path.exists());

    // Verify content
    assert_eq!(config.general().log_level, "info");
    assert_eq!(config.database().mode, "local");

    // Verify file is valid TOML
    let content = fs::read_to_string(&config_path).unwrap();
    let parsed: GlobalConfig = toml::from_str(&content).unwrap();
    assert_eq!(parsed.general().version, config.general().version);
}

#[tokio::test]
async fn test_load_existing_config() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create initial config
    let mut config = GlobalConfig::default();
    config.general_mut().log_level = "debug".to_string();
    config.database_mut().namespace = "test_ns".to_string();
    config.save_to_path(&config_path).await.unwrap();

    // Load the config
    let loaded_config = GlobalConfig::load_from_path(&config_path).await.unwrap();

    assert_eq!(loaded_config.general().log_level, "debug");
    assert_eq!(loaded_config.database().namespace, "test_ns");
    assert_eq!(loaded_config.pool().max_connections, 10); // Default value preserved
}

#[tokio::test]
async fn test_config_update_preserves_other_fields() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create initial config
    let mut config = GlobalConfig::default();
    config.general_mut().log_level = "info".to_string();
    config.database_mut().namespace = "initial".to_string();
    config.pool_mut().max_connections = 20;
    config.save_to_path(&config_path).await.unwrap();

    // Load and update only one field
    let mut loaded_config = GlobalConfig::load_from_path(&config_path).await.unwrap();
    loaded_config.database_mut().namespace = "updated".to_string();
    loaded_config.save_to_path(&config_path).await.unwrap();

    // Verify only the updated field changed
    let final_config = GlobalConfig::load_from_path(&config_path).await.unwrap();
    assert_eq!(final_config.database().namespace, "updated");
    assert_eq!(final_config.general().log_level, "info");
    assert_eq!(final_config.pool().max_connections, 20);
}

#[tokio::test]
async fn test_atomic_write_on_failure() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create valid initial config
    let config = GlobalConfig::default();
    config.save_to_path(&config_path).await.unwrap();

    let initial_content = fs::read_to_string(&config_path).unwrap();

    // Try to save invalid config
    let mut invalid_config = GlobalConfig::default();
    invalid_config.general_mut().log_level = "invalid_level".to_string();

    let result = invalid_config.save_to_path(&config_path).await;
    assert!(result.is_err());

    // Verify original file is unchanged
    let current_content = fs::read_to_string(&config_path).unwrap();
    assert_eq!(initial_content, current_content);

    // Verify temp file was cleaned up or doesn't exist
    let temp_path = config_path.with_extension("toml.tmp");
    assert!(!temp_path.exists());
}

#[tokio::test]
async fn test_environment_variable_overrides() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create base config
    let config = GlobalConfig::default();
    config.save_to_path(&config_path).await.unwrap();

    // Set environment variables
    let _guard_log = EnvGuard::new(ENV_LOG_LEVEL, "trace");
    let _guard_mode = EnvGuard::new(ENV_DB_MODE, "remote");
    let _guard_url = EnvGuard::new(ENV_DB_URL, "ws://remote.example.com:8000");
    let _guard_cache = EnvGuard::new(ENV_CACHE_SIZE_MB, "2048");

    // Load config (should apply env overrides)
    let loaded_config = GlobalConfig::load_from_path(&config_path).await.unwrap();

    assert_eq!(loaded_config.general().log_level, "trace");
    assert_eq!(loaded_config.database().mode, "remote");
    assert_eq!(
        loaded_config.database().remote_urls,
        vec!["ws://remote.example.com:8000"]
    );
    assert_eq!(loaded_config.cache().memory_size_mb, 2048);
}

#[tokio::test]
async fn test_invalid_environment_variable() {
    let (_temp_dir, config_path) = setup_temp_env();

    let config = GlobalConfig::default();
    config.save_to_path(&config_path).await.unwrap();

    // Set invalid cache size
    let _guard = EnvGuard::new(ENV_CACHE_SIZE_MB, "not_a_number");

    // Should fail to load due to invalid env var
    let result = GlobalConfig::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("cortex");
    let config_path = base_path.join("config.toml");

    let _guard = EnvGuard::new(ENV_CONFIG_PATH, config_path.to_str().unwrap());

    // Ensure directories are created
    GlobalConfig::ensure_directories().await.unwrap();

    // Verify all directories exist
    assert!(base_path.exists());
    assert!(base_path.join("data").exists());
    assert!(base_path.join("data").join("surrealdb").exists());
    assert!(base_path.join("logs").exists());
    assert!(base_path.join("run").exists());
    assert!(base_path.join("cache").exists());
    assert!(base_path.join("workspaces").exists());
}

#[tokio::test]
async fn test_load_or_create_creates_directories() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("cortex");
    let config_path = base_path.join("config.toml");

    let _guard = EnvGuard::new(ENV_CONFIG_PATH, config_path.to_str().unwrap());

    // Load or create should create all directories
    let _config = GlobalConfig::load_or_create_default().await.unwrap();

    // Verify all directories were created
    assert!(base_path.exists());
    assert!(base_path.join("data").exists());
    assert!(base_path.join("data").join("surrealdb").exists());
    assert!(base_path.join("logs").exists());
    assert!(base_path.join("run").exists());
    assert!(base_path.join("cache").exists());
    assert!(base_path.join("workspaces").exists());
}

#[tokio::test]
async fn test_invalid_toml_file() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Write invalid TOML
    fs::write(&config_path, "this is not valid TOML { [ }").unwrap();

    // Should fail to load
    let result = GlobalConfig::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_missing_required_fields() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Write incomplete TOML
    let incomplete_toml = r#"
[general]
version = "0.1.0"
# Missing log_level
    "#;

    fs::write(&config_path, incomplete_toml).unwrap();

    // Should fail to load due to missing required fields
    let result = GlobalConfig::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validation_on_load() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create config with invalid values
    let invalid_toml = r#"
[general]
version = "0.1.0"
log_level = "invalid_level"

[database]
mode = "local"
local_bind = "127.0.0.1:8000"
remote_urls = []
username = "root"
password = "root"
namespace = "cortex"
database = "knowledge"

[pool]
min_connections = 2
max_connections = 10
connection_timeout_ms = 5000
idle_timeout_ms = 300000

[cache]
memory_size_mb = 512
ttl_seconds = 300
redis_url = ""

[vfs]
max_file_size_mb = 100
auto_flush = false
flush_interval_seconds = 60

[ingestion]
parallel_workers = 4
chunk_size = 1000
generate_embeddings = true
embedding_model = "text-embedding-3-small"

[mcp]
server_bind = "127.0.0.1:3000"
cors_enabled = true
max_request_size_mb = 10
    "#;

    fs::write(&config_path, invalid_toml).unwrap();

    // Should fail validation on load
    let result = GlobalConfig::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_access() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create initial config
    let config = GlobalConfig::default();
    config.save_to_path(&config_path).await.unwrap();

    // Spawn multiple tasks that read the config
    let mut handles = vec![];
    for _ in 0..10 {
        let path = config_path.clone();
        let handle = tokio::spawn(async move {
            let config = GlobalConfig::load_from_path(&path).await.unwrap();
            assert_eq!(config.general().log_level, "info");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_config_path_helpers() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().join("cortex");
    let config_path = base_path.join("config.toml");

    let _guard = EnvGuard::new(ENV_CONFIG_PATH, config_path.to_str().unwrap());

    // Test all path helpers
    assert_eq!(
        GlobalConfig::base_dir().unwrap(),
        base_path
    );
    assert_eq!(
        GlobalConfig::config_path().unwrap(),
        config_path
    );
    assert_eq!(
        GlobalConfig::data_dir().unwrap(),
        base_path.join("data")
    );
    assert_eq!(
        GlobalConfig::surrealdb_dir().unwrap(),
        base_path.join("data").join("surrealdb")
    );
    assert_eq!(
        GlobalConfig::logs_dir().unwrap(),
        base_path.join("logs")
    );
    assert_eq!(
        GlobalConfig::run_dir().unwrap(),
        base_path.join("run")
    );
    assert_eq!(
        GlobalConfig::cache_dir().unwrap(),
        base_path.join("cache")
    );
    assert_eq!(
        GlobalConfig::workspaces_dir().unwrap(),
        base_path.join("workspaces")
    );
}

#[tokio::test]
async fn test_toml_pretty_format() {
    let (_temp_dir, config_path) = setup_temp_env();

    let config = GlobalConfig::default();
    config.save_to_path(&config_path).await.unwrap();

    // Read the file and verify it's well-formatted
    let content = fs::read_to_string(&config_path).unwrap();

    // Should contain section headers
    assert!(content.contains("[general]"));
    assert!(content.contains("[database]"));
    assert!(content.contains("[pool]"));
    assert!(content.contains("[cache]"));
    assert!(content.contains("[vfs]"));
    assert!(content.contains("[ingestion]"));
    assert!(content.contains("[mcp]"));

    // Should be valid TOML
    let parsed: GlobalConfig = toml::from_str(&content).unwrap();
    assert_eq!(parsed.general().version, config.general().version);
}

#[tokio::test]
async fn test_remote_database_validation() {
    let (_temp_dir, config_path) = setup_temp_env();

    // Create config with remote mode but no URLs
    let invalid_toml = r#"
[general]
version = "0.1.0"
log_level = "info"

[database]
mode = "remote"
local_bind = "127.0.0.1:8000"
remote_urls = []
username = "root"
password = "root"
namespace = "cortex"
database = "knowledge"

[pool]
min_connections = 2
max_connections = 10
connection_timeout_ms = 5000
idle_timeout_ms = 300000

[cache]
memory_size_mb = 512
ttl_seconds = 300
redis_url = ""

[vfs]
max_file_size_mb = 100
auto_flush = false
flush_interval_seconds = 60

[ingestion]
parallel_workers = 4
chunk_size = 1000
generate_embeddings = true
embedding_model = "text-embedding-3-small"

[mcp]
server_bind = "127.0.0.1:3000"
cors_enabled = true
max_request_size_mb = 10
    "#;

    fs::write(&config_path, invalid_toml).unwrap();

    // Should fail validation
    let result = GlobalConfig::load_from_path(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pool_configuration_validation() {
    let mut config = GlobalConfig::default();

    // Test valid pool config
    config.pool_mut().min_connections = 2;
    config.pool_mut().max_connections = 10;
    assert!(config.validate().is_ok());

    // Test invalid: min > max
    config.pool_mut().min_connections = 15;
    config.pool_mut().max_connections = 10;
    assert!(config.validate().is_err());

    // Test invalid: max = 0
    config.pool_mut().min_connections = 0;
    config.pool_mut().max_connections = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_ingestion_validation() {
    let mut config = GlobalConfig::default();

    // Valid config
    config.ingestion_mut().parallel_workers = 4;
    config.ingestion_mut().chunk_size = 1000;
    assert!(config.validate().is_ok());

    // Invalid: zero workers
    config.ingestion_mut().parallel_workers = 0;
    assert!(config.validate().is_err());

    // Reset
    config.ingestion_mut().parallel_workers = 4;

    // Invalid: zero chunk size
    config.ingestion_mut().chunk_size = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_vfs_validation() {
    let mut config = GlobalConfig::default();

    // Valid config
    config.vfs_mut().max_file_size_mb = 100;
    assert!(config.validate().is_ok());

    // Invalid: zero max file size
    config.vfs_mut().max_file_size_mb = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_mcp_validation() {
    let mut config = GlobalConfig::default();

    // Valid config
    config.mcp_mut().max_request_size_mb = 10;
    assert!(config.validate().is_ok());

    // Invalid: zero max request size
    config.mcp_mut().max_request_size_mb = 0;
    assert!(config.validate().is_err());
}
