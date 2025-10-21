//! Error types for the MCP server framework.
//!
//! This module provides comprehensive error handling for all aspects of the MCP server,
//! including tools, resources, transport, middleware, and protocol errors.
//!
//! # Error Hierarchy
//!
//! ```text
//! McpError (top-level)
//! ├── Transport(TransportError)
//! ├── Tool(ToolError)
//! ├── Resource(ResourceError)
//! ├── Middleware(MiddlewareError)
//! ├── Config(String)
//! └── Protocol(String)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::error::{McpError, ToolError};
//!
//! fn may_fail() -> Result<(), McpError> {
//!     Err(ToolError::NotFound("my_tool".to_string()).into())
//! }
//! ```

use std::time::Duration;
use thiserror::Error;

/// Result type alias for MCP operations.
///
/// This is a convenience alias for `std::result::Result<T, McpError>`.
pub type Result<T> = std::result::Result<T, McpError>;

/// Top-level error type for the MCP server framework.
///
/// This error type encompasses all possible errors that can occur during
/// MCP server operations, including transport, tool execution, resource access,
/// middleware processing, configuration, and protocol errors.
///
/// # Conversions
///
/// All sub-error types automatically convert to `McpError` via the `From` trait:
///
/// ```rust
/// use mcp_server::error::{McpError, ToolError};
///
/// let tool_error = ToolError::NotFound("echo".to_string());
/// let mcp_error: McpError = tool_error.into();
/// ```
#[derive(Debug, Error)]
pub enum McpError {
    /// Transport-layer error (I/O, connection issues).
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Tool execution error.
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    /// Resource access error.
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    /// Middleware processing error.
    #[error("Middleware error: {0}")]
    Middleware(#[from] MiddlewareError),

    /// Server configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// MCP protocol error (invalid messages, protocol violations).
    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Tool-specific errors.
///
/// These errors occur during tool registration, validation, or execution.
///
/// # Examples
///
/// ```rust
/// use mcp_server::error::ToolError;
/// use std::time::Duration;
///
/// // Tool not found
/// let error = ToolError::NotFound("nonexistent_tool".to_string());
///
/// // Execution timeout
/// let error = ToolError::Timeout(Duration::from_secs(30));
/// ```
#[derive(Debug, Error)]
pub enum ToolError {
    /// The requested tool was not found in the registry.
    ///
    /// This error occurs when a client requests a tool that hasn't been registered
    /// with the server.
    #[error("Tool not found: {0}")]
    NotFound(String),

    /// Tool already registered.
    ///
    /// This error occurs when attempting to register a tool with a name that
    /// already exists in the registry.
    #[error("Tool already registered: {0}")]
    AlreadyRegistered(String),

    /// Invalid input provided to a tool.
    ///
    /// This error occurs when the tool input fails JSON deserialization or
    /// schema validation.
    #[error("Invalid tool input: {0}")]
    InvalidInput(#[from] serde_json::Error),

    /// Tool execution failed.
    ///
    /// This error wraps any error that occurs during tool execution, providing
    /// a descriptive message about what went wrong.
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Tool execution exceeded the timeout limit.
    ///
    /// This error occurs when a tool takes longer than the configured timeout
    /// to complete execution.
    #[error("Tool execution timeout after {0:?}")]
    Timeout(Duration),

    /// Internal tool error.
    ///
    /// This is a catch-all for unexpected errors during tool operations.
    /// It wraps any error type via `anyhow::Error`.
    #[error("Internal tool error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Resource-specific errors.
///
/// These errors occur during resource registration, URI resolution, or content retrieval.
///
/// # Examples
///
/// ```rust
/// use mcp_server::error::ResourceError;
///
/// // Resource not found
/// let error = ResourceError::NotFound("app://config".to_string());
///
/// // Invalid URI
/// let error = ResourceError::InvalidUri("not a valid URI".to_string());
/// ```
#[derive(Debug, Error)]
pub enum ResourceError {
    /// The requested resource was not found.
    ///
    /// This error occurs when a client requests a resource URI that doesn't match
    /// any registered resource patterns.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid resource URI.
    ///
    /// This error occurs when the resource URI is malformed or doesn't match
    /// expected patterns.
    #[error("Invalid resource URI: {0}")]
    InvalidUri(String),

    /// Failed to read resource content.
    ///
    /// This error occurs when resource content cannot be retrieved, for example
    /// due to file system errors, database errors, or network issues.
    #[error("Failed to read resource: {0}")]
    ReadFailed(String),

    /// Internal resource error.
    ///
    /// This is a catch-all for unexpected errors during resource operations.
    /// It wraps any error type via `anyhow::Error`.
    #[error("Internal resource error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Transport-layer errors.
///
/// These errors occur during message transmission and reception over the transport layer.
///
/// # Examples
///
/// ```rust
/// use mcp_server::error::TransportError;
///
/// // Connection closed
/// let error = TransportError::Closed;
///
/// // Invalid message
/// let error = TransportError::InvalidMessage("malformed JSON".to_string());
/// ```
#[derive(Debug, Error)]
pub enum TransportError {
    /// I/O error during transport operations.
    ///
    /// This wraps standard I/O errors that occur during reading from or writing
    /// to the transport layer (e.g., stdin/stdout, sockets).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Transport connection was closed.
    ///
    /// This error occurs when attempting to use a closed transport connection.
    #[error("Connection closed")]
    Closed,

    /// Invalid or malformed message received.
    ///
    /// This error occurs when a message cannot be parsed or doesn't conform
    /// to the expected JSON-RPC format.
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

/// Middleware processing errors.
///
/// These errors occur during middleware execution, either in the request or response phase.
///
/// # Examples
///
/// ```rust
/// use mcp_server::error::MiddlewareError;
///
/// // Request blocked by middleware
/// let error = MiddlewareError::Blocked("rate limit exceeded".to_string());
/// ```
#[derive(Debug, Error)]
pub enum MiddlewareError {
    /// Request was blocked by middleware.
    ///
    /// This error occurs when middleware determines that a request should not
    /// be processed (e.g., due to authentication failure, rate limiting).
    #[error("Request blocked: {0}")]
    Blocked(String),

    /// Internal middleware error.
    ///
    /// This is a catch-all for unexpected errors during middleware operations.
    /// It wraps any error type via `anyhow::Error`.
    #[error("Internal middleware error: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Registry-specific errors.
///
/// These errors occur during tool or resource registration.
///
/// # Examples
///
/// ```rust
/// use mcp_server::error::RegistryError;
///
/// // Duplicate tool registration
/// let error = RegistryError::DuplicateTool("echo".to_string());
/// ```
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Attempted to register a tool that already exists.
    #[error("Tool already registered: {0}")]
    DuplicateTool(String),

    /// Attempted to register a resource that already exists.
    #[error("Resource already registered: {0}")]
    DuplicateResource(String),
}

// Conversion from RegistryError to ToolError
impl From<RegistryError> for ToolError {
    fn from(error: RegistryError) -> Self {
        match error {
            RegistryError::DuplicateTool(name) => ToolError::AlreadyRegistered(name),
            RegistryError::DuplicateResource(_) => {
                ToolError::Internal(anyhow::anyhow!("Resource registration error"))
            }
        }
    }
}

// ============================================================================
// JSON-RPC Error Conversions
// ============================================================================

/// JSON-RPC error code for tool not found.
pub const JSONRPC_TOOL_NOT_FOUND: i32 = -32601;

/// JSON-RPC error code for invalid tool input.
pub const JSONRPC_INVALID_PARAMS: i32 = -32602;

/// JSON-RPC error code for internal error.
pub const JSONRPC_INTERNAL_ERROR: i32 = -32603;

/// JSON-RPC error code for tool execution failure.
pub const JSONRPC_EXECUTION_FAILED: i32 = -32000;

/// JSON-RPC error code for tool timeout.
pub const JSONRPC_TIMEOUT: i32 = -32001;

/// JSON-RPC error code for resource not found.
pub const JSONRPC_RESOURCE_NOT_FOUND: i32 = -32002;

/// JSON-RPC error code for invalid resource URI.
pub const JSONRPC_INVALID_URI: i32 = -32003;

/// JSON-RPC error code for resource read failure.
pub const JSONRPC_READ_FAILED: i32 = -32004;

/// JSON-RPC 2.0 error object.
///
/// This struct represents a JSON-RPC error as defined in the JSON-RPC 2.0 specification.
/// It is used to communicate errors back to MCP clients in a standardized format.
///
/// # JSON-RPC Error Codes
///
/// Standard error codes:
/// - `-32700`: Parse error
/// - `-32600`: Invalid request
/// - `-32601`: Method/Tool not found
/// - `-32602`: Invalid params/input
/// - `-32603`: Internal error
///
/// MCP-specific error codes (implementation-defined, -32000 to -32099):
/// - `-32000`: Tool execution failed
/// - `-32001`: Tool timeout
/// - `-32002`: Resource not found
/// - `-32003`: Invalid resource URI
/// - `-32004`: Resource read failed
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    /// Error code as defined by JSON-RPC 2.0 specification.
    pub code: i32,

    /// Human-readable error message.
    pub message: String,

    /// Optional additional error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Creates a new JSON-RPC error.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new JSON-RPC error with additional data.
    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Convert `ToolError` to `JsonRpcError` for client responses.
///
/// This conversion maps tool errors to appropriate JSON-RPC error codes and messages,
/// ensuring clients receive standardized error information.
///
/// # Error Code Mapping
///
/// - `NotFound` → `-32601` (Method not found)
/// - `InvalidInput` → `-32602` (Invalid params)
/// - `ExecutionFailed` → `-32000` (Tool execution failed)
/// - `Timeout` → `-32001` (Tool timeout)
/// - `Internal` → `-32603` (Internal error)
impl From<ToolError> for JsonRpcError {
    fn from(error: ToolError) -> Self {
        match error {
            ToolError::NotFound(name) => JsonRpcError::new(
                JSONRPC_TOOL_NOT_FOUND,
                format!("Tool '{}' not found", name),
            ),
            ToolError::AlreadyRegistered(name) => JsonRpcError::new(
                JSONRPC_EXECUTION_FAILED,
                format!("Tool '{}' already registered", name),
            ),
            ToolError::InvalidInput(e) => JsonRpcError::with_data(
                JSONRPC_INVALID_PARAMS,
                "Invalid tool input",
                serde_json::json!({ "details": e.to_string() }),
            ),
            ToolError::ExecutionFailed(msg) => JsonRpcError::with_data(
                JSONRPC_EXECUTION_FAILED,
                "Tool execution failed",
                serde_json::json!({ "details": msg }),
            ),
            ToolError::Timeout(duration) => JsonRpcError::new(
                JSONRPC_TIMEOUT,
                format!("Tool execution timeout after {:?}", duration),
            ),
            ToolError::Internal(e) => JsonRpcError::with_data(
                JSONRPC_INTERNAL_ERROR,
                "Internal error",
                serde_json::json!({ "details": e.to_string() }),
            ),
        }
    }
}

/// Convert `ResourceError` to `JsonRpcError` for client responses.
///
/// This conversion maps resource errors to appropriate JSON-RPC error codes and messages,
/// ensuring clients receive standardized error information.
///
/// # Error Code Mapping
///
/// - `NotFound` → `-32002` (Resource not found)
/// - `InvalidUri` → `-32003` (Invalid resource URI)
/// - `ReadFailed` → `-32004` (Resource read failed)
/// - `Internal` → `-32603` (Internal error)
impl From<ResourceError> for JsonRpcError {
    fn from(error: ResourceError) -> Self {
        match error {
            ResourceError::NotFound(uri) => JsonRpcError::new(
                JSONRPC_RESOURCE_NOT_FOUND,
                format!("Resource '{}' not found", uri),
            ),
            ResourceError::InvalidUri(uri) => JsonRpcError::with_data(
                JSONRPC_INVALID_URI,
                "Invalid resource URI",
                serde_json::json!({ "uri": uri }),
            ),
            ResourceError::ReadFailed(msg) => JsonRpcError::with_data(
                JSONRPC_READ_FAILED,
                "Failed to read resource",
                serde_json::json!({ "details": msg }),
            ),
            ResourceError::Internal(e) => JsonRpcError::with_data(
                JSONRPC_INTERNAL_ERROR,
                "Internal error",
                serde_json::json!({ "details": e.to_string() }),
            ),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_error_not_found_display() {
        let error = ToolError::NotFound("my_tool".to_string());
        assert_eq!(error.to_string(), "Tool not found: my_tool");
    }

    #[test]
    fn test_tool_error_timeout_display() {
        let error = ToolError::Timeout(Duration::from_secs(30));
        assert!(error.to_string().contains("30s"));
    }

    #[test]
    fn test_resource_error_not_found_display() {
        let error = ResourceError::NotFound("app://config".to_string());
        assert_eq!(error.to_string(), "Resource not found: app://config");
    }

    #[test]
    fn test_transport_error_closed_display() {
        let error = TransportError::Closed;
        assert_eq!(error.to_string(), "Connection closed");
    }

    #[test]
    fn test_middleware_error_blocked_display() {
        let error = MiddlewareError::Blocked("rate limit".to_string());
        assert_eq!(error.to_string(), "Request blocked: rate limit");
    }

    #[test]
    fn test_tool_error_to_jsonrpc_not_found() {
        let error = ToolError::NotFound("echo".to_string());
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_TOOL_NOT_FOUND);
        assert!(jsonrpc_error.message.contains("echo"));
        assert!(jsonrpc_error.data.is_none());
    }

    #[test]
    fn test_tool_error_to_jsonrpc_invalid_input() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let error = ToolError::InvalidInput(json_error);
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_INVALID_PARAMS);
        assert_eq!(jsonrpc_error.message, "Invalid tool input");
        assert!(jsonrpc_error.data.is_some());
    }

    #[test]
    fn test_tool_error_to_jsonrpc_execution_failed() {
        let error = ToolError::ExecutionFailed("something went wrong".to_string());
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_EXECUTION_FAILED);
        assert_eq!(jsonrpc_error.message, "Tool execution failed");
        assert!(jsonrpc_error.data.is_some());

        let data = jsonrpc_error.data.unwrap();
        assert_eq!(data["details"], "something went wrong");
    }

    #[test]
    fn test_tool_error_to_jsonrpc_timeout() {
        let error = ToolError::Timeout(Duration::from_secs(30));
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_TIMEOUT);
        assert!(jsonrpc_error.message.contains("30s"));
        assert!(jsonrpc_error.data.is_none());
    }

    #[test]
    fn test_tool_error_to_jsonrpc_internal() {
        let error = ToolError::Internal(anyhow::anyhow!("internal failure"));
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_INTERNAL_ERROR);
        assert_eq!(jsonrpc_error.message, "Internal error");
        assert!(jsonrpc_error.data.is_some());
    }

    #[test]
    fn test_resource_error_to_jsonrpc_not_found() {
        let error = ResourceError::NotFound("file://test.txt".to_string());
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_RESOURCE_NOT_FOUND);
        assert!(jsonrpc_error.message.contains("file://test.txt"));
        assert!(jsonrpc_error.data.is_none());
    }

    #[test]
    fn test_resource_error_to_jsonrpc_invalid_uri() {
        let error = ResourceError::InvalidUri("not a uri".to_string());
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_INVALID_URI);
        assert_eq!(jsonrpc_error.message, "Invalid resource URI");
        assert!(jsonrpc_error.data.is_some());

        let data = jsonrpc_error.data.unwrap();
        assert_eq!(data["uri"], "not a uri");
    }

