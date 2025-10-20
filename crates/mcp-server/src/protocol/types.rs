//! MCP Protocol Types
//!
//! This module contains all the protocol-level types defined in the MCP specification (2025-03-26).
//!
//! # Type Categories
//!
//! - **Initialization**: InitializeParams, InitializeResult, ServerInfo, ClientInfo
//! - **Tools**: ToolDefinition, CallToolParams, CallToolResult, ToolContent
//! - **Resources**: ResourceDefinition, ReadResourceParams, ReadResourceResult, ResourceContent
//! - **Prompts**: PromptDefinition, GetPromptParams, GetPromptResult, PromptMessage
//! - **Logging**: LoggingLevel, LoggingMessage
//!
//! # Examples
//!
//! ```
//! use mcp_server::protocol::{InitializeParams, ClientInfo, ClientCapabilities};
//!
//! let params = InitializeParams {
//!     protocol_version: "2025-03-26".to_string(),
//!     capabilities: ClientCapabilities::default(),
//!     client_info: ClientInfo {
//!         name: "test-client".to_string(),
//!         version: "1.0.0".to_string(),
//!     },
//! };
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::capabilities::{ClientCapabilities, ServerCapabilities};

// ================================================================================================
// Initialization Types
// ================================================================================================

/// Parameters for the initialize request
///
/// Sent by the client to initiate the MCP protocol handshake.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{InitializeParams, ClientInfo, ClientCapabilities};
///
/// let params = InitializeParams {
///     protocol_version: "2025-03-26".to_string(),
///     capabilities: ClientCapabilities::default(),
///     client_info: ClientInfo {
///         name: "my-client".to_string(),
///         version: "1.0.0".to_string(),
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializeParams {
    /// Protocol version the client supports
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Result of the initialize request
///
/// Sent by the server in response to an initialize request.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{InitializeResult, ServerInfo, ServerCapabilities};
///
/// let result = InitializeResult {
///     protocol_version: "2025-03-26".to_string(),
///     capabilities: ServerCapabilities::default(),
///     server_info: ServerInfo {
///         name: "my-server".to_string(),
///         version: "1.0.0".to_string(),
///     },
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializeResult {
    /// Protocol version the server uses
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Information about the server
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ServerInfo;
///
/// let info = ServerInfo {
///     name: "example-server".to_string(),
///     version: "1.0.0".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,
}

/// Information about the client
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ClientInfo;
///
/// let info = ClientInfo {
///     name: "example-client".to_string(),
///     version: "0.1.0".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,
}

// ================================================================================================
// Tool Types
// ================================================================================================

/// Tool definition exposed by the server
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ToolDefinition;
/// use serde_json::json;
///
/// let tool = ToolDefinition {
///     name: "echo".to_string(),
///     description: Some("Echo a message".to_string()),
///     input_schema: json!({
///         "type": "object",
///         "properties": {
///             "message": {"type": "string"}
///         },
///         "required": ["message"]
///     }),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// Unique tool name
    pub name: String,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// JSON Schema for input validation
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Parameters for calling a tool
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::CallToolParams;
/// use serde_json::json;
///
/// let params = CallToolParams {
///     name: "echo".to_string(),
///     arguments: Some(json!({"message": "hello"})),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallToolParams {
    /// Tool name to call
    pub name: String,

    /// Tool arguments (must match inputSchema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Result of a tool call
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{CallToolResult, ToolContent};
///
/// let result = CallToolResult {
///     content: vec![
///         ToolContent::Text {
///             text: "Operation completed".to_string(),
///         }
///     ],
///     is_error: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallToolResult {
    /// Result content (text, images, etc.)
    pub content: Vec<ToolContent>,

    /// Whether this result represents an error
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content returned by a tool
///
/// Tools can return text, images, or references to resources.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ToolContent;
///
/// // Text content
/// let text = ToolContent::Text {
///     text: "Hello, world!".to_string(),
/// };
///
/// // Image content
/// let image = ToolContent::Image {
///     data: "base64encodeddata".to_string(),
///     mime_type: "image/png".to_string(),
/// };
///
/// // Resource reference
/// let resource = ToolContent::Resource {
///     uri: "file:///data.txt".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,

        /// Image MIME type (e.g., "image/png")
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Resource reference
    #[serde(rename = "resource")]
    Resource {
        /// Resource URI
        uri: String,
    },
}

