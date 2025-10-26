//! Unit Tests for cortex.ai.review_code
//!
//! Tests cover:
//! - Comprehensive code review
//! - Readability checks
//! - Performance checks
//! - Security checks
//! - Best practices checks
//! - Overall scoring
//! - Confidence filtering
//! - Multiple review aspects

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiReviewCodeTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_review_code_basic() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "bad_code",
        fixtures::QUALITY_ISSUES,
        "src/bad.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/bad.rs",
        "review_aspects": ["readability", "performance", "security", "best_practices"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should return review results
        assert!(!data["overall_score"].is_null());
        assert!(!data["comments"].is_null());
        assert!(!data["summary"].is_null());
        assert!(!data["strengths"].is_null());
        assert!(!data["improvements"].is_null());

        let score = data["overall_score"].as_f64().unwrap();
        assert!(score >= 0.0 && score <= 1.0, "Score should be between 0 and 1");
    }
}

#[tokio::test]
async fn test_review_code_readability() {
    let fixture = AiAssistedTestFixture::new().await;

    // Long function with poor naming
    let long_badly_named = format!(
        "fn a() {{\n{}\n}}",
        "    println!(\"test\");\n".repeat(150)
    );

    fixture.create_test_unit(
        "a",
        &long_badly_named,
        "src/util.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/util.rs",
        "review_aspects": ["readability"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();
        assert!(comments.len() > 0, "Should find readability issues");

        // Check for readability comments
        let readability_comments: Vec<_> = comments.iter()
            .filter(|c| c["category"].as_str().unwrap() == "readability")
            .collect();

        assert!(readability_comments.len() > 0, "Should have readability comments");

        // Check comment structure
        let first = readability_comments[0];
        assert!(!first["file_path"].is_null());
        assert!(!first["line"].is_null());
        assert!(!first["severity"].is_null());
        assert!(!first["comment"].is_null());
        assert!(!first["confidence"].is_null());
    }
}

#[tokio::test]
async fn test_review_code_performance() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "process_strings",
        fixtures::EXCESSIVE_CLONES,
        "src/processor.rs",
        1
    ).await.unwrap();

    fixture.create_test_unit(
        "find_pairs",
        fixtures::NESTED_LOOPS,
        "src/search.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "review_aspects": ["performance"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();

        // Should find performance issues
        let perf_comments: Vec<_> = comments.iter()
            .filter(|c| c["category"].as_str().unwrap() == "performance")
            .collect();

        assert!(perf_comments.len() > 0, "Should find performance issues");

        // Should detect excessive clones or nested loops
        let has_clone_issue = perf_comments.iter().any(|c| c["comment"].as_str().unwrap().contains("clone"));
        let has_loop_issue = perf_comments.iter().any(|c| c["comment"].as_str().unwrap().contains("loop") || c["comment"].as_str().unwrap().contains("O(n"));

        assert!(has_clone_issue || has_loop_issue, "Should detect clone or loop performance issues");
    }
}

#[tokio::test]
async fn test_review_code_security() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "unsafe_operation",
        fixtures::SECURITY_ISSUES,
        "src/unsafe.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/unsafe.rs",
        "review_aspects": ["security"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();

        // Should find security issues
        let security_comments: Vec<_> = comments.iter()
            .filter(|c| c["category"].as_str().unwrap() == "security")
            .collect();

        assert!(security_comments.len() > 0, "Should find security issues");

        // Should detect unsafe blocks or unwrap
        let has_unsafe = security_comments.iter().any(|c| c["comment"].as_str().unwrap().contains("unsafe"));
        let has_unwrap = security_comments.iter().any(|c| c["comment"].as_str().unwrap().contains("unwrap"));

        assert!(has_unsafe || has_unwrap, "Should detect unsafe blocks or unwrap usage");

        // Critical severity for unsafe
        let critical_comments: Vec<_> = security_comments.iter()
            .filter(|c| c["severity"].as_str().unwrap() == "critical")
            .collect();

        assert!(critical_comments.len() > 0, "Unsafe blocks should be flagged as critical");
    }
}

#[tokio::test]
async fn test_review_code_best_practices() {
    let fixture = AiAssistedTestFixture::new().await;

    let code_with_todo = r#"
fn incomplete_function() -> Result<(), String> {
    // TODO: Implement this
    Ok(())
}
    "#;

    fixture.create_test_unit(
        "incomplete_function",
        code_with_todo,
        "src/incomplete.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/incomplete.rs",
        "review_aspects": ["best_practices"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();

        // Should find TODO/FIXME comments
        let todo_comments: Vec<_> = comments.iter()
            .filter(|c| c["comment"].as_str().unwrap().contains("TODO") || c["comment"].as_str().unwrap().contains("FIXME"))
            .collect();

        assert!(todo_comments.len() > 0, "Should find TODO comments");
    }
}

#[tokio::test]
async fn test_review_code_severity_levels() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "unsafe_operation",
        fixtures::SECURITY_ISSUES,
        "src/unsafe.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/unsafe.rs",
        "review_aspects": ["security", "readability"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();

        // Should have different severity levels
        let severities: std::collections::HashSet<_> = comments.iter()
            .map(|c| c["severity"].as_str().unwrap())
            .collect();

        // Should have at least critical or major severity
        assert!(severities.contains("critical") || severities.contains("major") || severities.contains("minor"));
    }
}

