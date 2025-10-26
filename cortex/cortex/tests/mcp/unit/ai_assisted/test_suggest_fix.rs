//! Unit Tests for cortex.ai.suggest_fix
//!
//! Tests cover:
//! - Borrow checker error fixes
//! - Move error fixes
//! - Lifetime error fixes
//! - Type mismatch fixes
//! - Panic fixes (unwrap/expect)
//! - Root cause analysis
//! - Multiple fix suggestions

use super::test_helpers::*;
use cortex_mcp::tools::ai_assisted::AiSuggestFixTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_suggest_fix_borrow_error() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: cannot borrow `data` as mutable more than once at a time",
        "code_context": r#"
fn process() {
    let mut data = vec![1, 2, 3];
    let ref1 = &mut data;
    let ref2 = &mut data;  // Error here
}
        "#,
        "file_path": "src/main.rs",
        "line_number": 4,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Should provide fixes
        assert!(!data["fixes"].is_null());
        let fixes = data["fixes"].as_array().unwrap();
        assert!(fixes.len() > 0, "Should suggest at least one fix");

        // Check root cause analysis
        assert!(!data["root_cause_analysis"].is_null());
        let root_cause = data["root_cause_analysis"].as_str().unwrap();
        assert!(root_cause.contains("borrow") || root_cause.contains("mutable"));

        // Check fix structure
        let first_fix = &fixes[0];
        assert!(!first_fix["description"].is_null());
        assert!(!first_fix["fixed_code"].is_null());
        assert!(!first_fix["explanation"].is_null());
        assert!(first_fix["confidence"].as_f64().unwrap() > 0.0);
    }
}

#[tokio::test]
async fn test_suggest_fix_move_error() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: value used after move",
        "code_context": fixtures::ERROR_CONTEXT,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        assert!(fixes.len() > 0);

        // Should suggest using reference or clone
        let fix_descriptions: Vec<&str> = fixes.iter()
            .map(|f| f["description"].as_str().unwrap())
            .collect();

        assert!(fix_descriptions.iter().any(|d| d.contains("reference") || d.contains("clone")));

        // Check most likely fix
        assert!(!data["most_likely_fix"].is_null());
    }
}

#[tokio::test]
async fn test_suggest_fix_lifetime_error() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: `data` does not live long enough",
        "code_context": r#"
fn get_reference() -> &str {
    let data = String::from("test");
    &data  // Error: doesn't live long enough
}
        "#,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        assert!(fixes.len() > 0);

        // Should mention lifetime in root cause
        let root_cause = data["root_cause_analysis"].as_str().unwrap();
        assert!(root_cause.contains("lifetime") || root_cause.contains("live long enough"));
    }
}

#[tokio::test]
async fn test_suggest_fix_type_mismatch() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: expected `String`, found `&str`",
        "code_context": r#"
fn takes_string(s: String) {
    println!("{}", s);
}

fn main() {
    takes_string("hello");  // Error: expected String, found &str
}
        "#,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        assert!(fixes.len() > 0);

        // Should suggest type conversion
        let first_fix = &fixes[0];
        let fixed_code = first_fix["fixed_code"].as_str().unwrap();
        assert!(fixed_code.contains("into()") || fixed_code.contains("to_string()") || fixed_code.contains("conversion"));
    }
}

#[tokio::test]
async fn test_suggest_fix_unwrap_panic() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "panic: called `Option::unwrap()` on a `None` value",
        "code_context": r#"
fn get_value(map: HashMap<i32, String>) -> String {
    map.get(&42).unwrap()  // Can panic
}
        "#,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        assert!(fixes.len() > 0);

        // Should suggest proper error handling
        let fix_approaches: Vec<&str> = fixes.iter()
            .map(|f| f["fixed_code"].as_str().unwrap())
            .collect();

        assert!(fix_approaches.iter().any(|code| code.contains("match") || code.contains("?")));

        // Check root cause mentions unwrap
        let root_cause = data["root_cause_analysis"].as_str().unwrap();
        assert!(root_cause.contains("unwrap") || root_cause.contains("None"));
    }
}

#[tokio::test]
async fn test_suggest_fix_multiple_solutions() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: value used after move",
        "code_context": r#"
fn process() {
    let data = String::from("test");
    consume(data);
    println!("{}", data);
}
        "#,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        // Should provide multiple alternative fixes
        assert!(fixes.len() >= 2, "Should suggest multiple solutions");

        // Each fix should have different confidence
        let confidences: Vec<f64> = fixes.iter()
            .map(|f| f["confidence"].as_f64().unwrap())
            .collect();

        // Not all confidences should be exactly the same
        let unique_confidences: std::collections::HashSet<_> = confidences.iter()
            .map(|c| (c * 100.0) as i32)
            .collect();
        assert!(unique_confidences.len() > 1 || confidences.len() == 1, "Different fixes should have different confidence scores");
    }
}

#[tokio::test]
async fn test_suggest_fix_confidence_ordering() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: value used after move",
        "code_context": "let x = String::from(\"test\"); consume(x); println!(\"{}\", x);",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        if fixes.len() > 1 {
            // Most likely fix should match first fix (highest confidence)
            let most_likely = data["most_likely_fix"].as_str().unwrap();
            let first_desc = fixes[0]["description"].as_str().unwrap();

            assert_eq!(most_likely, first_desc, "Most likely fix should be the first fix");
        }
    }
}

#[tokio::test]
async fn test_suggest_fix_with_file_context() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: cannot borrow as mutable",
        "code_context": "let x = vec![1, 2]; let y = &mut x; let z = &mut x;",
        "file_path": "src/main.rs",
        "line_number": 42,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    // Should work with optional file path and line number
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["fixes"].is_null());
    }
}

#[tokio::test]
async fn test_suggest_fix_unknown_error() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "some completely unknown custom error",
        "code_context": "let x = 42;",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    // Should handle unknown errors gracefully
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // May return empty fixes or generic suggestions
        assert!(!data["fixes"].is_null());
        assert!(!data["most_likely_fix"].is_null());
    }
}

#[tokio::test]
async fn test_suggest_fix_additional_changes() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: value used after move",
        "code_context": "let x = String::from(\"test\"); consume(x); println!(\"{}\", x);",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        let fixes = data["fixes"].as_array().unwrap();
        for fix in fixes {
            // Should include additional_changes field
            assert!(!fix["additional_changes"].is_null());

            let additional = fix["additional_changes"].as_array().unwrap();
            // Can be empty or have changes
            assert!(additional.len() >= 0);
        }
    }
}

#[tokio::test]
async fn test_suggest_fix_empty_context() {
    let fixture = AiAssistedTestFixture::new().await;

    let tool = AiSuggestFixTool::new(fixture.ctx.clone());
    let input = json!({
        "error_message": "error: some error",
        "code_context": "",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    // Should handle empty context
    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["fixes"].is_null());
    }
}
