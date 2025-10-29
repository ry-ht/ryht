//! Agent Status Tool

use crate::mcp_server::{AgentRegistry};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
            status: execution.status.to_string(),
            result: execution.result,
            error: execution.error,
        })
    }
}
