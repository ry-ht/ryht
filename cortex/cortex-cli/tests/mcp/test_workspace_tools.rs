//! Tests for workspace management tools
//!
//! These tests verify:
//! - Workspace creation with project import
//! - Workspace retrieval and statistics
//! - Workspace listing
//! - Workspace activation
//! - Sync detection and update
//! - Export/materialization
//! - Archive and delete operations

use cortex_mcp::tools::workspace::*;
use cortex_parser::CodeParser;
use cortex_storage::ConnectionManager;
use cortex_storage::connection::ConnectionConfig;
use cortex_vfs::VirtualFileSystem;
use cortex_memory::SemanticMemorySystem;
use mcp_sdk::Tool;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

/// Create a test workspace context
async fn create_test_context() -> (WorkspaceContext, TempDir) {
    let temp_dir = TempDir::new().unwrap();

    let config = ConnectionConfig::memory();
    let storage = Arc::new(ConnectionManager::new(config).await.unwrap());

    let ctx = WorkspaceContext::new(storage).unwrap();

    (ctx, temp_dir)
}

/// Create a simple Rust project for testing
async fn create_test_project(dir: &std::path::Path) -> std::io::Result<()> {
    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(dir.join("Cargo.toml"), cargo_toml).await?;

    // Create src directory
    fs::create_dir(dir.join("src")).await?;

    // Create src/lib.rs
    let lib_rs = r#"//! Test library

/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A simple point structure
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate distance from origin
    pub fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;
    fs::write(dir.join("src").join("lib.rs"), lib_rs).await?;

    // Create src/main.rs
    let main_rs = r#"fn main() {
    println!("Hello, world!");
}
"#;
    fs::write(dir.join("src").join("main.rs"), main_rs).await?;

    // Create .gitignore
    let gitignore = r#"target/
Cargo.lock
.DS_Store
"#;
    fs::write(dir.join(".gitignore"), gitignore).await?;

    Ok(())
}

#[tokio::test]
async fn test_workspace_create_import() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    let tool = WorkspaceCreateTool::new(ctx.clone());

    // Create workspace with import
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": true,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let result = tool.execute(input, &context).await.unwrap();

    // Verify result
    assert!(result.is_success());
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    assert!(output["workspace_id"].is_string());
    assert_eq!(output["workspace_type"], "code");
    assert!(output["files_imported"].as_u64().unwrap() >= 2); // At least lib.rs and main.rs
    assert!(output["units_extracted"].as_u64().unwrap() > 0); // Should have extracted functions/structs

    println!("Created workspace: {:?}", output);
}

#[tokio::test]
async fn test_workspace_create_without_import() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("empty-project");
    fs::create_dir(&project_dir).await.unwrap();

    let tool = WorkspaceCreateTool::new(ctx.clone());

    let input = serde_json::json!({
        "name": "EmptyProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let result = tool.execute(input, &context).await.unwrap();

    assert!(result.is_success());
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    assert_eq!(output["files_imported"], 0);
    assert_eq!(output["units_extracted"], 0);
}

#[tokio::test]
async fn test_workspace_get() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    // First create a workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Now get the workspace
    let get_tool = WorkspaceGetTool::new(ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let get_result = get_tool.execute(get_input, &context).await.unwrap();
    assert!(get_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&get_result.content[0].text).unwrap();

    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["name"], "TestProject");
    assert_eq!(output["workspace_type"], "code");
    assert_eq!(output["source_type"], "local");
    assert_eq!(output["read_only"], false);
    assert!(output["stats"].is_object());
    assert!(output["stats"]["total_files"].as_u64().unwrap() >= 2);

    println!("Workspace info: {:?}", output);
}

#[tokio::test]
async fn test_workspace_list() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    // Create a couple of workspaces
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let context = mcp_sdk::prelude::ToolContext::default();

    for i in 1..=2 {
        let input = serde_json::json!({
            "name": format!("TestProject{}", i),
            "root_path": project_dir.to_string_lossy(),
            "auto_import": false,
        });
        create_tool.execute(input, &context).await.unwrap();
    }

    // List workspaces
    let list_tool = WorkspaceListTool::new(ctx.clone());
    let list_input = serde_json::json!({
        "limit": 10,
    });

    let list_result = list_tool.execute(list_input, &context).await.unwrap();
    assert!(list_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&list_result.content[0].text).unwrap();

    assert!(output["total"].as_u64().unwrap() >= 2);
    assert!(output["workspaces"].is_array());
    assert!(output["workspaces"].as_array().unwrap().len() >= 2);

    // Check workspace summary fields
    let workspace = &output["workspaces"][0];
    assert!(workspace["workspace_id"].is_string());
    assert!(workspace["name"].is_string());
    assert!(workspace["workspace_type"].is_string());
    assert!(workspace["created_at"].is_string());

    println!("Listed {} workspaces", output["total"]);
}

