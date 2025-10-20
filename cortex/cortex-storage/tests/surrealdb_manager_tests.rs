//! Integration tests for SurrealDB Manager

use cortex_storage::{SurrealDBConfig, SurrealDBManager, ServerStatus};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper to create a test configuration
fn create_test_config() -> (SurrealDBConfig, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path().to_path_buf();

    let config = SurrealDBConfig {
        bind_address: "127.0.0.1:19000".to_string(), // Use a different port for testing
        data_dir: base_path.join("data"),
        log_file: base_path.join("logs").join("test.log"),
        pid_file: base_path.join("test.pid"),
        username: "test".to_string(),
        password: "test".to_string(),
        storage_engine: "memory".to_string(), // Use memory for faster tests
        allow_guests: true,
        max_retries: 3,
        startup_timeout_secs: 30,
        auto_restart: false, // Disable for tests
        health_check_interval_secs: 30,
        max_restart_attempts: 5,
        start_on_boot: false,
    };

    (config, temp_dir)
}

#[tokio::test]
async fn test_manager_creation() {
    let (config, _temp) = create_test_config();
    let result = SurrealDBManager::new(config).await;
    assert!(result.is_ok(), "Failed to create manager: {:?}", result.err());
}

#[tokio::test]
async fn test_manager_with_invalid_config() {
    let (mut config, _temp) = create_test_config();
    config.username = String::new();

    let result = SurrealDBManager::new(config).await;
    assert!(result.is_err(), "Should fail with empty username");
}

#[tokio::test]
async fn test_directory_creation() {
    let (config, _temp) = create_test_config();

    // Ensure directories don't exist yet
    assert!(!config.data_dir.exists());

    let _manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // Now directories should exist
    assert!(config.data_dir.exists());
    assert!(config.log_file.parent().unwrap().exists());
    assert!(config.pid_file.parent().unwrap().exists());
}

#[tokio::test]
async fn test_connection_url() {
    let (config, _temp) = create_test_config();
    let manager = SurrealDBManager::new(config).await.unwrap();

    let url = manager.connection_url();
    assert_eq!(url, "http://127.0.0.1:19000");
}

