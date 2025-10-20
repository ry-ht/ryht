//! Simple MCP Server Example
//!
//! This example demonstrates the minimal code needed to create a working MCP server
//! with a single echo tool using stdio transport.
//!
//! # Features Demonstrated
//!
//! - Single tool registration
//! - Stdio transport (reads from stdin, writes to stdout)
//! - Basic error handling
//! - No middleware or hooks
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example simple
//! ```
//!
//! Then send JSON-RPC requests via stdin. Example:
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
//! {"jsonrpc":"2.0","id":2,"method":"tools/list"}
//! {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello!"}}}
//! ```

use mcp_server::prelude::*;
use serde_json::{json, Value};

/// A simple echo tool that returns the input message
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    /// Tool name as it appears to MCP clients
    fn name(&self) -> &str {
        "echo"
    }

    /// Optional description shown in tool listings
    fn description(&self) -> Option<&str> {
        Some("Echoes the input message back to the caller")
    }

    /// JSON Schema defining the expected input format
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    /// Execute the tool with the provided input
    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        // Extract the message from input
        let message = input["message"]
            .as_str()
            .ok_or_else(|| ToolError::ExecutionFailed("message is required".to_string()))?;

        // Return the message as a text result
        Ok(ToolResult::success_text(message))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging (optional)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Build the MCP server with a single tool
    let server = McpServer::builder()
        .name("simple-echo-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build();

    tracing::info!("Starting simple MCP server on stdio...");
    tracing::info!("Server: {}, Version: {}", server.config().name(), server.config().version());

    // Create stdio transport (reads from stdin, writes to stdout)
    let transport = StdioTransport::new();

    // Serve forever (until stdin is closed)
    server.serve(transport).await?;

    tracing::info!("Server shutdown");
    Ok(())
}
