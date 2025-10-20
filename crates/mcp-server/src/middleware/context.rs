//! Request context for middleware.
//!
//! This module provides the `RequestContext` type which carries information throughout
//! the request lifecycle. It's shared between middleware layers and provides:
//!
//! - **Timing**: Automatic request timing with start time tracking
//! - **Metadata**: Arbitrary key-value storage for middleware data
//! - **Method Tracking**: Access to the current request method
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use mcp_server::middleware::RequestContext;
//! use serde_json::json;
//!
//! let mut context = RequestContext::new("initialize".to_string());
//!
//! // Store metadata
//! context.set_metadata("user_id".to_string(), json!("12345"));
//!
//! // Retrieve metadata
//! let user_id = context.get_metadata("user_id");
//! assert_eq!(user_id, Some(&json!("12345")));
//!
//! // Check timing
//! if let Some(duration) = context.elapsed() {
//!     println!("Request took: {:?}", duration);
//! }
//! ```
//!
//! ## Middleware Context Sharing
//!
//! ```rust
//! use mcp_server::middleware::{Middleware, RequestContext};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//! use serde_json::json;
//!
//! struct AuthMiddleware;
//!
//! #[async_trait]
//! impl Middleware for AuthMiddleware {
//!     async fn on_request(
//!         &self,
//!         _request: &JsonRpcRequest,
//!         context: &mut RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         // Store authenticated user
//!         context.set_metadata("user".to_string(), json!({"id": "123", "role": "admin"}));
//!         Ok(())
//!     }
//!
//!     async fn on_response(
//!         &self,
//!         _response: &JsonRpcResponse,
//!         _context: &RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         Ok(())
//!     }
//! }
//!
//! struct LoggingMiddleware;
//!
//! #[async_trait]
//! impl Middleware for LoggingMiddleware {
//!     async fn on_request(
//!         &self,
//!         _request: &JsonRpcRequest,
//!         _context: &mut RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         Ok(())
//!     }
//!
//!     async fn on_response(
//!         &self,
//!         _response: &JsonRpcResponse,
//!         context: &RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         // Access user from context set by AuthMiddleware
//!         if let Some(user) = context.get_metadata("user") {
//!             println!("User {} completed request", user["id"]);
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Request context passed through middleware chain.
///
/// The `RequestContext` is created for each request and passed through all middleware.
/// It provides:
/// - Automatic timing from creation
/// - Metadata storage for middleware communication
/// - Access to the request method
///
/// # Thread Safety
///
/// `RequestContext` is not `Sync` because it's meant to be passed through a single
/// middleware chain, not shared across threads. Each request gets its own context.
///
/// # Examples
///
/// ```rust
/// use mcp_server::middleware::RequestContext;
/// use serde_json::json;
/// use std::time::Duration;
///
/// let mut context = RequestContext::new("tools/call".to_string());
///
/// // Access method
/// assert_eq!(context.method(), "tools/call");
///
/// // Store and retrieve metadata
/// context.set_metadata("request_id".to_string(), json!("req-123"));
/// assert_eq!(context.get_metadata("request_id"), Some(&json!("req-123")));
///
/// // Check timing (will be very small)
/// assert!(context.elapsed().is_some());
///
/// // Get start time
/// let start = context.start_time();
/// assert!(start.is_some());
/// ```
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// The request method being processed
    method: String,

    /// When the request started processing
    start_time: Option<Instant>,

    /// Arbitrary metadata storage for middleware
    metadata: HashMap<String, Value>,
}

impl RequestContext {
    /// Create a new request context.
    ///
    /// The context is initialized with:
    /// - The request method
    /// - Start time set to now
    /// - Empty metadata storage
    ///
    /// # Arguments
    ///
    /// * `method` - The JSON-RPC method being called
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    ///
    /// let context = RequestContext::new("initialize".to_string());
    /// assert_eq!(context.method(), "initialize");
    /// assert!(context.start_time().is_some());
    /// ```
    pub fn new(method: String) -> Self {
        Self {
            method,
            start_time: Some(Instant::now()),
            metadata: HashMap::new(),
        }
    }

