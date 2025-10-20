# MCP Server Framework - Examples Guide

**Version:** 0.1.0
**Last Updated:** 2025-10-20

This guide provides detailed, step-by-step examples for building MCP servers with increasing complexity.

---

## Table of Contents

1. [Getting Started Examples](#1-getting-started-examples)
2. [Tool Examples](#2-tool-examples)
3. [Resource Examples](#3-resource-examples)
4. [Transport Examples](#4-transport-examples)
5. [Middleware Examples](#5-middleware-examples)
6. [Complete Applications](#6-complete-applications)

---

## 1. Getting Started Examples

### 1.1 Minimal Echo Server

The simplest possible MCP server with one tool.

```rust
use mcp_server::prelude::*;

// Step 1: Define a tool struct
struct EchoTool;

// Step 2: Implement the Tool trait
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

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let message = input["message"]
            .as_str()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "message is required".to_string()
            ))?;

        Ok(ToolResult::success_text(message))
    }
}

// Step 3: Build and run server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    McpServer::builder()
        .name("echo-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

**How to run:**

```bash
cargo run
```

**How to test:**

```bash
# Send a JSON-RPC request via stdin
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello"}}}' | cargo run
```

---

### 1.2 Calculator Server

A server with multiple related tools.

```rust
use mcp_server::prelude::*;

// Addition tool
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds two numbers")
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

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let a = input["a"].as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("a must be a number".into()))?;
        let b = input["b"].as_f64()
            .ok_or_else(|| ToolError::ExecutionFailed("b must be a number".into()))?;

        Ok(ToolResult::success_json(json!({
            "result": a + b
        })))
    }
}

// Subtraction tool
struct SubtractTool;

#[async_trait]
impl Tool for SubtractTool {
    fn name(&self) -> &str {
        "subtract"
    }

    fn description(&self) -> Option<&str> {
        Some("Subtracts two numbers")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let a = input["a"].as_f64().unwrap();
        let b = input["b"].as_f64().unwrap();

        Ok(ToolResult::success_json(json!({
            "result": a - b
        })))
    }
}

// Multiply tool
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
                "a": { "type": "number" },
                "b": { "type": "number" }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let a = input["a"].as_f64().unwrap();
        let b = input["b"].as_f64().unwrap();

        Ok(ToolResult::success_json(json!({
            "result": a * b
        })))
    }
}

// Build server with all tools
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    McpServer::builder()
        .name("calculator-server")
        .version("1.0.0")
        .tool(AddTool)
        .tool(SubtractTool)
        .tool(MultiplyTool)
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

## 2. Tool Examples

### 2.1 Tool with Shared State

A tool that accesses a shared database.

```rust
use mcp_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

// Simple in-memory database
#[derive(Clone, serde::Serialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

struct Database {
    users: Mutex<Vec<User>>,
}

impl Database {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            users: Mutex::new(vec![
                User {
                    id: 1,
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
            ]),
        })
    }

    async fn get_user(&self, id: u64) -> Option<User> {
        self.users.lock().await
            .iter()
            .find(|u| u.id == id)
            .cloned()
    }

    async fn list_users(&self) -> Vec<User> {
        self.users.lock().await.clone()
    }
}

// Tool that uses shared database
struct GetUserTool {
    db: Arc<Database>,
}

#[async_trait]
impl Tool for GetUserTool {
    fn name(&self) -> &str {
        "get_user"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieves a user by ID")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "integer",
                    "description": "User ID",
                    "minimum": 1
                }
            },
            "required": ["id"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let id = input["id"].as_u64()
            .ok_or_else(|| ToolError::ExecutionFailed("id must be a positive integer".into()))?;

        match self.db.get_user(id).await {
            Some(user) => Ok(ToolResult::success_json(json!(user))),
            None => Err(ToolError::ExecutionFailed(format!("User {} not found", id))),
        }
    }
}

struct ListUsersTool {
    db: Arc<Database>,
}

#[async_trait]
impl Tool for ListUsersTool {
    fn name(&self) -> &str {
        "list_users"
    }

    fn description(&self) -> Option<&str> {
        Some("Lists all users")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        _input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let users = self.db.list_users().await;
        Ok(ToolResult::success_json(json!({ "users": users })))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create shared database
    let db = Database::new();

    // Build server with tools that share the database
    McpServer::builder()
        .name("user-service")
        .version("1.0.0")
        .tool(GetUserTool { db: db.clone() })
        .tool(ListUsersTool { db: db.clone() })
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

### 2.2 Tool with External API Call

A tool that makes HTTP requests to an external API.

```rust
use mcp_server::prelude::*;
use std::sync::Arc;

struct WeatherTool {
    client: Arc<reqwest::Client>,
    api_key: String,
}

impl WeatherTool {
    fn new(api_key: String) -> Self {
        Self {
            client: Arc::new(reqwest::Client::new()),
            api_key,
        }
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets current weather for a city")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["city"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let city = input["city"].as_str()
            .ok_or_else(|| ToolError::ExecutionFailed("city is required".into()))?;

        tracing::info!("Fetching weather for: {}", city);

        // Make API request
        let url = format!(
            "https://api.weatherapi.com/v1/current.json?key={}&q={}",
            self.api_key, city
        );

        let response = self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed(
                format!("API returned error: {}", response.status())
            ));
        }

        let weather: Value = response.json().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        Ok(ToolResult::success_json(json!({
            "city": city,
            "temperature": weather["current"]["temp_c"],
            "condition": weather["current"]["condition"]["text"],
            "humidity": weather["current"]["humidity"]
        })))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Get API key from environment
    let api_key = std::env::var("WEATHER_API_KEY")
        .expect("WEATHER_API_KEY environment variable not set");

    McpServer::builder()
        .name("weather-service")
        .version("1.0.0")
        .tool(WeatherTool::new(api_key))
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

### 2.3 Tool with Timeout

A tool that implements timeout protection.

```rust
use mcp_server::prelude::*;
use tokio::time::{timeout, Duration};

struct SlowTool;

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str {
        "slow_operation"
    }

    fn description(&self) -> Option<&str> {
        Some("A slow operation with timeout protection")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "duration_seconds": {
                    "type": "integer",
                    "description": "How long the operation takes",
                    "minimum": 1,
                    "maximum": 60
                }
            },
            "required": ["duration_seconds"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let duration_secs = input["duration_seconds"].as_u64().unwrap_or(5);

        // Define the operation
        let operation = async {
            tokio::time::sleep(Duration::from_secs(duration_secs)).await;
            "Operation completed".to_string()
        };

        // Apply timeout (30 seconds)
        let result = timeout(Duration::from_secs(30), operation)
            .await
            .map_err(|_| ToolError::Timeout(Duration::from_secs(30)))?;

        Ok(ToolResult::success_text(result))
    }
}
```

---

## 3. Resource Examples

### 3.1 Static Configuration Resource

A resource that provides static configuration.

```rust
use mcp_server::prelude::*;

#[derive(Clone, serde::Serialize)]
struct AppConfig {
    app_name: String,
    version: String,
    max_connections: u32,
    timeout_seconds: u32,
}

struct ConfigResource {
    config: AppConfig,
}

impl ConfigResource {
    fn new() -> Self {
        Self {
            config: AppConfig {
                app_name: "My Application".to_string(),
                version: "1.0.0".to_string(),
                max_connections: 100,
                timeout_seconds: 30,
            },
        }
    }
}

#[async_trait]
impl Resource for ConfigResource {
    fn uri_pattern(&self) -> &str {
        "app://config"
    }

    fn name(&self) -> Option<&str> {
        Some("Application Configuration")
    }

    fn description(&self) -> Option<&str> {
        Some("Main application configuration")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    async fn read(
        &self,
        uri: &str,
        _context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError> {
        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string_pretty(&self.config)
                .map_err(|e| ResourceError::ReadFailed(e.to_string()))?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    McpServer::builder()
        .name("config-server")
        .version("1.0.0")
        .resource(ConfigResource::new())
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

### 3.2 Dynamic File System Resource

A resource that reads files from the filesystem.

```rust
use mcp_server::prelude::*;
use std::path::PathBuf;

struct FileSystemResource {
    root: PathBuf,
}

impl FileSystemResource {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

#[async_trait]
impl Resource for FileSystemResource {
    fn uri_pattern(&self) -> &str {
        "file:///*"
    }

    fn name(&self) -> Option<&str> {
        Some("File System")
    }

    fn description(&self) -> Option<&str> {
        Some("Access to files in the allowed directory")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }

    async fn read(
        &self,
        uri: &str,
        _context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError> {
        // Extract path from URI
        let path = uri.strip_prefix("file:///")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        // Build full path
        let full_path = self.root.join(path);

        // Security: Ensure path is within root directory
        let canonical = full_path.canonicalize()
            .map_err(|e| ResourceError::ReadFailed(e.to_string()))?;

        if !canonical.starts_with(&self.root) {
            return Err(ResourceError::InvalidUri(
                "Path outside allowed directory".to_string()
            ));
        }

        // Read file
        let content = tokio::fs::read_to_string(&canonical).await
            .map_err(|e| ResourceError::ReadFailed(e.to_string()))?;

        // Determine MIME type from extension
        let mime_type = match full_path.extension().and_then(|s| s.to_str()) {
            Some("txt") => "text/plain",
            Some("json") => "application/json",
            Some("md") => "text/markdown",
            Some("html") => "text/html",
            _ => "application/octet-stream",
        };

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some(mime_type.to_string()),
            text: content,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;

    McpServer::builder()
        .name("file-server")
        .version("1.0.0")
        .resource(FileSystemResource::new(root))
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

### 3.3 Database-Backed Resource

A resource that reads data from a database.

```rust
use mcp_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, serde::Serialize)]
struct Document {
    id: String,
    title: String,
    content: String,
}

struct Database {
    documents: Mutex<Vec<Document>>,
}

impl Database {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            documents: Mutex::new(vec![
                Document {
                    id: "doc1".to_string(),
                    title: "Getting Started".to_string(),
                    content: "Welcome to our application...".to_string(),
                },
                Document {
                    id: "doc2".to_string(),
                    title: "API Reference".to_string(),
                    content: "API documentation...".to_string(),
                },
            ]),
        })
    }

    async fn get_document(&self, id: &str) -> Option<Document> {
        self.documents.lock().await
            .iter()
            .find(|d| d.id == id)
            .cloned()
    }
}

struct DocumentResource {
    db: Arc<Database>,
}

#[async_trait]
impl Resource for DocumentResource {
    fn uri_pattern(&self) -> &str {
        "doc://*"
    }

    fn name(&self) -> Option<&str> {
        Some("Documentation")
    }

    fn description(&self) -> Option<&str> {
        Some("Application documentation")
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    async fn read(
        &self,
        uri: &str,
        _context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError> {
        // Extract document ID from URI
        let doc_id = uri.strip_prefix("doc://")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        // Fetch from database
        let doc = self.db.get_document(doc_id).await
            .ok_or_else(|| ResourceError::NotFound(uri.to_string()))?;

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string_pretty(&doc)
                .map_err(|e| ResourceError::ReadFailed(e.to_string()))?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::new();

    McpServer::builder()
        .name("doc-server")
        .version("1.0.0")
        .resource(DocumentResource { db })
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

## 4. Transport Examples

### 4.1 HTTP Server

Running the server with HTTP/SSE transport.

```rust
use mcp_server::prelude::*;

// ... (define your tools here)

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build server
    let server = McpServer::builder()
        .name("http-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build()?;

    // Run with HTTP transport
    let addr = "127.0.0.1:3000".parse()?;
    server.serve(HttpTransport::new(addr)).await?;

    Ok(())
}
```

**Test with curl:**

```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{"message":"hello"}}}'
```

---

### 4.2 Custom Channel Transport

Implementing a custom transport using Tokio channels.

```rust
use mcp_server::prelude::*;
use tokio::sync::mpsc;

struct ChannelTransport {
    request_rx: mpsc::Receiver<JsonRpcRequest>,
    response_tx: mpsc::Sender<JsonRpcResponse>,
    closed: bool,
}

impl ChannelTransport {
    fn new(
        request_rx: mpsc::Receiver<JsonRpcRequest>,
        response_tx: mpsc::Sender<JsonRpcResponse>,
    ) -> Self {
        Self {
            request_rx,
            response_tx,
            closed: false,
        }
    }
}

#[async_trait]
impl Transport for ChannelTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        if self.closed {
            return None;
        }
        self.request_rx.recv().await
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
        self.response_tx.send(response).await
            .map_err(|_| TransportError::Closed)?;
        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed = true;
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels
    let (req_tx, req_rx) = mpsc::channel(100);
    let (res_tx, mut res_rx) = mpsc::channel(100);

    // Build server
    let server = McpServer::builder()
        .name("channel-server")
        .tool(EchoTool)
        .build()?;

    // Run server in background
    let server_handle = tokio::spawn(async move {
        server.serve(ChannelTransport::new(req_rx, res_tx)).await
    });

    // Send a request
    req_tx.send(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "echo",
            "arguments": {"message": "Hello"}
        })),
    }).await?;

    // Receive response
    if let Some(response) = res_rx.recv().await {
        println!("Response: {:?}", response);
    }

    server_handle.abort();
    Ok(())
}
```

---

## 5. Middleware Examples

### 5.1 Logging Middleware

Middleware that logs all requests and responses.

```rust
use mcp_server::prelude::*;

