# Axon: Coordination Patterns

## Overview

Coordination Patterns define the ways agents interact in the Axon system. This specification describes a communication architecture based on message bus, channel-based communication, and pub/sub system for efficient coordination between agents.

## Coordination Layer Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Coordination Layer                         │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌────────────────────────────────────────────────────────┐  │
│  │               Message Bus (Core)                       │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────────┐    │  │
│  │  │ Channel  │  │  Topic   │  │  Subscription   │    │  │
│  │  │ Registry │  │ Registry │  │    Manager      │    │  │
│  │  └──────────┘  └──────────┘  └─────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│  ┌─────────────────────────┼──────────────────────────────┐  │
│  │                         │                              │  │
│  │  ┌──────────────┐  ┌───▼──────────┐  ┌────────────┐  │  │
│  │  │   Direct     │  │   Pub/Sub    │  │  Request/  │  │  │
│  │  │   Messaging  │  │    System    │  │  Response  │  │  │
│  │  └──────────────┘  └──────────────┘  └────────────┘  │  │
│  │                                                        │  │
│  │  Communication Patterns                               │  │
│  └────────────────────────────────────────────────────────┘  │
│                            │                                  │
│  ┌─────────────────────────▼──────────────────────────────┐  │
│  │          Agent Discovery & Topology                    │  │
│  │                                                        │  │
│  │  ┌──────────┐  ┌──────────┐  ┌─────────────────┐    │  │
│  │  │Discovery │  │Topology  │  │   Cortex Agent  │    │  │
│  │  │ Service  │  │ Manager  │  │     Registry    │    │  │
│  │  └──────────┘  └──────────┘  └─────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

## Message Bus Architecture

### Core Message Bus

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, broadcast};

/// Message Bus - central communication system between agents
pub struct MessageBus {
    /// Direct channels for point-to-point communication
    channels: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<Message>>>>,

    /// Broadcast channels for pub/sub
    topics: Arc<RwLock<HashMap<Topic, broadcast::Sender<Message>>>>,

    /// Agent subscriptions to topics
    subscriptions: Arc<RwLock<HashMap<AgentId, Vec<Topic>>>>,

    /// Statistics and metrics
    metrics: Arc<BusMetrics>,

    /// Configuration
    config: BusConfig,
}

#[derive(Debug, Clone)]
pub struct BusConfig {
    pub channel_buffer_size: usize,
    pub topic_buffer_size: usize,
    pub message_ttl: Duration,
    pub enable_message_logging: bool,
    pub max_retry_attempts: u32,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            channel_buffer_size: 1000,
            topic_buffer_size: 10000,
            message_ttl: Duration::from_secs(300),
            enable_message_logging: true,
            max_retry_attempts: 3,
        }
    }
}

