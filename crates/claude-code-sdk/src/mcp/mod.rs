//! MCP (Model Context Protocol) integration module.
//!
//! This module provides integration with the mcp-sdk crate and adds
//! cc-sdk specific helpers for working with MCP servers.
//!
//! # Features
//!
//! - **Error Conversion**: Automatic conversion between mcp-sdk and cc-sdk error types
//! - **Tool Registry**: Convenient wrapper around mcp-sdk's tool registry
//! - **Server Helpers**: Simplified server configuration and spawning
//! - **Middleware**: Pre-built middleware for common use cases
//!
//! # Examples
//!
//! ## Basic Server Setup
//!
//! ```no_run
//! use crate::cc::mcp::{self, McpServer, Tool, ToolRegistry};
//! use async_trait::async_trait;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an MCP server
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .version("1.0.0")
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Tool Registry
//!
//! ```no_run
//! use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
//! use crate::cc::error::Error;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! let registry = ToolRegistry::new();
//! // Register tools...
//! let tool_names = registry.list_tool_names().await;
//! println!("Available tools: {:?}", tool_names);
//! # Ok(())
//! # }
//! ```
//!
//! ## Error Conversion
//!
//! ```no_run
//! use crate::cc::mcp;
//! use crate::cc::error::Error;
//!
//! # fn example() -> cc_sdk::Result<()> {
//! // MCP errors automatically convert to cc-sdk errors
//! let mcp_error = mcp::error::ToolError::NotFound("my_tool".to_string());
//! let cc_error: Error = mcp_error.into();
//! # Ok(())
//! # }
//! ```

// Re-export the entire mcp-sdk crate
pub use mcp_sdk::*;

// Re-export commonly used types for convenience
pub use mcp_sdk::{
    error::{McpError, ToolError, ResourceError, TransportError as McpTransportError, MiddlewareError},
    protocol::{JsonRpcRequest, JsonRpcResponse, ServerCapabilities},
    server::{McpServer, ServerBuilder},
    tool::{Tool, ToolContext, ToolResult, ToolDefinition},
    transport::Transport,
    middleware::Middleware,
    PROTOCOL_VERSION,
};

use crate::cc::options::McpServerConfig;
use crate::cc::result::Result;
use crate::cc::error::Error;
use std::sync::Arc;

/// Convert a cc-sdk McpServerConfig to an mcp-sdk server configuration.
///
/// This helper function bridges cc-sdk configuration types with mcp-sdk types.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::mcp::config_to_server_builder;
/// use crate::cc::types::McpServerConfig;
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
/// use crate::cc::mcp::{create_sdk_server_config, McpServer};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let server = McpServer::builder()
///     .name("my-server")
///     .version("1.0.0")
///     .build();
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

// ============================================================================
// Error Conversion
// ============================================================================

/// Convert mcp-sdk errors to cc-sdk errors.
///
/// This implementation provides comprehensive error mapping from mcp-sdk's
/// error types to cc-sdk's error hierarchy, ensuring proper error context
/// is preserved across the boundary.
///
/// # Error Mapping
///
/// - `McpError::Transport` → `Error::Transport`
/// - `McpError::Tool` → `Error::Protocol` (with descriptive message)
/// - `McpError::Resource` → `Error::Protocol` (with descriptive message)
/// - `McpError::Middleware` → `Error::Protocol` (with descriptive message)
/// - `McpError::Config` → `Error::Config`
/// - `McpError::Protocol` → `Error::Protocol`
///
/// # Examples
///
/// ```
/// use crate::cc::mcp::error::ToolError;
/// use crate::cc::error::Error;
///
/// let mcp_error = mcp_sdk::error::McpError::Tool(
///     ToolError::NotFound("my_tool".to_string())
/// );
/// let cc_error: Error = mcp_error.into();
/// ```
impl From<McpError> for Error {
    fn from(err: McpError) -> Self {
        match err {
            McpError::Transport(transport_err) => {
                // Map mcp-sdk transport errors to cc-sdk transport errors
                match transport_err {
                    mcp_sdk::error::TransportError::Io(io_err) => {
                        Error::Transport(crate::cc::error::TransportError::Io(io_err))
                    }
                    mcp_sdk::error::TransportError::Closed => {
                        Error::Transport(crate::cc::error::TransportError::Closed)
                    }
                    mcp_sdk::error::TransportError::InvalidMessage(msg) => {
                        Error::Transport(crate::cc::error::TransportError::InvalidMessage {
                            reason: msg.clone(),
                            raw: msg,
                        })
                    }
                }
            }
            McpError::Tool(tool_err) => {
                // Map tool errors to protocol errors with descriptive messages
                Error::Protocol(format!("MCP tool error: {}", tool_err))
            }
            McpError::Resource(resource_err) => {
                // Map resource errors to protocol errors with descriptive messages
                Error::Protocol(format!("MCP resource error: {}", resource_err))
            }
            McpError::Middleware(middleware_err) => {
                // Map middleware errors to protocol errors with descriptive messages
                Error::Protocol(format!("MCP middleware error: {}", middleware_err))
            }
            McpError::Config(msg) => {
                // Config errors map directly
                Error::Config(msg)
            }
            McpError::Protocol(msg) => {
                // Protocol errors map directly
                Error::Protocol(msg)
            }
        }
    }
}

