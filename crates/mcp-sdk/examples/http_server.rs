//! HTTP MCP Server Example
//!
//! This example demonstrates how to run an MCP server with HTTP transport,
//! making it accessible via HTTP POST requests instead of stdio.
//!
//! # Features Demonstrated
//!
//! - **HTTP Transport**: Server listening on localhost
//! - **Multiple Tools**: Echo and calculator tools
//! - **CORS Configuration**: Cross-origin resource sharing enabled
//! - **SSE Support**: Server-sent events for streaming (if implemented)
//! - **RESTful JSON-RPC**: Standard JSON-RPC over HTTP POST
//!
//! # Requirements
//!
//! This example requires the `http` feature to be enabled:
//!
//! ```bash
//! cargo run --example http_server --features http
//! ```
//!
//! # Usage
//!
//! Once the server is running, you can send JSON-RPC requests via HTTP POST:
//!
//! ```bash
//! # Initialize
//! curl -X POST http://localhost:3000/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"curl","version":"1.0"}}}'
//!
//! # List tools
//! curl -X POST http://localhost:3000/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
//!
//! # Call echo tool
//! curl -X POST http://localhost:3000/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello from HTTP!"}}}'
//!
//! # Call calculator tool
//! curl -X POST http://localhost:3000/mcp \
//!   -H "Content-Type: application/json" \
//!   -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"calculator","arguments":{"operation":"add","a":42,"b":8}}}'
//! ```

#[cfg(not(feature = "http"))]
fn main() {
    eprintln!("ERROR: This example requires the 'http' feature to be enabled.");
    eprintln!("Run with: cargo run --example http_server --features http");
    std::process::exit(1);
}

#[cfg(feature = "http")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}

#[cfg(feature = "http")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    use mcp_server::prelude::*;
    use serde_json::{json, Value};
    use std::net::SocketAddr;

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // =============================================================================
    // Tools
    // =============================================================================

    /// Echo tool - returns the input message
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

        async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
            let message = input["message"]
                .as_str()
                .ok_or_else(|| ToolError::ExecutionFailed("message is required".to_string()))?;

            Ok(ToolResult::success_text(message))
        }
    }

    /// Calculator tool - performs arithmetic operations
    struct CalculatorTool;

    #[async_trait]
    impl Tool for CalculatorTool {
        fn name(&self) -> &str {
            "calculator"
        }

        fn description(&self) -> Option<&str> {
            Some("Performs basic arithmetic operations (add, subtract, multiply, divide)")
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"],
                        "description": "The arithmetic operation to perform"
                    },
                    "a": {
                        "type": "number",
                        "description": "First operand"
                    },
                    "b": {
                        "type": "number",
                        "description": "Second operand"
                    }
                },
                "required": ["operation", "a", "b"]
            })
        }

        async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
            let operation = input["operation"]
                .as_str()
                .ok_or_else(|| ToolError::ExecutionFailed("operation is required".to_string()))?;

            let a = input["a"]
                .as_f64()
                .ok_or_else(|| ToolError::ExecutionFailed("'a' must be a number".to_string()))?;

            let b = input["b"]
                .as_f64()
                .ok_or_else(|| ToolError::ExecutionFailed("'b' must be a number".to_string()))?;

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(ToolError::ExecutionFailed("Division by zero".to_string()));
                    }
                    a / b
                }
                _ => return Err(ToolError::ExecutionFailed("Invalid operation".to_string())),
            };

            Ok(ToolResult::success_json(json!({
                "operation": operation,
                "a": a,
                "b": b,
                "result": result
            })))
        }
    }

    /// Health check resource
    struct HealthResource;

    #[async_trait]
    impl Resource for HealthResource {
        fn uri(&self) -> &str {
            "health://status"
        }

        fn name(&self) -> Option<&str> {
            Some("Health Status")
        }

        fn description(&self) -> Option<&str> {
            Some("Server health check endpoint")
        }

        fn mime_type(&self) -> Option<&str> {
            Some("application/json")
        }

        async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
            Ok(ResourceContent::text(
                json!({
                    "status": "healthy",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }).to_string(),
                "application/json",
            ))
        }
    }

    // =============================================================================
    // Middleware
    // =============================================================================

    /// Request logging middleware
    struct RequestLogger;

    #[async_trait]
    impl Middleware for RequestLogger {
        async fn on_request(
            &self,
            request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            tracing::info!(
                method = %request.method,
                id = ?request.id,
                "Incoming HTTP request"
            );
            Ok(())
        }

        async fn on_response(
            &self,
            response: &JsonRpcResponse,
            _context: &RequestContext,
        ) -> Result<(), MiddlewareError> {
            if response.is_error() {
                tracing::warn!(id = ?response.id, "Request failed");
            } else {
                tracing::info!(id = ?response.id, "Request completed");
            }
            Ok(())
        }
    }

    // =============================================================================
    // Server Setup
    // =============================================================================

    // Build the MCP server
    let server = McpServer::builder()
        .name("http-mcp-server")
        .version("1.0.0")
        .protocol_version("2025-03-26")
        .tool(EchoTool)
        .tool(CalculatorTool)
        .resource(HealthResource)
        .middleware(RequestLogger)
        .build();

    // Configure HTTP address
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;

    // Print server information
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║            HTTP MCP Server Example                    ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ Server:           {}                     ║", server.config().name());
    println!("║ Version:          {}                           ║", server.config().version());
    println!("║ Protocol Version: {}                      ║", server.config().protocol_version());
    println!("║ Address:          http://{}                ║", addr);
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ Endpoint:         POST /mcp                            ║");
    println!("║ Tools:            2 (echo, calculator)                 ║");
    println!("║ Resources:        1 (health://status)                  ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    println!("Example requests:");
    println!();
    println!("  # Initialize");
    println!("  curl -X POST http://{}/mcp \\", addr);
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"2025-03-26\",\"capabilities\":{{}},\"clientInfo\":{{\"name\":\"curl\",\"version\":\"1.0\"}}}}}}'");
    println!();
    println!("  # List tools");
    println!("  curl -X POST http://{}/mcp \\", addr);
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}}'");
    println!();
    println!("  # Call echo tool");
    println!("  curl -X POST http://{}/mcp \\", addr);
    println!("    -H \"Content-Type: application/json\" \\");
    println!("    -d '{{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{{\"name\":\"echo\",\"arguments\":{{\"message\":\"Hello!\"}}}}}}'");
    println!();
    println!("Server is now listening...");
    println!();

    // Create HTTP transport with CORS enabled
    let transport = HttpTransport::builder()
        .address(addr)
        .cors_enabled(true)
        .build();

    // Serve forever
    server.serve(transport).await?;

    Ok(())
}
