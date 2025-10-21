//! HTTP handlers for MCP endpoints.

use axum::{Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use mcp_server::tool::ToolDefinition;

/// Request to list tools
#[derive(Debug, Deserialize)]
pub struct ListToolsRequest {}

/// Response with tool list
#[derive(Debug, Serialize)]
pub struct ListToolsResponse {
    pub tools: Vec<ToolDefinition>,
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

/// Handler for listing available tools
pub async fn list_tools(
    Json(_payload): Json<ListToolsRequest>,
) -> impl IntoResponse {
    // TODO: Get tools from server instance
    let tools = vec![];
    Json(ListToolsResponse { tools })
}

/// Handler for calling a tool
pub async fn call_tool(
    Json(payload): Json<CallToolRequest>,
) -> Result<Json<CallToolResponse>, StatusCode> {
    tracing::info!("Calling tool: {}", payload.name);

    // TODO: Implement actual tool execution
    let result = serde_json::json!({
        "status": "success",
        "message": format!("Tool {} called with params: {}", payload.name, payload.parameters)
    });

    Ok(Json(CallToolResponse { result }))
}