    /// Get the request method.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    ///
    /// let context = RequestContext::new("tools/list".to_string());
    /// assert_eq!(context.method(), "tools/list");
    /// ```
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the request start time.
    ///
    /// Returns the `Instant` when this context was created.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use std::time::Instant;
    ///
    /// let context = RequestContext::new("test".to_string());
    /// let start = context.start_time();
    /// assert!(start.is_some());
    /// ```
    pub fn start_time(&self) -> Option<Instant> {
        self.start_time
    }

    /// Get elapsed time since request start.
    ///
    /// Returns the duration since the context was created, or `None` if
    /// timing wasn't started.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let context = RequestContext::new("test".to_string());
    /// thread::sleep(Duration::from_millis(10));
    ///
    /// let elapsed = context.elapsed();
    /// assert!(elapsed.is_some());
    /// assert!(elapsed.unwrap() >= Duration::from_millis(10));
    /// ```
    pub fn elapsed(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    /// Set metadata value.
    ///
    /// Stores arbitrary JSON values that can be accessed by other middleware
    /// in the chain. If a value already exists for the key, it's replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - JSON value to store
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    ///
    /// context.set_metadata("user_id".to_string(), json!("123"));
    /// context.set_metadata("role".to_string(), json!("admin"));
    ///
    /// assert_eq!(context.get_metadata("user_id"), Some(&json!("123")));
    /// ```
    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value.
    ///
    /// Returns a reference to the stored value, or `None` if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to lookup
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    /// context.set_metadata("key".to_string(), json!("value"));
    ///
    /// assert_eq!(context.get_metadata("key"), Some(&json!("value")));
    /// assert_eq!(context.get_metadata("missing"), None);
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Remove metadata value.
    ///
    /// Removes and returns the stored value, or `None` if the key doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to remove
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    /// context.set_metadata("key".to_string(), json!("value"));
    ///
    /// let removed = context.remove_metadata("key");
    /// assert_eq!(removed, Some(json!("value")));
    /// assert_eq!(context.get_metadata("key"), None);
    /// ```
    pub fn remove_metadata(&mut self, key: &str) -> Option<Value> {
        self.metadata.remove(key)
    }

    /// Check if metadata key exists.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to check
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    /// context.set_metadata("key".to_string(), json!("value"));
    ///
    /// assert!(context.has_metadata("key"));
    /// assert!(!context.has_metadata("missing"));
    /// ```
    pub fn has_metadata(&self, key: &str) -> bool {
        self.metadata.contains_key(key)
    }

    /// Get all metadata keys.
    ///
    /// Returns an iterator over all metadata keys.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    /// context.set_metadata("key1".to_string(), json!("value1"));
    /// context.set_metadata("key2".to_string(), json!("value2"));
    ///
    /// let keys: Vec<&String> = context.metadata_keys().collect();
    /// assert_eq!(keys.len(), 2);
    /// ```
    pub fn metadata_keys(&self) -> impl Iterator<Item = &String> {
        self.metadata.keys()
    }

    /// Clear all metadata.
    ///
    /// Removes all stored metadata from the context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::RequestContext;
    /// use serde_json::json;
    ///
    /// let mut context = RequestContext::new("test".to_string());
    /// context.set_metadata("key1".to_string(), json!("value1"));
    /// context.set_metadata("key2".to_string(), json!("value2"));
    ///
    /// context.clear_metadata();
    /// assert!(!context.has_metadata("key1"));
    /// assert!(!context.has_metadata("key2"));
    /// ```
    pub fn clear_metadata(&mut self) {
        self.metadata.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_new_context() {
        let context = RequestContext::new("test_method".to_string());
        assert_eq!(context.method(), "test_method");
        assert!(context.start_time().is_some());
        assert!(context.metadata.is_empty());
    }

    #[test]
    fn test_method() {
        let context = RequestContext::new("initialize".to_string());
        assert_eq!(context.method(), "initialize");
    }

    #[test]
    fn test_start_time() {
        let context = RequestContext::new("test".to_string());
        assert!(context.start_time().is_some());
    }

    #[test]
    fn test_elapsed() {
        let context = RequestContext::new("test".to_string());
        thread::sleep(Duration::from_millis(10));

        let elapsed = context.elapsed();
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= Duration::from_millis(10));
    }

    #[test]
    fn test_set_and_get_metadata() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("key1".to_string(), json!("value1"));
        context.set_metadata("key2".to_string(), json!(42));
        context.set_metadata("key3".to_string(), json!({"nested": "object"}));

        assert_eq!(context.get_metadata("key1"), Some(&json!("value1")));
        assert_eq!(context.get_metadata("key2"), Some(&json!(42)));
        assert_eq!(
            context.get_metadata("key3"),
            Some(&json!({"nested": "object"}))
        );
        assert_eq!(context.get_metadata("missing"), None);
    }

