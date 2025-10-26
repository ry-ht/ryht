//! Agent Messaging Adapter - Simplified interface for agents to use unified messaging
//!
//! This module provides a high-level adapter that agents can use to interact with
//! the unified message bus without needing to understand all the low-level details.

use super::*;
use crate::agents::AgentId;
use crate::coordination::{
    Message, MessageEnvelope, MessageCoordinator, UnifiedMessageBus,
};
use crate::cortex_bridge::{CortexBridge, SessionId, WorkspaceId, LockType};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info};

// ==============================================================================
// Agent Messaging Adapter
// ==============================================================================

/// High-level messaging adapter for agents
///
/// This provides a simplified interface that agents can use without worrying
/// about the complexity of the underlying unified message bus.
pub struct AgentMessagingAdapter {
    /// Agent identifier
    agent_id: AgentId,

    /// Current session
    session_id: SessionId,

    /// Current workspace
    workspace_id: WorkspaceId,

    /// Message receiver for this agent
    receiver: mpsc::UnboundedReceiver<MessageEnvelope>,

    /// Message bus reference
    bus: Arc<UnifiedMessageBus>,

    /// Coordinator reference
    coordinator: Arc<MessageCoordinator>,

    /// Cortex bridge
    cortex: Arc<CortexBridge>,
}

impl AgentMessagingAdapter {
    /// Create a new messaging adapter for an agent
    pub async fn new(
        agent_id: AgentId,
        session_id: SessionId,
        workspace_id: WorkspaceId,
        bus: Arc<UnifiedMessageBus>,
        coordinator: Arc<MessageCoordinator>,
        cortex: Arc<CortexBridge>,
    ) -> Result<Self> {
        info!("Creating messaging adapter for agent {} in session {}", agent_id, session_id);

        // Register agent with bus
        let receiver = bus.register_agent(agent_id.clone(), session_id.clone()).await?;

        Ok(Self {
            agent_id,
            session_id,
            workspace_id,
            receiver,
            bus,
            coordinator,
            cortex,
        })
    }

    // ==========================================================================
    // Simple Messaging API
    // ==========================================================================

    /// Send a message to another agent
    pub async fn send_to_agent(
        &self,
        target: AgentId,
        message: Message,
    ) -> Result<()> {
        let envelope = self.create_envelope(Some(target), None, message, 5);
        self.bus.send(envelope).await
    }

    /// Send a high-priority message to another agent
    pub async fn send_urgent(
        &self,
        target: AgentId,
        message: Message,
    ) -> Result<()> {
        let envelope = self.create_envelope(Some(target), None, message, 9);
        self.bus.send(envelope).await
    }

    /// Publish a message to a topic
    pub async fn publish_to_topic(
        &self,
        topic: String,
        message: Message,
    ) -> Result<usize> {
        let envelope = self.create_envelope(None, Some(topic), message, 5);
        self.bus.publish(envelope).await
    }

    /// Receive the next message (blocking)
    pub async fn receive(&mut self) -> Option<MessageEnvelope> {
        self.receiver.recv().await
    }

    /// Try to receive a message (non-blocking)
    pub fn try_receive(&mut self) -> Option<MessageEnvelope> {
        self.receiver.try_recv().ok()
    }

    /// Request and wait for a response
    pub async fn request(
        &self,
        target: AgentId,
        message: Message,
        timeout: Duration,
    ) -> Result<MessageEnvelope> {
        let envelope = self.create_envelope_with_correlation(
            Some(target),
            None,
            message,
            5,
            Some(uuid::Uuid::new_v4().to_string()),
        );

        self.coordinator.request_response(envelope, timeout).await
    }

    // ==========================================================================
    // Coordination Helpers
    // ==========================================================================

    /// Acquire a lock on an entity
    pub async fn acquire_lock(
        &self,
        entity_id: String,
        lock_type: LockType,
    ) -> Result<String> {
        self.coordinator.acquire_coordinated_lock(
            entity_id,
            lock_type,
            self.agent_id.clone(),
            self.session_id.clone(),
            self.workspace_id.clone(),
        ).await
    }