/// Convert mcp-sdk tool errors to cc-sdk errors.
///
/// This provides a more specific conversion for tool-specific errors.
impl From<ToolError> for Error {
    fn from(err: ToolError) -> Self {
        Error::Protocol(format!("MCP tool error: {}", err))
    }
}

/// Convert mcp-sdk resource errors to cc-sdk errors.
///
/// This provides a more specific conversion for resource-specific errors.
impl From<ResourceError> for Error {
    fn from(err: ResourceError) -> Self {
        Error::Protocol(format!("MCP resource error: {}", err))
    }
}

/// Convert mcp-sdk middleware errors to cc-sdk errors.
///
/// This provides a more specific conversion for middleware-specific errors.
impl From<MiddlewareError> for Error {
    fn from(err: MiddlewareError) -> Self {
        Error::Protocol(format!("MCP middleware error: {}", err))
    }
}

// ============================================================================
// Tool Registry Wrapper
// ============================================================================

/// Convenience wrapper around mcp-sdk's ToolRegistry.
///
/// This provides additional helper methods and a more ergonomic API
/// for working with tool registries in cc-sdk applications.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
/// use crate::cc::error::Error;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn name(&self) -> &str { "my_tool" }
///     fn input_schema(&self) -> Value { json!({}) }
///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
///         Ok(ToolResult::success_text("result"))
///     }
/// }
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let registry = ToolRegistry::new();
/// registry.register(MyTool).await?;
///
/// let tool_names = registry.list_tool_names().await;
/// assert_eq!(tool_names, vec!["my_tool"]);
/// # Ok(())
/// # }
/// ```
pub struct ToolRegistry {
    inner: mcp_sdk::tool::ToolRegistry,
}

