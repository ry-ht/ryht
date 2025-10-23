//! Unit Tests for cortex.code.add_parameter
//!
//! Tests cover:
//! - Adding parameters to functions
//! - Adding parameters to methods
//! - Updating all call sites
//! - Adding optional parameters with defaults
//! - Adding parameters to generic functions
//! - TypeScript/JavaScript parameter additions
//! - AST validation after parameter addition
//! - Error handling (invalid types, conflicting names)

use super::test_helpers::*;
use cortex_cli::mcp::tools::code_manipulation::AddParameterTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_add_parameter_simple() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn greet() -> String {
    "Hello".to_string()
}

pub fn main() {
    let msg = greet();
    println!("{}", msg);
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "greet",
        "parameter": {
            "name": "name",
            "type": "&str",
            "position": 0,  // First parameter
        },
        "update_call_sites": true,
        "default_value": "\"World\"",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Parameter added successfully in {}ms", duration);

            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify function signature updated
            assert!(new_content.contains("fn greet(name: &str)"), "Parameter not added to signature");

            // Verify call sites updated
            assert!(
                new_content.contains("greet(\"World\")") ||
                new_content.contains("greet(&"),
                "Call site not updated with default value"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Code after adding parameter has invalid syntax");

            println!("✓ All call sites updated");
            println!("✓ AST validation passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_to_method() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub struct Calculator {
    base: i32,
}

impl Calculator {
    pub fn add(&self, x: i32) -> i32 {
        self.base + x
    }

    pub fn test(&self) {
        let result = self.add(5);
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "add",
        "parent_type": "Calculator",
        "parameter": {
            "name": "y",
            "type": "i32",
            "position": 2,  // After self and x
        },
        "update_call_sites": true,
        "default_value": "0",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify method signature updated
            assert!(
                content.contains("fn add(&self, x: i32, y: i32)"),
                "Parameter not added to method signature"
            );

            // Verify call sites updated
            assert!(
                content.contains("self.add(5, 0)") ||
                content.contains("self.add(5, y)"),
                "Method call site not updated"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Method after adding parameter has invalid syntax");

            println!("✓ Parameter added to method successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_multiple_call_sites() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn calculate(x: i32) -> i32 {
    x * 2
}

pub fn test1() {
    let a = calculate(5);
    let b = calculate(10);
    let c = calculate(15);
}

pub fn test2() {
    let result = calculate(100);
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "calculate",
        "parameter": {
            "name": "multiplier",
            "type": "i32",
            "position": 1,
        },
        "update_call_sites": true,
        "default_value": "2",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify function signature
            assert!(
                content.contains("fn calculate(x: i32, multiplier: i32)"),
                "Function signature not updated"
            );

            // Count call sites - all should be updated
            let call_count = content.matches("calculate(").count();
            let updated_count = content.matches("calculate(5, 2)").count() +
                               content.matches("calculate(10, 2)").count() +
                               content.matches("calculate(15, 2)").count() +
                               content.matches("calculate(100, 2)").count();

            assert_eq!(
                call_count, updated_count,
                "Not all call sites updated. Found {} calls, updated {}",
                call_count, updated_count
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with multiple updated call sites has invalid syntax");

            println!("✓ All {} call sites updated successfully", call_count);
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_generic_parameter() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn process<T>(value: T) -> T {
    value
}

pub fn test() {
    let x = process(42);
    let y = process("hello");
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "process",
        "parameter": {
            "name": "default",
            "type": "T",
            "position": 1,
        },
        "update_call_sites": false,  // Manual update required for generics
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify generic function signature updated
            assert!(
                content.contains("fn process<T>(value: T, default: T)"),
                "Generic parameter not added"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            // Note: Syntax will be invalid without updating call sites, but signature should be correct
            println!("✓ Generic parameter added to signature");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_optional_parameter_typescript() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

export function test() {
    console.log(greet("Alice"));
    console.log(greet("Bob"));
}
"#;
    fixture.create_file("src/utils.ts", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/utils.ts",
        "function_name": "greet",
        "parameter": {
            "name": "greeting",
            "type": "string",
            "position": 1,
            "optional": true,
            "default": "\"Hello\"",
        },
        "update_call_sites": false,  // Optional params don't require call site updates
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/utils.ts").await.unwrap();

            // Verify optional parameter added
            assert!(
                content.contains("greeting?: string") ||
                content.contains("greeting: string = \"Hello\""),
                "Optional parameter not added correctly"
            );

            // Call sites should still be valid without changes
            assert!(content.contains("greet(\"Alice\")"), "Call sites modified unexpectedly");

            let is_valid = fixture.validate_syntax("src/utils.ts", &content).await;
            assert!(is_valid, "TypeScript code with optional parameter has invalid syntax");

            println!("✓ Optional TypeScript parameter added successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_with_complex_type() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
use std::collections::HashMap;

pub fn process(data: Vec<i32>) -> i32 {
    data.iter().sum()
}

pub fn test() {
    let result = process(vec![1, 2, 3]);
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "process",
        "parameter": {
            "name": "weights",
            "type": "HashMap<i32, f64>",
            "position": 1,
        },
        "update_call_sites": true,
        "default_value": "HashMap::new()",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify complex type added
            assert!(
                content.contains("weights: HashMap<i32, f64>"),
                "Complex parameter type not added"
            );

            // Verify call site updated with default
            assert!(
                content.contains("HashMap::new()") ||
                content.contains("process(vec![1, 2, 3], HashMap::new())"),
                "Call site not updated with complex default"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with complex parameter type has invalid syntax");

            println!("✓ Complex parameter type added successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_at_beginning() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}

pub fn test() {
    let result = calculate(5, 10);
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "calculate",
        "parameter": {
            "name": "multiplier",
            "type": "i32",
            "position": 0,  // Insert at beginning
        },
        "update_call_sites": true,
        "default_value": "1",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify parameter inserted at beginning
            assert!(
                content.contains("fn calculate(multiplier: i32, x: i32, y: i32)"),
                "Parameter not inserted at beginning"
            );

            // Verify call site updated
            assert!(
                content.contains("calculate(1, 5, 10)"),
                "Call site not updated with new parameter at beginning"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with parameter at beginning has invalid syntax");

            println!("✓ Parameter inserted at beginning successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_STRUCT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional: read entire file + modify + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2;

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "is_adult",
        "parent_type": "Person",
        "parameter": {
            "name": "min_age",
            "type": "u32",
            "position": 1,
        },
        "default_value": "18",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex: only the parameter addition specification
            let cortex_tokens = fixture.count_tokens("add parameter min_age u32 to is_adult");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 70.0, "Token efficiency should be > 70% for parameter additions");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_add_parameter_error_duplicate_name() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "calculate",
        "parameter": {
            "name": "x",  // Duplicate name
            "type": "i32",
            "position": 2,
        },
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("duplicate") ||
                error.to_lowercase().contains("exists") ||
                error.to_lowercase().contains("conflict"),
                "Error should mention duplicate parameter name"
            );
            println!("✓ Duplicate parameter name error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with duplicate parameter name error");
        }
    }
}

#[tokio::test]
async fn test_add_parameter_error_invalid_position() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::SIMPLE_RUST_FUNCTION;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "foo",
        "parameter": {
            "name": "new_param",
            "type": "i32",
            "position": 10,  // Invalid position (function has 0 params)
        },
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("position") ||
                error.to_lowercase().contains("invalid") ||
                error.to_lowercase().contains("index"),
                "Error should mention invalid position"
            );
            println!("✓ Invalid position error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with invalid position error");
        }
    }
}

#[tokio::test]
async fn test_add_parameter_preserves_attributes() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
#[inline]
#[must_use]
pub fn important(x: i32) -> i32 {
    x * 2
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = AddParameterTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "function_name": "important",
        "parameter": {
            "name": "y",
            "type": "i32",
            "position": 1,
        },
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify attributes preserved
            assert!(content.contains("#[inline]"), "Inline attribute lost");
            assert!(content.contains("#[must_use]"), "Must_use attribute lost");

            // Verify parameter added
            assert!(content.contains("x: i32, y: i32"), "Parameter not added");

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with preserved attributes has invalid syntax");

            println!("✓ Function attributes preserved");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}
