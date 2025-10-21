# Modern ClaudeClient Enhancements

This document describes the advanced features added to the `ClaudeClient` API, inspired by best practices from the Claude CLI and mcp-sdk patterns.

## Overview

The modern `ClaudeClient` has been enhanced with advanced capabilities while maintaining its type-safe, builder-based API. All enhancements follow the principle of making common operations easy while keeping complex operations possible.

## Features Added

### 1. Model Fallback Support

**Feature**: Configure multiple models with automatic fallback when the primary model is unavailable.

**Motivation**: Ensures high availability by automatically trying alternative models if the primary fails.

**API**:
```rust
use cc_sdk::{ClaudeClient, core::ModelId};

let client = ClaudeClient::builder()
    .discover_binary().await?
    .models(vec![
        ModelId::from("claude-sonnet-4-5-20250929"),  // Primary
        ModelId::from("claude-opus-4-5-20250929"),    // Fallback 1
        ModelId::from("claude-haiku-4-0-20240307"),   // Fallback 2
    ])
    .configure()
    .connect().await?
    .build()?;
```

**Implementation Details**:
- Primary model is set as the main `model` option
- Fallback models are stored in `extra_args` as `fallback-models`
- The CLI handles failover logic automatically

### 2. Tool Filtering (Disallowed Tools)

**Feature**: Explicitly disallow specific tools, even if they would otherwise be allowed.

**Motivation**: Provides fine-grained control over tool permissions, allowing "deny-list" patterns.

**API**:
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .allowed_tools(vec!["Bash".to_string(), "Read".to_string(), "Write".to_string()])
    .disallow_tool("Bash")       // Explicitly deny Bash
    .disallow_tool("Delete")     // Block Delete entirely
    .configure()
    .connect().await?
    .build()?;
```

**Key Points**:
- `disallowed_tools` takes precedence over `allowed_tools`
- Useful for security policies and safety constraints
- Both single-tool (`disallow_tool`) and batch (`disallowed_tools`) methods available

### 3. Session Forking

**Feature**: Create a new conversation branch from a resumed session point.

**Motivation**: Enables exploration of alternative approaches without modifying the original conversation history.

**API**:
```rust
use cc_sdk::core::SessionId;

let client = ClaudeClient::builder()
    .discover_binary().await?
    .resume_session(SessionId::new("previous-session-abc123"))
    .fork_session(true)  // Create a new branch
    .configure()
    .connect().await?
    .build()?;
```

**Use Cases**:
- Try different approaches from the same starting point
- Create conversation branches for A/B testing
- Preserve original session while experimenting

### 4. MCP Server Configuration Helpers

**Feature**: Simplified API for configuring Model Context Protocol (MCP) servers.

**Motivation**: Makes it easier to integrate external tools and resources through MCP.

**API**:
```rust
use cc_sdk::types::McpServerConfig;
use std::collections::HashMap;

// Full configuration
let client = ClaudeClient::builder()
    .discover_binary().await?
    .add_mcp_server(
        "filesystem",
        McpServerConfig::Stdio {
            command: "npx".to_string(),
            args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
            env: None,
        }
    )
    .configure()
    .connect().await?
    .build()?;

// Convenience helper for stdio servers
let client = ClaudeClient::builder()
    .discover_binary().await?
    .add_mcp_stdio_server(
        "filesystem",
        "npx",
        vec!["-y", "@modelcontextprotocol/server-filesystem"]
    )
    .mcp_tools(vec!["filesystem__read".to_string(), "filesystem__write".to_string()])
    .configure()
    .connect().await?
    .build()?;
```

**Server Types Supported**:
- Stdio (subprocess communication)
- SSE (Server-Sent Events)
- HTTP
- SDK (in-process)

### 5. Advanced Configuration Methods

**Feature**: Fine-grained control over Claude's behavior and output.

**API**:

#### Max Output Tokens
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .max_output_tokens(8000)  // Limit response length (1-32000)
    .configure()
    .connect().await?
    .build()?;
```

#### Max Turns
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .max_turns(20)  // Limit conversation rounds
    .configure()
    .connect().await?
    .build()?;
```

#### System Prompt
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .system_prompt("You are a helpful coding assistant specialized in Rust.")
    .configure()
    .connect().await?
    .build()?;
```

#### Environment Variables
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .add_env("CUSTOM_VAR", "value")
    .add_env("DEBUG", "true")
    .configure()
    .connect().await?
    .build()?;
```

#### Additional Directories
```rust
use std::path::PathBuf;

let client = ClaudeClient::builder()
    .discover_binary().await?
    .working_directory("/main/project")
    .add_directory(PathBuf::from("/shared/libraries"))
    .add_directory(PathBuf::from("/documentation"))
    .configure()
    .connect().await?
    .build()?;
```

#### Partial Messages
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .include_partial_messages(true)  // Stream incremental updates
    .configure()
    .connect().await?
    .build()?;
```

### 6. Session and Conversation Management

**Feature**: Helper methods for managing sessions and conversation history.

**API**:

#### Continue Conversation
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .continue_conversation(true)  // Resume most recent session
    .configure()
    .connect().await?
    .build()?;
