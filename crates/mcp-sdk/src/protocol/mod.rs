//! MCP Protocol Module
//!
//! This module provides complete implementations of the Model Context Protocol (MCP) specification (2025-03-26).
//!
//! # Overview
//!
//! The protocol module contains all the types and utilities needed to implement MCP communication:
//!
//! - **JSON-RPC 2.0**: Request/Response types and error handling
//! - **Capabilities**: Server and client capability negotiation
//! - **Protocol Types**: All MCP-specific types (tools, resources, prompts, etc.)
//!
//! # Module Structure
//!
//! - [`request`]: JSON-RPC 2.0 request types
//! - [`response`]: JSON-RPC 2.0 response types with helper methods
//! - [`error`]: JSON-RPC error types and MCP error codes
//! - [`capabilities`]: Server and client capabilities
//! - [`types`]: All MCP protocol types
//!
//! # Examples
//!
//! ## Handling Initialize Request
//!
//! ```
//! use mcp_server::protocol::{
//!     JsonRpcRequest, JsonRpcResponse, InitializeParams, InitializeResult,
//!     ServerInfo, ServerCapabilities
//! };
//! use serde_json::json;
//!
//! // Parse initialize request
//! let request = JsonRpcRequest::new(
//!     Some(json!(1)),
//!     "initialize".to_string(),
//!     Some(json!({
//!         "protocolVersion": "2025-03-26",
//!         "capabilities": {},
//!         "clientInfo": {
//!             "name": "test-client",
//!             "version": "1.0.0"
//!         }
//!     }))
//! );
//!
//! // Create successful response
//! let result = InitializeResult {
//!     protocol_version: "2025-03-26".to_string(),
//!     capabilities: ServerCapabilities::default(),
//!     server_info: ServerInfo {
//!         name: "my-server".to_string(),
//!         version: "1.0.0".to_string(),
//!     },
//! };
//!
//! let response = JsonRpcResponse::success(
//!     request.id,
//!     serde_json::to_value(result).unwrap()
//! );
//! ```
//!
//! ## Creating Tool Call Response
//!
//! ```
//! use mcp_server::protocol::{JsonRpcResponse, CallToolResult, ToolContent};
//! use serde_json::json;
//!
//! let result = CallToolResult {
//!     content: vec![
//!         ToolContent::Text {
//!             text: "Operation completed successfully".to_string(),
//!         }
//!     ],
//!     is_error: Some(false),
//! };
//!
//! let response = JsonRpcResponse::success(
//!     Some(json!(1)),
//!     serde_json::to_value(result).unwrap()
//! );
//! ```
//!
//! ## Error Responses
//!
//! ```
//! use mcp_server::protocol::{JsonRpcResponse, JsonRpcError};
//! use serde_json::json;
//!
//! // Method not found
//! let response = JsonRpcResponse::method_not_found(Some(json!(1)));
//!
//! // Tool not found
//! let response = JsonRpcResponse::tool_not_found(Some(json!(1)), "unknown_tool");
//!
//! // Invalid params
//! let response = JsonRpcResponse::invalid_params(
//!     Some(json!(1)),
//!     "Missing required parameter 'name'"
//! );
//! ```

// Module declarations
pub mod capabilities;
pub mod error;
pub mod request;
pub mod response;
pub mod types;

// Re-export commonly used types for convenience
pub use capabilities::{
    ClientCapabilities, LoggingCapability, PromptsCapability, ResourcesCapability,
    RootsCapability, SamplingCapability, ServerCapabilities, ToolsCapability,
};

pub use error::{codes, mcp_codes, JsonRpcError};

pub use request::JsonRpcRequest;

pub use response::JsonRpcResponse;

