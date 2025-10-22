//! Unit tests for workspace.activate MCP tool
//!
//! Tests cover:
//! - Successful workspace activation
//! - Error cases (invalid ID, missing workspace)
//! - Edge cases (activating already active workspace)
//! - Performance benchmarks
//! - Token efficiency vs traditional approach

use super::utils::TestHarness;
use cortex_mcp::tools::workspace::{
    WorkspaceActivateTool, WorkspaceCreateTool, WorkspaceGetTool,
};
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

// =============================================================================
// Successful Operations
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_basic() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["workspace_id"], workspace_id);
    assert_eq!(output["name"], "TestProject");
    assert_eq!(output["status"], "activated");
}

#[tokio::test]
async fn test_activate_workspace_verifies_exists() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    // Verify workspace is accessible after activation
    let get_tool = WorkspaceGetTool::new(harness.ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": false,
    });

    let get_result = get_tool
        .execute(get_input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(get_result.is_success());
}

#[tokio::test]
async fn test_activate_workspace_returns_name() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "MyWorkspace").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["name"], "MyWorkspace");
}

#[tokio::test]
async fn test_activate_workspace_multiple_times() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    // Activate first time
    let result1 = tool
        .execute(input.clone(), &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result1.is_success());

    // Activate second time (should still succeed)
    let result2 = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result2.is_success());

    let output: serde_json::Value = serde_json::from_str(&result2.content[0].text).unwrap();
    assert_eq!(output["status"], "activated");
}

#[tokio::test]
async fn test_activate_workspace_switch_between_workspaces() {
    let harness = TestHarness::new().await;
    let workspace_id1 = create_workspace(&harness, "Workspace1").await;
    let workspace_id2 = create_workspace(&harness, "Workspace2").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    // Activate first workspace
    let input1 = serde_json::json!({
        "workspace_id": workspace_id1,
    });

    let result1 = tool
        .execute(input1, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result1.is_success());

    // Switch to second workspace
    let input2 = serde_json::json!({
        "workspace_id": workspace_id2,
    });

    let result2 = tool
        .execute(input2, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result2.is_success());

    let output: serde_json::Value = serde_json::from_str(&result2.content[0].text).unwrap();
    assert_eq!(output["workspace_id"], workspace_id2);
    assert_eq!(output["name"], "Workspace2");
}

// =============================================================================
// Error Cases
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_error_invalid_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "workspace_id": "invalid-uuid",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("Invalid workspace ID"));
}

#[tokio::test]
async fn test_activate_workspace_error_missing_workspace() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    // Valid UUID but doesn't exist
    let input = serde_json::json!({
        "workspace_id": "00000000-0000-0000-0000-000000000000",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{:?}", err);
    assert!(err_msg.contains("not found"));
}

#[tokio::test]
async fn test_activate_workspace_error_invalid_json() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

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
async fn test_activate_workspace_error_missing_workspace_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    let input = serde_json::json!({});

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_activate_workspace_error_empty_string_id() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "workspace_id": "",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_activate_workspace_error_malformed_uuid() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "workspace_id": "not-a-uuid-123",
    });

    let result = tool.execute(input, &TestHarness::tool_context()).await;
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_with_special_name() {
    let harness = TestHarness::new().await;

    let project_dir = harness.create_rust_project("special").await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "Special-Chars_123.Project",
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

    // Activate workspace
    let activate_tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let activate_input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = activate_tool
        .execute(activate_input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["name"], "Special-Chars_123.Project");
}

#[tokio::test]
async fn test_activate_workspace_with_unicode_name() {
    let harness = TestHarness::new().await;

    let project_dir = harness.create_rust_project("unicode").await.unwrap();
    let tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let input = serde_json::json!({
        "name": "项目工作区",
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

    // Activate workspace
    let activate_tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let activate_input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = activate_tool
        .execute(activate_input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());

    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    assert_eq!(output["name"], "项目工作区");
}

#[tokio::test]
async fn test_activate_workspace_uppercase_uuid() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id.to_uppercase(),
    });

    // UUID parsing should be case-insensitive
    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(result.is_success());
}

// =============================================================================
// Performance Tests
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_performance() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
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
        "Activation should be very fast: {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_activate_workspace_performance_repeated() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    // Activate 10 times
    let start = std::time::Instant::now();
    for _ in 0..10 {
        tool.execute(input.clone(), &TestHarness::tool_context())
            .await
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 1000,
        "10 activations should complete in under 1 second"
    );
}

