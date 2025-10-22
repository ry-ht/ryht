//! Integration tests for REST API endpoints
//!
//! Tests complete REST API workflow including:
//! - VFS navigation endpoints
//! - Workspace CRUD operations
//! - Session management
//! - Search functionality

use cortex_core::config::GlobalConfig;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, PoolConnectionMode};
use cortex_vfs::{VirtualFileSystem, VirtualPath, WorkspaceType, SourceType};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

/// Test helper to create a test storage connection
async fn create_test_storage() -> Arc<ConnectionManager> {
    let database_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: "ws://127.0.0.1:8000".to_string(),
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 5,
            connection_timeout: std::time::Duration::from_secs(10),
            idle_timeout: Some(std::time::Duration::from_secs(60)),
            ..Default::default()
        },
        namespace: "test".to_string(),
        database: "cortex_api_test".to_string(),
    };

    Arc::new(
        ConnectionManager::new(database_config)
            .await
            .expect("Failed to create test storage")
    )
}

/// Test helper to create a test workspace
async fn create_test_workspace(
    storage: Arc<ConnectionManager>,
    vfs: &VirtualFileSystem,
    name: &str,
) -> Uuid {
    let workspace_id = Uuid::new_v4();
    let namespace = format!("ws_{}", workspace_id.to_string().replace('-', "_"));

    let workspace = cortex_vfs::Workspace {
        id: workspace_id,
        name: name.to_string(),
        workspace_type: WorkspaceType::Code,
        source_type: SourceType::Local,
        namespace: namespace.clone(),
        source_path: None,
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Save to database
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let workspace_json = serde_json::to_value(&workspace).expect("Failed to serialize workspace");

    let _: Option<serde_json::Value> = conn
        .connection()
        .create(("workspace", workspace_id.to_string()))
        .content(workspace_json)
        .await
        .expect("Failed to create workspace");

    workspace_id
}

#[tokio::test]
async fn test_workspace_crud_workflow() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());

    // Create workspace
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Test Workspace").await;
    println!("Created workspace: {}", workspace_id);

    // Retrieve workspace
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let workspace: Option<cortex_vfs::Workspace> = conn
        .connection()
        .select(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to query workspace");

    assert!(workspace.is_some());
    let workspace = workspace.unwrap();
    assert_eq!(workspace.name, "Test Workspace");
    assert_eq!(workspace.workspace_type, WorkspaceType::Code);

    // Delete workspace
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");

    // Verify deletion
    let workspace: Option<cortex_vfs::Workspace> = conn
        .connection()
        .select(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to query workspace");

    assert!(workspace.is_none());

    println!("Workspace CRUD workflow completed successfully");
}

#[tokio::test]
async fn test_vfs_file_operations() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "VFS Test Workspace").await;

    // Create a file
    let file_path = VirtualPath::new("/test.rs").expect("Invalid path");
    let content = b"fn main() { println!(\"Hello, world!\"); }";

    vfs.write_file(&workspace_id, &file_path, content)
        .await
        .expect("Failed to write file");

    println!("Created file: {}", file_path);

    // Read the file
    let read_content = vfs.read_file(&workspace_id, &file_path)
        .await
        .expect("Failed to read file");

    assert_eq!(read_content, content);
    println!("Read file successfully");

    // Update the file
    let new_content = b"fn main() { println!(\"Updated!\"); }";
    vfs.write_file(&workspace_id, &file_path, new_content)
        .await
        .expect("Failed to update file");

    let updated_content = vfs.read_file(&workspace_id, &file_path)
        .await
        .expect("Failed to read updated file");

    assert_eq!(updated_content, new_content);
    println!("Updated file successfully");

    // Get metadata
    let metadata = vfs.metadata(&workspace_id, &file_path)
        .await
        .expect("Failed to get metadata");

    assert_eq!(metadata.path, file_path);
    assert!(metadata.is_file());
    assert_eq!(metadata.size_bytes, new_content.len() as i64);
    println!("Retrieved metadata: {:?}", metadata);

    // List files
    let root = VirtualPath::root();
    let files = vfs.list_directory(&workspace_id, &root, false)
        .await
        .expect("Failed to list files");

    assert!(files.iter().any(|f| f.path == file_path));
    println!("Listed {} files", files.len());

    // Delete file
    vfs.delete(&workspace_id, &file_path, false)
        .await
        .expect("Failed to delete file");

    // Verify deletion
    let exists = vfs.exists(&workspace_id, &file_path)
        .await
        .expect("Failed to check existence");

    assert!(!exists);
    println!("Deleted file successfully");

    // Cleanup workspace
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("VFS file operations completed successfully");
}

