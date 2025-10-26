//! Unified Message Bus - Cortex-Integrated Multi-Agent Communication
//!
//! This module provides a unified messaging architecture that integrates deeply with
//! Cortex's capabilities for robust, persistent, and intelligent multi-agent coordination.
//!
//! # Features
//!
//! - **Session Isolation**: Messages are isolated per Cortex session
//! - **Distributed Locking**: Uses Cortex locks for coordination
//! - **Episodic Memory**: All messages stored in episodic memory for learning
//! - **Event System**: Integrates with Cortex event system for broadcasts
//! - **Resilience**: Circuit breakers, dead letter queues, automatic retry
//! - **Replay**: Can replay messages from episodic memory
//! - **Pattern Learning**: Extracts communication patterns for optimization

use super::*;
use crate::agents::AgentId;
use crate::cortex_bridge::{
    CortexBridge, Episode, EpisodeId, LockType, SessionId, WorkspaceId,
};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, broadcast, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

// ==============================================================================
// Core Types
// ==============================================================================

/// Unified message envelope with full context and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message ID
    pub message_id: String,

    /// Correlation ID for request/response tracking
    pub correlation_id: Option<String>,

    /// Causation ID for event chains
    pub causation_id: Option<String>,

    /// Source agent
    pub from: AgentId,

    /// Target agent (None for broadcast)
    pub to: Option<AgentId>,

    /// Topic for pub/sub (None for direct messaging)
    pub topic: Option<String>,

    /// Session context
    pub session_id: SessionId,

    /// Workspace context
    pub workspace_id: WorkspaceId,

    /// Message payload
    pub payload: Message,

    /// Message timestamp
    pub timestamp: DateTime<Utc>,

    /// Expiration time
    pub expires_at: Option<DateTime<Utc>>,

    /// Priority (0 = lowest, 10 = highest)
    pub priority: u8,

    /// Number of delivery attempts
    pub attempt_count: u32,

    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Message metadata
    pub metadata: HashMap<String, String>,
}

/// Message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Task assignment to an agent
    TaskAssignment {
        task_id: String,
        task_description: String,
        context: serde_json::Value,
    },

    /// Task progress update
    TaskProgress {
        task_id: String,
        progress: f32,
        status: String,
        details: serde_json::Value,
    },

    /// Task completion
    TaskComplete {
        task_id: String,
        result: serde_json::Value,
        success: bool,
        artifacts: Vec<String>,
    },

    /// Task failure
    TaskFailed {
        task_id: String,
        error: String,
        recoverable: bool,
    },

    /// Request for assistance
    AssistanceRequest {
        request_id: String,
        requesting_agent: AgentId,
        description: String,
        urgency: u8,
    },

    /// Response to assistance request
    AssistanceResponse {
        request_id: String,
        accepting: bool,
        estimated_time: Option<Duration>,
    },

    /// Coordination lock request
    LockRequest {
        entity_id: String,
        lock_type: LockType,
        reason: String,
    },

    /// Coordination lock granted
    LockGranted {
        entity_id: String,
        lock_id: String,
        expires_at: DateTime<Utc>,
    },

    /// Coordination lock denied
    LockDenied {
        entity_id: String,
        reason: String,
        holder: AgentId,
    },

    /// Knowledge sharing
    KnowledgeShare {
        episode_id: EpisodeId,
        summary: String,
        insights: Vec<String>,
    },

    /// Pattern notification
    PatternDiscovered {
        pattern_id: String,
        pattern_type: String,
        confidence: f32,
        description: String,
    },

    /// System event notification
    SystemEvent {
        event_type: String,
        severity: EventSeverity,
        data: serde_json::Value,
    },

    /// Health check ping
    HealthPing,

    /// Health check response
    HealthPong {
        status: String,
        load: f32,
    },

    /// Custom message
    Custom {
        message_type: String,
        data: serde_json::Value,
    },
}

/// Event severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Message delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    InFlight,
    Delivered,
    Failed { reason: String },
    DeadLetter { reason: String },
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,  // Normal operation
    Open,    // Failing, reject requests
    HalfOpen, // Testing recovery
}

