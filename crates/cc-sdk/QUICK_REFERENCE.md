# ClaudeClient Quick Reference

Quick reference for the enhanced modern `ClaudeClient` API.

## Basic Setup

```rust
use cc_sdk::{ClaudeClient, core::ModelId};
use cc_sdk::types::PermissionMode;
use futures::StreamExt;

let client = ClaudeClient::builder()
    .discover_binary().await?
    .model(ModelId::from("claude-sonnet-4-5-20250929"))
    .permission_mode(PermissionMode::AcceptEdits)
    .configure()
    .connect().await?
    .build()?;

let mut stream = client.send("Hello!").await?;
while let Some(msg) = stream.next().await {
    println!("{:?}", msg?);
}

client.disconnect().await?;
```

## Builder Methods (WithBinary State)

### Model Configuration
| Method | Description | Example |
|--------|-------------|---------|
| `.model(id)` | Set single model | `.model(ModelId::from("claude-sonnet-4-5"))` |
| `.models(vec)` | Model fallback | `.models(vec![primary, fallback])` |

### Permission & Tools
| Method | Description | Example |
|--------|-------------|---------|
| `.permission_mode(mode)` | Set permission mode | `.permission_mode(PermissionMode::AcceptEdits)` |
| `.add_allowed_tool(name)` | Allow single tool | `.add_allowed_tool("Bash")` |
| `.allowed_tools(vec)` | Set allowed tools | `.allowed_tools(vec!["Bash", "Read"])` |
| `.disallow_tool(name)` | Block single tool | `.disallow_tool("Delete")` |
| `.disallowed_tools(vec)` | Set disallowed tools | `.disallowed_tools(vec!["Bash", "Write"])` |

### Output & Limits
| Method | Description | Example |
|--------|-------------|---------|
| `.max_output_tokens(n)` | Limit response length (1-32000) | `.max_output_tokens(8000)` |
| `.max_turns(n)` | Limit conversation rounds | `.max_turns(20)` |
| `.system_prompt(s)` | Set system prompt | `.system_prompt("You are helpful")` |

### Session Management
| Method | Description | Example |
|--------|-------------|---------|
| `.working_directory(path)` | Set working directory | `.working_directory("/project")` |
| `.continue_conversation(bool)` | Continue last session | `.continue_conversation(true)` |
| `.resume_session(id)` | Resume specific session | `.resume_session(SessionId::new("abc"))` |
| `.fork_session(bool)` | Fork on resume | `.fork_session(true)` |

### MCP Integration
| Method | Description | Example |
|--------|-------------|---------|
| `.add_mcp_server(name, cfg)` | Add MCP server | `.add_mcp_server("fs", config)` |
| `.mcp_servers(map)` | Set all MCP servers | `.mcp_servers(servers)` |
| `.add_mcp_stdio_server(n, c, a)` | Add stdio MCP server | `.add_mcp_stdio_server("fs", "npx", args)` |
| `.mcp_tools(vec)` | Enable MCP tools | `.mcp_tools(vec!["fs__read"])` |

### Environment & Context
| Method | Description | Example |
|--------|-------------|---------|
| `.add_env(key, val)` | Add environment variable | `.add_env("DEBUG", "true")` |
| `.env(map)` | Set all env vars | `.env(env_map)` |
| `.add_directory(path)` | Add context directory | `.add_directory(PathBuf::from("/lib"))` |

### Streaming
| Method | Description | Example |
|--------|-------------|---------|
| `.include_partial_messages(bool)` | Stream incremental updates | `.include_partial_messages(true)` |

### State Transition
| Method | Description | Example |
|--------|-------------|---------|
| `.configure()` | Move to Configured state | `.configure()` |

## Builder Methods (Configured State)

| Method | Description | Example |
|--------|-------------|---------|
| `.connect()` | Connect and move to Connected | `.connect().await?` |

## Builder Methods (Connected State)

| Method | Description | Example |
|--------|-------------|---------|
| `.build()` | Build client | `.build()?` |
| `.with_prompt(s)` | Build with initial prompt | `.with_prompt("Hello")? ` |

## Runtime Methods (Connected Client)

### Messaging
| Method | Description | Returns |
|--------|-------------|---------|
| `.send(msg)` | Send message to Claude | `MessageStream` |
| `.send_with_files(msg, files)` | Send with attached files | `MessageStream` |
| `.interrupt()` | Interrupt current operation | `Result<()>` |