#[tokio::test]
async fn test_initial_status() {
    let (config, _temp) = create_test_config();
    let manager = SurrealDBManager::new(config).await.unwrap();

    assert_eq!(manager.status(), ServerStatus::Stopped);
    assert!(!manager.is_running().await);
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_find_binary() {
    let result = SurrealDBManager::find_surreal_binary().await;

    match result {
        Ok(path) => {
            println!("Found SurrealDB at: {:?}", path);
            assert!(path.exists());
        }
        Err(e) => {
            println!("SurrealDB not found (expected if not installed): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_start_stop_server() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Start the server
    println!("Starting server...");
    let start_result = manager.start().await;
    assert!(start_result.is_ok(), "Failed to start server: {:?}", start_result.err());

    // Give it a moment to fully initialize
    sleep(Duration::from_secs(2)).await;

    // Check status
    assert!(manager.is_running().await, "Server should be running");
    assert_eq!(manager.status(), ServerStatus::Running);

    // Health check
    let health_result = manager.health_check().await;
    assert!(health_result.is_ok(), "Health check failed: {:?}", health_result.err());

    // Stop the server
    println!("Stopping server...");
    let stop_result = manager.stop().await;
    assert!(stop_result.is_ok(), "Failed to stop server: {:?}", stop_result.err());

    // Give it a moment to shut down
    sleep(Duration::from_secs(1)).await;

    // Check status
    assert!(!manager.is_running().await, "Server should be stopped");
    assert_eq!(manager.status(), ServerStatus::Stopped);
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_restart_server() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Start the server
    println!("Starting server...");
    manager.start().await.expect("Failed to start server");
    sleep(Duration::from_secs(2)).await;

    assert!(manager.is_running().await);

    // Restart the server
    println!("Restarting server...");
    let restart_result = manager.restart().await;
    assert!(restart_result.is_ok(), "Failed to restart server: {:?}", restart_result.err());

    sleep(Duration::from_secs(2)).await;

    // Should still be running
    assert!(manager.is_running().await, "Server should be running after restart");
    assert_eq!(manager.status(), ServerStatus::Running);

    // Clean up
    manager.stop().await.expect("Failed to stop server");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_double_start() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Start the server
    manager.start().await.expect("Failed to start server");
    sleep(Duration::from_secs(2)).await;

    // Try to start again - should be idempotent
    let second_start = manager.start().await;
    assert!(second_start.is_ok(), "Second start should not fail");

    // Clean up
    manager.stop().await.expect("Failed to stop server");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_wait_for_ready() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Start the server
    manager.start().await.expect("Failed to start server");

    // Wait for ready with timeout
    let wait_result = manager.wait_for_ready(Duration::from_secs(30)).await;
    assert!(wait_result.is_ok(), "Server did not become ready: {:?}", wait_result.err());

    // Clean up
    manager.stop().await.expect("Failed to stop server");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_health_check_when_stopped() {
    let (config, _temp) = create_test_config();
    let manager = SurrealDBManager::new(config).await.unwrap();

    // Health check should fail when server is stopped
    let health_result = manager.health_check().await;
    assert!(health_result.is_err(), "Health check should fail when server is stopped");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_pid_file_management() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config.clone()).await.unwrap();

    // PID file should not exist initially
    assert!(!config.pid_file.exists());

    // Start the server
    manager.start().await.expect("Failed to start server");
    sleep(Duration::from_secs(2)).await;

    // PID file should exist now
    assert!(config.pid_file.exists(), "PID file should exist after start");

    // Read PID
    let pid_str = std::fs::read_to_string(&config.pid_file).unwrap();
    let pid: u32 = pid_str.trim().parse().unwrap();
    assert!(pid > 0, "PID should be greater than 0");

    // Stop the server
    manager.stop().await.expect("Failed to stop server");
    sleep(Duration::from_secs(1)).await;

    // PID file should be cleaned up
    assert!(!config.pid_file.exists(), "PID file should be removed after stop");
}

#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_multiple_managers_same_config() {
    let (config, _temp) = create_test_config();

    let mut manager1 = SurrealDBManager::new(config.clone()).await.unwrap();
    let manager2 = SurrealDBManager::new(config.clone()).await.unwrap();

    // Start with first manager
    manager1.start().await.expect("Failed to start server");
    sleep(Duration::from_secs(2)).await;

    // Second manager should also see it as running
    assert!(manager2.is_running().await);

    // Clean up
    manager1.stop().await.expect("Failed to stop server");
}

#[tokio::test]
async fn test_config_builder_pattern() {
    let (mut config, _temp) = create_test_config();

    config = config
        .with_auth("custom_user".to_string(), "custom_pass".to_string())
        .with_storage_engine("rocksdb".to_string())
        .with_allow_guests(false);

    assert_eq!(config.username, "custom_user");
    assert_eq!(config.password, "custom_pass");
    assert_eq!(config.storage_engine, "rocksdb");
    assert!(!config.allow_guests);
}

#[tokio::test]
async fn test_config_validation_empty_namespace() {
    let (mut config, _temp) = create_test_config();
    config.username = "".to_string();

    let result = config.validate();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_validation_zero_retries() {
    let (mut config, _temp) = create_test_config();
    config.max_retries = 0;

    let result = config.validate();
    assert!(result.is_err());
}

// Benchmark-style test (not a real benchmark, just measures timing)
#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_startup_time() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    let start = std::time::Instant::now();
    manager.start().await.expect("Failed to start server");
    let startup_time = start.elapsed();

    println!("Server startup time: {:?}", startup_time);

    // Startup should be reasonably fast (under 10 seconds for memory backend)
    assert!(startup_time.as_secs() < 10, "Startup took too long: {:?}", startup_time);

    // Clean up
    manager.stop().await.expect("Failed to stop server");
}

// Test concurrent operations
#[tokio::test]
#[ignore] // Only run when SurrealDB is installed
async fn test_concurrent_health_checks() {
    let (config, _temp) = create_test_config();
    let mut manager = SurrealDBManager::new(config).await.unwrap();

    // Start the server
    manager.start().await.expect("Failed to start server");
    sleep(Duration::from_secs(2)).await;

    // Perform multiple health checks concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let url = manager.connection_url();
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();
            let result = client.get(&format!("{}/health", url)).send().await;
            println!("Health check {}: {:?}", i, result.is_ok());
            result.is_ok()
        });
        handles.push(handle);
    }

    // Wait for all checks to complete
    let results = futures::future::join_all(handles).await;

    // All should succeed
    for result in results {
        assert!(result.unwrap(), "Health check failed");
    }

    // Clean up
    manager.stop().await.expect("Failed to stop server");
}
