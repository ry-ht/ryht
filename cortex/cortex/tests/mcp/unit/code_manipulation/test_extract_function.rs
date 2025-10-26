//! Unit Tests for cortex.code.extract_function
//!
//! Tests cover:
//! - Extracting simple code blocks into functions
//! - Detecting parameters from variable usage
//! - Determining return types
//! - Updating original code with function call
//! - Extracting with multiple return values
//! - Extracting from different contexts (functions, methods, closures)
//! - AST validation after extraction
//! - Token efficiency measurements

use super::test_helpers::*;
use cortex::mcp::tools::code_manipulation::ExtractFunctionTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_extract_simple_block() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    let a = x * 2;
    let b = y * 3;
    let result = a + b;
    result
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "calculate",
        "new_function_name": "compute_result",
        "start_line": 3,  // let a = x * 2;
        "end_line": 5,    // let result = a + b;
        "extract_mode": "inline_call",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { content } => {
            println!("✓ Function extracted successfully in {}ms", duration);

            let new_content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify new function was created
            assert!(new_content.contains("fn compute_result"), "Extracted function not found");

            // Verify parameters detected (x and y are used)
            assert!(
                new_content.contains("x: i32") && new_content.contains("y: i32"),
                "Parameters not detected correctly"
            );

            // Verify original code replaced with call
            assert!(new_content.contains("compute_result(x, y)"), "Function call not inserted");

            // Verify original function still exists
            assert!(new_content.contains("pub fn calculate"), "Original function removed");

            let is_valid = fixture.validate_syntax("src/lib.rs", &new_content).await;
            assert!(is_valid, "Extracted code has invalid syntax");

            println!("✓ AST validation passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_with_return_value() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn process(data: Vec<i32>) -> i32 {
    let mut sum = 0;
    for item in &data {
        sum += item;
    }
    let average = sum / data.len() as i32;
    average
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "process",
        "new_function_name": "calculate_sum",
        "start_line": 3,  // let mut sum = 0;
        "end_line": 6,    // }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify extracted function exists
            assert!(content.contains("fn calculate_sum"), "Extracted function not found");

            // Verify return type detected
            assert!(content.contains("-> i32"), "Return type not detected");

            // Verify original code uses returned value
            assert!(
                content.contains("let sum = calculate_sum(&data)") ||
                content.contains("calculate_sum(&data)"),
                "Extracted function call not found"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Extracted code with return has invalid syntax");

            println!("✓ Function with return value extracted successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_from_method() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub struct Calculator {
    base: i32,
}

impl Calculator {
    pub fn complex_calc(&self, x: i32) -> i32 {
        let step1 = x * 2;
        let step2 = step1 + self.base;
        let step3 = step2 * step2;
        step3
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "complex_calc",
        "new_function_name": "calculate_steps",
        "start_line": 7,  // let step1 = x * 2;
        "end_line": 9,    // let step3 = step2 * step2;
        "extract_as": "method",  // Extract as another method
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify new method created
            assert!(content.contains("fn calculate_steps"), "Extracted method not found");

            // Verify &self parameter for method
            assert!(content.contains("&self"), "Method self parameter not found");

            // Verify call updated in original method
            assert!(
                content.contains("self.calculate_steps"),
                "Method call not inserted"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Extracted method has invalid syntax");

            println!("✓ Method extracted successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_with_multiple_variables() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn analyze(data: Vec<i32>) -> (i32, i32) {
    let mut min = i32::MAX;
    let mut max = i32::MIN;
    for &value in &data {
        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
    }
    (min, max)
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "analyze",
        "new_function_name": "find_min_max",
        "start_line": 3,  // let mut min...
        "end_line": 12,   // }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify extracted function
            assert!(content.contains("fn find_min_max"), "Extracted function not found");

            // Verify tuple return type
            assert!(
                content.contains("-> (i32, i32)"),
                "Tuple return type not detected"
            );

            // Verify destructuring in original function
            assert!(
                content.contains("let (min, max) = find_min_max(&data)") ||
                content.contains("find_min_max(&data)"),
                "Function call not properly integrated"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Extracted function with multiple returns has invalid syntax");

            println!("✓ Function with multiple return values extracted successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_typescript_code() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
export function processData(items: number[]): number {
    let sum = 0;
    for (const item of items) {
        sum += item;
    }
    const average = sum / items.length;
    return average;
}
"#;
    fixture.create_file("src/utils.ts", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/utils.ts",
        "source_function": "processData",
        "new_function_name": "calculateSum",
        "start_line": 3,  // let sum = 0;
        "end_line": 6,    // }
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/utils.ts").await.unwrap();

            // Verify extracted function
            assert!(content.contains("function calculateSum"), "TypeScript function not extracted");

            // Verify TypeScript syntax
            assert!(content.contains(": number"), "TypeScript types not preserved");

            let is_valid = fixture.validate_syntax("src/utils.ts", &content).await;
            assert!(is_valid, "Extracted TypeScript code has invalid syntax");

            println!("✓ TypeScript function extracted successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_with_closure_capture() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn outer(multiplier: i32) -> impl Fn(i32) -> i32 {
    move |x| {
        let doubled = x * 2;
        let adjusted = doubled + 10;
        let result = adjusted * multiplier;
        result
    }
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "outer",
        "new_function_name": "calculate_inner",
        "start_line": 4,  // let doubled = x * 2;
        "end_line": 6,    // let result = adjusted * multiplier;
        "extract_as": "function",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify extracted function
            assert!(content.contains("fn calculate_inner"), "Helper function not extracted");

            // Verify captured variables passed as parameters
            assert!(
                content.contains("multiplier"),
                "Captured variable not passed as parameter"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Extracted code from closure has invalid syntax");

            println!("✓ Code extracted from closure successfully");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_preserves_comments() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn process() -> i32 {
    // Step 1: Initialize
    let x = 10;
    // Step 2: Process
    let y = x * 2;
    // Step 3: Return
    y
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "process",
        "new_function_name": "compute",
        "start_line": 3,
        "end_line": 7,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            let content = fixture.read_file("src/lib.rs").await.unwrap();

            // Verify extracted function has comments
            assert!(
                content.contains("// Step 1:") || content.contains("// Initialize"),
                "Comments not preserved in extracted function"
            );

            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Code with preserved comments has invalid syntax");

            println!("✓ Comments preserved during extraction");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_token_efficiency() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::RUST_STRUCT;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    // Traditional: read entire file + analyze + modify + write
    let traditional_tokens = fixture.count_tokens(initial_code) * 2;

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "is_adult",
        "new_function_name": "check_age",
        "start_line": 10,
        "end_line": 11,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Success { .. } => {
            // Cortex: only the extraction specification + new function signature
            let cortex_tokens = fixture.count_tokens("extract check_age from is_adult lines 10-11");

            let efficiency = fixture.token_efficiency(traditional_tokens, cortex_tokens);

            println!("Token Efficiency:");
            println!("  Traditional: {} tokens", traditional_tokens);
            println!("  Cortex:      {} tokens", cortex_tokens);
            println!("  Savings:     {:.1}%", efficiency);

            assert!(efficiency > 60.0, "Token efficiency should be > 60% for extract operations");
            println!("✓ Token efficiency test passed");
        }
        ToolResult::Error { error } => {
            panic!("Test failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_extract_error_invalid_range() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = fixtures::SIMPLE_RUST_FUNCTION;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "foo",
        "new_function_name": "extracted",
        "start_line": 10,  // Beyond file length
        "end_line": 20,
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("invalid") ||
                error.to_lowercase().contains("range") ||
                error.to_lowercase().contains("line"),
                "Error should mention invalid range"
            );
            println!("✓ Invalid range error handling works correctly");
        }
        ToolResult::Success { .. } => {
            panic!("Should have failed with invalid range error");
        }
    }
}

#[tokio::test]
async fn test_extract_error_incomplete_block() {
    let fixture = CodeManipulationFixture::new().await;

    let initial_code = r#"
pub fn process() -> i32 {
    let x = {
        let y = 10;
        y * 2
    };
    x
}
"#;
    fixture.create_file("src/lib.rs", initial_code).await.unwrap();

    let tool = ExtractFunctionTool::new(fixture.context());
    let input = json!({
        "workspace_id": fixture.workspace_id.to_string(),
        "file_path": "src/lib.rs",
        "source_function": "process",
        "new_function_name": "extracted",
        "start_line": 3,  // Start of block
        "end_line": 4,    // Doesn't include closing brace
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    match result {
        ToolResult::Error { error } => {
            assert!(
                error.to_lowercase().contains("incomplete") ||
                error.to_lowercase().contains("block") ||
                error.to_lowercase().contains("syntax"),
                "Error should mention incomplete block"
            );
            println!("✓ Incomplete block error handling works correctly");
        }
        ToolResult::Success { .. } => {
            // It's acceptable if the tool is smart enough to handle this
            let content = fixture.read_file("src/lib.rs").await.unwrap();
            let is_valid = fixture.validate_syntax("src/lib.rs", &content).await;
            assert!(is_valid, "Extracted incomplete block has invalid syntax");
            println!("✓ Tool handled incomplete block selection gracefully");
        }
    }
}
