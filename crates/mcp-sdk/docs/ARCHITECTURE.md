# MCP Server Framework - Architecture

**Version:** 0.1.0
**Last Updated:** 2025-10-20

This document describes the technical architecture, design decisions, and implementation details of the MCP Server framework.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Module Structure](#2-module-structure)
3. [Design Decisions](#3-design-decisions)
4. [Thread Safety Model](#4-thread-safety-model)
5. [Performance Characteristics](#5-performance-characteristics)
6. [Extension Points](#6-extension-points)
7. [Implementation Details](#7-implementation-details)

---

## 1. Overview

### 1.1 Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│                         Application Layer                         │
│  ┌────────────────┐  ┌────────────────┐  ┌─────────────────┐    │
│  │  User Tools    │  │  User Resources│  │ User Middleware │    │
│  └────────┬───────┘  └────────┬───────┘  └────────┬────────┘    │
└───────────┼──────────────────┼────────────────────┼──────────────┘
            │                  │                    │
┌───────────┼──────────────────┼────────────────────┼──────────────┐
│           ▼                  ▼                    ▼               │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              McpServer (Core Coordinator)                │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │  - Request routing                                       │    │
│  │  - Lifecycle management                                  │    │
│  │  - Component coordination                                │    │
│  └─────────────────────────────────────────────────────────┘    │
│           │                  │                    │               │
│  ┌────────┼──────────────────┼────────────────────┼──────────┐  │
│  │        ▼                  ▼                    ▼          │  │
│  │  ┌──────────┐  ┌──────────────┐  ┌──────────────────┐   │  │
│  │  │   Tool   │  │   Resource   │  │   Hook           │   │  │
│  │  │ Registry │  │   Registry   │  │  Registry        │   │  │
│  │  └──────────┘  └──────────────┘  └──────────────────┘   │  │
│  │                                                           │  │
│  │  Component Layer (Thread-Safe Registries)                │  │
│  └───────────────────────────────────────────────────────────┘  │
│                            │                                     │
│  ┌─────────────────────────┼─────────────────────────────────┐  │
│  │                         ▼                                  │  │
│  │           ┌─────────────────────────────┐                 │  │
│  │           │   Middleware Chain          │                 │  │
│  │           │  (Request/Response Pipeline)│                 │  │
│  │           └─────────────────────────────┘                 │  │
│  │                                                            │  │
│  │  Middleware Layer                                         │  │
│  └────────────────────────────────────────────────────────────┘  │
│                            │                                     │
│  ┌─────────────────────────┼─────────────────────────────────┐  │
│  │                         ▼                                  │  │
│  │           ┌─────────────────────────────┐                 │  │
│  │           │  Protocol Handler            │                 │  │
│  │           │  (JSON-RPC Request Router)   │                 │  │
│  │           └─────────────────────────────┘                 │  │
│  │                         │                                  │  │
│  │           ┌─────────────┴─────────────┐                   │  │
│  │           │                           │                   │  │
│  │    ┌──────▼──────┐            ┌──────▼──────┐            │  │
│  │    │  Request    │            │  Response   │            │  │
│  │    │  Validator  │            │  Builder    │            │  │
│  │    └─────────────┘            └─────────────┘            │  │
│  │                                                            │  │
│  │  Protocol Layer (JSON-RPC)                                │  │
│  └────────────────────────────────────────────────────────────┘  │
│                            │                                     │
│  ┌─────────────────────────┼─────────────────────────────────┐  │
│  │                         ▼                                  │  │
│  │           ┌─────────────────────────────┐                 │  │
│  │           │  Transport Interface        │                 │  │
│  │           │  (recv/send/close)          │                 │  │
│  │           └──────────┬──────────────────┘                 │  │
│  │                      │                                     │  │
│  │        ┌─────────────┼─────────────┐                      │  │
│  │        │             │             │                      │  │
│  │  ┌─────▼─────┐ ┌────▼────┐ ┌──────▼──────┐              │  │
│  │  │   Stdio   │ │  HTTP   │ │  WebSocket  │              │  │
│  │  │ Transport │ │Transport│ │  Transport  │              │  │
│  │  └───────────┘ └─────────┘ └─────────────┘              │  │
│  │                                                            │  │
│  │  Transport Layer (I/O)                                    │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                   │
│                    MCP Server Framework                           │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Philosophy

The framework follows these core principles:

1. **Separation of Concerns**: Each layer has a specific responsibility
2. **Dependency Inversion**: High-level modules don't depend on low-level details
3. **Open/Closed Principle**: Open for extension (traits), closed for modification
4. **Composition Over Inheritance**: Small, composable traits instead of deep hierarchies
5. **Type Safety First**: Compile-time guarantees wherever possible

---

## 2. Module Structure

### 2.1 Crate Organization

```
crates/mcp-server/
├── src/
│   ├── lib.rs              # Public API exports, crate-level docs
│   ├── prelude.rs          # Convenience re-exports
│   ├── error.rs            # Error types and conversions
│   │
│   ├── protocol/           # MCP protocol types
│   │   ├── mod.rs          # Protocol module exports
│   │   ├── types.rs        # Core protocol types
│   │   ├── request.rs      # JSON-RPC request types
│   │   ├── response.rs     # JSON-RPC response types
│   │   ├── error.rs        # JSON-RPC error types
│   │   └── capabilities.rs # Server/client capabilities
│   │
│   ├── tool/               # Tool system
│   │   ├── mod.rs          # Tool module exports
│   │   ├── traits.rs       # Tool trait definition
│   │   ├── context.rs      # ToolContext and builder
│   │   ├── result.rs       # ToolResult and ToolContent
│   │   └── registry.rs     # Thread-safe tool registry
│   │
│   ├── resource/           # Resource system
│   │   ├── mod.rs          # Resource module exports
│   │   ├── traits.rs       # Resource trait definition
│   │   ├── context.rs      # ResourceContext
│   │   ├── content.rs      # ResourceContent types
│   │   ├── uri.rs          # URI pattern matching
│   │   └── registry.rs     # Thread-safe resource registry
│   │
│   ├── transport/          # Transport layer
│   │   ├── mod.rs          # Transport module exports
│   │   ├── traits.rs       # Transport trait
│   │   ├── stdio.rs        # Stdio transport
│   │   ├── http.rs         # HTTP/SSE transport
│   │   └── mock.rs         # Mock transport (testing)
│   │
│   ├── middleware/         # Middleware system
│   │   ├── mod.rs          # Middleware module exports
│   │   ├── traits.rs       # Middleware trait
│   │   ├── context.rs      # RequestContext
│   │   ├── logging.rs      # LoggingMiddleware
│   │   └── metrics.rs      # MetricsMiddleware
│   │
│   ├── hooks/              # Hook system
│   │   ├── mod.rs          # Hooks module exports
│   │   ├── traits.rs       # Hook trait
│   │   ├── events.rs       # HookEvent types
│   │   └── registry.rs     # HookRegistry
│   │
│   └── server/             # Server core
│       ├── mod.rs          # Server module exports
│       ├── core.rs         # McpServer implementation
│       ├── builder.rs      # ServerBuilder
│       └── config.rs       # ServerConfig
│
├── examples/               # Example implementations
├── tests/                  # Integration tests
└── benches/               # Performance benchmarks
```

### 2.2 Module Dependencies

```
┌─────────────┐
│   server    │ ◄───┐
└──────┬──────┘     │
       │            │
   ┌───▼───┐    ┌───┴────┐
   │ tool  │    │resource│
   └───┬───┘    └───┬────┘
       │            │
   ┌───▼────────────▼───┐
   │     protocol       │
   └───────┬────────────┘
           │
   ┌───────▼────────┐
   │   transport    │
   └────────────────┘
```

**Dependency Rules:**

- `server` depends on `tool`, `resource`, `protocol`, `transport`, `middleware`, `hooks`
- `tool` and `resource` depend on `protocol` and `error`
- `protocol` has minimal dependencies (serde, serde_json)
- `transport` depends on `protocol`
- `middleware` depends on `protocol`
- `hooks` depends on all other modules
- **No circular dependencies**

---

## 3. Design Decisions

### 3.1 Trait-Based Design

**Decision:** Use traits for all major abstractions (Tool, Resource, Transport, Middleware, Hook).

**Rationale:**
- Enables compile-time polymorphism (no runtime overhead)
- Allows users to implement custom behavior
- Supports testing via mock implementations
- Maintains type safety

**Trade-offs:**
- ✅ Pro: Zero-cost abstraction
- ✅ Pro: Easy to test
- ✅ Pro: Extensible
- ❌ Con: Trait objects require dynamic dispatch (`Arc<dyn Trait>`)
- ❌ Con: Cannot use associated types with trait objects

**Implementation:**

```rust
// Static dispatch (zero-cost)
fn register_tool<T: Tool>(tool: T) { ... }

// Dynamic dispatch (needed for heterogeneous collections)
fn register_tool_dyn(tool: Arc<dyn Tool>) { ... }
```

### 3.2 Async-First Architecture

**Decision:** All I/O operations are async by default.

**Rationale:**
- Modern applications need concurrency
- MCP servers often handle multiple clients
- Tool execution may involve network/database calls
- Better resource utilization

**Trade-offs:**
- ✅ Pro: Excellent scalability
- ✅ Pro: Non-blocking I/O
- ✅ Pro: Standard in Rust ecosystem (Tokio)
- ❌ Con: Requires async runtime
- ❌ Con: Slightly more complex than sync

**Implementation:**

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(&self, ...) -> Result<ToolResult>;
}
```

### 3.3 Builder Pattern for Configuration

**Decision:** Use fluent builder API for server construction.

**Rationale:**
- Clear, self-documenting API
- Compile-time validation of required fields
- Easy to extend without breaking changes
- Common pattern in Rust

**Trade-offs:**
- ✅ Pro: Ergonomic API
- ✅ Pro: Type-safe
- ✅ Pro: Extensible
- ❌ Con: Slightly more code than direct construction

**Implementation:**

```rust
let server = McpServer::builder()
    .name("my-server")     // Required
    .version("1.0.0")      // Required
    .tool(MyTool)          // Optional (can chain multiple)
    .middleware(MyMw)      // Optional
    .build()?;             // Validates and builds
```

### 3.4 Error Handling Strategy

**Decision:** Use `thiserror` for error types and automatic JSON-RPC conversion.

**Rationale:**
- Type-safe error handling
- Automatic `From` trait implementations
- Easy to map to JSON-RPC error codes
- Clear error messages

**Trade-offs:**
- ✅ Pro: Type-safe
- ✅ Pro: Automatic conversions
- ✅ Pro: Easy to extend
- ❌ Con: Requires error enums (more verbose than strings)

**Implementation:**

```rust
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    // ... more variants
}

impl From<ToolError> for JsonRpcError {
    fn from(error: ToolError) -> Self {
        match error {
            ToolError::NotFound(name) => JsonRpcError {
                code: -32601,
                message: format!("Tool '{}' not found", name),
                data: None,
            },
            // ... more conversions
        }
    }
}
```

### 3.5 Thread-Safe Registries

**Decision:** Use `Arc<RwLock<HashMap>>` for registries.

**Rationale:**
- Allows concurrent reads
- Prevents data races
- Standard pattern for shared mutable state
- Works with async code

**Trade-offs:**
- ✅ Pro: Thread-safe
- ✅ Pro: Multiple concurrent readers
- ✅ Pro: Write safety
- ❌ Con: Lock contention on writes
- ❌ Con: Slight overhead vs. non-thread-safe

**Alternatives Considered:**

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| `Arc<RwLock<HashMap>>` | Multiple readers, thread-safe | Lock contention | ✅ **Chosen** |
| `DashMap` | Lock-free, very fast | Additional dependency | Considered for v0.2 |
| `Mutex<HashMap>` | Simpler | Only one accessor at a time | Too restrictive |
| `HashMap` (no sync) | Fastest | Not thread-safe | Not acceptable |

**Implementation:**

```rust
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;  // Multiple readers OK
        tools.get(name).cloned()
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let mut tools = self.tools.write().await;  // Exclusive write
        tools.insert(tool.name().to_string(), tool);
        Ok(())
    }
}
```

### 3.6 Transport Abstraction

**Decision:** Abstract transport as a trait with async methods.

**Rationale:**
- Supports multiple transport types (stdio, HTTP, WebSocket)
- Easy to test with mock transport
- Clean separation of protocol from I/O
- Extensible for custom transports

**Trade-offs:**
- ✅ Pro: Very flexible
- ✅ Pro: Testable
- ✅ Pro: Transport-agnostic server
- ❌ Con: All transports must fit the same interface

**Implementation:**

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn recv(&mut self) -> Option<JsonRpcRequest>;
    async fn send(&mut self, response: JsonRpcResponse) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
    fn is_closed(&self) -> bool;
}
```

### 3.7 Middleware Chain

**Decision:** Sequential execution of middleware in registration order.

**Rationale:**
- Predictable execution order
- Simple mental model
- Matches other frameworks (Express.js, axum)
- Easy to reason about

**Trade-offs:**
- ✅ Pro: Simple and predictable
- ✅ Pro: Easy to debug
- ✅ Pro: Familiar pattern
- ❌ Con: Order matters (must document)
- ❌ Con: Cannot parallelize

**Implementation:**

```rust
// Request phase: forward order
for middleware in &self.middleware {
    middleware.on_request(&request, &mut context).await?;
}

// Handle request
let response = self.handle(request).await;

// Response phase: reverse order
for middleware in self.middleware.iter().rev() {
    middleware.on_response(&response, &context).await?;
}
```

---

## 4. Thread Safety Model

### 4.1 Thread Safety Guarantees

All public types are `Send + Sync`:

```rust
// Core types
impl Send for McpServer {}
impl Sync for McpServer {}

// Trait requirements
pub trait Tool: Send + Sync { ... }
pub trait Resource: Send + Sync { ... }
pub trait Transport: Send + Sync { ... }
pub trait Middleware: Send + Sync { ... }
pub trait Hook: Send + Sync { ... }
```

### 4.2 Interior Mutability

When mutable state is needed, we use:

1. **RwLock** for read-heavy workloads (registries)
2. **Mutex** for write-heavy workloads
3. **Atomic** for simple counters/flags

**Example:**

```rust
pub struct ToolRegistry {
    // Read-heavy: most operations are lookups
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

pub struct MetricsCollector {
    // Write-heavy: frequent updates
    request_count: Arc<Mutex<HashMap<String, u64>>>,

    // Simple counter
    total_requests: Arc<AtomicU64>,
}
```

### 4.3 Shared State Patterns

**Pattern 1: Immutable Shared State**

```rust
struct DatabaseTool {
    // Shared, immutable reference
    config: Arc<Config>,
}
```

**Pattern 2: Mutable Shared State with RwLock**

```rust
struct CachingTool {
    // Multiple readers, exclusive writer
    cache: Arc<RwLock<HashMap<String, CachedValue>>>,
}
```

**Pattern 3: Message Passing**

```rust
struct AsyncTool {
    // Send messages instead of sharing state
    tx: mpsc::Sender<Command>,
}
```

### 4.4 Concurrency Guarantees

- **Tool Execution**: Multiple tools can execute concurrently
- **Resource Reading**: Multiple resources can be read concurrently
- **Middleware**: Executes sequentially (by design)
- **Hook Emission**: Hooks execute sequentially (by design)
- **Registry Operations**: Reads are concurrent, writes are exclusive

---

## 5. Performance Characteristics

### 5.1 Time Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Tool registration | O(1) | HashMap insert |
| Tool lookup | O(1) | HashMap get |
| Resource registration | O(1) | HashMap insert |
| Resource lookup | O(n) | Linear scan for pattern match |
| Middleware execution | O(m) | m = number of middleware |
| Hook emission | O(h) | h = number of hooks |
| Request parsing | O(k) | k = request size |
| Response building | O(k) | k = response size |

### 5.2 Space Complexity

| Component | Memory Usage | Notes |
|-----------|--------------|-------|
| McpServer | ~200KB | Base overhead |
| Tool (trait object) | ~48 bytes | Arc + vtable ptr |
| Resource (trait object) | ~48 bytes | Arc + vtable ptr |
| Request | ~2-4KB | Varies with payload |
| Response | ~2-4KB | Varies with payload |
| Context | ~256 bytes | Small, stack-allocated |

### 5.3 Latency Breakdown

```
Total Request Latency
├── Transport recv: ~10-50μs (stdio) / ~100-500μs (HTTP)
├── JSON parsing: ~50-100μs
├── Middleware chain: ~10-50μs per middleware
├── Request routing: ~10-50μs
├── Tool lookup: ~1-10μs
├── Tool execution: **VARIES** (user code)
├── Response building: ~50-100μs
├── JSON serialization: ~50-100μs
└── Transport send: ~10-50μs (stdio) / ~100-500μs (HTTP)

Framework Overhead: ~200-500μs (excluding tool execution)
```

### 5.4 Optimization Strategies

**1. Zero-Copy Deserialization**

```rust
// Avoid cloning when possible
fn process(&self, value: &Value) {  // Reference, not owned
    // Work with borrowed data
}
```

**2. Arena Allocation** (future optimization)

```rust
// Allocate related objects together
let arena = Arena::new();
let request = arena.alloc(JsonRpcRequest { ... });
let context = arena.alloc(RequestContext { ... });
```

**3. Object Pooling** (future optimization)

```rust
// Reuse allocations
let context = context_pool.get();
// ... use context ...
context_pool.return(context);
```

**4. Lazy Evaluation**

```rust
// Only compute when needed
fn input_schema(&self) -> Value {
    // Could cache this if called frequently
    schema_for!(MyInput)
}
```

### 5.5 Scalability

**Vertical Scaling (single machine):**
- Tokio runtime uses all available cores
- Concurrent tool execution
- Non-blocking I/O
- **Limit:** CPU/memory on single machine

**Horizontal Scaling (multiple machines):**
- Stateless design enables easy replication
- Load balancer can distribute requests
- No shared state between servers
- **Limit:** Load balancer capacity

---

## 6. Extension Points

### 6.1 Custom Tools

Implement the `Tool` trait:

```rust
struct MyTool {
    // Custom state
}

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn input_schema(&self) -> Value { /* ... */ }
    async fn execute(&self, input: Value, context: &ToolContext)
        -> Result<ToolResult, ToolError>
    {
        // Custom logic
    }
}
```

### 6.2 Custom Resources

Implement the `Resource` trait:

```rust
struct MyResource {
    // Custom state
}

#[async_trait]
impl Resource for MyResource {
    fn uri_pattern(&self) -> &str { "custom://*" }
    async fn read(&self, uri: &str, context: &ResourceContext)
        -> Result<ResourceContent, ResourceError>
    {
        // Custom logic
    }
}
```

### 6.3 Custom Transport

Implement the `Transport` trait:

```rust
struct MyTransport {
    // Custom I/O
}

#[async_trait]
impl Transport for MyTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        // Custom receive logic
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<()> {
        // Custom send logic
    }

    async fn close(&mut self) -> Result<()> {
        // Cleanup
    }

    fn is_closed(&self) -> bool {
        // Check state
    }
}
```

### 6.4 Custom Middleware

Implement the `Middleware` trait:

```rust
struct MyMiddleware {
    // Custom state
}

#[async_trait]
impl Middleware for MyMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        // Pre-processing
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        // Post-processing
    }
}
```

### 6.5 Custom Hooks

Implement the `Hook` trait:

```rust
struct MyHook {
    // Custom state
}

#[async_trait]
impl Hook for MyHook {
    async fn on_event(&self, event: HookEvent) -> Result<(), HookError> {
        match event {
            HookEvent::ToolCalled { name, args } => {
                // Handle tool call
            }
            _ => {}
        }
        Ok(())
    }
}
```

---

## 7. Implementation Details

### 7.1 Request Flow

```
1. Transport.recv()
   └─> Receives raw bytes
       └─> Parses to JsonRpcRequest

2. Middleware.on_request()
   └─> Each middleware processes request
       └─> Can modify RequestContext
           └─> Can block request (return error)

3. Server.handle_request()
   └─> Match on method:
       ├─> "initialize" → handle_initialize()
       ├─> "tools/list" → handle_list_tools()
       ├─> "tools/call" → handle_call_tool()
       │   └─> ToolRegistry.get(name)
       │       └─> Tool.execute(input, context)
       │           └─> Returns ToolResult
       ├─> "resources/list" → handle_list_resources()
       ├─> "resources/read" → handle_read_resource()
       │   └─> ResourceRegistry.find(uri)
       │       └─> Resource.read(uri, context)
       │           └─> Returns ResourceContent
       └─> unknown → method_not_found

4. Build JsonRpcResponse
   └─> Success: set result field
   └─> Error: set error field

5. Middleware.on_response()
   └─> Each middleware processes response (reverse order)
       └─> Can modify response
           └─> Can log/collect metrics

6. Transport.send()
   └─> Serializes JsonRpcResponse
       └─> Sends bytes
```

### 7.2 Tool Execution Details

```rust
async fn handle_call_tool(&self, params: CallToolParams) -> JsonRpcResponse {
    // 1. Get tool from registry
    let tool = match self.tools.get(&params.name).await {
        Some(tool) => tool,
        None => return error_response(ToolError::NotFound(params.name)),
    };

    // 2. Build context
    let context = ToolContext::builder()
        .session_id(&self.session_id)
        .client_info(&self.client_name, &self.client_version)
        .build();

    // 3. Execute tool
    let result = match tool.execute(params.arguments, &context).await {
        Ok(result) => result,
        Err(e) => return error_response(e),
    };

    // 4. Convert to protocol type
    let call_result = CallToolResult {
        content: result.content.into_iter()
            .map(|c| c.into_protocol_type())
            .collect(),
        is_error: Some(result.is_error),
    };

    // 5. Build response
    JsonRpcResponse::success(
        params.id,
        serde_json::to_value(call_result).unwrap()
    )
}
```

### 7.3 Resource Reading Details

```rust
async fn handle_read_resource(&self, params: ReadResourceParams) -> JsonRpcResponse {
    // 1. Find matching resource
    let resource = match self.resources.find(&params.uri).await {
        Some(resource) => resource,
        None => return error_response(ResourceError::NotFound(params.uri)),
    };

    // 2. Build context
    let context = ResourceContext::new();

    // 3. Read resource
    let content = match resource.read(&params.uri, &context).await {
        Ok(content) => content,
        Err(e) => return error_response(e),
    };

    // 4. Convert to protocol type
    let read_result = ReadResourceResult {
        contents: vec![content.into_protocol_type()],
    };

    // 5. Build response
    JsonRpcResponse::success(
        params.id,
        serde_json::to_value(read_result).unwrap()
    )
}
```

### 7.4 Transport Implementation: Stdio

```rust
pub struct StdioTransport {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
    closed: Arc<AtomicBool>,
}

#[async_trait]
impl Transport for StdioTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        if self.closed.load(Ordering::SeqCst) {
            return None;
        }

        let mut line = String::new();
        match self.stdin.read_line(&mut line).await {
            Ok(0) => {
                // EOF - close transport
                self.closed.store(true, Ordering::SeqCst);
                None
            }
            Ok(_) => {
                // Parse JSON-RPC request
                serde_json::from_str(&line).ok()
            }
            Err(_) => {
                self.closed.store(true, Ordering::SeqCst);
                None
            }
        }
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<()> {
        let json = serde_json::to_string(&response)?;
        writeln!(self.stdout, "{}", json).await?;
        self.stdout.flush().await?;
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

### 7.5 Registry Implementation

```rust
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) -> Result<(), ToolError> {
        let mut tools = self.tools.write().await;
        let name = tool.name().to_string();

        if tools.contains_key(&name) {
            return Err(ToolError::AlreadyRegistered(name));
        }

        tools.insert(name, tool);
        Ok(())
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values()
            .map(|tool| ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect()
    }

    pub async fn has(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    pub async fn count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }
}
```

---

## Appendix A: Design Patterns Used

### A.1 Trait Objects (Virtual Dispatch)

**Pattern:** Store heterogeneous collections of trait implementors.

```rust
// Heterogeneous collection
Vec<Arc<dyn Tool>>
```

**Use Cases:**
- Tool registry
- Resource registry
- Middleware chain
- Hook registry

### A.2 Builder Pattern

**Pattern:** Construct complex objects step-by-step.

```rust
McpServer::builder()
    .name("server")
    .tool(MyTool)
    .build()
```

**Use Cases:**
- Server construction
- Context building

### A.3 Strategy Pattern

**Pattern:** Encapsulate algorithms in traits.

```rust
trait Transport {
    async fn recv(&mut self) -> Option<Request>;
}
```

**Use Cases:**
- Transport implementations
- Middleware implementations

### A.4 Chain of Responsibility

**Pattern:** Pass request through chain of handlers.

```rust
for middleware in &middlewares {
    middleware.on_request(&request)?;
}
```

**Use Cases:**
- Middleware chain

### A.5 Registry Pattern

**Pattern:** Central registry for object lookup.

```rust
registry.register(tool);
registry.get("tool_name")
```

**Use Cases:**
- Tool registry
- Resource registry

### A.6 Observer Pattern (Hooks)

**Pattern:** Subscribe to events and get notified.

```rust
hooks.register(my_hook);
hooks.emit(HookEvent::ToolCalled { ... });
```

**Use Cases:**
- Hook system

---

## Appendix B: Future Optimizations

### B.1 Planned Optimizations (v0.2.0+)

1. **DashMap for Registries**
   - Replace `RwLock<HashMap>` with `DashMap`
   - Lock-free concurrent access
   - Better performance under high concurrency

2. **Schema Caching**
   - Cache generated JSON schemas
   - Avoid regenerating on every request
   - ~10-20% latency improvement

3. **Connection Pooling**
   - Pool database connections
   - Pool HTTP clients
   - Reduce connection overhead

4. **Batch Operations**
   - Support batching multiple tool calls
   - Reduce round trips
   - Better throughput

5. **Streaming Responses**
   - Stream large responses
   - Reduce memory usage
   - Better user experience

### B.2 Research Areas

1. **Zero-Copy JSON Parsing**
   - Use `serde_json::RawValue`
   - Avoid copying large payloads
   - Complex implementation

2. **Arena Allocation**
   - Allocate related objects together
   - Better cache locality
   - Requires lifetime management

3. **SIMD JSON Parsing**
   - Use `simd-json` crate
   - 2-3x faster JSON parsing
   - Platform-specific code

---

**End of Architecture Document**

For implementation questions, see the [LLM Guide](LLM_GUIDE.md) or [source code](../src/).
