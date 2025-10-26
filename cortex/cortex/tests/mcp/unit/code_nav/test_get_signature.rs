//! Unit Tests for cortex.code.get_signature
//!
//! Tests cover:
//! - Getting function signatures
//! - Getting method signatures
//! - Parameter information (name, type, optional, default)
//! - Return type information
//! - Visibility and modifiers
//! - Functions without parameters
//! - Functions with multiple parameters
//! - Generic/templated functions
//! - Error handling
//! - Performance measurement

use super::test_helpers::*;
use cortex::mcp::tools::code_nav::CodeGetSignatureTool;
use cortex_core::types::Parameter;
use mcp_sdk::prelude::*;
use serde_json::json;

#[tokio::test]
async fn test_get_signature_simple_function() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a simple function
    let func = fixtures::create_rust_function(
        "add",
        "myapp::math::add",
        "src/math.rs",
        10,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Failed to get signature");
    assert!(duration < 100, "Took too long: {}ms", duration);

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "add");
        assert_eq!(data["qualified_name"], "myapp::math::add");
        assert!(!data["signature"].is_null());
        assert!(data["signature"].as_str().unwrap().contains("add"));
    }
}

#[tokio::test]
async fn test_get_signature_with_parameters() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with parameters
    let params = vec![
        Parameter {
            name: "x".to_string(),
            param_type: Some("i32".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
        Parameter {
            name: "y".to_string(),
            param_type: Some("i32".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
    ];

    let func = fixtures::create_rust_function_with_params(
        "multiply",
        "myapp::math::multiply",
        "src/math.rs",
        20,
        params,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let parameters = data["parameters"].as_array().unwrap();

        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0]["name"], "x");
        assert_eq!(parameters[0]["param_type"], "i32");
        assert_eq!(parameters[1]["name"], "y");
        assert_eq!(parameters[1]["param_type"], "i32");
    }
}

#[tokio::test]
async fn test_get_signature_with_return_type() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with specific return type
    let mut func = fixtures::create_rust_function(
        "calculate",
        "myapp::calculate",
        "src/lib.rs",
        10,
    );
    func.return_type = Some("Result<i64, Error>".to_string());

    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["return_type"], "Result<i64, Error>");
    }
}

#[tokio::test]
async fn test_get_signature_with_visibility() {
    let fixture = CodeNavTestFixture::new().await;

    // Create public function
    let mut func = fixtures::create_rust_function(
        "public_api",
        "myapp::public_api",
        "src/lib.rs",
        10,
    );
    func.visibility = cortex_core::types::Visibility::Public;

    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["visibility"], "Public");
    }
}

#[tokio::test]
async fn test_get_signature_with_modifiers() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with modifiers
    let mut func = fixtures::create_rust_function(
        "async_fn",
        "myapp::async_fn",
        "src/lib.rs",
        10,
    );
    func.modifiers = vec!["pub".to_string(), "async".to_string()];

    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let modifiers = data["modifiers"].as_array().unwrap();

        assert!(modifiers.iter().any(|m| m == "pub"));
        assert!(modifiers.iter().any(|m| m == "async"));
    }
}

#[tokio::test]
async fn test_get_signature_optional_parameters() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with optional parameter
    let params = vec![
        Parameter {
            name: "required".to_string(),
            param_type: Some("String".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
        Parameter {
            name: "optional".to_string(),
            param_type: Some("Option<i32>".to_string()),
            is_optional: true,
            default_value: Some("None".to_string()),
            is_variadic: false,
            attributes: Vec::new(),
        },
    ];

    let func = fixtures::create_rust_function_with_params(
        "configure",
        "myapp::configure",
        "src/lib.rs",
        10,
        params,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let parameters = data["parameters"].as_array().unwrap();

        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0]["is_optional"], false);
        assert_eq!(parameters[1]["is_optional"], true);
        assert_eq!(parameters[1]["default_value"], "None");
    }
}

#[tokio::test]
async fn test_get_signature_no_parameters() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function without parameters
    let func = fixtures::create_rust_function(
        "initialize",
        "myapp::initialize",
        "src/lib.rs",
        10,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let parameters = data["parameters"].as_array().unwrap();
        assert_eq!(parameters.len(), 0);
    }
}

#[tokio::test]
async fn test_get_signature_typescript_method() {
    let fixture = CodeNavTestFixture::new().await;

    // Create TypeScript method
    let method = fixtures::create_typescript_method(
        "getUserById",
        "app.services.UserService.getUserById",
        "src/services.ts",
        20,
    );
    let method_id = fixture.store_unit(&method).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": method_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "getUserById");
        assert!(!data["signature"].is_null());
    }
}

#[tokio::test]
async fn test_get_signature_many_parameters() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with many parameters
    let params: Vec<Parameter> = (0..10)
        .map(|i| Parameter {
            name: format!("param{}", i),
            param_type: Some("String".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        })
        .collect();

    let func = fixtures::create_rust_function_with_params(
        "complex_fn",
        "myapp::complex_fn",
        "src/lib.rs",
        10,
        params,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let parameters = data["parameters"].as_array().unwrap();
        assert_eq!(parameters.len(), 10);

        // Verify all parameters are present
        for i in 0..10 {
            assert_eq!(parameters[i]["name"], format!("param{}", i));
        }
    }
}

