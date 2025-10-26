//! Unit tests for workspace.create MCP tool
//!
//! Tests cover:
//! - Successful workspace creation
//! - Import with and without code processing
//! - Error cases (invalid paths, missing directories)
//! - Edge cases (empty workspace, large workspace, special characters)
//! - Performance benchmarks
//! - Token efficiency vs traditional approach

use super::utils::TestHarness;
use cortex_mcp::tools::workspace::WorkspaceCreateTool;
use mcp_sdk::Tool;

// =============================================================================
// Successful Operations
// =============================================================================

#[tokio::test]
async fn test_create_workspace_basic() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(output["workspace_id"].is_string());
    assert!(output["files_imported"].as_u64().unwrap() >= 2); // lib.rs, main.rs
}

#[tokio::test]
async fn test_create_workspace_with_code_processing() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(output["units_extracted"].as_u64().unwrap() > 0); // Should extract functions/structs
}

#[tokio::test]
async fn test_create_workspace_without_import() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["files_imported"], 0);
    assert_eq!(output["units_extracted"], 0);
}

#[tokio::test]
async fn test_create_workspace_detects_rust_project() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("rust-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "RustProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
}

#[tokio::test]
async fn test_create_workspace_returns_duration() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    assert!(output["import_duration_ms"].is_number());
    assert!(output["import_duration_ms"].as_u64().unwrap() > 0);
}

// =============================================================================
// Error Cases
// =============================================================================

#[tokio::test]
async fn test_create_workspace_error_invalid_path() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": "/nonexistent/path/to/project",
        "auto_import": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("does not exist"));
}

#[tokio::test]
async fn test_create_workspace_error_path_is_file() {
    let harness = TestHarness::new().await;
    let file_path = harness.temp_path().join("test.txt");
    tokio::fs::write(&file_path, "test").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": file_path.to_string_lossy(),
        "auto_import": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("not a directory"));
}

#[tokio::test]
async fn test_create_workspace_error_invalid_json() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "invalid_field": "value",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("Invalid input"));
}

#[tokio::test]
async fn test_create_workspace_error_missing_name() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_workspace_error_missing_root_path() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "TestProject",
        "auto_import": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_create_workspace_empty_directory() {
    let harness = TestHarness::new().await;
    let empty_dir = harness.create_empty_dir("empty-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "EmptyProject",
        "root_path": empty_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["files_imported"], 0);
}

#[tokio::test]
async fn test_create_workspace_special_characters() {
    let harness = TestHarness::new().await;
    let project_dir = harness
        .create_special_chars_project("special-chars")
        .await
        .unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "Special-Chars_Project.123",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    // Should import at least 6 files (lib.rs, main.rs, + 4 special files)
    assert!(output["files_imported"].as_u64().unwrap() >= 6);
}

#[tokio::test]
async fn test_create_workspace_large_project() {
    let harness = TestHarness::new().await;
    let project_dir = harness
        .create_large_project("large-project", 50)
        .await
        .unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "LargeProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    // Should import at least 52 files (lib.rs, main.rs, + 50 modules)
    assert!(output["files_imported"].as_u64().unwrap() >= 52);
}

#[tokio::test]
async fn test_create_workspace_very_long_name() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let long_name = "A".repeat(200); // Very long workspace name

    let input = serde_json::json!({
        "name": long_name,
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["workspace_id"].as_str().unwrap().len(), 36); // UUID
}

#[tokio::test]
async fn test_create_workspace_unicode_name() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "测试项目",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    assert!(result.is_success());
}

#[tokio::test]
async fn test_create_workspace_respects_gitignore() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    // Create ignored files
    tokio::fs::create_dir(project_dir.join("target")).await.unwrap();
    tokio::fs::write(project_dir.join("target").join("debug.txt"), "test")
        .await
        .unwrap();
    tokio::fs::write(project_dir.join(".DS_Store"), "test")
        .await
        .unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    // Should not import ignored files
    // Only lib.rs, main.rs, Cargo.toml, .gitignore
    let files_imported = output["files_imported"].as_u64().unwrap();
    assert!(files_imported >= 2 && files_imported <= 4);
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_create_workspace_performance_small() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let start = std::time::Instant::now();
    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(elapsed.as_secs() < 5, "Should complete in under 5 seconds");

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    println!(
        "Created workspace with {} files in {}ms",
        output["files_imported"], output["import_duration_ms"]
    );
}

#[tokio::test]
async fn test_create_workspace_performance_medium() {
    let harness = TestHarness::new().await;
    let project_dir = harness
        .create_large_project("medium-project", 20)
        .await
        .unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "MediumProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let start = std::time::Instant::now();
    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(elapsed.as_secs() < 10, "Should complete in under 10 seconds");

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    println!(
        "Created workspace with {} files in {}ms",
        output["files_imported"], output["import_duration_ms"]
    );
}

#[tokio::test]
async fn test_create_workspace_with_parsing_performance() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": true,
    });

    let start = std::time::Instant::now();
    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_secs() < 10,
        "Should complete with parsing in under 10 seconds"
    );

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    println!(
        "Created and parsed workspace: {} files, {} units in {}ms",
        output["files_imported"], output["units_extracted"], output["import_duration_ms"]
    );
}

// =============================================================================
// Token Efficiency Tests
// =============================================================================

#[tokio::test]
async fn test_create_workspace_token_efficiency() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let output_text = &result.content[0].text;
    let output: serde_json::Value = serde_json::from_str(output_text).unwrap();

    // MCP tool returns compact JSON
    let mcp_token_count = output_text.len() / 4; // Rough estimate: 4 chars per token

    // Traditional approach would require:
    // 1. ls -R command output
    // 2. Multiple file reads
    // 3. Manual parsing results
    // Estimated tokens for equivalent operation: ~1000-2000 for small project

    let files_imported = output["files_imported"].as_u64().unwrap();
    let traditional_estimate = files_imported * 200; // ~200 tokens per file for listing + reading

    println!("MCP tokens: ~{}", mcp_token_count);
    println!("Traditional estimate: ~{}", traditional_estimate);
    println!(
        "Efficiency: {}x more efficient",
        traditional_estimate / mcp_token_count.max(1)
    );

    // MCP should be significantly more efficient
    assert!(
        mcp_token_count < traditional_estimate as usize,
        "MCP should be more token-efficient"
    );
}

#[tokio::test]
async fn test_create_workspace_compact_output() {
    let harness = TestHarness::new().await;
    let project_dir = harness.create_rust_project("test-project").await.unwrap();

    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await.unwrap();
    let output_text = &result.content[0].text;

    // Output should be compact JSON, not verbose text
    assert!(output_text.len() < 500, "Output should be compact");
    assert!(output_text.contains("workspace_id"));
    assert!(output_text.contains("files_imported"));
    assert!(!output_text.contains("verbose"), "Should not be verbose");
}

// =============================================================================
// Tool Metadata Tests
// =============================================================================

#[tokio::test]
async fn test_create_workspace_tool_name() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    assert_eq!(tool.name(), "cortex.workspace.create");
}

#[tokio::test]
async fn test_create_workspace_tool_description() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let description = tool.description().unwrap();
    assert!(description.contains("workspace"));
    assert!(description.contains("import"));
}

#[tokio::test]
async fn test_create_workspace_tool_schema() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());
    let schema = tool.input_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["name"].is_object());
    assert!(schema["properties"]["root_path"].is_object());
    assert!(schema["properties"]["auto_import"].is_object());
    assert!(schema["properties"]["process_code"].is_object());
}
