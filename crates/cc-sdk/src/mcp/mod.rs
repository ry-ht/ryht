//! MCP (Model Context Protocol) integration module.
//!
//! This module provides integration with the mcp-sdk crate and adds
//! cc-sdk specific helpers for working with MCP servers.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::mcp::{self, McpServer, Tool};
//! use async_trait::async_trait;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an MCP server
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .version("1.0.0")
//!     .build()?;
//! # Ok(())
//! # }
//! ```

// Re-export the entire mcp-sdk crate
pub use mcp_sdk::*;

// Re-export commonly used types for convenience
pub use mcp_sdk::{
    error::{McpError, ToolError, ResourceError, TransportError as McpTransportError},
    protocol::{JsonRpcRequest, JsonRpcResponse, ServerCapabilities},
    server::{McpServer, ServerBuilder},
    tool::{Tool, ToolContext, ToolResult},
    transport::Transport,
    PROTOCOL_VERSION,
};

use crate::types::McpServerConfig;
use crate::result::Result;
use crate::error::Error;

/// Convert a cc-sdk McpServerConfig to an mcp-sdk server configuration.
///
/// This helper function bridges cc-sdk configuration types with mcp-sdk types.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::mcp::config_to_server_builder;
/// use cc_sdk::types::McpServerConfig;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let config = McpServerConfig::Stdio {
///     command: "node".to_string(),
///     args: Some(vec!["server.js".to_string()]),
///     env: None,
/// };
///
/// // Convert config to builder (note: not all config types are supported yet)
/// // let builder = config_to_server_builder(&config)?;
/// # Ok(())
/// # }
/// ```
pub fn config_to_server_builder(config: &McpServerConfig) -> Result<ServerBuilder> {
    match config {
        McpServerConfig::Sdk { name, .. } => {
            // For SDK servers, create a basic builder with the name
            Ok(McpServer::builder().name(name.clone()))
        }
        _ => {
            // Other config types (Stdio, Sse, Http) are external servers
            // that would be spawned as processes, not built with mcp-sdk
            Err(Error::Config(format!(
                "Cannot convert {:?} config to server builder. Use for external servers only.",
                config
            )))
        }
    }
}

/// Helper to create an in-process SDK MCP server configuration.
///
/// This wraps an mcp-sdk server instance for use with cc-sdk.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::mcp::{create_sdk_server_config, McpServer};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let server = McpServer::builder()
///     .name("my-server")
///     .version("1.0.0")
///     .build()?;
///
/// let config = create_sdk_server_config("my-server", Arc::new(server));
/// # Ok(())
/// # }
/// ```
pub fn create_sdk_server_config(
    name: impl Into<String>,
    server: std::sync::Arc<dyn std::any::Any + Send + Sync>,
) -> McpServerConfig {
    McpServerConfig::Sdk {
        name: name.into(),
        instance: server,
    }
}
