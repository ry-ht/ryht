//! Agent Stop Tool
//!
//! Direct integration with AgentRegistry (no HTTP bridge).

use crate::mcp::tools::agent_registry::{AgentRegistry, ExecutionStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use mcp_sdk::prelude::*;
use async_trait::async_trait;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AgentStopInput {
    pub agent_id: String,
}

#[derive(Debug, Serialize)]
pub struct AgentStopOutput {
    pub agent_id: String,
    pub message: String,
}

pub struct AgentStopTool {
    registry: Arc<AgentRegistry>,
}

impl AgentStopTool {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }

    pub async fn stop_agent(&self, input: AgentStopInput) -> Result<AgentStopOutput> {
        // Mark agent as failed (we don't have a Cancelled status yet)
        self.registry.set_error(&input.agent_id, "Stopped by user".to_string()).await?;

        Ok(AgentStopOutput {
            agent_id: input.agent_id,
            message: "Agent stopped successfully".to_string(),
        })
    }
}

#[async_trait]
impl Tool for AgentStopTool {
    fn name(&self) -> &str {
        "axon.agent.stop"
    }

    fn description(&self) -> Option<&str> {
        Some("Stop a running agent")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(AgentStopInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentStopInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.stop_agent(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: Some(false),
        })
    }
}
