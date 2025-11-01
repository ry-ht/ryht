//! Agent Runtime - Main Runtime System Manager
//!
//! This module provides the main runtime system that coordinates process management,
//! MCP integration, and task execution for the multi-agent system.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::agents::{AgentId, AgentType};
use crate::orchestration::task_delegation::TaskDelegation;
use crate::orchestration::lead_agent::WorkerResult;
use crate::coordination::UnifiedMessageBus;

use super::{
    runtime_config::RuntimeConfig,
    agent_process::{ProcessManager, ProcessManagerStatistics},
    mcp_integration::McpServerPool,
    agent_executor::{AgentExecutor, ExecutorStatistics},
};

/// Result type for runtime operations
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Runtime errors
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Agent spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Runtime not initialized")]
    NotInitialized,

    #[error("Shutdown in progress")]
    ShuttingDown,

    #[error("Process error: {0}")]
    Process(String),

    #[error("Executor error: {0}")]
    Executor(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Other error: {0}")]
    Other(String),
}

/// Agent runtime system
///
/// The main orchestrator for the agent runtime, managing:
/// - Process lifecycle
/// - MCP server pool
/// - Task execution
/// - Health monitoring
/// - Resource cleanup
pub struct AgentRuntime {
    /// Process manager
    process_manager: Arc<ProcessManager>,

    /// MCP server pool
    mcp_pool: Arc<McpServerPool>,

    /// Task executor
    executor: Arc<AgentExecutor>,

    /// Message bus for inter-agent communication
    message_bus: Arc<UnifiedMessageBus>,

    /// Active agents registry
    active_agents: Arc<RwLock<HashMap<AgentId, AgentInfo>>>,

    /// Runtime configuration
    config: RuntimeConfig,

    /// Runtime state
    state: Arc<RwLock<RuntimeState>>,

    /// Statistics
    stats: Arc<RwLock<RuntimeStatistics>>,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub agent_id: AgentId,

    /// Agent name
    pub agent_name: String,

    /// Agent type
    pub agent_type: AgentType,

    /// Process ID
    pub process_id: u32,

    /// Spawn time
    pub spawned_at: chrono::DateTime<chrono::Utc>,

    /// Last activity
    pub last_activity: chrono::DateTime<chrono::Utc>,

    /// Tasks executed
    pub tasks_executed: u64,

    /// Tasks failed
    pub tasks_failed: u64,

    /// Agent status
    pub status: AgentStatus,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,

    /// Agent is ready for tasks
    Ready,

    /// Agent is executing a task
    Busy,

    /// Agent is idle
    Idle,

    /// Agent has failed
    Failed,

    /// Agent is shutting down
    ShuttingDown,

    /// Agent has terminated
    Terminated,
}

/// Runtime state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeState {
    /// Runtime is initializing
    Initializing,

    /// Runtime is running
    Running,

    /// Runtime is shutting down
    ShuttingDown,

    /// Runtime is stopped
    Stopped,
}

/// Runtime statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    /// Total agents spawned
    pub total_agents_spawned: u64,

    /// Currently active agents
    pub active_agents: usize,

    /// Total tasks executed
    pub total_tasks_executed: u64,

    /// Total tasks failed
    pub total_tasks_failed: u64,

    /// Uptime (seconds)
    pub uptime_seconds: u64,

    /// Process statistics
    pub process_stats: Option<ProcessManagerStatistics>,

    /// Executor statistics
    pub executor_stats: Option<ExecutorStatistics>,
}

impl AgentRuntime {
    /// Create a new agent runtime
    pub fn new(
        config: RuntimeConfig,
        message_bus: Arc<UnifiedMessageBus>,
    ) -> Self {
        info!("Initializing Agent Runtime");

        let process_manager = Arc::new(ProcessManager::new(
            config.process.clone(),
            config.resources.clone(),
        ));

        let mcp_pool = Arc::new(McpServerPool::new(config.mcp.clone()));

        let executor = Arc::new(AgentExecutor::new(
            process_manager.clone(),
            mcp_pool.clone(),
            config.clone(),
        ));

        Self {
            process_manager,
            mcp_pool,
            executor,
            message_bus,
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            config,
            state: Arc::new(RwLock::new(RuntimeState::Initializing)),
            stats: Arc::new(RwLock::new(RuntimeStatistics::default())),
        }
    }

    /// Initialize and start the runtime
    pub async fn start(&self) -> Result<()> {
        info!("Starting Agent Runtime");

        *self.state.write().await = RuntimeState::Running;

        // Start background tasks
        self.start_monitoring_tasks().await;

        info!("Agent Runtime started successfully");

        Ok(())
    }