struct RequestLogger;

#[async_trait]
impl Middleware for RequestLogger {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        // Log incoming request
        tracing::info!(
            method = %request.method,
            id = ?request.id,
            "Incoming request"
        );

        // Store start time for duration calculation
        context.set_metadata("start_time", json!(std::time::Instant::now()));

        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        // Calculate duration
        if let Some(start_json) = context.get_metadata("start_time") {
            // In real code, you'd properly deserialize the Instant
            let success = response.error.is_none();

            tracing::info!(
                id = ?response.id,
                success = success,
                "Request completed"
            );
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    McpServer::builder()
        .name("logged-server")
        .version("1.0.0")
        .tool(EchoTool)
        .middleware(RequestLogger)
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

### 5.2 Authentication Middleware

Middleware that validates API keys.

```rust
use mcp_server::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

struct AuthMiddleware {
    api_keys: Arc<HashSet<String>>,
}

impl AuthMiddleware {
    fn new(api_keys: Vec<String>) -> Self {
        Self {
            api_keys: Arc::new(api_keys.into_iter().collect()),
        }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        // Skip auth for initialize
        if request.method == "initialize" {
            return Ok(());
        }

        // Extract API key from params
        let api_key = request.params
            .as_ref()
            .and_then(|p| p.get("api_key"))
            .and_then(|k| k.as_str())
            .ok_or_else(|| MiddlewareError::Blocked(
                "API key required".to_string()
            ))?;

        // Validate
        if !self.api_keys.contains(api_key) {
            return Err(MiddlewareError::Blocked(
                "Invalid API key".to_string()
            ));
        }

        context.set_metadata("authenticated", json!(true));
        Ok(())
    }

    async fn on_response(
        &self,
        _response: &JsonRpcResponse,
        _context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_keys = vec![
        "key1".to_string(),
        "key2".to_string(),
    ];

    McpServer::builder()
        .name("authenticated-server")
        .version("1.0.0")
        .tool(EchoTool)
        .middleware(AuthMiddleware::new(api_keys))
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

## 6. Complete Applications

### 6.1 TODO List Manager

A complete application with tools, resources, and middleware.

```rust
use mcp_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

// Domain types
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Todo {
    id: u64,
    title: String,
    description: String,
    completed: bool,
}

// Database
struct TodoDatabase {
    todos: Mutex<Vec<Todo>>,
    next_id: Mutex<u64>,
}

impl TodoDatabase {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            todos: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        })
    }

    async fn create(&self, title: String, description: String) -> Todo {
        let mut todos = self.todos.lock().await;
        let mut next_id = self.next_id.lock().await;

        let todo = Todo {
            id: *next_id,
            title,
            description,
            completed: false,
        };

        *next_id += 1;
        todos.push(todo.clone());
        todo
    }

    async fn list(&self) -> Vec<Todo> {
        self.todos.lock().await.clone()
    }

    async fn get(&self, id: u64) -> Option<Todo> {
        self.todos.lock().await
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    async fn complete(&self, id: u64) -> Result<(), String> {
        let mut todos = self.todos.lock().await;
        todos.iter_mut()
            .find(|t| t.id == id)
            .map(|t| t.completed = true)
            .ok_or_else(|| format!("Todo {} not found", id))
    }
}

