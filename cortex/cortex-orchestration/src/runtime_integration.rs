//! Runtime Integration for Lead Agent
//!
//! This module provides integration between the orchestration layer (LeadAgent)
//! and the runtime system (AgentRuntime), enabling actual agent execution.

use std::sync::Arc;
use tracing::{debug, info};

use cortex_agents::{AgentId, AgentType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    lead_agent::{WorkerResult, LeadAgent},
    worker_registry::WorkerHandle,
    task_delegation::TaskDelegation,
    OrchestrationError, Result,
};

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Terminated,
    Failed,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: AgentId,
    pub name: String,
    pub agent_type: AgentType,
    pub status: AgentStatus,
}

/// Runtime statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    pub active_agents: usize,
    pub total_tasks_executed: usize,
    pub total_execution_time_ms: u64,
}

/// Runtime error types
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Executor error: {0}")]
    Executor(String),

    #[error("Spawn error: {0}")]
    Spawn(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Trait for agent runtime implementations
#[async_trait]
pub trait AgentRuntime: Send + Sync {
    /// Execute a task on an agent
    async fn execute_task(
        &self,
        agent_id: &AgentId,
        delegation: TaskDelegation,
    ) -> std::result::Result<WorkerResult, RuntimeError>;

    /// Spawn a new agent
    async fn spawn_agent(
        &self,
        name: String,
        agent_type: AgentType,
        command: &str,
        args: &[String],
    ) -> std::result::Result<AgentId, RuntimeError>;

    /// Terminate an agent
    async fn terminate_agent(&self, agent_id: &AgentId) -> std::result::Result<(), RuntimeError>;

    /// Get agent information
    async fn get_agent_info(&self, agent_id: &AgentId) -> Option<AgentInfo>;

    /// Get runtime statistics
    async fn get_statistics(&self) -> RuntimeStatistics;
}

/// Integration layer between LeadAgent and AgentRuntime
pub struct RuntimeIntegration {
    /// Agent runtime
    runtime: Arc<dyn AgentRuntime>,
}

impl RuntimeIntegration {
    /// Create new runtime integration
    pub fn new(runtime: Arc<dyn AgentRuntime>) -> Self {
        info!("Initializing Runtime Integration");

        Self { runtime }
    }

    /// Execute a task delegation on a worker via the runtime
    pub async fn execute_worker_task(
        &self,
        handle: &WorkerHandle,
        delegation: TaskDelegation,
    ) -> Result<WorkerResult> {
        debug!(
            "Executing task {} on worker {} via runtime",
            delegation.task_id, handle.worker_id
        );

        // Execute task through runtime
        let result = self.runtime
            .execute_task(&handle.worker_id, delegation)
            .await
            .map_err(|e| match e {
                RuntimeError::AgentNotFound(msg) => {
                    OrchestrationError::NoSuitableAgent { task_id: msg }
                }
                RuntimeError::Executor(msg) => {
                    OrchestrationError::ExecutionFailed { reason: msg }
                }
                _ => OrchestrationError::Other(anyhow::anyhow!("Runtime error: {}", e)),
            })?;

        Ok(result)
    }

    /// Spawn a worker agent
    pub async fn spawn_worker(
        &self,
        agent_name: String,
        agent_type: AgentType,
    ) -> Result<AgentId> {
        debug!("Spawning worker agent: {} ({:?})", agent_name, agent_type);

        // Spawn agent via runtime
        // Using cortex in MCP stdio mode as the agent process
        let agent_id = self.runtime
            .spawn_agent(
                agent_name.clone(),
                agent_type,
                "cortex",
                &["mcp".to_string(), "stdio".to_string()],
            )
            .await
            .map_err(|e| OrchestrationError::Other(anyhow::anyhow!("Spawn failed: {}", e)))?;

        info!("Worker agent {} spawned: {}", agent_name, agent_id);

        Ok(agent_id)
    }

    /// Terminate a worker agent
    pub async fn terminate_worker(&self, agent_id: &AgentId) -> Result<()> {
        debug!("Terminating worker agent: {}", agent_id);

        self.runtime
            .terminate_agent(agent_id)
            .await
            .map_err(|e| OrchestrationError::Other(anyhow::anyhow!("Terminate failed: {}", e)))?;

        info!("Worker agent {} terminated", agent_id);

        Ok(())
    }

    /// Check if worker is alive
    pub async fn is_worker_alive(&self, agent_id: &AgentId) -> bool {
        if let Some(agent_info) = self.runtime.get_agent_info(agent_id).await {
            agent_info.status != AgentStatus::Terminated
                && agent_info.status != AgentStatus::Failed
        } else {
            false
        }
    }

    /// Get runtime statistics
    pub async fn get_runtime_statistics(&self) -> RuntimeStatistics {
        self.runtime.get_statistics().await
    }
}

/// Extension trait for LeadAgent to use runtime integration
pub trait LeadAgentRuntimeExt {
    /// Set runtime integration
    fn with_runtime(self, runtime: Arc<dyn AgentRuntime>) -> LeadAgentWithRuntime;
}

/// LeadAgent with runtime integration
pub struct LeadAgentWithRuntime {
    /// Lead agent
    lead_agent: LeadAgent,

    /// Runtime integration
    runtime_integration: Arc<RuntimeIntegration>,
}

impl LeadAgentWithRuntime {
    /// Create new LeadAgent with runtime
    pub fn new(lead_agent: LeadAgent, runtime: Arc<dyn AgentRuntime>) -> Self {
        let runtime_integration = Arc::new(RuntimeIntegration::new(runtime));

        Self {
            lead_agent,
            runtime_integration,
        }
    }

    /// Get reference to lead agent
    pub fn lead_agent(&self) -> &LeadAgent {
        &self.lead_agent
    }

    /// Get reference to runtime integration
    pub fn runtime_integration(&self) -> &RuntimeIntegration {
        &self.runtime_integration
    }

    /// Handle query with runtime execution
    pub async fn handle_query(
        &self,
        query: &str,
        workspace_id: crate::cortex_bridge::WorkspaceId,
        session_id: crate::cortex_bridge::SessionId,
    ) -> Result<super::result_synthesizer::SynthesizedResult> {
        // Use the lead agent's handle_query method
        // The actual worker execution will be delegated through the runtime
        self.lead_agent.handle_query(query, workspace_id, session_id).await
    }

    /// Execute worker task via runtime
    pub async fn execute_worker_task_runtime(
        &self,
        handle: &WorkerHandle,
        delegation: TaskDelegation,
    ) -> Result<WorkerResult> {
        self.runtime_integration.execute_worker_task(handle, delegation).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_runtime_integration_creation() {
        // Test would require actual runtime initialization
        // Skipping for now as it requires full setup
    }
}
