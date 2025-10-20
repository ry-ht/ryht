//! Tool system for the MCP server.
//!
//! This module provides the core abstractions for defining and managing tools
//! in an MCP server. Tools are the primary way for AI models to interact with
//! external systems through the Model Context Protocol.
//!
//! # Overview
//!
//! The tool system consists of several key components:
//!
//! - [`Tool`]: The core trait that all tools must implement
//! - [`ToolRegistry`]: Thread-safe storage for registered tools
//! - [`ToolContext`]: Contextual information passed to tools during execution
//! - [`ToolResult`]: The result type returned by tool execution
//! - [`ToolContent`]: Different types of content that tools can return
//!
//! # Quick Start
//!
//! ## Defining a Simple Tool
//!
//! ```
//! use mcp_server::tool::{Tool, ToolContext, ToolResult};
//! use mcp_server::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{json, Value};
//!
//! struct EchoTool;
//!
//! #[async_trait]
//! impl Tool for EchoTool {
//!     fn name(&self) -> &str {
//!         "echo"
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("Echoes the input message back")
//!     }
//!
//!     fn input_schema(&self) -> Value {
//!         json!({
//!             "type": "object",
//!             "properties": {
//!                 "message": {
//!                     "type": "string",
//!                     "description": "The message to echo"
//!                 }
//!             },
//!             "required": ["message"]
//!         })
//!     }
//!
//!     async fn execute(
//!         &self,
//!         input: Value,
//!         _context: &ToolContext,
//!     ) -> Result<ToolResult, ToolError> {
//!         let message = input["message"]
//!             .as_str()
//!             .ok_or_else(|| ToolError::ExecutionFailed(
//!                 "message is required".to_string()
//!             ))?;
//!
//!         Ok(ToolResult::success_text(message))
//!     }
//! }
//! ```
//!
//! ## Using the Tool Registry
//!
//! ```
//! use mcp_server::tool::{Tool, ToolRegistry, ToolContext, ToolResult};
//! use mcp_server::error::ToolError;
//! use async_trait::async_trait;
//! use serde_json::{json, Value};
//!
//! struct MyTool;
//!
//! #[async_trait]
//! impl Tool for MyTool {
//!     fn name(&self) -> &str { "my_tool" }
//!     fn input_schema(&self) -> Value { json!({}) }
//!     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
//!         Ok(ToolResult::success_text("Success"))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let registry = ToolRegistry::new();
//!
//!     // Register tools
//!     registry.register(MyTool).await.unwrap();
//!
//!     // Check if tool exists
//!     assert!(registry.has("my_tool").await);
//!
//!     // Get tool for execution
//!     let tool = registry.get("my_tool").await.unwrap();
//!     let context = ToolContext::new();
//!     let result = tool.execute(json!({}), &context).await.unwrap();
//!
//!     assert!(result.is_success());
//! }
//! ```
//!
//! ## Working with Tool Context
//!
//! ```
//! use mcp_server::tool::ToolContext;
//! use serde_json::json;
//!
//! let context = ToolContext::builder()
//!     .session_id("session-123")
//!     .client_info("my-client", "1.0.0")
//!     .metadata("user_id", json!(42))
//!     .build();
//!
//! assert_eq!(context.session_id(), Some("session-123"));
//! assert_eq!(context.get_metadata("user_id"), Some(&json!(42)));
//! ```
//!
//! ## Tool Results and Content
//!
//! ```
//! use mcp_server::tool::{ToolResult, ToolContent};
//! use serde_json::json;
//!
//! // Success with text
//! let result = ToolResult::success_text("Operation completed");
//!
//! // Success with JSON
//! let result = ToolResult::success_json(json!({
//!     "status": "ok",
//!     "count": 42
//! }));
//!
//! // Success with multiple content items
//! let result = ToolResult::success(vec![
//!     ToolContent::text("Result:"),
//!     ToolContent::image("base64data", "image/png"),
//!     ToolContent::resource("file:///path/to/file.txt"),
//! ]);
//!
//! // Error result
//! let result = ToolResult::error("Something went wrong");
//! assert!(result.is_error());
//! ```

mod context;
mod registry;
mod result;
mod traits;

pub use context::{ToolContext, ToolContextBuilder};
pub use registry::ToolRegistry;
pub use result::{ToolContent, ToolResult};
pub use traits::Tool;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool definition for listing tools.
///
/// This struct represents the metadata about a tool that is sent to clients
/// when they request the list of available tools.
///
/// # Examples
///
/// ```
/// use mcp_server::tool::ToolDefinition;
/// use serde_json::json;
///
/// let def = ToolDefinition {
///     name: "my_tool".to_string(),
///     description: Some("A useful tool".to_string()),
///     input_schema: json!({
///         "type": "object",
///         "properties": {
///             "input": { "type": "string" }
///         }
///     }),
///     output_schema: Some(json!({
///         "type": "object",
///         "properties": {
///             "output": { "type": "string" }
///         }
///     })),
/// };
///
/// assert_eq!(def.name, "my_tool");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// The unique name of the tool
    pub name: String,

    /// Optional description of what the tool does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON schema for input validation
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,

    /// Optional JSON schema for output
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string" }
                }
            })),
        };

        let json = serde_json::to_value(&def).unwrap();
        assert_eq!(json["name"], "test_tool");
        assert_eq!(json["description"], "A test tool");
        assert_eq!(json["inputSchema"]["type"], "object");
        assert_eq!(json["outputSchema"]["type"], "object");
    }

    #[test]
    fn test_tool_definition_without_optional_fields() {
        let def = ToolDefinition {
            name: "minimal_tool".to_string(),
            description: None,
            input_schema: json!({}),
            output_schema: None,
        };

        let json = serde_json::to_value(&def).unwrap();
        assert_eq!(json["name"], "minimal_tool");
        assert!(json["description"].is_null());
        assert!(json["outputSchema"].is_null());
    }

    #[test]
    fn test_tool_definition_equality() {
        let def1 = ToolDefinition {
            name: "tool".to_string(),
            description: Some("desc".to_string()),
            input_schema: json!({}),
            output_schema: None,
        };

        let def2 = ToolDefinition {
            name: "tool".to_string(),
            description: Some("desc".to_string()),
            input_schema: json!({}),
            output_schema: None,
        };

        assert_eq!(def1, def2);
    }

    #[test]
    fn test_tool_definition_deserialization() {
        let json = json!({
            "name": "deserialize_test",
            "description": "Test deserialization",
            "inputSchema": {
                "type": "object"
            }
        });

        let def: ToolDefinition = serde_json::from_value(json).unwrap();
        assert_eq!(def.name, "deserialize_test");
        assert_eq!(def.description, Some("Test deserialization".to_string()));
        assert!(def.output_schema.is_none());
    }
}
