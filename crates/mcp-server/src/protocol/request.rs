//! JSON-RPC 2.0 Request types
//!
//! This module provides types for JSON-RPC 2.0 requests according to the specification.
//!
//! # Specification
//!
//! A rpc call is represented by sending a Request object to a Server. The Request object has the following members:
//!
//! - `jsonrpc`: A String specifying the version of the JSON-RPC protocol. MUST be exactly "2.0"
//! - `method`: A String containing the name of the method to be invoked
//! - `params` (optional): A Structured value that holds the parameter values to be used during the invocation
//! - `id` (optional): An identifier established by the Client. If not included, it's a notification
//!
//! # Examples
//!
//! ```
//! use mcp_server::protocol::JsonRpcRequest;
//! use serde_json::json;
//!
//! // Request with params
//! let request = JsonRpcRequest::new(
//!     Some(json!(1)),
//!     "initialize".to_string(),
//!     Some(json!({"protocolVersion": "2025-03-26"}))
//! );
//!
//! // Notification (no id)
//! let notification = JsonRpcRequest::notification(
//!     "notifications/message".to_string(),
//!     Some(json!({"level": "info", "message": "Starting..."}))
//! );
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request object
///
/// A Request object represents a call to a method on the server.
///
/// # Examples
///
/// ```
/// use mcp_server::protocol::JsonRpcRequest;
/// use serde_json::json;
///
/// // Create a request
/// let request = JsonRpcRequest::new(
///     Some(json!(1)),
///     "tools/call".to_string(),
///     Some(json!({"name": "echo", "arguments": {"message": "hello"}}))
/// );
///
/// assert_eq!(request.jsonrpc, "2.0");
/// assert_eq!(request.method, "tools/call");
/// assert!(request.id.is_some());
/// assert!(request.params.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Request identifier (if None, this is a notification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// Method name to invoke
    pub method: String,

    /// Method parameters (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier (None for notifications)
    /// * `method` - Method name
    /// * `params` - Optional parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let request = JsonRpcRequest::new(
    ///     Some(json!(42)),
    ///     "initialize".to_string(),
    ///     Some(json!({"protocolVersion": "2025-03-26"}))
    /// );
    ///
    /// assert_eq!(request.id, Some(json!(42)));
    /// assert_eq!(request.method, "initialize");
    /// ```
    pub fn new(id: Option<Value>, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }

    /// Create a new JSON-RPC notification (request without an id)
    ///
    /// Notifications are requests that don't expect a response.
    ///
    /// # Arguments
    ///
    /// * `method` - Method name
    /// * `params` - Optional parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let notification = JsonRpcRequest::notification(
    ///     "notifications/message".to_string(),
    ///     Some(json!({"level": "info", "message": "Operation complete"}))
    /// );
    ///
    /// assert!(notification.id.is_none());
    /// assert_eq!(notification.method, "notifications/message");
    /// ```
    pub fn notification(method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method,
            params,
        }
    }

    /// Check if this request is a notification
    ///
    /// A notification is a request without an id field.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
    /// assert!(!request.is_notification());
    ///
    /// let notification = JsonRpcRequest::notification("test".to_string(), None);
    /// assert!(notification.is_notification());
    /// ```
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Get the request ID as a string (if present)
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let request = JsonRpcRequest::new(Some(json!(42)), "test".to_string(), None);
    /// assert_eq!(request.id_as_string(), Some("42".to_string()));
    ///
    /// let request = JsonRpcRequest::new(Some(json!("abc")), "test".to_string(), None);
    /// assert_eq!(request.id_as_string(), Some("\"abc\"".to_string()));
    ///
    /// let notification = JsonRpcRequest::notification("test".to_string(), None);
    /// assert_eq!(notification.id_as_string(), None);
    /// ```
    pub fn id_as_string(&self) -> Option<String> {
        self.id.as_ref().map(|id| id.to_string())
    }
}

