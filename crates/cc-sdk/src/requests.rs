//! SDK Control Protocol request and response types
//!
//! This module contains types for the SDK Control Protocol, which enables
//! bidirectional communication between the SDK and Claude Code CLI.
//!
//! # Control Protocol
//!
//! The SDK Control Protocol supports:
//! - **Interrupts** - Cancel ongoing operations
//! - **Permissions** - Check if tools can be used
//! - **Initialization** - Configure hooks and settings
//! - **Mode Changes** - Update permission modes
//! - **Model Selection** - Change the model being used
//! - **Hook Callbacks** - Execute registered hooks
//! - **MCP Messages** - Send messages to MCP servers
//!
//! # Request Types
//!
//! - [`SDKControlRequest`] - Main request enum
//! - [`SDKControlInterruptRequest`] - Interrupt request
//! - [`SDKControlPermissionRequest`] - Permission check request
//! - [`SDKControlInitializeRequest`] - Initialization request
//! - [`SDKControlSetPermissionModeRequest`] - Set permission mode
//! - [`SDKControlSetModelRequest`] - Set model
//! - [`SDKHookCallbackRequest`] - Hook callback request
//! - [`SDKControlMcpMessageRequest`] - MCP message request
//!
//! # Response Types
//!
//! - [`ControlRequest`] - Control request
//! - [`ControlResponse`] - Control response
//!
//! # Example
//!
//! ```rust
//! use cc_sdk::requests::SDKControlInterruptRequest;
//!
//! let interrupt = SDKControlInterruptRequest {
//!     subtype: "interrupt".to_string(),
//! };
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::permissions::PermissionUpdate;

/// SDK Control Protocol - Interrupt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInterruptRequest {
    /// Subtype
    pub subtype: String,  // "interrupt"
}

/// SDK Control Protocol - Permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlPermissionRequest {
    /// Subtype
    pub subtype: String,  // "can_use_tool"
    /// Tool name
    pub tool_name: String,
    /// Tool input
    pub input: serde_json::Value,
    /// Permission suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<PermissionUpdate>>,
    /// Blocked path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
}

/// SDK Control Protocol - Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInitializeRequest {
    /// Subtype
    pub subtype: String,  // "initialize"
    /// Hooks configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, serde_json::Value>>,
}

/// SDK Control Protocol - Set permission mode request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlSetPermissionModeRequest {
    /// Subtype
    pub subtype: String,  // "set_permission_mode"
    /// Permission mode
    pub mode: String,
}

/// SDK Control Protocol - Set model request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlSetModelRequest {
    /// Subtype
    pub subtype: String, // "set_model"
    /// Model to set (None to clear)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// SDK Hook callback request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKHookCallbackRequest {
    /// Subtype
    pub subtype: String,  // "hook_callback"
    /// Callback ID
    pub callback_id: String,
    /// Input data
    pub input: serde_json::Value,
    /// Tool use ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// SDK Control Protocol - MCP message request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SDKControlMcpMessageRequest {
    /// Subtype
    pub subtype: String,  // "mcp_message"
    /// MCP server name
    pub mcp_server_name: String,
    /// Message to send
    pub message: serde_json::Value,
}

/// SDK Control Protocol request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SDKControlRequest {
    /// Interrupt request
    #[serde(rename = "interrupt")]
    Interrupt(SDKControlInterruptRequest),
    /// Permission request
    #[serde(rename = "can_use_tool")]
    CanUseTool(SDKControlPermissionRequest),
    /// Initialize request
    #[serde(rename = "initialize")]
    Initialize(SDKControlInitializeRequest),
    /// Set permission mode
    #[serde(rename = "set_permission_mode")]
    SetPermissionMode(SDKControlSetPermissionModeRequest),
    /// Set model
    #[serde(rename = "set_model")]
    SetModel(SDKControlSetModelRequest),
    /// Hook callback
    #[serde(rename = "hook_callback")]
    HookCallback(SDKHookCallbackRequest),
    /// MCP message
    #[serde(rename = "mcp_message")]
    McpMessage(SDKControlMcpMessageRequest),
}

/// Control request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlRequest {
    /// Interrupt the current operation
    Interrupt {
        /// Request ID
        request_id: String,
    },
}

/// Control response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlResponse {
    /// Interrupt acknowledged
    InterruptAck {
        /// Request ID
        request_id: String,
        /// Whether interrupt was successful
        success: bool,
    },
}
