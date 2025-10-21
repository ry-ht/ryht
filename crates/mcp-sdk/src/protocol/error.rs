//! JSON-RPC 2.0 Error types and MCP error codes
//!
//! This module provides error types and constants for JSON-RPC 2.0 and MCP-specific errors.
//!
//! # Standard JSON-RPC Error Codes
//!
//! - `-32700`: Parse error - Invalid JSON
//! - `-32600`: Invalid Request - The JSON sent is not a valid Request object
//! - `-32601`: Method not found - The method does not exist / is not available
//! - `-32602`: Invalid params - Invalid method parameter(s)
//! - `-32603`: Internal error - Internal JSON-RPC error
//!
//! # MCP-Specific Error Codes
//!
//! - `-32000`: Server error - Generic server-side error
//! - `-32001`: Timeout error - Operation timed out
//! - `-32002`: Connection error - Connection-related error
//! - `-32003`: Resource not found - Requested resource does not exist
//! - `-32004`: Tool not found - Requested tool does not exist
//!
//! # Examples
//!
//! ```
//! use mcp_server::protocol::JsonRpcError;
//!
//! // Create a method not found error
//! let error = JsonRpcError::method_not_found();
//! assert_eq!(error.code, -32601);
//!
//! // Create a custom error with data
//! let error = JsonRpcError::new(
//!     -32000,
//!     "Custom error".to_string(),
//!     Some(serde_json::json!({"details": "Additional info"}))
//! );
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard JSON-RPC 2.0 error codes
pub mod codes {
    /// Parse error - Invalid JSON was received by the server
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request - The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found - The method does not exist / is not available
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params - Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error - Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Server error start (reserved range: -32000 to -32099)
    pub const SERVER_ERROR_START: i32 = -32099;

    /// Server error end (reserved range: -32000 to -32099)
    pub const SERVER_ERROR_END: i32 = -32000;
}

/// MCP-specific error codes
pub mod mcp_codes {
    /// Generic server-side error
    pub const SERVER_ERROR: i32 = -32000;

    /// Operation timed out
    pub const TIMEOUT_ERROR: i32 = -32001;

    /// Connection-related error
    pub const CONNECTION_ERROR: i32 = -32002;

    /// Requested resource does not exist
    pub const RESOURCE_NOT_FOUND: i32 = -32003;

    /// Requested tool does not exist
    pub const TOOL_NOT_FOUND: i32 = -32004;

    /// Tool execution failed
    pub const TOOL_EXECUTION_FAILED: i32 = -32005;

    /// Resource read failed
    pub const RESOURCE_READ_FAILED: i32 = -32006;

    /// Invalid capability
    pub const INVALID_CAPABILITY: i32 = -32007;

    /// Protocol version mismatch
    pub const PROTOCOL_VERSION_MISMATCH: i32 = -32008;
}

