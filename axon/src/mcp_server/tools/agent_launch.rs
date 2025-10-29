//! Agent Launch Tool - Launch specialized agents for tasks

use crate::mcp_server::{AgentExecution, AgentRegistry, ExecutionStatus, McpServerConfig};
use crate::cortex_bridge::CortexBridge;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Agent launch tool input
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentLaunchInput {
    /// Agent type to launch
    pub agent_type: String,

    /// Task description
    pub task: String,

    /// Workspace ID (optional)
    pub workspace_id: Option<String>,

    /// Additional parameters (agent-specific)
    pub params: Option<serde_json::Value>,
}

/// Agent launch tool output
#[derive(Debug, Serialize)]
pub struct AgentLaunchOutput {
    /// Agent ID
    pub agent_id: String,

    /// Agent type
    pub agent_type: String,

    /// Status
    pub status: String,

    /// Message
    pub message: String,
}

/// Agent launch tool
pub struct AgentLaunchTool {
    config: Arc<McpServerConfig>,
    registry: Arc<AgentRegistry>,
    cortex: Arc<CortexBridge>,
}

impl AgentLaunchTool {
    /// Create new agent launch tool
    pub fn new(
        config: Arc<McpServerConfig>,
        registry: Arc<AgentRegistry>,
        cortex: Arc<CortexBridge>,
    ) -> Self {
        Self {
            config,
            registry,
            cortex,
        }
    }

    /// Launch agent
    pub async fn launch(&self, input: AgentLaunchInput) -> Result<AgentLaunchOutput> {
        let agent_id = format!("{}-{}", input.agent_type, Uuid::new_v4());

        // Create execution record
        let execution = AgentExecution {
            agent_id: agent_id.clone(),
            agent_type: input.agent_type.clone(),
            task: input.task.clone(),
            workspace_id: input.workspace_id.clone(),
            session_id: None,
            status: ExecutionStatus::Queued,
            started_at: chrono::Utc::now(),
            ended_at: None,
            result: None,
            error: None,
        };

        self.registry.register(execution).await?;

        // Launch agent based on type
        let agent_type = input.agent_type.clone();
        let agent_type_str = agent_type.clone();
        let task = input.task.clone();
        let workspace_id = input.workspace_id.clone();
        let params = input.params.clone();

        let config = Arc::clone(&self.config);
        let registry = Arc::clone(&self.registry);
        let cortex = Arc::clone(&self.cortex);
        let agent_id_clone = agent_id.clone();

        // Spawn agent task
        tokio::spawn(async move {
            let result = Self::execute_agent(
                &agent_type_str,
                &task,
                workspace_id.as_deref(),
                params,
                cortex,
            )
            .await;

            match result {
                Ok(output) => {
                    let _ = registry.update_status(&agent_id_clone, ExecutionStatus::Completed).await;
                    let _ = registry.set_result(&agent_id_clone, output).await;
                }
                Err(e) => {
                    let _ = registry.set_error(&agent_id_clone, e.to_string()).await;
                }
            }
        });

        Ok(AgentLaunchOutput {
            agent_id,
            agent_type,
            status: "launched".to_string(),
            message: "Agent launched successfully".to_string(),
        })
    }

    /// Execute agent based on type
    async fn execute_agent(
        agent_type: &str,
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        match agent_type {
            "developer" => {
                Self::execute_developer(task, workspace_id, params, cortex).await
            }
            "tester" => {
                Self::execute_tester(task, workspace_id, params, cortex).await
            }
            "reviewer" => {
                Self::execute_reviewer(task, workspace_id, params, cortex).await
            }
            "architect" => {
                Self::execute_architect(task, workspace_id, params, cortex).await
            }
            "researcher" => {
                Self::execute_researcher(task, workspace_id, params, cortex).await
            }
            "optimizer" => {
                Self::execute_optimizer(task, workspace_id, params, cortex).await
            }
            "documenter" => {
                Self::execute_documenter(task, workspace_id, params, cortex).await
            }
            _ => anyhow::bail!("Unknown agent type: {}", agent_type),
        }
    }

    async fn execute_developer(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement with fixed DeveloperAgent
        Ok(serde_json::json!({
            "status": "pending",
            "message": "DeveloperAgent execution not yet implemented"
        }))
    }

    async fn execute_tester(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement with fixed TesterAgent
        Ok(serde_json::json!({
            "status": "pending",
            "message": "TesterAgent execution not yet implemented"
        }))
    }

    async fn execute_reviewer(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement
        Ok(serde_json::json!({
            "status": "pending",
            "message": "ReviewerAgent execution not yet implemented"
        }))
    }

    async fn execute_architect(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement
        Ok(serde_json::json!({
            "status": "pending",
            "message": "ArchitectAgent execution not yet implemented"
        }))
    }

    async fn execute_researcher(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement
        Ok(serde_json::json!({
            "status": "pending",
            "message": "ResearcherAgent execution not yet implemented"
        }))
    }

    async fn execute_optimizer(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement
        Ok(serde_json::json!({
            "status": "pending",
            "message": "OptimizerAgent execution not yet implemented"
        }))
    }

    async fn execute_documenter(
        _task: &str,
        _workspace_id: Option<&str>,
        _params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        // TODO: Implement
        Ok(serde_json::json!({
            "status": "pending",
            "message": "DocumenterAgent execution not yet implemented"
        }))
    }
}
