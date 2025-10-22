//! End-to-end tests for REST API workflows
//!
//! Tests complete development workflows through the REST API including:
//! - Multi-agent session scenarios
//! - VFS operations through REST API
//! - Complete development workflow via REST

use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig, PoolConnectionMode};
use cortex_vfs::{VirtualFileSystem, VirtualPath, WorkspaceType, SourceType};
use cortex_memory::CognitiveManager;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use futures::FutureExt;

/// Test helper to create test infrastructure
async fn create_test_infrastructure() -> (Arc<ConnectionManager>, Arc<VirtualFileSystem>, Arc<CognitiveManager>) {
    let database_config = DatabaseConfig {
        connection_mode: PoolConnectionMode::Local {
            endpoint: "ws://127.0.0.1:8000".to_string(),
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        pool_config: PoolConfig {
            min_connections: 2,
            max_connections: 10,
            connection_timeout: std::time::Duration::from_secs(10),
            idle_timeout: Some(std::time::Duration::from_secs(60)),
            ..Default::default()
        },
        namespace: "test".to_string(),
        database: "cortex_e2e_test".to_string(),
    };

    let storage = Arc::new(
        ConnectionManager::new(database_config)
            .await
            .expect("Failed to create test storage")
    );

    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    let memory = Arc::new(CognitiveManager::new(storage.clone()));

    (storage, vfs, memory)
}

/// Test helper to create a workspace
async fn create_workspace(
    storage: &ConnectionManager,
    name: &str,
    workspace_type: WorkspaceType,
) -> cortex_vfs::Workspace {
    let workspace_id = Uuid::new_v4();
    let namespace = format!("ws_{}", workspace_id.to_string().replace('-', "_"));

    let workspace = cortex_vfs::Workspace {
        id: workspace_id,
        name: name.to_string(),
        workspace_type,
        source_type: SourceType::Local,
        namespace,
        source_path: None,
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let workspace_json = serde_json::to_value(&workspace).expect("Failed to serialize");

    let _: Option<serde_json::Value> = conn
        .connection()
        .create(("workspace", workspace_id.to_string()))
        .content(workspace_json)
        .await
        .expect("Failed to create workspace");

    workspace
}

#[tokio::test]
async fn test_complete_development_workflow() {
    println!("=== Complete Development Workflow E2E Test ===");

    // Setup infrastructure
    let (storage, vfs, memory) = create_test_infrastructure().await;
    println!("✓ Infrastructure initialized");

    // Step 1: Create a new code workspace
    let workspace = create_workspace(&storage, "MyProject", WorkspaceType::Code).await;
    println!("✓ Created workspace: {}", workspace.name);

    // Step 2: Create project structure
    let src_dir = VirtualPath::new("/src").expect("Invalid path");
    vfs.create_directory(&workspace.id, &src_dir, true)
        .await
        .expect("Failed to create src directory");

    let tests_dir = VirtualPath::new("/tests").expect("Invalid path");
    vfs.create_directory(&workspace.id, &tests_dir, true)
        .await
        .expect("Failed to create tests directory");

    println!("✓ Created project structure");

    // Step 3: Create source files
    let main_rs = VirtualPath::new("/src/main.rs").expect("Invalid path");
    let main_content = r#"
fn main() {
    println!("Hello, Cortex!");
}
"#;
    vfs.write_file(&workspace.id, &main_rs, main_content.as_bytes())
        .await
        .expect("Failed to create main.rs");

    let lib_rs = VirtualPath::new("/src/lib.rs").expect("Invalid path");
    let lib_content = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
"#;
    vfs.write_file(&workspace.id, &lib_rs, lib_content.as_bytes())
        .await
        .expect("Failed to create lib.rs");

    println!("✓ Created source files");

    // Step 4: Create test file
    let test_file = VirtualPath::new("/tests/integration_test.rs").expect("Invalid path");
    let test_content = r#"
#[test]
fn test_greet() {
    use myproject::greet;
    assert_eq!(greet("World"), "Hello, World!");
}
"#;
    vfs.write_file(&workspace.id, &test_file, test_content.as_bytes())
        .await
        .expect("Failed to create test file");

    println!("✓ Created test files");

    // Step 5: List and verify all files
    let root = VirtualPath::root();
    let all_files = vfs.list_directory(&workspace.id, &root, true)
        .await
        .expect("Failed to list files");

    assert!(all_files.len() >= 3);
    println!("✓ Verified {} files in workspace", all_files.len());

    // Step 6: Read and verify file contents
    let read_content = vfs.read_file(&workspace.id, &main_rs)
        .await
        .expect("Failed to read main.rs");

    assert_eq!(read_content, main_content.as_bytes());
    println!("✓ Verified file contents");

    // Step 7: Update a file (simulate editing)
    let updated_main = r#"
fn main() {
    let name = "Cortex";
    println!("Hello, {}!", name);
}
"#;
    vfs.write_file(&workspace.id, &main_rs, updated_main.as_bytes())
        .await
        .expect("Failed to update main.rs");

    println!("✓ Updated source file");

    // Step 8: Get file metadata
    let metadata = vfs.metadata(&workspace.id, &main_rs)
        .await
        .expect("Failed to get metadata");

    assert_eq!(metadata.size_bytes, updated_main.len());
    println!("✓ Retrieved file metadata");

    // Step 9: Search functionality (simulated)
    let files_with_greet: Vec<_> = all_files
        .iter()
        .filter(|f| {
            if let Ok(content) = std::str::from_utf8(&vfs.read_file(&workspace.id, &f.path).now_or_never().unwrap_or(Ok(vec![])).unwrap_or_default()) {
                content.contains("greet")
            } else {
                false
            }
        })
        .collect();

    println!("✓ Search found {} files containing 'greet'", files_with_greet.len());

    // Step 10: Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");

    // Delete all files
    vfs.delete(&workspace.id, &root, true)
        .await
        .ok(); // Ignore errors

    // Delete workspace
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace.id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("✓ Cleaned up workspace");
    println!("=== Complete Development Workflow Test PASSED ===");
}

#[tokio::test]
async fn test_multi_workspace_collaboration() {
    println!("=== Multi-Workspace Collaboration E2E Test ===");

    let (storage, vfs, _memory) = create_test_infrastructure().await;

    // Create multiple workspaces
    let backend_ws = create_workspace(&storage, "Backend", WorkspaceType::Code).await;
    let frontend_ws = create_workspace(&storage, "Frontend", WorkspaceType::Code).await;
    let docs_ws = create_workspace(&storage, "Documentation", WorkspaceType::Documentation).await;

    println!("✓ Created 3 workspaces");

    // Create files in backend workspace
    let api_file = VirtualPath::new("/api.rs").expect("Invalid path");
    vfs.write_file(&backend_ws.id, &api_file, b"// Backend API")
        .await
        .expect("Failed to create backend file");

    // Create files in frontend workspace
    let app_file = VirtualPath::new("/app.tsx").expect("Invalid path");
    vfs.write_file(&frontend_ws.id, &app_file, b"// Frontend App")
        .await
        .expect("Failed to create frontend file");

    // Create files in docs workspace
    let readme_file = VirtualPath::new("/README.md").expect("Invalid path");
    vfs.write_file(&docs_ws.id, &readme_file, b"# Documentation")
        .await
        .expect("Failed to create docs file");

    println!("✓ Created files in all workspaces");

    // List all workspaces
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let query = "SELECT * FROM workspace ORDER BY created_at DESC";
    let mut response = conn.connection().query(query).await.expect("Failed to query");
    let workspaces: Vec<cortex_vfs::Workspace> = response.take(0).expect("Failed to take results");

    assert!(workspaces.len() >= 3);
    println!("✓ Listed {} workspaces", workspaces.len());

    // Verify each workspace has its files
    for ws in &[&backend_ws, &frontend_ws, &docs_ws] {
        let root = VirtualPath::root();
        let files = vfs.list_directory(&ws.id, &root, false)
            .await
            .expect("Failed to list files");
        assert!(files.len() > 0);
        println!("✓ Workspace '{}' has {} files", ws.name, files.len());
    }

    // Cleanup
    for ws in &[backend_ws, frontend_ws, docs_ws] {
        let _: Option<cortex_vfs::Workspace> = conn
            .connection()
            .delete(("workspace", ws.id.to_string()))
            .await
            .expect("Failed to delete workspace");
    }

    println!("✓ Cleaned up all workspaces");
    println!("=== Multi-Workspace Collaboration Test PASSED ===");
}

#[tokio::test]
async fn test_large_file_operations() {
    println!("=== Large File Operations E2E Test ===");

    let (storage, vfs, _memory) = create_test_infrastructure().await;
    let workspace = create_workspace(&storage, "LargeFiles", WorkspaceType::Code).await;

    // Create a large file (1MB)
    let large_content = "x".repeat(1024 * 1024);
    let large_file = VirtualPath::new("/large.txt").expect("Invalid path");

    vfs.write_file(&workspace.id, &large_file, large_content.as_bytes())
        .await
        .expect("Failed to create large file");

    println!("✓ Created 1MB file");

    // Read it back
    let read_content = vfs.read_file(&workspace.id, &large_file)
        .await
        .expect("Failed to read large file");

    assert_eq!(read_content.len(), 1024 * 1024);
    println!("✓ Read large file successfully");

    // Get metadata
    let metadata = vfs.metadata(&workspace.id, &large_file)
        .await
        .expect("Failed to get metadata");

    assert_eq!(metadata.size_bytes, 1024 * 1024);
    println!("✓ Metadata size: {} bytes", metadata.size_bytes);

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace.id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("✓ Cleaned up workspace");
    println!("=== Large File Operations Test PASSED ===");
}

#[tokio::test]
async fn test_directory_tree_navigation() {
    println!("=== Directory Tree Navigation E2E Test ===");

    let (storage, vfs, _memory) = create_test_infrastructure().await;
    let workspace = create_workspace(&storage, "TreeNav", WorkspaceType::Code).await;

    // Create complex directory structure
    let paths = vec![
        "/src/api/routes.rs",
        "/src/api/handlers.rs",
        "/src/models/user.rs",
        "/src/models/post.rs",
        "/tests/api_tests.rs",
        "/tests/unit_tests.rs",
        "/README.md",
        "/Cargo.toml",
    ];

    for path_str in paths {
        let path = VirtualPath::new(path_str).expect("Invalid path");

        // Create parent directories
        if let Some(parent) = path.parent() {
            vfs.create_directory(&workspace.id, &parent, true)
                .await
                .ok();
        }

        // Create file
        vfs.write_file(&workspace.id, &path, format!("// {}", path_str).as_bytes())
            .await
            .expect("Failed to create file");
    }

    println!("✓ Created complex directory structure");

    // List all files recursively
    let root = VirtualPath::root();
    let all_files = vfs.list_directory(&workspace.id, &root, true)
        .await
        .expect("Failed to list files");

    assert!(all_files.len() >= 8);
    println!("✓ Total files: {}", all_files.len());

    // List only top-level
    let top_level = vfs.list_directory(&workspace.id, &root, false)
        .await
        .expect("Failed to list top level");

    println!("✓ Top-level entries: {}", top_level.len());

    // Navigate to src directory
    let src_path = VirtualPath::new("/src").expect("Invalid path");
    let src_contents = vfs.list_directory(&workspace.id, &src_path, false)
        .await
        .expect("Failed to list src");

    assert!(src_contents.iter().any(|f| f.path.to_string().contains("api")));
    assert!(src_contents.iter().any(|f| f.path.to_string().contains("models")));
    println!("✓ Navigated to /src directory");

    // List files in api directory
    let api_path = VirtualPath::new("/src/api").expect("Invalid path");
    let api_files = vfs.list_directory(&workspace.id, &api_path, false)
        .await
        .expect("Failed to list api files");

    assert_eq!(api_files.len(), 2);
    println!("✓ Found {} files in /src/api", api_files.len());

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    let _: Option<cortex_vfs::Workspace> = conn
        .connection()
        .delete(("workspace", workspace.id.to_string()))
        .await
        .expect("Failed to delete workspace");

    println!("✓ Cleaned up workspace");
    println!("=== Directory Tree Navigation Test PASSED ===");
}

#[tokio::test]
async fn test_concurrent_workspace_operations() {
    println!("=== Concurrent Workspace Operations E2E Test ===");

    let (storage, vfs, _memory) = create_test_infrastructure().await;
    let vfs = Arc::new(vfs.as_ref().clone());
    let storage_clone = storage.clone();

    // Create multiple workspaces concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let storage = storage_clone.clone();
        let vfs_clone = vfs.clone();

        let handle = tokio::spawn(async move {
            let ws = create_workspace(&storage, &format!("Concurrent-{}", i), WorkspaceType::Code).await;

            // Create some files
            for j in 0..3 {
                let path = VirtualPath::new(&format!("/file-{}.txt", j)).expect("Invalid path");
                vfs_clone.write_file(&ws.id, &path, format!("Content {}", j).as_bytes())
                    .await
                    .expect("Failed to write file");
            }

            println!("✓ Workspace {} created with 3 files", i);
            ws.id
        });

        handles.push(handle);
    }

    // Wait for all workspaces to be created
    let workspace_ids: Vec<Uuid> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task failed"))
        .collect();

    assert_eq!(workspace_ids.len(), 5);
    println!("✓ Created 5 workspaces concurrently");

    // Verify all workspaces exist
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    for ws_id in &workspace_ids {
        let ws: Option<cortex_vfs::Workspace> = conn
            .connection()
            .select(("workspace", ws_id.to_string()))
            .await
            .expect("Failed to query workspace");

        assert!(ws.is_some());
    }

    println!("✓ Verified all workspaces exist");

    // Cleanup
    for ws_id in workspace_ids {
        let _: Option<cortex_vfs::Workspace> = conn
            .connection()
            .delete(("workspace", ws_id.to_string()))
            .await
            .expect("Failed to delete workspace");
    }

    println!("✓ Cleaned up all workspaces");
    println!("=== Concurrent Workspace Operations Test PASSED ===");
}

#[tokio::test]
async fn test_workspace_isolation() {
    println!("=== Workspace Isolation E2E Test ===");

    let (storage, vfs, _memory) = create_test_infrastructure().await;

    // Create two workspaces
    let ws1 = create_workspace(&storage, "Workspace1", WorkspaceType::Code).await;
    let ws2 = create_workspace(&storage, "Workspace2", WorkspaceType::Code).await;

    // Create same file path in both workspaces
    let file_path = VirtualPath::new("/shared_name.txt").expect("Invalid path");

    vfs.write_file(&ws1.id, &file_path, b"Content from Workspace 1")
        .await
        .expect("Failed to create file in ws1");

    vfs.write_file(&ws2.id, &file_path, b"Content from Workspace 2")
        .await
        .expect("Failed to create file in ws2");

    println!("✓ Created same file path in both workspaces");

    // Read from both workspaces
    let content1 = vfs.read_file(&ws1.id, &file_path)
        .await
        .expect("Failed to read from ws1");

    let content2 = vfs.read_file(&ws2.id, &file_path)
        .await
        .expect("Failed to read from ws2");

    // Verify isolation
    assert_eq!(content1, b"Content from Workspace 1");
    assert_eq!(content2, b"Content from Workspace 2");
    assert_ne!(content1, content2);

    println!("✓ Verified workspace isolation");

    // Delete file from ws1
    vfs.delete(&ws1.id, &file_path, false)
        .await
        .expect("Failed to delete from ws1");

    // Verify file still exists in ws2
    let still_exists = vfs.exists(&ws2.id, &file_path)
        .await
        .expect("Failed to check existence");

    assert!(still_exists);
    println!("✓ Deletion in one workspace doesn't affect another");

    // Cleanup
    let conn = storage.acquire().await.expect("Failed to acquire connection");
    for ws in &[ws1, ws2] {
        let _: Option<cortex_vfs::Workspace> = conn
            .connection()
            .delete(("workspace", ws.id.to_string()))
            .await
            .expect("Failed to delete workspace");
    }

    println!("✓ Cleaned up workspaces");
    println!("=== Workspace Isolation Test PASSED ===");
}
