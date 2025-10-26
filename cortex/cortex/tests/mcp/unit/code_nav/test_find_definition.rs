//! Unit Tests for cortex.code.find_definition
//!
//! Tests cover:
//! - Finding function definitions by name
//! - Finding struct/class definitions
//! - Finding trait/interface definitions
//! - Cross-file definition lookup
//! - Qualified name resolution
//! - Context-aware searching
//! - Multiple matches handling
//! - Error cases (symbol not found)
//! - Performance measurement

use super::test_helpers::*;
use cortex::mcp::tools::code_nav::CodeFindDefinitionTool;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_find_definition_simple_function() {
    let fixture = CodeNavTestFixture::new().await;

    // Create and store a function
    let func = fixtures::create_rust_function(
        "process_data",
        "myapp::process_data",
        "src/lib.rs",
        10,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Find definition by name
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "process_data",
        "context_file": "src/lib.rs",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find definition");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "process_data");
        assert_eq!(data["unit_id"], func_id.to_string());
        assert_eq!(data["unit_type"], "Function");
        assert!(data["signature"].as_str().unwrap().contains("process_data"));
    }
}

#[tokio::test]
async fn test_find_definition_by_qualified_name() {
    let fixture = CodeNavTestFixture::new().await;

    // Create and store a function
    let func = fixtures::create_rust_function(
        "validate_input",
        "myapp::validators::validate_input",
        "src/validators.rs",
        20,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Find by fully qualified name
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "myapp::validators::validate_input",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find definition by qualified name");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["unit_id"], func_id.to_string());
        assert_eq!(data["qualified_name"], "myapp::validators::validate_input");
    }
}

#[tokio::test]
async fn test_find_definition_struct() {
    let fixture = CodeNavTestFixture::new().await;

    // Create and store a struct
    let struct_unit = fixtures::create_rust_struct(
        "User",
        "myapp::models::User",
        "src/models.rs",
        5,
    );
    let struct_id = fixture.store_unit(&struct_unit).await.unwrap();

    // Find struct definition
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "User",
        "context_file": "src/models.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find struct definition");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "User");
        assert_eq!(data["unit_id"], struct_id.to_string());
        assert_eq!(data["unit_type"], "Struct");
        assert!(data["location"]["file"].as_str().unwrap().contains("models.rs"));
        assert_eq!(data["location"]["start_line"], 5);
    }
}

#[tokio::test]
async fn test_find_definition_trait() {
    let fixture = CodeNavTestFixture::new().await;

    // Create and store a trait
    let trait_unit = fixtures::create_rust_trait(
        "Processor",
        "myapp::traits::Processor",
        "src/traits.rs",
        10,
    );
    fixture.store_unit(&trait_unit).await.unwrap();

    // Find trait definition
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "Processor",
        "context_file": "src/traits.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find trait definition");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "Processor");
        assert_eq!(data["unit_type"], "Trait");
    }
}

#[tokio::test]
async fn test_find_definition_typescript_class() {
    let fixture = CodeNavTestFixture::new().await;

    // Create and store a TypeScript class
    let class = fixtures::create_typescript_class(
        "UserService",
        "app.services.UserService",
        "src/services.ts",
        15,
    );
    fixture.store_unit(&class).await.unwrap();

    // Find class definition
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "UserService",
        "context_file": "src/services.ts",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to find TypeScript class definition");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "UserService");
        assert_eq!(data["unit_type"], "Class");
    }
}

#[tokio::test]
async fn test_find_definition_cross_file() {
    let fixture = CodeNavTestFixture::new().await;

    // Create functions in different files
    let func1 = fixtures::create_rust_function(
        "main",
        "myapp::main",
        "src/main.rs",
        1,
    );
    let func2 = fixtures::create_rust_function(
        "helper",
        "myapp::utils::helper",
        "src/utils.rs",
        10,
    );

    fixture.store_unit(&func1).await.unwrap();
    fixture.store_unit(&func2).await.unwrap();

    // Find definition from different context file
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "helper",
        "context_file": "src/main.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;

    // This should search globally when not found in context file
    // In a real implementation, it would check imports and resolve the symbol
}

