//! Basic usage example for Cortex MCP Server
//!
//! This example demonstrates how to:
//! 1. Create and configure the MCP server
//! 2. Run it with stdio transport
//! 3. Handle graceful shutdown

use cortex_mcp::CortexMcpServer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Cortex MCP Server example");

    // Create server with global configuration
    // This will load from ~/.ryht/cortex/config.toml or create defaults
    let server = CortexMcpServer::new().await?;

    tracing::info!("Server initialized successfully");
    tracing::info!("Registered 30 MCP tools");
    tracing::info!("Starting stdio transport...");

    // Serve over stdio
    // This will block until the client disconnects
    server.serve_stdio().await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}
