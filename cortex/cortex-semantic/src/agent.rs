//! Multi-agent coordination for semantic search.
//!
//! This module provides comprehensive multi-agent coordination features based on 2025
//! research in distributed AI systems:
//!
//! - Agent identity and context tracking
//! - Agent-specific embedding namespaces (isolation)
//! - Priority-based search queuing for urgent queries
//! - Collaborative and private memory pools
//! - Cross-agent knowledge retrieval with access control
//! - Conflict resolution strategies
//! - Performance metrics per agent
//!
//! # Architecture
//!
//! The multi-agent system follows the orchestrator-worker pattern:
//! - **AgentCoordinator**: Central orchestrator managing agent registry and resources
//! - **AgentContext**: Individual agent identity and capabilities
//! - **SearchOrchestrator**: Coordinates search across multiple agents
//! - **MemoryPool**: Shared semantic memory with access control
//!
//! # Example
//!
//! ```no_run
//! use cortex_semantic::agent::*;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create coordinator
//! let coordinator = AgentCoordinator::new();
//!
//! // Register agents
//! let agent1 = coordinator.register_agent(
//!     "worker-1",
//!     AgentRole::Worker,
//!     vec!["rust", "python"]
//! ).await?;
//!
//! let agent2 = coordinator.register_agent(
//!     "orchestrator",
//!     AgentRole::Orchestrator,
//!     vec![]
//! ).await?;
//!
//! // Create memory pool with access control
//! let pool = MemoryPool::new(AccessPolicy::Shared);
//!
//! # Ok(())
//! # }
//! ```

use crate::error::{Result, SemanticError};
use crate::types::{DocumentId, Vector};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Agent identifier.
pub type AgentId = String;

/// Namespace for agent-specific embeddings.
pub type Namespace = String;

/// Agent role in the multi-agent system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Orchestrator coordinates other agents
    Orchestrator,
    /// Worker performs specific tasks
    Worker,
    /// Specialist has domain expertise
    Specialist,
    /// Observer monitors without modifying state
    Observer,
}

/// Agent priority for search operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchPriority {
    /// Critical priority (0 - highest)
    Critical = 0,
    /// High priority (1)
    High = 1,
    /// Normal priority (2)
    Normal = 2,
    /// Low priority (3)
    Low = 3,
    /// Background priority (4 - lowest)
    Background = 4,
}

impl Default for SearchPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Agent context and identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Unique agent identifier
    pub agent_id: AgentId,
    /// Agent role
    pub role: AgentRole,
    /// Agent-specific namespace for embeddings
    pub namespace: Namespace,
    /// Agent capabilities/domains
    pub capabilities: HashSet<String>,
    /// Agent metadata
    pub metadata: HashMap<String, String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last active timestamp
    pub last_active: chrono::DateTime<chrono::Utc>,
}

impl AgentContext {
    /// Create a new agent context.
    pub fn new(agent_id: impl Into<String>, role: AgentRole, capabilities: Vec<String>) -> Self {
        let agent_id = agent_id.into();
        let namespace = format!("agent::{}", agent_id);

        Self {
            agent_id,
            role,
            namespace,
            capabilities: capabilities.into_iter().collect(),
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        }
    }

    /// Update last active timestamp.
    pub fn update_activity(&mut self) {
        self.last_active = chrono::Utc::now();
    }

    /// Check if agent has a specific capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(capability)
    }

    /// Add metadata.
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Access policy for memory pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessPolicy {
    /// All agents can read and write
    Shared,
    /// All agents can read, only owners can write
    ReadOnly,
    /// Only specific agents can access
    Private,
    /// Hierarchical access based on agent roles
    Hierarchical,
}

/// Memory access permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// Access policy
    pub policy: AccessPolicy,
    /// Owner agent IDs
    pub owners: HashSet<AgentId>,
    /// Readers (for private policy)
    pub readers: HashSet<AgentId>,
    /// Writers (for private policy)
    pub writers: HashSet<AgentId>,
}

impl AccessControl {
    /// Create a new access control with policy.
    pub fn new(policy: AccessPolicy) -> Self {
        Self {
            policy,
            owners: HashSet::new(),
            readers: HashSet::new(),
            writers: HashSet::new(),
        }
    }

