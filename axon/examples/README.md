# Claude Code SDK Examples

Production-ready examples demonstrating the modern Claude Code SDK for Rust.

## Quick Start

1. **Install Claude Code CLI**:
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

2. **Set API credentials**:
   ```bash
   export ANTHROPIC_API_KEY="your-api-key"
   ```

3. **Run an example**:
   ```bash
   cargo run --example modern_client
   ```

## Examples by Category

### 🚀 Getting Started

Start here to learn the fundamentals of the SDK.

#### **modern_client.rs** - Modern ClaudeClient API Basics
The best starting point. Demonstrates type-safe state transitions, binary discovery, and message streaming.

```bash
cargo run --example modern_client
```

**Key concepts:**
- Type-safe builder pattern with state transitions
- Automatic binary discovery
- Sending messages and receiving streaming responses
- Graceful disconnect/reconnect

#### **modern_client_comprehensive.rs** - Full ClaudeClient Demo
Comprehensive example showing all ClaudeClient features including multiple conversations, custom binary paths, and session management.

```bash
cargo run --example modern_client_comprehensive
```

#### **simple_query.rs** - One-Shot Queries
Simplest possible usage: send a query, get a response.

```bash
cargo run --example simple_query
```

**Key concepts:**
- `query()` function for one-shot interactions
- Custom options (model, system prompt)
- Streaming responses

---

### 💬 Interactive & Stateful Usage

Examples showing how to maintain conversation state.

#### **interactive.rs** - Interactive REPL Client
REPL-style client for multi-turn conversations with Claude.

```bash
cargo run --example interactive
```

**Key concepts:**
- Interactive conversation loop with state
- User input handling
- Real-time streaming responses

#### **streaming_output.rs** - Advanced Streaming Patterns
Demonstrates various streaming patterns and message handling strategies.

```bash
cargo run --example streaming_output
```

**Key concepts:**
- `receive_messages_stream()` for full control
- `receive_response_stream()` for convenience
- Multi-turn conversations with streaming
- Concurrent message processing

---

### ⚙️ Advanced Features

Production-ready patterns and optimizations.

#### **session_management.rs** - Session Management
Advanced session features: caching, filtering, searching, forking, and exporting.

```bash
cargo run --example session_management
```

**Key concepts:**
- Session discovery with caching
- Filtering and searching sessions
- Forking sessions for branching conversations
- Exporting sessions to different formats
- Session statistics and metadata

#### **optimized_client_demo.rs** - OptimizedClient Features
Performance optimization patterns including connection pooling, batch processing, and retry logic.

```bash
cargo run --example optimized_client_demo
```

**Key concepts:**
- Connection pooling for one-shot queries
- Interactive mode optimization
- Batch processing with concurrency control
- Exponential backoff retry strategies

#### **error_handling.rs** - Error Handling Best Practices
Comprehensive error handling patterns using modern error types.

```bash
cargo run --example error_handling
```

**Key concepts:**
- Handling binary discovery errors
- Connection retry with exponential backoff
- Stream error handling
- Custom error mapping for application errors
- Graceful degradation patterns

#### **hooks_typed.rs** - Strongly-Typed Hooks
Advanced hook system for intercepting and modifying SDK behavior.

```bash
cargo run --example hooks_typed
```

**Key concepts:**
- Pre/Post tool use hooks
- User prompt modification hooks
- Tool blocking and filtering
- Strongly-typed hook input/output

#### **permission_modes.rs** - Permission Control
Different permission modes and tool restrictions.

```bash
cargo run --example permission_modes
```

**Key concepts:**
- `Default`, `AcceptEdits`, and `BypassPermissions` modes
- Tool allow/deny lists
- Security considerations

---

### 🔌 MCP Integration

Model Context Protocol (MCP) server integration examples.

#### **sdk_mcp_calculator.rs** - Basic MCP Server
In-process MCP server with calculator tools using the modern mcp-sdk crate.

```bash
cargo run --example sdk_mcp_calculator
```

**Key concepts:**
- Creating MCP tools with `Tool` trait
- Registering tools with `McpServer`
- Configuring Claude to use MCP servers
- Tool use and response handling

#### **mcp_integration_patterns.rs** - Advanced MCP Patterns
Production-ready MCP integration patterns.

```bash
cargo run --example mcp_integration_patterns
```

**Key concepts:**
- Stateful tools with context
- Async tools with external API calls
- Complex input validation
- Structured JSON output
- Multiple MCP servers in one session
- Tool composition and chaining

---

### 🌐 Integration Examples

Real-world integration patterns.

#### **batch_processor.rs** - Batch Question Processor
Process multiple questions with retry logic and progress tracking.

