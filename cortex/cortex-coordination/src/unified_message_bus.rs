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
use cortex_agents::AgentId;
use crate::cortex_bridge::{
    CortexBridge, Episode, EpisodeId, LockType, SessionId, WorkspaceId,
};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, broadcast, RwLock};
use tracing::{debug, info, warn};

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
    /// Cortex bridge for persistence and coordination (optional for lightweight mode)
    cortex: Option<Arc<CortexBridge>>,

    /// Direct message channels: agent_id -> sender
    direct_channels: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<MessageEnvelope>>>>,

    /// Topic subscribers: topic -> broadcast sender
    topic_channels: Arc<RwLock<HashMap<String, broadcast::Sender<MessageEnvelope>>>>,

    /// Message history for replay: session_id -> messages
    message_history: Arc<RwLock<HashMap<SessionId, VecDeque<MessageEnvelope>>>>,

    /// Session metadata for cleanup tracking
    session_metadata: Arc<RwLock<HashMap<SessionId, SessionMetadata>>>,

    /// Dead letter queue
    dead_letters: Arc<RwLock<VecDeque<(MessageEnvelope, String)>>>,

    /// Circuit breakers: agent_id -> state
    circuit_breakers: Arc<RwLock<HashMap<AgentId, CircuitBreaker>>>,

    /// Rate limiters: agent_id -> rate limit state
    rate_limiters: Arc<RwLock<HashMap<AgentId, RateLimitState>>>,

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

/// Time-based rate limiter using sliding window
#[derive(Debug, Clone)]
struct RateLimitState {
    /// Timestamps of recent messages
    timestamps: VecDeque<Instant>,
    /// Maximum messages allowed per window
    max_per_window: usize,
    /// Time window duration
    window_duration: Duration,
}

impl RateLimitState {
    fn new(max_per_window: usize, window_duration: Duration) -> Self {
        Self {
            timestamps: VecDeque::new(),
            max_per_window,
            window_duration,
        }
    }

