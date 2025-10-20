# MCP Server Framework - Complete Specification

**Version:** 1.0.0
**Status:** DRAFT
**Last Updated:** 2025-01-20
**Authors:** Meridian Development Team

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Analysis of Existing Implementations](#2-analysis-of-existing-implementations)
3. [Design Philosophy](#3-design-philosophy)
4. [Core Architecture](#4-core-architecture)
5. [Type System & Schema Generation](#5-type-system--schema-generation)
6. [Tool Registration](#6-tool-registration)
7. [Resource Management](#7-resource-management)
8. [Transport Layer](#8-transport-layer)
9. [Request/Response Handling](#9-requestresponse-handling)
10. [Middleware & Hooks](#10-middleware--hooks)
11. [Error Handling](#11-error-handling)
12. [Testing Strategy](#12-testing-strategy)
13. [Implementation Plan](#13-implementation-plan)
14. [API Examples](#14-api-examples)
15. [Migration Guide](#15-migration-guide)

---

## 1. Executive Summary

### 1.1 Vision

Create a **universal, type-safe, ergonomic** Rust crate for building MCP (Model Context Protocol) servers with minimal boilerplate and maximum flexibility.

### 1.2 Goals

1. **Type Safety**: Compile-time guarantees for tool/resource definitions
2. **Ergonomics**: Minimal boilerplate through macros and builders
3. **Flexibility**: Support multiple transports and use cases
4. **Performance**: Zero-copy where possible, async by default
5. **Testability**: First-class support for testing at all levels
6. **Compliance**: 100% MCP spec 2025-03-26 compliant

### 1.3 Non-Goals

- Supporting non-MCP protocols
- Providing domain-specific tool implementations
- Managing LLM interactions (client-side concerns)

### 1.4 Success Criteria

- ✅ Can build a basic MCP server in <50 lines of code
- ✅ 100% test coverage on core functionality
- ✅ All tests pass with 0 flaky tests
- ✅ Comprehensive documentation with examples
- ✅ Compatible with official MCP clients (TS SDK, Claude Desktop)

---

## 2. Analysis of Existing Implementations

### 2.1 TypeScript SDK Analysis

**File:** `experiments/claude-agent-sdk/skd-docs.md`

#### Strengths:
- ✅ Excellent type safety with Zod schemas
- ✅ Clean `tool()` function for tool creation
- ✅ `createSdkMcpServer()` for in-process servers
- ✅ Comprehensive hook system
- ✅ Permission system
- ✅ Multiple transport support

#### Key APIs:
```typescript
// Tool creation
tool<Schema extends ZodRawShape>(
  name: string,
  description: string,
  inputSchema: Schema,
  handler: (args, extra) => Promise<CallToolResult>
)

// Server creation
createSdkMcpServer({
  name: string,
  version?: string,
  tools?: Array<SdkMcpToolDefinition<any>>
})
```

#### Learnings:
- Schema-first design is critical for type safety
- Separate tool definition from tool implementation
- Handler functions should be async by default
- Support for metadata enrichment (`_meta` field)

### 2.2 Rust SDK Analysis (claude-sdk-rs)

**File:** `crates/claude-sdk-rs/src/mcp/server/mod.rs`

#### Strengths:
- ✅ Basic MCP protocol implementation
- ✅ Node trait for abstraction
- ✅ TypeId for runtime type tracking
- ✅ HashMap-based tool storage

#### Weaknesses:
- ❌ No macro support (verbose)
- ❌ Limited transport options
- ❌ No resource support
- ❌ Minimal error handling
- ❌ No hook system
- ❌ Manual schema definition

#### Key APIs:
```rust
pub struct MCPToolServer {
    server_name: String,
    server_version: String,
    tools: Arc<RwLock<HashMap<String, (ToolMetadata, Arc<dyn Node>)>>>,
    capabilities: ServerCapabilities,
}

impl MCPToolServer {
    pub async fn register_node_as_tool<T>(
        &self,
        node: Arc<T>,
        metadata: ToolMetadata,
    ) -> Result<(), WorkflowError>
}
```

#### Learnings:
- Need builder pattern for cleaner API
- TypeId is useful but not enough
- Need macro-based tool registration
- Arc<RwLock<>> is necessary for shared state

### 2.3 Cortex MCP Server Analysis

**Files:**
- `cortex/src/mcp/server.rs` (1000+ lines)
- `cortex/src/mcp/tools.rs` (2300+ lines)
- `cortex/src/mcp/handlers.rs` (large)

#### Strengths:
- ✅ Production-ready implementation
- ✅ Multiple transports (stdio, HTTP)
- ✅ ServerMode abstraction (single/multi-project)
- ✅ Comprehensive tool catalog (60+ tools)
- ✅ Resource support
- ✅ Protocol version negotiation
- ✅ Metrics and monitoring

#### Weaknesses:
- ❌ Tightly coupled to Meridian domain
- ❌ No reusable abstractions
- ❌ Manual tool registration
- ❌ Duplicated code patterns

#### Key Patterns:
```rust
// Tool definition pattern
Tool {
    name: "tool.name".to_string(),
    description: Some("Description".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": { ... },
        "required": [...]
    }),
    output_schema: None,
    _meta: None,
}

// Handler pattern
async fn handle_tool_call(
    &self,
    tool_name: &str,
    arguments: Value,
) -> Result<Value>
```

#### Learnings:
- Need separation of concerns (domain vs framework)
- Builder pattern for server configuration
- Transport abstraction is crucial
- Tool catalog should be composable
- Metrics/monitoring should be pluggable

### 2.4 Synthesis

**What to keep:**
- Type safety from TS SDK (via macros + serde)
- Node abstraction from claude-sdk-rs
- Production patterns from Cortex
- Schema-first design

**What to improve:**
- Ergonomics (macros, builders)
- Modularity (pluggable everything)
- Testability (mock transports, test harness)
- Documentation (examples, guides)

---

## 3. Design Philosophy

### 3.1 Core Principles

1. **Type Safety First**
   - Compile-time validation of tool signatures
   - Automatic schema generation from Rust types
   - No runtime surprises

2. **Zero Boilerplate**
   - Macros for common patterns
   - Builder APIs for configuration
   - Sensible defaults

3. **Composition Over Inheritance**
   - Small, focused traits
   - Mixins via trait composition
   - No deep hierarchies

4. **Async by Default**
   - All I/O operations async
   - Tokio-based runtime
   - Backpressure support

5. **Testability**
   - Mock implementations for testing
   - In-memory transports
   - Deterministic behavior

### 3.2 API Design Goals

```rust
// Goal: Build a server in ~50 lines
use mcp_server::prelude::*;

#[derive(McpTool)]
#[tool(
    name = "echo",
    description = "Echo the input back"
)]
struct EchoTool;

#[mcp_handler]
impl EchoTool {
    async fn handle(&self, input: EchoInput) -> Result<EchoOutput> {
        Ok(EchoOutput { message: input.message })
    }
}

#[tokio::main]
async fn main() {
    McpServer::builder()
        .name("example-server")
        .version("1.0.0")
        .tool(EchoTool)
        .transport(StdioTransport::new())
        .build()
        .serve()
        .await
        .unwrap();
}
```

---

## 4. Core Architecture

### 4.1 Crate Structure

```
crates/mcp-server/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API exports
│   ├── prelude.rs          # Convenience re-exports
│   ├── server/
│   │   ├── mod.rs          # McpServer core
│   │   ├── builder.rs      # Builder pattern
│   │   └── config.rs       # Configuration
│   ├── protocol/
│   │   ├── mod.rs          # MCP protocol types
│   │   ├── request.rs      # JSON-RPC requests
│   │   ├── response.rs     # JSON-RPC responses
│   │   ├── error.rs        # MCP error codes
│   │   └── capabilities.rs # Server capabilities
│   ├── tool/
│   │   ├── mod.rs          # Tool trait & registry
│   │   ├── macros.rs       # #[derive(McpTool)]
│   │   ├── schema.rs       # Schema generation
│   │   └── handler.rs      # Handler trait
│   ├── resource/
│   │   ├── mod.rs          # Resource trait & registry
│   │   ├── macros.rs       # #[derive(McpResource)]
│   │   └── uri.rs          # URI handling
│   ├── transport/
│   │   ├── mod.rs          # Transport trait
│   │   ├── stdio.rs        # Stdio transport
│   │   ├── http.rs         # HTTP/SSE transport
│   │   ├── websocket.rs    # WebSocket transport
│   │   └── mock.rs         # Mock transport for testing
│   ├── middleware/
│   │   ├── mod.rs          # Middleware trait
│   │   ├── logging.rs      # Request logging
│   │   ├── metrics.rs      # Metrics collection
│   │   └── auth.rs         # Authentication
│   ├── hooks/
│   │   ├── mod.rs          # Hook system
│   │   └── types.rs        # Hook event types
│   ├── error.rs            # Error types
│   └── testing/
│       ├── mod.rs          # Test utilities
│       ├── mock_client.rs  # Mock MCP client
│       └── harness.rs      # Test harness
├── examples/
│   ├── simple.rs           # Minimal example
│   ├── full_featured.rs    # All features
│   └── custom_transport.rs # Custom transport
└── tests/
    ├── protocol_tests.rs   # Protocol compliance
    ├── integration.rs      # Integration tests
    └── e2e.rs             # End-to-end tests
```

### 4.2 Core Traits

```rust
/// A tool that can be called by MCP clients
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> Option<&str> {
        None
    }

    /// JSON schema for input validation
    fn input_schema(&self) -> JsonSchema;

    /// Optional JSON schema for output
    fn output_schema(&self) -> Option<JsonSchema> {
        None
    }

    /// Execute the tool with validated input
    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// Optional metadata
    fn metadata(&self) -> Option<Value> {
        None
    }
}

/// A resource that can be read by MCP clients
#[async_trait]
pub trait Resource: Send + Sync {
    /// Resource URI pattern
    fn uri(&self) -> &str;

    /// Resource name
    fn name(&self) -> Option<&str> {
        None
    }

    /// Resource description
    fn description(&self) -> Option<&str> {
        None
    }

    /// MIME type
    fn mime_type(&self) -> Option<&str> {
        None
    }

    /// Read the resource
    async fn read(
        &self,
        uri: &str,
        context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError>;
}

/// Transport layer for sending/receiving messages
#[async_trait]
pub trait Transport: Send + Sync {
    /// Receive next request
    async fn recv(&mut self) -> Option<JsonRpcRequest>;

    /// Send a response
    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError>;

    /// Close the transport
    async fn close(&mut self) -> Result<(), TransportError>;
}

/// Middleware for intercepting requests/responses
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process request before routing
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<(), MiddlewareError>;

    /// Process response before sending
    async fn on_response(
        &self,
        response: &JsonRpcResponse,
    ) -> Result<(), MiddlewareError>;
}
```

### 4.3 Server Core

```rust
/// Main MCP server
pub struct McpServer {
    config: ServerConfig,
    tools: Arc<ToolRegistry>,
    resources: Arc<ResourceRegistry>,
    middleware: Vec<Arc<dyn Middleware>>,
    hooks: HookRegistry,
}

impl McpServer {
    /// Create a new builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Serve with the configured transport
    pub async fn serve<T: Transport>(
        self,
        mut transport: T,
    ) -> Result<(), ServerError> {
        // Main event loop
        while let Some(request) = transport.recv().await {
            // Apply middleware
            for mw in &self.middleware {
                mw.on_request(&request).await?;
            }

            // Route and handle
            let response = self.handle_request(request).await;

            // Apply middleware
            for mw in &self.middleware {
                mw.on_response(&response).await?;
            }

            // Send response
            transport.send(response).await?;
        }

        Ok(())
    }

    /// Handle a single request
    async fn handle_request(
        &self,
        request: JsonRpcRequest,
    ) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            "resources/list" => self.handle_list_resources(request).await,
            "resources/read" => self.handle_read_resource(request).await,
            _ => JsonRpcResponse::method_not_found(request.id),
        }
    }
}
```

---

## 5. Type System & Schema Generation

### 5.1 Schema Generation

Use `schemars` for automatic JSON Schema generation:

```rust
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EchoInput {
    /// The message to echo
    #[schemars(description = "The message to echo back")]
    pub message: String,

    /// Optional repeat count
    #[schemars(description = "Number of times to repeat", default = "1")]
    #[serde(default = "default_repeat")]
    pub repeat: usize,
}

fn default_repeat() -> usize { 1 }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EchoOutput {
    /// The echoed message
    pub message: String,
}

// Auto-generate schema
let schema = schema_for!(EchoInput);
```

### 5.2 Macro-Based Tool Registration

```rust
/// Derive macro for tools
#[proc_macro_derive(McpTool, attributes(tool))]
pub fn derive_mcp_tool(input: TokenStream) -> TokenStream {
    // Parse attributes
    let attrs = parse_tool_attributes(&input);

    // Generate Tool impl
    quote! {
        #[async_trait]
        impl Tool for #name {
            fn name(&self) -> &str {
                #tool_name
            }

            fn description(&self) -> Option<&str> {
                Some(#tool_description)
            }

            fn input_schema(&self) -> JsonSchema {
                schemars::schema_for!(#input_type)
            }

            fn output_schema(&self) -> Option<JsonSchema> {
                Some(schemars::schema_for!(#output_type))
            }

            async fn execute(
                &self,
                input: Value,
                context: &ToolContext,
            ) -> Result<ToolResult, ToolError> {
                // Deserialize input
                let typed_input: #input_type =
                    serde_json::from_value(input)
                        .map_err(ToolError::InvalidInput)?;

                // Call handler
                let output = self.handle(typed_input)
                    .await
                    .map_err(ToolError::ExecutionFailed)?;

                // Serialize output
                let value = serde_json::to_value(output)
                    .map_err(ToolError::SerializationFailed)?;

                Ok(ToolResult::success(value))
            }
        }
    }.into()
}
```

### 5.3 Type-Safe Handler

```rust
/// Handler trait with typed input/output
#[async_trait]
pub trait TypedToolHandler<I, O>: Send + Sync
where
    I: DeserializeOwned + JsonSchema,
    O: Serialize + JsonSchema,
{
    async fn handle(&self, input: I) -> Result<O, ToolError>;
}

/// Blanket implementation for Tool
#[async_trait]
impl<T, I, O> Tool for T
where
    T: TypedToolHandler<I, O> + ToolMetadata,
    I: DeserializeOwned + JsonSchema,
    O: Serialize + JsonSchema,
{
    fn input_schema(&self) -> JsonSchema {
        schema_for!(I)
    }

    fn output_schema(&self) -> Option<JsonSchema> {
        Some(schema_for!(O))
    }

    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let typed_input: I = serde_json::from_value(input)?;
        let output = self.handle(typed_input).await?;
        let value = serde_json::to_value(output)?;
        Ok(ToolResult::success(value))
    }
}
```

---

## 6. Tool Registration

### 6.1 Registry Design

```rust
/// Thread-safe tool registry
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub async fn register<T: Tool + 'static>(&self, tool: T) -> Result<()> {
        let mut tools = self.tools.write().await;
        let name = tool.name().to_string();

        if tools.contains_key(&name) {
            return Err(RegistryError::DuplicateTool(name));
        }

        tools.insert(name, Arc::new(tool));
        Ok(())
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// List all tools
    pub async fn list(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
                output_schema: tool.output_schema(),
            })
            .collect()
    }
}
```

### 6.2 Builder Integration

```rust
pub struct ServerBuilder {
    name: String,
    version: String,
    tools: Vec<Box<dyn Tool>>,
    resources: Vec<Box<dyn Resource>>,
    middleware: Vec<Arc<dyn Middleware>>,
}

impl ServerBuilder {
    /// Add a tool
    pub fn tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Add multiple tools
    pub fn tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = Box<dyn Tool>>,
    {
        self.tools.extend(tools);
        self
    }

    /// Build the server
    pub fn build(self) -> McpServer {
        let registry = ToolRegistry::new();

        // Register all tools
        for tool in self.tools {
            registry.register(tool).await.unwrap();
        }

        McpServer {
            config: ServerConfig {
                name: self.name,
                version: self.version,
            },
            tools: Arc::new(registry),
            // ... rest of initialization
        }
    }
}
```

---

## 7. Resource Management

### 7.1 Resource Pattern

```rust
/// Resource trait
#[async_trait]
pub trait Resource: Send + Sync {
    /// URI pattern (supports wildcards)
    fn uri_pattern(&self) -> &str;

    /// Check if URI matches this resource
    fn matches(&self, uri: &str) -> bool {
        // Support glob patterns
        glob_match(self.uri_pattern(), uri)
    }

    /// Read the resource
    async fn read(
        &self,
        uri: &str,
        context: &ResourceContext,
    ) -> Result<ResourceContent, ResourceError>;
}

/// Resource content
pub enum ResourceContent {
    Text {
        text: String,
        mime_type: String,
    },
    Blob {
        data: Vec<u8>,
        mime_type: String,
    },
}
```

### 7.2 Example: Static Resource

```rust
#[derive(McpResource)]
#[resource(
    uri = "app://config",
    name = "Application Config",
    mime_type = "application/json"
)]
struct ConfigResource {
    config: AppConfig,
}

#[async_trait]
impl ResourceHandler for ConfigResource {
    type Output = AppConfig;

    async fn read(&self, uri: &str) -> Result<Self::Output> {
        Ok(self.config.clone())
    }
}
```

### 7.3 Example: Dynamic Resource

```rust
#[derive(McpResource)]
#[resource(
    uri = "db://users/*",
    name = "User Database",
    mime_type = "application/json"
)]
struct UserResource {
    db: Arc<Database>,
}

#[async_trait]
impl ResourceHandler for UserResource {
    type Output = User;

    async fn read(&self, uri: &str) -> Result<Self::Output> {
        // Extract user ID from URI
        let user_id = uri.strip_prefix("db://users/").unwrap();

        // Query database
        self.db.get_user(user_id).await
    }
}
```

---

## 8. Transport Layer

### 8.1 Transport Trait

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    /// Receive next request (blocking)
    async fn recv(&mut self) -> Option<JsonRpcRequest>;

    /// Send response
    async fn send(&mut self, response: JsonRpcResponse) -> Result<()>;

    /// Close transport gracefully
    async fn close(&mut self) -> Result<()>;

    /// Check if transport is closed
    fn is_closed(&self) -> bool;
}
```

### 8.2 Stdio Transport

```rust
pub struct StdioTransport {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
    closed: Arc<AtomicBool>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(io::stdin()),
            stdout: io::stdout(),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        if self.closed.load(Ordering::SeqCst) {
            return None;
        }

        let mut line = String::new();
        match self.stdin.read_line(&mut line).await {
            Ok(0) => None, // EOF
            Ok(_) => {
                serde_json::from_str(&line).ok()
            }
            Err(_) => None,
        }
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<()> {
        let json = serde_json::to_string(&response)?;
        writeln!(self.stdout, "{}", json)?;
        self.stdout.flush()?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}
```

### 8.3 HTTP/SSE Transport

```rust
pub struct HttpTransport {
    addr: SocketAddr,
    router: Router,
    shutdown_rx: watch::Receiver<bool>,
}

impl HttpTransport {
    pub fn new(addr: SocketAddr) -> Self {
        let router = Router::new()
            .route("/mcp", post(handle_mcp_request))
            .route("/mcp/sse", get(handle_sse));

        Self {
            addr,
            router,
            shutdown_rx: watch::channel(false).1,
        }
    }

    pub async fn serve(self, server: Arc<McpServer>) -> Result<()> {
        axum::Server::bind(&self.addr)
            .serve(self.router.into_make_service())
            .with_graceful_shutdown(shutdown_signal(self.shutdown_rx))
            .await?;

        Ok(())
    }
}

async fn handle_mcp_request(
    State(server): State<Arc<McpServer>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = server.handle_request(request).await;
    Json(response)
}

async fn handle_sse(
    State(server): State<Arc<McpServer>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // SSE stream implementation
    todo!()
}
```

### 8.4 Mock Transport (Testing)

```rust
pub struct MockTransport {
    requests: Arc<Mutex<VecDeque<JsonRpcRequest>>>,
    responses: Arc<Mutex<Vec<JsonRpcResponse>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(VecDeque::new())),
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Queue a request for recv()
    pub fn push_request(&self, request: JsonRpcRequest) {
        self.requests.lock().unwrap().push_back(request);
    }

    /// Get all sent responses
    pub fn responses(&self) -> Vec<JsonRpcResponse> {
        self.responses.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        self.requests.lock().unwrap().pop_front()
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<()> {
        self.responses.lock().unwrap().push(response);
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn is_closed(&self) -> bool {
        false
    }
}
```

---

## 9. Request/Response Handling

### 9.1 JSON-RPC Types

```rust
/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn method_not_found(id: Option<Value>) -> Self {
        Self::error(
            id,
            JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            },
        )
    }
}

/// JSON-RPC 2.0 error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
```

### 9.2 MCP Protocol Types

```rust
/// Initialize request params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Map<String, Value>>,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

/// Tool call params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
    pub is_error: Option<bool>,
}

/// Tool content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource { uri: String },
}
```

---

## 10. Middleware & Hooks

### 10.1 Middleware System

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Called before request is processed
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError>;

    /// Called after response is created
    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError>;
}

/// Example: Logging middleware
pub struct LoggingMiddleware {
    logger: slog::Logger,
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        info!(self.logger, "Incoming request";
            "method" => &request.method,
            "id" => ?request.id,
        );
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        _context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if response.error.is_some() {
            warn!(self.logger, "Request failed";
                "id" => ?response.id,
                "error" => ?response.error,
            );
        } else {
            info!(self.logger, "Request succeeded";
                "id" => ?response.id,
            );
        }
        Ok(())
    }
}

/// Example: Metrics middleware
pub struct MetricsMiddleware {
    collector: Arc<MetricsCollector>,
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        context.start_time = Some(Instant::now());
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        if let Some(start) = context.start_time {
            let duration = start.elapsed();
            self.collector.record_request(
                &context.method,
                duration,
                response.error.is_none(),
            );
        }
        Ok(())
    }
}
```

### 10.2 Hook System

```rust
/// Hook event types
pub enum HookEvent {
    ServerStarted,
    ServerStopped,
    ClientConnected,
    ClientDisconnected,
    ToolCalled { name: String, args: Value },
    ToolCompleted { name: String, result: Result<Value, ToolError> },
    ResourceRead { uri: String },
    Error { error: Box<dyn Error> },
}

/// Hook callback
#[async_trait]
pub trait Hook: Send + Sync {
    async fn on_event(&self, event: HookEvent) -> Result<(), HookError>;
}

/// Hook registry
pub struct HookRegistry {
    hooks: Arc<RwLock<Vec<Arc<dyn Hook>>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register<H: Hook + 'static>(&self, hook: H) {
        let mut hooks = self.hooks.write().await;
        hooks.push(Arc::new(hook));
    }

    pub async fn emit(&self, event: HookEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_event(event.clone()).await {
                eprintln!("Hook error: {}", e);
            }
        }
    }
}

/// Example: Audit hook
pub struct AuditHook {
    db: Arc<Database>,
}

#[async_trait]
impl Hook for AuditHook {
    async fn on_event(&self, event: HookEvent) -> Result<(), HookError> {
        match event {
            HookEvent::ToolCalled { name, args } => {
                self.db.log_tool_call(&name, &args).await?;
            }
            HookEvent::ResourceRead { uri } => {
                self.db.log_resource_read(&uri).await?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

---

## 11. Error Handling

### 11.1 Error Types

```rust
/// Top-level server error
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    #[error("Middleware error: {0}")]
    Middleware(#[from] MiddlewareError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Tool-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(#[from] serde_json::Error),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Transport errors
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection closed")]
    Closed,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}
```

### 11.2 Error Conversion

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
            ToolError::ExecutionFailed(msg) => JsonRpcError {
                code: -32000,
                message: "Tool execution failed".to_string(),
                data: Some(json!({ "details": msg })),
            },
            ToolError::Timeout(duration) => JsonRpcError {
                code: -32001,
                message: format!("Tool execution timeout after {:?}", duration),
                data: None,
            },
            ToolError::Internal(e) => JsonRpcError {
                code: -32603,
                message: "Internal error".to_string(),
                data: Some(json!({ "details": e.to_string() })),
            },
        }
    }
}
```

---

## 12. Testing Strategy

### 12.1 Testing Layers

```
┌─────────────────────────────────────┐
│         E2E Tests                   │ ← Full server + real client
├─────────────────────────────────────┤
│      Integration Tests              │ ← Server + mock transport
├─────────────────────────────────────┤
│        Unit Tests                   │ ← Individual components
└─────────────────────────────────────┘
```

### 12.2 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_register() {
        let registry = ToolRegistry::new();
        let tool = EchoTool::new();

        registry.register(tool).await.unwrap();
        assert!(registry.get("echo").await.is_some());
    }

    #[tokio::test]
    async fn test_tool_registry_duplicate() {
        let registry = ToolRegistry::new();

        registry.register(EchoTool::new()).await.unwrap();
        let result = registry.register(EchoTool::new()).await;

        assert!(matches!(result, Err(RegistryError::DuplicateTool(_))));
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = EchoTool::new();
        let input = json!({ "message": "hello" });
        let context = ToolContext::default();

        let result = tool.execute(input, &context).await.unwrap();
        assert_eq!(result.content[0].text(), Some("hello"));
    }
}
```

### 12.3 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_server_initialization() {
        let mut transport = MockTransport::new();

        // Queue initialize request
        transport.push_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        });

        // Create server
        let server = McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .build();

        // Serve (will process one request)
        tokio::spawn(async move {
            server.serve(transport.clone()).await
        });

        // Wait and check response
        tokio::time::sleep(Duration::from_millis(100)).await;
        let responses = transport.responses();

        assert_eq!(responses.len(), 1);
        assert!(responses[0].result.is_some());
    }

    #[tokio::test]
    async fn test_tool_call_flow() {
        let mut transport = MockTransport::new();

        // Queue tool call request
        transport.push_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "echo",
                "arguments": {
                    "message": "test message"
                }
            })),
        });

        // Create server with echo tool
        let server = McpServer::builder()
            .name("test-server")
            .tool(EchoTool::new())
            .build();

        // Serve
        tokio::spawn(async move {
            server.serve(transport.clone()).await
        });

        // Check response
        tokio::time::sleep(Duration::from_millis(100)).await;
        let responses = transport.responses();

        assert_eq!(responses.len(), 1);
        let result = responses[0].result.as_ref().unwrap();
        assert_eq!(result["content"][0]["text"], "test message");
    }
}
```

### 12.4 E2E Tests

```rust
#[cfg(test)]
mod e2e_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_server_lifecycle() {
        // Start server on random port
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let transport = HttpTransport::new(addr);

        let server = McpServer::builder()
            .name("e2e-test-server")
            .tool(EchoTool::new())
            .tool(AddTool::new())
            .build();

        let handle = tokio::spawn(async move {
            server.serve(transport).await
        });

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Create client and test
        let client = reqwest::Client::new();

        // Test initialize
        let init_response = client
            .post(&format!("http://127.0.0.1:{}/mcp", addr.port()))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap();

        assert!(init_response.result.is_some());

        // Test list tools
        let list_response = client
            .post(&format!("http://127.0.0.1:{}/mcp", addr.port()))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            }))
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap();

        let tools = list_response.result.unwrap()["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);

        // Test tool call
        let call_response = client
            .post(&format!("http://127.0.0.1:{}/mcp", addr.port()))
            .json(&json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": "e2e test"
                    }
                }
            }))
            .send()
            .await
            .unwrap()
            .json::<JsonRpcResponse>()
            .await
            .unwrap();

        assert!(call_response.result.is_some());

        // Cleanup
        handle.abort();
    }
}
```

### 12.5 Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
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
        assert_eq!(request.id, parsed.id);
    }

    #[test]
    fn test_tool_registry_never_panics(
        tool_names in proptest::collection::vec("[a-z]{1,20}", 0..100),
    ) {
        tokio_test::block_on(async {
            let registry = ToolRegistry::new();

            for name in tool_names {
                let tool = MockTool { name: name.clone() };
                let _ = registry.register(tool).await;
            }

            // Should never panic
        });
    }
}
```

### 12.6 Test Coverage Goals

- **Unit Tests:** 100% line coverage
- **Integration Tests:** All protocol flows covered
- **E2E Tests:** Full lifecycle scenarios
- **Property Tests:** Invariants validated

---

## 13. Implementation Plan

### Phase 1: Core Protocol (Week 1-2)

**Goals:**
- ✅ Basic JSON-RPC types
- ✅ Protocol types (initialize, capabilities, etc.)
- ✅ Error handling
- ✅ Basic server structure

**Tasks:**
1. Set up crate structure
2. Implement JSON-RPC types with serde
3. Implement MCP protocol types
4. Implement error types and conversions
5. Write unit tests (target: 100% coverage)

**Acceptance Criteria:**
- Can parse/serialize all MCP protocol messages
- Error handling covers all cases
- Tests pass with 100% coverage

### Phase 2: Tool System (Week 2-3)

**Goals:**
- ✅ Tool trait
- ✅ Tool registry
- ✅ Schema generation with schemars
- ✅ Basic macros

**Tasks:**
1. Define Tool trait
2. Implement ToolRegistry with Arc<RwLock<HashMap>>
3. Integrate schemars for schema generation
4. Create #[derive(McpTool)] macro
5. Create #[mcp_handler] macro
6. Write comprehensive tests

**Acceptance Criteria:**
- Can register and execute tools
- Schemas auto-generate from Rust types
- Macros reduce boilerplate by >80%
- Tests cover all tool operations

### Phase 3: Resource System (Week 3-4)

**Goals:**
- ✅ Resource trait
- ✅ Resource registry
- ✅ URI pattern matching
- ✅ Resource macros

**Tasks:**
1. Define Resource trait
2. Implement ResourceRegistry
3. Implement URI pattern matching (glob support)
4. Create #[derive(McpResource)] macro
5. Write resource tests

**Acceptance Criteria:**
- Can register and read resources
- URI patterns work correctly
- Tests cover all resource operations

### Phase 4: Transport Layer (Week 4-5)

**Goals:**
- ✅ Transport trait
- ✅ Stdio transport
- ✅ HTTP/SSE transport
- ✅ Mock transport for testing

**Tasks:**
1. Define Transport trait
2. Implement StdioTransport
3. Implement HttpTransport with axum
4. Implement MockTransport
5. Write transport tests

**Acceptance Criteria:**
- All transports work correctly
- Can switch transports without code changes
- Mock transport enables easy testing

### Phase 5: Server & Builder (Week 5-6)

**Goals:**
- ✅ McpServer core
- ✅ ServerBuilder with fluent API
- ✅ Request routing
- ✅ Response handling

**Tasks:**
1. Implement McpServer struct
2. Implement ServerBuilder with fluent API
3. Implement request routing
4. Implement handle_initialize, handle_list_tools, handle_call_tool
5. Write server integration tests

**Acceptance Criteria:**
- Can build server with builder pattern
- All MCP methods work correctly
- Integration tests pass

### Phase 6: Middleware & Hooks (Week 6-7)

**Goals:**
- ✅ Middleware trait
- ✅ Hook system
- ✅ Built-in middleware (logging, metrics)

**Tasks:**
1. Define Middleware trait
2. Implement middleware chain
3. Create LoggingMiddleware
4. Create MetricsMiddleware
5. Define HookEvent enum
6. Implement HookRegistry
7. Write middleware/hook tests

**Acceptance Criteria:**
- Middleware chain works correctly
- Hooks fire at correct times
- Tests cover all middleware operations

### Phase 7: Testing & Documentation (Week 7-8)

**Goals:**
- ✅ 100% test coverage
- ✅ Comprehensive examples
- ✅ API documentation
- ✅ User guide

**Tasks:**
1. Write remaining unit tests (target: 100% coverage)
2. Write integration tests for all flows
3. Write E2E tests with real clients
4. Create simple.rs example
5. Create full_featured.rs example
6. Create custom_transport.rs example
7. Write API documentation (rustdoc)
8. Write user guide with examples
9. Write migration guide from existing implementations

**Acceptance Criteria:**
- Test coverage ≥ 100% on core
- All examples compile and run
- Documentation is comprehensive
- User can build server from docs alone

### Phase 8: Polish & Release (Week 8)

**Goals:**
- ✅ Performance optimization
- ✅ Code review
- ✅ CI/CD setup
- ✅ Release preparation

**Tasks:**
1. Profile and optimize hot paths
2. Code review and refactoring
3. Set up GitHub Actions CI
4. Set up documentation hosting
5. Prepare CHANGELOG
6. Tag v1.0.0 release

**Acceptance Criteria:**
- Performance meets benchmarks
- All tests pass in CI
- Documentation published
- Release tagged

---

## 14. API Examples

### 14.1 Minimal Example

```rust
use mcp_server::prelude::*;

#[derive(McpTool)]
#[tool(name = "echo", description = "Echo a message")]
struct EchoTool;

#[derive(Deserialize, JsonSchema)]
struct EchoInput {
    message: String,
}

#[derive(Serialize, JsonSchema)]
struct EchoOutput {
    message: String,
}

#[mcp_handler]
impl EchoTool {
    async fn handle(&self, input: EchoInput) -> Result<EchoOutput> {
        Ok(EchoOutput { message: input.message })
    }
}

#[tokio::main]
async fn main() {
    McpServer::builder()
        .name("simple-server")
        .version("1.0.0")
        .tool(EchoTool)
        .transport(StdioTransport::new())
        .serve()
        .await
        .unwrap();
}
```

### 14.2 Full-Featured Example

```rust
use mcp_server::prelude::*;

// Define tools
#[derive(McpTool)]
#[tool(
    name = "add",
    description = "Add two numbers",
    category = "math"
)]
struct AddTool;

#[derive(Deserialize, JsonSchema)]
struct AddInput {
    #[schemars(description = "First number")]
    a: f64,
    #[schemars(description = "Second number")]
    b: f64,
}

#[derive(Serialize, JsonSchema)]
struct AddOutput {
    result: f64,
}

#[mcp_handler]
impl AddTool {
    async fn handle(&self, input: AddInput) -> Result<AddOutput> {
        Ok(AddOutput {
            result: input.a + input.b,
        })
    }
}

// Define resources
#[derive(McpResource)]
#[resource(
    uri = "app://config",
    name = "Application Config",
    mime_type = "application/json"
)]
struct ConfigResource {
    config: AppConfig,
}

#[async_trait]
impl ResourceHandler for ConfigResource {
    type Output = AppConfig;

    async fn read(&self, _uri: &str) -> Result<Self::Output> {
        Ok(self.config.clone())
    }
}

// Define middleware
struct CustomMiddleware;

#[async_trait]
impl Middleware for CustomMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<()> {
        println!("Request: {}", request.method);
        Ok(())
    }

    async fn on_response(
        &self,
        _response: &JsonRpcResponse,
        _context: &RequestContext,
    ) -> Result<()> {
        println!("Response sent");
        Ok(())
    }
}

// Define hook
struct AuditHook;

#[async_trait]
impl Hook for AuditHook {
    async fn on_event(&self, event: HookEvent) -> Result<()> {
        match event {
            HookEvent::ToolCalled { name, .. } => {
                println!("Tool called: {}", name);
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let config = AppConfig::load().unwrap();

    McpServer::builder()
        .name("full-featured-server")
        .version("1.0.0")
        // Register tools
        .tool(AddTool)
        .tool(SubtractTool)
        .tool(MultiplyTool)
        // Register resources
        .resource(ConfigResource { config: config.clone() })
        // Add middleware
        .middleware(LoggingMiddleware::new())
        .middleware(MetricsMiddleware::new())
        .middleware(CustomMiddleware)
        // Add hooks
        .hook(AuditHook)
        // Configure transport
        .transport(HttpTransport::new("127.0.0.1:3000".parse().unwrap()))
        // Build and serve
        .build()
        .serve()
        .await
        .unwrap();
}
```

### 14.3 Custom Transport Example

```rust
use mcp_server::prelude::*;

// Custom transport using channels
struct ChannelTransport {
    rx: mpsc::Receiver<JsonRpcRequest>,
    tx: mpsc::Sender<JsonRpcResponse>,
}

#[async_trait]
impl Transport for ChannelTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        self.rx.recv().await
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<()> {
        self.tx.send(response).await
            .map_err(|e| TransportError::Internal(e.into()))
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.rx.is_closed()
    }
}

#[tokio::main]
async fn main() {
    let (req_tx, req_rx) = mpsc::channel(100);
    let (res_tx, res_rx) = mpsc::channel(100);

    let transport = ChannelTransport {
        rx: req_rx,
        tx: res_tx,
    };

    let server = McpServer::builder()
        .name("channel-server")
        .tool(EchoTool)
        .build();

    // Serve in background
    tokio::spawn(async move {
        server.serve(transport).await.unwrap();
    });

    // Send requests from another task
    tokio::spawn(async move {
        req_tx.send(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "echo",
                "arguments": { "message": "hello" }
            })),
        }).await.unwrap();

        let response = res_rx.recv().await.unwrap();
        println!("Response: {:?}", response);
    });
}
```

---

## 15. Migration Guide

### 15.1 Migrating from claude-sdk-rs

**Before:**
```rust
let server = MCPToolServer::new(
    "my-server".to_string(),
    "1.0.0".to_string(),
);