    /// Check if agent can read.
    pub fn can_read(&self, agent_id: &AgentId, role: AgentRole) -> bool {
        match self.policy {
            AccessPolicy::Shared => true,
            AccessPolicy::ReadOnly => true,
            AccessPolicy::Private => {
                self.owners.contains(agent_id) || self.readers.contains(agent_id)
            }
            AccessPolicy::Hierarchical => {
                matches!(role, AgentRole::Orchestrator) || self.readers.contains(agent_id)
            }
        }
    }

    /// Check if agent can write.
    pub fn can_write(&self, agent_id: &AgentId, role: AgentRole) -> bool {
        match self.policy {
            AccessPolicy::Shared => true,
            AccessPolicy::ReadOnly => self.owners.contains(agent_id),
            AccessPolicy::Private => {
                self.owners.contains(agent_id) || self.writers.contains(agent_id)
            }
            AccessPolicy::Hierarchical => {
                matches!(role, AgentRole::Orchestrator) || self.owners.contains(agent_id)
            }
        }
    }

    /// Add owner.
    pub fn add_owner(&mut self, agent_id: impl Into<AgentId>) {
        self.owners.insert(agent_id.into());
    }

    /// Add reader.
    pub fn add_reader(&mut self, agent_id: impl Into<AgentId>) {
        self.readers.insert(agent_id.into());
    }

    /// Add writer.
    pub fn add_writer(&mut self, agent_id: impl Into<AgentId>) {
        self.writers.insert(agent_id.into());
    }
}

/// Shared semantic memory pool with access control.
#[derive(Debug)]
pub struct MemoryPool {
    /// Pool identifier
    pub pool_id: String,
    /// Access control
    pub access_control: Arc<RwLock<AccessControl>>,
    /// Stored embeddings: (doc_id, agent_id, vector, metadata)
    entries: Arc<DashMap<DocumentId, MemoryEntry>>,
    /// Statistics
    stats: Arc<MemoryPoolStats>,
}

/// Memory pool entry.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub doc_id: DocumentId,
    pub agent_id: AgentId,
    pub vector: Vector,
    pub metadata: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
}

/// Memory pool statistics.
#[derive(Debug, Default)]
pub struct MemoryPoolStats {
    pub total_entries: std::sync::atomic::AtomicU64,
    pub reads: std::sync::atomic::AtomicU64,
    pub writes: std::sync::atomic::AtomicU64,
    pub access_denied: std::sync::atomic::AtomicU64,
}

impl MemoryPool {
    /// Create a new memory pool.
    pub fn new(policy: AccessPolicy) -> Self {
        Self {
            pool_id: Uuid::new_v4().to_string(),
            access_control: Arc::new(RwLock::new(AccessControl::new(policy))),
            entries: Arc::new(DashMap::new()),
            stats: Arc::new(MemoryPoolStats::default()),
        }
    }

