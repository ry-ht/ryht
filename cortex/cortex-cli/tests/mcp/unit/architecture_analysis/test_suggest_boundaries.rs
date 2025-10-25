//! Unit Tests for cortex.arch.suggest_boundaries

use super::test_helpers::*;
use cortex_mcp::tools::architecture_analysis::ArchSuggestBoundariesTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_suggest_boundaries_basic() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchSuggestBoundariesTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["suggestions"].is_null());
    }
}

#[tokio::test]
async fn test_suggest_boundaries_with_target() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchSuggestBoundariesTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "target_modularity": 0.8,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_suggest_boundaries_empty() {
    let fixture = ArchAnalysisTestFixture::new().await;

    let tool = ArchSuggestBoundariesTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}