```bash
cargo run --example batch_processor
```

**Key concepts:**
- Batch processing from files
- Rate limit detection and retry
- Statistics tracking
- Progress reporting

#### **rest_api_server.rs** - REST API Server
Expose Claude Code SDK via REST API.

```bash
cargo run --example rest_api_server
```

**Key concepts:**
- Axum web framework integration
- Query and batch endpoints
- Health checks and metrics
- Mock mode for testing

#### **openai_compatible_server.rs** - OpenAI API Compatible Server
OpenAI API compatible wrapper around Claude Code.

```bash
cargo run --example openai_compatible_server
```

**Key concepts:**
- OpenAI API compatibility layer
- Chat completions endpoint
- Model listing
- Message format conversion

---

## Example Count Summary

- **Total examples**: 15
- **Getting Started**: 3 examples
- **Interactive & Stateful**: 2 examples
- **Advanced Features**: 5 examples
- **MCP Integration**: 2 examples
- **Integration Examples**: 3 examples

**Before consolidation**: 47 examples (8,640 lines)
**After consolidation**: 15 examples (~4,200 lines)
**Reduction**: 68% fewer examples, focused on quality and real-world use cases

---

## Key Concepts

### Modern API (Phase 3)

The SDK uses a type-safe builder pattern with state transitions:

```rust
let client = ClaudeClient::builder()
    .discover_binary().await?           // NoBinary -> WithBinary
    .model(ModelId::from("..."))        // Configuration
    .permission_mode(PermissionMode::AcceptEdits)
    .configure()                        // WithBinary -> Configured
    .connect().await?                   // Configured -> Connected
    .build()?;                          // Build final client
```

### Error Handling

Use the modern `error` module with structured error types:

```rust
use cc_sdk::error::{SdkError, ClientError, BinaryError};

match result {
    Err(SdkError::Binary(BinaryError::NotFound(path))) => {
        // Handle binary not found
    }
    Err(SdkError::Client(ClientError::ConnectionFailed(msg))) => {
        // Handle connection error
    }
    Ok(value) => {
        // Success
    }
}
```

### Session Management

Leverage the session module for advanced features:

```rust
use cc_sdk::session::{
    cache::{SessionCache, CacheConfig},
    filter::{SessionFilter, SortBy},
    management::{fork_session, export_session, ExportFormat},
};

// List and filter sessions
let filter = SessionFilter::default()
    .with_date_range(start, end)
    .with_sort_by(SortBy::CreatedDesc);
let sessions = search_sessions(filter).await?;

// Fork a session
let new_id = fork_session(&session_id, Some("branched-conversation")).await?;
```

### MCP Integration

Use the mcp-sdk crate for in-process MCP servers:

```rust
use cc_sdk::mcp::{McpServer, Tool, ToolContext, ToolResult};

let server = McpServer::builder()
    .name("my-tools")
    .tool(Arc::new(MyTool))
    .build()?;

let config = create_sdk_server_config("my-tools", Arc::new(server));
```

---

## Troubleshooting

### "Claude CLI not found"
```bash
which claude
# If not found, install:
npm install -g @anthropic-ai/claude-code
```

### "API key not found"
```bash
export ANTHROPIC_API_KEY="your-api-key"
```

### Permission errors
- Use `PermissionMode::AcceptEdits` for automatic edit acceptance
- Use `PermissionMode::BypassPermissions` only in trusted environments
- Restrict tools with `allowed_tools()` when possible

### Model errors
Available models (as of 2025):
- `claude-opus-4-1-20250514` (most capable)
- `claude-sonnet-4-5-20250929` (balanced)
- `claude-3-5-sonnet-20241022` (fast)
- `claude-3-5-haiku-20241022` (fastest)

---

## Best Practices

1. **Use modern ClaudeClient API** - The type-safe builder pattern catches errors at compile time
2. **Handle errors properly** - Use structured error types and implement retry logic
3. **Leverage session management** - Cache, filter, and fork sessions for better performance
4. **Use MCP for reusable tools** - Create modular, testable tools with the mcp-sdk
5. **Monitor costs** - All responses include `total_cost_usd` field
6. **Secure by default** - Use minimal permissions and explicit tool allow lists

---

## Contributing

When adding new examples:

1. Focus on **one clear concept** per example
2. Use **modern API** (ClaudeClient, error::*, session::*)
3. Include **comprehensive documentation** in file comments
4. Add **error handling** patterns
5. Keep it **runnable** with minimal setup
6. Update this README with proper categorization

Prefer quality over quantity - each example should demonstrate something unique and valuable.