#[tokio::test]
async fn test_get_signature_with_parameter_types() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with various parameter types
    let params = vec![
        Parameter {
            name: "count".to_string(),
            param_type: Some("usize".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
        Parameter {
            name: "name".to_string(),
            param_type: Some("&str".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
        Parameter {
            name: "data".to_string(),
            param_type: Some("Vec<u8>".to_string()),
            is_optional: false,
            default_value: None,
            is_variadic: false,
            attributes: Vec::new(),
        },
    ];

    let func = fixtures::create_rust_function_with_params(
        "process",
        "myapp::process",
        "src/lib.rs",
        10,
        params,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        let parameters = data["parameters"].as_array().unwrap();

        assert_eq!(parameters[0]["param_type"], "usize");
        assert_eq!(parameters[1]["param_type"], "&str");
        assert_eq!(parameters[2]["param_type"], "Vec<u8>");
    }
}

#[tokio::test]
async fn test_get_signature_error_invalid_id() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with invalid unit_id
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": "not-a-valid-id",
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail with invalid unit_id");
}

#[tokio::test]
async fn test_get_signature_error_unit_not_found() {
    let fixture = CodeNavTestFixture::new().await;

    // Try with non-existent unit_id
    let fake_id = cortex_core::id::CortexId::new();
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": fake_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_err(), "Should fail when unit not found");
}

#[tokio::test]
async fn test_get_signature_includes_qualified_name() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with qualified name
    let func = fixtures::create_rust_function(
        "connect",
        "myapp::database::connection::connect",
        "src/db/connection.rs",
        10,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["qualified_name"], "myapp::database::connection::connect");
    }
}

#[tokio::test]
async fn test_get_signature_private_function() {
    let fixture = CodeNavTestFixture::new().await;

    // Create private function
    let mut func = fixtures::create_rust_function(
        "helper",
        "myapp::helper",
        "src/lib.rs",
        10,
    );
    func.visibility = cortex_core::types::Visibility::Private;

    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature (should work for private functions too)
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok(), "Should be able to get signature of private functions");

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["visibility"], "Private");
    }
}

#[tokio::test]
async fn test_get_signature_struct_constructor() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a struct (which might have a constructor)
    let struct_unit = fixtures::create_rust_struct(
        "Config",
        "myapp::Config",
        "src/config.rs",
        10,
    );
    let struct_id = fixture.store_unit(&struct_unit).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": struct_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();
        assert_eq!(data["name"], "Config");
    }
}

#[tokio::test]
async fn test_get_signature_complete_metadata() {
    let fixture = CodeNavTestFixture::new().await;

    // Create function with complete metadata
    let params = vec![Parameter {
        name: "value".to_string(),
        param_type: Some("i32".to_string()),
        is_optional: false,
        default_value: None,
        is_variadic: false,
        attributes: Vec::new(),
    }];

    let func = fixtures::create_rust_function_with_params(
        "increment",
        "myapp::math::increment",
        "src/math.rs",
        10,
        params,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature and verify all fields
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Verify all required fields are present
        assert!(!data["unit_id"].is_null());
        assert!(!data["name"].is_null());
        assert!(!data["qualified_name"].is_null());
        assert!(!data["signature"].is_null());
        assert!(!data["parameters"].is_null());
        assert!(!data["return_type"].is_null());
        assert!(!data["visibility"].is_null());
        assert!(!data["modifiers"].is_null());
    }
}

#[tokio::test]
async fn test_get_signature_performance() {
    let fixture = CodeNavTestFixture::new().await;

    // Create many functions
    let mut func_ids = Vec::new();
    for i in 0..100 {
        let func = fixtures::create_rust_function(
            &format!("fn_{}", i),
            &format!("myapp::fn_{}", i),
            "src/lib.rs",
            i * 10,
        );
        let func_id = fixture.store_unit(&func).await.unwrap();
        func_ids.push(func_id);
    }

    // Measure signature retrieval performance
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_ids[50].to_string(),
    });

    let (result, duration) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());
    assert!(duration < 50, "Signature lookup should be very fast, took {}ms", duration);
}

#[tokio::test]
async fn test_get_signature_output_format() {
    let fixture = CodeNavTestFixture::new().await;

    // Create a function
    let func = fixtures::create_rust_function(
        "test_fn",
        "myapp::test_fn",
        "src/lib.rs",
        10,
    );
    let func_id = fixture.store_unit(&func).await.unwrap();

    // Get signature
    let tool = CodeGetSignatureTool::new(fixture.ctx.clone());
    let input = json!({
        "unit_id": func_id.to_string(),
    });

    let (result, _) = fixture.execute_tool(&tool, input).await;
    assert!(result.is_ok());

    if let Ok(ToolResult::Success { content }) = result {
        let data: serde_json::Value = serde_json::from_value(content).unwrap();

        // Verify the output is a valid JSON object with expected structure
        assert!(data.is_object());
        assert!(data.get("unit_id").is_some());
        assert!(data.get("name").is_some());
        assert!(data.get("signature").is_some());
    }
}