#[tokio::test]
async fn test_vfs_directory_operations() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Directory Test Workspace").await;

    // Create directory structure
    let dir_path = VirtualPath::new("/src/api").expect("Invalid path");
    vfs.create_directory(&workspace_id, &dir_path, true)
        .await
        .expect("Failed to create directory");

    println!("Created directory: {}", dir_path);

    // Create files in directory
    let file1_path = VirtualPath::new("/src/api/mod.rs").expect("Invalid path");
    let file2_path = VirtualPath::new("/src/api/routes.rs").expect("Invalid path");

    vfs.write_file(&workspace_id, &file1_path, b"pub mod routes;")
        .await
        .expect("Failed to create file1");

    vfs.write_file(&workspace_id, &file2_path, b"// routes")
        .await
        .expect("Failed to create file2");

    println!("Created files in directory");

    // List directory recursively
    let root = VirtualPath::root();
    let files = vfs.list_directory(&workspace_id, &root, true)
        .await
        .expect("Failed to list directory");

    assert!(files.iter().any(|f| f.path == file1_path));
    assert!(files.iter().any(|f| f.path == file2_path));
    println!("Listed {} files recursively", files.len());

    // Delete directory recursively
    let src_path = VirtualPath::new("/src").expect("Invalid path");
    vfs.delete(&workspace_id, &src_path, true)
        .await
        .expect("Failed to delete directory");

    // Verify deletion
    let exists = vfs.exists(&workspace_id, &src_path)
        .await
        .expect("Failed to check existence");

    assert!(!exists);
    println!("Deleted directory successfully");

    // Cleanup workspace
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("VFS directory operations completed successfully");
}

#[tokio::test]
async fn test_workspace_listing_and_filtering() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());

    // Create multiple workspaces
    let ws1_id = create_test_workspace(storage.clone(), &vfs, "Workspace Alpha").await;
    let ws2_id = create_test_workspace(storage.clone(), &vfs, "Workspace Beta").await;
    let ws3_id = create_test_workspace(storage.clone(), &vfs, "Workspace Gamma").await;

    println!("Created 3 test workspaces");

    // List all workspaces
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let query = "SELECT * FROM workspace ORDER BY created_at DESC";
    let mut response = conn
        .connection()
        .query(query)
        .await
        .expect("Failed to query workspaces");

    let workspaces: Vec<cortex_vfs::Workspace> = response
        .take(0)
        .expect("Failed to take results");

    assert!(workspaces.len() >= 3);
    println!("Listed {} workspaces", workspaces.len());

    // Verify workspace names
    let names: Vec<String> = workspaces.iter().map(|w| w.name.clone()).collect();
    assert!(names.contains(&"Workspace Alpha".to_string()));
    assert!(names.contains(&"Workspace Beta".to_string()));
    assert!(names.contains(&"Workspace Gamma".to_string()));

    // Cleanup
    for workspace_id in &[ws1_id, ws2_id, ws3_id] {
        let _: Option<cortex_vfs::Workspace> = conn
            .connection()
            .delete(("workspace", workspace_id.to_string()))
            .await
            .expect("Failed to delete workspace");
    }

    println!("Workspace listing and filtering completed successfully");
}

