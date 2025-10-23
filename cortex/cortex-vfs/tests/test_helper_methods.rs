//! Test the ergonomic helper methods for VFS

use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy,
};
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn create_test_vfs() -> (Arc<VirtualFileSystem>, Arc<ConnectionManager>) {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 0,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(30)),
            max_lifetime: Some(Duration::from_secs(60)),
            retry_policy: RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: false,
            recycle_after_uses: Some(10000),
            shutdown_grace_period: Duration::from_secs(30),
        },
        namespace: format!("test_{}", Uuid::new_v4()),
        database: "test".to_string(),
    };

    let storage = Arc::new(ConnectionManager::new(config).await.unwrap());
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

    (vfs, storage)
}

#[tokio::test]
async fn test_create_file_helper() {
    // Setup
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Create a file
    let path = VirtualPath::new("test.rs").unwrap();
    let content = b"fn main() {}";

    let result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(result.is_ok(), "create_file should succeed");

    let vnode = result.unwrap();
    assert_eq!(vnode.workspace_id, workspace_id);
    assert_eq!(vnode.path, path);
    assert_eq!(vnode.size_bytes, content.len());
    assert!(vnode.is_file());

    // Verify we can read it back
    let read_content = vfs.read_file(&workspace_id, &path).await;
    assert!(read_content.is_ok());
    assert_eq!(read_content.unwrap(), content);

    // Try to create again - should fail
    let duplicate_result = vfs.create_file(&workspace_id, &path, content).await;
    assert!(duplicate_result.is_err(), "creating duplicate file should fail");
}

#[tokio::test]
async fn test_get_file_helper() {
    // Setup
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Create a file using write_file
    let path = VirtualPath::new("src/lib.rs").unwrap();
    let content = b"pub fn hello() {}";

    vfs.write_file(&workspace_id, &path, content).await.unwrap();

    // Get it using the helper
    let result = vfs.get_file(&workspace_id, &path).await;
    assert!(result.is_ok(), "get_file should succeed");

    let vnode = result.unwrap();
    assert_eq!(vnode.workspace_id, workspace_id);
    assert_eq!(vnode.path, path);
    assert!(vnode.is_file());

    // Try to get non-existent file
    let missing_path = VirtualPath::new("missing.rs").unwrap();
    let missing_result = vfs.get_file(&workspace_id, &missing_path).await;
    assert!(missing_result.is_err(), "getting non-existent file should fail");
}

#[tokio::test]
async fn test_update_file_helper() {
    // Setup
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Create initial file
    let path = VirtualPath::new("config.rs").unwrap();
    let initial_content = b"const VERSION: &str = \"1.0\";";

    let create_result = vfs.create_file(&workspace_id, &path, initial_content).await;
    assert!(create_result.is_ok());
    let initial_vnode = create_result.unwrap();
    let initial_version = initial_vnode.version;

    // Update the file
    let updated_content = b"const VERSION: &str = \"2.0\";";
    let update_result = vfs.update_file(&workspace_id, &path, updated_content).await;
    assert!(update_result.is_ok(), "update_file should succeed");

    let updated_vnode = update_result.unwrap();
    assert_eq!(updated_vnode.workspace_id, workspace_id);
    assert_eq!(updated_vnode.path, path);
    assert_eq!(updated_vnode.size_bytes, updated_content.len());
    assert!(updated_vnode.version > initial_version, "version should increment");

    // Verify updated content
    let read_content = vfs.read_file(&workspace_id, &path).await.unwrap();
    assert_eq!(read_content, updated_content);

    // Try to update non-existent file
    let missing_path = VirtualPath::new("missing.rs").unwrap();
    let missing_result = vfs.update_file(&workspace_id, &missing_path, b"content").await;
    assert!(missing_result.is_err(), "updating non-existent file should fail");
}

#[tokio::test]
async fn test_file_workflow() {
    // Setup
    let (vfs, _storage) = create_test_vfs().await;
    let workspace_id = Uuid::new_v4();

    // Complete workflow: create -> get -> update -> get
    let path = VirtualPath::new("workflow.rs").unwrap();

    // 1. Create
    let create_result = vfs.create_file(&workspace_id, &path, b"// Step 1").await;
    assert!(create_result.is_ok());

    // 2. Get
    let get_result = vfs.get_file(&workspace_id, &path).await;
    assert!(get_result.is_ok());
    let vnode = get_result.unwrap();
    assert_eq!(vnode.version, 1);

    // 3. Update
    let update_result = vfs.update_file(&workspace_id, &path, b"// Step 2").await;
    assert!(update_result.is_ok());

    // 4. Get again
    let get_result2 = vfs.get_file(&workspace_id, &path).await;
    assert!(get_result2.is_ok());
    let vnode2 = get_result2.unwrap();
    assert_eq!(vnode2.version, 2);

    // Verify content
    let content = vfs.read_file(&workspace_id, &path).await.unwrap();
    assert_eq!(content, b"// Step 2");
}