/// JSON-RPC 2.0 error object
///
/// # Specification
///
/// When a rpc call encounters an error, the Response Object MUST contain the error member
/// with a value that is an Object with the following members:
///
/// - `code`: A Number that indicates the error type that occurred
/// - `message`: A String providing a short description of the error
/// - `data` (optional): A Primitive or Structured value that contains additional information about the error
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::JsonRpcError;
/// use serde_json::json;
///
/// // Method not found
/// let error = JsonRpcError::method_not_found();
/// assert_eq!(error.message, "Method not found");
///
/// // Invalid params with details
/// let error = JsonRpcError::invalid_params("Missing required field 'name'");
/// assert_eq!(error.code, -32602);
///
/// // Custom error
/// let error = JsonRpcError::new(-32000, "Custom error".to_string(), None);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    /// Error code indicating the error type
    pub code: i32,

    /// Short description of the error
    pub message: String,

    /// Additional information about the error (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error
    ///
    /// # Arguments
    ///
    /// * `code` - Error code
    /// * `message` - Error message
    /// * `data` - Optional additional data
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    /// use serde_json::json;
    ///
    /// let error = JsonRpcError::new(
    ///     -32000,
    ///     "Custom error".to_string(),
    ///     Some(json!({"details": "Something went wrong"}))
    /// );
    /// ```
    pub fn new(code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    /// Create a parse error (-32700)
    ///
    /// Invalid JSON was received by the server.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::parse_error(Some("Unexpected token at line 5".to_string()));
    /// assert_eq!(error.code, -32700);
    /// ```
    pub fn parse_error(details: Option<String>) -> Self {
        Self {
            code: codes::PARSE_ERROR,
            message: "Parse error".to_string(),
            data: details.map(|d| Value::String(d)),
        }
    }

    /// Create an invalid request error (-32600)
    ///
    /// The JSON sent is not a valid Request object.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::invalid_request(Some("Missing 'method' field".to_string()));
    /// assert_eq!(error.code, -32600);
    /// ```
    pub fn invalid_request(details: Option<String>) -> Self {
        Self {
            code: codes::INVALID_REQUEST,
            message: "Invalid Request".to_string(),
            data: details.map(|d| Value::String(d)),
        }
    }

    /// Create a method not found error (-32601)
    ///
    /// The method does not exist / is not available.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::method_not_found();
    /// assert_eq!(error.code, -32601);
    /// assert_eq!(error.message, "Method not found");
    /// ```
    pub fn method_not_found() -> Self {
        Self {
            code: codes::METHOD_NOT_FOUND,
            message: "Method not found".to_string(),
            data: None,
        }
    }

    /// Create an invalid params error (-32602)
    ///
    /// Invalid method parameter(s).
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::invalid_params("Missing required parameter 'name'");
    /// assert_eq!(error.code, -32602);
    /// ```
    pub fn invalid_params(details: &str) -> Self {
        Self {
            code: codes::INVALID_PARAMS,
            message: "Invalid params".to_string(),
            data: Some(Value::String(details.to_string())),
        }
    }

    /// Create an internal error (-32603)
    ///
    /// Internal JSON-RPC error.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::internal_error(Some("Database connection failed".to_string()));
    /// assert_eq!(error.code, -32603);
    /// ```
    pub fn internal_error(details: Option<String>) -> Self {
        Self {
            code: codes::INTERNAL_ERROR,
            message: "Internal error".to_string(),
            data: details.map(|d| Value::String(d)),
        }
    }

    /// Create a server error (-32000)
    ///
    /// Generic server-side error.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::server_error("Service unavailable".to_string());
    /// assert_eq!(error.code, -32000);
    /// ```
    pub fn server_error(message: String) -> Self {
        Self {
            code: mcp_codes::SERVER_ERROR,
            message,
            data: None,
        }
    }

    /// Create a timeout error (-32001)
    ///
    /// Operation timed out.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::timeout_error("Operation exceeded 30s timeout".to_string());
    /// assert_eq!(error.code, -32001);
    /// ```
    pub fn timeout_error(message: String) -> Self {
        Self {
            code: mcp_codes::TIMEOUT_ERROR,
            message,
            data: None,
        }
    }

    /// Create a tool not found error (-32004)
    ///
    /// Requested tool does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::tool_not_found("calculate");
    /// assert_eq!(error.code, -32004);
    /// ```
    pub fn tool_not_found(tool_name: &str) -> Self {
        Self {
            code: mcp_codes::TOOL_NOT_FOUND,
            message: format!("Tool '{}' not found", tool_name),
            data: None,
        }
    }

    /// Create a resource not found error (-32003)
    ///
    /// Requested resource does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcError;
    ///
    /// let error = JsonRpcError::resource_not_found("file:///data.txt");
    /// assert_eq!(error.code, -32003);
    /// ```
    pub fn resource_not_found(uri: &str) -> Self {
        Self {
            code: mcp_codes::RESOURCE_NOT_FOUND,
            message: format!("Resource '{}' not found", uri),
            data: None,
        }
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC Error {}: {}", self.code, self.message)?;
        if let Some(data) = &self.data {
            write!(f, " (data: {})", data)?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonRpcError {}

/// Convert `ToolError` to `JsonRpcError`
impl From<crate::error::ToolError> for JsonRpcError {
    fn from(error: crate::error::ToolError) -> Self {
        use crate::error::ToolError;
        match error {
            ToolError::NotFound(name) => JsonRpcError::tool_not_found(&name),
            ToolError::AlreadyRegistered(name) => JsonRpcError::new(
                mcp_codes::SERVER_ERROR,
                format!("Tool '{}' is already registered", name),
                None,
            ),
            ToolError::InvalidInput(e) => JsonRpcError::invalid_params(&e.to_string()),
            ToolError::ExecutionFailed(msg) => JsonRpcError::new(
                mcp_codes::TOOL_EXECUTION_FAILED,
                format!("Tool execution failed: {}", msg),
                None,
            ),
            ToolError::Timeout(duration) => JsonRpcError::timeout_error(
                format!("Tool execution timeout after {:?}", duration),
            ),
            ToolError::Internal(e) => JsonRpcError::internal_error(Some(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_error_codes_constants() {
        assert_eq!(codes::PARSE_ERROR, -32700);
        assert_eq!(codes::INVALID_REQUEST, -32600);
        assert_eq!(codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(codes::INVALID_PARAMS, -32602);
        assert_eq!(codes::INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_mcp_error_codes() {
        assert_eq!(mcp_codes::SERVER_ERROR, -32000);
        assert_eq!(mcp_codes::TIMEOUT_ERROR, -32001);
        assert_eq!(mcp_codes::CONNECTION_ERROR, -32002);
        assert_eq!(mcp_codes::RESOURCE_NOT_FOUND, -32003);
        assert_eq!(mcp_codes::TOOL_NOT_FOUND, -32004);
    }

    #[test]
    fn test_new_error() {
        let error = JsonRpcError::new(
            -32000,
            "Test error".to_string(),
            Some(json!({"key": "value"})),
        );

        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "Test error");
        assert_eq!(error.data, Some(json!({"key": "value"})));
    }

    #[test]
    fn test_parse_error() {
        let error = JsonRpcError::parse_error(None);
        assert_eq!(error.code, codes::PARSE_ERROR);
        assert_eq!(error.message, "Parse error");
        assert_eq!(error.data, None);

        let error = JsonRpcError::parse_error(Some("Invalid JSON".to_string()));
        assert_eq!(error.data, Some(Value::String("Invalid JSON".to_string())));
    }

    #[test]
    fn test_invalid_request() {
        let error = JsonRpcError::invalid_request(None);
        assert_eq!(error.code, codes::INVALID_REQUEST);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_method_not_found() {
        let error = JsonRpcError::method_not_found();
        assert_eq!(error.code, codes::METHOD_NOT_FOUND);
        assert_eq!(error.message, "Method not found");
        assert_eq!(error.data, None);
    }

    #[test]
    fn test_invalid_params() {
        let error = JsonRpcError::invalid_params("Missing field 'x'");
        assert_eq!(error.code, codes::INVALID_PARAMS);
        assert_eq!(error.message, "Invalid params");
        assert_eq!(error.data, Some(Value::String("Missing field 'x'".to_string())));
    }

    #[test]
    fn test_internal_error() {
        let error = JsonRpcError::internal_error(None);
        assert_eq!(error.code, codes::INTERNAL_ERROR);
        assert_eq!(error.message, "Internal error");

        let error = JsonRpcError::internal_error(Some("Database error".to_string()));
        assert_eq!(error.data, Some(Value::String("Database error".to_string())));
    }

    #[test]
    fn test_server_error() {
        let error = JsonRpcError::server_error("Service unavailable".to_string());
        assert_eq!(error.code, mcp_codes::SERVER_ERROR);
        assert_eq!(error.message, "Service unavailable");
    }

    #[test]
    fn test_timeout_error() {
        let error = JsonRpcError::timeout_error("Timeout after 30s".to_string());
        assert_eq!(error.code, mcp_codes::TIMEOUT_ERROR);
        assert_eq!(error.message, "Timeout after 30s");
    }

    #[test]
    fn test_tool_not_found() {
        let error = JsonRpcError::tool_not_found("my_tool");
        assert_eq!(error.code, mcp_codes::TOOL_NOT_FOUND);
        assert_eq!(error.message, "Tool 'my_tool' not found");
    }

    #[test]
    fn test_resource_not_found() {
        let error = JsonRpcError::resource_not_found("file:///test.txt");
        assert_eq!(error.code, mcp_codes::RESOURCE_NOT_FOUND);
        assert_eq!(error.message, "Resource 'file:///test.txt' not found");
    }

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::new(
            -32600,
            "Test".to_string(),
            Some(json!({"details": "info"})),
        );

        let json = serde_json::to_string(&error).unwrap();
        let deserialized: JsonRpcError = serde_json::from_str(&json).unwrap();

        assert_eq!(error, deserialized);
    }

    #[test]
    fn test_error_serialization_without_data() {
        let error = JsonRpcError::method_not_found();
        let json = serde_json::to_value(&error).unwrap();

        assert!(!json.get("data").is_some());
        assert_eq!(json["code"], -32601);
        assert_eq!(json["message"], "Method not found");
    }

    #[test]
    fn test_error_display() {
        let error = JsonRpcError::method_not_found();
        let display = format!("{}", error);
        assert_eq!(display, "JSON-RPC Error -32601: Method not found");

        let error = JsonRpcError::new(
            -32000,
            "Test".to_string(),
            Some(json!("details")),
        );
        let display = format!("{}", error);
        assert!(display.contains("JSON-RPC Error -32000: Test"));
        assert!(display.contains("data:"));
    }

    #[test]
    fn test_error_is_error_trait() {
        let error = JsonRpcError::method_not_found();
        let _: &dyn std::error::Error = &error;
    }
}
