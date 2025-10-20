//! JSON-RPC 2.0 Response types
//!
//! This module provides types for JSON-RPC 2.0 responses according to the specification.
//!
//! # Specification
//!
//! When a rpc call is made, the Server MUST reply with a Response. The Response is expressed as a single JSON Object, with the following members:
//!
//! - `jsonrpc`: A String specifying the version of the JSON-RPC protocol. MUST be exactly "2.0"
//! - `result`: Required on success. Must not exist if there was an error
//! - `error`: Required on error. Must not exist if there was no error
//! - `id`: Must be the same as the value of the id member in the Request Object
//!
//! # Examples
//!
//! ```
//! use mcp_server::protocol::JsonRpcResponse;
//! use serde_json::json;
//!
//! // Success response
//! let response = JsonRpcResponse::success(
//!     Some(json!(1)),
//!     json!({"message": "Operation completed"})
//! );
//!
//! // Error response
//! let response = JsonRpcResponse::method_not_found(Some(json!(1)));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::JsonRpcError;

/// JSON-RPC 2.0 Response object
///
/// When a rpc call is made, the Server MUST reply with a Response, except for notifications.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::{JsonRpcResponse, JsonRpcError};
/// use serde_json::json;
///
/// // Success response
/// let response = JsonRpcResponse::success(
///     Some(json!(1)),
///     json!({"result": "ok"})
/// );
/// assert!(response.result.is_some());
/// assert!(response.error.is_none());
///
/// // Error response
/// let response = JsonRpcResponse::error(
///     Some(json!(2)),
///     JsonRpcError::method_not_found()
/// );
/// assert!(response.result.is_none());
/// assert!(response.error.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    /// JSON-RPC protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Response identifier (matches request id)
    pub id: Option<Value>,

    /// Result value (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error object (present on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a new success response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `result` - Result value
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::success(
    ///     Some(json!(1)),
    ///     json!({"tools": []})
    /// );
    ///
    /// assert_eq!(response.id, Some(json!(1)));
    /// assert!(response.result.is_some());
    /// assert!(response.error.is_none());
    /// ```
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create a new error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `error` - Error object
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::{JsonRpcResponse, JsonRpcError};
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::error(
    ///     Some(json!(1)),
    ///     JsonRpcError::invalid_params("Missing field 'name'")
    /// );
    ///
    /// assert!(response.result.is_none());
    /// assert!(response.error.is_some());
    /// ```
    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Create a method not found error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::method_not_found(Some(json!(1)));
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32601);
    /// ```
    pub fn method_not_found(id: Option<Value>) -> Self {
        Self::error(id, JsonRpcError::method_not_found())
    }

    /// Create an invalid params error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `details` - Error details
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::invalid_params(
    ///     Some(json!(1)),
    ///     "Missing required parameter 'name'"
    /// );
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32602);
    /// ```
    pub fn invalid_params(id: Option<Value>, details: &str) -> Self {
        Self::error(id, JsonRpcError::invalid_params(details))
    }

    /// Create an internal error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `details` - Optional error details
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::internal_error(
    ///     Some(json!(1)),
    ///     Some("Database connection failed".to_string())
    /// );
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32603);
    /// ```
    pub fn internal_error(id: Option<Value>, details: Option<String>) -> Self {
        Self::error(id, JsonRpcError::internal_error(details))
    }

    /// Create a parse error response
    ///
    /// # Arguments
    ///
    /// * `details` - Optional error details
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    ///
    /// let response = JsonRpcResponse::parse_error(Some("Unexpected token".to_string()));
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32700);
    /// assert_eq!(response.id, None);
    /// ```
    pub fn parse_error(details: Option<String>) -> Self {
        Self::error(None, JsonRpcError::parse_error(details))
    }

    /// Create an invalid request error response
    ///
    /// # Arguments
    ///
    /// * `details` - Optional error details
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    ///
    /// let response = JsonRpcResponse::invalid_request(Some("Missing 'method' field".to_string()));
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32600);
    /// ```
    pub fn invalid_request(details: Option<String>) -> Self {
        Self::error(None, JsonRpcError::invalid_request(details))
    }

    /// Create a server error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `message` - Error message
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::server_error(
    ///     Some(json!(1)),
    ///     "Service temporarily unavailable".to_string()
    /// );
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32000);
    /// ```
    pub fn server_error(id: Option<Value>, message: String) -> Self {
        Self::error(id, JsonRpcError::server_error(message))
    }

    /// Create a tool not found error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `tool_name` - Name of the tool that was not found
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::tool_not_found(Some(json!(1)), "calculate");
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32004);
    /// ```
    pub fn tool_not_found(id: Option<Value>, tool_name: &str) -> Self {
        Self::error(id, JsonRpcError::tool_not_found(tool_name))
    }

    /// Create a resource not found error response
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier
    /// * `uri` - URI of the resource that was not found
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::resource_not_found(
    ///     Some(json!(1)),
    ///     "file:///data.txt"
    /// );
    ///
    /// assert!(response.error.is_some());
    /// assert_eq!(response.error.unwrap().code, -32003);
    /// ```
    pub fn resource_not_found(id: Option<Value>, uri: &str) -> Self {
        Self::error(id, JsonRpcError::resource_not_found(uri))
    }

    /// Check if this response is a success
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
    /// assert!(response.is_success());
    ///
    /// let response = JsonRpcResponse::method_not_found(Some(json!(1)));
    /// assert!(!response.is_success());
    /// ```
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Check if this response is an error
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::method_not_found(Some(json!(1)));
    /// assert!(response.is_error());
    ///
    /// let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
    /// assert!(!response.is_error());
    /// ```
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

impl Default for JsonRpcResponse {
    /// Create a default JSON-RPC response (success with null result)
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: Some(Value::Null),
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_success_response() {
        let response = JsonRpcResponse::success(
            Some(json!(1)),
            json!({"message": "Success"}),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert_eq!(response.result, Some(json!({"message": "Success"})));
        assert_eq!(response.error, None);
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_error_response() {
        let response = JsonRpcResponse::error(
            Some(json!(1)),
            JsonRpcError::method_not_found(),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert_eq!(response.result, None);
        assert!(response.error.is_some());
        assert!(!response.is_success());
        assert!(response.is_error());
    }

    #[test]
    fn test_method_not_found() {
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
        assert_eq!(response.error.as_ref().unwrap().message, "Method not found");
    }

    #[test]
    fn test_invalid_params() {
        let response = JsonRpcResponse::invalid_params(
            Some(json!(1)),
            "Missing field 'x'",
        );

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32602);
        assert_eq!(response.error.as_ref().unwrap().message, "Invalid params");
    }

    #[test]
    fn test_internal_error() {
        let response = JsonRpcResponse::internal_error(
            Some(json!(1)),
            Some("DB error".to_string()),
        );

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32603);
    }

    #[test]
    fn test_parse_error() {
        let response = JsonRpcResponse::parse_error(Some("Invalid JSON".to_string()));

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32700);
        assert_eq!(response.id, None);
    }

    #[test]
    fn test_invalid_request() {
        let response = JsonRpcResponse::invalid_request(Some("Missing method".to_string()));

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32600);
    }

    #[test]
    fn test_server_error() {
        let response = JsonRpcResponse::server_error(
            Some(json!(1)),
            "Service unavailable".to_string(),
        );

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32000);
        assert_eq!(response.error.as_ref().unwrap().message, "Service unavailable");
    }

    #[test]
    fn test_tool_not_found() {
        let response = JsonRpcResponse::tool_not_found(Some(json!(1)), "my_tool");

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32004);
        assert!(response.error.as_ref().unwrap().message.contains("my_tool"));
    }

    #[test]
    fn test_resource_not_found() {
        let response = JsonRpcResponse::resource_not_found(
            Some(json!(1)),
            "file:///test.txt",
        );

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32003);
        assert!(response.error.as_ref().unwrap().message.contains("file:///test.txt"));
    }

    #[test]
    fn test_serialization_success() {
        let response = JsonRpcResponse::success(
            Some(json!(1)),
            json!({"result": "ok"}),
        );

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: JsonRpcResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_serialization_error() {
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: JsonRpcResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_serialization_omits_result_on_error() {
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));
        let json = serde_json::to_value(&response).unwrap();

        assert!(!json.get("result").is_some());
        assert!(json.get("error").is_some());
    }

    #[test]
    fn test_serialization_omits_error_on_success() {
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        let json = serde_json::to_value(&response).unwrap();

        assert!(json.get("result").is_some());
        assert!(!json.get("error").is_some());
    }

    #[test]
    fn test_is_success() {
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        assert!(response.is_success());

        let response = JsonRpcResponse::method_not_found(Some(json!(1)));
        assert!(!response.is_success());
    }

    #[test]
    fn test_is_error() {
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));
        assert!(response.is_error());

        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        assert!(!response.is_error());
    }

    #[test]
    fn test_default() {
        let response = JsonRpcResponse::default();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, None);
        assert_eq!(response.result, Some(Value::Null));
        assert_eq!(response.error, None);
        assert!(response.is_success());
    }

    #[test]
    fn test_clone() {
        let original = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_debug() {
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("JsonRpcResponse"));
    }
}