impl ToolRegistry {
    /// Creates a new empty tool registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cc::mcp::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: mcp_sdk::tool::ToolRegistry::new(),
        }
    }

    /// Registers a tool in the registry.
    ///
    /// # Errors
    ///
    /// Returns an error if a tool with the same name is already registered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str { "my_tool" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// registry.register(MyTool).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register<T: Tool + 'static>(&self, tool: T) -> Result<()> {
        self.inner.register(tool).await.map_err(Error::from)
    }

    /// Registers a tool from an Arc.
    ///
    /// # Errors
    ///
    /// Returns an error if a tool with the same name is already registered.
    pub async fn register_arc(&self, tool: Arc<dyn Tool>) -> Result<()> {
        self.inner.register_arc(tool).await.map_err(Error::from)
    }

    /// Gets a tool by name.
    ///
    /// Returns `None` if the tool is not found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// # use async_trait::async_trait;
    /// # use serde_json::{json, Value};
    /// #
    /// # struct MyTool;
    /// #
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// registry.register(MyTool).await?;
    ///
    /// let tool = registry.get("my_tool").await;
    /// assert!(tool.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.inner.get(name).await
    }

    /// Checks if a tool with the given name exists.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// # use async_trait::async_trait;
    /// # use serde_json::{json, Value};
    /// #
    /// # struct MyTool;
    /// #
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// registry.register(MyTool).await?;
    ///
    /// assert!(registry.has("my_tool").await);
    /// assert!(!registry.has("nonexistent").await);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn has(&self, name: &str) -> bool {
        self.inner.has(name).await
    }

    /// Lists all registered tools as tool definitions.
    ///
    /// Returns a vector of `ToolDefinition` containing metadata for each tool.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// # use async_trait::async_trait;
    /// # use serde_json::{json, Value};
    /// #
    /// # struct Tool1;
    /// # struct Tool2;
    /// #
    /// # #[async_trait]
    /// # impl Tool for Tool1 {
    /// #     fn name(&self) -> &str { "tool1" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # #[async_trait]
    /// # impl Tool for Tool2 {
    /// #     fn name(&self) -> &str { "tool2" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// registry.register(Tool1).await?;
    /// registry.register(Tool2).await?;
    ///
    /// let definitions = registry.list().await;
    /// assert_eq!(definitions.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self) -> Vec<ToolDefinition> {
        self.inner.list().await
    }

    /// Lists all registered tool names.
    ///
    /// This is a convenience method that returns just the names without
    /// full metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// # use async_trait::async_trait;
    /// # use serde_json::{json, Value};
    /// #
    /// # struct Tool1;
    /// # struct Tool2;
    /// #
    /// # #[async_trait]
    /// # impl Tool for Tool1 {
    /// #     fn name(&self) -> &str { "tool1" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # #[async_trait]
    /// # impl Tool for Tool2 {
    /// #     fn name(&self) -> &str { "tool2" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// registry.register(Tool1).await?;
    /// registry.register(Tool2).await?;
    ///
    /// let names = registry.list_tool_names().await;
    /// assert_eq!(names.len(), 2);
    /// assert!(names.contains(&"tool1".to_string()));
    /// assert!(names.contains(&"tool2".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_tool_names(&self) -> Vec<String> {
        self.inner
            .list()
            .await
            .into_iter()
            .map(|def| def.name)
            .collect()
    }

    /// Returns the number of registered tools.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::cc::mcp::{ToolRegistry, Tool, ToolContext, ToolResult};
    /// # use async_trait::async_trait;
    /// # use serde_json::{json, Value};
    /// #
    /// # struct MyTool;
    /// #
    /// # #[async_trait]
    /// # impl Tool for MyTool {
    /// #     fn name(&self) -> &str { "my_tool" }
    /// #     fn input_schema(&self) -> Value { json!({}) }
    /// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, mcp_sdk::error::ToolError> {
    /// #         Ok(ToolResult::success_text(""))
    /// #     }
    /// # }
    /// #
    /// # async fn example() -> cc_sdk::Result<()> {
    /// let registry = ToolRegistry::new();
    /// assert_eq!(registry.count().await, 0);
    ///
    /// registry.register(MyTool).await?;
    /// assert_eq!(registry.count().await, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(&self) -> usize {
        self.inner.count().await
    }

    /// Returns the underlying mcp-sdk ToolRegistry.
    ///
    /// This allows access to the raw mcp-sdk registry if needed for
    /// advanced use cases.
    pub fn inner(&self) -> &mcp_sdk::tool::ToolRegistry {
        &self.inner
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ToolRegistry {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// ============================================================================
// Server Configuration Helpers
// ============================================================================

/// Validates an MCP server configuration.
///
/// This function performs validation checks on the configuration to ensure
/// it's properly formed before attempting to spawn a server.
///
/// # Errors
///
/// Returns an error if:
/// - Stdio config has an empty command
/// - SSE config has an invalid URL
/// - HTTP config has an invalid URL
///
/// # Examples
///
/// ```
/// use crate::cc::mcp::validate_mcp_config;
/// use crate::cc::types::McpServerConfig;
///
/// let config = McpServerConfig::Stdio {
///     command: "node".to_string(),
///     args: Some(vec!["server.js".to_string()]),
///     env: None,
/// };
///
/// validate_mcp_config(&config).unwrap();
/// ```
pub fn validate_mcp_config(config: &McpServerConfig) -> Result<()> {
    match config {
        McpServerConfig::Stdio { command, .. } => {
            if command.is_empty() {
                return Err(Error::Config(
                    "Stdio MCP server command cannot be empty".to_string(),
                ));
            }
            Ok(())
        }
        McpServerConfig::Sse { url, .. } => {
            if url.is_empty() {
                return Err(Error::Config("SSE MCP server URL cannot be empty".to_string()));
            }
            // Basic URL validation
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(Error::Config(format!(
                    "SSE MCP server URL must start with http:// or https://, got: {}",
                    url
                )));
            }
            Ok(())
        }
        McpServerConfig::Http { url, .. } => {
            if url.is_empty() {
                return Err(Error::Config(
                    "HTTP MCP server URL cannot be empty".to_string(),
                ));
            }
            // Basic URL validation
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(Error::Config(format!(
                    "HTTP MCP server URL must start with http:// or https://, got: {}",
                    url
                )));
            }
            Ok(())
        }
        McpServerConfig::Sdk { name, .. } => {
            if name.is_empty() {
                return Err(Error::Config("SDK MCP server name cannot be empty".to_string()));
            }
            Ok(())
        }
    }
}

// ============================================================================
// Middleware Utilities
// ============================================================================

/// Common middleware utilities for MCP servers.
///
/// This module provides pre-built middleware for typical use cases.
pub mod middleware {
    #![allow(unused_imports)]
    use super::*;

    /// Creates a logging middleware instance.
    ///
    /// This middleware logs all requests and responses passing through the server.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::mcp;
    ///
    /// let server = mcp::McpServer::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    ///
    /// // Note: Middleware registration would happen through server builder
    /// let logging_mw = mcp::middleware::logging();
    /// ```
    pub fn logging() -> mcp_sdk::middleware::LoggingMiddleware {
        mcp_sdk::middleware::LoggingMiddleware::new()
    }

    /// Creates a metrics middleware instance.
    ///
    /// This middleware collects metrics about request processing times
    /// and response counts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::mcp;
    ///
    /// let metrics_mw = mcp::middleware::metrics();
    /// ```
    pub fn metrics() -> mcp_sdk::middleware::MetricsMiddleware {
        mcp_sdk::middleware::MetricsMiddleware::new()
    }
}