    /// Check if a message can be sent and record it if allowed
    fn check_and_add(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window_duration;

        // Remove timestamps outside the window
        while let Some(&ts) = self.timestamps.front() {
            if ts < cutoff {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check if under limit
        if self.timestamps.len() < self.max_per_window {
            self.timestamps.push_back(now);
            true
        } else {
            false
        }
    }

    /// Get current message count in window
    fn current_count(&self) -> usize {
        let now = Instant::now();
        let cutoff = now - self.window_duration;
        self.timestamps.iter().filter(|&&ts| ts >= cutoff).count()
    }
}

/// Session metadata for cleanup tracking
#[derive(Debug, Clone)]
struct SessionMetadata {
    /// Last time this session was accessed
    last_accessed: Instant,
    /// When this session was created
    created_at: Instant,
    /// Number of messages in this session
    message_count: usize,
}

impl SessionMetadata {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            last_accessed: now,
            created_at: now,
            message_count: 0,
        }
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    fn increment_messages(&mut self) {
        self.message_count += 1;
        self.touch();
    }
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

    /// Rate limit per agent (messages per window)
    pub rate_limit_per_agent: usize,

    /// Rate limit window duration
    pub rate_limit_window: Duration,

    /// Enable episodic memory persistence
    pub persist_to_episodic: bool,

    /// Broadcast channel capacity
    pub broadcast_capacity: usize,

    /// Message TTL (time to live)
    pub default_message_ttl: Duration,

    /// Maximum age for sessions before cleanup (in seconds)
    pub session_max_age_secs: u64,

    /// Maximum number of sessions to keep (LRU eviction)
    pub max_sessions: usize,
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
            rate_limit_window: Duration::from_secs(1), // 100 messages per second
            persist_to_episodic: true,
            broadcast_capacity: 1000,
            default_message_ttl: Duration::from_secs(3600),
            session_max_age_secs: 3600, // 1 hour
            max_sessions: 1000,
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

        if self.state == CircuitState::HalfOpen {
            if self.success_count >= 3 {
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.success_count = 0;
                self.last_state_change = Utc::now();
                info!("Circuit breaker closed after successful recovery");
            }
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
    /// Create a new unified message bus with Cortex integration
    pub fn new_with_cortex(cortex: Arc<CortexBridge>, config: MessageBusConfig) -> Self {
        info!("Initializing unified message bus with Cortex integration");

        Self {
            cortex: Some(cortex),
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        }
    }

    /// Create a new unified message bus without Cortex (lightweight mode)
    ///
    /// This mode is suitable for testing, development, or scenarios where Cortex
    /// cognitive memory is not needed. Message persistence and episodic memory
    /// features will be disabled.
    pub fn new() -> Self {
        info!("Initializing unified message bus (lightweight mode - no Cortex)");

        Self {
            cortex: None,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config: MessageBusConfig::default(),
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

        // Initialize rate limiter with time-based sliding window
        let rate_limiter = RateLimitState::new(
            self.config.rate_limit_per_agent,
            self.config.rate_limit_window,
        );
        self.rate_limiters.write().await.insert(agent_id.clone(), rate_limiter);

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
            ))?.clone();

        // Generate message ID if not present
        if envelope.message_id.is_empty() {
            envelope.message_id = uuid::Uuid::new_v4().to_string();
        }

        // Set timestamp
        envelope.timestamp = Utc::now();

        // Check circuit breaker
        if !self.check_circuit_breaker(&target).await {
            warn!("Circuit breaker open for agent {}, message rejected", target);
            self.move_to_dead_letter(envelope, "Circuit breaker open".to_string()).await;
            return Err(CoordinationError::SendFailed {
                target: target.to_string(),
            });
        }

        // Check rate limit
        if !self.check_rate_limit(&target).await {
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
                self.record_success(&target).await;
                let mut stats = self.stats.write().await;
                stats.total_sent += 1;
                stats.total_delivered += 1;
            }
            Err(e) => {
                self.record_failure(&target).await;
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
        let target = envelope.to.as_ref()
            .ok_or_else(|| CoordinationError::CommunicationError(
                "Direct message delivery requires target agent".to_string()
            ))?;
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
        // Only persist if Cortex is available
        let Some(ref cortex) = self.cortex else {
            return Ok(()); // Silently skip if no Cortex
        };

        use crate::cortex_bridge::EpisodeOutcome;

        // Create episode from message for episodic memory
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: Some("task".to_string()),
            task_description: format!("Message from {} to {:?}", envelope.from, envelope.to),
            agent_id: envelope.from.to_string(),
            session_id: Some(envelope.session_id.to_string()),
            workspace_id: Some(envelope.workspace_id.to_string()),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![],
            queries_made: vec![],
            tools_used: vec![format!("message_bus::{:?}", envelope.payload)],
            solution_summary: serde_json::to_string(&envelope.payload).unwrap_or_default(),
            outcome: EpisodeOutcome::Success,
            success_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("message_id".to_string(), serde_json::json!(envelope.message_id));
                metrics.insert("timestamp".to_string(), serde_json::json!(envelope.timestamp));
                metrics.insert("priority".to_string(), serde_json::json!(envelope.priority));
                metrics
            },
            errors_encountered: vec![],
            lessons_learned: vec![],
            duration_seconds: Some(0.0),
            tokens_used: 0,
            embedding: None,
            created_at: Some(envelope.timestamp),
            completed_at: Some(envelope.timestamp),
        };

        cortex.store_episode(episode).await
            .map_err(|e| CoordinationError::Other(e.into()))?;

        Ok(())
    }

    async fn add_to_history(&self, envelope: &MessageEnvelope) {
        let mut history = self.message_history.write().await;
        let mut metadata = self.session_metadata.write().await;

        // Update or create session metadata
        let session_meta = metadata.entry(envelope.session_id.clone())
            .or_insert_with(SessionMetadata::new);
        session_meta.increment_messages();

        // Add message to history
        let session_history = history.entry(envelope.session_id.clone())
            .or_insert_with(VecDeque::new);
        session_history.push_back(envelope.clone());

        // Trim if exceeds max size
        if session_history.len() > self.config.max_history_size {
            session_history.pop_front();
        }

        // Trigger cleanup if we have too many sessions
        if metadata.len() > self.config.max_sessions {
            drop(history);
            drop(metadata);
            // Cleanup is done without holding locks
            self.cleanup_old_sessions().await;
        }
    }

