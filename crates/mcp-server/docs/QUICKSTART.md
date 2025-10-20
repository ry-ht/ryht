# MCP Server Framework - Quick Start Guide

Get started with the MCP Server Framework in under 5 minutes!

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
mcp-server = "0.1.0"
tokio = { version = "1.48", features = ["full"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "1.0"
```

## Your First Server

Create `src/main.rs`:

```rust
use mcp_server::prelude::*;

// Define a simple tool
struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("Echo back the input message")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo"
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
            .ok_or_else(|| ToolError::InvalidInput(
                serde_json::Error::custom("missing message")
            ))?;

        Ok(ToolResult::success_text(message.to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the server
    let server = McpServer::builder()
        .name("echo-server")
        .version("1.0.0")
        .tool(EchoTool)
        .build()?;

    // Create stdio transport
    let transport = StdioTransport::new();

    // Serve forever
    server.serve(transport).await?;

    Ok(())
}
```

## Run It

```bash
cargo run
```

## Test It

In another terminal, send JSON-RPC requests via stdin:

```bash
# Initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | cargo run

# List tools
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | cargo run

# Call echo tool
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello!"}}}' | cargo run
```

## Next Steps

- 📖 Read the [LLM Guide](docs/LLM_GUIDE.md) for complete documentation
- 🔧 Check [examples/](examples/) for more complex servers
- 🚀 See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for technical details
- 🧪 Review [integration tests](tests/) for usage patterns

## Common Patterns

### Adding Multiple Tools

```rust
let server = McpServer::builder()
    .name("my-server")
    .tool(EchoTool)
    .tool(CalculatorTool)
    .tool(WeatherTool)
    .build()?;
```

### Adding Middleware

```rust
use mcp_server::middleware::LoggingMiddleware;

let server = McpServer::builder()
    .name("my-server")
    .tool(EchoTool)
    .middleware(LoggingMiddleware::new())
    .build()?;
```

### Using HTTP Transport

```toml
[dependencies]
mcp-server = { version = "0.1.0", features = ["http"] }
```

```rust
let transport = HttpTransport::new("127.0.0.1:3000".parse()?);
server.serve(transport).await?;
```

## Getting Help

- 📚 [Complete Documentation](docs/)
- 💬 [GitHub Issues](https://github.com/omnitron-dev/meridian/issues)
- 🔍 [Examples](examples/)

Happy building! 🚀
