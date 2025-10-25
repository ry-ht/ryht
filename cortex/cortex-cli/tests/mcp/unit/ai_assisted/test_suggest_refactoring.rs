//! Unit Tests for cortex.ai.suggest_refactoring
//!
//! Tests cover:
//! - Basic refactoring suggestions
//! - Long function detection
//! - Complex conditional detection
//! - Poor naming detection
//! - High complexity detection
//! - Confidence filtering
//! - Multiple refactoring types

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiSuggestRefactoringTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_suggest_refactoring_long_function() {
    let fixture = AiAssistedTestFixture::new().await;

    // Create a long function with 60+ lines
    let long_body = format!("{}\n{}", fixtures::LONG_FUNCTION, "    // More lines\n".repeat(50));
    fixture.create_test_unit("process_data", &long_body, "src/main.rs", 10).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/main.rs",
        "refactoring_types": ["extract_function"],
        "min_confidence": 0.7,
        "include_impact_analysis": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should suggest extract_function refactoring
        assert!(data["total_count"].as_i64().unwrap() > 0, "Should find refactoring opportunities");

        let suggestions = data["suggestions"].as_array().unwrap();
        assert!(suggestions.iter().any(|s| s["refactoring_type"] == "extract_function"));

        // Check suggestion structure
        let first = &suggestions[0];
        assert!(!first["suggestion_id"].is_null());
        assert!(!first["description"].is_null());
        assert!(!first["reasoning"].is_null());
        assert!(first["confidence"].as_f64().unwrap() >= 0.7);
    }
}

#[tokio::test]
async fn test_suggest_refactoring_complex_conditionals() {
    let fixture = AiAssistedTestFixture::new().await;

    fixture.create_test_unit("validate", fixtures::COMPLEX_CONDITIONALS, "src/validation.rs", 5).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/validation.rs",
        "refactoring_types": ["simplify_logic"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let suggestions = data["suggestions"].as_array().unwrap();
        assert!(suggestions.iter().any(|s| s["refactoring_type"] == "simplify_logic"));

        // Check impact analysis
        assert!(!data["estimated_improvement"].is_null());
        assert!(data["estimated_improvement"]["readability_score"].as_f64().unwrap() > 0.0);
    }
}

#[tokio::test]
async fn test_suggest_refactoring_poor_naming() {
    let fixture = AiAssistedTestFixture::new().await;

    // Create function with short, cryptic name
    let body = "fn a() { println!(\"test\"); }";
    fixture.create_test_unit("a", body, "src/util.rs", 1).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/util.rs",
        "refactoring_types": ["improve_naming"],
        "min_confidence": 0.6,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let suggestions = data["suggestions"].as_array().unwrap();
        if !suggestions.is_empty() {
            assert!(suggestions.iter().any(|s| s["refactoring_type"] == "improve_naming"));

            // Check that breaking_changes flag is set for naming changes
            let naming_suggestion = suggestions.iter().find(|s| s["refactoring_type"] == "improve_naming").unwrap();
            assert_eq!(naming_suggestion["impact"]["breaking_changes"].as_bool().unwrap(), true);
        }
    }
}