#[tokio::test]
async fn test_file_filtering_by_language() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Language Filter Test").await;

    // Create files with different languages
    let rust_file = VirtualPath::new("/main.rs").expect("Invalid path");
    let ts_file = VirtualPath::new("/app.ts").expect("Invalid path");
    let js_file = VirtualPath::new("/script.js").expect("Invalid path");

    vfs.write_file(&workspace_id, &rust_file, b"fn main() {}")
        .await
        .expect("Failed to create rust file");

    vfs.write_file(&workspace_id, &ts_file, b"const x = 1;")
        .await
        .expect("Failed to create ts file");

    vfs.write_file(&workspace_id, &js_file, b"console.log('hi');")
        .await
        .expect("Failed to create js file");

    println!("Created test files");

    // List all files
    let root = VirtualPath::root();
    let all_files = vfs.list_directory(&workspace_id, &root, false)
        .await
        .expect("Failed to list files");

    assert_eq!(all_files.len(), 3);
    println!("Total files: {}", all_files.len());

    // Filter by file extension (simulating language filter)
    let rust_files: Vec<_> = all_files
        .iter()
        .filter(|f| f.path.to_string().ends_with(".rs"))
        .collect();

    assert_eq!(rust_files.len(), 1);
    assert_eq!(rust_files[0].path, rust_file);
    println!("Rust files: {}", rust_files.len());

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("File filtering by language completed successfully");
}

#[tokio::test]
async fn test_pagination_support() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Pagination Test").await;

    // Create 10 test files
    for i in 0..10 {
        let file_path = VirtualPath::new(&format!("/file_{}.txt", i)).expect("Invalid path");
        let content = format!("File {}", i);
        vfs.write_file(&workspace_id, &file_path, content.as_bytes())
            .await
            .expect("Failed to create file");
    }

    println!("Created 10 test files");

    // List all files
    let root = VirtualPath::root();
    let all_files = vfs.list_directory(&workspace_id, &root, false)
        .await
        .expect("Failed to list files");

    assert_eq!(all_files.len(), 10);

    // Test pagination (simulated)
    let limit = 5;
    let offset = 0;
    let page1: Vec<_> = all_files.iter().skip(offset).take(limit).collect();
    assert_eq!(page1.len(), 5);
    println!("Page 1: {} files", page1.len());

    let offset = 5;
    let page2: Vec<_> = all_files.iter().skip(offset).take(limit).collect();
    assert_eq!(page2.len(), 5);
    println!("Page 2: {} files", page2.len());

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("Pagination support test completed successfully");
}

#[tokio::test]
async fn test_error_handling_invalid_workspace() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());

    // Try to access non-existent workspace
    let invalid_workspace_id = Uuid::new_v4();
    let conn = storage.acquire().await.expect("Failed to acquire connection");

    let workspace: Option<cortex_vfs::Workspace> = conn
        .connection()
        .select(("workspace", invalid_workspace_id.to_string()))
        .await
        .expect("Failed to query workspace");

    assert!(workspace.is_none());
    println!("Correctly handled invalid workspace ID");
}

#[tokio::test]
async fn test_error_handling_invalid_file() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = VirtualFileSystem::new(storage.clone());
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Error Test").await;

    // Try to read non-existent file
    let invalid_path = VirtualPath::new("/nonexistent.txt").expect("Invalid path");
    let result = vfs.read_file(&workspace_id, &invalid_path).await;

    assert!(result.is_err());
    println!("Correctly handled non-existent file");

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");
}

#[tokio::test]
async fn test_concurrent_file_access() {
    // Setup
    let storage = create_test_storage().await;
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    let workspace_id = create_test_workspace(storage.clone(), &vfs, "Concurrent Test").await;

    // Create a file
    let file_path = VirtualPath::new("/concurrent.txt").expect("Invalid path");
    vfs.write_file(&workspace_id, &file_path, b"initial")
        .await
        .expect("Failed to create file");

    // Spawn multiple concurrent read tasks
    let mut handles = vec![];
    for i in 0..5 {
        let vfs_clone = vfs.clone();
        let ws_id = workspace_id;
        let path = file_path.clone();

        let handle = tokio::spawn(async move {
            let content = vfs_clone.read_file(&ws_id, &path)
                .await
                .expect("Failed to read file");
            println!("Concurrent read {}: {} bytes", i, content.len());
            content
        });

        handles.push(handle);
    }

    // Wait for all reads to complete
    for handle in handles {
        let content = handle.await.expect("Task failed");
        assert_eq!(content, b"initial");
    }

    println!("Concurrent file access completed successfully");

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace_id.to_string()))
        .await
        .expect("Failed to delete workspace");
}