// Tools
struct CreateTodoTool {
    db: Arc<TodoDatabase>,
}

#[async_trait]
impl Tool for CreateTodoTool {
    fn name(&self) -> &str { "create_todo" }

    fn description(&self) -> Option<&str> {
        Some("Creates a new TODO item")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["title", "description"]
        })
    }

    async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
        let title = input["title"].as_str().unwrap().to_string();
        let description = input["description"].as_str().unwrap().to_string();

        let todo = self.db.create(title, description).await;
        Ok(ToolResult::success_json(json!(todo)))
    }
}

struct ListTodosTool {
    db: Arc<TodoDatabase>,
}

#[async_trait]
impl Tool for ListTodosTool {
    fn name(&self) -> &str { "list_todos" }

    fn description(&self) -> Option<&str> {
        Some("Lists all TODO items")
    }

    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
        let todos = self.db.list().await;
        Ok(ToolResult::success_json(json!({ "todos": todos })))
    }
}

struct CompleteTodoTool {
    db: Arc<TodoDatabase>,
}

#[async_trait]
impl Tool for CompleteTodoTool {
    fn name(&self) -> &str { "complete_todo" }

    fn description(&self) -> Option<&str> {
        Some("Marks a TODO as completed")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "integer", "minimum": 1 }
            },
            "required": ["id"]
        })
    }

    async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
        let id = input["id"].as_u64().unwrap();

        self.db.complete(id).await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(ToolResult::success_text("TODO marked as completed"))
    }
}