#[tokio::test]
async fn test_suggest_refactoring_high_complexity() {
    let fixture = AiAssistedTestFixture::new().await;

    // Create function with high cyclomatic complexity
    let complex_body = r#"
fn complex_function(a: i32, b: i32, c: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            while c > 0 {
                for i in 0..10 {
                    if i > 5 && a > b || c > 10 {
                        match a {
                            1 => return 1,
                            2 => return 2,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    0
}
    "#;

    fixture.create_test_unit("complex_function", complex_body, "src/complex.rs", 1).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/complex.rs",
        "refactoring_types": ["reduce_complexity"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let suggestions = data["suggestions"].as_array().unwrap();
        assert!(suggestions.iter().any(|s| s["refactoring_type"] == "reduce_complexity"));
    }
}

#[tokio::test]
async fn test_suggest_refactoring_confidence_filtering() {
    let fixture = AiAssistedTestFixture::new().await;

    let long_body = format!("{}\n{}", fixtures::LONG_FUNCTION, "    // More lines\n".repeat(50));
    fixture.create_test_unit("process_data", &long_body, "src/main.rs", 10).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());

    // Test with high confidence threshold
    let input_high = json!({
        "scope_path": "src/main.rs",
        "refactoring_types": ["extract_function"],
        "min_confidence": 0.95,
    });

    let (result_high, _) = fixture.execute_tool(&tool, input_high).await;
    assert!(result_high.is_ok());

    // Test with low confidence threshold
    let input_low = json!({
        "scope_path": "src/main.rs",
        "refactoring_types": ["extract_function"],
        "min_confidence": 0.5,
    });

    let (result_low, _) = fixture.execute_tool(&tool, input_low).await;
    assert!(result_low.is_ok());

    // Low threshold should return same or more suggestions
    if let (Ok(ToolResult::Success { content: content_high }), Ok(ToolResult::Success { content: content_low })) = (result_high, result_low) {
        let data_high: serde_json::Value = serde_json::from_value(content_high).unwrap();
        let data_low: serde_json::Value = serde_json::from_value(content_low).unwrap();

        let count_high = data_high["total_count"].as_i64().unwrap();
        let count_low = data_low["total_count"].as_i64().unwrap();

        assert!(count_low >= count_high, "Lower confidence threshold should return same or more results");
    }
}

#[tokio::test]
async fn test_suggest_refactoring_multiple_types() {
    let fixture = AiAssistedTestFixture::new().await;

    // Create function with multiple issues
    let problematic_code = r#"
fn a(x: i32, y: i32) -> i32 {
    if x > 0 {
        if y > 0 {
            if x > 10 {
                if y > 10 {
                    if x > 100 {
                        if y > 100 {
                            return x + y;
                        }
                    }
                }
            }
        }
    }
    0
}
    "#.repeat(10); // Make it long too

    fixture.create_test_unit("a", &problematic_code, "src/bad.rs", 1).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/bad.rs",
        "refactoring_types": ["extract_function", "simplify_logic", "improve_naming", "reduce_complexity"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should find multiple types of issues
        let suggestions = data["suggestions"].as_array().unwrap();
        let refactoring_types: std::collections::HashSet<_> = suggestions
            .iter()
            .map(|s| s["refactoring_type"].as_str().unwrap())
            .collect();

        assert!(refactoring_types.len() > 1, "Should suggest multiple refactoring types");
    }
}

#[tokio::test]
async fn test_suggest_refactoring_empty_scope() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "nonexistent/path.rs",
        "refactoring_types": ["extract_function"],
        "min_confidence": 0.5,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should return empty suggestions for non-existent scope
        assert_eq!(data["total_count"].as_i64().unwrap(), 0);
        assert_eq!(data["suggestions"].as_array().unwrap().len(), 0);
    }
}

#[tokio::test]
async fn test_suggest_refactoring_impact_analysis() {
    let fixture = AiAssistedTestFixture::new().await;

    let long_body = format!("{}\n{}", fixtures::LONG_FUNCTION, "    // More lines\n".repeat(50));
    fixture.create_test_unit("process_data", &long_body, "src/main.rs", 10).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());
    let input = json!({
        "scope_path": "src/main.rs",
        "refactoring_types": ["extract_function"],
        "include_impact_analysis": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let suggestions = data["suggestions"].as_array().unwrap();
        if !suggestions.is_empty() {
            let first = &suggestions[0];

            // Check impact fields
            assert!(!first["impact"].is_null());
            assert!(!first["impact"]["readability_score"].is_null());
            assert!(!first["impact"]["maintainability_score"].is_null());
            assert!(!first["impact"]["performance_impact"].is_null());
            assert!(!first["impact"]["risk_level"].is_null());
            assert!(!first["impact"]["breaking_changes"].is_null());

            // Check effort estimate
            assert!(!first["effort_estimate"].is_null());
        }
    }
}

#[tokio::test]
async fn test_suggest_refactoring_default_types() {
    let fixture = AiAssistedTestFixture::new().await;

    let long_body = format!("{}\n{}", fixtures::LONG_FUNCTION, "    // More lines\n".repeat(50));
    fixture.create_test_unit("process_data", &long_body, "src/main.rs", 10).await.unwrap();

    let tool = AiSuggestRefactoringTool::new(fixture.ctx.clone());

    // Test without specifying refactoring_types (should use defaults)
    let input = json!({
        "scope_path": "src/main.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
}
