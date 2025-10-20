//! # MCP Server Framework
//!
//! A universal, type-safe, ergonomic Rust crate for building MCP (Model Context Protocol) servers.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mcp_server::prelude::*;
//!
//! #[derive(McpTool)]
//! #[tool(name = "echo", description = "Echo a message")]
//! struct EchoTool;
//!
//! #[derive(serde::Deserialize, schemars::JsonSchema)]
//! struct EchoInput {
//!     message: String,
//! }
//!
//! #[derive(serde::Serialize, schemars::JsonSchema)]
//! struct EchoOutput {
//!     message: String,
//! }
//!
//! #[async_trait::async_trait]
//! impl ToolHandler for EchoTool {
//!     type Input = EchoInput;
//!     type Output = EchoOutput;
//!
//!     async fn handle(&self, input: Self::Input) -> Result<Self::Output, ToolError> {
//!         Ok(EchoOutput { message: input.message })
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     McpServer::builder()
//!         .name("simple-server")
//!         .version("1.0.0")
//!         .tool(EchoTool)
//!         .build()?
//!         .serve(StdioTransport::new())
//!         .await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Type Safety**: Compile-time validation of tool signatures
//! - **Zero Boilerplate**: Derive macros for common patterns
//! - **Async by Default**: Built on Tokio for excellent performance
//! - **Multiple Transports**: stdio, HTTP/SSE, WebSocket
//! - **Testability**: Mock implementations and test harness included
//! - **100% MCP Spec Compliant**: Implements MCP protocol 2025-03-26
//!
//! ## Modules
//!
//! - [`protocol`]: MCP protocol types (JSON-RPC, requests, responses)
//! - [`tool`]: Tool registration and execution
//! - [`resource`]: Resource management
//! - [`transport`]: Transport layer abstractions
//! - [`middleware`]: Request/response middleware
//! - [`hooks`]: Event hook system
//! - [`server`]: Core MCP server implementation
//! - [`error`]: Error types and conversions

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::result_large_err)]

pub mod error;
pub mod hooks;
pub mod middleware;
pub mod prelude;
pub mod protocol;
pub mod resource;
pub mod server;
pub mod tool;
pub mod transport;

// Re-export commonly used types at crate root
pub use error::{McpError, ToolError, ResourceError, TransportError};
pub use protocol::{JsonRpcRequest, JsonRpcResponse, ServerCapabilities};
pub use server::{McpServer, ServerBuilder};
pub use tool::{Tool, ToolContext, ToolResult};
pub use transport::Transport;

/// Current MCP protocol version supported
pub const PROTOCOL_VERSION: &str = "2025-03-26";

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
