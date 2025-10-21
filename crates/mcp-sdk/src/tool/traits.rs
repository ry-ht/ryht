//! Tool trait definition.
//!
//! This module defines the core `Tool` trait that all MCP tools must implement.

use super::{ToolContext, ToolResult};
use crate::error::ToolError;
use async_trait::async_trait;
use serde_json::Value;

/// A tool that can be called by MCP clients.
///
/// Tools are the primary way for AI models to interact with external systems
/// through the MCP protocol. Each tool has a name, optional description, and
/// JSON schemas for input validation.
///
/// # Examples
///
/// ## Basic Tool Implementation
///
/// ```
/// use mcp_server::tool::{Tool, ToolContext, ToolResult};
/// use mcp_server::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct EchoTool;
///
/// #[async_trait]
/// impl Tool for EchoTool {
///     fn name(&self) -> &str {
///         "echo"
///     }
///
///     fn description(&self) -> Option<&str> {
///         Some("Echoes the input message back")
///     }
///
///     fn input_schema(&self) -> Value {
///         json!({
///             "type": "object",
///             "properties": {
///                 "message": {
///                     "type": "string",
///                     "description": "The message to echo"
///                 }
///             },
///             "required": ["message"]
///         })
///     }
///
///     async fn execute(
///         &self,
///         input: Value,
///         _context: &ToolContext,
///     ) -> Result<ToolResult, ToolError> {
///         let message = input["message"]
///             .as_str()
///             .ok_or_else(|| ToolError::InvalidInput(
///                 serde_json::from_str("\"message is required\"").unwrap()
///             ))?;
///
///         Ok(ToolResult::success_text(message))
///     }
/// }
/// ```
///
/// ## Tool with Output Schema
///
/// ```
/// use mcp_server::tool::{Tool, ToolContext, ToolResult};
/// use mcp_server::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct AddTool;
///
/// #[async_trait]
/// impl Tool for AddTool {
///     fn name(&self) -> &str {
///         "add"
///     }
///
///     fn description(&self) -> Option<&str> {
///         Some("Adds two numbers")
///     }
///
///     fn input_schema(&self) -> Value {
///         json!({
///             "type": "object",
///             "properties": {
///                 "a": { "type": "number" },
///                 "b": { "type": "number" }
///             },
///             "required": ["a", "b"]
///         })
///     }
///
///     fn output_schema(&self) -> Option<Value> {
///         Some(json!({
///             "type": "object",
///             "properties": {
///                 "result": { "type": "number" }
///             }
///         }))
///     }
///
///     async fn execute(
///         &self,
///         input: Value,
///         _context: &ToolContext,
///     ) -> Result<ToolResult, ToolError> {
///         let a = input["a"].as_f64().ok_or_else(|| {
///             ToolError::ExecutionFailed("Invalid number for 'a'".to_string())
///         })?;
///         let b = input["b"].as_f64().ok_or_else(|| {
///             ToolError::ExecutionFailed("Invalid number for 'b'".to_string())
///         })?;
///
///         let result = json!({ "result": a + b });
///         Ok(ToolResult::success_json(result))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of this tool.
    ///
    /// Tool names must be unique within a server and should follow these conventions:
    /// - Use lowercase letters, numbers, underscores, and hyphens
    /// - Be descriptive but concise
    /// - Avoid special characters
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::Tool;
    /// use async_trait::async_trait;
    ///
    /// struct MyTool;
    ///
    /// # use mcp_server::tool::{ToolContext, ToolResult};
    /// # use mcp_server::error::ToolError;
    /// # use serde_json::Value;
    /// #
    /// #[async_trait]
    /// impl Tool for MyTool {
    ///     fn name(&self) -> &str {
    ///         "my_tool"
    ///     }
    /// #
    /// #   fn input_schema(&self) -> Value {
    /// #       serde_json::json!({})
    /// #   }
    /// #
    /// #   async fn execute(
    /// #       &self,
    /// #       _input: Value,
    /// #       _context: &ToolContext,
    /// #   ) -> Result<ToolResult, ToolError> {
    /// #       Ok(ToolResult::success_text(""))
    /// #   }
    /// }
    ///
    /// let tool = MyTool;
    /// assert_eq!(tool.name(), "my_tool");
    /// ```
    fn name(&self) -> &str;

    /// Returns an optional description of what this tool does.
    ///
    /// The description helps AI models understand when and how to use the tool.
    /// It should be clear and concise, explaining the tool's purpose and behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::Tool;
    /// # use async_trait::async_trait;
    /// # use mcp_server::tool::{ToolContext, ToolResult};
    /// # use mcp_server::error::ToolError;
    /// # use serde_json::Value;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    /// #   fn name(&self) -> &str { "my_tool" }
    /// #
    ///     fn description(&self) -> Option<&str> {
    ///         Some("Performs a specific operation on the input data")
    ///     }
    /// #
    /// #   fn input_schema(&self) -> Value { serde_json::json!({}) }
    /// #   async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    /// #       Ok(ToolResult::success_text(""))
    /// #   }
    /// }
    /// ```
    fn description(&self) -> Option<&str> {
        None
    }

    /// Returns the JSON schema for validating tool input.
    ///
    /// The schema should be a JSON Schema Draft 7 object that describes
    /// the expected input format. This is used for client-side validation
    /// and documentation generation.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::Tool;
    /// use serde_json::json;
    /// # use async_trait::async_trait;
    /// # use mcp_server::tool::{ToolContext, ToolResult};
    /// # use mcp_server::error::ToolError;
    /// # use serde_json::Value;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    /// #   fn name(&self) -> &str { "my_tool" }
    /// #
    ///     fn input_schema(&self) -> Value {
    ///         json!({
    ///             "type": "object",
    ///             "properties": {
    ///                 "input": {
    ///                     "type": "string",
    ///                     "description": "Input parameter"
    ///                 }
    ///             },
    ///             "required": ["input"]
    ///         })
    ///     }
    /// #   async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    /// #       Ok(ToolResult::success_text(""))
    /// #   }
    /// }
    /// ```
    fn input_schema(&self) -> Value;

    /// Returns an optional JSON schema for the tool's output.
    ///
    /// This is useful for documentation and validation purposes, though
    /// output validation is typically not enforced by the protocol.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::Tool;
    /// use serde_json::json;
    /// # use async_trait::async_trait;
    /// # use mcp_server::tool::{ToolContext, ToolResult};
    /// # use mcp_server::error::ToolError;
    /// # use serde_json::Value;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    /// #   fn name(&self) -> &str { "my_tool" }
    /// #   fn input_schema(&self) -> Value { json!({}) }
    /// #
    ///     fn output_schema(&self) -> Option<Value> {
    ///         Some(json!({
    ///             "type": "object",
    ///             "properties": {
    ///                 "result": { "type": "string" }
    ///             }
    ///         }))
    ///     }
    /// #   async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    /// #       Ok(ToolResult::success_text(""))
    /// #   }
    /// }
    /// ```
    fn output_schema(&self) -> Option<Value> {
        None
    }

    /// Returns optional metadata about this tool.
    ///
    /// Metadata can include additional information like categories, tags,
    /// version information, or any other custom data.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::Tool;
    /// use serde_json::json;
    /// # use async_trait::async_trait;
    /// # use mcp_server::tool::{ToolContext, ToolResult};
    /// # use mcp_server::error::ToolError;
    /// # use serde_json::Value;
    ///
    /// struct MyTool;
    ///
    /// #[async_trait]
    /// impl Tool for MyTool {
    /// #   fn name(&self) -> &str { "my_tool" }
    /// #   fn input_schema(&self) -> Value { json!({}) }
    /// #
    ///     fn metadata(&self) -> Option<Value> {
    ///         Some(json!({
    ///             "category": "utilities",
    ///             "version": "1.0.0",
    ///             "tags": ["helper", "utility"]
    ///         }))
    ///     }
    /// #   async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    /// #       Ok(ToolResult::success_text(""))
    /// #   }
    /// }
    /// ```
    fn metadata(&self) -> Option<Value> {
        None
    }

    /// Executes the tool with the given input and context.
    ///
    /// This is the core method that implements the tool's functionality.
    /// It receives validated JSON input and a context object, and should
    /// return either a successful result or an error.
    ///
    /// # Arguments
    ///
    /// * `input` - The JSON input, which should conform to the input schema
    /// * `context` - Contextual information about the request (session ID, client info, etc.)
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` if:
    /// - The input is invalid
    /// - The operation fails
    /// - The tool times out
    /// - An internal error occurs
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::{Tool, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct GreetTool;
    ///
    /// #[async_trait]
    /// impl Tool for GreetTool {
    ///     fn name(&self) -> &str {
    ///         "greet"
    ///     }
    ///
    ///     fn input_schema(&self) -> Value {
    ///         json!({
    ///             "type": "object",
    ///             "properties": {
    ///                 "name": { "type": "string" }
    ///             },
    ///             "required": ["name"]
    ///         })
    ///     }
    ///
    ///     async fn execute(
    ///         &self,
    ///         input: Value,
    ///         _context: &ToolContext,
    ///     ) -> Result<ToolResult, ToolError> {
    ///         let name = input["name"]
    ///             .as_str()
    ///             .ok_or_else(|| ToolError::ExecutionFailed(
    ///                 "name is required".to_string()
    ///             ))?;
    ///
    ///         Ok(ToolResult::success_text(format!("Hello, {}!", name)))
    ///     }
    /// }
    /// ```
    async fn execute(
        &self,
        input: Value,
        context: &ToolContext,
    ) -> Result<ToolResult, ToolError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            "test_tool"
        }

        fn description(&self) -> Option<&str> {
            Some("A test tool")
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            })
        }

        fn output_schema(&self) -> Option<Value> {
            Some(json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string" }
                }
            }))
        }

        fn metadata(&self) -> Option<Value> {
            Some(json!({
                "version": "1.0.0"
            }))
        }

        async fn execute(
            &self,
            input: Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            let text = input["input"].as_str().unwrap_or("default");
            Ok(ToolResult::success_text(text))
        }
    }

    #[tokio::test]
    async fn test_tool_name() {
        let tool = TestTool;
        assert_eq!(tool.name(), "test_tool");
    }

    #[tokio::test]
    async fn test_tool_description() {
        let tool = TestTool;
        assert_eq!(tool.description(), Some("A test tool"));
    }

    #[tokio::test]
    async fn test_tool_input_schema() {
        let tool = TestTool;
        let schema = tool.input_schema();
        assert_eq!(schema["type"], "object");
    }

    #[tokio::test]
    async fn test_tool_output_schema() {
        let tool = TestTool;
        let schema = tool.output_schema().unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let tool = TestTool;
        let metadata = tool.metadata().unwrap();
        assert_eq!(metadata["version"], "1.0.0");
    }

    #[tokio::test]
    async fn test_tool_execute() {
        let tool = TestTool;
        let context = ToolContext::new();
        let input = json!({"input": "test"});

        let result = tool.execute(input, &context).await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.content[0].as_text(), Some("test"));
    }

    #[tokio::test]
    async fn test_tool_execute_with_default() {
        let tool = TestTool;
        let context = ToolContext::new();
        let input = json!({});

        let result = tool.execute(input, &context).await.unwrap();
        assert_eq!(result.content[0].as_text(), Some("default"));
    }
}
