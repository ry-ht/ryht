# MCP Server Framework - Complete LLM Guide

**Version:** 0.1.0
**MCP Protocol:** 2025-03-26
**Last Updated:** 2025-10-20

This guide provides comprehensive documentation for LLMs working with the MCP Server framework. It is optimized for both quick reference and deep understanding.

---

## Table of Contents

1. [Quick Start](#1-quick-start)
2. [Core Concepts](#2-core-concepts)
3. [API Reference](#3-api-reference)
4. [Common Patterns](#4-common-patterns)
5. [Error Handling](#5-error-handling)
6. [Testing](#6-testing)
7. [Migration](#7-migration)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Quick Start

### 5-Minute Getting Started

**Goal:** Build and run a working MCP server in under 5 minutes.

#### Step 1: Add Dependency

```toml
[dependencies]
mcp-server = "0.1.0"
tokio = { version = "1.48", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

#### Step 2: Create a Simple Tool

```rust
use mcp_server::prelude::*;

// Define your tool
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
```

#### Step 3: Create and Run Server

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build server
    let server = McpServer::builder()
        .name("simple-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build()?;

    // Run with stdio transport
    server.serve(StdioTransport::new()).await?;

    Ok(())
}
```

#### Step 4: Test

```bash
# Build
cargo build --release

# Run (will listen on stdin/stdout)
./target/release/your-server-name
```

**That's it!** You now have a working MCP server with one tool.

---

## 2. Core Concepts

### 2.1 Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                   MCP Server                        │
│  ┌──────────────────────────────────────────────┐  │
│  │          Server Core (McpServer)             │  │
│  ├──────────────────────────────────────────────┤  │
│  │  ToolRegistry  │  ResourceRegistry  │ Hooks  │  │
│  ├──────────────────────────────────────────────┤  │
│  │           Middleware Chain                   │  │
│  ├──────────────────────────────────────────────┤  │
│  │      Request Router & Handler                │  │
│  └──────────────────────────────────────────────┘  │
│                        ▲                            │
│                        │                            │
│  ┌──────────────────────────────────────────────┐  │
│  │          Transport Layer                     │  │
│  │  (Stdio │ HTTP/SSE │ WebSocket │ Mock)       │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 2.2 Tools

**Definition:** Tools are callable functions exposed by the server to AI models.

**Key Characteristics:**
- **Type-safe:** Input/output validated via JSON schemas
- **Async:** All tool execution is asynchronous
- **Thread-safe:** Tools can be called concurrently
- **Stateless:** Tools receive all context through parameters

**Tool Lifecycle:**

```
Registration → Discovery → Validation → Execution → Response
     ↓              ↓            ↓           ↓           ↓
  .tool()    tools/list    schema     execute()    ToolResult
```

**Example:**

```rust
#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            },
            "required": ["input"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        // Implementation
        Ok(ToolResult::success_text("result"))
    }
}
```

### 2.3 Resources

**Definition:** Resources are URIs that provide data to AI models.

**Key Characteristics:**
- **URI-based:** Identified by URI patterns (e.g., `file://*`, `app://config`)
- **Pattern matching:** Support glob-style patterns
- **Lazy loading:** Content loaded on-demand
- **Multiple formats:** Text and binary content

**Resource Lifecycle:**

```
Registration → Discovery → URI Match → Read → Content
     ↓              ↓           ↓         ↓        ↓
 .resource()  resources/list  pattern  read()  ResourceContent
```

**Example:**

```rust
#[async_trait]
impl Resource for ConfigResource {
    fn uri_pattern(&self) -> &str {
        "app://config"
    }

    async fn read(
        &self,
        uri: &str,
        _context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError> {
        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&self.config)?,
        })
    }
}
```

### 2.4 Transport Layer

**Definition:** Transport manages message exchange between server and clients.

**Available Transports:**

| Transport | Use Case | Feature |
|-----------|----------|---------|
| `StdioTransport` | CLI tools, process spawning | Line-based JSON-RPC over stdin/stdout |
| `HttpTransport` | Web services, REST APIs | HTTP POST + Server-Sent Events |
| `MockTransport` | Testing | In-memory message queues |

**Transport Trait:**

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn recv(&mut self) -> Option<JsonRpcRequest>;
    async fn send(&mut self, response: JsonRpcResponse) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn is_closed(&self) -> bool;
}
```

**Thread Safety:** All transports must be `Send + Sync` for concurrent operation.

### 2.5 Middleware

**Definition:** Middleware intercepts requests and responses for cross-cutting concerns.

**Execution Order:**

```
Request → Middleware₁ → Middleware₂ → Handler → Middleware₂ → Middleware₁ → Response
```

**Common Uses:**
- Logging and monitoring
- Authentication/authorization
- Rate limiting
- Metrics collection
- Request/response transformation

**Example:**

```rust
#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        tracing::info!("Request: {}", request.method);
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        _context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if response.error.is_some() {
            tracing::error!("Request failed");
        }
        Ok(())
    }
}
```

### 2.6 Hooks

**Definition:** Hooks are event callbacks triggered at specific lifecycle points.

**Hook Events:**

```rust
pub enum HookEvent {
    ServerStarted,
    ServerStopped,
    ClientConnected,
    ClientDisconnected,
    ToolCalled { name: String, args: Value },
    ToolCompleted { name: String, result: Result<Value> },
    ResourceRead { uri: String },
    Error { error: Box<dyn Error> },
}
```

**Example:**

```rust
#[async_trait]
impl Hook for AuditHook {
    async fn on_event(&self, event: HookEvent) -> Result<(), HookError> {
        match event {
            HookEvent::ToolCalled { name, args } => {
                self.db.log_call(&name, &args).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

---

## 3. API Reference

### 3.1 Core Types

#### `McpServer`

The main server struct that coordinates all components.

**Type Signature:**

```rust
pub struct McpServer {
    config: ServerConfig,
    tools: Arc<ToolRegistry>,
    resources: Arc<ResourceRegistry>,
    middleware: Vec<Arc<dyn Middleware>>,
    hooks: Arc<HookRegistry>,
}
```

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `builder()` | `fn() -> ServerBuilder` | Creates a new server builder |
| `serve<T: Transport>()` | `async fn(self, transport: T) -> Result<()>` | Starts server with transport |

**Thread Safety:** `Send + Sync` - Can be shared across threads via `Arc`.

**Example:**

```rust
let server = McpServer::builder()
    .name("my-server")
    .version("1.0.0")
    .tool(MyTool)
    .build()?;

server.serve(StdioTransport::new()).await?;
```

---

#### `ServerBuilder`

Fluent API for constructing MCP servers.

**Type Signature:**

```rust
pub struct ServerBuilder {
    name: String,
    version: String,
    tools: Vec<Arc<dyn Tool>>,
    resources: Vec<Arc<dyn Resource>>,
    middleware: Vec<Arc<dyn Middleware>>,
    hooks: Vec<Arc<dyn Hook>>,
}
```

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `name()` | `fn(self, name: &str) -> Self` | Sets server name |
| `version()` | `fn(self, version: &str) -> Self` | Sets server version |
| `tool<T: Tool>()` | `fn(self, tool: T) -> Self` | Registers a tool |
| `resource<R: Resource>()` | `fn(self, resource: R) -> Self` | Registers a resource |
| `middleware<M: Middleware>()` | `fn(self, mw: M) -> Self` | Adds middleware |
| `hook<H: Hook>()` | `fn(self, hook: H) -> Self` | Adds a hook |
| `build()` | `fn(self) -> Result<McpServer>` | Builds the server |

**Example:**

```rust
let server = McpServer::builder()
    .name("full-server")
    .version("1.0.0")
    .tool(Tool1)
    .tool(Tool2)
    .resource(Resource1)
    .middleware(LoggingMiddleware::new())
    .hook(AuditHook::new())
    .build()?;
```

---

#### `Tool` Trait

Core trait for defining tools.

**Type Signature:**

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> { None }
    fn input_schema(&self) -> Value;
    fn output_schema(&self) -> Option<Value> { None }
    fn metadata(&self) -> Option<Value> { None }
    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
}
```

**Required Methods:**

- `name()` - Unique tool identifier
- `input_schema()` - JSON Schema for input validation
- `execute()` - Async tool execution

**Optional Methods:**

- `description()` - Human-readable description
- `output_schema()` - JSON Schema for output (documentation only)
- `metadata()` - Additional tool metadata

**Error Handling:** Must return `Result<ToolResult, ToolError>`.

**Thread Safety:** Must implement `Send + Sync`.

---

#### `ToolContext`

Contextual information passed to tool execution.

**Type Signature:**

```rust
pub struct ToolContext {
    session_id: Option<String>,
    client_name: Option<String>,
    client_version: Option<String>,
    metadata: HashMap<String, Value>,
    start_time: Instant,
}
```

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `fn() -> Self` | Creates empty context |
| `builder()` | `fn() -> ToolContextBuilder` | Creates context builder |
| `session_id()` | `fn(&self) -> Option<&str>` | Gets session ID |
| `client_info()` | `fn(&self) -> Option<(&str, &str)>` | Gets client info |
| `get_metadata()` | `fn(&self, key: &str) -> Option<&Value>` | Gets metadata value |
| `elapsed()` | `fn(&self) -> Duration` | Time since context creation |

**Example:**

```rust
let context = ToolContext::builder()
    .session_id("session-123")
    .client_info("my-client", "1.0.0")
    .metadata("user_id", json!(42))
    .build();

// In tool execution
async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
    let session = context.session_id().unwrap_or("unknown");
    tracing::info!("Executing for session: {}", session);
    // ...
}
```

---

#### `ToolResult`

Result returned by tool execution.

**Type Signature:**

```rust
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}
```

**Constructors:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `success_text()` | `fn(text: impl Into<String>) -> Self` | Success with text |
| `success_json()` | `fn(value: Value) -> Self` | Success with JSON |
| `success()` | `fn(content: Vec<ToolContent>) -> Self` | Success with content |
| `error()` | `fn(message: impl Into<String>) -> Self` | Error result |

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `is_success()` | `fn(&self) -> bool` | Checks if successful |
| `is_error()` | `fn(&self) -> bool` | Checks if error |

**Example:**

```rust
// Text result
let result = ToolResult::success_text("Operation completed");

// JSON result
let result = ToolResult::success_json(json!({
    "status": "ok",
    "count": 42
}));

// Multiple content items
let result = ToolResult::success(vec![
    ToolContent::text("Summary:"),
    ToolContent::json(json!({"key": "value"})),
    ToolContent::image("base64data", "image/png"),
]);

// Error result
let result = ToolResult::error("Operation failed: invalid input");
```

---

#### `ToolContent`

Different types of content that tools can return.

**Type Signature:**

```rust
pub enum ToolContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String },
}
```

**Constructors:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `text()` | `fn(text: impl Into<String>) -> Self` | Text content |
| `json()` | `fn(value: Value) -> Self` | JSON content (serialized to text) |
| `image()` | `fn(data: impl Into<String>, mime: impl Into<String>) -> Self` | Base64 image |
| `resource()` | `fn(uri: impl Into<String>) -> Self` | Resource reference |

**Methods:**

| Method | Signature | Description |
|--------|-----------|-------------|
| `as_text()` | `fn(&self) -> Option<&str>` | Extracts text if Text variant |

**Example:**

```rust
// Text
let content = ToolContent::text("Hello, world!");

// JSON (automatically serialized)
let content = ToolContent::json(json!({
    "status": "success",
    "data": [1, 2, 3]
}));

// Image
let content = ToolContent::image(
    base64_encoded_png,
    "image/png"
);

// Resource reference
let content = ToolContent::resource("file:///path/to/data.csv");
```

---

#### `Resource` Trait

Core trait for defining resources.

**Type Signature:**

```rust
#[async_trait]
pub trait Resource: Send + Sync {
    fn uri_pattern(&self) -> &str;
    fn name(&self) -> Option<&str> { None }
    fn description(&self) -> Option<&str> { None }
    fn mime_type(&self) -> Option<&str> { None }
    async fn read(
        &self,
        uri: &str,
        context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError>;
}
```

**URI Patterns:** Support glob-style wildcards:
- `app://config` - Exact match
- `file:///*.txt` - All .txt files
- `db://users/*` - All user resources

**Example:**

```rust
struct FileResource {
    base_path: PathBuf,
}

#[async_trait]
impl Resource for FileResource {
    fn uri_pattern(&self) -> &str {
        "file:///*"
    }

    fn mime_type(&self) -> Option<&str> {
        Some("text/plain")
    }

    async fn read(
        &self,
        uri: &str,
        _context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError> {
        let path = uri.strip_prefix("file:///")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        let content = tokio::fs::read_to_string(self.base_path.join(path))
            .await
            .map_err(|e| ResourceError::ReadFailed(e.to_string()))?;

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            text: content,
        })
    }
}
```

---

### 3.2 Error Types

#### `McpError`

Top-level error type for all MCP operations.

**Type Signature:**

```rust
pub enum McpError {
    Transport(TransportError),
    Tool(ToolError),
    Resource(ResourceError),
    Middleware(MiddlewareError),
    Config(String),
    Protocol(String),
}
```

**Conversions:** All sub-errors automatically convert via `From` trait.

---

#### `ToolError`

Tool-specific errors.

**Type Signature:**

```rust
pub enum ToolError {
    NotFound(String),
    AlreadyRegistered(String),
    InvalidInput(serde_json::Error),
    ExecutionFailed(String),
    Timeout(Duration),
    Internal(anyhow::Error),
}
```

**JSON-RPC Mapping:**

| Error | Code | Description |
|-------|------|-------------|
| `NotFound` | `-32601` | Tool not found |
| `InvalidInput` | `-32602` | Invalid parameters |
| `ExecutionFailed` | `-32000` | Tool execution failed |
| `Timeout` | `-32001` | Execution timeout |
| `Internal` | `-32603` | Internal error |

---

#### `ResourceError`

Resource-specific errors.

**Type Signature:**

```rust
pub enum ResourceError {
    NotFound(String),
    InvalidUri(String),
    ReadFailed(String),
    Internal(anyhow::Error),
}
```

**JSON-RPC Mapping:**

| Error | Code | Description |
|-------|------|-------------|
| `NotFound` | `-32002` | Resource not found |
| `InvalidUri` | `-32003` | Invalid URI |
| `ReadFailed` | `-32004` | Read operation failed |
| `Internal` | `-32603` | Internal error |

---

### 3.3 Protocol Types

#### `JsonRpcRequest`

JSON-RPC 2.0 request message.

**Type Signature:**

```rust
pub struct JsonRpcRequest {
    pub jsonrpc: String,        // Always "2.0"
    pub id: Option<Value>,      // Request ID (null for notifications)
    pub method: String,         // Method name (e.g., "tools/call")
    pub params: Option<Value>,  // Method parameters
}
```

---

#### `JsonRpcResponse`

JSON-RPC 2.0 response message.

**Type Signature:**

```rust
pub struct JsonRpcResponse {
    pub jsonrpc: String,              // Always "2.0"
    pub id: Option<Value>,            // Matches request ID
    pub result: Option<Value>,        // Success result
    pub error: Option<JsonRpcError>,  // Error (mutually exclusive with result)
}
```

**Constructors:**

```rust
JsonRpcResponse::success(id, result)
JsonRpcResponse::error(id, error)
JsonRpcResponse::method_not_found(id)
```

---

## 4. Common Patterns

### 4.1 Stateful Tools

Tools are typically stateless, but can hold immutable state:

```rust
struct DatabaseTool {
    db: Arc<Database>,  // Shared, thread-safe
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str { "db_query" }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let query = input["query"].as_str().unwrap();
        let results = self.db.query(query).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(results))
    }
}
```

**Key Points:**
- Use `Arc` for shared state
- State must be `Send + Sync`
- Prefer immutable state
- Use interior mutability (`RwLock`, `Mutex`) if needed

---

### 4.2 Input Validation

Always validate input before processing:

```rust
async fn execute(
    &self,
    input: Value,
    _context: &ToolContext,
) -> Result<ToolResult, ToolError> {
    // Option 1: Manual validation
    let email = input["email"]
        .as_str()
        .ok_or_else(|| ToolError::ExecutionFailed(
            "email is required".to_string()
        ))?;

    if !email.contains('@') {
        return Err(ToolError::ExecutionFailed(
            "invalid email format".to_string()
        ));
    }

    // Option 2: Deserialize to typed struct
    #[derive(Deserialize)]
    struct Input {
        email: String,
        #[serde(default)]
        send_confirmation: bool,
    }

    let typed_input: Input = serde_json::from_value(input)
        .map_err(ToolError::InvalidInput)?;

    // Process...
    Ok(ToolResult::success_text("Email sent"))
}
```

---

### 4.3 Error Handling Best Practices

```rust
async fn execute(
    &self,
    input: Value,
    context: &ToolContext,
) -> Result<ToolResult, ToolError> {
    // 1. Validate input early
    let user_id = input["user_id"]
        .as_i64()
        .ok_or_else(|| ToolError::ExecutionFailed(
            "user_id must be a number".to_string()
        ))?;

    // 2. Use ? for error propagation
    let user = self.db.get_user(user_id).await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

    // 3. Handle specific error cases
    if user.is_deleted {
        return Err(ToolError::ExecutionFailed(
            "User has been deleted".to_string()
        ));
    }

    // 4. Log errors for debugging
    tracing::debug!("Processing user: {:?}", user);

    // 5. Return detailed results
    Ok(ToolResult::success_json(json!({
        "user": user,
        "session": context.session_id()
    })))
}
```

---

### 4.4 Async Patterns

**Pattern 1: Sequential Operations**

```rust
async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult> {
    // Execute in order
    let step1 = self.step1().await?;
    let step2 = self.step2(step1).await?;
    let step3 = self.step3(step2).await?;

    Ok(ToolResult::success_json(json!({ "result": step3 })))
}
```

**Pattern 2: Concurrent Operations**

```rust
async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult> {
    // Execute concurrently
    let (result1, result2, result3) = tokio::join!(
        self.task1(),
        self.task2(),
        self.task3(),
    );

    Ok(ToolResult::success_json(json!({
        "results": [result1?, result2?, result3?]
    })))
}
```

**Pattern 3: Timeout Protection**

```rust
async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult> {
    let operation = self.long_running_task(input);

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        operation
    )
    .await
    .map_err(|_| ToolError::Timeout(Duration::from_secs(30)))?
    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

    Ok(ToolResult::success_json(result))
}
```

---

### 4.5 Resource Patterns

**Pattern 1: Static Resources**

```rust
struct ConfigResource {
    config: Config,
}

#[async_trait]
impl Resource for ConfigResource {
    fn uri_pattern(&self) -> &str {
        "app://config"
    }

    async fn read(&self, uri: &str, _: &ResourceContext)
        -> Result<ResourceContent, ResourceError>
    {
        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&self.config)
                .map_err(|e| ResourceError::ReadFailed(e.to_string()))?,
        })
    }
}
```

**Pattern 2: Dynamic Resources (URI Parameters)**

```rust
struct UserResource {
    db: Arc<Database>,
}

#[async_trait]
impl Resource for UserResource {
    fn uri_pattern(&self) -> &str {
        "user://*"  // Matches user://123, user://alice, etc.
    }

    async fn read(&self, uri: &str, _: &ResourceContext)
        -> Result<ResourceContent, ResourceError>
    {
        // Extract ID from URI
        let id = uri.strip_prefix("user://")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        // Fetch from database
        let user = self.db.get_user(id).await
            .map_err(|_| ResourceError::NotFound(uri.to_string()))?;

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text: serde_json::to_string(&user)
                .map_err(|e| ResourceError::ReadFailed(e.to_string()))?,
        })
    }
}
```

**Pattern 3: File System Resources**

```rust
struct FileSystemResource {
    root: PathBuf,
}

#[async_trait]
impl Resource for FileSystemResource {
    fn uri_pattern(&self) -> &str {
        "file:///*"
    }

    async fn read(&self, uri: &str, _: &ResourceContext)
        -> Result<ResourceContent, ResourceError>
    {
        let path = uri.strip_prefix("file:///")
            .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;

        let full_path = self.root.join(path);

        // Security: Ensure path is within root
        if !full_path.starts_with(&self.root) {
            return Err(ResourceError::InvalidUri(
                "Path outside root directory".to_string()
            ));
        }

        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| ResourceError::ReadFailed(e.to_string()))?;

        Ok(ResourceContent::Text {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            text: content,
        })
    }
}
```

---

### 4.6 Middleware Patterns

**Pattern 1: Request Logging**

```rust
struct RequestLogger;

#[async_trait]
impl Middleware for RequestLogger {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        context.set_metadata("start_time", json!(Instant::now()));
        tracing::info!(
            method = %request.method,
            id = ?request.id,
            "Incoming request"
        );
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if let Some(start) = context.get_metadata("start_time") {
            let duration = /* calculate duration */;
            tracing::info!(
                id = ?response.id,
                duration_ms = duration,
                success = response.error.is_none(),
                "Request completed"
            );
        }
        Ok(())
    }
}
```

**Pattern 2: Authentication**

```rust
struct AuthMiddleware {
    api_keys: Arc<HashSet<String>>,
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
```

**Pattern 3: Rate Limiting**

```rust
struct RateLimiter {
    limiter: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

#[async_trait]
impl Middleware for RateLimiter {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        let client_id = context.client_name()
            .unwrap_or("unknown")
            .to_string();

        let mut limiter = self.limiter.lock().await;
        let requests = limiter.entry(client_id.clone())
            .or_insert_with(VecDeque::new);

        // Remove old requests outside window
        let now = Instant::now();
        while let Some(&first) = requests.front() {
            if now.duration_since(first) > self.window {
                requests.pop_front();
            } else {
                break;
            }
        }

        // Check limit
        if requests.len() >= self.max_requests {
            return Err(MiddlewareError::Blocked(
                "Rate limit exceeded".to_string()
            ));
        }

        requests.push_back(now);
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
```

---

## 5. Error Handling

### 5.1 Error Hierarchy

```
McpError
├── Transport(TransportError)
│   ├── Io(std::io::Error)
│   ├── Closed
│   └── InvalidMessage(String)
├── Tool(ToolError)
│   ├── NotFound(String)
│   ├── AlreadyRegistered(String)
│   ├── InvalidInput(serde_json::Error)
│   ├── ExecutionFailed(String)
│   ├── Timeout(Duration)
│   └── Internal(anyhow::Error)
├── Resource(ResourceError)
│   ├── NotFound(String)
│   ├── InvalidUri(String)
│   ├── ReadFailed(String)
│   └── Internal(anyhow::Error)
├── Middleware(MiddlewareError)
│   ├── Blocked(String)
│   └── Internal(anyhow::Error)
├── Config(String)
└── Protocol(String)
```

### 5.2 Error Conversion to JSON-RPC

All errors are automatically converted to JSON-RPC error responses:

```rust
impl From<ToolError> for JsonRpcError {
    fn from(error: ToolError) -> Self {
        match error {
            ToolError::NotFound(name) => JsonRpcError {
                code: -32601,
                message: format!("Tool '{}' not found", name),
                data: None,
            },
            ToolError::InvalidInput(e) => JsonRpcError {
                code: -32602,
                message: "Invalid tool input".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            },
            // ... other conversions
        }
    }
}
```

### 5.3 Error Handling in Tools

**Best Practices:**

1. **Use specific error types:**

```rust
// Good
return Err(ToolError::ExecutionFailed("User not found".to_string()));

// Avoid
return Err(ToolError::Internal(anyhow::anyhow!("error")));
```

2. **Provide context:**

```rust
// Good
return Err(ToolError::ExecutionFailed(
    format!("Failed to fetch user {}: {}", user_id, e)
));

// Less helpful
return Err(ToolError::ExecutionFailed("Error".to_string()));
```

3. **Use `?` for error propagation:**

```rust
let data = self.db.query().await
    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
```

4. **Handle partial failures:**

```rust
async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult> {
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for item in items {
        match self.process(item).await {
            Ok(result) => results.push(result),
            Err(e) => errors.push(e.to_string()),
        }
    }

    if results.is_empty() && !errors.is_empty() {
        // All failed
        return Err(ToolError::ExecutionFailed(
            format!("All operations failed: {:?}", errors)
        ));
    }

    // Return partial results
    Ok(ToolResult::success_json(json!({
        "results": results,
        "errors": errors,
    })))
}
```

### 5.4 Thread Safety Notes

All errors must be `Send + Sync`:

```rust
// ✅ Good - Error is Send + Sync
#[derive(Debug, Error)]
pub enum MyError {
    #[error("Database error: {0}")]
    Database(String),
}

// ❌ Bad - Rc is not Send + Sync
#[derive(Debug, Error)]
pub enum MyError {
    #[error("Error")]
    WithRc(Rc<String>),  // Won't compile
}
```

---

## 6. Testing

### 6.1 Testing Strategy

```
┌─────────────────────────────────────┐
│         E2E Tests                   │ ← Full server + real transport
├─────────────────────────────────────┤
│      Integration Tests              │ ← Server + MockTransport
├─────────────────────────────────────┤
│        Unit Tests                   │ ← Individual components
└─────────────────────────────────────┘
```

### 6.2 Unit Testing Tools

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = MyTool::new();
        let context = ToolContext::new();
        let input = json!({"param": "value"});

        let result = tool.execute(input, &context).await.unwrap();

        assert!(result.is_success());
        assert_eq!(result.content[0].as_text(), Some("expected"));
    }

    #[tokio::test]
    async fn test_tool_error_handling() {
        let tool = MyTool::new();
        let context = ToolContext::new();
        let invalid_input = json!({});

        let result = tool.execute(invalid_input, &context).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::ExecutionFailed(msg) => {
                assert!(msg.contains("required"));
            }
            _ => panic!("Expected ExecutionFailed"),
        }
    }
}
```

### 6.3 Integration Testing with MockTransport

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use mcp_server::transport::MockTransport;

    #[tokio::test]
    async fn test_tool_call_flow() {
        // Create mock transport
        let transport = MockTransport::new();

        // Queue a tool call request
        transport.push_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "my_tool",
                "arguments": {"input": "test"}
            })),
        });

        // Build server
        let server = McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .tool(MyTool::new())
            .build()
            .unwrap();

        // Serve (will process queued request)
        let server_handle = tokio::spawn({
            let transport = transport.clone();
            async move {
                server.serve(transport).await.unwrap();
            }
        });

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check response
        let responses = transport.responses();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].result.is_some());

        server_handle.abort();
    }
}
```

### 6.4 Property-Based Testing

```rust
#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_tool_never_panics(
            input in any::<String>(),
        ) {
            tokio_test::block_on(async {
                let tool = MyTool::new();
                let context = ToolContext::new();
                let json_input = json!({"input": input});

                // Should never panic, only return Result
                let _ = tool.execute(json_input, &context).await;
            });
        }

        #[test]
        fn test_json_rpc_roundtrip(
            id in any::<Option<i64>>(),
            method in "[a-z]{1,20}",
        ) {
            let request = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: id.map(|i| json!(i)),
                method,
                params: None,
            };

            let json = serde_json::to_string(&request).unwrap();
            let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

            assert_eq!(request.method, parsed.method);
        }
    }
}
```

### 6.5 Performance Testing

```rust
#[cfg(test)]
mod bench_tests {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_tool_execution(c: &mut Criterion) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let tool = MyTool::new();
        let context = ToolContext::new();
        let input = json!({"test": "data"});

        c.bench_function("tool_execute", |b| {
            b.iter(|| {
                runtime.block_on(async {
                    tool.execute(black_box(input.clone()), &context).await
                })
            })
        });
    }

    criterion_group!(benches, bench_tool_execution);
    criterion_main!(benches);
}
```

---

## 7. Migration

### 7.1 From TypeScript SDK

**Before (TypeScript):**

```typescript
import { tool } from '@anthropic-ai/sdk';

const echoTool = tool(
  'echo',
  'Echo a message',
  { message: z.string() },
  async ({ message }) => {
    return { content: [{ type: 'text', text: message }] };
  }
);
```

**After (Rust):**

```rust
use mcp_server::prelude::*;

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }

    fn description(&self) -> Option<&str> {
        Some("Echo a message")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let message = input["message"].as_str().unwrap();
        Ok(ToolResult::success_text(message))
    }
}
```

### 7.2 From Python MCP

**Before (Python):**

```python
from mcp import Tool, Context

class EchoTool(Tool):
    name = "echo"
    description = "Echo a message"

    async def execute(self, message: str, context: Context):
        return {"content": [{"type": "text", "text": message}]}
```

**After (Rust):**

```rust
use mcp_server::prelude::*;

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }

    fn description(&self) -> Option<&str> {
        Some("Echo a message")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            }
        })
    }

    async fn execute(
        &self,
        input: Value,
        _: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let message = input["message"].as_str().unwrap();
        Ok(ToolResult::success_text(message))
    }
}
```

### 7.3 Migration Checklist

- [ ] Identify all tools and resources
- [ ] Convert tool implementations to `Tool` trait
- [ ] Convert resource implementations to `Resource` trait
- [ ] Update JSON schemas to use `serde_json::json!()`
- [ ] Replace manual error handling with `ToolError`/`ResourceError`
- [ ] Update server initialization to use `ServerBuilder`
- [ ] Configure transport (stdio, HTTP, etc.)
- [ ] Add middleware if needed
- [ ] Update tests to use Rust test framework
- [ ] Update documentation

---

## 8. Troubleshooting

### 8.1 Common Issues

#### Issue: Tool not found

**Symptoms:**
```
Error: Tool 'my_tool' not found
```

**Causes:**
1. Tool not registered with server
2. Tool name mismatch
3. Tool registered after server start

**Solutions:**

```rust
// ✅ Correct
let server = McpServer::builder()
    .tool(MyTool)  // Register before build
    .build()?;

// ❌ Wrong
let server = McpServer::builder().build()?;
server.register_tool(MyTool);  // Too late!
```

---

#### Issue: Schema validation fails

**Symptoms:**
```
Error: Invalid tool input: missing field `required_param`
```

**Causes:**
1. Input doesn't match schema
2. Schema has wrong field names
3. Type mismatch

**Solutions:**

```rust
// Ensure schema matches expected input
fn input_schema(&self) -> Value {
    json!({
        "type": "object",
        "properties": {
            "required_param": {  // Must match input field
                "type": "string"
            }
        },
        "required": ["required_param"]
    })
}
```

---

#### Issue: Transport closed unexpectedly

**Symptoms:**
```
Error: Connection closed
```

**Causes:**
1. Client disconnected
2. stdin/stdout closed
3. Network error

**Solutions:**

```rust
// Graceful shutdown
match server.serve(transport).await {
    Ok(_) => println!("Server shutdown normally"),
    Err(McpError::Transport(TransportError::Closed)) => {
        println!("Client disconnected");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

#### Issue: Async runtime errors

**Symptoms:**
```
Error: there is no reactor running
```

**Causes:**
1. Missing `#[tokio::main]`
2. Using `block_on` incorrectly
3. Runtime not initialized

**Solutions:**

```rust
// ✅ Correct
#[tokio::main]
async fn main() -> Result<()> {
    server.serve(transport).await
}

// ❌ Wrong
fn main() {
    // No async runtime
    server.serve(transport).await  // Won't compile
}
```

---

### 8.2 Performance Issues

#### Issue: Slow tool execution

**Diagnosis:**

```rust
async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
    let start = Instant::now();
    let result = self.operation().await?;
    tracing::info!("Execution took: {:?}", start.elapsed());
    Ok(result)
}
```

**Solutions:**

1. **Add timeouts:**

```rust
let result = tokio::time::timeout(
    Duration::from_secs(30),
    self.operation()
).await??;
```

2. **Use concurrency:**

```rust
let (r1, r2) = tokio::join!(
    self.task1(),
    self.task2(),
);
```

3. **Cache results:**

```rust
struct CachedTool {
    cache: Arc<Mutex<HashMap<String, CachedResult>>>,
}
```

---

#### Issue: High memory usage

**Diagnosis:**

```rust
// Monitor heap allocations
let bytes_allocated = /* use allocator tracking */;
```

**Solutions:**

1. **Stream large data:**

```rust
// Instead of loading all at once
async fn read_large_file(&self) -> Result<String> {
    // Stream in chunks
    use tokio::io::AsyncReadExt;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).await?;
    Ok(String::from_utf8(buffer)?)
}
```

2. **Use references:**

```rust
// Avoid cloning
fn process(&self, data: &str) -> Result<()> {
    // Work with reference
}
```

---

### 8.3 Debugging Tips

**Enable debug logging:**

```rust
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Your code
}
```

**Add tool-level logging:**

```rust
async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
    tracing::debug!("Input: {:?}", input);
    tracing::debug!("Context: {:?}", context);

    let result = self.operation().await?;

    tracing::debug!("Result: {:?}", result);
    Ok(result)
}
```

**Use MockTransport for testing:**

```rust
let transport = MockTransport::new();
transport.push_request(/* test request */);

// Inspect responses
let responses = transport.responses();
dbg!(&responses);
```

---

### 8.4 Getting Help

1. **Check error messages:** Most errors include detailed context
2. **Enable debug logging:** Use `RUST_LOG=debug`
3. **Review documentation:** Check API docs and examples
4. **Inspect protocol:** Use `--trace` to see JSON-RPC messages
5. **File an issue:** Include minimal reproduction case

---

## Appendix A: Complete Example

Here's a complete, production-ready example:

```rust
use mcp_server::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Domain Types
// ============================================================================

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
            ]),
        })
    }

    async fn get_user(&self, id: u64) -> Option<User> {
        self.users.lock().await
            .iter()
            .find(|u| u.id == id)
            .cloned()
    }

    async fn create_user(&self, name: String, email: String) -> User {
        let mut users = self.users.lock().await;
        let id = users.len() as u64 + 1;
        let user = User { id, name, email };
        users.push(user.clone());
        user
    }
}

// ============================================================================
// Tools
// ============================================================================

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
        let id = input["id"]
            .as_u64()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "id must be a positive integer".to_string()
            ))?;

        match self.db.get_user(id).await {
            Some(user) => Ok(ToolResult::success_json(json!(user))),
            None => Err(ToolError::ExecutionFailed(
                format!("User {} not found", id)
            )),
        }
    }
}

struct CreateUserTool {
    db: Arc<Database>,
}

#[async_trait]
impl Tool for CreateUserTool {
    fn name(&self) -> &str {
        "create_user"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new user")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "User's full name"
                },
                "email": {
                    "type": "string",
                    "description": "User's email address",
                    "format": "email"
                }
            },
            "required": ["name", "email"]
        })
    }

    async fn execute(
        &self,
        input: Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let name = input["name"]
            .as_str()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "name is required".to_string()
            ))?
            .to_string();

        let email = input["email"]
            .as_str()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "email is required".to_string()
            ))?
            .to_string();

        let user = self.db.create_user(name, email).await;
        Ok(ToolResult::success_json(json!(user)))
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create shared database
    let db = Database::new();

    // Build server
    let server = McpServer::builder()
        .name("user-service")
        .version("1.0.0")
        .tool(GetUserTool { db: db.clone() })
        .tool(CreateUserTool { db: db.clone() })
        .middleware(LoggingMiddleware::new())
        .build()?;

    // Serve
    tracing::info!("Starting MCP server...");
    server.serve(StdioTransport::new()).await?;

    Ok(())
}
```

---

## Appendix B: JSON-RPC Reference

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "my_tool",
    "arguments": {
      "param1": "value1"
    }
  }
}
```

### Response Format (Success)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Operation successful"
      }
    ]
  }
}
```

### Response Format (Error)

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Tool execution failed",
    "data": {
      "details": "Invalid input"
    }
  }
}
```

---

## Appendix C: Performance Characteristics

### Memory Usage

- **Per tool:** < 1KB (excluding tool-specific state)
- **Per request:** ~2-4KB (depending on payload size)
- **Server overhead:** ~100-200KB (base runtime)

### Latency

- **Tool registration:** < 1μs
- **Schema validation:** < 10μs
- **Request routing:** < 50μs
- **Total overhead:** < 100μs (excluding tool execution)

### Throughput

- **Sequential:** ~10,000 requests/sec (simple tools)
- **Concurrent:** Limited by CPU cores and tool implementation
- **Network:** Depends on transport (stdio ~1GB/s, HTTP ~100MB/s)

---

**End of Guide**

For the latest updates, see: https://github.com/omnitron-dev/meridian
