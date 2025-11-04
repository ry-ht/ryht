//! Agent Registry for tracking agent executions
//!
//! This module provides a registry for tracking agent executions, their status,
//! and results. It replaces the legacy mcp_server::AgentRegistry.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use cortex_storage::ConnectionManager;

/// Status of an agent execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// Queued for execution
    Queued,
    /// Currently running
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
}

/// Agent execution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecution {
    /// Unique agent execution ID
    pub agent_id: String,
    /// Type of agent (developer, tester, reviewer, etc.)
    pub agent_type: String,
    /// Task description
    pub task: String,
    /// Optional workspace ID
    pub workspace_id: Option<String>,
    /// Optional session ID
    pub session_id: Option<String>,
    /// Current execution status
    pub status: ExecutionStatus,
    /// Timestamp when execution started
    pub started_at: DateTime<Utc>,
    /// Timestamp when execution ended (if completed/failed)
    pub ended_at: Option<DateTime<Utc>>,
    /// Execution result (if completed)
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl AgentExecution {
    /// Create a new agent execution
    pub fn new(
        agent_id: String,
        agent_type: String,
        task: String,
        workspace_id: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            agent_id,
            agent_type,
            task,
            workspace_id,
            session_id,
            status: ExecutionStatus::Queued,
            started_at: Utc::now(),
            ended_at: None,
            result: None,
            error: None,
        }
    }

    /// Mark execution as running
    pub fn mark_running(&mut self) {
        self.status = ExecutionStatus::Running;
    }

    /// Mark execution as completed with result
    pub fn mark_completed(&mut self, result: serde_json::Value) {
        self.status = ExecutionStatus::Completed;
        self.ended_at = Some(Utc::now());
        self.result = Some(result);
    }

    /// Mark execution as failed with error
    pub fn mark_failed(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.ended_at = Some(Utc::now());
        self.error = Some(error);
    }
}

