//! Agent Stop Tool

use crate::mcp_server::{AgentRegistry, ExecutionStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
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
        self.registry.update_status(&input.agent_id, ExecutionStatus::Cancelled).await?;

        Ok(AgentStopOutput {
            agent_id: input.agent_id,
            message: "Agent stopped successfully".to_string(),
        })
    }
}