### Permission Management
| Method | Description | Returns |
|--------|-------------|---------|
| `.set_permission_mode(mode)` | Update permission mode | `Result<()>` |

### Session & History
| Method | Description | Returns |
|--------|-------------|---------|
| `.list_project_sessions()` | List all sessions | `Result<Vec<Session>>` |
| `.get_history()` | Get conversation history | `Result<Vec<Message>>` |

### Introspection
| Method | Description | Returns |
|--------|-------------|---------|
| `.is_connected()` | Check connection status | `bool` |
| `.session_id()` | Get current session ID | `&SessionId` |
| `.model()` | Get current model | `Option<ModelId>` |
| `.binary_path()` | Get Claude binary path | `Option<&BinaryPath>` |
| `.options()` | Get full configuration | `&ClaudeCodeOptions` |

### Connection
| Method | Description | Returns |
|--------|-------------|---------|
| `.disconnect()` | Disconnect (→ Disconnected) | `Result<ClaudeClient<Disconnected>>` |

## Permission Modes

| Mode | Description |
|------|-------------|
| `PermissionMode::Default` | CLI prompts for dangerous tools |
| `PermissionMode::AcceptEdits` | Auto-accept file edits |
| `PermissionMode::Plan` | Planning mode |
| `PermissionMode::BypassPermissions` | Allow all (use with caution) |

## MCP Server Types

```rust
use cc_sdk::types::McpServerConfig;

// Stdio
McpServerConfig::Stdio {
    command: "npx".to_string(),
    args: Some(vec!["-y".to_string(), "package-name".to_string()]),
    env: Some(env_map),
}

// SSE
McpServerConfig::Sse {
    url: "http://localhost:3000".to_string(),
    headers: Some(headers),
}

// HTTP
McpServerConfig::Http {
    url: "http://localhost:3000".to_string(),
    headers: Some(headers),
}

// SDK (in-process)
McpServerConfig::Sdk {
    name: "server-name".to_string(),
    instance: Arc::new(server),
}
```

## Common Patterns

### High Availability
```rust
.models(vec![
    ModelId::from("claude-sonnet-4-5"),
    ModelId::from("claude-opus-4-5"),
])
```

### Secure Configuration
```rust
.allowed_tools(vec!["Read".to_string()])
.disallow_tool("Delete")
.permission_mode(PermissionMode::Default)
```

### Session Forking
```rust
.resume_session(previous_id)
.fork_session(true)
```

### MCP Integration (Stdio)
```rust
.add_mcp_stdio_server(
    "filesystem",
    "npx",
    vec!["-y", "@modelcontextprotocol/server-filesystem"]
)
.mcp_tools(vec!["filesystem__read".to_string()])
```

### Dynamic Permission Update
```rust
// During runtime
client.set_permission_mode(PermissionMode::AcceptEdits).await?;
```

### Get Session History
```rust
let history = client.get_history().await?;
for msg in history {
    match msg {
        Message::User(u) => println!("User: {:?}", u.content),
        Message::Assistant(a) => println!("Assistant: {:?}", a.content),
        _ => {}
    }
}
```

## Type States

```
NoBinary → WithBinary → Configured → Connected → Disconnected
    ↓          ↓            ↓            ↓            ↓
 discover   model()    configure()   send()     reconnect()
 binary()   tools()    connect()     interrupt()
            env()                    get_history()
```

## Error Handling

```rust
use cc_sdk::error::Error;

match client.send("Hello").await {
    Ok(stream) => { /* handle stream */ },
    Err(Error::Client(e)) => { /* client error */ },
    Err(Error::Transport(e)) => { /* transport error */ },
    Err(Error::Session(e)) => { /* session error */ },
    Err(e) => { /* other error */ },
}
```

## Testing

```bash
# Run all modern client tests
cargo test --lib client::modern::tests

# Run specific test
cargo test --lib client::modern::tests::test_model_fallback_configuration
```

## See Also

- **Detailed Documentation**: `MODERN_CLIENT_ENHANCEMENTS.md`
- **Enhancement Report**: `../ENHANCEMENT_REPORT.md`
- **Module Docs**: `src/client/modern.rs`
