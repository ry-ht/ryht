//! Agent Stop Tool
//!
//! TODO (Phase 6): Implement actual agent stopping
//! Currently a stub that returns success

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentStopInput {
    pub agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct AgentStopOutput {
    pub agent_id: String,
    pub message: String,
}

pub struct AgentStopContext;

impl AgentStopContext {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentStopContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AgentStopTool {
    context: AgentStopContext,
}

impl AgentStopTool {
    pub fn new(context: AgentStopContext) -> Self {
        Self { context }
    }

    pub async fn stop_agent(&self, input: AgentStopInput) -> Result<AgentStopOutput> {
        tracing::info!(agent_id = %input.agent_id, "Agent stop requested (STUB)");

        Ok(AgentStopOutput {
            agent_id: input.agent_id,
            message: "Agent stop not yet implemented. This is a stub.".to_string(),
        })
    }
}

impl mcp_sdk::Tool for AgentStopTool {
    fn name(&self) -> &str {
        "axon_agent_stop"
    }

    fn description(&self) -> Option<&str> {
        Some("Stop a running agent by agent_id. The agent will be cancelled and resources released.")
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "The agent ID to stop"
                }
            },
            "required": ["agent_id"]
        })
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<serde_json::Value, mcp_sdk::Error> {
        let input: AgentStopInput = serde_json::from_value(arguments)
            .map_err(|e| mcp_sdk::Error::InvalidParams(e.to_string()))?;

        let output = self.stop_agent(input).await
            .map_err(|e| mcp_sdk::Error::InternalError(e.to_string()))?;

        serde_json::to_value(output)
            .map_err(|e| mcp_sdk::Error::InternalError(e.to_string()))
    }
}