impl MessageBus {
    pub fn new(config: BusConfig) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            topics: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(BusMetrics::new()),
            config,
        }
    }

    /// Registers agent in message bus
    pub async fn register_agent(&self, agent_id: AgentId) -> mpsc::UnboundedReceiver<Message> {
        let (tx, rx) = mpsc::unbounded_channel();

        self.channels.write().await.insert(agent_id.clone(), tx);
        self.subscriptions.write().await.insert(agent_id.clone(), Vec::new());

        info!("Agent {} registered with message bus", agent_id);
        self.metrics.agents_registered.fetch_add(1, Ordering::Relaxed);

        rx
    }

    /// Sends message to specific agent (point-to-point)
    pub async fn send(&self, target: AgentId, message: Message) -> Result<()> {
        let channels = self.channels.read().await;

        let tx = channels
            .get(&target)
            .ok_or_else(|| CoordinationError::AgentNotFound {
                agent_id: target.clone(),
            })?;

        tx.send(message.clone()).map_err(|_| CoordinationError::SendFailed {
            target: target.clone(),
        })?;

        self.metrics.messages_sent.fetch_add(1, Ordering::Relaxed);

        if self.config.enable_message_logging {
            debug!("Sent message to {}: {:?}", target, message);
        }

        Ok(())
    }

    /// Sends message with delivery confirmation
    pub async fn send_with_ack(&self, target: AgentId, message: Message) -> Result<Ack> {
        let message_id = MessageId::new();
        let (ack_tx, mut ack_rx) = mpsc::channel(1);

        // Add callback for confirmation
        let message_with_ack = Message::WithAck {
            id: message_id.clone(),
            inner: Box::new(message),
            ack_channel: ack_tx,
        };

        self.send(target.clone(), message_with_ack).await?;

        // Wait for confirmation with timeout
        tokio::time::timeout(Duration::from_secs(30), ack_rx.recv())
            .await
            .map_err(|_| CoordinationError::AckTimeout {
                message_id: message_id.clone(),
                target: target.clone(),
            })?
            .ok_or_else(|| CoordinationError::AckFailed {
                message_id,
                target,
            })
    }

    /// Creates or gets broadcast channel for topic
    pub async fn get_or_create_topic(&self, topic: Topic) -> broadcast::Sender<Message> {
        let mut topics = self.topics.write().await;

        if let Some(tx) = topics.get(&topic) {
            tx.clone()
        } else {
            let (tx, _) = broadcast::channel(self.config.topic_buffer_size);
            topics.insert(topic.clone(), tx.clone());
            info!("Created topic: {}", topic);
            tx
        }
    }

    /// Publishes message to topic (pub/sub)
    pub async fn publish(&self, topic: Topic, message: Message) -> Result<usize> {
        let tx = self.get_or_create_topic(topic.clone()).await;

        let subscriber_count = tx.receiver_count();

        if subscriber_count == 0 {
            warn!("Publishing to topic {} with no subscribers", topic);
        }

        tx.send(message.clone()).map_err(|_| CoordinationError::PublishFailed {
            topic: topic.clone(),
        })?;

        self.metrics.messages_published.fetch_add(1, Ordering::Relaxed);

        if self.config.enable_message_logging {
            debug!("Published to topic {}: {:?}", topic, message);
        }

        Ok(subscriber_count)
    }

    /// Subscribes agent to topic
    pub async fn subscribe(
        &self,
        agent_id: AgentId,
        topic: Topic,
    ) -> broadcast::Receiver<Message> {
        let tx = self.get_or_create_topic(topic.clone()).await;
        let rx = tx.subscribe();

        // Register subscription
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(agent_id.clone())
            .or_insert_with(Vec::new)
            .push(topic.clone());

        info!("Agent {} subscribed to topic {}", agent_id, topic);
        self.metrics.subscriptions_created.fetch_add(1, Ordering::Relaxed);

        rx
    }

    /// Unsubscribes agent from topic
    pub async fn unsubscribe(&self, agent_id: AgentId, topic: Topic) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;

        if let Some(topics) = subscriptions.get_mut(&agent_id) {
            topics.retain(|t| *t != topic);
            info!("Agent {} unsubscribed from topic {}", agent_id, topic);
            Ok(())
        } else {
            Err(CoordinationError::AgentNotFound { agent_id })
        }
    }
}

/// Message types in the system
#[derive(Debug, Clone)]
pub enum Message {
    /// Task assignment to agent
    TaskAssignment {
        task: Task,
        agent_id: AgentId,
        session_id: SessionId,
        context: Vec<Episode>,
    },

    /// Task execution progress
    TaskProgress {
        task_id: TaskId,
        agent_id: AgentId,
        progress: f32,
        message: String,
    },

    /// Task completion
    TaskComplete {
        task_id: TaskId,
        agent_id: AgentId,
        result: TaskOutput,
    },

    /// Task execution error
    TaskFailed {
        task_id: TaskId,
        agent_id: AgentId,
        error: String,
    },

    /// Help request from another agent
    HelpRequest {
        from: AgentId,
        task_id: TaskId,
        context: String,
        urgency: Urgency,
    },

    /// Response to help request
    HelpResponse {
        to: AgentId,
        task_id: TaskId,
        suggestions: Vec<Suggestion>,
    },

    /// Proposal for consensus (Sangha voting)
    ConsensusProposal {
        proposal_id: ProposalId,
        proposer: AgentId,
        proposal: Proposal,
        votes_required: u32,
        deadline: DateTime<Utc>,
    },

    /// Vote on proposal
    ConsensusVote {
        proposal_id: ProposalId,
        voter: AgentId,
        vote: Vote,
        justification: String,
    },

    /// Consensus result
    ConsensusResult {
        proposal_id: ProposalId,
        outcome: Outcome,
        votes_for: u32,
        votes_against: u32,
    },

    /// System event
    SystemEvent {
        event_type: EventType,
        source: AgentId,
        data: serde_json::Value,
    },

    /// Heartbeat for availability check
    Heartbeat {
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
        status: AgentStatus,
    },

