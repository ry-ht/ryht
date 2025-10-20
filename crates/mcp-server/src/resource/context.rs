//! Resource context for read operations.
//!
//! This module provides the [`ResourceContext`] struct which carries contextual
//! information about resource read requests, such as session data and metadata.

use std::collections::HashMap;
use serde_json::Value;

/// Context information passed to resource read operations.
///
/// The context provides additional information about the request, including
/// session data, metadata, and any other contextual information that might
/// be useful when reading a resource.
///
/// # Examples
///
/// ## Creating a Context
///
/// ```rust
/// use mcp_server::resource::ResourceContext;
///
/// let context = ResourceContext::new("session-123");
/// assert_eq!(context.session_id(), Some("session-123"));
/// ```
///
/// ## Adding Metadata
///
/// ```rust
/// use mcp_server::resource::ResourceContext;
/// use serde_json::json;
///
/// let mut context = ResourceContext::new("session-123");
/// context.set_metadata("user_id", json!("user-456"));
/// context.set_metadata("role", json!("admin"));
///
/// assert_eq!(
///     context.get_metadata("user_id"),
///     Some(&json!("user-456"))
/// );
/// ```
///
/// ## Using in Resource Implementation
///
/// ```rust
/// use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
/// use mcp_server::error::ResourceError;
/// use async_trait::async_trait;
///
/// struct SecureResource;
///
/// #[async_trait]
/// impl Resource for SecureResource {
///     fn uri_pattern(&self) -> &str {
///         "secure://data"
///     }
///
///     async fn read(
///         &self,
///         _uri: &str,
///         context: &ResourceContext,
///     ) -> Result<ResourceContent, ResourceError> {
///         // Check authorization from context
///         let user_role = context.get_metadata("role")
///             .and_then(|v| v.as_str())
///             .ok_or_else(|| ResourceError::ReadFailed("Unauthorized".to_string()))?;
///
///         if user_role != "admin" {
///             return Err(ResourceError::ReadFailed("Insufficient permissions".to_string()));
///         }
///
///         Ok(ResourceContent::text("Secure data", "text/plain"))
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ResourceContext {
    /// Optional session identifier
    session_id: Option<String>,

    /// Additional metadata as key-value pairs
    metadata: HashMap<String, Value>,
}

