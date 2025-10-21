//! Type definitions for MCP protocol.

use serde::{Deserialize, Serialize};

/// MCP protocol version
pub const MCP_VERSION: &str = "1.0";

/// Standard MCP error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

impl McpError {
    pub fn new(code: i32, message: String) -> Self {
        Self { code, message }
    }

    pub fn internal_error(message: String) -> Self {
        Self::new(-32603, message)
    }

    pub fn invalid_params(message: String) -> Self {
        Self::new(-32602, message)
    }

    pub fn method_not_found(message: String) -> Self {
        Self::new(-32601, message)
    }
}