    /// Spawn a new agent process
    pub async fn spawn_agent(
        &self,
        agent_name: String,
        agent_type: AgentType,
        command: &str,
        args: &[String],
    ) -> Result<AgentId> {
        info!("Spawning agent: {} ({:?})", agent_name, agent_type);

        // Check runtime state
        let state = *self.state.read().await;
        if state != RuntimeState::Running {
            return Err(RuntimeError::NotInitialized);
        }

        let agent_id = AgentId::new();

        // Spawn process
        self.process_manager
            .spawn(agent_id.clone(), agent_name.clone(), command, args)
            .await
            .map_err(|e| RuntimeError::SpawnFailed(e.to_string()))?;

        // Initialize MCP server
        self.mcp_pool
            .get_or_create(&agent_id)
            .await
            .map_err(|e| RuntimeError::SpawnFailed(e.to_string()))?;

        // Register agent
        let agent_info = AgentInfo {
            agent_id: agent_id.clone(),
            agent_name: agent_name.clone(),
            agent_type,
            process_id: 0, // Would be set from process manager
            spawned_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            tasks_executed: 0,
            tasks_failed: 0,
            status: AgentStatus::Ready,
        };

        self.active_agents.write().await.insert(agent_id.clone(), agent_info);

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_agents_spawned += 1;
        stats.active_agents = self.active_agents.read().await.len();

        info!("Agent {} spawned successfully", agent_id);

        Ok(agent_id)
    }

    /// Execute a task on an agent
    pub async fn execute_task(
        &self,
        agent_id: &AgentId,
        delegation: TaskDelegation,
    ) -> Result<WorkerResult> {
        debug!("Executing task on agent: {}", agent_id);

        // Update agent status
        if let Some(agent_info) = self.active_agents.write().await.get_mut(agent_id) {
            agent_info.status = AgentStatus::Busy;
            agent_info.last_activity = chrono::Utc::now();
        } else {
            return Err(RuntimeError::AgentNotFound(agent_id.to_string()));
        }

        // Execute task
        let result = self.executor
            .execute_task(agent_id, delegation)
            .await
            .map_err(|e| RuntimeError::Executor(e.to_string()));

        // Update agent info
        if let Some(agent_info) = self.active_agents.write().await.get_mut(agent_id) {
            agent_info.status = AgentStatus::Idle;
            agent_info.last_activity = chrono::Utc::now();

            if result.is_ok() {
                agent_info.tasks_executed += 1;
            } else {
                agent_info.tasks_failed += 1;
            }
        }

        // Update runtime statistics
        let mut stats = self.stats.write().await;
        if result.is_ok() {
            stats.total_tasks_executed += 1;
        } else {
            stats.total_tasks_failed += 1;
        }

        result
    }

    /// Execute multiple tasks in parallel
    pub async fn execute_tasks_parallel(
        &self,
        tasks: Vec<(AgentId, TaskDelegation)>,
    ) -> Vec<Result<WorkerResult>> {
        info!("Executing {} tasks in parallel", tasks.len());

        let max_parallel = self.config.process.max_concurrent_processes;

        let results = self.executor
            .execute_tasks_parallel(tasks, max_parallel)
            .await;

        results.into_iter()
            .map(|r| r.map_err(|e| RuntimeError::Executor(e.to_string())))
            .collect()
    }

    /// Terminate an agent
    pub async fn terminate_agent(&self, agent_id: &AgentId) -> Result<()> {
        info!("Terminating agent: {}", agent_id);

        // Update agent status
        if let Some(agent_info) = self.active_agents.write().await.get_mut(agent_id) {
            agent_info.status = AgentStatus::ShuttingDown;
        }

        // Shutdown MCP server
        if let Err(e) = self.mcp_pool.shutdown(agent_id).await {
            warn!("Failed to shutdown MCP server: {}", e);
        }

        // Terminate process
        self.process_manager
            .terminate(agent_id)
            .await
            .map_err(|e| RuntimeError::Process(e.to_string()))?;

        // Remove from active agents
        self.active_agents.write().await.remove(agent_id);

        // Update statistics
        self.stats.write().await.active_agents = self.active_agents.read().await.len();

        info!("Agent {} terminated", agent_id);

        Ok(())
    }

    /// Get agent information
    pub async fn get_agent_info(&self, agent_id: &AgentId) -> Option<AgentInfo> {
        self.active_agents.read().await.get(agent_id).cloned()
    }

    /// Get all active agents
    pub async fn get_active_agents(&self) -> Vec<AgentInfo> {
        self.active_agents.read().await.values().cloned().collect()
    }

    /// Get runtime statistics
    pub async fn get_statistics(&self) -> RuntimeStatistics {
        let mut stats = self.stats.read().await.clone();

        // Update with current process and executor stats
        stats.process_stats = Some(self.process_manager.get_statistics().await);
        stats.executor_stats = Some(self.executor.get_statistics().await);

        stats
    }

    /// Execute developer agent task
    pub async fn execute_developer_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing developer task: {}", task);

        // Create task delegation for developer agent
        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["code_generation".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 5,
            required_capabilities: vec!["code_generation".to_string()],
            context: params.clone(),
        };

        // Find or spawn developer agent
        let agent_id = self.find_or_spawn_agent(AgentType::Developer).await?;