impl Default for JsonRpcRequest {
    /// Create a default JSON-RPC request
    ///
    /// Creates a notification with an empty method name.
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: String::new(),
            params: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_request() {
        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "test_method".to_string(),
            Some(json!({"key": "value"})),
        );

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(json!(1)));
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, Some(json!({"key": "value"})));
    }

    #[test]
    fn test_new_request_without_params() {
        let request = JsonRpcRequest::new(
            Some(json!(2)),
            "test_method".to_string(),
            None,
        );

        assert_eq!(request.id, Some(json!(2)));
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, None);
    }

    #[test]
    fn test_notification() {
        let notification = JsonRpcRequest::notification(
            "notification_method".to_string(),
            Some(json!({"data": "value"})),
        );

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.id, None);
        assert_eq!(notification.method, "notification_method");
        assert_eq!(notification.params, Some(json!({"data": "value"})));
    }

    #[test]
    fn test_is_notification() {
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        assert!(!request.is_notification());

        let notification = JsonRpcRequest::notification("test".to_string(), None);
        assert!(notification.is_notification());
    }

    #[test]
    fn test_id_as_string() {
        // Number ID
        let request = JsonRpcRequest::new(Some(json!(42)), "test".to_string(), None);
        assert_eq!(request.id_as_string(), Some("42".to_string()));

        // String ID
        let request = JsonRpcRequest::new(Some(json!("abc")), "test".to_string(), None);
        assert_eq!(request.id_as_string(), Some("\"abc\"".to_string()));

        // No ID
        let notification = JsonRpcRequest::notification("test".to_string(), None);
        assert_eq!(notification.id_as_string(), None);
    }

    #[test]
    fn test_serialization() {
        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "test".to_string(),
            Some(json!({"param": "value"})),
        );

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_serialization_without_id() {
        let notification = JsonRpcRequest::notification("test".to_string(), None);
        let json = serde_json::to_value(&notification).unwrap();

        assert!(!json.get("id").is_some());
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "test");
    }

    #[test]
    fn test_serialization_without_params() {
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let json = serde_json::to_value(&request).unwrap();

        assert!(!json.get("params").is_some());
        assert_eq!(json["id"], 1);
    }

    #[test]
    fn test_deserialization_from_spec() {
        // Example from JSON-RPC spec
        let json = r#"{"jsonrpc": "2.0", "method": "subtract", "params": [42, 23], "id": 1}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "subtract");
        assert_eq!(request.id, Some(json!(1)));
        assert_eq!(request.params, Some(json!([42, 23])));
    }

    #[test]
    fn test_deserialization_notification() {
        let json = r#"{"jsonrpc": "2.0", "method": "update"}"#;
        let notification: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert!(notification.is_notification());
        assert_eq!(notification.method, "update");
    }

    #[test]
    fn test_default() {
        let request = JsonRpcRequest::default();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, None);
        assert_eq!(request.method, "");
        assert_eq!(request.params, None);
        assert!(request.is_notification());
    }

    #[test]
    fn test_various_id_types() {
        // Number ID
        let request = JsonRpcRequest::new(Some(json!(123)), "test".to_string(), None);
        assert_eq!(request.id, Some(json!(123)));

        // String ID
        let request = JsonRpcRequest::new(Some(json!("req-123")), "test".to_string(), None);
        assert_eq!(request.id, Some(json!("req-123")));

        // Null ID (valid in JSON-RPC 2.0)
        let request = JsonRpcRequest::new(Some(Value::Null), "test".to_string(), None);
        assert_eq!(request.id, Some(Value::Null));
    }

    #[test]
    fn test_clone() {
        let original = JsonRpcRequest::new(
            Some(json!(1)),
            "test".to_string(),
            Some(json!({"key": "value"})),
        );
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_debug() {
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("JsonRpcRequest"));
        assert!(debug_str.contains("test"));
    }
}
