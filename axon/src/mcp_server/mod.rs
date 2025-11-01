//! MCP Server for Axon Agent Orchestration
//!
//! This module provides an MCP (Model Context Protocol) server that exposes
//! Axon's multi-agent capabilities as tools that can be used by Claude Code
//! and other MCP clients.
//!
//! # Tools Provided
//!
//! - `agent_launch` - Launch specialized agents (developer, tester, reviewer, etc.)
//! - `agent_status` - Check agent status and progress
//! - `agent_stop` - Stop running agents
//! - `orchestrate_task` - Orchestrate multi-agent workflows
//! - `query_cortex` - Query Cortex knowledge graph
//! - `session_create` - Create isolated work sessions
//! - `session_merge` - Merge session changes

use anyhow::{Context, Result};
use cortex_core::config::GlobalConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod tools;
pub mod server;

pub use server::AxonMcpServer;

/// MCP Server configuration
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,

    /// Cortex base URL
    pub cortex_url: String,

    /// Working directory for agent operations
    pub working_dir: std::path::PathBuf,

    /// Maximum concurrent agents
    pub max_concurrent_agents: usize,

    /// Default timeout for agent operations (seconds)
    pub default_timeout_secs: u64,
}

impl Default for McpServerConfig {
    /// Create a default McpServerConfig with hardcoded fallback values.
    ///
    /// **Important**: This uses fallback defaults and should only be used
    /// when GlobalConfig is not available. Prefer using `McpServerConfig::from_global_config()`
    /// to get configuration from GlobalConfig.
    fn default() -> Self {
        Self {
            name: "axon-mcp-server".to_string(),
            version: crate::VERSION.to_string(),
            cortex_url: "http://localhost:8080".to_string(), // Cortex API server default port
            working_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            max_concurrent_agents: 10,
            default_timeout_secs: 3600, // 1 hour
        }
    }
}

impl McpServerConfig {
    /// Create a McpServerConfig from GlobalConfig
    ///
    /// This is the preferred way to create a McpServerConfig as it reads
    /// from the global configuration file.
    pub async fn from_global_config() -> Result<Self> {
        let config = GlobalConfig::load_or_create_default()
            .await
            .context("Failed to load GlobalConfig")?;

        Ok(Self {
            name: "axon-mcp-server".to_string(),
            version: crate::VERSION.to_string(),
            cortex_url: format!(
                "http://{}:{}",
                config.cortex().server.host,
                config.cortex().server.port
            ),
            working_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            max_concurrent_agents: config.axon().runtime.max_agents,
            default_timeout_secs: config.axon().runtime.agent_timeout_seconds,
        })
    }
}

/// Agent execution state
#[derive(Debug, Clone)]
pub struct AgentExecution {
    /// Agent ID
    pub agent_id: String,

    /// Agent type
    pub agent_type: String,

    /// Task description
    pub task: String,

    /// Workspace ID
    pub workspace_id: Option<String>,

    /// Session ID
    pub session_id: Option<String>,

    /// Status
    pub status: ExecutionStatus,

    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// End time
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Result
    pub result: Option<serde_json::Value>,

    /// Error
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Queued
    Queued,

    /// Running
    Running,

    /// Completed
    Completed,

    /// Failed
    Failed,

    /// Cancelled
    Cancelled,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Agent registry for tracking running agents
#[derive(Debug, Default)]
pub struct AgentRegistry {
    executions: Arc<RwLock<std::collections::HashMap<String, AgentExecution>>>,
}

impl AgentRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register new execution
    pub async fn register(&self, execution: AgentExecution) -> Result<()> {
        let mut executions = self.executions.write().await;
        executions.insert(execution.agent_id.clone(), execution);
        Ok(())
    }

    /// Get execution by ID
    pub async fn get(&self, agent_id: &str) -> Option<AgentExecution> {
        let executions = self.executions.read().await;
        executions.get(agent_id).cloned()
    }

    /// Update execution status
    pub async fn update_status(&self, agent_id: &str, status: ExecutionStatus) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(agent_id) {
            execution.status = status;
            if matches!(status, ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled) {
                execution.ended_at = Some(chrono::Utc::now());
            }
        }
        Ok(())
    }

    /// Set execution result
    pub async fn set_result(&self, agent_id: &str, result: serde_json::Value) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(agent_id) {
            execution.result = Some(result);
        }
        Ok(())
    }

    /// Set execution error
    pub async fn set_error(&self, agent_id: &str, error: String) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(execution) = executions.get_mut(agent_id) {
            execution.error = Some(error);
            execution.status = ExecutionStatus::Failed;
            execution.ended_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    /// List all executions
    pub async fn list(&self) -> Vec<AgentExecution> {
        let executions = self.executions.read().await;
        executions.values().cloned().collect()
    }

    /// Remove execution
    pub async fn remove(&self, agent_id: &str) -> Option<AgentExecution> {
        let mut executions = self.executions.write().await;
        executions.remove(agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_registry() {
        let registry = AgentRegistry::new();

        let execution = AgentExecution {
            agent_id: "test-agent-1".to_string(),
            agent_type: "developer".to_string(),
            task: "Generate code".to_string(),
            workspace_id: Some("ws-123".to_string()),
            session_id: Some("sess-456".to_string()),
            status: ExecutionStatus::Running,
            started_at: chrono::Utc::now(),
            ended_at: None,
            result: None,
            error: None,
        };

        registry.register(execution.clone()).await.unwrap();

        let retrieved = registry.get("test-agent-1").await.unwrap();
        assert_eq!(retrieved.agent_id, "test-agent-1");
        assert_eq!(retrieved.status, ExecutionStatus::Running);

        registry.update_status("test-agent-1", ExecutionStatus::Completed).await.unwrap();

        let updated = registry.get("test-agent-1").await.unwrap();
        assert_eq!(updated.status, ExecutionStatus::Completed);
        assert!(updated.ended_at.is_some());
    }
}