#[tokio::test]
async fn test_review_code_overall_score() {
    let fixture = AiAssistedTestFixture::new().await;

    // Clean, good code
    let good_code = r#"
/// Calculates the sum of two numbers
///
/// # Arguments
/// * `a` - First number
/// * `b` - Second number
///
/// # Returns
/// The sum of a and b
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
    "#;

    fixture.create_test_unit(
        "add",
        good_code,
        "src/good.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/good.rs",
        "review_aspects": ["readability", "performance", "security", "best_practices"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let score = data["overall_score"].as_f64().unwrap();

        // Good code should have high score
        assert!(score > 0.8, "Good code should have high quality score");

        // Should identify strengths
        let strengths = data["strengths"].as_array().unwrap();
        assert!(strengths.len() > 0, "Should identify code strengths");
    }
}

#[tokio::test]
async fn test_review_code_suggestions() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "bad_code",
        fixtures::QUALITY_ISSUES,
        "src/bad.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/bad.rs",
        "review_aspects": ["readability", "performance", "security"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let comments = data["comments"].as_array().unwrap();

        // Some comments should have suggestions
        let with_suggestions = comments.iter()
            .filter(|c| !c["suggestion"].is_null())
            .count();

        assert!(with_suggestions > 0, "Should provide actionable suggestions");
    }
}

#[tokio::test]
async fn test_review_code_confidence_filtering() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "bad_code",
        fixtures::QUALITY_ISSUES,
        "src/bad.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());

    // High confidence
    let input_high = json!({
        "scope_path": "src/bad.rs",
        "review_aspects": ["readability"],
        "min_confidence": 0.9,
    });

    let (result_high, _) = fixture.execute_tool(&tool, input_high).await;

    // Low confidence
    let input_low = json!({
        "scope_path": "src/bad.rs",
        "review_aspects": ["readability"],
        "min_confidence": 0.5,
    });

    let (result_low, _) = fixture.execute_tool(&tool, input_low).await;

    assert!(result_high.is_ok());
    assert!(result_low.is_ok());

    if let (Ok(ToolResult::Success { content: content_high }), Ok(ToolResult::Success { content: content_low })) = (result_high, result_low) {
        let data_high: serde_json::Value = serde_json::from_value(content_high).unwrap();
        let data_low: serde_json::Value = serde_json::from_value(content_low).unwrap();

        let count_high = data_high["comments"].as_array().unwrap().len();
        let count_low = data_low["comments"].as_array().unwrap().len();

        // Lower confidence threshold should return same or more comments
        assert!(count_low >= count_high, "Lower confidence should yield more or equal comments");
    }
}

#[tokio::test]
async fn test_review_code_empty_scope() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "nonexistent/path.rs",
        "review_aspects": ["readability"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should return valid review with no comments
        assert_eq!(data["comments"].as_array().unwrap().len(), 0);
        assert!(data["overall_score"].as_f64().unwrap() >= 0.0);
    }
}

#[tokio::test]
async fn test_review_code_summary() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit(
        "bad_code",
        fixtures::QUALITY_ISSUES,
        "src/bad.rs",
        1
    ).await.unwrap();

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/bad.rs",
        "review_aspects": ["readability", "performance", "security"],
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let summary = data["summary"].as_str().unwrap();

        // Summary should mention issue counts
        assert!(summary.contains("issue") || summary.contains("comment") || summary.contains("found"));
        assert!(summary.len() > 0);
    }
}

#[tokio::test]
async fn test_review_code_performance() {
    let fixture = AiAssistedTestFixture::new().await;

    // Create multiple units for comprehensive review
    for i in 0..10 {
        fixture.create_test_unit(
            &format!("func_{}", i),
            "fn test() { println!(\"test\"); }",
            &format!("src/file_{}.rs", i),
            1
        ).await.unwrap();
    }

    let tool = AiReviewCodeTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/",
        "review_aspects": ["readability"],
    });

    let (_, duration) = fixture.execute_tool(&tool, input).await;

    // Should complete reasonably fast even with multiple units
    assert!(duration < 2000, "Code review should complete in less than 2 seconds");
}
