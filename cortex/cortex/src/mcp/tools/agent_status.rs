//! Agent Status Tool
//!
//! TODO (Phase 6): Implement actual agent status tracking
//! Currently a stub that returns "not found" for all agents

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentStatusInput {
    pub agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct AgentStatusOutput {
    pub agent_id: String,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub struct AgentStatusContext;

impl AgentStatusContext {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentStatusContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AgentStatusTool {
    context: AgentStatusContext,
}

impl AgentStatusTool {
    pub fn new(context: AgentStatusContext) -> Self {
        Self { context }
    }

    pub async fn check_status(&self, input: AgentStatusInput) -> Result<AgentStatusOutput> {
        tracing::info!(agent_id = %input.agent_id, "Agent status check requested (STUB)");

        Ok(AgentStatusOutput {
            agent_id: input.agent_id.clone(),
            status: "not_found".to_string(),
            result: None,
            error: Some("Agent status tracking not yet implemented. This is a stub.".to_string()),
        })
    }
}

impl mcp_sdk::Tool for AgentStatusTool {
    fn name(&self) -> &str {
        "axon_agent_status"
    }

    fn description(&self) -> Option<&str> {
        Some("Check the status of a running agent by agent_id. Returns current status, progress, and results if completed.")
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "The agent ID to check status for"
                }
            },
            "required": ["agent_id"]
        })
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<serde_json::Value, mcp_sdk::Error> {
        let input: AgentStatusInput = serde_json::from_value(arguments)
            .map_err(|e| mcp_sdk::Error::InvalidParams(e.to_string()))?;

        let output = self.check_status(input).await
            .map_err(|e| mcp_sdk::Error::InternalError(e.to_string()))?;

        serde_json::to_value(output)
            .map_err(|e| mcp_sdk::Error::InternalError(e.to_string()))
    }
}