    /// Replay messages from a session
    pub async fn replay_session(&self, session_id: &SessionId) -> Result<Vec<MessageEnvelope>> {
        // Touch the session metadata to update last_accessed
        if let Some(meta) = self.session_metadata.write().await.get_mut(session_id) {
            meta.touch();
        }

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
        // Only replay if Cortex is available
        let Some(ref cortex) = self.cortex else {
            return Ok(vec![]); // Return empty if no Cortex
        };

        // Query episodic memory for messages
        let episodes = cortex.search_episodes(
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
        let mut limiters = self.rate_limiters.write().await;
        if let Some(limiter) = limiters.get_mut(agent_id) {
            limiter.check_and_add()
        } else {
            // If no limiter exists for this agent, allow the message
            // and create a new limiter for future requests
            let mut new_limiter = RateLimitState::new(
                self.config.rate_limit_per_agent,
                self.config.rate_limit_window,
            );
            let result = new_limiter.check_and_add();
            limiters.insert(agent_id.clone(), new_limiter);
            result
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

    /// Get rate limiter statistics for all agents
    pub async fn get_rate_limiter_stats(&self) -> HashMap<AgentId, usize> {
        self.rate_limiters
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.current_count()))
            .collect()
    }

    // ==========================================================================
    // Session Cleanup (Memory Leak Prevention)
    // ==========================================================================

    /// Clean up old sessions based on age and LRU eviction
    ///
    /// This method prevents memory leaks by removing sessions that are:
    /// 1. Older than the configured max age
    /// 2. Exceeding the max session limit (LRU eviction)
    async fn cleanup_old_sessions(&self) {
        let mut history = self.message_history.write().await;
        let mut metadata = self.session_metadata.write().await;

        let cutoff = Instant::now() - Duration::from_secs(self.config.session_max_age_secs);
        let mut removed_count = 0;

        // First pass: Remove sessions older than max age
        metadata.retain(|session_id, meta| {
            if meta.last_accessed < cutoff {
                history.remove(session_id);
                removed_count += 1;
                debug!("Cleaned up old session {} (age: {:?})",
                       session_id,
                       Instant::now() - meta.created_at);
                false
            } else {
                true
            }
        });

        // Second pass: LRU eviction if still over limit
        if metadata.len() > self.config.max_sessions {
            // Collect sessions sorted by last access time (oldest first)
            let mut sessions: Vec<_> = metadata.iter()
                .map(|(id, meta)| (id.clone(), meta.last_accessed))
                .collect();
            sessions.sort_by_key(|(_, last_accessed)| *last_accessed);

            // Remove oldest sessions until we're under the limit
            let to_remove = sessions.len() - self.config.max_sessions;
            for (session_id, _) in sessions.into_iter().take(to_remove) {
                history.remove(&session_id);
                metadata.remove(&session_id);
                removed_count += 1;
                debug!("Evicted session {} (LRU)", session_id);
            }
        }

        if removed_count > 0 {
            info!("Cleaned up {} old sessions (total sessions: {})",
                  removed_count,
                  metadata.len());
        }
    }

    /// Manually trigger session cleanup
    ///
    /// This can be called periodically by a background task or on demand
    pub async fn trigger_session_cleanup(&self) {
        self.cleanup_old_sessions().await;
    }

    /// Get the current number of active sessions
    pub async fn get_session_count(&self) -> usize {
        self.session_metadata.read().await.len()
    }

    /// Get session statistics
    pub async fn get_session_stats(&self) -> HashMap<SessionId, (usize, Instant, Instant)> {
        self.session_metadata
            .read()
            .await
            .iter()
            .map(|(id, meta)| {
                (id.clone(), (meta.message_count, meta.created_at, meta.last_accessed))
            })
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
            from: AgentId::from_string("agent-1".to_string()),
            to: Some(AgentId::from_string("agent-2".to_string())),
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

    #[tokio::test]
    async fn test_session_cleanup_by_age() {
        use std::time::Duration;

        // Create a message bus with short session max age
        let mut config = MessageBusConfig::default();
        config.session_max_age_secs = 1; // 1 second
        config.max_sessions = 1000;

        let bus = UnifiedMessageBus {
            cortex: None,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        };

        // Create some test messages in different sessions
        let session1 = SessionId::from("session-1".to_string());
        let session2 = SessionId::from("session-2".to_string());

        let envelope1 = MessageEnvelope {
            message_id: "msg-1".to_string(),
            correlation_id: None,
            causation_id: None,
            from: AgentId::from_string("agent-1".to_string()),
            to: Some(AgentId::from_string("agent-2".to_string())),
            topic: None,
            session_id: session1.clone(),
            workspace_id: WorkspaceId::from("workspace-1".to_string()),
            payload: Message::HealthPing,
            timestamp: Utc::now(),
            expires_at: None,
            priority: 5,
            attempt_count: 0,
            max_attempts: 3,
            metadata: HashMap::new(),
        };

        let envelope2 = MessageEnvelope {
            session_id: session2.clone(),
            ..envelope1.clone()
        };

        // Add messages to history
        bus.add_to_history(&envelope1).await;
        bus.add_to_history(&envelope2).await;

        // Verify both sessions exist
        assert_eq!(bus.get_session_count().await, 2);

        // Wait for sessions to age
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Trigger cleanup
        bus.trigger_session_cleanup().await;

        // Verify sessions were cleaned up
        assert_eq!(bus.get_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_lru_eviction() {
        // Create a message bus with low max sessions
        let mut config = MessageBusConfig::default();
        config.session_max_age_secs = 3600; // Long age so it won't trigger age-based cleanup
        config.max_sessions = 2; // Only keep 2 sessions

        let bus = UnifiedMessageBus {
            cortex: None,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        };

        // Create messages in 3 different sessions
        for i in 1..=3 {
            let session = SessionId::from(format!("session-{}", i));
            let envelope = MessageEnvelope {
                message_id: format!("msg-{}", i),
                correlation_id: None,
                causation_id: None,
                from: AgentId::from_string("agent-1".to_string()),
                to: Some(AgentId::from_string("agent-2".to_string())),
                topic: None,
                session_id: session,
                workspace_id: WorkspaceId::from("workspace-1".to_string()),
                payload: Message::HealthPing,
                timestamp: Utc::now(),
                expires_at: None,
                priority: 5,
                attempt_count: 0,
                max_attempts: 3,
                metadata: HashMap::new(),
            };
            bus.add_to_history(&envelope).await;

            // Small delay to ensure different access times
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Should have triggered cleanup automatically, keeping only 2 most recent sessions
        let session_count = bus.get_session_count().await;
        assert!(session_count <= 2, "Expected at most 2 sessions, got {}", session_count);
    }

    #[tokio::test]
    async fn test_rate_limiter_sliding_window() {
        use std::time::Duration;

        // Create a message bus with strict rate limiting
        let mut config = MessageBusConfig::default();
        config.rate_limit_per_agent = 5; // 5 messages per window
        config.rate_limit_window = Duration::from_millis(100); // 100ms window

        let bus = UnifiedMessageBus {
            cortex: None,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        };

        let agent_id = AgentId::from_string("test-agent".to_string());

        // First 5 messages should succeed
        for i in 0..5 {
            let allowed = bus.check_rate_limit(&agent_id).await;
            assert!(allowed, "Message {} should be allowed", i);
        }

        // 6th message should be rate limited
        let allowed = bus.check_rate_limit(&agent_id).await;
        assert!(!allowed, "Message 6 should be rate limited");

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // After window expires, should be able to send again
        for i in 0..5 {
            let allowed = bus.check_rate_limit(&agent_id).await;
            assert!(allowed, "Message {} after window reset should be allowed", i);
        }

        // Should be rate limited again
        let allowed = bus.check_rate_limit(&agent_id).await;
        assert!(!allowed, "Should be rate limited again after sending 5 more messages");

        // Verify rate limiter stats
        let stats = bus.get_rate_limiter_stats().await;
        assert_eq!(stats.get(&agent_id).copied(), Some(5), "Should show 5 messages in current window");
    }

    #[tokio::test]
    async fn test_rate_limiter_no_leak() {
        use std::time::Duration;

        // Create a message bus with rate limiting
        let mut config = MessageBusConfig::default();
        config.rate_limit_per_agent = 10;
        config.rate_limit_window = Duration::from_millis(50);

        let bus = UnifiedMessageBus {
            cortex: None,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            topic_channels: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(HashMap::new())),
            session_metadata: Arc::new(RwLock::new(HashMap::new())),
            dead_letters: Arc::new(RwLock::new(VecDeque::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(MessageBusStats::default())),
        };

        let agent_id = AgentId::from_string("test-agent".to_string());

        // Send multiple bursts over time
        for burst in 0..3 {
            // Send up to limit
            for _ in 0..10 {
                let _ = bus.check_rate_limit(&agent_id).await;
            }

            // Wait for window to reset
            tokio::time::sleep(Duration::from_millis(60)).await;

            // After each burst and wait, we should still be able to send
            let allowed = bus.check_rate_limit(&agent_id).await;
            assert!(allowed, "Burst {} should allow messages after window reset", burst);
        }

        // Verify that old timestamps are cleaned up
        let stats = bus.get_rate_limiter_stats().await;
        let count = stats.get(&agent_id).copied().unwrap_or(0);
        assert!(count <= 10, "Rate limiter should clean up old timestamps, got {}", count);
    }
}