    /// Release a lock
    pub async fn release_lock(
        &self,
        entity_id: String,
        lock_id: String,
    ) -> Result<()> {
        self.coordinator.release_coordinated_lock(
            entity_id,
            lock_id,
            self.agent_id.clone(),
            self.session_id.clone(),
            self.workspace_id.clone(),
        ).await
    }

    /// Share knowledge with specific agents
    pub async fn share_knowledge(
        &self,
        episode_id: String,
        summary: String,
        insights: Vec<String>,
        target_agents: Vec<AgentId>,
    ) -> Result<()> {
        self.coordinator.share_knowledge(
            episode_id,
            summary,
            insights,
            self.agent_id.clone(),
            target_agents,
            self.session_id.clone(),
            self.workspace_id.clone(),
        ).await
    }

    /// Broadcast knowledge to a topic
    pub async fn broadcast_knowledge(
        &self,
        episode_id: String,
        summary: String,
        insights: Vec<String>,
        topic: String,
    ) -> Result<usize> {
        self.coordinator.broadcast_knowledge(
            episode_id,
            summary,
            insights,
            self.agent_id.clone(),
            topic,
            self.session_id.clone(),
            self.workspace_id.clone(),
        ).await
    }

    /// Request assistance from another agent
    pub async fn request_assistance(
        &self,
        target: AgentId,
        description: String,
        urgency: u8,
    ) -> Result<()> {
        let message = Message::AssistanceRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            requesting_agent: self.agent_id.clone(),
            description,
            urgency,
        };

        self.send_to_agent(target, message).await
    }

    /// Respond to assistance request
    pub async fn respond_to_assistance(
        &self,
        request_id: String,
        requester: AgentId,
        accepting: bool,
        estimated_time: Option<Duration>,
    ) -> Result<()> {
        let message = Message::AssistanceResponse {
            request_id,
            accepting,
            estimated_time,
        };

        self.send_to_agent(requester, message).await
    }

    /// Update task progress
    pub async fn update_task_progress(
        &self,
        task_id: String,
        progress: f32,
        status: String,
        details: serde_json::Value,
        coordinator: AgentId,
    ) -> Result<()> {
        let message = Message::TaskProgress {
            task_id,
            progress,
            status,
            details,
        };

        self.send_to_agent(coordinator, message).await
    }

    /// Report task completion
    pub async fn complete_task(
        &self,
        task_id: String,
        result: serde_json::Value,
        success: bool,
        artifacts: Vec<String>,
        coordinator: AgentId,
    ) -> Result<()> {
        let message = Message::TaskComplete {
            task_id,
            result,
            success,
            artifacts,
        };

        self.send_to_agent(coordinator, message).await
    }

    /// Report task failure
    pub async fn fail_task(
        &self,
        task_id: String,
        error: String,
        recoverable: bool,
        coordinator: AgentId,
    ) -> Result<()> {
        let message = Message::TaskFailed {
            task_id,
            error,
            recoverable,
        };

        self.send_to_agent(coordinator, message).await
    }

    /// Send health check response
    pub async fn respond_to_health_check(
        &self,
        requester: AgentId,
        status: String,
        load: f32,
    ) -> Result<()> {
        let message = Message::HealthPong { status, load };
        self.send_to_agent(requester, message).await
    }

    // ==========================================================================
    // Subscriptions
    // ==========================================================================

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) -> tokio::sync::broadcast::Receiver<MessageEnvelope> {
        self.bus.subscribe(topic).await
    }

    // ==========================================================================
    // Message History & Replay
    // ==========================================================================

    /// Get message history for current session
    pub async fn get_message_history(&self) -> Result<Vec<MessageEnvelope>> {
        self.bus.replay_session(&self.session_id).await
    }

    /// Replay messages from episodic memory
    pub async fn replay_from_memory(&self, limit: usize) -> Result<Vec<MessageEnvelope>> {
        self.bus.replay_from_episodic(&self.session_id, limit).await
    }

    // ==========================================================================
    // Utilities
    // ==========================================================================

    /// Get agent ID
    pub fn agent_id(&self) -> &AgentId {
        &self.agent_id
    }

    /// Get session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Get workspace ID
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Create a basic message envelope
    fn create_envelope(
        &self,
        to: Option<AgentId>,
        topic: Option<String>,
        payload: Message,
        priority: u8,
    ) -> MessageEnvelope {
        MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: None,
            causation_id: None,
            from: self.agent_id.clone(),
            to,
            topic,
            session_id: self.session_id.clone(),
            workspace_id: self.workspace_id.clone(),
            payload,
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority,
            attempt_count: 0,
            max_attempts: 3,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a message envelope with correlation ID
    fn create_envelope_with_correlation(
        &self,
        to: Option<AgentId>,
        topic: Option<String>,
        payload: Message,
        priority: u8,
        correlation_id: Option<String>,
    ) -> MessageEnvelope {
        let mut envelope = self.create_envelope(to, topic, payload, priority);
        envelope.correlation_id = correlation_id;
        envelope
    }
}

