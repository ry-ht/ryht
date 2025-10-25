//! Unit Tests for cortex.arch.check_violations

use super::test_helpers::*;
use cortex_mcp::tools::architecture_analysis::ArchCheckViolationsTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_check_violations_circular() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_circular_dependency().await.unwrap();

    let tool = ArchCheckViolationsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "rules": [
            {"rule_type": "no_circular_dependencies"}
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let violations = data["violations"].as_array().unwrap();
        // Should detect circular dependency violation
        assert!(violations.len() > 0, "Should detect circular dependency");
    }
}

#[tokio::test]
async fn test_check_violations_layered() {
    let fixture = ArchAnalysisTestFixture::new().await;
    fixture.create_layered_architecture().await.unwrap();

    let tool = ArchCheckViolationsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "rules": [
            {"rule_type": "enforce_layered_architecture"}
        ]
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_check_violations_empty() {
    let fixture = ArchAnalysisTestFixture::new().await;

    let tool = ArchCheckViolationsTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "rules": []
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}