// Resources
struct TodoResource {
    db: Arc<TodoDatabase>,
}

#[async_trait]
impl Resource for TodoResource {
    fn uri_pattern(&self) -> &str {
        "todo://*"
    }

    async fn read(&self, uri: &str, _: &ResourceContext)
        -> Result<ResourceContent, ResourceError>
    {
        let id_str = uri.strip_prefix("todo://")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        let id: u64 = id_str.parse()
            .map_err(|_| ResourceError::InvalidUri(uri.to_string()))?;

        let todo = self.db.get(id).await
            .ok_or_else(|| ResourceError::NotFound(uri.to_string()))?;

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string_pretty(&todo).unwrap(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db = TodoDatabase::new();

    McpServer::builder()
        .name("todo-manager")
        .version("1.0.0")
        .tool(CreateTodoTool { db: db.clone() })
        .tool(ListTodosTool { db: db.clone() })
        .tool(CompleteTodoTool { db: db.clone() })
        .resource(TodoResource { db: db.clone() })
        .middleware(LoggingMiddleware::new())
        .build()?
        .serve(StdioTransport::new())
        .await?;

    Ok(())
}
```

---

**End of Examples Guide**

For more information, see:
- [LLM Guide](LLM_GUIDE.md) - Complete API documentation
- [Architecture](ARCHITECTURE.md) - Technical details
- [Source Code](../src/) - Implementation
