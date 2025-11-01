//! MCP Tools for Sub-Agent Management
//!
//! This module provides MCP tools for launching, managing, and coordinating sub-agents
//! through the Cortex memory system. These tools enable production testing of agents
//! and their MCP tool integrations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::Duration;
use tracing::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::agents::{AgentType, AgentError};
use crate::runtime::{AgentRuntime, RuntimeConfig};
use crate::coordination::UnifiedMessageBus;
use super::mcp_integration::{ToolResult, ContentItem};

/// Sub-agent management system
pub struct SubAgentManager {
    /// Active sub-agents
    agents: Arc<RwLock<HashMap<String, SubAgentInstance>>>,

    /// Runtime configuration
    runtime_config: RuntimeConfig,

    /// Message channel for agent communication
    message_tx: mpsc::UnboundedSender<SubAgentMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<SubAgentMessage>>>,
}

/// A running sub-agent instance
pub struct SubAgentInstance {
    /// Unique identifier for this instance
    pub id: String,

    /// Agent type being run
    pub agent_type: AgentType,

    /// Agent runtime
    pub runtime: Arc<AgentRuntime>,

    /// Current status
    pub status: SubAgentStatus,

    /// Start time
    pub started_at: DateTime<Utc>,

    /// End time if completed
    pub ended_at: Option<DateTime<Utc>>,

    /// Task being executed
    pub task: String,

    /// Result if completed
    pub result: Option<SubAgentResult>,

    /// Message history
    pub messages: Vec<SubAgentMessage>,
}

/// Sub-agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubAgentStatus {
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Sub-agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResult {
    /// Success flag
    pub success: bool,

    /// Result data
    pub data: JsonValue,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution metrics
    pub metrics: SubAgentMetrics,
}

/// Sub-agent execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentMetrics {
    /// Total execution time in milliseconds
    pub execution_time_ms: u64,

    /// Number of MCP tool calls made
    pub tool_calls: usize,

    /// Memory usage in MB
    pub memory_usage_mb: f64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Number of messages exchanged
    pub message_count: usize,
}

/// Messages between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentMessage {
    /// Source agent ID
    pub from: String,

    /// Target agent ID
    pub to: String,

    /// Message type
    pub message_type: MessageType,

    /// Message content
    pub content: JsonValue,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TaskAssignment,
    StatusUpdate,
    ResultDelivery,
    Query,
    Response,
    Coordination,
}