let node = Arc::new(MyNode::new());
let metadata = ToolMetadata::new(
    "my_tool".to_string(),
    "Description".to_string(),
    json!({"type": "object"}),
    TypeId::of::<MyNode>(),
);

server.register_node_as_tool(node, metadata).await?;
```

**After:**
```rust
#[derive(McpTool)]
#[tool(name = "my_tool", description = "Description")]
struct MyTool;

#[mcp_handler]
impl MyTool {
    async fn handle(&self, input: MyInput) -> Result<MyOutput> {
        // implementation
    }
}

let server = McpServer::builder()
    .name("my-server")
    .version("1.0.0")
    .tool(MyTool)
    .transport(StdioTransport::new())
    .build();

server.serve().await?;
```

**Key Changes:**
- Use derive macros instead of manual registration
- Type-safe handlers with input/output types
- Builder pattern for server configuration
- Transport is explicit

### 15.2 Migrating from Cortex MCP

**Before (Cortex):**
```rust
// Manually define tool
Tool {
    name: "my_tool".to_string(),
    description: Some("Description".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": {
            "input": {"type": "string"}
        }
    }),
    output_schema: None,
    _meta: None,
}

// Manually implement handler
async fn handle_my_tool(&self, args: Value) -> Result<Value> {
    let input: MyInput = serde_json::from_value(args)?;
    // implementation
    Ok(serde_json::to_value(output)?)
}
```

**After:**
```rust
#[derive(McpTool)]
#[tool(name = "my_tool", description = "Description")]
struct MyTool;