#[tokio::test]
async fn test_workspace_activate() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    // Create workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Activate workspace
    let activate_tool = WorkspaceActivateTool::new(ctx.clone());
    let activate_input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let activate_result = activate_tool.execute(activate_input, &context).await.unwrap();
    assert!(activate_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&activate_result.content[0].text).unwrap();

    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["name"], "TestProject");
    assert_eq!(output["status"], "activated");
}

#[tokio::test]
async fn test_workspace_sync_from_disk() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    // Create and import workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Add a new file to the project
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Ensure timestamp difference
    let new_file = r#"pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;
    fs::write(project_dir.join("src").join("math.rs"), new_file).await.unwrap();

    // Modify existing file
    let modified_main = r#"fn main() {
    println!("Hello, modified world!");
}
"#;
    fs::write(project_dir.join("src").join("main.rs"), modified_main).await.unwrap();

    // Sync from disk
    let sync_tool = WorkspaceSyncTool::new(ctx.clone());
    let sync_input = serde_json::json!({
        "workspace_id": workspace_id,
        "detect_moves": true,
        "re_parse": false,
    });

    let sync_result = sync_tool.execute(sync_input, &context).await.unwrap();
    assert!(sync_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&sync_result.content[0].text).unwrap();

    assert!(output["files_added"].as_u64().unwrap() >= 1); // math.rs
    assert!(output["files_modified"].as_u64().unwrap() >= 1); // main.rs
    assert_eq!(output["files_deleted"], 0);

    println!("Sync results: {:?}", output);
}

#[tokio::test]
async fn test_workspace_export() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    // Create and import workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Export to a new location
    let export_dir = temp_dir.path().join("exported");
    let export_tool = WorkspaceExportTool::new(ctx.clone());
    let export_input = serde_json::json!({
        "workspace_id": workspace_id,
        "target_path": export_dir.to_string_lossy(),
        "preserve_permissions": true,
        "preserve_timestamps": true,
    });

    let export_result = export_tool.execute(export_input, &context).await.unwrap();
    assert!(export_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&export_result.content[0].text).unwrap();

    assert!(output["files_exported"].as_u64().unwrap() >= 2);
    assert!(output["directories_created"].as_u64().unwrap() >= 1);
    assert!(output["bytes_written"].as_u64().unwrap() > 0);

    // Verify exported files exist
    assert!(export_dir.join("src").join("lib.rs").exists());
    assert!(export_dir.join("src").join("main.rs").exists());

    println!("Export results: {:?}", output);
}

#[tokio::test]
async fn test_workspace_archive() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();

    // Create workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Archive workspace
    let archive_tool = WorkspaceArchiveTool::new(ctx.clone());
    let archive_input = serde_json::json!({
        "workspace_id": workspace_id,
        "reason": "Testing archive functionality",
    });

    let archive_result = archive_tool.execute(archive_input, &context).await.unwrap();
    assert!(archive_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&archive_result.content[0].text).unwrap();

    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["status"], "archived");

    // Verify workspace is now read-only
    let get_tool = WorkspaceGetTool::new(ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let get_result = get_tool.execute(get_input, &context).await.unwrap();
    let get_output: serde_json::Value = serde_json::from_str(&get_result.content[0].text).unwrap();

    assert_eq!(get_output["read_only"], true);
}

#[tokio::test]
async fn test_workspace_delete() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();

    // Create workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Delete workspace
    let delete_tool = WorkspaceDeleteTool::new(ctx.clone());
    let delete_input = serde_json::json!({
        "workspace_id": workspace_id,
        "confirm": true,
    });

    let delete_result = delete_tool.execute(delete_input, &context).await.unwrap();
    assert!(delete_result.is_success());

    let output: serde_json::Value = serde_json::from_str(&delete_result.content[0].text).unwrap();

    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["status"], "deleted");
    assert!(output["message"].as_str().unwrap().contains("permanently deleted"));

    // Verify workspace no longer exists
    let get_tool = WorkspaceGetTool::new(ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let get_result = get_tool.execute(get_input, &context).await;
    assert!(get_result.is_err()); // Should fail - workspace deleted
}

