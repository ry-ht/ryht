//! HTTP handlers for MCP endpoints.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mcp_sdk::prelude::*;
use mcp_sdk::tool::ToolContent;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared state for MCP handlers
#[derive(Clone)]
pub struct McpHandlerState {
    pub server: Arc<mcp_sdk::McpServer>,
}

impl McpHandlerState {
    pub fn new(server: mcp_sdk::McpServer) -> Self {
        Self {
            server: Arc::new(server),
        }
    }
}

/// Request to list tools
#[derive(Debug, Deserialize)]
pub struct ListToolsRequest {}

/// Response with tool list
#[derive(Debug, Serialize)]
pub struct ListToolsResponse {
    pub tools: Vec<mcp_sdk::tool::ToolDefinition>,
}

/// Request to call a tool
#[derive(Debug, Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    pub parameters: serde_json::Value,
}

/// Response from tool call
#[derive(Debug, Serialize)]
pub struct CallToolResponse {
    pub result: serde_json::Value,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Handler for listing available tools
pub async fn list_tools(
    State(state): State<McpHandlerState>,
    Json(_payload): Json<ListToolsRequest>,
) -> impl IntoResponse {
    tracing::info!("Listing available tools");

    // Get the tool registry from the MCP server
    let server = state.server.as_ref();
    let tools = server.tools().list().await;

    tracing::info!("Found {} available tools", tools.len());

    Json(ListToolsResponse { tools })
}

/// Handler for calling a tool
pub async fn call_tool(
    State(state): State<McpHandlerState>,
    Json(payload): Json<CallToolRequest>,
) -> impl IntoResponse {
    tracing::info!("Calling tool: {} with params: {:?}", payload.name, payload.parameters);

    // Get the tool registry from the MCP server
    let server = state.server.as_ref();
    let tool = match server.tools().get(&payload.name).await {
        Some(tool) => tool,
        None => {
            tracing::warn!("Tool not found: {}", payload.name);
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "tool_not_found".to_string(),
                    message: format!("Tool '{}' not found", payload.name),
                }),
            ).into_response();
        }
    };

    // Create tool context
    let context = ToolContext::builder()
        .request_id(serde_json::json!(uuid::Uuid::new_v4().to_string()))
        .build();

    // Execute the tool
    match tool.execute(payload.parameters, &context).await {
        Ok(result) => {
            tracing::info!("Tool '{}' executed successfully", payload.name);

            // Convert ToolResult to JSON
            let result_json = if result.is_error.unwrap_or(false) {
                // Return error result
                serde_json::json!({
                    "is_error": true,
                    "content": result.content.iter().map(|c| {
                        match c {
                            ToolContent::Text { text } => serde_json::json!({
                                "type": "text",
                                "text": text
                            }),
                            ToolContent::Image { data, mime_type } => serde_json::json!({
                                "type": "image",
                                "data": data,
                                "mime_type": mime_type
                            }),
                            ToolContent::Resource { uri } => serde_json::json!({
                                "type": "resource",
                                "uri": uri
                            }),
                        }
                    }).collect::<Vec<_>>()
                })
            } else {
                // Return success result
                serde_json::json!({
                    "is_error": false,
                    "content": result.content.iter().map(|c| {
                        match c {
                            ToolContent::Text { text } => serde_json::json!({
                                "type": "text",
                                "text": text
                            }),
                            ToolContent::Image { data, mime_type } => serde_json::json!({
                                "type": "image",
                                "data": data,
                                "mime_type": mime_type
                            }),
                            ToolContent::Resource { uri } => serde_json::json!({
                                "type": "resource",
                                "uri": uri
                            }),
                        }
                    }).collect::<Vec<_>>()
                })
            };

            Json(CallToolResponse {
                result: result_json,
            }).into_response()
        }
        Err(e) => {
            tracing::error!("Tool '{}' execution failed: {}", payload.name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "execution_failed".to_string(),
                    message: format!("Tool execution failed: {}", e),
                }),
            ).into_response()
        }
    }
}
