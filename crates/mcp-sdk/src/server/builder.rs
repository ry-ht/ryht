//! Server builder for fluent API construction.
//!
//! This module provides the `ServerBuilder` type which enables a fluent,
//! type-safe API for constructing MCP servers with tools, resources, middleware, and hooks.

use super::{McpServer, ServerConfig};
use crate::tool::Tool;
use crate::resource::Resource;
use crate::middleware::Middleware;
use crate::hooks::Hook;
use std::sync::Arc;

/// Builder for constructing an `McpServer` with a fluent API.
///
/// `ServerBuilder` provides a convenient way to configure and build an MCP server
/// with tools, resources, middleware, and hooks. It uses the builder pattern
/// to enable method chaining for a clean configuration experience.
///
/// # Examples
///
/// ## Basic Server
///
/// ```
/// use mcp_server::server::ServerBuilder;
/// use mcp_server::tool::{Tool, ToolContext, ToolResult};
/// use mcp_server::error::ToolError;
/// use async_trait::async_trait;
/// use serde_json::{json, Value};
///
/// struct MyTool;
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn name(&self) -> &str { "my_tool" }
///     fn input_schema(&self) -> Value { json!({}) }
///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
///         Ok(ToolResult::success_text("success"))
///     }
/// }
///
/// let server = ServerBuilder::new()
///     .name("my-server")
///     .version("1.0.0")
///     .tool(MyTool)
///     .build();
///
/// assert_eq!(server.config().name(), "my-server");
/// ```
///
/// ## Server with Multiple Tools
///
/// ```
/// use mcp_server::server::ServerBuilder;
/// # use mcp_server::tool::{Tool, ToolContext, ToolResult};
/// # use mcp_server::error::ToolError;
/// # use async_trait::async_trait;
/// # use serde_json::{json, Value};
/// #
/// # struct Tool1;
/// # struct Tool2;
/// # struct Tool3;
/// #
/// # #[async_trait]
/// # impl Tool for Tool1 {
/// #     fn name(&self) -> &str { "tool1" }
/// #     fn input_schema(&self) -> Value { json!({}) }
/// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
/// #         Ok(ToolResult::success_text(""))
/// #     }
/// # }
/// #
/// # #[async_trait]
/// # impl Tool for Tool2 {
/// #     fn name(&self) -> &str { "tool2" }
/// #     fn input_schema(&self) -> Value { json!({}) }
/// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
/// #         Ok(ToolResult::success_text(""))
/// #     }
/// # }
/// #
/// # #[async_trait]
/// # impl Tool for Tool3 {
/// #     fn name(&self) -> &str { "tool3" }
/// #     fn input_schema(&self) -> Value { json!({}) }
/// #     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
/// #         Ok(ToolResult::success_text(""))
/// #     }
/// # }
///
/// let server = ServerBuilder::new()
///     .name("multi-tool-server")
///     .version("1.0.0")
///     .tool(Tool1)
///     .tool(Tool2)
///     .tool(Tool3)
///     .build();
/// ```
pub struct ServerBuilder {
    name: Option<String>,
    version: Option<String>,
    protocol_version: Option<String>,
    tools: Vec<Arc<dyn Tool>>,
    resources: Vec<Arc<dyn Resource>>,
    middleware: Vec<Arc<dyn Middleware>>,
    hooks: Vec<Arc<dyn Hook>>,
}

impl std::fmt::Debug for ServerBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerBuilder")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("protocol_version", &self.protocol_version)
            .field("tools", &format!("<{} tools>", self.tools.len()))
            .field("resources", &format!("<{} resources>", self.resources.len()))
            .field("middleware", &format!("<{} middleware>", self.middleware.len()))
            .field("hooks", &format!("<{} hooks>", self.hooks.len()))
            .finish()
    }
}

