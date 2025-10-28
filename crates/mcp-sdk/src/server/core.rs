//! Core MCP server implementation.
//!
//! This module contains the main `McpServer` struct and its request handling logic.

use super::{ServerBuilder, ServerConfig};
use crate::protocol::*;
use crate::tool::{Tool, ToolContext, ToolRegistry};
use serde_json::json;
use std::sync::Arc;

/// Main MCP server instance.
///
/// `McpServer` handles all MCP protocol requests including initialization,
/// tool listing/calling, and resource operations. It maintains registries
/// for tools and resources, and can be extended with middleware and hooks.
///
/// # Examples
///
/// ## Creating a Server
///
/// ```
/// use mcp_server::server::McpServer;
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
///     async fn execute(&self, input: Value, _: &ToolContext) -> Result<ToolResult, ToolError> {
///         Ok(ToolResult::success_text(input["message"].as_str().unwrap_or("")))
///     }
/// }
///
/// let server = McpServer::builder()
///     .name("echo-server")
///     .version("1.0.0")
///     .tool(EchoTool)
///     .build();
///
/// assert_eq!(server.config().name(), "echo-server");
/// ```
///
/// ## Handling Requests
///
/// ```
/// use mcp_server::server::McpServer;
/// use mcp_server::protocol::JsonRpcRequest;
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() {
///     let server = McpServer::builder()
///         .name("test-server")
///         .version("1.0.0")
///         .build();
///
///     let request = JsonRpcRequest::new(
///         Some(json!(1)),
///         "initialize".to_string(),
///         Some(json!({
///             "protocolVersion": "2025-03-26",
///             "capabilities": {},
///             "clientInfo": {
///                 "name": "test-client",
///                 "version": "1.0.0"
///             }
///         }))
///     );
///
///     let response = server.handle_request(request).await;
///     assert!(response.is_success());
/// }
/// ```
#[derive(Debug)]
pub struct McpServer {
    config: ServerConfig,
    tools: Arc<ToolRegistry>,
}

impl McpServer {
    /// Creates a new `McpServer` with the given configuration and tools.
    ///
    /// This is typically called by `ServerBuilder::build()` rather than directly.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `tools` - Vec of Arc-wrapped tools to register
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::{McpServer, ServerConfig};
    /// use std::sync::Arc;
    ///
    /// let config = ServerConfig::new("my-server", "1.0.0");
    /// let tools = vec![];
    /// let server = McpServer::new(config, tools);
    /// ```
    pub fn new(config: ServerConfig, tools: Vec<Arc<dyn Tool>>) -> Self {
        let registry = ToolRegistry::new();

        // Register all tools (ignoring errors for now as this is during construction)
        for tool in tools {
            let _ = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    registry.register_arc(tool).await
                })
            });
        }

        Self {
            config,
            tools: Arc::new(registry),
        }
    }

    /// Creates a new `ServerBuilder` for constructing servers.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::McpServer;
    ///
    /// let server = McpServer::builder()
    ///     .name("my-server")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Returns a reference to the server configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::McpServer;
    ///
    /// let server = McpServer::builder()
    ///     .name("test-server")
    ///     .version("1.0.0")
    ///     .build();
    ///
    /// assert_eq!(server.config().name(), "test-server");
    /// ```
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Returns a reference to the tool registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::McpServer;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let server = McpServer::builder()
    ///         .name("test-server")
    ///         .version("1.0.0")
    ///         .build();
    ///
    ///     let count = server.tools().count().await;
    ///     assert_eq!(count, 0);
    /// }
    /// ```
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Handles a JSON-RPC request and returns a response.
    ///
    /// This is the main entry point for processing MCP protocol requests.
    /// It routes requests to the appropriate handler based on the method name.
    ///
    /// # Supported Methods
    ///
    /// - `initialize` - Server initialization handshake
    /// - `tools/list` - List all available tools
    /// - `tools/call` - Execute a tool
    /// - `resources/list` - List all available resources
    /// - `resources/read` - Read a resource
    ///
    /// # Arguments
    ///
    /// * `request` - The JSON-RPC request to handle
    ///
    /// # Examples
    ///
    /// ```
    /// use mcp_server::server::McpServer;
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let server = McpServer::builder()
    ///         .name("test-server")
    ///         .version("1.0.0")
    ///         .build();
    ///
    ///     let request = JsonRpcRequest::new(
    ///         Some(json!(1)),
    ///         "tools/list".to_string(),
    ///         None
    ///     );
    ///
    ///     let response = server.handle_request(request).await;
    ///     assert!(response.is_success());
    /// }
    /// ```
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await,
            "tools/list" => self.handle_tools_list(request).await,
            "tools/call" => self.handle_tools_call(request).await,
            "resources/list" => self.handle_resources_list(request).await,
            "resources/read" => self.handle_resources_read(request).await,
            _ => JsonRpcResponse::method_not_found(request.id),
        }
    }

    /// Handles the `initialize` request.
    ///
    /// This performs the MCP protocol handshake, negotiating capabilities
    /// with the client and returning server information.
    ///
    /// # Arguments
    ///
    /// * `request` - The initialize request
    ///
    /// # Returns
    ///
    /// A response containing:
    /// - Protocol version
    /// - Server capabilities
    /// - Server information (name, version)
    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Parse initialize params
        let _params = match request.params {
            Some(ref p) => match serde_json::from_value::<InitializeParams>(p.clone()) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::invalid_params(
                        request.id,
                        &format!("Invalid initialize params: {}", e),
                    )
                }
            },
            None => {
                return JsonRpcResponse::invalid_params(
                    request.id,
                    "Initialize params are required",
                )
            }
        };

        // Build server capabilities
        let capabilities = ServerCapabilities::builder()
            .with_tools(ToolsCapability {
                list_changed: Some(false),
            })
            .build();

        // Build initialize result
        let result = InitializeResult {
            protocol_version: self.config.protocol_version.clone(),
            capabilities,
            server_info: ServerInfo {
                name: self.config.name.clone(),
                version: self.config.version.clone(),
            },
        };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => JsonRpcResponse::internal_error(
                request.id,
                Some(format!("Failed to serialize initialize result: {}", e)),
            ),
        }
    }

    /// Handles the `tools/list` request.
    ///
    /// Returns a list of all registered tools with their definitions.
    ///
    /// # Arguments
    ///
    /// * `request` - The tools/list request
    async fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = self.tools.list().await;

        let result = ListToolsResult {
            tools: tools
                .into_iter()
                .map(|def| crate::protocol::ToolDefinition {
                    name: def.name,
                    description: def.description,
                    input_schema: def.input_schema,
                })
                .collect(),
        };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => JsonRpcResponse::internal_error(
                request.id,
                Some(format!("Failed to serialize tools list: {}", e)),
            ),
        }
    }

    /// Handles the `tools/call` request.
    ///
    /// Executes a tool with the provided arguments and returns the result.
    ///
    /// # Arguments
    ///
    /// * `request` - The tools/call request
    async fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Parse call params
        let params = match request.params {
            Some(ref p) => match serde_json::from_value::<CallToolParams>(p.clone()) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::invalid_params(
                        request.id,
                        &format!("Invalid tool call params: {}", e),
                    )
                }
            },
            None => {
                return JsonRpcResponse::invalid_params(request.id, "Tool call params are required")
            }
        };

        // Get the tool
        let tool = match self.tools.get(&params.name).await {
            Some(tool) => tool,
            None => return JsonRpcResponse::tool_not_found(request.id, &params.name),
        };

        // Execute the tool
        let context = ToolContext::builder()
            .request_id(request.id.clone().unwrap_or(json!(null)))
            .build();

        let input = params.arguments.unwrap_or(json!({}));

        match tool.execute(input, &context).await {
            Ok(result) => {
                // Convert ToolResult to CallToolResult
                let call_result = CallToolResult {
                    content: result
                        .content
                        .into_iter()
                        .map(|c| match c {
                            crate::tool::ToolContent::Text { text } => ToolContent::Text { text },
                            crate::tool::ToolContent::Image { data, mime_type } => {
                                ToolContent::Image { data, mime_type }
                            }
                            crate::tool::ToolContent::Resource { uri } => {
                                ToolContent::Resource { uri }
                            }
                        })
                        .collect(),
                    is_error: result.is_error,
                };

                match serde_json::to_value(&call_result) {
                    Ok(value) => JsonRpcResponse::success(request.id, value),
                    Err(e) => JsonRpcResponse::internal_error(
                        request.id,
                        Some(format!("Failed to serialize tool result: {}", e)),
                    ),
                }
            }
            Err(e) => {
                let error: crate::protocol::JsonRpcError = e.into();
                JsonRpcResponse::error(request.id, error)
            }
        }
    }

    /// Handles the `resources/list` request.
    ///
    /// Returns a list of all registered resources.
    ///
    /// # Arguments
    ///
    /// * `request` - The resources/list request
    async fn handle_resources_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // For now, return an empty list since resources aren't implemented yet
        let result = ListResourcesResult { resources: vec![] };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => JsonRpcResponse::internal_error(
                request.id,
                Some(format!("Failed to serialize resources list: {}", e)),
            ),
        }
    }

    /// Handles the `resources/read` request.
    ///
    /// Reads and returns the content of a resource.
    ///
    /// # Arguments
    ///
    /// * `request` - The resources/read request
    async fn handle_resources_read(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Parse read params
        let _params = match request.params {
            Some(ref p) => match serde_json::from_value::<ReadResourceParams>(p.clone()) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::invalid_params(
                        request.id,
                        &format!("Invalid resource read params: {}", e),
                    )
                }
            },
            None => {
                return JsonRpcResponse::invalid_params(
                    request.id,
                    "Resource read params are required",
                )
            }
        };

        // For now, return resource not found since resources aren't fully implemented
        JsonRpcResponse::resource_not_found(request.id, "Resources not yet implemented")
    }

    /// Serves the MCP server using the provided transport.
    ///
    /// This method runs the main server loop, receiving requests from the transport,
    /// processing them through `handle_request()`, and sending responses back.
    /// The loop continues until the transport is closed (e.g., stdin is closed).
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport implementation to use for communication
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::server::McpServer;
    /// use mcp_server::transport::StdioTransport;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server = McpServer::builder()
    ///         .name("my-server")
    ///         .version("1.0.0")
    ///         .build();
    ///
    ///     let transport = StdioTransport::new();
    ///     server.serve(transport).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn serve<T: crate::transport::Transport>(
        &self,
        mut transport: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use tracing::{info, warn};

        info!("MCP server starting");

        loop {
            match transport.recv().await {
                Some(request) => {
                    let response = self.handle_request(request).await;
                    if let Err(e) = transport.send(response).await {
                        warn!("Failed to send response: {}", e);
                        break;
                    }
                }
                None => {
                    info!("Transport closed, shutting down");
                    break;
                }
            }
        }

        info!("MCP server stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ToolError;
    use crate::tool::{ToolResult, ToolContext};
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

        fn description(&self) -> Option<&str> {
            Some("A test tool")
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                }
            })
        }

        async fn execute(
            &self,
            input: Value,
            _context: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            let message = input["message"].as_str().unwrap_or("default");
            Ok(ToolResult::success_text(message))
        }
    }

    #[test]
    fn test_server_new() {
        let config = ServerConfig::new("test-server", "1.0.0");
        let server = McpServer::new(config, vec![]);
        assert_eq!(server.config().name(), "test-server");
    }

    #[test]
    fn test_server_builder() {
        let server = McpServer::builder()
            .name("builder-server")
            .version("2.0.0")
            .build();

        assert_eq!(server.config().name(), "builder-server");
        assert_eq!(server.config().version(), "2.0.0");
    }

    #[test]
    fn test_server_config_getter() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let config = server.config();
        assert_eq!(config.name(), "test");
        assert_eq!(config.version(), "1.0.0");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_server_tools_getter() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .tool(TestTool::new("tool1"))
            .build();

        let tools = server.tools();
        assert!(tools.has("tool1").await);
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let server = McpServer::builder()
            .name("test-server")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "initialize".to_string(),
            Some(json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        );

        let response = server.handle_request(request).await;
        assert!(response.is_success());

        let result: InitializeResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.server_info.name, "test-server");
        assert_eq!(result.protocol_version, "2025-03-26");
    }

    #[tokio::test]
    async fn test_handle_initialize_invalid_params() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request =
            JsonRpcRequest::new(Some(json!(1)), "initialize".to_string(), Some(json!({})));

        let response = server.handle_request(request).await;
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, codes::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_initialize_missing_params() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(Some(json!(1)), "initialize".to_string(), None);

        let response = server.handle_request(request).await;
        assert!(response.is_error());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_tools_list() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .tool(TestTool::new("tool1"))
            .tool(TestTool::new("tool2"))
            .build();

        let request = JsonRpcRequest::new(Some(json!(2)), "tools/list".to_string(), None);

        let response = server.handle_request(request).await;
        assert!(response.is_success());

        let result: ListToolsResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.tools.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_tools_call() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .tool(TestTool::new("echo"))
            .build();

        let request = JsonRpcRequest::new(
            Some(json!(3)),
            "tools/call".to_string(),
            Some(json!({
                "name": "echo",
                "arguments": {
                    "message": "hello"
                }
            })),
        );

        let response = server.handle_request(request).await;
        assert!(response.is_success());

        let result: CallToolResult = serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolContent::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_handle_tools_call_not_found() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(
            Some(json!(4)),
            "tools/call".to_string(),
            Some(json!({
                "name": "nonexistent",
                "arguments": {}
            })),
        );

        let response = server.handle_request(request).await;
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, mcp_codes::TOOL_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_handle_tools_call_invalid_params() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(
            Some(json!(5)),
            "tools/call".to_string(),
            Some(json!({})),
        );

        let response = server.handle_request(request).await;
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, codes::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_resources_list() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(Some(json!(6)), "resources/list".to_string(), None);

        let response = server.handle_request(request).await;
        assert!(response.is_success());

        let result: ListResourcesResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.resources.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_resources_read() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request = JsonRpcRequest::new(
            Some(json!(7)),
            "resources/read".to_string(),
            Some(json!({
                "uri": "test://resource"
            })),
        );

        let response = server.handle_request(request).await;
        assert!(response.is_error());
    }

    #[tokio::test]
    async fn test_handle_method_not_found() {
        let server = McpServer::builder()
            .name("test")
            .version("1.0.0")
            .build();

        let request =
            JsonRpcRequest::new(Some(json!(8)), "unknown/method".to_string(), None);

        let response = server.handle_request(request).await;
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, codes::METHOD_NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_server_with_multiple_tools() {
        let server = McpServer::builder()
            .name("multi-tool")
            .version("1.0.0")
            .tool(TestTool::new("tool1"))
            .tool(TestTool::new("tool2"))
            .tool(TestTool::new("tool3"))
            .build();

        let count = server.tools().count().await;
        assert_eq!(count, 3);
    }
}
