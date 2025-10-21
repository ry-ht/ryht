//! Full-Featured MCP Server Example
//!
//! This example demonstrates all major features of the MCP server framework:
//!
//! # Features Demonstrated
//!
//! - **Multiple Tools**: Echo, Add, and Multiply tools
//! - **Resources**: Static configuration and dynamic data resources
//! - **Middleware**: Logging and metrics collection
//! - **Hooks**: Custom event tracking for auditing
//! - **Stdio Transport**: Standard input/output communication
//! - **Comprehensive Error Handling**: Proper error types and conversions
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example full_featured
//! ```
//!
//! # Example Requests
//!
//! Initialize:
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
//! ```
//!
//! List tools:
//! ```json
//! {"jsonrpc":"2.0","id":2,"method":"tools/list"}
//! ```
//!
//! Call echo tool:
//! ```json
//! {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello!"}}}
//! ```
//!
//! Call add tool:
//! ```json
//! {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"add","arguments":{"a":5,"b":3}}}
//! ```
//!
//! List resources:
//! ```json
//! {"jsonrpc":"2.0","id":5,"method":"resources/list"}
//! ```
//!
//! Read config resource:
//! ```json
//! {"jsonrpc":"2.0","id":6,"method":"resources/read","params":{"uri":"app://config"}}
//! ```

use mcp_server::prelude::*;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
        Some("Echoes the input message back to the caller")
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

/// Add tool - adds two numbers
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds two numbers together and returns the result")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let a = input["a"]
            .as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("'a' must be a number".to_string()))?;
        let b = input["b"]
            .as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("'b' must be a number".to_string()))?;

        let result = a + b;

        Ok(ToolResult::success_json(json!({
            "operation": "addition",
            "a": a,
            "b": b,
            "result": result
        })))
    }
}

/// Multiply tool - multiplies two numbers
struct MultiplyTool;

#[async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &str {
        "multiply"
    }

    fn description(&self) -> Option<&str> {
        Some("Multiplies two numbers and returns the result")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "x": {
                    "type": "number",
                    "description": "First number"
                },
                "y": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["x", "y"]
        })
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> Result<ToolResult, ToolError> {
        let x = input["x"]
            .as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("'x' must be a number".to_string()))?;
        let y = input["y"]
            .as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("'y' must be a number".to_string()))?;

        let result = x * y;

        Ok(ToolResult::success_json(json!({
            "operation": "multiplication",
            "x": x,
            "y": y,
            "result": result
        })))
    }
}

// =============================================================================
// Resources
// =============================================================================

/// Static configuration resource
struct ConfigResource {
    config: Value,
}

#[async_trait]
impl Resource for ConfigResource {
    fn uri(&self) -> &str {
        "app://config"
    }

    fn name(&self) -> Option<&str> {
        Some("Application Configuration")
    }

    fn description(&self) -> Option<&str> {
        Some("Static application configuration in JSON format")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
        Ok(ResourceContent::text(
            serde_json::to_string_pretty(&self.config).unwrap(),
            "application/json",
        ))
    }
}

/// Dynamic data resource - returns current server statistics
struct StatsResource {
    start_time: Instant,
}

#[async_trait]
impl Resource for StatsResource {
    fn uri(&self) -> &str {
        "app://stats"
    }

    fn name(&self) -> Option<&str> {
        Some("Server Statistics")
    }

    fn description(&self) -> Option<&str> {
        Some("Dynamic server statistics including uptime and metrics")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
        let uptime_secs = self.start_time.elapsed().as_secs();

        let stats = json!({
            "uptime_seconds": uptime_secs,
            "uptime_formatted": format!("{}h {}m {}s",
                uptime_secs / 3600,
                (uptime_secs % 3600) / 60,
                uptime_secs % 60
            ),
            "status": "running"
        });

        Ok(ResourceContent::text(
            serde_json::to_string_pretty(&stats).unwrap(),
            "application/json",
        ))
    }
}

// =============================================================================
// Middleware
// =============================================================================

/// Logging middleware - logs all requests and responses
struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        tracing::info!(
            "→ Incoming request: method={}, id={:?}",
            request.method,
            request.id
        );
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        _context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if response.is_error() {
            tracing::warn!(
                "← Error response: id={:?}, error={:?}",
                response.id,
                response.error
            );
        } else {
            tracing::info!("← Success response: id={:?}", response.id);
        }
        Ok(())
    }
}

