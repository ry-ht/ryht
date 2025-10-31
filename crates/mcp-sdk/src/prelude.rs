//! Convenient re-exports for common use cases.
//!
//! This module contains all the commonly used types and traits needed to build
//! an MCP server. Import everything from this module to get started quickly:
//!
//! ```rust
//! use mcp_server::prelude::*;
//! ```

// Core types
pub use crate::error::{McpError, Result, ToolError, ResourceError, TransportError};
pub use crate::protocol::{
    JsonRpcRequest, JsonRpcResponse, JsonRpcError,
    InitializeParams, InitializeResult,
    ServerCapabilities, ServerInfo, ClientInfo,
    ToolDefinition, CallToolParams, CallToolResult,
    ResourceDefinition, ResourceContent,
};

// Tool system
pub use crate::tool::{
    Tool, ToolContext, ToolResult, ToolRegistry,
};

// Resource system
pub use crate::resource::{
    Resource, ResourceContext, ResourceRegistry,
};

// Transport
pub use crate::transport::{Transport, StdioTransport};

#[cfg(feature = "http")]
pub use crate::transport::HttpTransport;

// Server
pub use crate::server::{McpServer, ServerBuilder, ServerConfig};

// Middleware
pub use crate::middleware::{
    Middleware, RequestContext, MiddlewareRegistry,
    LoggingMiddleware, MetricsMiddleware,
};

// Hooks
pub use crate::hooks::{Hook, HookEvent, HookRegistry};

// External re-exports for convenience
pub use async_trait::async_trait;
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
pub use schemars::JsonSchema;

// Macros (if/when we implement them)
// pub use mcp_server_macros::*;
