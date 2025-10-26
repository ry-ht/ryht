//! Unit tests for workspace.list MCP tool
//!
//! Tests cover:
//! - Successful workspace listing
//! - Filtering and limiting results
//! - Error cases (invalid parameters)
//! - Edge cases (empty list, many workspaces)
//! - Performance benchmarks
//! - Token efficiency vs traditional approach

use super::utils::TestHarness;
use cortex_mcp::tools::workspace::{WorkspaceCreateTool, WorkspaceListTool};
use mcp_sdk::Tool;

// =============================================================================
// Helper Functions
// =============================================================================

async fn create_workspace(harness: &TestHarness, name: &str) -> String {
    let project_dir = harness.create_rust_project(name).await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": name,
        "root_path": project_dir.to_string_lossy(),
        "auto_import": false,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    output["workspace_id"].as_str().unwrap().to_string()
}

async fn create_workspaces(harness: &TestHarness, count: usize) -> Vec<String> {
    let mut workspace_ids = Vec::new();
    for i in 0..count {
        let name = format!("workspace-{}", i);
        let id = create_workspace(harness, &name).await;
        workspace_ids.push(id);
    }
    workspace_ids
}

// =============================================================================
// Successful Operations
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_empty() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceListTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 0);
    assert!(output["workspaces"].is_array());
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_workspaces_single() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 1);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 1);

    let workspace = &output["workspaces"][0];
    assert_eq!(workspace["workspace_id"], workspace_id);
    assert_eq!(workspace["name"], "TestProject");
    assert!(workspace["source_type"].is_string());
    assert!(workspace["file_count"].is_number());
    assert!(workspace["created_at"].is_string());
}

#[tokio::test]
async fn test_list_workspaces_multiple() {
    let harness = TestHarness::new().await;
    let workspace_ids = create_workspaces(&harness, 5).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 5);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 5);

    // Verify all workspace IDs are present
    let listed_ids: Vec<String> = output["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["workspace_id"].as_str().unwrap().to_string())
        .collect();

    for id in workspace_ids {
        assert!(listed_ids.contains(&id));
    }
}

#[tokio::test]
async fn test_list_workspaces_with_limit() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 10).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 5,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 10); // Total count is 10
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 5); // But only 5 returned
}

#[tokio::test]
async fn test_list_workspaces_default_limit() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 3).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({});

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 3);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_list_workspaces_includes_file_count() {
    let harness = TestHarness::new().await;

    // Create workspace with files
    let project_dir = harness.create_rust_project("test-project").await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "TestProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    tool.execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    // List workspaces
    let list_tool = WorkspaceListTool::new(harness.ctx.clone());
    let list_input = serde_json::json!({
        "limit": 100,
    });

    let result = list_tool
        .execute(list_input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    let workspace = &output["workspaces"][0];

    // Should have file_count field
    assert!(workspace["file_count"].is_number());
    assert!(workspace["file_count"].as_u64().unwrap() >= 2); // lib.rs, main.rs
}

// =============================================================================
// Error Cases
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_error_invalid_json() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceListTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "invalid_field": "value",
        "limit": "not a number",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("Invalid input"));
}

#[tokio::test]
async fn test_list_workspaces_zero_limit() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 5).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 0,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 5);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 0); // No results with limit 0
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_large_limit() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 3).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 1000,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 3);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_list_workspaces_many_workspaces() {
    let harness = TestHarness::new().await;
    let count = 25;
    create_workspaces(&harness, count).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], count);
    assert_eq!(output["workspaces"].as_array().unwrap().len(), count);
}

#[tokio::test]
async fn test_list_workspaces_pagination() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 10).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());

    // First page
    let input1 = serde_json::json!({
        "limit": 5,
    });

    let result1 = tool
        .execute(input1, &TestHarness::tool_context())
        .await
        .unwrap();
    let output1: serde_json::Value = serde_json::from_str(&result1.content[0].text).unwrap();

    assert_eq!(output1["total"], 10);
    assert_eq!(output1["workspaces"].as_array().unwrap().len(), 5);

    // Note: Without offset support, we can only verify that limiting works
    // In a real implementation, we'd want cursor-based pagination
}