impl ServerBuilder {
    /// Creates a new `ServerBuilder`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    ///
    /// let builder = ServerBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            name: None,
            version: None,
            protocol_version: None,
            tools: Vec::new(),
            resources: Vec::new(),
            middleware: Vec::new(),
            hooks: Vec::new(),
        }
    }

    /// Sets the server name.
    ///
    /// This is used during the MCP initialization handshake to identify
    /// the server to clients.
    ///
    /// # Arguments
    ///
    /// * `name` - The server name
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    ///
    /// let builder = ServerBuilder::new()
    ///     .name("my-server");
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the server version.
    ///
    /// This is used during the MCP initialization handshake to identify
    /// the server version to clients.
    ///
    /// # Arguments
    ///
    /// * `version` - The server version
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    ///
    /// let builder = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0");
    /// ```
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the MCP protocol version.
    ///
    /// If not set, defaults to "2025-03-26".
    ///
    /// # Arguments
    ///
    /// * `protocol_version` - The MCP protocol version
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    ///
    /// let builder = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .protocol_version("2025-03-26");
    /// ```
    pub fn protocol_version(mut self, protocol_version: impl Into<String>) -> Self {
        self.protocol_version = Some(protocol_version.into());
        self
    }

    /// Registers a tool with the server.
    ///
    /// Tools are registered and will be available when the server handles
    /// `tools/list` and `tools/call` requests.
    ///
    /// # Arguments
    ///
    /// * `tool` - The tool to register
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    /// use mcp_server::tool::{Tool, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    ///
    /// struct EchoTool;
    ///
    /// #[async_trait]
    /// impl Tool for EchoTool {
    ///     fn name(&self) -> &str { "echo" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text("echo"))
    ///     }
    /// }
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .tool(EchoTool)
    ///     .build();
    /// ```
    pub fn tool<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Arc::new(tool));
        self
    }

    /// Registers multiple tools with the server.
    ///
    /// This is a convenience method for registering multiple tools at once.
    ///
    /// # Arguments
    ///
    /// * `tools` - An iterator of Arc-wrapped tools
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    /// use mcp_server::tool::{Tool, ToolContext, ToolResult};
    /// use mcp_server::error::ToolError;
    /// use async_trait::async_trait;
    /// use serde_json::{json, Value};
    /// use std::sync::Arc;
    ///
    /// struct Tool1;
    /// struct Tool2;
    ///
    /// #[async_trait]
    /// impl Tool for Tool1 {
    ///     fn name(&self) -> &str { "tool1" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// #[async_trait]
    /// impl Tool for Tool2 {
    ///     fn name(&self) -> &str { "tool2" }
    ///     fn input_schema(&self) -> Value { json!({}) }
    ///     async fn execute(&self, _: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
    ///         Ok(ToolResult::success_text(""))
    ///     }
    /// }
    ///
    /// let tools: Vec<Arc<dyn Tool>> = vec![
    ///     Arc::new(Tool1),
    ///     Arc::new(Tool2),
    /// ];
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .tools(tools)
    ///     .build();
    /// ```
    pub fn tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Tool>>,
    {
        self.tools.extend(tools);
        self
    }

    /// Registers a resource with the server.
    ///
    /// Resources provide URI-addressable content that clients can read.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource to register
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    /// use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
    /// use mcp_server::error::ResourceError;
    /// use async_trait::async_trait;
    ///
    /// struct ConfigResource;
    ///
    /// #[async_trait]
    /// impl Resource for ConfigResource {
    ///     fn uri_pattern(&self) -> &str { "app://config" }
    ///     async fn read(&self, _: &str, _: &ResourceContext) -> Result<ResourceContent, ResourceError> {
    ///         Ok(ResourceContent::text("config", "application/json"))
    ///     }
    /// }
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .resource(ConfigResource)
    ///     .build();
    /// ```
    pub fn resource<R: Resource + 'static>(mut self, resource: R) -> Self {
        self.resources.push(Arc::new(resource));
        self
    }

    /// Registers multiple resources with the server.
    ///
    /// # Arguments
    ///
    /// * `resources` - An iterator of Arc-wrapped resources
    pub fn resources<I>(mut self, resources: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Resource>>,
    {
        self.resources.extend(resources);
        self
    }

    /// Registers a middleware with the server.
    ///
    /// Middleware intercepts requests and responses for cross-cutting concerns
    /// like logging, authentication, metrics, etc.
    ///
    /// # Arguments
    ///
    /// * `middleware` - The middleware to register
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    /// use mcp_server::middleware::{Middleware, RequestContext, LoggingMiddleware};
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .middleware(LoggingMiddleware::new())
    ///     .build();
    /// ```
    pub fn middleware<M: Middleware + 'static>(mut self, middleware: M) -> Self {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Registers multiple middleware with the server.
    ///
    /// # Arguments
    ///
    /// * `middleware` - An iterator of Arc-wrapped middleware
    pub fn middlewares<I>(mut self, middleware: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Middleware>>,
    {
        self.middleware.extend(middleware);
        self
    }

    /// Registers a hook with the server.
    ///
    /// Hooks respond to server events for monitoring, auditing, and analytics.
    ///
    /// # Arguments
    ///
    /// * `hook` - The hook to register
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    /// use mcp_server::hooks::{Hook, HookEvent};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct MyHook;
    ///
    /// #[async_trait]
    /// impl Hook for MyHook {
    ///     async fn on_event(&self, _: &HookEvent) -> Result<(), MiddlewareError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .hook(MyHook)
    ///     .build();
    /// ```
    pub fn hook<H: Hook + 'static>(mut self, hook: H) -> Self {
        self.hooks.push(Arc::new(hook));
        self
    }

    /// Registers multiple hooks with the server.
    ///
    /// # Arguments
    ///
    /// * `hooks` - An iterator of Arc-wrapped hooks
    pub fn hooks<I>(mut self, hooks: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn Hook>>,
    {
        self.hooks.extend(hooks);
        self
    }

    /// Builds the `McpServer`.
    ///
    /// This consumes the builder and creates a new `McpServer` instance
    /// with all the configured tools, resources, middleware, and hooks.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Server name is not set
    /// - Server version is not set
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::ServerBuilder;
    ///
    /// let server = ServerBuilder::new()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    ///
    /// assert_eq!(server.config().name(), "my-server");
    /// ```
    pub fn build(self) -> McpServer {
        let config = ServerConfig {
            name: self.name.expect("Server name is required"),
            version: self.version.expect("Server version is required"),
            protocol_version: self.protocol_version.unwrap_or_else(|| "2025-03-26".to_string()),
        };

        McpServer::new(config, self.tools, self.resources, self.middleware, self.hooks)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ToolContext, ToolResult};
    use crate::error::ToolError;
    use async_trait::async_trait;
    use serde_json::{json, Value};

    struct TestTool {
        name: String,
    }

    impl TestTool {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn input_schema(&self) -> Value {
            json!({})
        }

        async fn execute(
            &self,
            _input: Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success_text("test"))
        }
    }

    #[test]
    fn test_builder_new() {
        let builder = ServerBuilder::new();
        assert!(builder.name.is_none());
        assert!(builder.version.is_none());
        assert!(builder.tools.is_empty());
    }

    #[test]
    fn test_builder_default() {
        let builder = ServerBuilder::default();
        assert!(builder.name.is_none());
    }

    #[test]
    fn test_builder_name() {
        let builder = ServerBuilder::new().name("test-server");
        assert_eq!(builder.name, Some("test-server".to_string()));
    }

    #[test]
    fn test_builder_version() {
        let builder = ServerBuilder::new().version("1.2.3");
        assert_eq!(builder.version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_builder_protocol_version() {
        let builder = ServerBuilder::new().protocol_version("custom");
        assert_eq!(builder.protocol_version, Some("custom".to_string()));
    }

    #[test]
    fn test_builder_tool() {
        let builder = ServerBuilder::new().tool(TestTool::new("tool1"));
        assert_eq!(builder.tools.len(), 1);
    }

    #[test]
    fn test_builder_multiple_tools() {
        let builder = ServerBuilder::new()
            .tool(TestTool::new("tool1"))
            .tool(TestTool::new("tool2"))
            .tool(TestTool::new("tool3"));

        assert_eq!(builder.tools.len(), 3);
    }

    #[test]
    fn test_builder_tools_vec() {
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(TestTool::new("tool1")),
            Arc::new(TestTool::new("tool2")),
        ];

        let builder = ServerBuilder::new().tools(tools);
        assert_eq!(builder.tools.len(), 2);
    }

    #[test]
    fn test_builder_build_basic() {
        let server = ServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .build();

        assert_eq!(server.config().name(), "test-server");
        assert_eq!(server.config().version(), "1.0.0");
        assert_eq!(server.config().protocol_version(), "2025-03-26");
    }

    #[test]
    fn test_builder_build_with_protocol_version() {
        let server = ServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .protocol_version("custom-version")
            .build();

        assert_eq!(server.config().protocol_version(), "custom-version");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_builder_build_with_tools() {
        let server = ServerBuilder::new()
            .name("test-server")
            .version("1.0.0")
            .tool(TestTool::new("tool1"))
            .tool(TestTool::new("tool2"))
            .build();

        assert_eq!(server.config().name(), "test-server");
    }

    #[test]
    #[should_panic(expected = "Server name is required")]
    fn test_builder_build_missing_name() {
        ServerBuilder::new().version("1.0.0").build();
    }

    #[test]
    #[should_panic(expected = "Server version is required")]
    fn test_builder_build_missing_version() {
        ServerBuilder::new().name("test").build();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_builder_fluent_api() {
        let server = ServerBuilder::new()
            .name("fluent-server")
            .version("2.0.0")
            .protocol_version("2025-03-26")
            .tool(TestTool::new("tool1"))
            .tool(TestTool::new("tool2"))
            .build();

        assert_eq!(server.config().name(), "fluent-server");
        assert_eq!(server.config().version(), "2.0.0");
    }

    #[test]
    fn test_builder_string_conversion() {
        // Test that Into<String> works with various types
        let server = ServerBuilder::new()
            .name(String::from("owned-string"))
            .version("static-str")
            .build();

        assert_eq!(server.config().name(), "owned-string");
        assert_eq!(server.config().version(), "static-str");
    }
}