/// List tools result
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ListToolsResult;
///
/// let result = ListToolsResult {
///     tools: vec![],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListToolsResult {
    /// Available tools
    pub tools: Vec<ToolDefinition>,
}

// ================================================================================================
// Resource Types
// ================================================================================================

/// Resource definition exposed by the server
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ResourceDefinition;
///
/// let resource = ResourceDefinition {
///     uri: "file:///config.json".to_string(),
///     name: Some("Application Config".to_string()),
///     description: Some("Main configuration file".to_string()),
///     mime_type: Some("application/json".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceDefinition {
    /// Resource URI
    pub uri: String,

    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type of the resource
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Parameters for reading a resource
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ReadResourceParams;
///
/// let params = ReadResourceParams {
///     uri: "file:///data.txt".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadResourceParams {
    /// URI of the resource to read
    pub uri: String,
}

/// Result of reading a resource
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{ReadResourceResult, ResourceContent};
///
/// let result = ReadResourceResult {
///     contents: vec![
///         ResourceContent::Text {
///             uri: "file:///data.txt".to_string(),
///             mime_type: Some("text/plain".to_string()),
///             text: "File contents".to_string(),
///         }
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadResourceResult {
    /// Resource contents
    pub contents: Vec<ResourceContent>,
}

/// Content of a resource
///
/// Resources can contain text or binary data.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ResourceContent;
///
/// // Text resource
/// let text = ResourceContent::Text {
///     uri: "file:///data.txt".to_string(),
///     mime_type: Some("text/plain".to_string()),
///     text: "Hello, world!".to_string(),
/// };
///
/// // Binary resource
/// let blob = ResourceContent::Blob {
///     uri: "file:///image.png".to_string(),
///     mime_type: Some("image/png".to_string()),
///     blob: "base64data".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ResourceContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// Resource URI
        uri: String,

        /// MIME type
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,

        /// Text content
        text: String,
    },

    /// Binary content
    #[serde(rename = "blob")]
    Blob {
        /// Resource URI
        uri: String,

        /// MIME type
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,

        /// Base64-encoded binary data
        blob: String,
    },
}

/// List resources result
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ListResourcesResult;
///
/// let result = ListResourcesResult {
///     resources: vec![],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListResourcesResult {
    /// Available resources
    pub resources: Vec<ResourceDefinition>,
}

// ================================================================================================
// Prompt Types
// ================================================================================================

/// Prompt definition exposed by the server
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::PromptDefinition;
///
/// let prompt = PromptDefinition {
///     name: "greeting".to_string(),
///     description: Some("Generate a greeting".to_string()),
///     arguments: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptDefinition {
    /// Unique prompt name
    pub name: String,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Prompt arguments schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// Prompt argument definition
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::PromptArgument;
///
/// let arg = PromptArgument {
///     name: "name".to_string(),
///     description: Some("Name to greet".to_string()),
///     required: Some(true),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,

    /// Argument description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether the argument is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// Parameters for getting a prompt
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::GetPromptParams;
/// use std::collections::HashMap;
///
/// let mut arguments = HashMap::new();
/// arguments.insert("name".to_string(), "Alice".to_string());
///
/// let params = GetPromptParams {
///     name: "greeting".to_string(),
///     arguments: Some(arguments),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPromptParams {
    /// Prompt name
    pub name: String,

    /// Prompt arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// Result of getting a prompt
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{GetPromptResult, PromptMessage};
///
/// let result = GetPromptResult {
///     description: Some("Greeting prompt".to_string()),
///     messages: vec![
///         PromptMessage {
///             role: "user".to_string(),
///             content: "Hello, Alice!".to_string(),
///         }
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPromptResult {
    /// Prompt description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Prompt messages
    pub messages: Vec<PromptMessage>,
}