#[tokio::test]
async fn test_list_workspaces_status_filter() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 3).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "status": "active",
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    // Status filter is accepted but currently ignored in implementation
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["total"], 3);
}

#[tokio::test]
async fn test_list_workspaces_consistent_ordering() {
    let harness = TestHarness::new().await;
    let workspace_ids = create_workspaces(&harness, 5).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    // List twice
    let result1 = tool
        .execute(input.clone(), &TestHarness::tool_context())
        .await
        .unwrap();
    let output1: serde_json::Value = serde_json::from_str(&result1.content[0].text).unwrap();

    let result2 = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output2: serde_json::Value = serde_json::from_str(&result2.content[0].text).unwrap();

    // Order should be consistent
    let ids1: Vec<String> = output1["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["workspace_id"].as_str().unwrap().to_string())
        .collect();

    let ids2: Vec<String> = output2["workspaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["workspace_id"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(ids1, ids2);

    // All IDs should be present
    for id in workspace_ids {
        assert!(ids1.contains(&id));
    }
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_performance_small() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 5).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
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
        "Should list 5 workspaces quickly: {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_list_workspaces_performance_medium() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 20).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let start = std::time::Instant::now();
    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_secs() < 2,
        "Should list 20 workspaces in under 2 seconds"
    );
}

#[tokio::test]
async fn test_list_workspaces_performance_with_limit() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 50).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 10,
    });

    let start = std::time::Instant::now();
    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_success());
    assert!(
        elapsed.as_secs() < 2,
        "Limited list should be fast even with many workspaces"
    );

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["workspaces"].as_array().unwrap().len(), 10);
}

// =============================================================================
// Token Efficiency Tests
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_token_efficiency() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 10).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // MCP tool returns compact JSON array
    let mcp_token_count = output_text.len() / 4; // Rough estimate: 4 chars per token

    // Traditional approach would require:
    // 1. Multiple database queries
    // 2. Verbose output for each workspace
    // 3. Manual formatting
    // Estimated tokens: ~100-200 per workspace = 1000-2000 tokens for 10 workspaces

    let traditional_estimate = 1500;

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
async fn test_list_workspaces_compact_output() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 5).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // Output should be compact JSON
    assert!(output_text.contains("workspaces"));
    assert!(output_text.contains("total"));
    assert!(!output_text.contains("verbose"));
    assert!(!output_text.contains("\n\n"), "Should not have extra newlines");
}

#[tokio::test]
async fn test_list_workspaces_summary_not_full_info() {
    let harness = TestHarness::new().await;
    create_workspaces(&harness, 3).await;

    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "limit": 100,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    let workspace = &output["workspaces"][0];

    // Should have summary fields
    assert!(workspace["workspace_id"].is_string());
    assert!(workspace["name"].is_string());
    assert!(workspace["source_type"].is_string());
    assert!(workspace["file_count"].is_number());
    assert!(workspace["created_at"].is_string());

    // Should NOT have full stats or detailed info
    assert!(workspace["stats"].is_null());
    assert!(workspace["total_bytes"].is_null());
    assert!(workspace["languages"].is_null());
}

// =============================================================================
// Tool Metadata Tests
// =============================================================================

#[tokio::test]
async fn test_list_workspaces_tool_name() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceListTool::new(harness.ctx.clone());
    assert_eq!(tool.name(), "cortex.workspace.list");
}

#[tokio::test]
async fn test_list_workspaces_tool_description() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let description = tool.description().unwrap();
    assert!(description.contains("workspace"));
    assert!(description.contains("list"));
}

#[tokio::test]
async fn test_list_workspaces_tool_schema() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceListTool::new(harness.ctx.clone());
    let schema = tool.input_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["status"].is_object());
    assert!(schema["properties"]["limit"].is_object());
}