    /// Store an embedding in the pool.
    pub async fn store(
        &self,
        agent_id: &AgentId,
        role: AgentRole,
        doc_id: DocumentId,
        vector: Vector,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        // Check write permission
        let ac = self.access_control.read().await;
        if !ac.can_write(agent_id, role) {
            self.stats.access_denied.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(SemanticError::Concurrent(format!(
                "Agent {} does not have write permission",
                agent_id
            )));
        }
        drop(ac);

        let entry = MemoryEntry {
            doc_id: doc_id.clone(),
            agent_id: agent_id.clone(),
            vector,
            metadata,
            created_at: chrono::Utc::now(),
            access_count: 0,
        };

        self.entries.insert(doc_id, entry);
        self.stats.writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.stats.total_entries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Retrieve an embedding from the pool.
    pub async fn retrieve(
        &self,
        agent_id: &AgentId,
        role: AgentRole,
        doc_id: &DocumentId,
    ) -> Result<Option<MemoryEntry>> {
        // Check read permission
        let ac = self.access_control.read().await;
        if !ac.can_read(agent_id, role) {
            self.stats.access_denied.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(SemanticError::Concurrent(format!(
                "Agent {} does not have read permission",
                agent_id
            )));
        }
        drop(ac);

        self.stats.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let entry = self.entries.get_mut(doc_id).map(|mut e| {
            e.access_count += 1;
            e.clone()
        });

        Ok(entry)
    }

    /// Search across all entries the agent has access to.
    pub async fn search(
        &self,
        agent_id: &AgentId,
        role: AgentRole,
        query: &[f32],
        limit: usize,
    ) -> Result<Vec<(DocumentId, f32)>> {
        // Check read permission
        let ac = self.access_control.read().await;
        if !ac.can_read(agent_id, role) {
            self.stats.access_denied.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Err(SemanticError::Concurrent(format!(
                "Agent {} does not have read permission",
                agent_id
            )));
        }
        drop(ac);

        self.stats.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Simple cosine similarity search
        let mut results: Vec<(DocumentId, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let score = crate::types::cosine_similarity(query, &entry.vector);
                (entry.doc_id.clone(), score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    /// Get statistics.
    pub fn stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("total_entries".to_string(),
            self.stats.total_entries.load(std::sync::atomic::Ordering::Relaxed));
        stats.insert("reads".to_string(),
            self.stats.reads.load(std::sync::atomic::Ordering::Relaxed));
        stats.insert("writes".to_string(),
            self.stats.writes.load(std::sync::atomic::Ordering::Relaxed));
        stats.insert("access_denied".to_string(),
            self.stats.access_denied.load(std::sync::atomic::Ordering::Relaxed));
        stats
    }
}

/// Agent coordinator - central orchestrator for multi-agent system.
pub struct AgentCoordinator {
    /// Registered agents
    agents: Arc<DashMap<AgentId, Arc<RwLock<AgentContext>>>>,
    /// Memory pools
    memory_pools: Arc<DashMap<String, Arc<MemoryPool>>>,
    /// Agent metrics
    metrics: Arc<DashMap<AgentId, Arc<AgentMetrics>>>,
    /// Semaphore for limiting concurrent operations
    concurrency_limit: Arc<Semaphore>,
}

/// Per-agent metrics.
#[derive(Debug, Default)]
pub struct AgentMetrics {
    pub search_count: std::sync::atomic::AtomicU64,
    pub cache_hits: std::sync::atomic::AtomicU64,
    pub cache_misses: std::sync::atomic::AtomicU64,
    pub total_search_time_ms: std::sync::atomic::AtomicU64,
    pub memory_usage_bytes: std::sync::atomic::AtomicU64,
    pub cross_agent_requests: std::sync::atomic::AtomicU64,
    pub conflicts_resolved: std::sync::atomic::AtomicU64,
}

impl AgentMetrics {
    /// Record a search operation.
    pub fn record_search(&self, duration_ms: u64, cache_hit: bool) {
        self.search_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_search_time_ms.fetch_add(duration_ms, std::sync::atomic::Ordering::Relaxed);
        if cache_hit {
            self.cache_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Get average search latency.
    pub fn avg_search_latency_ms(&self) -> f64 {
        let count = self.search_count.load(std::sync::atomic::Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        let total = self.total_search_time_ms.load(std::sync::atomic::Ordering::Relaxed);
        total as f64 / count as f64
    }

    /// Get cache hit rate.
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }
        hits as f64 / total as f64
    }

    /// Get metrics as HashMap.
    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("search_count".to_string(),
            self.search_count.load(std::sync::atomic::Ordering::Relaxed) as f64);
        map.insert("avg_search_latency_ms".to_string(), self.avg_search_latency_ms());
        map.insert("cache_hit_rate".to_string(), self.cache_hit_rate());
        map.insert("memory_usage_mb".to_string(),
            self.memory_usage_bytes.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1024.0 / 1024.0);
        map.insert("cross_agent_requests".to_string(),
            self.cross_agent_requests.load(std::sync::atomic::Ordering::Relaxed) as f64);
        map.insert("conflicts_resolved".to_string(),
            self.conflicts_resolved.load(std::sync::atomic::Ordering::Relaxed) as f64);
        map
    }
}

impl AgentCoordinator {
    /// Create a new agent coordinator.
    pub fn new() -> Self {
        Self::with_concurrency_limit(100) // Default 100 concurrent operations
    }

    /// Create coordinator with custom concurrency limit.
    pub fn with_concurrency_limit(limit: usize) -> Self {
        info!("Initializing AgentCoordinator with concurrency limit: {}", limit);

        Self {
            agents: Arc::new(DashMap::new()),
            memory_pools: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
            concurrency_limit: Arc::new(Semaphore::new(limit)),
        }
    }

    /// Register a new agent.
    pub async fn register_agent(
        &self,
        agent_id: impl Into<String>,
        role: AgentRole,
        capabilities: Vec<String>,
    ) -> Result<Arc<RwLock<AgentContext>>> {
        let agent_id = agent_id.into();

        info!("Registering agent: {} (role: {:?})", agent_id, role);

        let context = AgentContext::new(&agent_id, role, capabilities);
        let context = Arc::new(RwLock::new(context));

        self.agents.insert(agent_id.clone(), context.clone());
        self.metrics.insert(agent_id.clone(), Arc::new(AgentMetrics::default()));

        Ok(context)
    }

    /// Unregister an agent.
    pub async fn unregister_agent(&self, agent_id: &AgentId) -> Result<()> {
        info!("Unregistering agent: {}", agent_id);

        self.agents.remove(agent_id);
        self.metrics.remove(agent_id);

        Ok(())
    }

    /// Get agent context.
    pub fn get_agent(&self, agent_id: &AgentId) -> Option<Arc<RwLock<AgentContext>>> {
        self.agents.get(agent_id).map(|a| a.clone())
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<AgentId> {
        self.agents.iter().map(|e| e.key().clone()).collect()
    }

    /// Get agent metrics.
    pub fn get_metrics(&self, agent_id: &AgentId) -> Option<Arc<AgentMetrics>> {
        self.metrics.get(agent_id).map(|m| m.clone())
    }

    /// Create a memory pool.
    pub fn create_memory_pool(&self, pool_id: impl Into<String>, policy: AccessPolicy) -> Arc<MemoryPool> {
        let pool_id = pool_id.into();

        info!("Creating memory pool: {} (policy: {:?})", pool_id, policy);

        let pool = Arc::new(MemoryPool::new(policy));
        self.memory_pools.insert(pool_id, pool.clone());

        pool
    }

    /// Get a memory pool.
    pub fn get_memory_pool(&self, pool_id: &str) -> Option<Arc<MemoryPool>> {
        self.memory_pools.get(pool_id).map(|p| p.clone())
    }

    /// Acquire concurrency permit for operation.
    pub async fn acquire_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        self.concurrency_limit
            .acquire()
            .await
            .map_err(|e| SemanticError::Concurrent(e.to_string()))
    }

    /// Get system-wide statistics.
    pub fn system_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert("total_agents".to_string(),
            serde_json::json!(self.agents.len()));
        stats.insert("total_memory_pools".to_string(),
            serde_json::json!(self.memory_pools.len()));

        // Aggregate metrics
        let mut total_searches = 0u64;
        let mut total_cross_agent = 0u64;

        for entry in self.metrics.iter() {
            let metrics = entry.value();
            total_searches += metrics.search_count.load(std::sync::atomic::Ordering::Relaxed);
            total_cross_agent += metrics.cross_agent_requests.load(std::sync::atomic::Ordering::Relaxed);
        }

        stats.insert("total_searches".to_string(), serde_json::json!(total_searches));
        stats.insert("total_cross_agent_requests".to_string(), serde_json::json!(total_cross_agent));

        stats
    }
}

impl Default for AgentCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Search request with priority.
#[derive(Debug, Clone)]
pub struct PrioritizedSearchRequest {
    pub request_id: String,
    pub agent_id: AgentId,
    pub query: String,
    pub priority: SearchPriority,
    pub namespace: Option<Namespace>,
    pub created_at: Instant,
}

impl PrioritizedSearchRequest {
    /// Create a new prioritized search request.
    pub fn new(
        agent_id: impl Into<AgentId>,
        query: impl Into<String>,
        priority: SearchPriority,
    ) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            query: query.into(),
            priority,
            namespace: None,
            created_at: Instant::now(),
        }
    }