    #[test]
    fn test_metadata_overwrite() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("key".to_string(), json!("value1"));
        assert_eq!(context.get_metadata("key"), Some(&json!("value1")));

        context.set_metadata("key".to_string(), json!("value2"));
        assert_eq!(context.get_metadata("key"), Some(&json!("value2")));
    }

    #[test]
    fn test_remove_metadata() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("key".to_string(), json!("value"));
        assert_eq!(context.get_metadata("key"), Some(&json!("value")));

        let removed = context.remove_metadata("key");
        assert_eq!(removed, Some(json!("value")));
        assert_eq!(context.get_metadata("key"), None);

        // Removing non-existent key
        let removed = context.remove_metadata("missing");
        assert_eq!(removed, None);
    }

    #[test]
    fn test_has_metadata() {
        let mut context = RequestContext::new("test".to_string());

        assert!(!context.has_metadata("key"));

        context.set_metadata("key".to_string(), json!("value"));
        assert!(context.has_metadata("key"));

        context.remove_metadata("key");
        assert!(!context.has_metadata("key"));
    }

    #[test]
    fn test_metadata_keys() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("key1".to_string(), json!("value1"));
        context.set_metadata("key2".to_string(), json!("value2"));
        context.set_metadata("key3".to_string(), json!("value3"));

        let mut keys: Vec<&String> = context.metadata_keys().collect();
        keys.sort();

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], "key1");
        assert_eq!(keys[1], "key2");
        assert_eq!(keys[2], "key3");
    }

    #[test]
    fn test_clear_metadata() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("key1".to_string(), json!("value1"));
        context.set_metadata("key2".to_string(), json!("value2"));

        assert!(context.has_metadata("key1"));
        assert!(context.has_metadata("key2"));

        context.clear_metadata();

        assert!(!context.has_metadata("key1"));
        assert!(!context.has_metadata("key2"));
        assert_eq!(context.metadata_keys().count(), 0);
    }

    #[test]
    fn test_clone() {
        let mut context = RequestContext::new("test".to_string());
        context.set_metadata("key".to_string(), json!("value"));

        let cloned = context.clone();

        assert_eq!(cloned.method(), "test");
        assert_eq!(cloned.get_metadata("key"), Some(&json!("value")));
    }

    #[test]
    fn test_debug() {
        let context = RequestContext::new("test".to_string());
        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("RequestContext"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_multiple_metadata_types() {
        let mut context = RequestContext::new("test".to_string());

        context.set_metadata("string".to_string(), json!("text"));
        context.set_metadata("number".to_string(), json!(123));
        context.set_metadata("boolean".to_string(), json!(true));
        context.set_metadata("null".to_string(), Value::Null);
        context.set_metadata("array".to_string(), json!([1, 2, 3]));
        context.set_metadata("object".to_string(), json!({"key": "value"}));

        assert_eq!(context.get_metadata("string"), Some(&json!("text")));
        assert_eq!(context.get_metadata("number"), Some(&json!(123)));
        assert_eq!(context.get_metadata("boolean"), Some(&json!(true)));
        assert_eq!(context.get_metadata("null"), Some(&Value::Null));
        assert_eq!(context.get_metadata("array"), Some(&json!([1, 2, 3])));
        assert_eq!(
            context.get_metadata("object"),
            Some(&json!({"key": "value"}))
        );
    }

    #[test]
    fn test_context_with_empty_method() {
        let context = RequestContext::new("".to_string());
        assert_eq!(context.method(), "");
    }

    #[test]
    fn test_elapsed_timing_accuracy() {
        let context = RequestContext::new("test".to_string());
        let sleep_duration = Duration::from_millis(50);

        thread::sleep(sleep_duration);

        let elapsed = context.elapsed().unwrap();
        // Allow for some timing variation
        assert!(elapsed >= sleep_duration);
        assert!(elapsed < sleep_duration + Duration::from_millis(50));
    }
}