    /// Message with confirmation
    WithAck {
        id: MessageId,
        inner: Box<Message>,
        ack_channel: mpsc::Sender<Ack>,
    },
}

#[derive(Debug, Clone)]
pub struct Ack {
    pub message_id: MessageId,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(String);

impl Topic {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    // Standard system topics
    pub fn task_updates() -> Self {
        Self("task_updates".to_string())
    }

    pub fn agent_status() -> Self {
        Self("agent_status".to_string())
    }

    pub fn consensus() -> Self {
        Self("consensus".to_string())
    }

    pub fn system_events() -> Self {
        Self("system_events".to_string())
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

## Channel-Based Communication

### Typed Channels for different message types

```rust
/// Channel Manager manages typed channels
pub struct ChannelManager {
    bus: Arc<MessageBus>,
    typed_channels: Arc<RwLock<HashMap<ChannelType, TypedChannel>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChannelType {
    TaskManagement,
    Consensus,
    Coordination,
    Monitoring,
    Emergency,
}

pub struct TypedChannel {
    channel_type: ChannelType,
    priority: Priority,
    filter: Option<MessageFilter>,
}

impl ChannelManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            bus,
            typed_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates typed channel for tasks
    pub async fn create_task_channel(&self) -> TaskChannel {
        TaskChannel {
            bus: self.bus.clone(),
            topic: Topic::new("tasks"),
        }
    }

    /// Creates typed channel for consensus
    pub async fn create_consensus_channel(&self) -> ConsensusChannel {
        ConsensusChannel {
            bus: self.bus.clone(),
            topic: Topic::consensus(),
        }
    }
}

/// Specialized channel for tasks
pub struct TaskChannel {
    bus: Arc<MessageBus>,
    topic: Topic,
}

impl TaskChannel {
    /// Sends task assignment
    pub async fn assign_task(
        &self,
        agent_id: AgentId,
        task: Task,
        session_id: SessionId,
        context: Vec<Episode>,
    ) -> Result<()> {
        let message = Message::TaskAssignment {
            task,
            agent_id: agent_id.clone(),
            session_id,
            context,
        };

        self.bus.send(agent_id, message).await
    }

    /// Sends progress update
    pub async fn report_progress(
        &self,
        task_id: TaskId,
        agent_id: AgentId,
        progress: f32,
        message: String,
    ) -> Result<()> {
        let msg = Message::TaskProgress {
            task_id,
            agent_id,
            progress,
            message,
        };

        self.bus.publish(self.topic.clone(), msg).await?;
        Ok(())
    }

    /// Subscribes to task updates
    pub async fn subscribe_updates(&self, agent_id: AgentId) -> broadcast::Receiver<Message> {
        self.bus.subscribe(agent_id, self.topic.clone()).await
    }
}

/// Specialized channel for consensus
pub struct ConsensusChannel {
    bus: Arc<MessageBus>,
    topic: Topic,
}

impl ConsensusChannel {
    /// Creates new proposal for voting
    pub async fn propose(
        &self,
        proposer: AgentId,
        proposal: Proposal,
        votes_required: u32,
        deadline: DateTime<Utc>,
    ) -> Result<ProposalId> {
        let proposal_id = ProposalId::new();

        let message = Message::ConsensusProposal {
            proposal_id: proposal_id.clone(),
            proposer,
            proposal,
            votes_required,
            deadline,
        };

        self.bus.publish(self.topic.clone(), message).await?;

        Ok(proposal_id)
    }

    /// Sends vote
    pub async fn vote(
        &self,
        proposal_id: ProposalId,
        voter: AgentId,
        vote: Vote,
        justification: String,
    ) -> Result<()> {
        let message = Message::ConsensusVote {
            proposal_id,
            voter,
            vote,
            justification,
        };

        self.bus.publish(self.topic.clone(), message).await?;
        Ok(())
    }

    /// Subscribes to consensus events
    pub async fn subscribe(&self, agent_id: AgentId) -> broadcast::Receiver<Message> {
        self.bus.subscribe(agent_id, self.topic.clone()).await
    }
}
```

## Pub/Sub System

### Pub/Sub for event-driven coordination

```rust
/// Pub/Sub Manager for managing subscriptions
pub struct PubSubManager {
    bus: Arc<MessageBus>,
    event_handlers: Arc<RwLock<HashMap<AgentId, Vec<EventHandler>>>>,
}

pub struct EventHandler {
    pub topic: Topic,
    pub filter: Option<MessageFilter>,
    pub handler: Arc<dyn Fn(Message) -> BoxFuture<'static, ()> + Send + Sync>,
}

pub struct MessageFilter {
    pub event_types: Vec<EventType>,
    pub source_agents: Vec<AgentId>,
    pub priority_threshold: Option<Priority>,
}

impl PubSubManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            bus,
            event_handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers event handler
    pub async fn register_handler(
        &self,
        agent_id: AgentId,
        topic: Topic,
        filter: Option<MessageFilter>,
        handler: impl Fn(Message) -> BoxFuture<'static, ()> + Send + Sync + 'static,
    ) -> Result<()> {
        let event_handler = EventHandler {
            topic: topic.clone(),
            filter,
            handler: Arc::new(handler),
        };

        self.event_handlers
            .write()
            .await
            .entry(agent_id.clone())
            .or_insert_with(Vec::new)
            .push(event_handler);

        // Subscribe to topic
        let mut receiver = self.bus.subscribe(agent_id.clone(), topic.clone()).await;

        // Start background task for message processing
        let handlers = self.event_handlers.clone();
        let agent_id_clone = agent_id.clone();

        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                let handlers = handlers.read().await;

                if let Some(agent_handlers) = handlers.get(&agent_id_clone) {
                    for handler in agent_handlers {
                        if handler.topic == topic {
                            // Check filter
                            if let Some(filter) = &handler.filter {
                                if !Self::message_matches_filter(&message, filter) {
                                    continue;
                                }
                            }

                            // Call handler
                            (handler.handler)(message.clone()).await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Checks if message matches filter
    fn message_matches_filter(message: &Message, filter: &MessageFilter) -> bool {
        // Check event type
        if !filter.event_types.is_empty() {
            if let Message::SystemEvent { event_type, .. } = message {
                if !filter.event_types.contains(event_type) {
                    return false;
                }
            }
        }

        // Check source
        if !filter.source_agents.is_empty() {
            let source = match message {
                Message::SystemEvent { source, .. } => Some(source),
                Message::TaskProgress { agent_id, .. } => Some(agent_id),
                Message::TaskComplete { agent_id, .. } => Some(agent_id),
                _ => None,
            };

            if let Some(src) = source {
                if !filter.source_agents.contains(src) {
                    return false;
                }
            }
        }

        true
    }
}

use futures::future::BoxFuture;
```

## Request-Response Pattern

### Request-Response implementation over message bus

```rust
/// Request-Response Manager for synchronous communication
pub struct RequestResponseManager {
    bus: Arc<MessageBus>,
    pending_requests: Arc<RwLock<HashMap<RequestId, PendingRequest>>>,
}

pub struct PendingRequest {
    pub requester: AgentId,
    pub created_at: Instant,
    pub timeout: Duration,
    pub response_tx: oneshot::Sender<Response>,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub id: RequestId,
    pub from: AgentId,
    pub to: AgentId,
    pub request_type: RequestType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum RequestType {
    GetAgentStatus,
    GetTaskStatus { task_id: TaskId },
    RequestAssistance { context: String },
    QueryCapabilities,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Response {
    pub request_id: RequestId,
    pub from: AgentId,
    pub success: bool,
    pub payload: serde_json::Value,
}

impl RequestResponseManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            bus,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Sends request and waits for response
    pub async fn request(
        &self,
        from: AgentId,
        to: AgentId,
        request_type: RequestType,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<Response> {
        let request_id = RequestId::new();
        let (response_tx, response_rx) = oneshot::channel();

        // Register pending request
        self.pending_requests.write().await.insert(
            request_id.clone(),
            PendingRequest {
                requester: from.clone(),
                created_at: Instant::now(),
                timeout,
                response_tx,
            },
        );

        // Send request
        let request = Request {
            id: request_id.clone(),
            from: from.clone(),
            to: to.clone(),
            request_type,
            payload,
        };

        let message = Message::SystemEvent {
            event_type: EventType::Request,
            source: from,
            data: serde_json::to_value(&request)?,
        };

        self.bus.send(to, message).await?;

        // Wait for response with timeout
        let response = tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| CoordinationError::RequestTimeout {
                request_id: request_id.clone(),
            })?
            .map_err(|_| CoordinationError::ResponseChannelClosed {
                request_id,
            })?;

        Ok(response)
    }

    /// Sends response to request
    pub async fn respond(
        &self,
        request_id: RequestId,
        from: AgentId,
        success: bool,
        payload: serde_json::Value,
    ) -> Result<()> {
        let response = Response {
            request_id: request_id.clone(),
            from,
            success,
            payload,
        };

        // Find pending request
        let pending = self.pending_requests
            .write()
            .await
            .remove(&request_id)
            .ok_or_else(|| CoordinationError::RequestNotFound {
                request_id: request_id.clone(),
            })?;

        // Send response through channel
        pending.response_tx
            .send(response)
            .map_err(|_| CoordinationError::ResponseChannelClosed { request_id })?;

        Ok(())
    }

    /// Cleans up expired requests
    pub async fn cleanup_expired_requests(&self) {
        let mut pending = self.pending_requests.write().await;
        let now = Instant::now();

        pending.retain(|_, req| {
            now.duration_since(req.created_at) < req.timeout
        });
    }
}
```

## Broadcasting и Multicasting

### Patterns for mass message broadcasting

```rust
/// Broadcasting Manager for mass broadcasting
pub struct BroadcastManager {
    bus: Arc<MessageBus>,
}

impl BroadcastManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self { bus }
    }

    /// Sends message to all registered agents
    pub async fn broadcast_to_all(&self, message: Message) -> Result<BroadcastResult> {
        let channels = self.bus.channels.read().await;
        let total_agents = channels.len();
        let mut successful = 0;
        let mut failed = Vec::new();

        for (agent_id, tx) in channels.iter() {
            match tx.send(message.clone()) {
                Ok(_) => successful += 1,
                Err(_) => failed.push(agent_id.clone()),
            }
        }

        Ok(BroadcastResult {
            total_agents,
            successful,
            failed,
        })
    }

    /// Multicast - sending message to group of agents
    pub async fn multicast(
        &self,
        targets: Vec<AgentId>,
        message: Message,
    ) -> Result<BroadcastResult> {
        let channels = self.bus.channels.read().await;
        let total_agents = targets.len();
        let mut successful = 0;
        let mut failed = Vec::new();

        for agent_id in targets {
            if let Some(tx) = channels.get(&agent_id) {
                match tx.send(message.clone()) {
                    Ok(_) => successful += 1,
                    Err(_) => failed.push(agent_id.clone()),
                }
            } else {
                failed.push(agent_id);
            }
        }

        Ok(BroadcastResult {
            total_agents,
            successful,
            failed,
        })
    }

    /// Sending message to agents with specific capabilities
    pub async fn multicast_by_capability(
        &self,
        capability: Capability,
        message: Message,
    ) -> Result<BroadcastResult> {
        // Get list of agents with required capability through agent registry
        // (assuming AgentRegistry exists)
        let targets = self.find_agents_by_capability(capability).await?;
        self.multicast(targets, message).await
    }

    async fn find_agents_by_capability(&self, _capability: Capability) -> Result<Vec<AgentId>> {
        // TODO: Integration with Agent Registry
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct BroadcastResult {
    pub total_agents: usize,
    pub successful: usize,
    pub failed: Vec<AgentId>,
}
```

## Agent Discovery through Cortex

### Integration with Cortex for agent discovery

```rust
/// Agent Discovery Service using Cortex
pub struct AgentDiscoveryService {
    cortex_bridge: Arc<CortexBridge>,
    local_cache: Arc<RwLock<HashMap<AgentId, AgentInfo>>>,
    cache_ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub status: AgentStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

#[derive(Debug, Clone)]
pub struct AgentMetadata {
    pub version: String,
    pub specialization: Vec<String>,
    pub max_concurrent_tasks: usize,
    pub performance_score: f32,
}

impl AgentDiscoveryService {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        Self {
            cortex_bridge,
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
        }
    }

    /// Registers agent in system through Cortex
    pub async fn register_agent(&self, agent_info: AgentInfo) -> Result<()> {
        // Save agent information in Cortex
        // Use semantic search for indexing capabilities

        // Update local cache
        self.local_cache.write().await.insert(agent_info.id.clone(), agent_info.clone());

        info!("Registered agent {} with capabilities: {:?}", agent_info.id, agent_info.capabilities);
        Ok(())
    }

    /// Finds agents by capabilities
    pub async fn find_agents_by_capabilities(
        &self,
        required_capabilities: &[Capability],
    ) -> Result<Vec<AgentInfo>> {
        // Check cache
        let cache = self.local_cache.read().await;
        let cached_results: Vec<_> = cache
            .values()
            .filter(|agent| {
                required_capabilities
                    .iter()
                    .all(|cap| agent.capabilities.contains(cap))
            })
            .cloned()
            .collect();

        if !cached_results.is_empty() {
            return Ok(cached_results);
        }

        // If not in cache, search through Cortex
        // TODO: Use Cortex semantic search for finding agents

        Ok(cached_results)
    }

    /// Finds the most suitable agent for task
    pub async fn find_best_agent(&self, task: &Task) -> Result<AgentInfo> {
        let candidates = self.find_agents_by_capabilities(&task.requirements.capabilities).await?;

        if candidates.is_empty() {
            return Err(CoordinationError::NoSuitableAgent {
                task_id: task.id.clone(),
                requirements: task.requirements.clone(),
            }.into());
        }

        // Choose agent with best performance score and availability
        let best = candidates
            .into_iter()
            .filter(|agent| agent.status == AgentStatus::Idle || agent.status == AgentStatus::Working)
            .max_by(|a, b| {
                a.metadata.performance_score
                    .partial_cmp(&b.metadata.performance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| CoordinationError::NoAvailableAgent)?;

        Ok(best)
    }

    /// Updates agent status
    pub async fn update_agent_status(&self, agent_id: AgentId, status: AgentStatus) -> Result<()> {
        let mut cache = self.local_cache.write().await;

        if let Some(agent) = cache.get_mut(&agent_id) {
            agent.status = status;
            agent.last_heartbeat = Utc::now();
        }

        Ok(())
    }

    /// Heartbeat for checking agent availability
    pub async fn heartbeat(&self, agent_id: AgentId) -> Result<()> {
        self.update_agent_status(agent_id, AgentStatus::Working).await
    }

    /// Cleans up information about unavailable agents
    pub async fn cleanup_stale_agents(&self) {
        let mut cache = self.local_cache.write().await;
        let threshold = Utc::now() - chrono::Duration::seconds(self.cache_ttl.as_secs() as i64);

        cache.retain(|_, agent| agent.last_heartbeat > threshold);
    }
}
```

## Coordination Topologies

### Different agent coordination topologies

```rust
/// Topology Manager manages communication topology
pub struct TopologyManager {
    current_topology: Arc<RwLock<Topology>>,
    bus: Arc<MessageBus>,
}

#[derive(Debug, Clone)]
pub enum Topology {
    /// Star - one central orchestrator
    Star {
        coordinator: AgentId,
        workers: Vec<AgentId>,
    },

    /// Mesh - each agent can communicate with each other
    FullMesh {
        nodes: Vec<AgentId>,
    },

    /// Ring - agents organized in a ring
    Ring {
        nodes: Vec<AgentId>,
    },

    /// Hierarchical - hierarchical structure
    Hierarchical {
        root: AgentId,
        levels: Vec<Vec<AgentId>>,
    },

    /// Pipeline - sequential processing
    Pipeline {
        stages: Vec<AgentId>,
    },
}

impl TopologyManager {
    pub fn new(bus: Arc<MessageBus>) -> Self {
        Self {
            current_topology: Arc::new(RwLock::new(Topology::FullMesh { nodes: Vec::new() })),
            bus,
        }
    }

    /// Sets Star topology
    pub async fn setup_star_topology(
        &self,
        coordinator: AgentId,
        workers: Vec<AgentId>,
    ) -> Result<()> {
        let topology = Topology::Star { coordinator: coordinator.clone(), workers };
        *self.current_topology.write().await = topology;

        info!("Setup star topology with coordinator {}", coordinator);
        Ok(())
    }

    /// Sets Mesh topology
    pub async fn setup_mesh_topology(&self, nodes: Vec<AgentId>) -> Result<()> {
        let topology = Topology::FullMesh { nodes };
        *self.current_topology.write().await = topology;

        info!("Setup mesh topology");
        Ok(())
    }

    /// Sets Pipeline topology
    pub async fn setup_pipeline_topology(&self, stages: Vec<AgentId>) -> Result<()> {
        let topology = Topology::Pipeline { stages };
        *self.current_topology.write().await = topology;

        info!("Setup pipeline topology");
        Ok(())
    }

    /// Sends message according to current topology
    pub async fn route_message(
        &self,
        from: AgentId,
        to: Option<AgentId>,
        message: Message,
    ) -> Result<()> {
        let topology = self.current_topology.read().await;

        match &*topology {
            Topology::Star { coordinator, workers } => {
                // In Star topology all messages go through coordinator
                if from == *coordinator {
                    // Coordinator sends directly to workers
                    if let Some(to) = to {
                        self.bus.send(to, message).await?;
                    } else {
                        // Broadcast to all workers
                        for worker in workers {
                            self.bus.send(worker.clone(), message.clone()).await?;
                        }
                    }
                } else {
                    // Workers send through coordinator
                    self.bus.send(coordinator.clone(), message).await?;
                }
            }

            Topology::FullMesh { .. } => {
                // In Mesh agents send directly
                if let Some(to) = to {
                    self.bus.send(to, message).await?;
                } else {
                    return Err(CoordinationError::InvalidRoute {
                        reason: "Target required in mesh topology".to_string(),
                    }.into());
                }
            }

            Topology::Ring { nodes } => {
                // In Ring message goes around the ring
                let from_idx = nodes.iter().position(|id| *id == from)
                    .ok_or_else(|| CoordinationError::AgentNotInTopology { agent_id: from })?;

                let next_idx = (from_idx + 1) % nodes.len();
                self.bus.send(nodes[next_idx].clone(), message).await?;
            }

            Topology::Pipeline { stages } => {
                // In Pipeline message goes to next stage
                let from_idx = stages.iter().position(|id| *id == from)
                    .ok_or_else(|| CoordinationError::AgentNotInTopology { agent_id: from })?;

                if from_idx + 1 < stages.len() {
                    self.bus.send(stages[from_idx + 1].clone(), message).await?;
                } else {
                    // Last stage - pipeline completion
                    info!("Pipeline completed");
                }
            }

            Topology::Hierarchical { root, levels } => {
                // In Hierarchical messages go up/down the hierarchy
                if from == *root {
                    // Root sends to first level
                    if let Some(first_level) = levels.first() {
                        for agent in first_level {
                            self.bus.send(agent.clone(), message.clone()).await?;
                        }
                    }
                } else {
                    // Find sender level and send to higher level
                    for (level_idx, level) in levels.iter().enumerate() {
                        if level.contains(&from) {
                            if level_idx == 0 {
                                // First level sends to root
                                self.bus.send(root.clone(), message).await?;
                            } else {
                                // Send to previous level
                                let prev_level = &levels[level_idx - 1];
                                for agent in prev_level {
                                    self.bus.send(agent.clone(), message.clone()).await?;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
```

## Coordination Patterns Usage Examples

### Example 1: Star Topology for task coordination

```rust
pub async fn coordinate_with_star_topology(
    bus: Arc<MessageBus>,
    coordinator_id: AgentId,
    worker_ids: Vec<AgentId>,
    tasks: Vec<Task>,
) -> Result<()> {
    let topology_manager = TopologyManager::new(bus.clone());

    // Setup Star topology
    topology_manager.setup_star_topology(coordinator_id.clone(), worker_ids.clone()).await?;

    // Coordinator assigns tasks to workers
    for (task, worker_id) in tasks.into_iter().zip(worker_ids.iter().cycle()) {
        let message = Message::TaskAssignment {
            task: task.clone(),
            agent_id: worker_id.clone(),
            session_id: SessionId::new(),
            context: Vec::new(),
        };

        topology_manager.route_message(coordinator_id.clone(), Some(worker_id.clone()), message).await?;
    }

    Ok(())
}
```

### Example 2: Pipeline for sequential processing

```rust
pub async fn process_with_pipeline(
    bus: Arc<MessageBus>,
    stages: Vec<AgentId>,
    initial_data: serde_json::Value,
) -> Result<()> {
    let topology_manager = TopologyManager::new(bus.clone());

    // Setup Pipeline topology
    topology_manager.setup_pipeline_topology(stages.clone()).await?;

    // Send data to first stage
    let message = Message::SystemEvent {
        event_type: EventType::DataProcessing,
        source: stages[0].clone(),
        data: initial_data,
    };

    topology_manager.route_message(stages[0].clone(), None, message).await?;

    Ok(())
}
```

### Example 3: Pub/Sub for event-driven coordination

```rust
pub async fn setup_event_driven_coordination(
    bus: Arc<MessageBus>,
    agents: Vec<AgentId>,
) -> Result<()> {
    let pubsub_manager = PubSubManager::new(bus.clone());

    // Each agent subscribes to task updates
    for agent_id in agents {
        pubsub_manager.register_handler(
            agent_id.clone(),
            Topic::task_updates(),
            None,
            |message| {
                Box::pin(async move {
                    match message {
                        Message::TaskComplete { task_id, result, .. } => {
                            info!("Task {} completed: {:?}", task_id, result);
                        }
                        Message::TaskFailed { task_id, error, .. } => {
                            error!("Task {} failed: {}", task_id, error);
                        }
                        _ => {}
                    }
                })
            },
        ).await?;
    }

    Ok(())
}
```

### Example 4: Request-Response for synchronous interaction

```rust
pub async fn agent_request_assistance(
    request_response: Arc<RequestResponseManager>,
    requester: AgentId,
    helper: AgentId,
    context: String,
) -> Result<Response> {
    let response = request_response.request(
        requester,
        helper,
        RequestType::RequestAssistance { context },
        serde_json::json!({}),
        Duration::from_secs(30),
    ).await?;

    if response.success {
        info!("Received assistance: {:?}", response.payload);
    }

    Ok(response)
}
```

## Metrics and Monitoring

```rust
/// Metrics for Coordination Layer
pub struct BusMetrics {
    pub agents_registered: AtomicU64,
    pub messages_sent: AtomicU64,
    pub messages_published: AtomicU64,
    pub subscriptions_created: AtomicU64,
    pub messages_dropped: AtomicU64,
    pub avg_message_latency: AtomicF64,
}

impl BusMetrics {
    pub fn new() -> Self {
        Self {
            agents_registered: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_published: AtomicU64::new(0),
            subscriptions_created: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            avg_message_latency: AtomicF64::new(0.0),
        }
    }

    pub fn export(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            agents_registered: self.agents_registered.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_published: self.messages_published.load(Ordering::Relaxed),
            subscriptions_created: self.subscriptions_created.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            avg_message_latency_ms: self.avg_message_latency.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub agents_registered: u64,
    pub messages_sent: u64,
    pub messages_published: u64,
    pub subscriptions_created: u64,
    pub messages_dropped: u64,
    pub avg_message_latency_ms: f64,
}
```

## Cortex WebSocket Events

The coordination layer integrates with Cortex via WebSocket for real-time event streaming. These events enable agents to react to system changes immediately.

### Event Types

```rust
#[derive(Debug, Clone)]
pub enum CortexEvent {
    /// Session created for an agent
    SessionCreated { session_id: String },

    /// Session merged back to workspace
    SessionMerged { session_id: String, conflicts: u32 },

    /// Session closed and cleaned up
    SessionClosed { session_id: String },

    /// Lock acquired on an entity
    LockAcquired { lock_id: String, entity_id: String },

    /// Lock released
    LockReleased { lock_id: String },

    /// Deadlock detected between agents
    LockDeadlock { entity_id: String, agents: Vec<String> },

    /// Conflict detected during merge
    ConflictDetected { session_id: String, files: Vec<String> },

    /// File changed in workspace
    FileChanged { path: String, workspace_id: String },

    /// Code pattern detected
    PatternDetected { pattern: String, confidence: f32 },
}
```

### Event Subscription Example

```rust
pub async fn subscribe_to_cortex_events(
    cortex_bridge: Arc<CortexBridge>,
    agent_id: AgentId,
) -> Result<()> {
    // Subscribe to session events
    cortex_bridge.subscribe_events(
        EventFilter::Sessions,
        move |event| {
            match event {
                CortexEvent::SessionMerged { session_id, conflicts } => {
                    if conflicts > 0 {
                        warn!("Session {} merged with {} conflicts", session_id, conflicts);
                    }
                }
                CortexEvent::SessionClosed { session_id } => {
                    info!("Session {} closed", session_id);
                }
                _ => {}
            }
        },
    ).await;

    // Subscribe to lock events for coordination
    cortex_bridge.subscribe_events(
        EventFilter::Locks,
        move |event| {
            match event {
                CortexEvent::LockDeadlock { entity_id, agents } => {
                    error!("Deadlock detected on {}: agents {:?}", entity_id, agents);
                    // Trigger deadlock resolution
                }
                _ => {}
            }
        },
    ).await;

    Ok(())
}
```

---

Coordination Patterns provide flexible and efficient communication between agents, supporting various topologies and interaction patterns depending on task requirements.
