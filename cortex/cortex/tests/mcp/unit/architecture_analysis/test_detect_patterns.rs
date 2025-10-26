//! Unit Tests for cortex.arch.detect_patterns

use super::test_helpers::*;
use cortex_mcp::tools::architecture_analysis::ArchDetectPatternsTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_detect_patterns_layered() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchDetectPatternsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["patterns"].is_null());

        let patterns = data["patterns"].as_array().unwrap();
        // Should detect layered architecture
        assert!(patterns.iter().any(|p| p["pattern_type"].as_str().unwrap().contains("layer")));
    }
}

#[tokio::test]
async fn test_detect_patterns_hub_and_spoke() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_hub_and_spoke().await.unwrap();

    let tool = ArchDetectPatternsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_patterns_empty() {
    let fixture = ArchAnalysisTestFixture::new().await;

    let tool = ArchDetectPatternsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}
