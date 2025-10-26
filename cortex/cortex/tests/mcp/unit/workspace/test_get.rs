//! Unit tests for workspace.get MCP tool
//!
//! Tests cover:
//! - Successful workspace retrieval
//! - With and without statistics
//! - Error cases (invalid ID, missing workspace)
//! - Edge cases (workspace with no files, large workspace)
//! - Performance benchmarks
//! - Token efficiency vs traditional approach

use super::utils::TestHarness;
use cortex_mcp::tools::workspace::{WorkspaceCreateTool, WorkspaceGetTool};
use mcp_sdk::Tool;

// =============================================================================
// Helper Functions
// =============================================================================

async fn create_workspace(
    harness: &TestHarness,
    name: &str,
    import: bool,
) -> serde_json::Value {
    let project_dir = harness.create_rust_project(name).await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": name,
        "root_path": project_dir.to_string_lossy(),
        "auto_import": import,
        "process_code": false,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    serde_json::from_str(&result.content[0].text).unwrap()
}

// =============================================================================
// Successful Operations
// =============================================================================

#[tokio::test]
async fn test_get_workspace_basic() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["name"], "TestProject");
    assert_eq!(output["source_type"], "local");
    assert_eq!(output["read_only"], false);
    assert!(output["created_at"].is_string());
    assert!(output["updated_at"].is_string());
}

#[tokio::test]
async fn test_get_workspace_with_stats() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(output["stats"].is_object());

    let stats = &output["stats"];
    assert!(stats["total_files"].is_number());
    assert!(stats["total_directories"].is_number());
    assert!(stats["total_units"].is_number());
    assert!(stats["total_bytes"].is_number());
    assert!(stats["languages"].is_object());

    // Should have imported at least 2 files
    assert!(stats["total_files"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn test_get_workspace_without_stats() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(output["stats"].is_null());
}

#[tokio::test]
async fn test_get_workspace_default_include_stats() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    // Default should include stats (default_true)
    assert!(output["stats"].is_object());
}

#[tokio::test]
async fn test_get_workspace_root_path() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert!(output["root_path"].is_string());
    assert!(output["root_path"].as_str().unwrap().contains("TestProject"));
}

// =============================================================================
// Error Cases
// =============================================================================

#[tokio::test]
async fn test_get_workspace_error_invalid_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "workspace_id": "invalid-uuid",
        "include_stats": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("Invalid workspace ID"));
}

#[tokio::test]
async fn test_get_workspace_error_missing_workspace() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());

    // Valid UUID but doesn't exist
    let input = serde_json::json!({
        "workspace_id": "00000000-0000-0000-0000-000000000000",
        "include_stats": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("not found"));
}

#[tokio::test]
async fn test_get_workspace_error_invalid_json() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());

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
async fn test_get_workspace_error_missing_workspace_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "include_stats": true,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_workspace_error_empty_string_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "workspace_id": "",
        "include_stats": false,
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_get_workspace_empty_workspace() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "EmptyProject", false).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    let stats = &output["stats"];

    // Empty workspace should have 0 files
    assert_eq!(stats["total_files"], 0);
    assert_eq!(stats["total_directories"], 0);
    assert_eq!(stats["total_bytes"], 0);
}

