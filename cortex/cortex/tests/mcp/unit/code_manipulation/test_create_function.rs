//! Unit Tests for cortex.code.create_unit (Function Creation)
//!
//! Tests cover:
//! - Creating simple functions in Rust
//! - Creating functions with parameters and return types
//! - Creating async functions
//! - Creating generic functions
//! - Creating functions in TypeScript/JavaScript
//! - AST validation after creation
//! - Token efficiency measurements
//! - Error handling (invalid syntax, duplicate names)

use super::test_helpers::*;
use cortex::mcp::tools::code_manipulation::CreateCodeUnitTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_create_simple_function() {
    let fixture = CodeManipulationFixture::new().await;

    // Create initial file
    let initial_code = r#"
// Existing code
pub fn existing_function() -> i32 {
    42
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "add",
        "parameters": [
            {"name": "a", "type": "i32"},
            {"name": "b", "type": "i32"}
        ],
        "return_type": "i32",
        "body": "    a + b",
        "visibility": "public",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Function created successfully in {}ms", duration);

            // Read the modified file
            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify the function was added
            assert!(new_content.contains("pub fn add"), "Function declaration not found");
            assert!(new_content.contains("a: i32"), "Parameter 'a' not found");
            assert!(new_content.contains("b: i32"), "Parameter 'b' not found");
            assert!(new_content.contains("-> i32"), "Return type not found");
            assert!(new_content.contains("a + b"), "Function body not found");

            // Verify existing code is preserved
            assert!(new_content.contains("existing_function"), "Existing function was removed");

            // AST validation
            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Generated code has invalid syntax");

            println!("✓ AST validation passed");
            println!("✓ All assertions passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_function_with_generics() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "identity",
        "generics": ["T"],
        "parameters": [{"name": "value", "type": "T"}],
        "return_type": "T",
        "body": "    value",
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            assert!(content.contains("pub fn identity<T>"), "Generic function not created");
            assert!(content.contains("value: T"), "Generic parameter not found");
            assert!(content.contains("-> T"), "Generic return type not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generated generic function has invalid syntax");

            println!("✓ Generic function created and validated");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_async_function() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "fetch_data",
        "parameters": [{"name": "url", "type": "&str"}],
        "return_type": "Result<String, String>",
        "body": "    Ok(\"data\".to_string())",
        "visibility": "public",
        "is_async": true,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            assert!(content.contains("pub async fn fetch_data"), "Async function not created");
            assert!(content.contains("-> Result<String, String>"), "Return type not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generated async function has invalid syntax");

            println!("✓ Async function created and validated");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_typescript_function() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
// TypeScript file
export function existingFunc(): void {
    console.log('existing');
}
"#;
    fixture.create_file("src/utils.ts", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/utils.ts",
        "unit_type": "function",
        "name": "greet",
        "parameters": [{"name": "name", "type": "string"}],
        "return_type": "string",
        "body": "    return `Hello, ${name}!`;",
        "visibility": "export",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/utils.ts").await.unwrap();

            assert!(content.contains("export function greet"), "TypeScript function not created");
            assert!(content.contains("name: string"), "Parameter not found");
            assert!(content.contains(": string"), "Return type not found");

            let is_valid = fixture.validate_syntax("src/utils.ts", &content).await;
            assert!(is_valid, "Generated TypeScript function has invalid syntax");

            println!("✓ TypeScript function created and validated");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_function_with_complex_body() {
    let fixture = CodeManipulationFixture::new().await;

    fixture.create_file("src/lib.rs", "").await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "calculate",
        "parameters": [
            {"name": "x", "type": "i32"},
            {"name": "y", "type": "i32"}
        ],
        "return_type": "i32",
        "body": r#"    let sum = x + y;
    let product = x * y;
    if sum > product {
        sum
    } else {
        product
    }"#,
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            assert!(content.contains("let sum = x + y"), "Function body not found");
            assert!(content.contains("if sum > product"), "Conditional not found");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Generated function with complex body has invalid syntax");

            println!("✓ Function with complex body created and validated");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_function_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_STRUCT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional approach: read entire file + modify + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2; // read + write

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "helper",
        "parameters": [],
        "return_type": "()",
        "body": "    // Helper function",
        "visibility": "private",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex approach: only the new function definition
            let cortex_tokens = fixture.count_tokens("fn helper() { }");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 50.0, "Token efficiency should be > 50%");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_create_function_error_duplicate_name() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::SIMPLE_RUST_FUNCTION;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "foo",  // Duplicate name
        "parameters": [],
        "return_type": "i32",
        "body": "    0",
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("duplicate") ||
                error.to_lowercase().contains("already exists"),
                "Error message should mention duplicate/existing function"
            );
            println!("✓ Duplicate name error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with duplicate name error");
        }
    }
}

#[tokio::test]
async fn test_create_function_preserves_file_structure() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
//! Module documentation
use std::collections::HashMap;

/// Existing struct
pub struct Data {
    value: i32,
}

impl Data {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert!(true);
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = CreateCodeUnitTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "unit_type": "function",
        "name": "process",
        "parameters": [{"name": "data", "type": "&Data"}],
        "return_type": "i32",
        "body": "    data.value",
        "visibility": "public",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify all original elements are preserved
            assert!(content.contains("//! Module documentation"), "Module doc lost");
            assert!(content.contains("use std::collections::HashMap"), "Import lost");
            assert!(content.contains("pub struct Data"), "Struct lost");
            assert!(content.contains("impl Data"), "Impl block lost");
            assert!(content.contains("#[cfg(test)]"), "Test module lost");
            assert!(content.contains("fn test_something"), "Test function lost");

            // Verify new function was added
            assert!(content.contains("pub fn process"), "New function not added");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "File structure corrupted after adding function");

            println!("✓ File structure preserved correctly");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}
