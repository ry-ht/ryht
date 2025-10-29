//! Cortex MCP Integration
//!
//! This crate integrates the mcp-sdk framework with Cortex, providing
//! 30 production-ready MCP tools across three categories:
//! - Workspace Management (8 tools)
//! - Virtual Filesystem (12 tools)
//! - Code Navigation (10 tools)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use cortex_mcp::CortexMcpServer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create server with global configuration
//!     let server = CortexMcpServer::new().await?;
//!
//!     // Serve over stdio
//!     server.serve_stdio().await?;
//!     Ok(())
//! }
//! ```

pub mod server;
pub mod tools;
pub mod handlers;
pub mod types;
pub mod graph_algorithms;
pub mod context;

pub use server::{CortexMcpServer, CortexMcpServerBuilder};

/// Re-export commonly used types
pub mod prelude {
    pub use super::server::{CortexMcpServer, CortexMcpServerBuilder};
    pub use super::tools::*;
    pub use super::types::*;
    pub use super::context::CortexToolContext;
    // Re-export specific items from mcp_sdk to avoid conflicts
    pub use mcp_sdk::prelude::{McpServer, Tool, ToolError, ToolResult};
}
