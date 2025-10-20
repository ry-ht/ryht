//! Integration Tests for MCP Server
//!
//! This file contains comprehensive integration tests that verify the MCP server
//! functionality including:
//!
//! - Full server initialization flow
//! - Tool registration and execution
//! - Resource registration and reading (TODO: when implemented)
//! - Middleware chain execution (TODO: when implemented)
//! - Hook emission (TODO: when implemented)
//! - Error handling throughout the stack
//! - Multiple tools and resources
//! - Concurrent requests

use mcp_server::prelude::*;
use mcp_server::protocol::{ListToolsResult, ToolContent};
use serde_json::{json, Value};
use std::sync::Arc;

// =============================================================================
// Test Fixtures - Tools
// =============================================================================

/// Simple echo tool for testing
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("Echoes the input message back")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let message = input["message"]
            .as_str()
            .ok_or_else(|| ToolError::ExecutionFailed("message is required".to_string()))?;

        Ok(ToolResult::success_text(message))
    }
}

/// Add tool for testing arithmetic operations
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds two numbers together")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "number"},
                "b": {"type": "number"}
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let a = input["a"].as_f64().ok_or_else(|| {
            ToolError::ExecutionFailed("parameter 'a' must be a number".to_string())
        })?;
        let b = input["b"].as_f64().ok_or_else(|| {
            ToolError::ExecutionFailed("parameter 'b' must be a number".to_string())
        })?;

        let result = a + b;
        Ok(ToolResult::success_json(json!({"result": result})))
    }
}

/// Multiply tool for testing multiple tool registration
struct MultiplyTool;

#[async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &str {
        "multiply"
    }

    fn description(&self) -> Option<&str> {
        Some("Multiplies two numbers")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "x": {"type": "number"},
                "y": {"type": "number"}
            },
            "required": ["x", "y"]
        })
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let x = input["x"].as_f64().ok_or_else(|| {
            ToolError::ExecutionFailed("parameter 'x' must be a number".to_string())
        })?;
        let y = input["y"].as_f64().ok_or_else(|| {
            ToolError::ExecutionFailed("parameter 'y' must be a number".to_string())
        })?;

        let result = x * y;
        Ok(ToolResult::success_json(json!({"result": result})))
    }
}

/// Tool that simulates an error
struct FailingTool;

#[async_trait]
impl Tool for FailingTool {
    fn name(&self) -> &str {
        "failing_tool"
    }

    fn input_schema(&self) -> Value {
        json!({})
    }

    async fn execute(&self, _input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        Err(ToolError::ExecutionFailed("Simulated failure".to_string()))
    }
}

// =============================================================================
// Test Fixtures - Resources (for future implementation)
// =============================================================================

// TODO: Implement when ServerBuilder supports .resource()
// Resource fixtures will be added here when the API supports resource registration

// =============================================================================
// Test Fixtures - Middleware (for future implementation)
// =============================================================================

// TODO: Implement when ServerBuilder supports .middleware()
// Middleware fixtures will be added here when the API supports middleware registration

// =============================================================================
// Test Fixtures - Hooks (for future implementation)
// =============================================================================

// TODO: Implement when ServerBuilder supports .hook()
// Hook fixtures will be added here when the API supports hook registration

// =============================================================================
// Helper Functions
// =============================================================================

/// Helper to call a tool via handle_request
async fn call_tool_via_request(
    server: &McpServer,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<CallToolResult, String> {
    let request = JsonRpcRequest::new(
        Some(json!(1)),
        "tools/call".to_string(),
        Some(json!({
            "name": tool_name,
            "arguments": arguments
        })),
    );

    let response = server.handle_request(request).await;

    if response.is_success() {
        let result: CallToolResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| format!("Failed to parse result: {}", e))?;
        Ok(result)
    } else {
        let error = response.error.unwrap();
        Err(format!("{}: {}", error.code, error.message))
    }
}

