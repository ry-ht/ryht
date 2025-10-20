//! Tool execution context.
//!
//! This module provides the `ToolContext` struct which carries session information
//! and other contextual data during tool execution.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Context information provided to tools during execution.
///
/// `ToolContext` carries session information, request metadata, and any custom
/// data that tools might need during execution. It is passed to every tool's
/// `execute` method.
///
/// # Examples
///
/// ```
/// use mcp_server::tool::ToolContext;
///
/// let context = ToolContext::builder()
///     .session_id("session-123")
///     .client_info("my-client", "1.0.0")
///     .build();
///
/// assert_eq!(context.session_id(), Some("session-123"));
/// ```
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Unique session identifier
    session_id: Option<String>,

    /// Client name
    client_name: Option<String>,

    /// Client version
    client_version: Option<String>,

    /// Request ID from JSON-RPC
    request_id: Option<Value>,

    /// Custom metadata
    metadata: Arc<HashMap<String, Value>>,
}

impl ToolContext {
    /// Creates a new empty `ToolContext`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContext;
    ///
    /// let context = ToolContext::new();
    /// assert!(context.session_id().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            session_id: None,
            client_name: None,
            client_version: None,
            request_id: None,
            metadata: Arc::new(HashMap::new()),
        }
    }

    /// Creates a new `ToolContextBuilder` for constructing a context.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContext;
    ///
    /// let context = ToolContext::builder()
    ///     .session_id("session-123")
    ///     .build();
    /// ```
    pub fn builder() -> ToolContextBuilder {
        ToolContextBuilder::new()
    }

    /// Returns the session ID if set.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContext;
    ///
    /// let context = ToolContext::builder()
    ///     .session_id("session-123")
    ///     .build();
    ///
    /// assert_eq!(context.session_id(), Some("session-123"));
    /// ```
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Returns the client name if set.
    pub fn client_name(&self) -> Option<&str> {
        self.client_name.as_deref()
    }

    /// Returns the client version if set.
    pub fn client_version(&self) -> Option<&str> {
        self.client_version.as_deref()
    }

    /// Returns the request ID if set.
    pub fn request_id(&self) -> Option<&Value> {
        self.request_id.as_ref()
    }

    /// Returns a reference to the metadata map.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContext;
    /// use serde_json::json;
    ///
    /// let context = ToolContext::builder()
    ///     .metadata("key", json!("value"))
    ///     .build();
    ///
    /// assert_eq!(context.metadata().get("key"), Some(&json!("value")));
    /// ```
    pub fn metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Gets a metadata value by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::tool::ToolContext;
    /// use serde_json::json;
    ///
    /// let context = ToolContext::builder()
    ///     .metadata("user_id", json!(42))
    ///     .build();
    ///
    /// assert_eq!(context.get_metadata("user_id"), Some(&json!(42)));
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a `ToolContext`.
///
/// # Examples
///
/// ```
/// use mcp_server::tool::ToolContext;
/// use serde_json::json;
///
/// let context = ToolContext::builder()
///     .session_id("session-123")
///     .client_info("my-client", "1.0.0")
///     .metadata("user_id", json!(42))
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct ToolContextBuilder {
    session_id: Option<String>,
    client_name: Option<String>,
    client_version: Option<String>,
    request_id: Option<Value>,
    metadata: HashMap<String, Value>,
}

impl ToolContextBuilder {
    /// Creates a new `ToolContextBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the session ID.
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    /// Sets the client information.
    pub fn client_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.client_name = Some(name.into());
        self.client_version = Some(version.into());
        self
    }

    /// Sets the client name.
    pub fn client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = Some(name.into());
        self
    }

    /// Sets the client version.
    pub fn client_version(mut self, version: impl Into<String>) -> Self {
        self.client_version = Some(version.into());
        self
    }

    /// Sets the request ID.
    pub fn request_id(mut self, id: Value) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Adds a metadata key-value pair.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Builds the `ToolContext`.
    pub fn build(self) -> ToolContext {
        ToolContext {
            session_id: self.session_id,
            client_name: self.client_name,
            client_version: self.client_version,
            request_id: self.request_id,
            metadata: Arc::new(self.metadata),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_context_new() {
        let context = ToolContext::new();
        assert!(context.session_id().is_none());
        assert!(context.client_name().is_none());
        assert!(context.metadata().is_empty());
    }

    #[test]
    fn test_context_default() {
        let context = ToolContext::default();
        assert!(context.session_id().is_none());
    }

    #[test]
    fn test_context_builder_session_id() {
        let context = ToolContext::builder()
            .session_id("test-session")
            .build();
        assert_eq!(context.session_id(), Some("test-session"));
    }

    #[test]
    fn test_context_builder_client_info() {
        let context = ToolContext::builder()
            .client_info("my-client", "1.2.3")
            .build();
        assert_eq!(context.client_name(), Some("my-client"));
        assert_eq!(context.client_version(), Some("1.2.3"));
    }

    #[test]
    fn test_context_builder_client_name() {
        let context = ToolContext::builder().client_name("test-client").build();
        assert_eq!(context.client_name(), Some("test-client"));
    }

    #[test]
    fn test_context_builder_client_version() {
        let context = ToolContext::builder().client_version("2.0.0").build();
        assert_eq!(context.client_version(), Some("2.0.0"));
    }

    #[test]
    fn test_context_builder_request_id() {
        let context = ToolContext::builder().request_id(json!(123)).build();
        assert_eq!(context.request_id(), Some(&json!(123)));
    }

    #[test]
    fn test_context_builder_metadata() {
        let context = ToolContext::builder()
            .metadata("key1", json!("value1"))
            .metadata("key2", json!(42))
            .build();

        assert_eq!(context.get_metadata("key1"), Some(&json!("value1")));
        assert_eq!(context.get_metadata("key2"), Some(&json!(42)));
        assert_eq!(context.get_metadata("key3"), None);
    }

    #[test]
    fn test_context_builder_full() {
        let context = ToolContext::builder()
            .session_id("session-123")
            .client_info("client", "1.0")
            .request_id(json!(456))
            .metadata("user", json!("alice"))
            .metadata("role", json!("admin"))
            .build();

        assert_eq!(context.session_id(), Some("session-123"));
        assert_eq!(context.client_name(), Some("client"));
        assert_eq!(context.client_version(), Some("1.0"));
        assert_eq!(context.request_id(), Some(&json!(456)));
        assert_eq!(context.get_metadata("user"), Some(&json!("alice")));
        assert_eq!(context.get_metadata("role"), Some(&json!("admin")));
    }

    #[test]
    fn test_context_clone() {
        let context1 = ToolContext::builder()
            .session_id("session-456")
            .metadata("key", json!("value"))
            .build();

        let context2 = context1.clone();
        assert_eq!(context2.session_id(), Some("session-456"));
        assert_eq!(context2.get_metadata("key"), Some(&json!("value")));
    }

    #[test]
    fn test_metadata_immutability() {
        let context = ToolContext::builder()
            .metadata("key", json!("value"))
            .build();

        // Cloning should share the same Arc
        let _context2 = context.clone();
        assert_eq!(context.get_metadata("key"), Some(&json!("value")));
    }
}