```

#### Resume Specific Session
```rust
let client = ClaudeClient::builder()
    .discover_binary().await?
    .resume_session(SessionId::new("session-abc123"))
    .configure()
    .connect().await?
    .build()?;
```

#### List Project Sessions
```rust
let sessions = client.list_project_sessions().await?;
for session in sessions {
    println!("Session: {:?}", session.id);
}
```

#### Get Conversation History
```rust
let history = client.get_history().await?;
for msg in history {
    println!("{:?}", msg);
}
```

### 7. Dynamic Permission Management

**Feature**: Update permission mode without reconnecting.

**API**:
```rust
use cc_sdk::types::PermissionMode;

// Change permission mode mid-session
client.set_permission_mode(PermissionMode::AcceptEdits).await?;

// Later, switch to more restrictive mode
client.set_permission_mode(PermissionMode::Default).await?;
```

**Permission Modes**:
- `Default` - CLI prompts for dangerous tools
- `AcceptEdits` - Auto-accept file edits
- `Plan` - Planning mode
- `BypassPermissions` - Allow all (use with caution)

### 8. Client Introspection

**Feature**: Query client state and configuration.

**API**:
```rust
// Check connection status
if client.is_connected() {
    println!("Connected!");
}

// Get session ID
let session_id = client.session_id();

// Get current model
if let Some(model) = client.model() {
    println!("Using model: {}", model);
}

// Get binary path
if let Some(path) = client.binary_path() {
    println!("Claude binary: {:?}", path);
}

// Get full configuration
let options = client.options();
println!("Permission mode: {:?}", options.permission_mode);
```

## Design Patterns Applied

### 1. Builder Pattern (from mcp-sdk)
- Fluent API for method chaining
- Type-safe state transitions
- Clear configuration intent

### 2. Type-State Pattern
- Compile-time safety for state transitions
- Prevents invalid operations (e.g., sending without connecting)
- Clear progression: NoBinary → WithBinary → Configured → Connected

### 3. Middleware-Inspired Context
- Environment variables for customization
- Hook support for intercepting behavior
- MCP integration for extensibility

### 4. Smart Defaults
- Sensible defaults for all configuration
- Progressive disclosure (simple things simple, complex things possible)
- Validation (e.g., token limits clamped to valid range)

## Complete Example

```rust
use cc_sdk::{ClaudeClient, core::ModelId};
use cc_sdk::types::{PermissionMode, McpServerConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use futures::StreamExt;

#[tokio::main]
async fn main() -> cc_sdk::Result<()> {
    // Build fully-configured client
    let client = ClaudeClient::builder()
        // Binary discovery
        .discover_binary().await?

        // Model configuration with fallback
        .models(vec![
            ModelId::from("claude-sonnet-4-5-20250929"),
            ModelId::from("claude-opus-4-5-20250929"),
        ])

        // Tool filtering
        .allowed_tools(vec!["Bash".to_string(), "Read".to_string(), "Write".to_string()])
        .disallow_tool("Delete")

        // Permission and limits
        .permission_mode(PermissionMode::AcceptEdits)
        .max_output_tokens(8000)
        .max_turns(20)

        // MCP integration
        .add_mcp_stdio_server(
            "filesystem",
            "npx",
            vec!["-y", "@modelcontextprotocol/server-filesystem"]
        )
        .mcp_tools(vec!["filesystem__read".to_string()])

        // Working directories
        .working_directory("/main/project")
        .add_directory(PathBuf::from("/shared/libs"))

        // Environment and customization
        .add_env("DEBUG", "true")
        .include_partial_messages(true)
        .system_prompt("You are a helpful Rust assistant.")

        // Session management
        .continue_conversation(true)

        // Connect and build
        .configure()
        .connect().await?
        .build()?;

    // Use the client
    let mut stream = client.send("Help me refactor this code").await?;
    while let Some(msg) = stream.next().await {
        println!("{:?}", msg?);
    }

    // Dynamic permission update
    client.set_permission_mode(PermissionMode::Default).await?;

    // Get session info
    let sessions = client.list_project_sessions().await?;
    println!("Found {} sessions", sessions.len());

    // Clean disconnect
    client.disconnect().await?;

    Ok(())
}
```

## Testing

All features are covered by comprehensive unit tests in `crates/cc-sdk/src/client/modern.rs`:

- Model fallback configuration
- Tool filtering (allowed/disallowed)
- Session forking
- MCP server configuration
- Advanced configuration options
- Environment variables
- Directory management
- Permission modes
- Builder fluency
- Token limit clamping

Run tests with:
```bash
cargo test --lib client::modern::tests
```

## Backward Compatibility

All enhancements are additive and maintain full backward compatibility:
- Existing code continues to work unchanged
- New methods are opt-in
- Default behaviors preserved
- No breaking changes to public API

## Future Enhancements

Potential future additions (not yet implemented):
- Middleware hooks for request/response interception
- Retry policies with exponential backoff
- Connection pooling for multiple concurrent sessions
- Advanced streaming with backpressure
- Custom transport implementations
- Plugin system for extended functionality