#[tokio::test]
async fn test_activate_workspace_performance_switching() {
    let harness = TestHarness::new().await;
    let workspace_id1 = create_workspace(&harness, "Workspace1").await;
    let workspace_id2 = create_workspace(&harness, "Workspace2").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());

    // Switch between workspaces 10 times
    let start = std::time::Instant::now();
    for i in 0..10 {
        let id = if i % 2 == 0 {
            &workspace_id1
        } else {
            &workspace_id2
        };

        let input = serde_json::json!({
            "workspace_id": id,
        });

        tool.execute(input, &TestHarness::tool_context())
            .await
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 1000,
        "Switching 10 times should complete in under 1 second"
    );
}

// =============================================================================
// Token Efficiency Tests
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_token_efficiency() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // MCP tool returns minimal JSON
    let mcp_token_count = output_text.len() / 4; // Rough estimate: 4 chars per token

    // Traditional approach would require:
    // 1. Command to check if workspace exists
    // 2. Command to set active workspace
    // 3. Confirmation output
    // Estimated tokens: ~100-200

    let traditional_estimate = 150;

    println!("MCP tokens: ~{}", mcp_token_count);
    println!("Traditional estimate: ~{}", traditional_estimate);
    println!(
        "Efficiency: {}x more efficient",
        traditional_estimate / mcp_token_count.max(1)
    );

    // MCP should be comparable or more efficient
    assert!(
        mcp_token_count <= traditional_estimate,
        "MCP should be token-efficient"
    );
}

#[tokio::test]
async fn test_activate_workspace_compact_output() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output_text = &result.content[0].text;

    // Output should be compact JSON
    assert!(output_text.len() < 200, "Output should be very compact");
    assert!(output_text.contains("workspace_id"));
    assert!(output_text.contains("status"));
    assert!(!output_text.contains("verbose"));
}

#[tokio::test]
async fn test_activate_workspace_minimal_fields() {
    let harness = TestHarness::new().await;
    let workspace_id = create_workspace(&harness, "TestProject").await;

    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let result = tool
        .execute(input, &TestHarness::tool_context())
        .await
        .unwrap();
    let output: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();

    // Should only have essential fields
    assert!(output["workspace_id"].is_string());
    assert!(output["name"].is_string());
    assert!(output["status"].is_string());

    // Should NOT include unnecessary details
    assert!(output["stats"].is_null());
    assert!(output["file_count"].is_null());
    assert!(output["created_at"].is_null());
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_workflow() {
    let harness = TestHarness::new().await;

    // 1. Create workspace
    let project_dir = harness.create_rust_project("workflow").await.unwrap();
    let create_tool = WorkspaceCreateTool::new(harness.ctx.clone());

    let create_input = serde_json::json!({
        "name": "WorkflowProject",
        "root_path": project_dir.to_string_lossy(),
        "auto_import": true,
        "process_code": false,
    });

    let create_result = create_tool
        .execute(create_input, &TestHarness::tool_context())
        .await
        .unwrap();
    let create_output: serde_json::Value =
        serde_json::from_str(&create_result.content[0].text).unwrap();
    let workspace_id = create_output["workspace_id"].as_str().unwrap();

    // 2. Activate workspace
    let activate_tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let activate_input = serde_json::json!({
        "workspace_id": workspace_id,
    });

    let activate_result = activate_tool
        .execute(activate_input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(activate_result.is_success());

    // 3. Get workspace info
    let get_tool = WorkspaceGetTool::new(harness.ctx.clone());
    let get_input = serde_json::json!({
        "workspace_id": workspace_id,
        "include_stats": true,
    });

    let get_result = get_tool
        .execute(get_input, &TestHarness::tool_context())
        .await
        .unwrap();
    assert!(get_result.is_success());

    let get_output: serde_json::Value = serde_json::from_str(&get_result.content[0].text).unwrap();
    assert_eq!(get_output["name"], "WorkflowProject");
    assert!(get_output["stats"]["total_files"].as_u64().unwrap() >= 2);
}

// =============================================================================
// Tool Metadata Tests
// =============================================================================

#[tokio::test]
async fn test_activate_workspace_tool_name() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    assert_eq!(tool.name(), "cortex.workspace.activate");
}

#[tokio::test]
async fn test_activate_workspace_tool_description() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let description = tool.description().unwrap();
    assert!(description.contains("workspace"));
    assert!(description.contains("active"));
}

#[tokio::test]
async fn test_activate_workspace_tool_schema() {
    let harness = TestHarness::new().await;
    let tool = WorkspaceActivateTool::new(harness.ctx.clone());
    let schema = tool.input_schema();

    assert!(schema.is_object());
    assert!(schema["properties"]["workspace_id"].is_object());
}
