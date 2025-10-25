//! Unit Tests for cortex.ai.explain_code
//!
//! Tests cover:
//! - Basic code explanation
//! - Complexity analysis
//! - Algorithm explanation
//! - Usage examples generation
//! - Different detail levels
//! - Dependency explanation

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiExplainCodeTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_explain_code_basic() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "calculate_fibonacci",
        fixtures::DOCUMENTED_FUNCTION,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "detail_level": "detailed",
        "include_examples": true,
        "explain_dependencies": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Check required fields
        assert!(!data["summary"].is_null());
        assert!(!data["detailed_explanation"].is_null());
        assert!(!data["purpose"].is_null());
        assert!(!data["examples"].is_null());

        // Should have explanation content
        assert!(data["summary"].as_str().unwrap().len() > 0);
        assert!(data["detailed_explanation"].as_str().unwrap().len() > 0);
    }
}

#[tokio::test]
async fn test_explain_code_with_examples() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "calculate_fibonacci",
        fixtures::DOCUMENTED_FUNCTION,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should include examples
        let examples = data["examples"].as_array().unwrap();
        assert!(examples.len() > 0, "Should generate usage examples");

        // Check example structure
        let first_example = &examples[0];
        assert!(!first_example["scenario"].is_null());
        assert!(!first_example["example_code"].is_null());
        assert!(!first_example["explanation"].is_null());
    }
}

#[tokio::test]
async fn test_explain_code_without_examples() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "calculate_fibonacci",
        fixtures::DOCUMENTED_FUNCTION,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should not include examples
        let examples = data["examples"].as_array().unwrap();
        assert_eq!(examples.len(), 0, "Should not generate examples when disabled");
    }
}

#[tokio::test]
async fn test_explain_code_complexity_analysis() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "detail_level": "detailed",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should include complexity analysis
        assert!(!data["complexity_analysis"].is_null());

        let complexity = data["complexity_analysis"].as_str().unwrap();
        // Should detect nested loops
        assert!(complexity.contains("O(n") || complexity.contains("loop"), "Should analyze time complexity");
    }
}

#[tokio::test]
async fn test_explain_code_algorithm_detection() {
    let fixture = AiAssistedTestFixture::new().await;

    // Code using HashMap
    let hashmap_code = r#"
fn count_occurrences(items: Vec<i32>) -> HashMap<i32, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item).or_insert(0) += 1;
    }
    counts
}
    "#;

    let unit_id = fixture.create_test_unit(
        "count_occurrences",
        hashmap_code,
        "src/stats.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should explain algorithm
        assert!(!data["algorithm_explanation"].is_null());

        let algo = data["algorithm_explanation"].as_str().unwrap();
        assert!(algo.contains("hash") || algo.contains("HashMap"), "Should detect hash-based data structure");
    }
}

#[tokio::test]
async fn test_explain_code_async_detection() {
    let fixture = AiAssistedTestFixture::new().await;

    let async_code = r#"
async fn fetch_data(url: &str) -> Result<String, Error> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}
    "#;

    let unit_id = fixture.create_test_unit(
        "fetch_data",
        async_code,
        "src/network.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let explanation = data["detailed_explanation"].as_str().unwrap();
        assert!(explanation.contains("async"), "Should mention async operations");
    }
}

#[tokio::test]
async fn test_explain_code_error_propagation() {
    let fixture = AiAssistedTestFixture::new().await;

    let error_code = r#"
fn parse_number(s: &str) -> Result<i32, ParseIntError> {
    let num = s.parse::<i32>()?;
    Ok(num)
}
    "#;

    let unit_id = fixture.create_test_unit(
        "parse_number",
        error_code,
        "src/parser.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let explanation = data["detailed_explanation"].as_str().unwrap();
        assert!(explanation.contains("?") || explanation.contains("error"), "Should mention error propagation");

        // Should generate error handling example
        let examples = data["examples"].as_array().unwrap();
        assert!(examples.iter().any(|e| e["scenario"].as_str().unwrap().contains("error")));
    }
}

#[tokio::test]
async fn test_explain_code_nonexistent_unit() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "nonexistent-unit-id",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent unit");
}

#[tokio::test]
async fn test_explain_code_different_detail_levels() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "calculate_fibonacci",
        fixtures::DOCUMENTED_FUNCTION,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());

    // Test with different detail levels
    for detail_level in ["brief", "detailed", "comprehensive"] {
        let input = json!({
            "unit_id": unit_id.clone(),
            "detail_level": detail_level,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Should work with detail level: {}", detail_level);
    }
}

#[tokio::test]
async fn test_explain_code_performance() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "calculate_fibonacci",
        fixtures::DOCUMENTED_FUNCTION,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiExplainCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
    });

    let (_, duration) = fixture.execute_tool(&tool, input).await;

    // Should complete reasonably fast (< 1000ms for in-memory DB)
    assert!(duration < 1000, "Explanation should complete in less than 1 second");
}
