//! Integration tests for MCP module enhancements
//!
//! This test file verifies the new functionality added to the cc-sdk MCP module,
//! including error conversion, tool registry wrapper, config validation, and middleware.

use cc_sdk::mcp::{
    self, validate_mcp_config, Tool, ToolContext, ToolRegistry, ToolResult,
};
use cc_sdk::options::McpServerConfig;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

// ============================================================================
// Test Tools
// ============================================================================

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("Echoes back the input")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {"type": "string"}
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, mcp_sdk::error::ToolError> {
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("no message");
        Ok(ToolResult::success_text(message))
    }
}

struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> Option<&str> {
        Some("Performs basic arithmetic")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["add", "subtract", "multiply", "divide"]},
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["operation", "a", "b"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, mcp_sdk::error::ToolError> {
        let operation = input.get("operation").and_then(|v| v.as_str()).unwrap();
        let a = input.get("a").and_then(|v| v.as_f64()).unwrap();
        let b = input.get("b").and_then(|v| v.as_f64()).unwrap();

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Err(mcp_sdk::error::ToolError::ExecutionFailed(
                        "Division by zero".to_string(),
                    ));
                }
                a / b
            }
            _ => {
                return Err(mcp_sdk::error::ToolError::ExecutionFailed(
                    format!("Unknown operation: {}", operation),
                ))
            }
        };

        Ok(ToolResult::success_text(&format!("{}", result)))
    }
}

// ============================================================================
// Error Conversion Tests
// ============================================================================

#[test]
fn test_error_conversion_tool_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::ToolError;

    let mcp_error = ToolError::NotFound("my_tool".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Protocol(msg) => {
            assert!(msg.contains("MCP tool error"));
            assert!(msg.contains("my_tool"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_error_conversion_tool_error_execution_failed() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::ToolError;

    let mcp_error = ToolError::ExecutionFailed("something went wrong".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Protocol(msg) => {
            assert!(msg.contains("MCP tool error"));
            assert!(msg.contains("something went wrong"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_error_conversion_resource_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::ResourceError;

    let mcp_error = ResourceError::NotFound("file://test.txt".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Protocol(msg) => {
            assert!(msg.contains("MCP resource error"));
            assert!(msg.contains("file://test.txt"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_error_conversion_middleware_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::MiddlewareError;

    let mcp_error = MiddlewareError::Blocked("rate limit exceeded".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Protocol(msg) => {
            assert!(msg.contains("MCP middleware error"));
            assert!(msg.contains("rate limit exceeded"));
        }
        _ => panic!("Expected Protocol error"),
    }
}

#[test]
fn test_error_conversion_transport_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::{McpError, TransportError};

    let mcp_error = McpError::Transport(TransportError::Closed);
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Transport(t) => match t {
            cc_sdk::error::TransportError::Closed => {}
            _ => panic!("Expected Closed transport error"),
        },
        _ => panic!("Expected Transport error"),
    }
}

#[test]
fn test_error_conversion_config_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::McpError;

    let mcp_error = McpError::Config("invalid configuration".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Config(msg) => {
            assert_eq!(msg, "invalid configuration");
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_error_conversion_protocol_error() {
    use cc_sdk::error::Error;
    use mcp_sdk::error::McpError;

    let mcp_error = McpError::Protocol("protocol violation".to_string());
    let cc_error: Error = mcp_error.into();

    match cc_error {
        Error::Protocol(msg) => {
            assert_eq!(msg, "protocol violation");
        }
        _ => panic!("Expected Protocol error"),
    }
}

// ============================================================================
// Tool Registry Tests
// ============================================================================

#[tokio::test]
async fn test_tool_registry_new() {
    let registry = ToolRegistry::new();
    assert_eq!(registry.count().await, 0);
}

#[tokio::test]
async fn test_tool_registry_default() {
    let registry = ToolRegistry::default();
    assert_eq!(registry.count().await, 0);
}

#[tokio::test]
async fn test_tool_registry_register() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    assert_eq!(registry.count().await, 1);
    assert!(registry.has("echo").await);
}

#[tokio::test]
async fn test_tool_registry_register_multiple() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();
    registry.register(CalculatorTool).await.unwrap();

    assert_eq!(registry.count().await, 2);
    assert!(registry.has("echo").await);
    assert!(registry.has("calculator").await);
}

#[tokio::test]
async fn test_tool_registry_register_duplicate_error() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    let result = registry.register(EchoTool).await;
    assert!(result.is_err());

    // Should still only have one tool
    assert_eq!(registry.count().await, 1);
}

#[tokio::test]
async fn test_tool_registry_get() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    let tool = registry.get("echo").await;
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().name(), "echo");
}

#[tokio::test]
async fn test_tool_registry_get_nonexistent() {
    let registry = ToolRegistry::new();
    let tool = registry.get("nonexistent").await;
    assert!(tool.is_none());
}

#[tokio::test]
async fn test_tool_registry_has() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    assert!(registry.has("echo").await);
    assert!(!registry.has("nonexistent").await);
}

#[tokio::test]
async fn test_tool_registry_list() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();
    registry.register(CalculatorTool).await.unwrap();

    let definitions = registry.list().await;
    assert_eq!(definitions.len(), 2);

    let names: Vec<&str> = definitions.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"echo"));
    assert!(names.contains(&"calculator"));
}

#[tokio::test]
async fn test_tool_registry_list_tool_names() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();
    registry.register(CalculatorTool).await.unwrap();

    let names = registry.list_tool_names().await;
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"echo".to_string()));
    assert!(names.contains(&"calculator".to_string()));
}