impl ResourceContext {
    /// Creates a new resource context with the given session ID.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier for this context
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    ///
    /// let context = ResourceContext::new("session-123");
    /// assert_eq!(context.session_id(), Some("session-123"));
    /// ```
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            metadata: HashMap::new(),
        }
    }

    /// Returns the session ID, if present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    ///
    /// let context = ResourceContext::new("session-123");
    /// assert_eq!(context.session_id(), Some("session-123"));
    ///
    /// let empty_context = ResourceContext::default();
    /// assert_eq!(empty_context.session_id(), None);
    /// ```
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Sets a metadata value for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The metadata key
    /// * `value` - The metadata value (as JSON)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    /// use serde_json::json;
    ///
    /// let mut context = ResourceContext::default();
    /// context.set_metadata("user_id", json!("user-123"));
    /// context.set_metadata("timestamp", json!(1234567890));
    /// ```
    pub fn set_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Gets a metadata value for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The metadata key to look up
    ///
    /// # Returns
    ///
    /// The metadata value if present, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    /// use serde_json::json;
    ///
    /// let mut context = ResourceContext::default();
    /// context.set_metadata("user_id", json!("user-123"));
    ///
    /// assert_eq!(context.get_metadata("user_id"), Some(&json!("user-123")));
    /// assert_eq!(context.get_metadata("nonexistent"), None);
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Returns an iterator over all metadata entries.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    /// use serde_json::json;
    ///
    /// let mut context = ResourceContext::default();
    /// context.set_metadata("key1", json!("value1"));
    /// context.set_metadata("key2", json!("value2"));
    ///
    /// let count = context.metadata().count();
    /// assert_eq!(count, 2);
    /// ```
    pub fn metadata(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.metadata.iter()
    }

    /// Removes a metadata value for the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The metadata key to remove
    ///
    /// # Returns
    ///
    /// The removed value if it existed, or `None` if not found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    /// use serde_json::json;
    ///
    /// let mut context = ResourceContext::default();
    /// context.set_metadata("temp", json!("value"));
    ///
    /// let removed = context.remove_metadata("temp");
    /// assert_eq!(removed, Some(json!("value")));
    /// assert_eq!(context.get_metadata("temp"), None);
    /// ```
    pub fn remove_metadata(&mut self, key: &str) -> Option<Value> {
        self.metadata.remove(key)
    }

    /// Clears all metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::ResourceContext;
    /// use serde_json::json;
    ///
    /// let mut context = ResourceContext::default();
    /// context.set_metadata("key1", json!("value1"));
    /// context.set_metadata("key2", json!("value2"));
    ///
    /// context.clear_metadata();
    /// assert_eq!(context.metadata().count(), 0);
    /// ```
    pub fn clear_metadata(&mut self) {
        self.metadata.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_with_session_id() {
        let context = ResourceContext::new("session-123");
        assert_eq!(context.session_id(), Some("session-123"));
    }

    #[test]
    fn test_default_no_session_id() {
        let context = ResourceContext::default();
        assert_eq!(context.session_id(), None);
    }

    #[test]
    fn test_set_and_get_metadata() {
        let mut context = ResourceContext::default();
        context.set_metadata("key1", json!("value1"));
        context.set_metadata("key2", json!(42));

        assert_eq!(context.get_metadata("key1"), Some(&json!("value1")));
        assert_eq!(context.get_metadata("key2"), Some(&json!(42)));
    }

    #[test]
    fn test_get_nonexistent_metadata() {
        let context = ResourceContext::default();
        assert_eq!(context.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_metadata_iterator() {
        let mut context = ResourceContext::default();
        context.set_metadata("key1", json!("value1"));
        context.set_metadata("key2", json!("value2"));
        context.set_metadata("key3", json!("value3"));

        let count = context.metadata().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_remove_metadata() {
        let mut context = ResourceContext::default();
        context.set_metadata("key", json!("value"));

        let removed = context.remove_metadata("key");
        assert_eq!(removed, Some(json!("value")));
        assert_eq!(context.get_metadata("key"), None);
    }

    #[test]
    fn test_remove_nonexistent_metadata() {
        let mut context = ResourceContext::default();
        let removed = context.remove_metadata("nonexistent");
        assert_eq!(removed, None);
    }

    #[test]
    fn test_clear_metadata() {
        let mut context = ResourceContext::default();
        context.set_metadata("key1", json!("value1"));
        context.set_metadata("key2", json!("value2"));

        context.clear_metadata();
        assert_eq!(context.metadata().count(), 0);
    }

    #[test]
    fn test_context_clone() {
        let mut context1 = ResourceContext::new("session-123");
        context1.set_metadata("key", json!("value"));

        let context2 = context1.clone();
        assert_eq!(context2.session_id(), Some("session-123"));
        assert_eq!(context2.get_metadata("key"), Some(&json!("value")));
    }

    #[test]
    fn test_metadata_overwrite() {
        let mut context = ResourceContext::default();
        context.set_metadata("key", json!("value1"));
        context.set_metadata("key", json!("value2"));

        assert_eq!(context.get_metadata("key"), Some(&json!("value2")));
    }

    #[test]
    fn test_complex_metadata_values() {
        let mut context = ResourceContext::default();

        context.set_metadata("string", json!("text"));
        context.set_metadata("number", json!(123));
        context.set_metadata("bool", json!(true));
        context.set_metadata("array", json!([1, 2, 3]));
        context.set_metadata("object", json!({"nested": "value"}));

        assert_eq!(context.get_metadata("string"), Some(&json!("text")));
        assert_eq!(context.get_metadata("number"), Some(&json!(123)));
        assert_eq!(context.get_metadata("bool"), Some(&json!(true)));
        assert_eq!(context.get_metadata("array"), Some(&json!([1, 2, 3])));
        assert_eq!(context.get_metadata("object"), Some(&json!({"nested": "value"})));
    }
}
