//! MCP Server Implementation
//!
//! This module provides the core server implementation for the Model Context Protocol (MCP).
//! It includes the main server struct, builder pattern for configuration, and request handling.
//!
//! # Overview
//!
//! The server module consists of:
//!
//! - **[`McpServer`]**: The main server struct that handles MCP protocol requests
//! - **[`ServerBuilder`]**: Fluent API for building and configuring servers
//! - **[`ServerConfig`]**: Server configuration including name, version, and protocol version
//!
//! # Architecture
//!
//! The MCP server follows a layered architecture:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │        McpServer (main)             │
//! ├─────────────────────────────────────┤
//! │  - handle_request()                 │
//! │  - handle_initialize()              │
//! │  - handle_tools_*()                 │
//! │  - handle_resources_*()             │
//! ├─────────────────────────────────────┤
//! │     Middleware Chain                │
//! │  (before/after request processing)  │
//! ├─────────────────────────────────────┤
//! │    Hook System                      │
//! │  (event emission at key points)     │
//! ├─────────────────────────────────────┤
//! │   Registries                        │
//! │  - ToolRegistry                     │
//! │  - ResourceRegistry                 │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Examples
//!
//! ## Basic Server
//!
//! ```rust,no_run
//! use mcp_server::server::McpServer;
//! use mcp_server::tool::{Tool, ToolContext, ToolResult};
//! use mcp_server::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{json, Value};
//!
//! struct EchoTool;
//!
//! #[async_trait]
//! impl Tool for EchoTool {
//!     fn name(&self) -> &str { "echo" }
//!     fn input_schema(&self) -> Value { json!({}) }
//!     async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
//!         Ok(ToolResult::success_text("echo"))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::builder()
//!         .name("my-server")
//!         .version("1.0.0")
//!         .tool(EchoTool)
//!         .build();
//!
//!     // Use server to handle requests
//! }
//! ```
//!
//! ## Server with Middleware and Hooks
//!
//! ```rust,no_run
//! use mcp_server::server::McpServer;
//! # use mcp_server::tool::{Tool, ToolContext, ToolResult};
//! # use mcp_server::error::ToolError;
//! # use async_trait::async_trait;
//! # use serde_json::{json, Value};
//! #
//! # struct MyTool;
//! # #[async_trait]
//! # impl Tool for MyTool {
//! #     fn name(&self) -> &str { "my_tool" }
//! #     fn input_schema(&self) -> Value { json!({}) }
//! #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
//! #         Ok(ToolResult::success_text(""))
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::builder()
//!         .name("full-featured-server")
//!         .version("1.0.0")
//!         .protocol_version("2025-03-26")
//!         .tool(MyTool)
//!         .build();
//! }
//! ```

pub mod builder;
pub mod config;
pub mod core;

// Re-export main types for convenience
pub use builder::ServerBuilder;
pub use config::ServerConfig;
pub use core::McpServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Ensure main types are accessible
        let config = ServerConfig::new("test", "1.0.0");
        assert_eq!(config.name(), "test");

        let _builder = ServerBuilder::new();
    }
}