/// Metrics middleware - tracks request counts and timing
struct MetricsMiddleware {
    total_requests: Arc<Mutex<u64>>,
    successful_requests: Arc<Mutex<u64>>,
    failed_requests: Arc<Mutex<u64>>,
}

impl MetricsMiddleware {
    fn new() -> Self {
        Self {
            total_requests: Arc::new(Mutex::new(0)),
            successful_requests: Arc::new(Mutex::new(0)),
            failed_requests: Arc::new(Mutex::new(0)),
        }
    }

    fn print_stats(&self) {
        let total = *self.total_requests.lock().unwrap();
        let successful = *self.successful_requests.lock().unwrap();
        let failed = *self.failed_requests.lock().unwrap();

        tracing::info!(
            "📊 Metrics: total={}, successful={}, failed={}",
            total,
            successful,
            failed
        );
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn on_request(
        &self,
        _request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        *self.total_requests.lock().unwrap() += 1;
        context.set_metadata("start_time", json!(Instant::now().elapsed().as_micros()));
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if response.is_error() {
            *self.failed_requests.lock().unwrap() += 1;
        } else {
            *self.successful_requests.lock().unwrap() += 1;
        }

        if let Some(start_time) = context.get_metadata("start_time") {
            let elapsed = Instant::now().elapsed().as_micros() - start_time.as_u64().unwrap();
            tracing::debug!("⏱️  Request took {}μs", elapsed);
        }

        Ok(())
    }
}

// =============================================================================
// Hooks
// =============================================================================

/// Audit hook - logs all tool calls and resource accesses for auditing
struct AuditHook;

#[async_trait]
impl Hook for AuditHook {
    async fn on_event(&self, event: HookEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            HookEvent::ServerStarted => {
                tracing::info!("🚀 Server started");
            }
            HookEvent::ServerStopped => {
                tracing::info!("🛑 Server stopped");
            }
            HookEvent::ToolCalled { name, args } => {
                tracing::info!("🔧 Tool called: {} with args: {:?}", name, args);
            }
            HookEvent::ToolCompleted { name, result } => {
                match result {
                    Ok(_) => tracing::info!("✅ Tool completed successfully: {}", name),
                    Err(e) => tracing::warn!("❌ Tool failed: {} - {}", name, e),
                }
            }
            HookEvent::ResourceRead { uri } => {
                tracing::info!("📖 Resource read: {}", uri);
            }
            HookEvent::Error { error } => {
                tracing::error!("💥 Error occurred: {}", error);
            }
        }
        Ok(())
    }
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with pretty output
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Create application configuration
    let config = json!({
        "version": "1.0.0",
        "name": "Full-Featured MCP Server",
        "features": ["tools", "resources", "middleware", "hooks"],
        "max_concurrent_requests": 100,
        "timeout_seconds": 30
    });

    // Create metrics middleware
    let metrics = Arc::new(MetricsMiddleware::new());
    let metrics_clone = Arc::clone(&metrics);

    // Build the server with all features
    let server = McpServer::builder()
        .name("full-featured-server")
        .version("1.0.0")
        .protocol_version("2025-03-26")
        // Register tools
        .tool(EchoTool)
        .tool(AddTool)
        .tool(MultiplyTool)
        // Register resources
        .resource(ConfigResource { config })
        .resource(StatsResource {
            start_time: Instant::now(),
        })
        // Add middleware
        .middleware(LoggingMiddleware)
        .middleware((*metrics).clone())
        // Add hooks
        .hook(AuditHook)
        .build();

    // Print server information
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         Full-Featured MCP Server Example              ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ Server:           {}                    ║", server.config().name());
    println!("║ Version:          {}                           ║", server.config().version());
    println!("║ Protocol Version: {}                      ║", server.config().protocol_version());
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ Tools:            3 (echo, add, multiply)             ║");
    println!("║ Resources:        2 (app://config, app://stats)       ║");
    println!("║ Middleware:       2 (logging, metrics)                ║");
    println!("║ Hooks:            1 (audit)                            ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    println!("Server is now listening on stdio...");
    println!("Send JSON-RPC requests via stdin to interact with the server.");
    println!();

    // Spawn a background task to print metrics every 30 seconds
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            metrics_clone.print_stats();
        }
    });

    // Create stdio transport
    let transport = StdioTransport::new();

    // Serve forever
    server.serve(transport).await?;

    tracing::info!("Server shutdown gracefully");
    Ok(())
}