    /// Set namespace for search.
    pub fn with_namespace(mut self, namespace: impl Into<Namespace>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Get age of request.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Priority queue for search requests.
pub struct SearchQueue {
    /// Queues by priority level
    queues: Arc<RwLock<HashMap<SearchPriority, VecDeque<PrioritizedSearchRequest>>>>,
    /// Maximum queue size per priority
    max_queue_size: usize,
}

impl SearchQueue {
    /// Create a new search queue.
    pub fn new(max_queue_size: usize) -> Self {
        let mut queues = HashMap::new();
        queues.insert(SearchPriority::Critical, VecDeque::new());
        queues.insert(SearchPriority::High, VecDeque::new());
        queues.insert(SearchPriority::Normal, VecDeque::new());
        queues.insert(SearchPriority::Low, VecDeque::new());
        queues.insert(SearchPriority::Background, VecDeque::new());

        Self {
            queues: Arc::new(RwLock::new(queues)),
            max_queue_size,
        }
    }

    /// Enqueue a search request.
    pub async fn enqueue(&self, request: PrioritizedSearchRequest) -> Result<()> {
        let mut queues = self.queues.write().await;

        let queue = queues.get_mut(&request.priority)
            .ok_or_else(|| SemanticError::Search("Invalid priority".to_string()))?;

        if queue.len() >= self.max_queue_size {
            warn!("Queue full for priority {:?}, dropping oldest request", request.priority);
            queue.pop_front();
        }

        debug!("Enqueuing request {} for agent {} with priority {:?}",
            request.request_id, request.agent_id, request.priority);

        queue.push_back(request);
        Ok(())
    }

    /// Dequeue highest priority request.
    pub async fn dequeue(&self) -> Option<PrioritizedSearchRequest> {
        let mut queues = self.queues.write().await;

        // Try each priority in order
        for priority in [
            SearchPriority::Critical,
            SearchPriority::High,
            SearchPriority::Normal,
            SearchPriority::Low,
            SearchPriority::Background,
        ] {
            if let Some(queue) = queues.get_mut(&priority) {
                if let Some(request) = queue.pop_front() {
                    debug!("Dequeuing request {} with priority {:?}",
                        request.request_id, priority);
                    return Some(request);
                }
            }
        }

        None
    }

    /// Get queue sizes.
    pub async fn queue_sizes(&self) -> HashMap<SearchPriority, usize> {
        let queues = self.queues.read().await;
        queues.iter().map(|(p, q)| (*p, q.len())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_context_creation() {
        let context = AgentContext::new("worker-1", AgentRole::Worker, vec!["rust".to_string()]);

        assert_eq!(context.agent_id, "worker-1");
        assert_eq!(context.role, AgentRole::Worker);
        assert_eq!(context.namespace, "agent::worker-1");
        assert!(context.has_capability("rust"));
        assert!(!context.has_capability("python"));
    }

    #[test]
    fn test_access_control_shared() {
        let ac = AccessControl::new(AccessPolicy::Shared);

        assert!(ac.can_read(&"agent1".to_string(), AgentRole::Worker));
        assert!(ac.can_write(&"agent1".to_string(), AgentRole::Worker));
    }

    #[test]
    fn test_access_control_private() {
        let mut ac = AccessControl::new(AccessPolicy::Private);
        ac.add_owner("owner");
        ac.add_reader("reader");

        assert!(ac.can_read(&"owner".to_string(), AgentRole::Worker));
        assert!(ac.can_write(&"owner".to_string(), AgentRole::Worker));

        assert!(ac.can_read(&"reader".to_string(), AgentRole::Worker));
        assert!(!ac.can_write(&"reader".to_string(), AgentRole::Worker));

        assert!(!ac.can_read(&"stranger".to_string(), AgentRole::Worker));
    }

    #[test]
    fn test_access_control_hierarchical() {
        let mut ac = AccessControl::new(AccessPolicy::Hierarchical);
        ac.add_reader("worker");

        // Orchestrator can always read/write
        assert!(ac.can_read(&"orchestrator".to_string(), AgentRole::Orchestrator));
        assert!(ac.can_write(&"orchestrator".to_string(), AgentRole::Orchestrator));

        // Worker with permission can read
        assert!(ac.can_read(&"worker".to_string(), AgentRole::Worker));
        assert!(!ac.can_write(&"worker".to_string(), AgentRole::Worker));
    }

    #[tokio::test]
    async fn test_memory_pool_shared() {
        let pool = MemoryPool::new(AccessPolicy::Shared);

        let vector = vec![1.0, 2.0, 3.0];

        pool.store(
            &"agent1".to_string(),
            AgentRole::Worker,
            "doc1".to_string(),
            vector.clone(),
            HashMap::new(),
        )
        .await
        .unwrap();

        let entry = pool.retrieve(
            &"agent2".to_string(),
            AgentRole::Worker,
            &"doc1".to_string(),
        )
        .await
        .unwrap();

        assert!(entry.is_some());
        assert_eq!(entry.unwrap().vector, vector);
    }

    #[tokio::test]
    async fn test_memory_pool_private() {
        let pool = MemoryPool::new(AccessPolicy::Private);

        // Add owner
        {
            let mut ac = pool.access_control.write().await;
            ac.add_owner("agent1");
        }

        let vector = vec![1.0, 2.0, 3.0];

        // Owner can store
        pool.store(
            &"agent1".to_string(),
            AgentRole::Worker,
            "doc1".to_string(),
            vector.clone(),
            HashMap::new(),
        )
        .await
        .unwrap();

        // Non-owner cannot retrieve
        let result = pool.retrieve(
            &"agent2".to_string(),
            AgentRole::Worker,
            &"doc1".to_string(),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agent_coordinator() {
        let coordinator = AgentCoordinator::new();

        let _agent1 = coordinator.register_agent(
            "worker-1",
            AgentRole::Worker,
            vec!["rust".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(coordinator.list_agents().len(), 1);

        let context = coordinator.get_agent(&"worker-1".to_string()).unwrap();
        let context_read = context.read().await;
        assert_eq!(context_read.agent_id, "worker-1");
    }

    #[tokio::test]
    async fn test_search_queue() {
        let queue = SearchQueue::new(10);

        let req1 = PrioritizedSearchRequest::new("agent1", "query1", SearchPriority::Normal);
        let req2 = PrioritizedSearchRequest::new("agent2", "query2", SearchPriority::Critical);
        let req3 = PrioritizedSearchRequest::new("agent3", "query3", SearchPriority::Low);

        queue.enqueue(req1).await.unwrap();
        queue.enqueue(req2).await.unwrap();
        queue.enqueue(req3).await.unwrap();

        // Should dequeue critical first
        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.priority, SearchPriority::Critical);

        // Then normal
        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.priority, SearchPriority::Normal);

        // Then low
        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.priority, SearchPriority::Low);
    }

    #[test]
    fn test_agent_metrics() {
        let metrics = AgentMetrics::default();

        metrics.record_search(100, true);
        metrics.record_search(200, false);
        metrics.record_search(150, true);

        assert_eq!(metrics.search_count.load(std::sync::atomic::Ordering::Relaxed), 3);
        assert_eq!(metrics.avg_search_latency_ms(), 150.0);
        assert!((metrics.cache_hit_rate() - 0.6666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_memory_pool_search() {
        let pool = MemoryPool::new(AccessPolicy::Shared);

        // Store some vectors
        pool.store(
            &"agent1".to_string(),
            AgentRole::Worker,
            "doc1".to_string(),
            vec![1.0, 0.0, 0.0],
            HashMap::new(),
        )
        .await
        .unwrap();

        pool.store(
            &"agent1".to_string(),
            AgentRole::Worker,
            "doc2".to_string(),
            vec![0.0, 1.0, 0.0],
            HashMap::new(),
        )
        .await
        .unwrap();

        // Search with query vector
        let query = vec![1.0, 0.0, 0.0];
        let results = pool.search(
            &"agent2".to_string(),
            AgentRole::Worker,
            &query,
            2,
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1"); // Should be most similar
    }
}
