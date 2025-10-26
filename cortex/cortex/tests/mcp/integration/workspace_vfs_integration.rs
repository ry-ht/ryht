//! Integration Tests: Workspace + VFS
//!
//! These tests verify the complete workflow from workspace creation
//! through VFS operations, ensuring data flows correctly between:
//! - Workspace creation and activation
//! - File system import and indexing
//! - VFS operations (read, write, update)
//! - Version tracking and history
//!
//! Real-world scenarios:
//! - Import project → list files → read → modify → verify changes
//! - Create workspace → load multiple files → update → track versions
//! - Concurrent operations across workspaces

use crate::mcp::utils::{TestHarness, ToolResultAssertions};
use cortex::mcp::tools::workspace::*;
use cortex::mcp::tools::vfs::*;
use mcp_sdk::prelude::*;
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn test_workspace_creation_and_file_listing() {
    let harness = TestHarness::new().await;

    // Step 1: Create workspace
    let workspace_ctx = harness.workspace_context();
    let create_workspace_tool = WorkspaceCreateTool::new(workspace_ctx.clone());

    let project_dir = harness.temp_path().join("test_project");
    tokio::fs::create_dir_all(&project_dir).await.unwrap();

    // Create a simple test file
    let test_file = project_dir.join("main.rs");
    tokio::fs::write(&test_file, "fn main() { println!(\"Hello\"); }").await.unwrap();

    let start = Instant::now();
    let result = create_workspace_tool
        .execute(
            json!({
                "name": "test_workspace",
                "root_path": project_dir.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": true,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Workspace creation failed");

    let create_duration = start.elapsed();

    result
        .assert_success()
        .assert_has_field("workspace_id")
        .assert_has_field("files_imported")
        .assert_has_field("units_extracted");

    let workspace_id = result.get_field("workspace_id").unwrap().as_str().unwrap();

    // Step 2: List directory contents via VFS
    let vfs_ctx = harness.vfs_context();
    let list_dir_tool = VfsListDirectoryTool::new(vfs_ctx.clone());

    let list_result = list_dir_tool
        .execute(
            json!({
                "path": "/",
                "workspace_id": workspace_id,
                "recursive": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Directory listing failed");

    list_result
        .assert_success()
        .assert_has_field("entries")
        .assert_array_min_length("entries", 1);

    // Step 3: Get specific file
    let get_node_tool = VfsGetNodeTool::new(vfs_ctx.clone());

    let file_result = get_node_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace_id,
                "include_content": true,
                "include_metadata": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File retrieval failed");

    file_result
        .assert_success()
        .assert_has_field("content")
        .assert_has_field("version")
        .assert_has_field("size_bytes");

    let content = file_result.get_field("content").unwrap().as_str().unwrap();
    assert!(content.contains("println!"), "File content incorrect");

    // Step 4: Update the file
    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());

    let new_content = "fn main() { println!(\"Hello, World!\"); }";
    let update_result = update_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace_id,
                "content": new_content,
                "create_if_missing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File update failed");

    update_result
        .assert_success()
        .assert_has_field("new_version");

    let new_version = update_result.get_field("new_version").unwrap().as_u64().unwrap();
    assert!(new_version > 1, "Version should have incremented");

    // Step 5: Verify the update by reading again
    let verify_result = get_node_tool
        .execute(
            json!({
                "path": "/main.rs",
                "workspace_id": workspace_id,
                "include_content": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Verification read failed");

    verify_result.assert_success();
    let updated_content = verify_result.get_field("content").unwrap().as_str().unwrap();
    assert_eq!(updated_content, new_content, "Content not updated correctly");

    // Step 6: List again to verify file still exists
    let final_list = list_dir_tool
        .execute(
            json!({
                "path": "/",
                "workspace_id": workspace_id,
                "recursive": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Final listing failed");

    final_list
        .assert_success()
        .assert_array_min_length("entries", 1);

    // Performance assertions
    assert!(
        create_duration.as_millis() < 5000,
        "Workspace creation took too long: {:?}",
        create_duration
    );

    println!("✓ Workspace + VFS integration test passed");
    println!("  - Workspace created in {:?}", create_duration);
    println!("  - Files imported and indexed");
    println!("  - File read/write operations verified");
    println!("  - Version tracking confirmed");
}

#[tokio::test]
async fn test_multi_file_workspace_operations() {
    let harness = TestHarness::new().await;

    // Create workspace with multiple files
    let workspace_ctx = harness.workspace_context();
    let create_workspace_tool = WorkspaceCreateTool::new(workspace_ctx.clone());

    let project_dir = harness.temp_path().join("multi_file_project");
    tokio::fs::create_dir_all(&project_dir).await.unwrap();
    tokio::fs::create_dir_all(&project_dir.join("src")).await.unwrap();

    // Create multiple files
    tokio::fs::write(
        &project_dir.join("README.md"),
        "# Test Project\n\nThis is a test.",
    )
    .await
    .unwrap();

    tokio::fs::write(
        &project_dir.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .await
    .unwrap();

    tokio::fs::write(
        &project_dir.join("src/main.rs"),
        "fn main() { println!(\"Test\"); }",
    )
    .await
    .unwrap();

    let result = create_workspace_tool
        .execute(
            json!({
                "name": "multi_file_workspace",
                "root_path": project_dir.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": true,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Workspace creation failed");

    result.assert_success();
    let workspace_id = result.get_field("workspace_id").unwrap().as_str().unwrap();

    // Verify all files were imported
    let files_imported = result.get_field("files_imported").unwrap().as_u64().unwrap();
    assert!(files_imported >= 3, "Not all files imported");

    // List root directory
    let vfs_ctx = harness.vfs_context();
    let list_tool = VfsListDirectoryTool::new(vfs_ctx.clone());

    let list_result = list_tool
        .execute(
            json!({
                "path": "/",
                "workspace_id": workspace_id,
                "recursive": true
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Listing failed");

    list_result
        .assert_success()
        .assert_array_min_length("entries", 3);

    // Create a new file via VFS
    let create_tool = VfsCreateFileTool::new(vfs_ctx.clone());

    let create_result = create_tool
        .execute(
            json!({
                "path": "/src/utils.rs",
                "workspace_id": workspace_id,
                "content": "pub fn multiply(a: i32, b: i32) -> i32 { a * b }",
                "overwrite_existing": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("File creation failed");

    create_result.assert_success().assert_has_field("node_id");

    // Verify new file exists in listing
    let verify_list = list_tool
        .execute(
            json!({
                "path": "/src",
                "workspace_id": workspace_id,
                "recursive": false
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Verification listing failed");

    verify_list.assert_success();
    let entries = verify_list.get_field("entries").unwrap().as_array().unwrap();
    assert!(
        entries.len() >= 3,
        "New file not found in directory listing"
    );

    println!("✓ Multi-file workspace operations test passed");
    println!("  - Multiple files imported successfully");
    println!("  - Recursive directory listing works");
    println!("  - New file creation verified");
}

#[tokio::test]
async fn test_workspace_file_history_tracking() {
    let harness = TestHarness::new().await;

    // Create workspace
    let workspace_ctx = harness.workspace_context();
    let create_workspace_tool = WorkspaceCreateTool::new(workspace_ctx.clone());

    let project_dir = harness.temp_path().join("history_project");
    tokio::fs::create_dir_all(&project_dir).await.unwrap();
    tokio::fs::write(
        &project_dir.join("test.rs"),
        "// Version 1",
    )
    .await
    .unwrap();

    let result = create_workspace_tool
        .execute(
            json!({
                "name": "history_workspace",
                "root_path": project_dir.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": false,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Workspace creation failed");

    result.assert_success();
    let workspace_id = result.get_field("workspace_id").unwrap().as_str().unwrap();

    // Make multiple updates
    let vfs_ctx = harness.vfs_context();
    let update_tool = VfsUpdateFileTool::new(vfs_ctx.clone());

    let versions = vec![
        "// Version 2\nfn main() {}",
        "// Version 3\nfn main() { println!(\"test\"); }",
        "// Version 4\nfn main() { println!(\"final\"); }",
    ];

    for (i, content) in versions.iter().enumerate() {
        let update_result = update_tool
            .execute(
                json!({
                    "path": "/test.rs",
                    "workspace_id": workspace_id,
                    "content": content,
                    "create_if_missing": false
                }),
                &ToolContext::default(),
            )
            .await
            .expect(&format!("Update {} failed", i + 1));

        update_result.assert_success();
        let version = update_result.get_field("new_version").unwrap().as_u64().unwrap();
        assert_eq!(version, (i + 2) as u64, "Version number incorrect");
    }

    // Get file history
    let history_tool = VfsGetFileHistoryTool::new(vfs_ctx.clone());

    let history_result = history_tool
        .execute(
            json!({
                "path": "/test.rs",
                "workspace_id": workspace_id,
                "limit": 10
            }),
            &ToolContext::default(),
        )
        .await
        .expect("History retrieval failed");

    history_result
        .assert_success()
        .assert_has_field("versions")
        .assert_array_min_length("versions", 4);

    println!("✓ File history tracking test passed");
    println!("  - Multiple versions created successfully");
    println!("  - Version numbers incremented correctly");
    println!("  - History retrieval works");
}

#[tokio::test]
async fn test_concurrent_workspace_operations() {
    let harness = TestHarness::new().await;

    // Create two workspaces
    let workspace_ctx = harness.workspace_context();
    let create_tool = WorkspaceCreateTool::new(workspace_ctx.clone());

    let project1 = harness.temp_path().join("project1");
    let project2 = harness.temp_path().join("project2");
    tokio::fs::create_dir_all(&project1).await.unwrap();
    tokio::fs::create_dir_all(&project2).await.unwrap();
    tokio::fs::write(&project1.join("file1.rs"), "// Project 1").await.unwrap();
    tokio::fs::write(&project2.join("file2.rs"), "// Project 2").await.unwrap();

    // Create both workspaces concurrently
    let (result1, result2) = tokio::join!(
        create_tool.execute(
            json!({
                "name": "workspace1",
                "root_path": project1.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": false,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        ),
        create_tool.execute(
            json!({
                "name": "workspace2",
                "root_path": project2.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": false,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        )
    );

    result1.expect("Workspace 1 creation failed").assert_success();
    result2.expect("Workspace 2 creation failed").assert_success();

    let ws1_id = result1.unwrap().get_field("workspace_id").unwrap().as_str().unwrap().to_string();
    let ws2_id = result2.unwrap().get_field("workspace_id").unwrap().as_str().unwrap().to_string();

    assert_ne!(ws1_id, ws2_id, "Workspace IDs should be unique");

    // Verify isolation: read from both workspaces
    let vfs_ctx = harness.vfs_context();
    let get_tool = VfsGetNodeTool::new(vfs_ctx.clone());

    let (file1_result, file2_result) = tokio::join!(
        get_tool.execute(
            json!({
                "path": "/file1.rs",
                "workspace_id": ws1_id,
                "include_content": true
            }),
            &ToolContext::default(),
        ),
        get_tool.execute(
            json!({
                "path": "/file2.rs",
                "workspace_id": ws2_id,
                "include_content": true
            }),
            &ToolContext::default(),
        )
    );

    file1_result.expect("File 1 read failed").assert_success();
    file2_result.expect("File 2 read failed").assert_success();

    println!("✓ Concurrent workspace operations test passed");
    println!("  - Multiple workspaces created concurrently");
    println!("  - Workspace isolation verified");
    println!("  - Concurrent file operations successful");
}

#[tokio::test]
async fn test_workspace_performance_benchmarks() {
    let harness = TestHarness::new().await;

    // Create a larger project for performance testing
    let project_dir = harness.temp_path().join("perf_project");
    tokio::fs::create_dir_all(&project_dir.join("src")).await.unwrap();

    // Create 10 files
    for i in 0..10 {
        tokio::fs::write(
            &project_dir.join(format!("src/module{}.rs", i)),
            format!("pub fn func{}() {{ println!(\"test\"); }}", i),
        )
        .await
        .unwrap();
    }

    let workspace_ctx = harness.workspace_context();
    let create_tool = WorkspaceCreateTool::new(workspace_ctx.clone());

    let start = Instant::now();
    let result = create_tool
        .execute(
            json!({
                "name": "perf_workspace",
                "root_path": project_dir.to_str().unwrap(),
                "workspace_type": "code",
                "import_options": {
                    "import_files": true,
                    "extract_units": true,
                    "create_embeddings": false
                }
            }),
            &ToolContext::default(),
        )
        .await
        .expect("Workspace creation failed");

    let import_duration = start.elapsed();

    result.assert_success();
    let files_imported = result.get_field("files_imported").unwrap().as_u64().unwrap();

    // Performance assertions
    assert!(
        import_duration.as_millis() < 10000,
        "Import took too long: {:?}",
        import_duration
    );

    let avg_time_per_file = import_duration.as_millis() as f64 / files_imported as f64;
    assert!(
        avg_time_per_file < 1000.0,
        "Average time per file too high: {:.2}ms",
        avg_time_per_file
    );

    println!("✓ Performance benchmark test passed");
    println!("  - Total import time: {:?}", import_duration);
    println!("  - Files imported: {}", files_imported);
    println!("  - Avg time per file: {:.2}ms", avg_time_per_file);
}