impl Drop for AgentMessagingAdapter {
    fn drop(&mut self) {
        debug!("Dropping messaging adapter for agent {}", self.agent_id);
        // Unregistration will be handled by the orchestrator
    }
}

// ==============================================================================
// Convenience Builder
// ==============================================================================

/// Builder for agent messaging adapter
pub struct AgentMessagingAdapterBuilder {
    agent_id: Option<AgentId>,
    session_id: Option<SessionId>,
    workspace_id: Option<WorkspaceId>,
    bus: Option<Arc<UnifiedMessageBus>>,
    coordinator: Option<Arc<MessageCoordinator>>,
    cortex: Option<Arc<CortexBridge>>,
}

impl AgentMessagingAdapterBuilder {
    pub fn new() -> Self {
        Self {
            agent_id: None,
            session_id: None,
            workspace_id: None,
            bus: None,
            coordinator: None,
            cortex: None,
        }
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn session_id(mut self, session_id: SessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn workspace_id(mut self, workspace_id: WorkspaceId) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    pub fn bus(mut self, bus: Arc<UnifiedMessageBus>) -> Self {
        self.bus = Some(bus);
        self
    }

    pub fn coordinator(mut self, coordinator: Arc<MessageCoordinator>) -> Self {
        self.coordinator = Some(coordinator);
        self
    }

    pub fn cortex(mut self, cortex: Arc<CortexBridge>) -> Self {
        self.cortex = Some(cortex);
        self
    }

    pub async fn build(self) -> Result<AgentMessagingAdapter> {
        let agent_id = self.agent_id.ok_or_else(|| {
            CoordinationError::CommunicationError("Agent ID required".to_string())
        })?;

        let session_id = self.session_id.ok_or_else(|| {
            CoordinationError::CommunicationError("Session ID required".to_string())
        })?;

        let workspace_id = self.workspace_id.ok_or_else(|| {
            CoordinationError::CommunicationError("Workspace ID required".to_string())
        })?;

        let bus = self.bus.ok_or_else(|| {
            CoordinationError::CommunicationError("Message bus required".to_string())
        })?;

        let coordinator = self.coordinator.ok_or_else(|| {
            CoordinationError::CommunicationError("Coordinator required".to_string())
        })?;

        let cortex = self.cortex.ok_or_else(|| {
            CoordinationError::CommunicationError("Cortex bridge required".to_string())
        })?;

        AgentMessagingAdapter::new(
            agent_id,
            session_id,
            workspace_id,
            bus,
            coordinator,
            cortex,
        ).await
    }
}

impl Default for AgentMessagingAdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        let builder = AgentMessagingAdapterBuilder::new()
            .agent_id(AgentId::from("test-agent".to_string()))
            .session_id(SessionId::from("test-session".to_string()))
            .workspace_id(WorkspaceId::from("test-workspace".to_string()));

        // Builder should have all required fields set
        assert!(builder.agent_id.is_some());
        assert!(builder.session_id.is_some());
        assert!(builder.workspace_id.is_some());
    }
}
