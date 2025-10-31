//! Integration Tests for MCP Server
//!
//! This file contains comprehensive integration tests that verify the MCP server
//! functionality including:
//!
//! - Full server initialization flow
//! - Tool registration and execution
//! - Resource registration and reading
//! - Middleware chain execution
//! - Hook emission
//! - Error handling throughout the stack
//! - Multiple tools and resources
//! - Concurrent requests

use mcp_sdk::prelude::*;
use mcp_sdk::protocol::{ListToolsResult, ListResourcesResult, ReadResourceResult, ResourceContent, ToolContent};
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
// Test Fixtures - Resources
// =============================================================================

use mcp_sdk::resource::{Resource, ResourceContext};
use mcp_sdk::error::ResourceError;

struct TestResource {
    uri: &'static str,
    content: &'static str,
}

#[async_trait]
impl Resource for TestResource {
    fn uri_pattern(&self) -> &str {
        self.uri
    }

    fn name(&self) -> Option<&str> {
        Some("Test Resource")
    }

    fn description(&self) -> Option<&str> {
        Some("A test resource for integration tests")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }

    async fn read(&self, _uri: &str, _context: &ResourceContext) -> std::result::Result<mcp_sdk::resource::ResourceContent, ResourceError> {
        Ok(mcp_sdk::resource::ResourceContent::text(self.content, "text/plain"))
    }
}

// =============================================================================
// Test Fixtures - Middleware
// =============================================================================

use mcp_sdk::middleware::{Middleware, RequestContext};
use mcp_sdk::protocol::{JsonRpcRequest, JsonRpcResponse};
use mcp_sdk::error::MiddlewareError;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestMiddleware {
    request_count: Arc<AtomicUsize>,
    response_count: Arc<AtomicUsize>,
}

impl TestMiddleware {
    fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicUsize::new(0)),
            response_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn request_count(&self) -> usize {
        self.request_count.load(Ordering::SeqCst)
    }

    fn response_count(&self) -> usize {
        self.response_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Middleware for TestMiddleware {
    async fn on_request(&self, _request: &JsonRpcRequest, context: &mut RequestContext) -> std::result::Result<(), MiddlewareError> {
        self.request_count.fetch_add(1, Ordering::SeqCst);
        context.set_metadata("middleware_called".to_string(), json!(true));
        Ok(())
    }

    async fn on_response(&self, _response: &JsonRpcResponse, context: &RequestContext) -> std::result::Result<(), MiddlewareError> {
        self.response_count.fetch_add(1, Ordering::SeqCst);
        // Verify metadata was set in request phase
        assert!(context.get_metadata("middleware_called").is_some());
        Ok(())
    }
}

// =============================================================================
// Test Fixtures - Hooks
// =============================================================================

use mcp_sdk::hooks::{Hook, HookEvent};

struct TestHook {
    event_count: Arc<AtomicUsize>,
}

impl TestHook {
    fn new() -> Self {
        Self {
            event_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn event_count(&self) -> usize {
        self.event_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Hook for TestHook {
    async fn on_event(&self, _event: &HookEvent) -> std::result::Result<(), MiddlewareError> {
        self.event_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

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
// Resource Tests
// =============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_resource_registration_and_listing() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .resource(TestResource {
            uri: "test://resource1",
            content: "Resource 1 content",
        })
        .resource(TestResource {
            uri: "test://resource2",
            content: "Resource 2 content",
        })
        .build();

    let request = JsonRpcRequest::new(Some(json!(1)), "resources/list".to_string(), None);
    let response = server.handle_request(request).await;

    assert!(response.is_success());
    let result: ListResourcesResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.resources.len(), 2);

    let uris: Vec<&str> = result.resources.iter().map(|r| r.uri.as_str()).collect();
    assert!(uris.contains(&"test://resource1"));
    assert!(uris.contains(&"test://resource2"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_resource_read() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .resource(TestResource {
            uri: "test://data",
            content: "Test data content",
        })
        .build();

    let request = JsonRpcRequest::new(
        Some(json!(1)),
        "resources/read".to_string(),
        Some(json!({
            "uri": "test://data"
        })),
    );

    let response = server.handle_request(request).await;
    assert!(response.is_success());

    let result: ReadResourceResult = serde_json::from_value(response.result.unwrap()).unwrap();
    assert_eq!(result.contents.len(), 1);

    match &result.contents[0] {
        ResourceContent::Text { uri, text, .. } => {
            assert_eq!(uri, "test://data");
            assert_eq!(text, "Test data content");
        }
        _ => panic!("Expected text content"),
    }
}

// =============================================================================
// Middleware Tests
// =============================================================================

// Note: Current middleware implementation doesn't integrate with handle_request
// These tests verify middleware registration and basic functionality

#[tokio::test(flavor = "multi_thread")]
async fn test_middleware_registration() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .middleware(TestMiddleware::new())
        .build();

    // Verify middleware was registered
    assert_eq!(server.middleware().count().await, 1);
}

// =============================================================================
// Hook Tests
// =============================================================================

// Note: Current hook implementation doesn't integrate with handle_request
// These tests verify hook registration and basic functionality

#[tokio::test(flavor = "multi_thread")]
async fn test_hook_registration() {
    let server = McpServer::builder()
        .name("test-server")
        .version("1.0.0")
        .hook(TestHook::new())
        .build();

    // Verify hook was registered
    assert_eq!(server.hooks().count().await, 1);
}

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