#[tokio::test]
async fn test_tool_registry_clone() {
    let registry1 = ToolRegistry::new();
    registry1.register(EchoTool).await.unwrap();

    let registry2 = registry1.clone();
    assert!(registry2.has("echo").await);

    // Both registries share the same underlying storage
    registry2.register(CalculatorTool).await.unwrap();
    assert!(registry1.has("calculator").await);
}

#[tokio::test]
async fn test_tool_registry_inner_access() {
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    let inner = registry.inner();
    assert!(inner.has("echo").await);
}

#[tokio::test]
async fn test_tool_registry_register_arc() {
    let registry = ToolRegistry::new();
    let tool = Arc::new(EchoTool);

    registry.register_arc(tool).await.unwrap();
    assert_eq!(registry.count().await, 1);
    assert!(registry.has("echo").await);
}

// ============================================================================
// Config Validation Tests
// ============================================================================

#[test]
fn test_validate_stdio_config_valid() {
    let config = McpServerConfig::Stdio {
        command: "node".to_string(),
        args: Some(vec!["server.js".to_string()]),
        env: None,
    };

    assert!(validate_mcp_config(&config).is_ok());
}

#[test]
fn test_validate_stdio_config_empty_command() {
    let config = McpServerConfig::Stdio {
        command: "".to_string(),
        args: None,
        env: None,
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        cc_sdk::error::Error::Config(msg) => {
            assert!(msg.contains("command cannot be empty"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_validate_sse_config_valid() {
    let config = McpServerConfig::Sse {
        url: "https://example.com/events".to_string(),
        headers: None,
    };

    assert!(validate_mcp_config(&config).is_ok());
}

#[test]
fn test_validate_sse_config_empty_url() {
    let config = McpServerConfig::Sse {
        url: "".to_string(),
        headers: None,
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        cc_sdk::error::Error::Config(msg) => {
            assert!(msg.contains("URL cannot be empty"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_validate_sse_config_invalid_url() {
    let config = McpServerConfig::Sse {
        url: "not-a-url".to_string(),
        headers: None,
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        cc_sdk::error::Error::Config(msg) => {
            assert!(msg.contains("must start with http://"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_validate_http_config_valid() {
    let config = McpServerConfig::Http {
        url: "http://localhost:3000".to_string(),
        headers: None,
    };

    assert!(validate_mcp_config(&config).is_ok());
}

#[test]
fn test_validate_http_config_empty_url() {
    let config = McpServerConfig::Http {
        url: "".to_string(),
        headers: None,
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_http_config_invalid_url() {
    let config = McpServerConfig::Http {
        url: "ftp://invalid".to_string(),
        headers: None,
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_sdk_config_valid() {
    let server = mcp::McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .build();

    let config = McpServerConfig::Sdk {
        name: "test-server".to_string(),
        instance: Arc::new(server),
    };

    assert!(validate_mcp_config(&config).is_ok());
}

#[test]
fn test_validate_sdk_config_empty_name() {
    let server = mcp::McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .build();

    let config = McpServerConfig::Sdk {
        name: "".to_string(),
        instance: Arc::new(server),
    };

    let result = validate_mcp_config(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        cc_sdk::error::Error::Config(msg) => {
            assert!(msg.contains("name cannot be empty"));
        }
        _ => panic!("Expected Config error"),
    }
}

// ============================================================================
// Middleware Tests
// ============================================================================

#[test]
fn test_middleware_logging_creation() {
    let _logging_mw = mcp::middleware::logging();
    // Just verify it can be created without panicking
}

#[test]
fn test_middleware_metrics_creation() {
    let _metrics_mw = mcp::middleware::metrics();
    // Just verify it can be created without panicking
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_workflow_with_registry_and_server() {
    // Create a tool registry
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();
    registry.register(CalculatorTool).await.unwrap();

    // Verify tools are registered
    assert_eq!(registry.count().await, 2);

    // List all tools
    let tool_names = registry.list_tool_names().await;
    assert_eq!(tool_names.len(), 2);

    // Get specific tool
    let echo_tool = registry.get("echo").await;
    assert!(echo_tool.is_some());

    // Execute tool
    let result = echo_tool
        .unwrap()
        .execute(
            json!({"message": "Hello, World!"}),
            &ToolContext::new(),
        )
        .await
        .unwrap();

    assert!(result.is_success());
}

#[tokio::test]
async fn test_calculator_tool_execution() {
    let registry = ToolRegistry::new();
    registry.register(CalculatorTool).await.unwrap();

    let calc_tool = registry.get("calculator").await.unwrap();

    // Test addition
    let result = calc_tool
        .execute(
            json!({
                "operation": "add",
                "a": 5.0,
                "b": 3.0
            }),
            &ToolContext::new(),
        )
        .await
        .unwrap();

    assert!(result.is_success());
    if let Some(content) = result.content.get(0) {
        if let Some(text) = content.as_text() {
            // The result is formatted as a string
            assert!(text.contains("8"));
        }
    }

    // Test division by zero
    let result = calc_tool
        .execute(
            json!({
                "operation": "divide",
                "a": 10.0,
                "b": 0.0
            }),
            &ToolContext::new(),
        )
        .await;

    assert!(result.is_err());
}

#[test]
fn test_config_to_server_builder_sdk() {
    let server = mcp::McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .build();

    let config = McpServerConfig::Sdk {
        name: "test-server".to_string(),
        instance: Arc::new(server),
    };

    let result = mcp::config_to_server_builder(&config);
    assert!(result.is_ok());
}

#[test]
fn test_config_to_server_builder_stdio_error() {
    let config = McpServerConfig::Stdio {
        command: "node".to_string(),
        args: Some(vec!["server.js".to_string()]),
        env: None,
    };

    let result = mcp::config_to_server_builder(&config);
    assert!(result.is_err());

    match result.unwrap_err() {
        cc_sdk::error::Error::Config(msg) => {
            assert!(msg.contains("Cannot convert"));
        }
        _ => panic!("Expected Config error"),
    }
}

#[test]
fn test_create_sdk_server_config() {
    let server = mcp::McpServer::builder()
        .name("my-server")
        .version("1.0.0")
        .build();

    let config = mcp::create_sdk_server_config("my-server", Arc::new(server));

    match config {
        McpServerConfig::Sdk { name, .. } => {
            assert_eq!(name, "my-server");
        }
        _ => panic!("Expected Sdk config"),
    }
}

// ============================================================================
// Documentation Examples Verification
// ============================================================================

#[tokio::test]
async fn test_readme_example_tool_registry() {
    // Verify the example from the module documentation works
    let registry = ToolRegistry::new();
    registry.register(EchoTool).await.unwrap();

    let tool_names = registry.list_tool_names().await;
    assert!(tool_names.contains(&"echo".to_string()));
}

#[test]
fn test_readme_example_error_conversion() {
    use cc_sdk::error::Error;

    // Verify the error conversion example from documentation
    let mcp_error = mcp::error::ToolError::NotFound("my_tool".to_string());
    let _cc_error: Error = mcp_error.into();
    // No panic means success
}

#[test]
fn test_readme_example_config_validation() {
    // Verify the config validation example from documentation
    let config = McpServerConfig::Stdio {
        command: "node".to_string(),
        args: Some(vec!["server.js".to_string()]),
        env: None,
    };

    validate_mcp_config(&config).unwrap();
}