#[tokio::test]
async fn test_workspace_delete_requires_confirmation() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("test-project");
    fs::create_dir(&project_dir).await.unwrap();

    // Create workspace
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let create_result = create_tool.execute(input, &context).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Try to delete without confirmation
    let delete_tool = WorkspaceDeleteTool::new(ctx.clone());
    let delete_input = serde_json::json!({
        "workspace_id": workspace_id,
        "confirm": false,
    });

    let delete_result = delete_tool.execute(delete_input, &context).await;
    assert!(delete_result.is_err()); // Should fail - no confirmation
}

#[tokio::test]
async fn test_workspace_create_detects_project_type() {
    let (ctx, temp_dir) = create_test_context().await;

    // Test Rust project
    let rust_dir = temp_dir.path().join("rust-project");
    fs::create_dir(&rust_dir).await.unwrap();
    fs::write(rust_dir.join("Cargo.toml"), "[package]\nname = \"test\"").await.unwrap();

    let tool = WorkspaceCreateTool::new(ctx.clone());
    let input = serde_json::json!({
        "name": "RustProject",
        "root_path": rust_dir.to_string_lossy(),
        "auto_import": false,
    });

    let context = mcp_sdk::prelude::ToolContext::default();
    let result = tool.execute(input, &context).await.unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    assert_eq!(output["workspace_type"], "code");
}

#[tokio::test]
async fn test_workspace_full_lifecycle() {
    let (ctx, temp_dir) = create_test_context().await;
    let project_dir = temp_dir.path().join("lifecycle-project");
    fs::create_dir(&project_dir).await.unwrap();
    create_test_project(&project_dir).await.unwrap();

    let context = mcp_sdk::prelude::ToolContext::default();

    // 1. Create workspace with import and parsing
    let create_tool = WorkspaceCreateTool::new(ctx.clone());
    let create_result = create_tool.execute(
        serde_json::json!({
            "name": "LifecycleProject",
            "root_path": project_dir.to_string_lossy(),
            "auto_import": true,
            "process_code": true,
        }),
        &context,
    ).await.unwrap();
    let create_output: serde_json::Value = serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    println!("Created workspace: {}", workspace_id);
    assert!(create_output["files_imported"].as_u64().unwrap() >= 2);
    assert!(create_output["units_extracted"].as_u64().unwrap() > 0);

    // 2. Get workspace info
    let get_tool = WorkspaceGetTool::new(ctx.clone());
    let get_result = get_tool.execute(
        serde_json::json!({
            "workspace_id": workspace_id,
            "include_stats": true,
        }),
        &context,
    ).await.unwrap();
    let get_output: serde_json::Value = serde_json::from_str(&get_result.content[0].text).unwrap();

    println!("Workspace stats: {:?}", get_output["stats"]);
    assert!(get_output["stats"]["total_files"].as_u64().unwrap() >= 2);

    // 3. Activate workspace
    let activate_tool = WorkspaceActivateTool::new(ctx.clone());
    activate_tool.execute(
        serde_json::json!({"workspace_id": workspace_id}),
        &context,
    ).await.unwrap();

    // 4. Modify filesystem
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    fs::write(
        project_dir.join("src").join("new.rs"),
        "pub fn new_function() {}",
    ).await.unwrap();

    // 5. Sync from disk
    let sync_tool = WorkspaceSyncTool::new(ctx.clone());
    let sync_result = sync_tool.execute(
        serde_json::json!({
            "workspace_id": workspace_id,
            "detect_moves": true,
            "re_parse": true,
        }),
        &context,
    ).await.unwrap();
    let sync_output: serde_json::Value = serde_json::from_str(&sync_result.content[0].text).unwrap();

    println!("Sync results: {:?}", sync_output);
    assert!(sync_output["files_added"].as_u64().unwrap() >= 1);

    // 6. Export workspace
    let export_dir = temp_dir.path().join("exported-lifecycle");
    let export_tool = WorkspaceExportTool::new(ctx.clone());
    export_tool.execute(
        serde_json::json!({
            "workspace_id": workspace_id,
            "target_path": export_dir.to_string_lossy(),
        }),
        &context,
    ).await.unwrap();

    assert!(export_dir.join("src").join("new.rs").exists());

    // 7. Archive workspace
    let archive_tool = WorkspaceArchiveTool::new(ctx.clone());
    archive_tool.execute(
        serde_json::json!({"workspace_id": workspace_id}),
        &context,
    ).await.unwrap();

    // 8. Delete workspace
    let delete_tool = WorkspaceDeleteTool::new(ctx.clone());
    delete_tool.execute(
        serde_json::json!({
            "workspace_id": workspace_id,
            "confirm": true,
        }),
        &context,
    ).await.unwrap();

    println!("Full lifecycle test completed successfully");
}
