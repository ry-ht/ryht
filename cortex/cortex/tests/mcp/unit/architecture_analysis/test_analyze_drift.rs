//! Unit Tests for cortex.arch.analyze_drift

use super::test_helpers::*;
use cortex_mcp::tools::architecture_analysis::ArchAnalyzeDriftTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_analyze_drift_basic() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchAnalyzeDriftTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "baseline_snapshot": null,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["drift_analysis"].is_null());
    }
}

#[tokio::test]
async fn test_analyze_drift_with_thresholds() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchAnalyzeDriftTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "thresholds": {
            "max_drift_score": 0.5,
            "max_new_violations": 10
        }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_drift_empty() {
    let fixture = ArchAnalysisTestFixture::new().await;

    let tool = ArchAnalyzeDriftTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}