pub use types::{
    CallToolParams, CallToolResult, ClientInfo, GetPromptParams, GetPromptResult,
    InitializeParams, InitializeResult, ListPromptsResult, ListResourcesResult, ListToolsResult,
    LoggingLevel, LoggingMessageParams, ProgressParams, PromptArgument, PromptDefinition,
    PromptMessage, ReadResourceParams, ReadResourceResult, ResourceContent, ResourceDefinition,
    ServerInfo, ToolContent, ToolDefinition,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_full_initialization_flow() {
        // Client sends initialize request
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

        assert_eq!(request.method, "initialize");
        assert!(!request.is_notification());

        // Parse params
        let params: InitializeParams =
            serde_json::from_value(request.params.unwrap()).unwrap();
        assert_eq!(params.protocol_version, "2025-03-26");
        assert_eq!(params.client_info.name, "test-client");

        // Server responds with capabilities
        let result = InitializeResult {
            protocol_version: "2025-03-26".to_string(),
            capabilities: ServerCapabilities::builder()
                .with_tools(ToolsCapability {
                    list_changed: Some(true),
                })
                .build(),
            server_info: ServerInfo {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let response = JsonRpcResponse::success(
            Some(json!(1)),
            serde_json::to_value(&result).unwrap(),
        );

        assert!(response.is_success());
        assert_eq!(response.id, Some(json!(1)));
    }

    #[test]
    fn test_tool_call_flow() {
        // Client calls a tool
        let request = JsonRpcRequest::new(
            Some(json!(2)),
            "tools/call".to_string(),
            Some(json!({
                "name": "echo",
                "arguments": {
                    "message": "hello"
                }
            })),
        );

        let params: CallToolParams = serde_json::from_value(request.params.unwrap()).unwrap();
        assert_eq!(params.name, "echo");

        // Server returns result
        let result = CallToolResult {
            content: vec![ToolContent::Text {
                text: "hello".to_string(),
            }],
            is_error: Some(false),
        };

        let response = JsonRpcResponse::success(
            Some(json!(2)),
            serde_json::to_value(&result).unwrap(),
        );

        assert!(response.is_success());
    }

    #[test]
    fn test_error_responses() {
        // Method not found
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));
        assert!(response.is_error());
        assert_eq!(response.error.as_ref().unwrap().code, codes::METHOD_NOT_FOUND);

        // Tool not found
        let response = JsonRpcResponse::tool_not_found(Some(json!(2)), "unknown");
        assert!(response.is_error());
        assert_eq!(
            response.error.as_ref().unwrap().code,
            mcp_codes::TOOL_NOT_FOUND
        );

        // Invalid params
        let response = JsonRpcResponse::invalid_params(Some(json!(3)), "Missing field");
        assert!(response.is_error());
        assert_eq!(response.error.as_ref().unwrap().code, codes::INVALID_PARAMS);
    }

    #[test]
    fn test_resource_read_flow() {
        // Client reads a resource
        let request = JsonRpcRequest::new(
            Some(json!(3)),
            "resources/read".to_string(),
            Some(json!({
                "uri": "file:///config.json"
            })),
        );

        let params: ReadResourceParams = serde_json::from_value(request.params.unwrap()).unwrap();
        assert_eq!(params.uri, "file:///config.json");

        // Server returns content
        let result = ReadResourceResult {
            contents: vec![ResourceContent::Text {
                uri: "file:///config.json".to_string(),
                mime_type: Some("application/json".to_string()),
                text: r#"{"key": "value"}"#.to_string(),
            }],
        };

        let response = JsonRpcResponse::success(
            Some(json!(3)),
            serde_json::to_value(&result).unwrap(),
        );

        assert!(response.is_success());
    }

    #[test]
    fn test_list_tools_flow() {
        let request = JsonRpcRequest::new(Some(json!(4)), "tools/list".to_string(), None);

        assert_eq!(request.method, "tools/list");

        let result = ListToolsResult {
            tools: vec![
                ToolDefinition {
                    name: "echo".to_string(),
                    description: Some("Echo a message".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string"}
                        },
                        "required": ["message"]
                    }),
                },
                ToolDefinition {
                    name: "calculate".to_string(),
                    description: Some("Perform calculation".to_string()),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "expression": {"type": "string"}
                        }
                    }),
                },
            ],
        };

        let response = JsonRpcResponse::success(
            Some(json!(4)),
            serde_json::to_value(&result).unwrap(),
        );

        assert!(response.is_success());
        let result_value = response.result.unwrap();
        assert_eq!(result_value["tools"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_notification_flow() {
        // Server sends a notification (no id)
        let notification = JsonRpcRequest::notification(
            "notifications/message".to_string(),
            Some(json!({
                "level": "info",
                "logger": "test-server",
                "data": "Server started"
            })),
        );

        assert!(notification.is_notification());
        assert_eq!(notification.id, None);

        let params: LoggingMessageParams =
            serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(params.level, LoggingLevel::Info);
        assert_eq!(params.data, "Server started");
    }

    #[test]
    fn test_progress_notification() {
        let notification = JsonRpcRequest::notification(
            "notifications/progress".to_string(),
            Some(json!({
                "progressToken": "task-123",
                "progress": 50.0,
                "total": 100.0
            })),
        );

        let params: ProgressParams = serde_json::from_value(notification.params.unwrap()).unwrap();
        assert_eq!(params.progress_token, "task-123");
        assert_eq!(params.progress, 50.0);
        assert_eq!(params.total, Some(100.0));
    }

    #[test]
    fn test_all_error_types() {
        // Standard JSON-RPC errors
        assert_eq!(codes::PARSE_ERROR, -32700);
        assert_eq!(codes::INVALID_REQUEST, -32600);
        assert_eq!(codes::METHOD_NOT_FOUND, -32601);
        assert_eq!(codes::INVALID_PARAMS, -32602);
        assert_eq!(codes::INTERNAL_ERROR, -32603);

        // MCP-specific errors
        assert_eq!(mcp_codes::SERVER_ERROR, -32000);
        assert_eq!(mcp_codes::TIMEOUT_ERROR, -32001);
        assert_eq!(mcp_codes::CONNECTION_ERROR, -32002);
        assert_eq!(mcp_codes::RESOURCE_NOT_FOUND, -32003);
        assert_eq!(mcp_codes::TOOL_NOT_FOUND, -32004);

        // Test error creation
        let error = JsonRpcError::parse_error(None);
        assert_eq!(error.code, codes::PARSE_ERROR);

        let error = JsonRpcError::method_not_found();
        assert_eq!(error.code, codes::METHOD_NOT_FOUND);

        let error = JsonRpcError::tool_not_found("test");
        assert_eq!(error.code, mcp_codes::TOOL_NOT_FOUND);
    }

    #[test]
    fn test_capability_negotiation() {
        let server_caps = ServerCapabilities::builder()
            .with_tools(ToolsCapability {
                list_changed: Some(true),
            })
            .with_resources(ResourcesCapability {
                subscribe: Some(true),
                list_changed: Some(true),
            })
            .with_logging(LoggingCapability {})
            .build();

        assert!(server_caps.tools.is_some());
        assert!(server_caps.resources.is_some());
        assert!(server_caps.logging.is_some());
        assert!(server_caps.prompts.is_none());

        let client_caps = ClientCapabilities {
            roots: Some(RootsCapability {
                list_changed: Some(true),
            }),
            sampling: Some(SamplingCapability {}),
            experimental: Default::default(),
        };

        assert!(client_caps.roots.is_some());
        assert!(client_caps.sampling.is_some());
    }

    #[test]
    fn test_roundtrip_serialization() {
        // Test that all types can be serialized and deserialized
        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "test".to_string(),
            Some(json!({"key": "value"})),
        );
        let json = serde_json::to_string(&request).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, parsed);

        let response = JsonRpcResponse::success(Some(json!(1)), json!({"result": "ok"}));
        let json = serde_json::to_string(&response).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, parsed);

        let error = JsonRpcError::method_not_found();
        let json = serde_json::to_string(&error).unwrap();
        let parsed: JsonRpcError = serde_json::from_str(&json).unwrap();
        assert_eq!(error, parsed);
    }
}
