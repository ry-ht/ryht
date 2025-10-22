//! HTTP handlers for MCP endpoints.

use axum::{Json, response::IntoResponse, http::StatusCode};
use serde::{Deserialize, Serialize};
use mcp_sdk::tool::ToolDefinition;

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
    use crate::mcp::tools;

    // Get all registered tool definitions
    let tools = tools::get_tools();
    tracing::info!("Listing {} available tools", tools.len());

    Json(ListToolsResponse { tools })
}

/// Handler for calling a tool
pub async fn call_tool(
    Json(payload): Json<CallToolRequest>,
) -> Result<Json<CallToolResponse>, StatusCode> {
    tracing::info!("Calling tool: {} with params: {:?}", payload.name, payload.parameters);

    // Note: In a full implementation, this would:
    // 1. Look up the tool by name from a registry
    // 2. Deserialize parameters into the tool's input type
    // 3. Execute the tool with the proper context
    // 4. Return the tool's output as JSON
    //
    // For now, we return a placeholder response indicating the tool was called.
    // The actual execution would happen through the MCP SDK's tool dispatch mechanism.

    let result = serde_json::json!({
        "status": "pending_implementation",
        "tool": payload.name,
        "message": "Tool execution requires MCP server context and proper dispatch mechanism",
        "parameters_received": payload.parameters,
    });

    tracing::warn!(
        "Tool execution not fully implemented. Tool '{}' call recorded but not executed.",
        payload.name
    );

    Ok(Json(CallToolResponse { result }))
}