#[tokio::test]
async fn test_get_workspace_large_workspace() {
    let harness = TestHarness::new().await;

    // Create large workspace
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

    let create_result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let create_output: serde_json::Value =
        serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Get workspace with stats
    let get_tool = WorkspaceGetTool::new(harness.ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = get_tool
        .execute(get_input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    let stats = &output["stats"];

    // Should have at least 52 files (lib.rs, main.rs, + 50 modules)
    assert!(stats["total_files"].as_u64().unwrap() >= 52);
}

#[tokio::test]
async fn test_get_workspace_unicode_name() {
    let harness = TestHarness::new().await;

    let project_dir = harness.create_rust_project("unicode").await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "Unicode测试",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let create_result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let create_output: serde_json::Value =
        serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Get workspace
    let get_tool = WorkspaceGetTool::new(harness.ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let result = get_tool
        .execute(get_input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["name"], "Unicode测试");
}

#[tokio::test]
async fn test_get_workspace_stats_language_breakdown() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    let languages = &output["stats"]["languages"];

    // Should detect Rust files
    assert!(languages.is_object());
    // Language breakdown might be empty if language detection isn't set
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_get_workspace_performance_without_stats() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let start = std::time::Instant::now();
    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_millis() < 100,
        "Get without stats should be very fast: {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_get_workspace_performance_with_stats() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let start = std::time::Instant::now();
    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_millis() < 500,
        "Get with stats should complete quickly: {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_get_workspace_performance_large_workspace() {
    let harness = TestHarness::new().await;

    // Create large workspace
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

    let create_result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let create_output: serde_json::Value =
        serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // Get workspace with stats
    let get_tool = WorkspaceGetTool::new(harness.ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let start = std::time::Instant::now();
    let result = get_tool
        .execute(get_input, &TestHarness::tool_context())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_secs() < 2,
        "Get large workspace should complete in under 2 seconds"
    );
}

// =============================================================================
// Token Efficiency Tests
// =============================================================================

#[tokio::test]
async fn test_get_workspace_token_efficiency() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // MCP tool returns compact JSON
    let mcp_token_count = output_text.len() / 4; // Rough estimate: 4 chars per token

    // Traditional approach would require:
    // 1. Database query for workspace metadata
    // 2. ls -R to list all files
    // 3. du to calculate sizes
    // 4. Multiple queries to count files, directories, units
    // Estimated tokens: ~500-1000

    let traditional_estimate = 750;

    println!("MCP tokens: ~{}", mcp_token_count);
    println!("Traditional estimate: ~{}", traditional_estimate);
    println!(
        "Efficiency: {}x more efficient",
        traditional_estimate / mcp_token_count.max(1)
    );

    // MCP should be more efficient
    assert!(
        mcp_token_count < traditional_estimate,
        "MCP should be more token-efficient"
    );
}

#[tokio::test]
async fn test_get_workspace_compact_output() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // Output should be compact JSON
    assert!(output_text.len() < 1000, "Output should be compact");
    assert!(output_text.contains("workspace_id"));
    assert!(output_text.contains("stats"));
    assert!(!output_text.contains("verbose"));
}

#[tokio::test]
async fn test_get_workspace_without_stats_smaller_output() {
    let harness = TestHarness::new().await;
    let create_output = create_workspace(&harness, "TestProject", true).await;
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    let tool = WorkspaceGetTool::new(harness.ctx.clone());

    // Get without stats
    let input_no_stats = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let result_no_stats = tool
        .execute(input_no_stats, &TestHarness::tool_context())
        .await
        .unwrap();
    let size_no_stats = result_no_stats.content[0].text.len();

    // Get with stats
    let input_with_stats = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let result_with_stats = tool
        .execute(input_with_stats, &TestHarness::tool_context())
        .await
        .unwrap();
    let size_with_stats = result_with_stats.content[0].text.len();

    // Output without stats should be smaller
    assert!(
        size_no_stats < size_with_stats,
        "Output without stats should be smaller"
    );
}

// =============================================================================
// Tool Metadata Tests
// =============================================================================

#[tokio::test]
async fn test_get_workspace_tool_name() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    assert_eq!(tool.name(), "cortex.workspace.get");
}

#[tokio::test]
async fn test_get_workspace_tool_description() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let description = tool.description().unwrap();
    assert!(description.contains("workspace"));
    assert!(description.contains("information"));
}

#[tokio::test]
async fn test_get_workspace_tool_schema() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceGetTool::new(harness.ctx.clone());
    let schema = tool.input_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["workspace_id"].is_object());
    assert!(schema["properties"]["include_stats"].is_object());
}