/// A message in a prompt
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::PromptMessage;
///
/// let message = PromptMessage {
///     role: "user".to_string(),
///     content: "What is the weather?".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMessage {
    /// Message role (e.g., "user", "assistant")
    pub role: String,

    /// Message content
    pub content: String,
}

/// List prompts result
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ListPromptsResult;
///
/// let result = ListPromptsResult {
///     prompts: vec![],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListPromptsResult {
    /// Available prompts
    pub prompts: Vec<PromptDefinition>,
}

// ================================================================================================
// Logging Types
// ================================================================================================

/// Logging level
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::LoggingLevel;
///
/// let level = LoggingLevel::Info;
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Notice level
    Notice,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical level
    Critical,
    /// Alert level
    Alert,
    /// Emergency level
    Emergency,
}

/// Logging message parameters
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{LoggingMessageParams, LoggingLevel};
///
/// let params = LoggingMessageParams {
///     level: LoggingLevel::Info,
///     logger: Some("my-server".to_string()),
///     data: "Operation completed".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingMessageParams {
    /// Log level
    pub level: LoggingLevel,

    /// Logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,

    /// Log message data
    pub data: String,
}

// ================================================================================================
// Notification Types
// ================================================================================================

/// Progress notification parameters
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::ProgressParams;
///
/// let params = ProgressParams {
///     progress_token: "token-123".to_string(),
///     progress: 50.0,
///     total: Some(100.0),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressParams {
    /// Progress token
    #[serde(rename = "progressToken")]
    pub progress_token: String,

    /// Current progress
    pub progress: f64,

    /// Total progress (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================================
    // Initialization Tests
    // ============================================================================================

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "test".to_string(),
                version: "1.0".to_string(),
            },
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["protocolVersion"], "2025-03-26");
        assert_eq!(json["clientInfo"]["name"], "test");
    }

    #[test]
    fn test_initialize_result_roundtrip() {
        let result = InitializeResult {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: InitializeResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    // ============================================================================================
    // Tool Tests
    // ============================================================================================

    #[test]
    fn test_tool_definition() {
        let tool = ToolDefinition {
            name: "echo".to_string(),
            description: Some("Echo tool".to_string()),
            input_schema: json!({"type": "object"}),
        };

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["name"], "echo");
        assert_eq!(json["inputSchema"]["type"], "object");
    }

    #[test]
    fn test_call_tool_params() {
        let params = CallToolParams {
            name: "echo".to_string(),
            arguments: Some(json!({"message": "hello"})),
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: CallToolParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, deserialized);
    }

    #[test]
    fn test_call_tool_result() {
        let result = CallToolResult {
            content: vec![ToolContent::Text {
                text: "Success".to_string(),
            }],
            is_error: Some(false),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][0]["text"], "Success");
        assert_eq!(json["isError"], false);
    }

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::Text {
            text: "Hello".to_string(),
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello");
    }

    #[test]
    fn test_tool_content_image() {
        let content = ToolContent::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["mimeType"], "image/png");
    }

    #[test]
    fn test_tool_content_resource() {
        let content = ToolContent::Resource {
            uri: "file:///test.txt".to_string(),
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "resource");
        assert_eq!(json["uri"], "file:///test.txt");
    }

    // ============================================================================================
    // Resource Tests
    // ============================================================================================

    #[test]
    fn test_resource_definition() {
        let resource = ResourceDefinition {
            uri: "file:///config.json".to_string(),
            name: Some("Config".to_string()),
            description: Some("Configuration file".to_string()),
            mime_type: Some("application/json".to_string()),
        };

        let json = serde_json::to_value(&resource).unwrap();
        assert_eq!(json["uri"], "file:///config.json");
        assert_eq!(json["mimeType"], "application/json");
    }

    #[test]
    fn test_read_resource_params() {
        let params = ReadResourceParams {
            uri: "file:///data.txt".to_string(),
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: ReadResourceParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, deserialized);
    }

    #[test]
    fn test_resource_content_text() {
        let content = ResourceContent::Text {
            uri: "file:///test.txt".to_string(),
            mime_type: Some("text/plain".to_string()),
            text: "Content".to_string(),
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Content");
    }

    #[test]
    fn test_resource_content_blob() {
        let content = ResourceContent::Blob {
            uri: "file:///image.png".to_string(),
            mime_type: Some("image/png".to_string()),
            blob: "base64data".to_string(),
        };

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "blob");
        assert_eq!(json["blob"], "base64data");
    }

    // ============================================================================================
    // Prompt Tests
    // ============================================================================================

    #[test]
    fn test_prompt_definition() {
        let prompt = PromptDefinition {
            name: "greeting".to_string(),
            description: Some("Greeting prompt".to_string()),
            arguments: Some(vec![PromptArgument {
                name: "name".to_string(),
                description: Some("Name".to_string()),
                required: Some(true),
            }]),
        };

        let json = serde_json::to_value(&prompt).unwrap();
        assert_eq!(json["name"], "greeting");
        assert_eq!(json["arguments"][0]["name"], "name");
    }

    #[test]
    fn test_get_prompt_params() {
        let mut arguments = HashMap::new();
        arguments.insert("name".to_string(), "Alice".to_string());

        let params = GetPromptParams {
            name: "greeting".to_string(),
            arguments: Some(arguments),
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: GetPromptParams = serde_json::from_str(&json).unwrap();
        assert_eq!(params, deserialized);
    }

    // ============================================================================================
    // Logging Tests
    // ============================================================================================

    #[test]
    fn test_logging_level_serialization() {
        let level = LoggingLevel::Info;
        let json = serde_json::to_value(&level).unwrap();
        assert_eq!(json, "info");

        let level = LoggingLevel::Error;
        let json = serde_json::to_value(&level).unwrap();
        assert_eq!(json, "error");
    }

    #[test]
    fn test_logging_level_ordering() {
        assert!(LoggingLevel::Debug < LoggingLevel::Info);
        assert!(LoggingLevel::Info < LoggingLevel::Warning);
        assert!(LoggingLevel::Warning < LoggingLevel::Error);
    }

    #[test]
    fn test_logging_message_params() {
        let params = LoggingMessageParams {
            level: LoggingLevel::Info,
            logger: Some("test".to_string()),
            data: "Message".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["level"], "info");
        assert_eq!(json["data"], "Message");
    }

    // ============================================================================================
    // Notification Tests
    // ============================================================================================

    #[test]
    fn test_progress_params() {
        let params = ProgressParams {
            progress_token: "token-123".to_string(),
            progress: 50.0,
            total: Some(100.0),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["progressToken"], "token-123");
        assert_eq!(json["progress"], 50.0);
        assert_eq!(json["total"], 100.0);
    }

    // ============================================================================================
    // General Tests
    // ============================================================================================

    #[test]
    fn test_clone_all_types() {
        let init_params = InitializeParams {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "test".to_string(),
                version: "1.0".to_string(),
            },
        };
        let _cloned = init_params.clone();

        let tool = ToolDefinition {
            name: "test".to_string(),
            description: None,
            input_schema: json!({}),
        };
        let _cloned = tool.clone();
    }

    #[test]
    fn test_debug_all_types() {
        let info = ServerInfo {
            name: "test".to_string(),
            version: "1.0".to_string(),
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("ServerInfo"));
    }
}