    #[test]
    fn test_resource_error_to_jsonrpc_read_failed() {
        let error = ResourceError::ReadFailed("file not readable".to_string());
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_READ_FAILED);
        assert_eq!(jsonrpc_error.message, "Failed to read resource");
        assert!(jsonrpc_error.data.is_some());

        let data = jsonrpc_error.data.unwrap();
        assert_eq!(data["details"], "file not readable");
    }

    #[test]
    fn test_resource_error_to_jsonrpc_internal() {
        let error = ResourceError::Internal(anyhow::anyhow!("internal failure"));
        let jsonrpc_error: JsonRpcError = error.into();

        assert_eq!(jsonrpc_error.code, JSONRPC_INTERNAL_ERROR);
        assert_eq!(jsonrpc_error.message, "Internal error");
        assert!(jsonrpc_error.data.is_some());
    }

    #[test]
    fn test_jsonrpc_error_serialization() {
        let error = JsonRpcError::with_data(
            -32000,
            "Test error",
            json!({ "key": "value" }),
        );

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":-32000"));
        assert!(json.contains("\"message\":\"Test error\""));
        assert!(json.contains("\"key\":\"value\""));
    }

    #[test]
    fn test_jsonrpc_error_deserialization() {
        let json = r#"{"code":-32000,"message":"Test error","data":{"key":"value"}}"#;
        let error: JsonRpcError = serde_json::from_str(json).unwrap();

        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Test error");
        assert!(error.data.is_some());
    }

    #[test]
    fn test_mcp_error_from_tool_error() {
        let tool_error = ToolError::NotFound("test".to_string());
        let mcp_error: McpError = tool_error.into();

        match mcp_error {
            McpError::Tool(ToolError::NotFound(name)) => assert_eq!(name, "test"),
            _ => panic!("Expected Tool error"),
        }
    }

    #[test]
    fn test_mcp_error_from_resource_error() {
        let resource_error = ResourceError::NotFound("test".to_string());
        let mcp_error: McpError = resource_error.into();

        match mcp_error {
            McpError::Resource(ResourceError::NotFound(uri)) => assert_eq!(uri, "test"),
            _ => panic!("Expected Resource error"),
        }
    }

    #[test]
    fn test_mcp_error_from_transport_error() {
        let transport_error = TransportError::Closed;
        let mcp_error: McpError = transport_error.into();

        match mcp_error {
            McpError::Transport(TransportError::Closed) => {},
            _ => panic!("Expected Transport error"),
        }
    }

    #[test]
    fn test_mcp_error_from_middleware_error() {
        let middleware_error = MiddlewareError::Blocked("test".to_string());
        let mcp_error: McpError = middleware_error.into();

        match mcp_error {
            McpError::Middleware(MiddlewareError::Blocked(msg)) => assert_eq!(msg, "test"),
            _ => panic!("Expected Middleware error"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_result() -> Result<String> {
            Ok("success".to_string())
        }

        assert!(returns_result().is_ok());
    }

    #[test]
    fn test_error_display_formats() {
        let errors = vec![
            McpError::Config("invalid config".to_string()).to_string(),
            McpError::Protocol("invalid protocol".to_string()).to_string(),
            McpError::Tool(ToolError::NotFound("test".to_string())).to_string(),
            McpError::Resource(ResourceError::NotFound("test".to_string())).to_string(),
            McpError::Transport(TransportError::Closed).to_string(),
            McpError::Middleware(MiddlewareError::Blocked("test".to_string())).to_string(),
        ];

        for error in errors {
            assert!(!error.is_empty());
        }
    }

    #[test]
    fn test_error_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let transport_error: TransportError = io_error.into();
        let mcp_error: McpError = transport_error.into();

        assert!(mcp_error.to_string().contains("test"));
    }

    #[test]
    fn test_jsonrpc_error_without_data() {
        let error = JsonRpcError::new(-32000, "Test error");
        let json = serde_json::to_string(&error).unwrap();

        // data field should be omitted when None
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_error_constants() {
        assert_eq!(JSONRPC_TOOL_NOT_FOUND, -32601);
        assert_eq!(JSONRPC_INVALID_PARAMS, -32602);
        assert_eq!(JSONRPC_INTERNAL_ERROR, -32603);
        assert_eq!(JSONRPC_EXECUTION_FAILED, -32000);
        assert_eq!(JSONRPC_TIMEOUT, -32001);
        assert_eq!(JSONRPC_RESOURCE_NOT_FOUND, -32002);
        assert_eq!(JSONRPC_INVALID_URI, -32003);
        assert_eq!(JSONRPC_READ_FAILED, -32004);
    }
}
