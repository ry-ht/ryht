//! Unit Tests for cortex.ai.generate_docstring
//!
//! Tests cover:
//! - Basic docstring generation
//! - Different documentation styles
//! - Parameter documentation
//! - Return value documentation
//! - Error documentation
//! - Panic documentation
//! - Example generation
//! - Quality scoring

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiGenerateDocstringTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_generate_docstring_basic() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "style": "rustdoc",
        "include_examples": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should generate docstring
        assert!(!data["docstring"].is_null());
        assert!(!data["style"].is_null());
        assert!(!data["quality_score"].is_null());

        let docstring = data["docstring"].as_str().unwrap();
        assert!(docstring.len() > 0, "Should generate non-empty docstring");
        assert!(docstring.contains("///"), "Should use Rust doc comment format");
    }
}

#[tokio::test]
async fn test_generate_docstring_with_parameters() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "style": "rustdoc",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should document parameters
        assert!(docstring.contains("# Parameters") || docstring.contains("Parameters"));
    }
}

#[tokio::test]
async fn test_generate_docstring_with_return() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "style": "rustdoc",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should document return value
        assert!(docstring.contains("# Returns") || docstring.contains("Returns"));
    }
}

#[tokio::test]
async fn test_generate_docstring_with_errors() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "style": "rustdoc",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should document errors for Result return type
        assert!(docstring.contains("# Errors") || docstring.contains("Errors"));
    }
}

#[tokio::test]
async fn test_generate_docstring_with_panics() {
    let fixture = AiAssistedTestFixture::new().await;

    let code_with_unwrap = r#"
fn get_value(data: Vec<i32>) -> i32 {
    data.get(0).unwrap()
}
    "#;

    let unit_id = fixture.create_test_unit(
        "get_value",
        code_with_unwrap,
        "src/util.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "style": "rustdoc",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should document panics for unwrap() usage
        assert!(docstring.contains("# Panics") || docstring.contains("Panics"));
    }
}

#[tokio::test]
async fn test_generate_docstring_with_examples() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should include examples
        assert!(docstring.contains("# Examples") || docstring.contains("Examples"));
        assert!(docstring.contains("```"));
    }
}

#[tokio::test]
async fn test_generate_docstring_without_examples() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": false,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();

        // Should not include examples
        assert!(!docstring.contains("# Examples"));
    }
}

#[tokio::test]
async fn test_generate_docstring_quality_score() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let quality_score = data["quality_score"].as_f64().unwrap();

        // Quality score should be between 0 and 1
        assert!(quality_score >= 0.0 && quality_score <= 1.0);

        // With examples and comprehensive doc, should have good quality
        assert!(quality_score > 0.5, "Comprehensive documentation should have good quality score");
    }
}

#[tokio::test]
async fn test_generate_docstring_different_styles() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());

    for style in ["rustdoc", "rust", "google"] {
        let input = json!({
            "unit_id": unit_id.clone(),
            "style": style,
        });

        let (result, _) = fixture.execute_tool(&tool, input).await;
        assert!(result.is_ok(), "Should work with style: {}", style);

        if let Ok(ToolResult::Success { content }) = result {
            let data: serde_json::Value = serde_json::from_value(content).unwrap();
            assert_eq!(data["style"].as_str().unwrap(), style);
        }
    }
}

#[tokio::test]
async fn test_generate_docstring_nonexistent_unit() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "nonexistent-unit-id",
        "style": "rustdoc",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail for nonexistent unit");
}

#[tokio::test]
async fn test_generate_docstring_simple_function() {
    let fixture = AiAssistedTestFixture::new().await;

    let simple_code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
    "#;

    let unit_id = fixture.create_test_unit(
        "add",
        simple_code,
        "src/math.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let docstring = data["docstring"].as_str().unwrap();
        assert!(docstring.len() > 0);

        // Should have reasonable quality even for simple functions
        let quality = data["quality_score"].as_f64().unwrap();
        assert!(quality > 0.0);
    }
}

#[tokio::test]
async fn test_generate_docstring_performance() {
    let fixture = AiAssistedTestFixture::new().await;

    let unit_id = fixture.create_test_unit(
        "process_user_data",
        fixtures::UNDOCUMENTED_FUNCTION,
        "src/user.rs",
        1
    ).await.unwrap();

    let tool = AiGenerateDocstringTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": unit_id,
        "include_examples": true,
    });

    let (_, duration) = fixture.execute_tool(&tool, input).await;

    // Should complete reasonably fast
    assert!(duration < 1000, "Docstring generation should complete in less than 1 second");
}