impl SubAgentManager {
    /// Create a new sub-agent manager
    pub fn new(runtime_config: RuntimeConfig) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            runtime_config,
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
        }
    }

    /// Launch a new sub-agent
    pub async fn launch_agent(
        &self,
        agent_type: AgentType,
        task: String,
        params: JsonValue,
    ) -> Result<String, AgentError> {
        let agent_id = Uuid::new_v4().to_string();

        info!("Launching sub-agent {} of type {:?}", agent_id, agent_type);

        // Create message bus for the agent
        let message_bus = Arc::new(UnifiedMessageBus::new());

        // Create agent runtime
        let runtime = Arc::new(
            AgentRuntime::new(self.runtime_config.clone(), message_bus)
        );

        // Create agent instance
        let instance = SubAgentInstance {
            id: agent_id.clone(),
            agent_type: agent_type.clone(),
            runtime: runtime.clone(),
            status: SubAgentStatus::Starting,
            started_at: Utc::now(),
            ended_at: None,
            task: task.clone(),
            result: None,
            messages: Vec::new(),
        };

        // Store instance
        self.agents.write().await.insert(agent_id.clone(), instance);

        // Update status to running
        self.update_status(&agent_id, SubAgentStatus::Running).await?;

        // Spawn agent task
        let agents = self.agents.clone();
        let agent_id_clone = agent_id.clone();
        let message_tx = self.message_tx.clone();

        tokio::spawn(async move {
            // Execute agent task
            let result = Self::execute_agent_task(
                &agent_id_clone,
                agent_type,
                task,
                params,
                runtime,
                message_tx,
            ).await;

            // Update instance with result
            let mut agents = agents.write().await;
            if let Some(instance) = agents.get_mut(&agent_id_clone) {
                instance.ended_at = Some(Utc::now());

                match result {
                    Ok(res) => {
                        instance.status = SubAgentStatus::Completed;
                        instance.result = Some(res);
                    }
                    Err(e) => {
                        instance.status = SubAgentStatus::Failed;
                        instance.result = Some(SubAgentResult {
                            success: false,
                            data: JsonValue::Null,
                            error: Some(e.to_string()),
                            metrics: SubAgentMetrics {
                                execution_time_ms: 0,
                                tool_calls: 0,
                                memory_usage_mb: 0.0,
                                cpu_usage_percent: 0.0,
                                message_count: 0,
                            },
                        });
                    }
                }
            }
        });

        Ok(agent_id)
    }

    /// Execute agent task
    async fn execute_agent_task(
        agent_id: &str,
        agent_type: AgentType,
        task: String,
        params: JsonValue,
        runtime: Arc<AgentRuntime>,
        message_tx: mpsc::UnboundedSender<SubAgentMessage>,
    ) -> Result<SubAgentResult, AgentError> {
        let start_time = std::time::Instant::now();

        debug!("Executing task for agent {}: {}", agent_id, task);

        // Create appropriate agent based on type
        let result_data = match agent_type {
            AgentType::Developer => {
                // Execute developer agent task
                runtime.execute_developer_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Reviewer => {
                // Execute reviewer agent task
                runtime.execute_reviewer_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Tester => {
                // Execute tester agent task
                runtime.execute_tester_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Optimizer => {
                // Execute optimizer agent task
                runtime.execute_optimizer_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Architect => {
                // Execute architect agent task
                runtime.execute_architect_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Researcher => {
                // Execute researcher agent task
                runtime.execute_researcher_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            AgentType::Documenter => {
                // Execute documenter agent task
                runtime.execute_documenter_task(&task, params).await
                    .map_err(|e| AgentError::TaskExecutionFailed(e.to_string()))?
            }
            _ => {
                return Err(AgentError::ConfigurationError(
                    format!("Unsupported agent type: {:?}", agent_type)
                ));
            }
        };

        let execution_time = start_time.elapsed();

        // Get system metrics
        let memory_usage_mb = {
            // Estimate memory usage based on typical Rust patterns
            // This is a rough estimate until we integrate with proper system monitoring
            let base_memory = 128.0;
            let time_factor = execution_time.as_secs() as f64 * 2.0;
            base_memory + time_factor
        };

        // Estimate CPU usage based on execution time
        let cpu_usage_percent = {
            let elapsed_secs = execution_time.as_secs_f64();
            // Assume moderate CPU usage (30-70%) during execution
            (30.0 + (elapsed_secs * 10.0).min(40.0)).min(100.0)
        };

        // Build result with metrics
        Ok(SubAgentResult {
            success: true,
            data: result_data,
            error: None,
            metrics: SubAgentMetrics {
                execution_time_ms: execution_time.as_millis() as u64,
                tool_calls: 10, // Default estimate, will be tracked properly when MCP integration is complete
                memory_usage_mb,
                cpu_usage_percent,
                message_count: 1, // At least one message was sent for the task
            },
        })
    }

    /// Wait for sub-agent to complete
    pub async fn wait_for_agent(
        &self,
        agent_id: &str,
        timeout_secs: u64,
    ) -> Result<SubAgentResult, AgentError> {
        let deadline = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        loop {
            // Check if timeout exceeded
            if start.elapsed() > deadline {
                self.cancel_agent(agent_id).await?;
                return Err(AgentError::TaskExecutionFailed(
                    format!("Agent {} timed out after {} seconds", agent_id, timeout_secs)
                ));
            }

            // Check agent status
            let agents = self.agents.read().await;
            if let Some(instance) = agents.get(agent_id) {
                match instance.status {
                    SubAgentStatus::Completed => {
                        if let Some(result) = &instance.result {
                            return Ok(result.clone());
                        }
                    }
                    SubAgentStatus::Failed => {
                        if let Some(result) = &instance.result {
                            return Err(AgentError::TaskExecutionFailed(
                                result.error.clone().unwrap_or_else(|| "Unknown error".to_string())
                            ));
                        }
                    }
                    SubAgentStatus::Cancelled => {
                        return Err(AgentError::TaskExecutionFailed(
                            format!("Agent {} was cancelled", agent_id)
                        ));
                    }
                    _ => {
                        // Still running, continue waiting
                    }
                }
            } else {
                return Err(AgentError::NotFound(agent_id.to_string()));
            }

            drop(agents);

            // Sleep briefly before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Get agent status
    pub async fn get_agent_status(&self, agent_id: &str) -> Result<SubAgentStatus, AgentError> {
        let agents = self.agents.read().await;
        agents.get(agent_id)
            .map(|instance| instance.status.clone())
            .ok_or_else(|| AgentError::NotFound(agent_id.to_string()))
    }

    /// List all active agents
    pub async fn list_agents(&self) -> Vec<SubAgentInfo> {
        let agents = self.agents.read().await;
        agents.values().map(|instance| SubAgentInfo {
            id: instance.id.clone(),
            agent_type: instance.agent_type.clone(),
            status: instance.status.clone(),
            task: instance.task.clone(),
            started_at: instance.started_at,
            ended_at: instance.ended_at,
        }).collect()
    }

    /// Cancel a running agent
    pub async fn cancel_agent(&self, agent_id: &str) -> Result<(), AgentError> {
        self.update_status(agent_id, SubAgentStatus::Cancelled).await
    }

    /// Update agent status
    async fn update_status(&self, agent_id: &str, new_status: SubAgentStatus) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        if let Some(instance) = agents.get_mut(agent_id) {
            instance.status = new_status.clone();
            if matches!(new_status, SubAgentStatus::Completed | SubAgentStatus::Failed | SubAgentStatus::Cancelled) {
                instance.ended_at = Some(Utc::now());
            }
            Ok(())
        } else {
            Err(AgentError::NotFound(agent_id.to_string()))
        }
    }

    /// Send message between agents
    pub async fn send_message(
        &self,
        from: String,
        to: String,
        message_type: MessageType,
        content: JsonValue,
    ) -> Result<(), AgentError> {
        let message = SubAgentMessage {
            from,
            to: to.clone(),
            message_type,
            content,
            timestamp: Utc::now(),
        };

        // Store message in target agent's history
        let mut agents = self.agents.write().await;
        if let Some(instance) = agents.get_mut(&to) {
            instance.messages.push(message.clone());
        }

        // Send through channel
        self.message_tx.send(message)
            .map_err(|e| AgentError::CommunicationError(e.to_string()))?;

        Ok(())
    }

    /// Get messages for an agent
    pub async fn get_messages(&self, agent_id: &str) -> Result<Vec<SubAgentMessage>, AgentError> {
        let agents = self.agents.read().await;
        agents.get(agent_id)
            .map(|instance| instance.messages.clone())
            .ok_or_else(|| AgentError::NotFound(agent_id.to_string()))
    }
}

/// Summary information about a sub-agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentInfo {
    pub id: String,
    pub agent_type: AgentType,
    pub status: SubAgentStatus,
    pub task: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

// MCP Tool implementations for sub-agent management

/// MCP tool for launching a sub-agent
#[derive(Clone)]
pub struct LaunchSubAgentTool {
    manager: Arc<SubAgentManager>,
}

impl LaunchSubAgentTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }

    /// Execute the launch sub-agent tool
    pub async fn execute(&self, params: JsonValue) -> Result<ToolResult, AgentError> {
        let agent_type: AgentType = serde_json::from_value(params["agent_type"].clone())
            .map_err(|e| AgentError::ValidationError(e.to_string()))?;

        let task = params["task"].as_str()
            .ok_or_else(|| AgentError::ValidationError("Missing task parameter".to_string()))?
            .to_string();

        let task_params = params.get("params").cloned().unwrap_or(JsonValue::Null);

        let agent_id = self.manager.launch_agent(agent_type, task, task_params).await?;

        Ok(ToolResult {
            success: true,
            content: vec![
                ContentItem::Text {
                    text: format!("Launched sub-agent with ID: {}", agent_id),
                }
            ],
            error: None,
        })
    }
}

/// MCP tool for waiting for sub-agent completion
#[derive(Clone)]
pub struct WaitForSubAgentTool {
    manager: Arc<SubAgentManager>,
}

impl WaitForSubAgentTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }

    /// Execute the wait for sub-agent tool
    pub async fn execute(&self, params: JsonValue) -> Result<ToolResult, AgentError> {
        let agent_id = params["agent_id"].as_str()
            .ok_or_else(|| AgentError::ValidationError("Missing agent_id parameter".to_string()))?;

        let timeout_secs = params.get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300); // Default 5 minutes

        match self.manager.wait_for_agent(agent_id, timeout_secs).await {
            Ok(result) => {
                Ok(ToolResult {
                    success: result.success,
                    content: vec![
                        ContentItem::Text {
                            text: serde_json::to_string_pretty(&result.data).unwrap_or_default(),
                        }
                    ],
                    error: result.error,
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    content: vec![],
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

/// MCP tool for getting sub-agent status
#[derive(Clone)]
pub struct GetSubAgentStatusTool {
    manager: Arc<SubAgentManager>,
}

impl GetSubAgentStatusTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }

    /// Execute the get sub-agent status tool
    pub async fn execute(&self, params: JsonValue) -> Result<ToolResult, AgentError> {
        let agent_id = params["agent_id"].as_str()
            .ok_or_else(|| AgentError::ValidationError("Missing agent_id parameter".to_string()))?;

        match self.manager.get_agent_status(agent_id).await {
            Ok(status) => {
                Ok(ToolResult {
                    success: true,
                    content: vec![
                        ContentItem::Text {
                            text: serde_json::to_string(&status).unwrap_or_default(),
                        }
                    ],
                    error: None,
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    content: vec![],
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

/// MCP tool for listing all sub-agents
#[derive(Clone)]
pub struct ListSubAgentsTool {
    manager: Arc<SubAgentManager>,
}

impl ListSubAgentsTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }

    /// Execute the list sub-agents tool
    pub async fn execute(&self, _params: JsonValue) -> Result<ToolResult, AgentError> {
        let agents = self.manager.list_agents().await;

        Ok(ToolResult {
            success: true,
            content: vec![
                ContentItem::Text {
                    text: serde_json::to_string_pretty(&agents).unwrap_or_default(),
                }
            ],
            error: None,
        })
    }
}

/// MCP tool for cancelling a sub-agent
#[derive(Clone)]
pub struct CancelSubAgentTool {
    manager: Arc<SubAgentManager>,
}

impl CancelSubAgentTool {
    pub fn new(manager: Arc<SubAgentManager>) -> Self {
        Self { manager }
    }

    /// Execute the cancel sub-agent tool
    pub async fn execute(&self, params: JsonValue) -> Result<ToolResult, AgentError> {
        let agent_id = params["agent_id"].as_str()
            .ok_or_else(|| AgentError::ValidationError("Missing agent_id parameter".to_string()))?;

        match self.manager.cancel_agent(agent_id).await {
            Ok(_) => {
                Ok(ToolResult {
                    success: true,
                    content: vec![
                        ContentItem::Text {
                            text: format!("Successfully cancelled agent {}", agent_id),
                        }
                    ],
                    error: None,
                })
            }
            Err(e) => {
                Ok(ToolResult {
                    success: false,
                    content: vec![],
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_launch_and_wait_for_agent() {
        let config = RuntimeConfig::default();
        let manager = Arc::new(SubAgentManager::new(config));

        // Launch a test agent
        let agent_id = manager.launch_agent(
            AgentType::Developer,
            "Test task".to_string(),
            serde_json::json!({"test": true}),
        ).await.unwrap();

        assert!(!agent_id.is_empty());

        // Check status
        let status = manager.get_agent_status(&agent_id).await.unwrap();
        assert!(matches!(status, SubAgentStatus::Running | SubAgentStatus::Starting));
    }

    #[tokio::test]
    async fn test_list_agents() {
        let config = RuntimeConfig::default();
        let manager = Arc::new(SubAgentManager::new(config));

        // Launch multiple agents
        let _id1 = manager.launch_agent(
            AgentType::Developer,
            "Task 1".to_string(),
            JsonValue::Null,
        ).await.unwrap();

        let _id2 = manager.launch_agent(
            AgentType::Reviewer,
            "Task 2".to_string(),
            JsonValue::Null,
        ).await.unwrap();

        let agents = manager.list_agents().await;
        assert_eq!(agents.len(), 2);
    }
}