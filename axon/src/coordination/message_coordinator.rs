//! Message Coordinator - High-level coordination patterns using unified message bus
//!
//! This module provides coordination patterns and workflows built on top of the
//! unified message bus, leveraging Cortex's distributed locking and sessions.

use super::*;
use crate::agents::AgentId;
use crate::coordination::unified_message_bus::{
    Message, MessageEnvelope, UnifiedMessageBus, EventSeverity,
};
use crate::cortex_bridge::{CortexBridge, LockType, SessionId, WorkspaceId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, info, warn};

// ==============================================================================
// Coordination Patterns
// ==============================================================================

/// Coordinator for high-level multi-agent coordination patterns
pub struct MessageCoordinator {
    /// Unified message bus
    bus: Arc<UnifiedMessageBus>,

    /// Cortex bridge for distributed operations
    cortex: Arc<CortexBridge>,

    /// Pending requests: request_id -> response channel
    pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<MessageEnvelope>>>>,

    /// Active locks: entity_id -> (agent_id, lock_id)
    active_locks: Arc<RwLock<HashMap<String, (AgentId, String)>>>,
}

impl MessageCoordinator {
    /// Create a new message coordinator
    pub fn new(bus: Arc<UnifiedMessageBus>, cortex: Arc<CortexBridge>) -> Self {
        Self {
            bus,
            cortex,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            active_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ==========================================================================
    // Request/Response Pattern
    // ==========================================================================

    /// Send a request and wait for response (with timeout)
    pub async fn request_response(
        &self,
        envelope: MessageEnvelope,
        timeout_duration: Duration,
    ) -> Result<MessageEnvelope> {
        let request_id = envelope.correlation_id.clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let (tx, rx) = tokio::sync::oneshot::channel();

        // Register pending request
        self.pending_requests.write().await.insert(request_id.clone(), tx);

        // Send request
        self.bus.send(envelope).await?;

        // Wait for response with timeout
        match timeout(timeout_duration, rx).await {
            Ok(Ok(response)) => {
                self.pending_requests.write().await.remove(&request_id);
                Ok(response)
            }
            Ok(Err(_)) => {
                self.pending_requests.write().await.remove(&request_id);
                Err(CoordinationError::CommunicationError(
                    "Response channel closed".to_string()
                ))
            }
            Err(_) => {
                self.pending_requests.write().await.remove(&request_id);
                Err(CoordinationError::CommunicationError(
                    "Request timeout".to_string()
                ))
            }
        }
    }

    /// Handle incoming response
    pub async fn handle_response(&self, envelope: MessageEnvelope) -> Result<()> {
        if let Some(correlation_id) = &envelope.correlation_id {
            let mut pending = self.pending_requests.write().await;
            if let Some(tx) = pending.remove(correlation_id) {
                let _ = tx.send(envelope);
                return Ok(());
            }
        }

        warn!("Received response with no matching request");
        Ok(())
    }

    // ==========================================================================
    // Coordinated Lock Pattern
    // ==========================================================================

    /// Request a coordinated lock through Cortex
    pub async fn acquire_coordinated_lock(
        &self,
        entity_id: String,
        lock_type: LockType,
        agent_id: AgentId,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<String> {
        use crate::cortex_bridge::AgentId as CortexAgentId;

        info!("Agent {} requesting {} lock on {}", agent_id,
              if matches!(lock_type, LockType::Shared) { "shared" } else { "exclusive" },
              entity_id);

        // Convert AgentId to CortexAgentId
        let cortex_agent_id = CortexAgentId::from(agent_id.to_string());

        // Acquire lock through Cortex
        let lock_id = self.cortex
            .acquire_lock(&entity_id, lock_type, &cortex_agent_id, &session_id)
            .await
            .map_err(|e| CoordinationError::Other(e.into()))?;

        // Track active lock
        self.active_locks
            .write()
            .await
            .insert(entity_id.clone(), (agent_id.clone(), lock_id.to_string()));

        // Broadcast lock acquisition event
        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            causation_id: None,
            from: agent_id,
            to: None,
            topic: Some("system.locks".to_string()),
            session_id,
            workspace_id,
            payload: Message::SystemEvent {
                event_type: "lock_acquired".to_string(),
                severity: EventSeverity::Info,
                data: serde_json::json!({
                    "entity_id": entity_id,
                    "lock_id": lock_id.to_string(),
                    "lock_type": format!("{:?}", lock_type),
                }),
            },
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 7,
            attempt_count: 0,
            max_attempts: 1,
            metadata: HashMap::new(),
        };

        let _ = self.bus.publish(envelope).await;

        Ok(lock_id.to_string())
    }

    /// Release a coordinated lock
    pub async fn release_coordinated_lock(
        &self,
        entity_id: String,
        lock_id: String,
        agent_id: AgentId,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<()> {
        use crate::cortex_bridge::LockId;

        info!("Agent {} releasing lock {} on {}", agent_id, lock_id, entity_id);

        // Convert String to LockId
        let cortex_lock_id = LockId::from(lock_id.clone());

        // Release lock through Cortex
        self.cortex.release_lock(&cortex_lock_id)
            .await
            .map_err(|e| CoordinationError::Other(e.into()))?;

        // Remove from tracking
        self.active_locks.write().await.remove(&entity_id);

        // Broadcast lock release event
        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            causation_id: None,
            from: agent_id,
            to: None,
            topic: Some("system.locks".to_string()),
            session_id,
            workspace_id,
            payload: Message::SystemEvent {
                event_type: "lock_released".to_string(),
                severity: EventSeverity::Info,
                data: serde_json::json!({
                    "entity_id": entity_id,
                    "lock_id": lock_id,
                }),
            },
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 7,
            attempt_count: 0,
            max_attempts: 1,
            metadata: HashMap::new(),
        };

        let _ = self.bus.publish(envelope).await;

        Ok(())
    }

    /// Check if an entity is locked
    pub async fn is_entity_locked(&self, entity_id: &str) -> Result<bool> {
        self.cortex.is_locked(entity_id)
            .await
            .map_err(|e| CoordinationError::Other(e.into()))
    }

    /// Get lock holder for an entity
    pub async fn get_lock_holder(&self, entity_id: &str) -> Option<AgentId> {
        self.active_locks
            .read()
            .await
            .get(entity_id)
            .map(|(agent_id, _)| agent_id.clone())
    }

    // ==========================================================================
    // Workflow Coordination Pattern
    // ==========================================================================

    /// Coordinate a multi-agent workflow with task distribution
    pub async fn coordinate_workflow(
        &self,
        workflow_id: String,
        tasks: Vec<WorkflowTask>,
        coordinator_agent: AgentId,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<WorkflowExecution> {
        info!("Starting workflow {} with {} tasks", workflow_id, tasks.len());

        let execution = WorkflowExecution {
            workflow_id: workflow_id.clone(),
            status: WorkflowStatus::Running,
            tasks: tasks.clone(),
            completed_tasks: vec![],
            failed_tasks: vec![],
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        // Assign tasks to agents
        for task in tasks {
            let envelope = MessageEnvelope {
                message_id: uuid::Uuid::new_v4().to_string(),
                correlation_id: Some(workflow_id.clone()),
                causation_id: None,
                from: coordinator_agent.clone(),
                to: Some(task.assigned_agent.clone()),
                topic: None,
                session_id: session_id.clone(),
                workspace_id: workspace_id.clone(),
                payload: Message::TaskAssignment {
                    task_id: task.task_id.clone(),
                    task_description: task.description.clone(),
                    context: task.context.clone(),
                },
                timestamp: chrono::Utc::now(),
                expires_at: task.deadline,
                priority: task.priority,
                attempt_count: 0,
                max_attempts: 3,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("workflow_id".to_string(), workflow_id.clone());
                    meta
                },
            };

            self.bus.send(envelope).await?;
        }

        Ok(execution)
    }

    /// Update workflow task status
    pub async fn update_workflow_task(
        &self,
        workflow_id: String,
        task_id: String,
        status: TaskStatus,
        _result: Option<serde_json::Value>,
    ) -> Result<()> {
        debug!("Updating workflow {} task {} to {:?}", workflow_id, task_id, status);

        // Would update workflow state here
        // This is a simplified version - production would use a workflow engine

        Ok(())
    }

    // ==========================================================================
    // Knowledge Sharing Pattern
    // ==========================================================================

    /// Share knowledge (episode) with specific agents
    pub async fn share_knowledge(
        &self,
        episode_id: String,
        summary: String,
        insights: Vec<String>,
        source_agent: AgentId,
        target_agents: Vec<AgentId>,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<()> {
        use crate::cortex_bridge::EpisodeId;

        info!("Agent {} sharing episode {} with {} agents",
              source_agent, episode_id, target_agents.len());

        for target in target_agents {
            let envelope = MessageEnvelope {
                message_id: uuid::Uuid::new_v4().to_string(),
                correlation_id: None,
                causation_id: None,
                from: source_agent.clone(),
                to: Some(target),
                topic: None,
                session_id: session_id.clone(),
                workspace_id: workspace_id.clone(),
                payload: Message::KnowledgeShare {
                    episode_id: EpisodeId::from(episode_id.clone()),
                    summary: summary.clone(),
                    insights: insights.clone(),
                },
                timestamp: chrono::Utc::now(),
                expires_at: None,
                priority: 6,
                attempt_count: 0,
                max_attempts: 3,
                metadata: HashMap::new(),
            };

            self.bus.send(envelope).await?;
        }

        Ok(())
    }

    /// Broadcast knowledge to all agents on a topic
    pub async fn broadcast_knowledge(
        &self,
        episode_id: String,
        summary: String,
        insights: Vec<String>,
        source_agent: AgentId,
        topic: String,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<usize> {
        use crate::cortex_bridge::EpisodeId;

        info!("Broadcasting episode {} to topic {}", episode_id, topic);

        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            causation_id: None,
            from: source_agent,
            to: None,
            topic: Some(topic),
            session_id,
            workspace_id,
            payload: Message::KnowledgeShare {
                episode_id: EpisodeId::from(episode_id),
                summary,
                insights,
            },
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 6,
            attempt_count: 0,
            max_attempts: 1,
            metadata: HashMap::new(),
        };

        self.bus.publish(envelope).await
    }

    // ==========================================================================
    // Health Monitoring Pattern
    // ==========================================================================

    /// Ping an agent for health check
    pub async fn ping_agent(
        &self,
        target_agent: AgentId,
        source_agent: AgentId,
        session_id: SessionId,
        workspace_id: WorkspaceId,
    ) -> Result<(String, f32)> {
        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: Some(uuid::Uuid::new_v4().to_string()),
            causation_id: None,
            from: source_agent,
            to: Some(target_agent.clone()),
            topic: None,
            session_id,
            workspace_id,
            payload: Message::HealthPing,
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 9,
            attempt_count: 0,
            max_attempts: 3,
            metadata: HashMap::new(),
        };

        let response = self.request_response(envelope, Duration::from_secs(5)).await?;

        match response.payload {
            Message::HealthPong { status, load } => Ok((status, load)),
            _ => Err(CoordinationError::CommunicationError(
                "Invalid health check response".to_string()
            )),
        }
    }

    /// Get active locks count
    pub async fn get_active_locks_count(&self) -> usize {
        self.active_locks.read().await.len()
    }

    /// Get pending requests count
    pub async fn get_pending_requests_count(&self) -> usize {
        self.pending_requests.read().await.len()
    }
}

// ==============================================================================
// Supporting Types
// ==============================================================================

/// Workflow task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTask {
    pub task_id: String,
    pub description: String,
    pub assigned_agent: AgentId,
    pub context: serde_json::Value,
    pub priority: u8,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub dependencies: Vec<String>,
}

/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub tasks: Vec<WorkflowTask>,
    pub completed_tasks: Vec<String>,
    pub failed_tasks: Vec<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Workflow status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_task_creation() {
        let task = WorkflowTask {
            task_id: "task-1".to_string(),
            description: "Test task".to_string(),
            assigned_agent: AgentId::from_string("agent-1"),
            context: serde_json::json!({}),
            priority: 5,
            deadline: None,
            dependencies: vec![],
        };

        assert_eq!(task.task_id, "task-1");
        assert_eq!(task.priority, 5);
    }

    #[test]
    fn test_workflow_execution_state() {
        let execution = WorkflowExecution {
            workflow_id: "workflow-1".to_string(),
            status: WorkflowStatus::Running,
            tasks: vec![],
            completed_tasks: vec![],
            failed_tasks: vec![],
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        assert_eq!(execution.status, WorkflowStatus::Running);
        assert!(execution.completed_at.is_none());
    }
}