/// Registry for tracking agent executions
///
/// This provides in-memory tracking of agent executions with optional
/// persistence to storage for durability across restarts.
#[derive(Clone)]
pub struct AgentRegistry {
    /// In-memory execution tracking
    executions: Arc<RwLock<HashMap<String, AgentExecution>>>,
    /// Storage backend for persistence
    storage: Arc<ConnectionManager>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            storage,
        }
    }

    /// Register a new agent execution
    pub async fn register(&self, execution: AgentExecution) -> Result<()> {
        let agent_id = execution.agent_id.clone();

        // Store in memory
        let mut executions = self.executions.write().await;
        executions.insert(agent_id.clone(), execution.clone());
        drop(executions);

        // TODO: Persist to storage for durability
        // self.storage.store_execution(&execution).await?;

        tracing::info!(agent_id = %agent_id, agent_type = %execution.agent_type, "Agent execution registered");
        Ok(())
    }

    /// Get an agent execution by ID
    pub async fn get(&self, agent_id: &str) -> Option<AgentExecution> {
        let executions = self.executions.read().await;
        executions.get(agent_id).cloned()
    }

    /// Update agent execution status
    pub async fn update_status(&self, agent_id: &str, status: ExecutionStatus) -> Result<()> {
        let mut executions = self.executions.write().await;

        if let Some(execution) = executions.get_mut(agent_id) {
            execution.status = status.clone();

            if status == ExecutionStatus::Completed || status == ExecutionStatus::Failed {
                execution.ended_at = Some(Utc::now());
            }

            tracing::info!(agent_id = %agent_id, status = ?status, "Agent status updated");

            // TODO: Persist status update
            // self.storage.update_execution_status(agent_id, &status).await?;

            Ok(())
        } else {
            anyhow::bail!("Agent execution not found: {}", agent_id)
        }
    }

    /// Set execution result
    pub async fn set_result(&self, agent_id: &str, result: serde_json::Value) -> Result<()> {
        let mut executions = self.executions.write().await;

        if let Some(execution) = executions.get_mut(agent_id) {
            execution.result = Some(result.clone());
            execution.status = ExecutionStatus::Completed;
            execution.ended_at = Some(Utc::now());

            tracing::info!(agent_id = %agent_id, "Agent result set");

            // TODO: Persist result
            // self.storage.update_execution_result(agent_id, &result).await?;

            Ok(())
        } else {
            anyhow::bail!("Agent execution not found: {}", agent_id)
        }
    }

    /// Set execution error
    pub async fn set_error(&self, agent_id: &str, error: String) -> Result<()> {
        let mut executions = self.executions.write().await;

        if let Some(execution) = executions.get_mut(agent_id) {
            execution.error = Some(error.clone());
            execution.status = ExecutionStatus::Failed;
            execution.ended_at = Some(Utc::now());

            tracing::error!(agent_id = %agent_id, error = %error, "Agent execution failed");

            // TODO: Persist error
            // self.storage.update_execution_error(agent_id, &error).await?;

            Ok(())
        } else {
            anyhow::bail!("Agent execution not found: {}", agent_id)
        }
    }

    /// List all executions
    pub async fn list_all(&self) -> Vec<AgentExecution> {
        let executions = self.executions.read().await;
        executions.values().cloned().collect()
    }

    /// List executions by status
    pub async fn list_by_status(&self, status: ExecutionStatus) -> Vec<AgentExecution> {
        let executions = self.executions.read().await;
        executions.values()
            .filter(|e| e.status == status)
            .cloned()
            .collect()
    }

    /// List executions by agent type
    pub async fn list_by_type(&self, agent_type: &str) -> Vec<AgentExecution> {
        let executions = self.executions.read().await;
        executions.values()
            .filter(|e| e.agent_type == agent_type)
            .cloned()
            .collect()
    }

    /// Remove completed/failed executions older than specified duration
    pub async fn cleanup_old(&self, max_age: chrono::Duration) -> Result<usize> {
        let cutoff = Utc::now() - max_age;
        let mut executions = self.executions.write().await;

        let before_count = executions.len();
        executions.retain(|_, execution| {
            // Keep running/queued executions
            if execution.status == ExecutionStatus::Running || execution.status == ExecutionStatus::Queued {
                return true;
            }

            // Keep recent completed/failed executions
            if let Some(ended_at) = execution.ended_at {
                ended_at > cutoff
            } else {
                true
            }
        });

        let removed = before_count - executions.len();
        if removed > 0 {
            tracing::info!(removed = removed, "Cleaned up old agent executions");
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> AgentRegistry {
        let storage = Arc::new(ConnectionManager::default());
        AgentRegistry::new(storage)
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = create_test_registry();

        let execution = AgentExecution::new(
            "agent-1".to_string(),
            "developer".to_string(),
            "Implement feature".to_string(),
            Some("workspace-1".to_string()),
            None,
        );

        registry.register(execution.clone()).await.unwrap();

        let retrieved = registry.get("agent-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_update_status() {
        let registry = create_test_registry();

        let execution = AgentExecution::new(
            "agent-2".to_string(),
            "tester".to_string(),
            "Run tests".to_string(),
            None,
            None,
        );

        registry.register(execution).await.unwrap();
        registry.update_status("agent-2", ExecutionStatus::Running).await.unwrap();

        let retrieved = registry.get("agent-2").await.unwrap();
        assert_eq!(retrieved.status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_set_result() {
        let registry = create_test_registry();

        let execution = AgentExecution::new(
            "agent-3".to_string(),
            "reviewer".to_string(),
            "Review code".to_string(),
            None,
            None,
        );

        registry.register(execution).await.unwrap();

        let result = serde_json::json!({"status": "success"});
        registry.set_result("agent-3", result.clone()).await.unwrap();

        let retrieved = registry.get("agent-3").await.unwrap();
        assert_eq!(retrieved.status, ExecutionStatus::Completed);
        assert_eq!(retrieved.result, Some(result));
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let registry = create_test_registry();

        let exec1 = AgentExecution::new(
            "agent-4".to_string(),
            "developer".to_string(),
            "Task 1".to_string(),
            None,
            None,
        );

        let mut exec2 = AgentExecution::new(
            "agent-5".to_string(),
            "tester".to_string(),
            "Task 2".to_string(),
            None,
            None,
        );
        exec2.status = ExecutionStatus::Running;

        registry.register(exec1).await.unwrap();
        registry.register(exec2).await.unwrap();

        let queued = registry.list_by_status(ExecutionStatus::Queued).await;
        assert_eq!(queued.len(), 1);

        let running = registry.list_by_status(ExecutionStatus::Running).await;
        assert_eq!(running.len(), 1);
    }
}