#[derive(Deserialize, JsonSchema)]
struct MyInput {
    #[schemars(description = "Input description")]
    input: String,
}

#[derive(Serialize, JsonSchema)]
struct MyOutput {
    result: String,
}

#[mcp_handler]
impl MyTool {
    async fn handle(&self, input: MyInput) -> Result<MyOutput> {
        // implementation
    }
}
```

**Key Changes:**
- Schema auto-generated from types
- Type-safe input/output
- No manual JSON conversion
- Compile-time validation

### 15.3 Migration Checklist

- [ ] Replace manual tool definitions with derive macros
- [ ] Define typed input/output structs with JsonSchema
- [ ] Replace manual handlers with typed handlers
- [ ] Replace manual server setup with builder
- [ ] Add transport explicitly
- [ ] Update tests to use new API
- [ ] Update documentation

---

## 16. Appendix

### 16.1 Performance Benchmarks

Target performance metrics:

- **Tool registration:** < 1μs per tool
- **Schema generation:** < 10μs per tool
- **Request parsing:** < 100μs per request
- **Tool execution overhead:** < 50μs
- **Memory per tool:** < 1KB

### 16.2 Dependencies

```toml
[dependencies]
# Core
tokio = { version = "1.48", features = ["full"] }
async-trait = "0.1"
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "0.8", features = ["preserve_order"] }

# HTTP transport
axum = { version = "0.8", optional = true }
tower = { version = "0.5", optional = true }
tower-http = { version = "0.6", features = ["cors"], optional = true }

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
tokio-test = "0.4"
proptest = "1.0"
criterion = "0.7"
tempfile = "3.0"

[features]
default = ["stdio"]
stdio = []
http = ["axum", "tower", "tower-http"]
all = ["stdio", "http"]
```

### 16.3 References

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Claude Agent SDK (TypeScript)](https://github.com/anthropics/claude-agent-sdk)
- [schemars Documentation](https://docs.rs/schemars/)
- [axum Documentation](https://docs.rs/axum/)

---

## Status: READY FOR IMPLEMENTATION

This specification is complete and ready for implementation. The next step is to begin Phase 1 (Core Protocol) and work through the implementation plan sequentially.

**Estimated Total Time:** 8 weeks
**Team Size:** 1-2 developers
**Test Coverage Target:** 100% for core, 90%+ overall
**Success Rate Target:** 100% on all tests (0 flaky tests)
