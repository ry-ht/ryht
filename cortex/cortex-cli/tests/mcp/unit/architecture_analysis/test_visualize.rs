//! Unit Tests for cortex.arch.visualize

use super::test_helpers::*;
use cortex_mcp::tools::architecture_analysis::ArchVisualizeTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_visualize_basic() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchVisualizeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "output_format": "mermaid",
        "view_type": "dependency",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["visualization"].is_null());
    }
}

#[tokio::test]
async fn test_visualize_empty_scope() {
    let fixture = ArchAnalysisTestFixture::new().await;

    let tool = ArchVisualizeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "nonexistent/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_visualize_different_formats() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchVisualizeTool::new(fixture.ctx.clone());

    for format in ["mermaid", "dot", "json"] {
        let input = json!({
            "scope_path": "src/",
            "output_format": format,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Should work with format: {}", format);
    }
}