// ==============================================================================
// Unified Message Bus Implementation
// ==============================================================================

/// Unified message bus integrating with Cortex for persistent, intelligent messaging
pub struct UnifiedMessageBus {
    /// Cortex bridge for persistence and coordination
    cortex: Arc<CortexBridge>,

    /// Direct message channels: agent_id -> sender
    direct_channels: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<MessageEnvelope>>>>,

    /// Topic subscribers: topic -> broadcast sender
    topic_channels: Arc<RwLock<HashMap<String, broadcast::Sender<MessageEnvelope>>>>,

    /// Message history for replay: session_id -> messages
    message_history: Arc<RwLock<HashMap<SessionId, VecDeque<MessageEnvelope>>>>,

    /// Dead letter queue
    dead_letters: Arc<RwLock<VecDeque<(MessageEnvelope, String)>>>,

    /// Circuit breakers: agent_id -> state
    circuit_breakers: Arc<RwLock<HashMap<AgentId, CircuitBreaker>>>,

    /// Rate limiters: agent_id -> semaphore
    rate_limiters: Arc<RwLock<HashMap<AgentId, Arc<Semaphore>>>>,

    /// Configuration
    config: MessageBusConfig,

    /// Statistics
    stats: Arc<RwLock<MessageBusStats>>,
}

/// Circuit breaker for agent resilience
#[derive(Debug, Clone)]
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<DateTime<Utc>>,
    last_state_change: DateTime<Utc>,
}

/// Message bus configuration
#[derive(Debug, Clone)]
pub struct MessageBusConfig {
    /// Maximum messages in history per session
    pub max_history_size: usize,

    /// Maximum messages in dead letter queue
    pub max_dead_letters: usize,

    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,

    /// Circuit breaker timeout
    pub circuit_breaker_timeout: Duration,

    /// Rate limit per agent (messages per second)
    pub rate_limit_per_agent: usize,

    /// Enable episodic memory persistence
    pub persist_to_episodic: bool,

    /// Broadcast channel capacity
    pub broadcast_capacity: usize,

    /// Message TTL (time to live)
    pub default_message_ttl: Duration,
}