/// Helper to list tools via handle_request
async fn list_tools_via_request(server: &McpServer) -> std::result::Result<Vec<ToolDefinition>, String> {
    let request = JsonRpcRequest::new(Some(json!(1)), "tools/list".to_string(), None);

    let response = server.handle_request(request).await;

    if response.is_success() {
        let result: ListToolsResult = serde_json::from_value(response.result.unwrap())
            .map_err(|e| format!("Failed to parse result: {}", e))?;
        Ok(result.tools)
    } else {
        let error = response.error.unwrap();
        Err(format!("{}: {}", error.code, error.message))
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_server_initialization() {
    // Create a basic server
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .build();

    // Verify server config
    assert_eq!(server.config().name(), "test-server");
    assert_eq!(server.config().version(), "1.0.0");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_registration_and_listing() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .tool(AddTool)
        .build();

    // List tools using handle_request
    let tools = list_tools_via_request(&server).await.unwrap();
    assert_eq!(tools.len(), 2);

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(tool_names.contains(&"echo"));
    assert!(tool_names.contains(&"add"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_execution_success() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build();

    let input = json!({"message": "Hello, World!"});

    let result = call_tool_via_request(&server, "echo", input)
        .await
        .expect("Tool execution should succeed");

    assert_eq!(result.is_error, None);  // Successful results don't set is_error
    assert_eq!(result.content.len(), 1);

    if let ToolContent::Text { text } = &result.content[0] {
        assert_eq!(text, "Hello, World!");
    } else {
        panic!("Expected text content");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_execution_with_math() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(AddTool)
        .tool(MultiplyTool)
        .build();

    // Test addition
    let result = call_tool_via_request(&server, "add", json!({"a": 5.0, "b": 3.0}))
        .await
        .expect("Add tool should succeed");

    assert_eq!(result.is_error, None);  // Successful results don't set is_error
    let content_str = match &result.content[0] {
        ToolContent::Text { text } => text,
        _ => panic!("Expected text content"),
    };
    let value: Value = serde_json::from_str(content_str).unwrap();
    assert_eq!(value["result"], 8.0);

    // Test multiplication
    let result = call_tool_via_request(&server, "multiply", json!({"x": 4.0, "y": 7.0}))
        .await
        .expect("Multiply tool should succeed");

    assert_eq!(result.is_error, None);  // Successful results don't set is_error
    let content_str = match &result.content[0] {
        ToolContent::Text { text } => text,
        _ => panic!("Expected text content"),
    };
    let value: Value = serde_json::from_str(content_str).unwrap();
    assert_eq!(value["result"], 28.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_not_found_error() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build();

    let result = call_tool_via_request(&server, "nonexistent", json!({})).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("nonexistent") || error_msg.contains("not found"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_execution_error() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(FailingTool)
        .build();

    let result = call_tool_via_request(&server, "failing_tool", json!({})).await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("Simulated failure") || error_msg.contains("failed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_tool_invalid_input() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(AddTool)
        .build();

    // Missing required parameter 'b'
    let result = call_tool_via_request(&server, "add", json!({"a": 5.0})).await;

    // Should fail due to missing parameter
    assert!(result.is_err());
}

// =============================================================================
// Resource Tests (TODO: Implement when ServerBuilder supports resources)
// =============================================================================

// TODO: Add resource tests when .resource() is implemented

// =============================================================================
// Middleware Tests (TODO: Implement when ServerBuilder supports middleware)
// =============================================================================

// TODO: Add middleware tests when .middleware() is implemented

// =============================================================================
// Hook Tests (TODO: Implement when ServerBuilder supports hooks)
// =============================================================================

// TODO: Add hook tests when .hook() is implemented

// =============================================================================
// Concurrent Execution Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_tool_execution() {
    let server = Arc::new(
        McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .tool(AddTool)
            .tool(MultiplyTool)
            .build(),
    );

    let mut handles = vec![];

    // Spawn 10 concurrent tasks
    for i in 0..10 {
        let server = Arc::clone(&server);
        let handle = tokio::spawn(async move {
            if i % 2 == 0 {
                // Even: use add
                call_tool_via_request(&server, "add", json!({"a": i as f64, "b": 1.0})).await
            } else {
                // Odd: use multiply
                call_tool_via_request(&server, "multiply", json!({"x": i as f64, "y": 2.0})).await
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10);
}

// =============================================================================
// Full Lifecycle Tests (Partial - without middleware/hooks/resources)
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_partial_server_lifecycle() {
    // Build server with tools only (resources, middleware, hooks not yet supported)
    let server = McpServer::builder()
        .name("full-lifecycle-test")
        .version("1.0.0")
        .protocol_version("2025-03-26")
        .tool(EchoTool)
        .tool(AddTool)
        .tool(MultiplyTool)
        .build();

    // Verify server config
    assert_eq!(server.config().name(), "full-lifecycle-test");
    assert_eq!(server.config().version(), "1.0.0");
    assert_eq!(server.config().protocol_version(), "2025-03-26");

    // List and verify tools
    let tools = list_tools_via_request(&server).await.unwrap();
    assert_eq!(tools.len(), 3);

    // Execute tools
    let _ = call_tool_via_request(&server, "echo", json!({"message": "test"}))
        .await
        .unwrap();
    let _ = call_tool_via_request(&server, "add", json!({"a": 1.0, "b": 2.0}))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_tools_and_resources() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .tool(AddTool)
        .tool(MultiplyTool)
        .tool(FailingTool)
        .build();

    // Verify all tools are registered
    let tools = list_tools_via_request(&server).await.unwrap();
    assert_eq!(tools.len(), 4);

    // Test each tool works independently
    assert!(call_tool_via_request(&server, "echo", json!({"message": "test"}))
        .await
        .is_ok());
    assert!(call_tool_via_request(&server, "add", json!({"a": 1.0, "b": 2.0}))
        .await
        .is_ok());
    assert!(call_tool_via_request(&server, "multiply", json!({"x": 3.0, "y": 4.0}))
        .await
        .is_ok());
    assert!(call_tool_via_request(&server, "failing_tool", json!({}))
        .await
        .is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_context_passing() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build();

    // Note: Context is created internally by handle_request
    // We're testing that the server properly handles tool execution
    let result = call_tool_via_request(&server, "echo", json!({"message": "test"}))
        .await
        .unwrap();

    assert_eq!(result.is_error, None);  // Successful results don't set is_error
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_capabilities() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build();

    // Test via initialize request
    let request = JsonRpcRequest::new(
        Some(json!(1)),
        "initialize".to_string(),
        Some(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    );

    let response = server.handle_request(request).await;
    assert!(response.is_success());

    let result: InitializeResult = serde_json::from_value(response.result.unwrap()).unwrap();

    // Verify tools capability is present
    assert!(result.capabilities.tools.is_some());

    // Resources capability should be None or Some(empty) since we haven't added resources
    // (This is implementation-dependent)
}