        // Execute task
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute reviewer agent task
    pub async fn execute_reviewer_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing reviewer task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["code_review".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 5,
            required_capabilities: vec!["code_review".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Reviewer).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute tester agent task
    pub async fn execute_tester_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing tester task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["testing".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 5,
            required_capabilities: vec!["test_generation".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Tester).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute optimizer agent task
    pub async fn execute_optimizer_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing optimizer task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["optimization".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 7,
            required_capabilities: vec!["performance_optimization".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Optimizer).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute architect agent task
    pub async fn execute_architect_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing architect task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["architecture".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 8,
            required_capabilities: vec!["system_design".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Architect).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute researcher agent task
    pub async fn execute_researcher_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing researcher task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["research".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 5,
            required_capabilities: vec!["information_gathering".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Researcher).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Execute documenter agent task
    pub async fn execute_documenter_task(&self, task: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        debug!("Executing documenter task: {}", task);

        let delegation = TaskDelegation {
            task_id: uuid::Uuid::new_v4().to_string(),
            objective: task.to_string(),
            output_format: crate::orchestration::strategy_library::OutputFormat {
                format_type: "json".to_string(),
                required_sections: Vec::new(),
                optional_sections: Vec::new(),
                schema: None,
            },
            allowed_tools: vec!["cortex_code".to_string()],
            boundaries: crate::orchestration::task_delegation::TaskBoundaries {
                scope: vec!["documentation".to_string()],
                constraints: Vec::new(),
                max_tool_calls: 100,
                timeout: std::time::Duration::from_secs(300),
            },
            priority: 3,
            required_capabilities: vec!["documentation".to_string()],
            context: params.clone(),
        };

        let agent_id = self.find_or_spawn_agent(AgentType::Documenter).await?;
        let result = self.execute_task(&agent_id, delegation).await?;

        Ok(serde_json::json!({
            "success": result.success,
            "result": result.result,
            "duration_ms": result.duration.as_millis(),
            "tokens_used": result.tokens_used,
            "cost_cents": result.cost_cents,
        }))
    }

    /// Find an existing agent or spawn a new one
    async fn find_or_spawn_agent(&self, agent_type: AgentType) -> Result<AgentId> {
        let agents = self.active_agents.read().await;

        // Try to find existing agent of the same type
        for (id, info) in agents.iter() {
            if info.agent_type == agent_type && info.status == AgentStatus::Ready {
                return Ok(id.clone());
            }
        }

        drop(agents);

        // No suitable agent found, spawn a new one
        let agent_name = format!("{:?}-{}", agent_type, uuid::Uuid::new_v4());
        self.spawn_agent(
            agent_name,
            agent_type,
            "cortex",
            &["mcp".to_string(), "stdio".to_string()],
        ).await
    }

    /// Shutdown the runtime
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Agent Runtime");

        *self.state.write().await = RuntimeState::ShuttingDown;

        // Terminate all agents
        let agent_ids: Vec<AgentId> = self.active_agents.read().await.keys().cloned().collect();

        for agent_id in agent_ids {
            if let Err(e) = self.terminate_agent(&agent_id).await {
                warn!("Failed to terminate agent {}: {}", agent_id, e);
            }
        }

        // Shutdown MCP pool
        if let Err(e) = self.mcp_pool.shutdown_all().await {
            warn!("Failed to shutdown MCP pool: {}", e);
        }

        *self.state.write().await = RuntimeState::Stopped;

        info!("Agent Runtime shutdown complete");

        Ok(())
    }

    /// Start background monitoring tasks
    async fn start_monitoring_tasks(&self) {
        let process_manager = self.process_manager.clone();
        let active_agents = self.active_agents.clone();
        let health_check_interval = self.config.monitoring.health_check_interval;

        // Health check task
        tokio::spawn(async move {
            let mut interval = interval(health_check_interval);

            loop {
                interval.tick().await;

                // Cleanup dead processes
                process_manager.cleanup_dead_processes().await;

                // Check agent health
                let agents: Vec<AgentId> = active_agents.read().await.keys().cloned().collect();

                for agent_id in agents {
                    if !process_manager.is_alive(&agent_id).await {
                        warn!("Agent {} process is dead", agent_id);

                        if let Some(agent_info) = active_agents.write().await.get_mut(&agent_id) {
                            agent_info.status = AgentStatus::Failed;
                        }
                    }
                }
            }
        });

        info!("Background monitoring tasks started");
    }

    /// Check if runtime is running
    pub async fn is_running(&self) -> bool {
        *self.state.read().await == RuntimeState::Running
    }

    /// Get runtime state
    pub async fn get_state(&self) -> RuntimeState {
        *self.state.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state() {
        let state = RuntimeState::Initializing;
        assert_eq!(state, RuntimeState::Initializing);
    }

    #[test]
    fn test_agent_status() {
        let status = AgentStatus::Ready;
        assert_eq!(status, AgentStatus::Ready);
    }

    #[test]
    fn test_runtime_statistics() {
        let stats = RuntimeStatistics::default();
        assert_eq!(stats.total_agents_spawned, 0);
        assert_eq!(stats.active_agents, 0);
    }
}