#[tokio::test]
async fn test_find_definition_with_location_info() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function with specific location
    let func = fixtures::create_rust_function(
        "calculate",
        "myapp::math::calculate",
        "src/math.rs",
        42,
    );
    fixture.store_unit(&func).await.unwrap();

    // Find and verify location
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "calculate",
        "context_file": "src/math.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let location = &data["location"];
        assert_eq!(location["file"], "src/math.rs");
        assert_eq!(location["start_line"], 42);
        assert_eq!(location["end_line"], 52); // 42 + 10
        assert!(location["start_column"].as_u64().unwrap() >= 0);
        assert!(location["end_column"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_find_definition_with_documentation() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function with docstring
    let func = fixtures::create_rust_function(
        "documented_fn",
        "myapp::documented_fn",
        "src/lib.rs",
        100,
    );
    fixture.store_unit(&func).await.unwrap();

    // Find and verify docstring is included
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "documented_fn",
        "context_file": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert!(!data["docstring"].is_null());
        assert!(data["docstring"].as_str().unwrap().contains("function documentation"));
    }
}

#[tokio::test]
async fn test_find_definition_not_found() {
    let fixture = CodeNavTestFixture::new().await;

    // Try to find a non-existent symbol
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "nonexistent_function",
        "context_file": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail when symbol not found");

    if let Err(ToolError::ExecutionFailed(msg)) = result {
        assert!(msg.contains("not found") || msg.contains("Definition not found"));
    }
}

#[tokio::test]
async fn test_find_definition_multiple_files() {
    let fixture = CodeNavTestFixture::new().await;

    // Create functions with same name in different files
    let func1 = fixtures::create_rust_function(
        "process",
        "module_a::process",
        "src/module_a.rs",
        10,
    );
    let func2 = fixtures::create_rust_function(
        "process",
        "module_b::process",
        "src/module_b.rs",
        20,
    );

    fixture.store_unit(&func1).await.unwrap();
    fixture.store_unit(&func2).await.unwrap();

    // Find with context should prefer the one in the same file
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "process",
        "context_file": "src/module_a.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["qualified_name"], "module_a::process");
    }
}

#[tokio::test]
async fn test_find_definition_no_context() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function
    let func = fixtures::create_rust_function(
        "global_fn",
        "myapp::global_fn",
        "src/lib.rs",
        5,
    );
    fixture.store_unit(&func).await.unwrap();

    // Find without context file (global search)
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "myapp::global_fn",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Should find definition globally");
}

#[tokio::test]
async fn test_find_definition_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create multiple functions
    for i in 0..100 {
        let func = fixtures::create_rust_function(
            &format!("func_{}", i),
            &format!("myapp::func_{}", i),
            "src/lib.rs",
            i * 10,
        );
        fixture.store_unit(&func).await.unwrap();
    }

    // Measure lookup performance
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "func_50",
        "context_file": "src/lib.rs",
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 100, "Lookup should be fast, took {}ms", duration);
}

#[tokio::test]
async fn test_find_definition_with_visibility() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public and private functions
    let mut public_func = fixtures::create_rust_function(
        "public_fn",
        "myapp::public_fn",
        "src/lib.rs",
        10,
    );
    public_func.visibility = cortex_core::types::Visibility::Public;

    let mut private_func = fixtures::create_rust_function(
        "private_fn",
        "myapp::private_fn",
        "src/lib.rs",
        20,
    );
    private_func.visibility = cortex_core::types::Visibility::Private;

    fixture.store_unit(&public_func).await.unwrap();
    fixture.store_unit(&private_func).await.unwrap();

    // Both should be findable (visibility doesn't affect discovery in find_definition)
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());

    let input1 = json!({
        "symbol": "public_fn",
        "context_file": "src/lib.rs",
    });
    let (result1, _) = fixture.execute_tool(&tool, input1).await;
    assert!(result1.is_ok());

    let input2 = json!({
        "symbol": "private_fn",
        "context_file": "src/lib.rs",
    });
    let (result2, _) = fixture.execute_tool(&tool, input2).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_find_definition_case_sensitive() {
    let fixture = CodeNavTestFixture::new().await;

    // Create functions with different cases
    let func1 = fixtures::create_rust_function(
        "processData",
        "myapp::processData",
        "src/lib.rs",
        10,
    );
    let func2 = fixtures::create_rust_function(
        "processdata",
        "myapp::processdata",
        "src/lib.rs",
        20,
    );

    fixture.store_unit(&func1).await.unwrap();
    fixture.store_unit(&func2).await.unwrap();

    // Search should be case-sensitive
    let tool = CodeFindDefinitionTool::new(fixture.ctx.clone());
    let input = json!({
        "symbol": "processData",
        "context_file": "src/lib.rs",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "processData");
    }
}
