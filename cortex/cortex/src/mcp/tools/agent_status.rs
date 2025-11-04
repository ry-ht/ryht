//! Agent Status Tool
//!
//! Direct integration with AgentRegistry (no HTTP bridge).

use crate::mcp::tools::agent_registry::AgentRegistry;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use mcp_sdk::prelude::*;
use async_trait::async_trait;

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

pub struct AgentStatusTool {
    registry: Arc<AgentRegistry>,
}

impl AgentStatusTool {
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self { registry }
    }

    pub async fn check_status(&self, input: AgentStatusInput) -> Result<AgentStatusOutput> {
        let execution = self.registry.get(&input.agent_id).await
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", input.agent_id))?;

        Ok(AgentStatusOutput {
            agent_id: execution.agent_id,
            status: format!("{:?}", execution.status),
            result: execution.result,
            error: execution.error,
        })
    }
}

#[async_trait]
impl Tool for AgentStatusTool {
    fn name(&self) -> &str {
        "axon.agent.status"
    }

    fn description(&self) -> Option<&str> {
        Some("Check the status of a running agent")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(AgentStatusInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentStatusInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.check_status(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: false,
        })
    }
}