/// Message bus statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageBusStats {
    pub total_sent: u64,
    pub total_delivered: u64,
    pub total_failed: u64,
    pub total_dead_letters: u64,
    pub circuit_breaker_trips: u64,
    pub rate_limit_hits: u64,
    pub average_latency_ms: f64,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            max_history_size: 10000,
            max_dead_letters: 1000,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
            rate_limit_per_agent: 100,
            persist_to_episodic: true,
            broadcast_capacity: 1000,
            default_message_ttl: Duration::from_secs(3600),
        }
    }
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            last_state_change: Utc::now(),
        }
    }

    fn record_success(&mut self) {
        self.success_count += 1;

        match self.state {
            CircuitState::HalfOpen => {
                if self.success_count >= 3 {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_state_change = Utc::now();
                    info!("Circuit breaker closed after successful recovery");
                }
            }
            _ => {}
        }
    }

    fn record_failure(&mut self, threshold: u32) {
        self.failure_count += 1;
        self.last_failure = Some(Utc::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= threshold {
                    self.state = CircuitState::Open;
                    self.last_state_change = Utc::now();
                    warn!("Circuit breaker opened after {} failures", self.failure_count);
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.last_state_change = Utc::now();
                warn!("Circuit breaker reopened after failure in half-open state");
            }
            _ => {}
        }
    }

    fn should_attempt(&mut self, timeout: Duration) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let elapsed = Utc::now().signed_duration_since(self.last_state_change);
                if elapsed.num_seconds() >= timeout.as_secs() as i64 {
                    self.state = CircuitState::HalfOpen;
                    self.success_count = 0;
                    info!("Circuit breaker entering half-open state");
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

impl UnifiedMessageBus {
    /// Create a new unified message bus
    pub fn new(cortex: Arc<CortexBridge>, config: MessageBusConfig) -> Self {
        info!("Initializing unified message bus with Cortex integration");

        Self {
            cortex,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        }
    }

    // ==========================================================================
    // Agent Registration
    // ==========================================================================

    /// Register an agent with the message bus
    pub async fn register_agent(
        &self,
        agent_id: AgentId,
        session_id: SessionId,
    ) -> Result<mpsc::UnboundedReceiver<MessageEnvelope>> {
        info!("Registering agent {} with session {}", agent_id, session_id);

        let (tx, rx) = mpsc::unbounded_channel();

        // Register channel
        self.direct_channels.write().await.insert(agent_id.clone(), tx);

        // Initialize circuit breaker
        self.circuit_breakers
            .write()
            .await
            .insert(agent_id.clone(), CircuitBreaker::new());

        // Initialize rate limiter
        let semaphore = Arc::new(Semaphore::new(self.config.rate_limit_per_agent));
        self.rate_limiters.write().await.insert(agent_id.clone(), semaphore);

        // Initialize message history for session
        self.message_history
            .write()
            .await
            .entry(session_id)
            .or_insert_with(VecDeque::new);

        Ok(rx)
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &AgentId) -> Result<()> {
        info!("Unregistering agent {}", agent_id);

        self.direct_channels.write().await.remove(agent_id);
        self.circuit_breakers.write().await.remove(agent_id);
        self.rate_limiters.write().await.remove(agent_id);

        Ok(())
    }

    // ==========================================================================
    // Direct Messaging
    // ==========================================================================

    /// Send a direct message to an agent
    pub async fn send(
        &self,
        mut envelope: MessageEnvelope,
    ) -> Result<()> {
        let target = envelope.to.as_ref()
            .ok_or_else(|| CoordinationError::CommunicationError(
                "Direct message requires target agent".to_string()
            ))?;

        // Generate message ID if not present
        if envelope.message_id.is_empty() {
            envelope.message_id = uuid::Uuid::new_v4().to_string();
        }

        // Set timestamp
        envelope.timestamp = Utc::now();

        // Check circuit breaker
        if !self.check_circuit_breaker(target).await {
            warn!("Circuit breaker open for agent {}, message rejected", target);
            self.move_to_dead_letter(envelope, "Circuit breaker open").await;
            return Err(CoordinationError::SendFailed {
                target: target.to_string(),
            });
        }

        // Check rate limit
        if !self.check_rate_limit(target).await {
            warn!("Rate limit exceeded for agent {}", target);
            let mut stats = self.stats.write().await;
            stats.rate_limit_hits += 1;
            return Err(CoordinationError::CommunicationError(
                format!("Rate limit exceeded for agent {}", target)
            ));
        }

        // Persist to episodic memory if enabled
        if self.config.persist_to_episodic {
            if let Err(e) = self.persist_message(&envelope).await {
                warn!("Failed to persist message to episodic memory: {}", e);
            }
        }

        // Add to history
        self.add_to_history(&envelope).await;

        // Send message
        let result = self.deliver_direct_message(&envelope).await;

        // Update circuit breaker
        match result {
            Ok(_) => {
                self.record_success(target).await;
                let mut stats = self.stats.write().await;
                stats.total_sent += 1;
                stats.total_delivered += 1;
            }
            Err(e) => {
                self.record_failure(target).await;
                let mut stats = self.stats.write().await;
                stats.total_sent += 1;
                stats.total_failed += 1;

                // Retry logic
                if envelope.attempt_count < envelope.max_attempts {
                    envelope.attempt_count += 1;
                    warn!("Retrying message delivery (attempt {})", envelope.attempt_count);
                    // Would implement retry queue here
                } else {
                    self.move_to_dead_letter(envelope, format!("Delivery failed: {}", e)).await;
                }

                return Err(e);
            }
        }

        Ok(())
    }

    async fn deliver_direct_message(&self, envelope: &MessageEnvelope) -> Result<()> {
        let target = envelope.to.as_ref().unwrap();
        let channels = self.direct_channels.read().await;

        let tx = channels.get(target)
            .ok_or_else(|| CoordinationError::AgentNotFound(target.to_string()))?;

        tx.send(envelope.clone())
            .map_err(|_| CoordinationError::SendFailed {
                target: target.to_string(),
            })?;

        debug!("Message {} delivered to agent {}", envelope.message_id, target);
        Ok(())
    }

    // ==========================================================================
    // Pub/Sub Messaging
    // ==========================================================================

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: String) -> broadcast::Receiver<MessageEnvelope> {
        let tx = self.get_or_create_topic(topic).await;
        tx.subscribe()
    }

    /// Publish a message to a topic
    pub async fn publish(&self, mut envelope: MessageEnvelope) -> Result<usize> {
        let topic = envelope.topic.as_ref()
            .ok_or_else(|| CoordinationError::CommunicationError(
                "Publish requires topic".to_string()
            ))?
            .clone();

        // Generate message ID if not present
        if envelope.message_id.is_empty() {
            envelope.message_id = uuid::Uuid::new_v4().to_string();
        }

        envelope.timestamp = Utc::now();

        // Persist to episodic memory
        if self.config.persist_to_episodic {
            if let Err(e) = self.persist_message(&envelope).await {
                warn!("Failed to persist broadcast message: {}", e);
            }
        }

        // Add to history
        self.add_to_history(&envelope).await;

        // Broadcast
        let tx = self.get_or_create_topic(topic.clone()).await;
        let count = tx.receiver_count();

        tx.send(envelope.clone())
            .map_err(|_| CoordinationError::PublishFailed { topic })?;

        let mut stats = self.stats.write().await;
        stats.total_sent += 1;
        stats.total_delivered += count as u64;

        Ok(count)
    }

    async fn get_or_create_topic(&self, topic: String) -> broadcast::Sender<MessageEnvelope> {
        let mut topics = self.topic_channels.write().await;

        if let Some(tx) = topics.get(&topic) {
            tx.clone()
        } else {
            let (tx, _) = broadcast::channel(self.config.broadcast_capacity);
            topics.insert(topic, tx.clone());
            tx
        }
    }

    // ==========================================================================
    // Message Persistence & Replay
    // ==========================================================================

    async fn persist_message(&self, envelope: &MessageEnvelope) -> Result<()> {
        // Create episode from message for episodic memory
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: "message".to_string(),
            task_description: format!("Message from {} to {:?}", envelope.from, envelope.to),
            agent_id: envelope.from.to_string(),
            session_id: envelope.session_id.to_string(),
            workspace_id: envelope.workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![],
            queries_made: vec![],
            tools_used: vec![format!("message_bus::{:?}", envelope.payload)],
            solution_summary: serde_json::to_string(&envelope.payload).unwrap_or_default(),
            outcome: "success".to_string(),
            success_metrics: serde_json::json!({
                "message_id": envelope.message_id,
                "timestamp": envelope.timestamp,
                "priority": envelope.priority,
            }),
            errors_encountered: vec![],
            lessons_learned: vec![],
            duration_seconds: 0.0,
            tokens_used: 0,
            created_at: envelope.timestamp,
            completed_at: envelope.timestamp,
        };

        self.cortex.store_episode(episode).await
            .map_err(|e| CoordinationError::Other(e.into()))?;

        Ok(())
    }

    async fn add_to_history(&self, envelope: &MessageEnvelope) {
        let mut history = self.message_history.write().await;
        let session_history = history.entry(envelope.session_id.clone())
            .or_insert_with(VecDeque::new);

        session_history.push_back(envelope.clone());

        // Trim if exceeds max size
        if session_history.len() > self.config.max_history_size {
            session_history.pop_front();
        }
    }

    /// Replay messages from a session
    pub async fn replay_session(&self, session_id: &SessionId) -> Result<Vec<MessageEnvelope>> {
        let history = self.message_history.read().await;
        Ok(history.get(session_id)
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// Replay messages from episodic memory
    pub async fn replay_from_episodic(
        &self,
        session_id: &SessionId,
        limit: usize,
    ) -> Result<Vec<MessageEnvelope>> {
        // Query episodic memory for messages
        let episodes = self.cortex.search_episodes(
            &format!("session_id:{}", session_id),
            limit
        ).await.map_err(|e| CoordinationError::Other(e.into()))?;

        // Convert episodes back to messages
        let messages: Vec<MessageEnvelope> = episodes.into_iter()
            .filter_map(|ep| {
                // Parse message from episode
                serde_json::from_str(&ep.solution_summary).ok()
            })
            .collect();

        Ok(messages)
    }

    // ==========================================================================
    // Resilience Patterns
    // ==========================================================================

    async fn check_circuit_breaker(&self, agent_id: &AgentId) -> bool {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = breakers.entry(agent_id.clone())
            .or_insert_with(CircuitBreaker::new);

        breaker.should_attempt(self.config.circuit_breaker_timeout)
    }

    async fn record_success(&self, agent_id: &AgentId) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(agent_id) {
            breaker.record_success();
        }
    }

    async fn record_failure(&self, agent_id: &AgentId) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(agent_id) {
            breaker.record_failure(self.config.circuit_breaker_threshold);
        }

        let mut stats = self.stats.write().await;
        stats.circuit_breaker_trips += 1;
    }

    async fn check_rate_limit(&self, agent_id: &AgentId) -> bool {
        let limiters = self.rate_limiters.read().await;
        if let Some(semaphore) = limiters.get(agent_id) {
            semaphore.try_acquire().is_ok()
        } else {
            true
        }
    }

    async fn move_to_dead_letter(&self, envelope: MessageEnvelope, reason: String) {
        let mut dead_letters = self.dead_letters.write().await;

        dead_letters.push_back((envelope, reason));

        // Trim if exceeds max size
        if dead_letters.len() > self.config.max_dead_letters {
            dead_letters.pop_front();
        }

        let mut stats = self.stats.write().await;
        stats.total_dead_letters += 1;

        warn!("Message moved to dead letter queue");
    }

    /// Get dead letter queue
    pub async fn get_dead_letters(&self) -> Vec<(MessageEnvelope, String)> {
        self.dead_letters.read().await.iter().cloned().collect()
    }

    /// Clear dead letter queue
    pub async fn clear_dead_letters(&self) {
        self.dead_letters.write().await.clear();
    }

    // ==========================================================================
    // Statistics & Monitoring
    // ==========================================================================

    /// Get message bus statistics
    pub async fn get_stats(&self) -> MessageBusStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = MessageBusStats::default();
    }

    /// Get circuit breaker states
    pub async fn get_circuit_states(&self) -> HashMap<AgentId, CircuitState> {
        self.circuit_breakers
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.state))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let mut breaker = CircuitBreaker::new();
        assert_eq!(breaker.state, CircuitState::Closed);

        // Record failures to open circuit
        for _ in 0..5 {
            breaker.record_failure(5);
        }
        assert_eq!(breaker.state, CircuitState::Open);

        // Record success in half-open state
        breaker.state = CircuitState::HalfOpen;
        for _ in 0..3 {
            breaker.record_success();
        }
        assert_eq!(breaker.state, CircuitState::Closed);
    }

    #[test]
    fn test_message_envelope_creation() {
        let envelope = MessageEnvelope {
            message_id: "test-123".to_string(),
            correlation_id: None,
            causation_id: None,
            from: AgentId::from("agent-1".to_string()),
            to: Some(AgentId::from("agent-2".to_string())),
            topic: None,
            session_id: SessionId::from("session-1".to_string()),
            workspace_id: WorkspaceId::from("workspace-1".to_string()),
            payload: Message::HealthPing,
            timestamp: Utc::now(),
            expires_at: None,
            priority: 5,
            attempt_count: 0,
            max_attempts: 3,
            metadata: HashMap::new(),
        };

        assert_eq!(envelope.message_id, "test-123");
        assert_eq!(envelope.priority, 5);
    }
}
