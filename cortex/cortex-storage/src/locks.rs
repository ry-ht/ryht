//! Entity Lock System with Deadlock Detection
//!
//! This module provides fine-grained entity locking for multi-agent coordination:
//! - Read locks (shared)
//! - Write locks (exclusive)
//! - Intent locks (hierarchical)
//! - Deadlock detection using wait-for graphs
//! - Automatic timeout and cleanup
//! - Session-based lock management

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cortex_core::error::{CortexError, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

// ==============================================================================
// Type Aliases
// ==============================================================================

pub type LockId = String;
pub type SessionId = String;

// ==============================================================================
// Lock Data Model
// ==============================================================================

/// Entity lock representing a lock on a specific entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLock {
    /// Unique lock identifier
    pub lock_id: LockId,
    /// Entity being locked (code unit ID, VNode ID, etc.)
    pub entity_id: String,
    /// Type of entity being locked
    pub entity_type: EntityType,
    /// Type of lock (Read, Write, Intent)
    pub lock_type: LockType,
    /// Session holding the lock
    pub holder_session: SessionId,
    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,
    /// When the lock expires
    pub expires_at: DateTime<Utc>,
    /// Lock metadata
    pub metadata: LockMetadata,
}

/// Lock metadata for tracking and debugging
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockMetadata {
    /// Agent ID that requested the lock
    pub agent_id: Option<String>,
    /// Purpose of the lock
    pub purpose: Option<String>,
    /// Stack trace or context
    pub context: Option<String>,
    /// Number of times lock was renewed
    pub renewal_count: u32,
}

/// Type of lock
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum LockType {
    /// Shared lock - multiple readers allowed
    Read,
    /// Exclusive lock - single writer
    Write,
    /// Intent lock - for hierarchical locking
    Intent,
}

/// Type of entity being locked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    /// Virtual filesystem node
    VNode,
    /// Code unit (function, class, etc.)
    CodeUnit,
    /// Dependency graph node
    Dependency,
    /// Entire workspace
    Workspace,
    /// Custom entity type
    Custom,
}

/// Lock request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockRequest {
    /// Entity to lock
    pub entity_id: String,
    /// Type of entity
    pub entity_type: EntityType,
    /// Type of lock requested
    pub lock_type: LockType,
    /// Lock timeout duration
    pub timeout: Duration,
    /// Optional metadata
    pub metadata: Option<LockMetadata>,
}

/// Result of lock acquisition attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LockAcquisition {
    /// Lock successfully acquired
    Acquired(EntityLock),
    /// Lock would block - another session holds incompatible lock
    WouldBlock {
        /// Lock ID that's blocking
        blocking_lock_id: LockId,
        /// Session holding the blocking lock
        holder_session: SessionId,
    },
    /// Deadlock detected
    Deadlock {
        /// Sessions involved in the deadlock cycle
        cycle: Vec<SessionId>,
    },
    /// Lock request timed out
    Timeout,
}

/// Lock statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockStatistics {
    /// Total locks acquired
    pub total_acquired: u64,
    /// Total locks released
    pub total_released: u64,
    /// Total lock conflicts
    pub total_conflicts: u64,
    /// Total deadlocks detected
    pub total_deadlocks: u64,
    /// Total timeouts
    pub total_timeouts: u64,
    /// Current active locks
    pub active_locks: usize,
    /// Current waiting sessions
    pub waiting_sessions: usize,
}

// ==============================================================================
// Wait-For Graph for Deadlock Detection
// ==============================================================================

/// Wait-for graph for tracking lock dependencies between sessions
#[derive(Debug, Default)]
pub struct WaitForGraph {
    /// Edges: waiter -> set of holders
    edges: HashMap<SessionId, HashSet<SessionId>>,
    /// Reverse edges for efficient removal
    reverse_edges: HashMap<SessionId, HashSet<SessionId>>,
}

impl WaitForGraph {
    /// Create a new wait-for graph
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add a wait edge (waiter is waiting for holder)
    pub fn add_wait_edge(&mut self, waiter: SessionId, holder: SessionId) {
        self.edges
            .entry(waiter.clone())
            .or_insert_with(HashSet::new)
            .insert(holder.clone());

        self.reverse_edges
            .entry(holder)
            .or_insert_with(HashSet::new)
            .insert(waiter);
    }

    /// Remove a wait edge
    pub fn remove_wait_edge(&mut self, waiter: &SessionId, holder: &SessionId) {
        if let Some(holders) = self.edges.get_mut(waiter) {
            holders.remove(holder);
            if holders.is_empty() {
                self.edges.remove(waiter);
            }
        }

        if let Some(waiters) = self.reverse_edges.get_mut(holder) {
            waiters.remove(waiter);
            if waiters.is_empty() {
                self.reverse_edges.remove(holder);
            }
        }
    }

    /// Remove all edges for a session (both as waiter and holder)
    pub fn remove_session(&mut self, session: &SessionId) {
        // Remove as waiter
        if let Some(holders) = self.edges.remove(session) {
            for holder in holders {
                if let Some(waiters) = self.reverse_edges.get_mut(&holder) {
                    waiters.remove(session);
                    if waiters.is_empty() {
                        self.reverse_edges.remove(&holder);
                    }
                }
            }
        }

        // Remove as holder
        if let Some(waiters) = self.reverse_edges.remove(session) {
            for waiter in waiters {
                if let Some(holders) = self.edges.get_mut(&waiter) {
                    holders.remove(session);
                    if holders.is_empty() {
                        self.edges.remove(&waiter);
                    }
                }
            }
        }
    }

    /// Detect cycles using DFS-based algorithm
    pub fn detect_cycle(&self) -> Option<Vec<SessionId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for session in self.edges.keys() {
            if !visited.contains(session) {
                if let Some(cycle) = self.dfs_detect_cycle(
                    session,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycle(
        &self,
        session: &SessionId,
        visited: &mut HashSet<SessionId>,
        rec_stack: &mut HashSet<SessionId>,
        path: &mut Vec<SessionId>,
    ) -> Option<Vec<SessionId>> {
        visited.insert(session.clone());
        rec_stack.insert(session.clone());
        path.push(session.clone());

        if let Some(neighbors) = self.edges.get(session) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_detect_cycle(
                        neighbor,
                        visited,
                        rec_stack,
                        path,
                    ) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found cycle - extract it from path
                    let cycle_start = path.iter().position(|s| s == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.remove(session);
        path.pop();
        None
    }

    /// Get all sessions waiting for a specific session
    pub fn get_waiters(&self, holder: &SessionId) -> HashSet<SessionId> {
        self.reverse_edges
            .get(holder)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if there's a path from source to target
    pub fn has_path(&self, source: &SessionId, target: &SessionId) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(source.clone());
        visited.insert(source.clone());

        while let Some(current) = queue.pop_front() {
            if &current == target {
                return true;
            }

            if let Some(neighbors) = self.edges.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        false
    }

    /// Get statistics about the wait-for graph
    pub fn statistics(&self) -> WaitGraphStatistics {
        WaitGraphStatistics {
            total_edges: self.edges.values().map(|s| s.len()).sum(),
            waiting_sessions: self.edges.len(),
            max_wait_depth: self.calculate_max_depth(),
        }
    }

    /// Calculate maximum depth of wait chains
    fn calculate_max_depth(&self) -> usize {
        let mut max_depth = 0;

        for session in self.edges.keys() {
            let depth = self.calculate_depth(session, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// Calculate depth of wait chain from a session
    fn calculate_depth(&self, session: &SessionId, visited: &mut HashSet<SessionId>) -> usize {
        if visited.contains(session) {
            return 0; // Prevent infinite recursion on cycles
        }

        visited.insert(session.clone());

        let max_child_depth = self
            .edges
            .get(session)
            .map(|neighbors| {
                neighbors
                    .iter()
                    .map(|n| self.calculate_depth(n, visited))
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        visited.remove(session);
        1 + max_child_depth
    }
}

/// Wait-for graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitGraphStatistics {
    pub total_edges: usize,
    pub waiting_sessions: usize,
    pub max_wait_depth: usize,
}

// ==============================================================================
// Deadlock Detector
// ==============================================================================

/// Deadlock detection and resolution
pub struct DeadlockDetector {
    /// Wait-for graph
    graph: Arc<RwLock<WaitForGraph>>,
    /// Detection interval
    detection_interval: Duration,
    /// Statistics
    stats: Arc<RwLock<DeadlockStatistics>>,
}

/// Deadlock information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deadlock {
    /// Sessions in the deadlock cycle
    pub cycle: Vec<SessionId>,
    /// When the deadlock was detected
    pub detected_at: DateTime<Utc>,
}

/// Deadlock detection statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeadlockStatistics {
    pub total_detected: u64,
    pub total_resolved: u64,
    pub last_detection: Option<DateTime<Utc>>,
}

impl DeadlockDetector {
    /// Create a new deadlock detector
    pub fn new(detection_interval: Duration) -> Self {
        Self {
            graph: Arc::new(RwLock::new(WaitForGraph::new())),
            detection_interval,
            stats: Arc::new(RwLock::new(DeadlockStatistics::default())),
        }
    }

    /// Add a wait edge to the graph
    pub fn add_wait(&self, waiter: SessionId, holder: SessionId) {
        let mut graph = self.graph.write();
        graph.add_wait_edge(waiter, holder);
    }

    /// Remove a wait edge from the graph
    pub fn remove_wait(&self, waiter: &SessionId, holder: &SessionId) {
        let mut graph = self.graph.write();
        graph.remove_wait_edge(waiter, holder);
    }

    /// Remove all waits for a session
    pub fn remove_session(&self, session: &SessionId) {
        let mut graph = self.graph.write();
        graph.remove_session(session);
    }

    /// Check for deadlocks
    pub fn check_deadlock(&self) -> Option<Deadlock> {
        let graph = self.graph.read();

        if let Some(cycle) = graph.detect_cycle() {
            let mut stats = self.stats.write();
            stats.total_detected += 1;
            stats.last_detection = Some(Utc::now());

            info!("Deadlock detected involving {} sessions: {:?}", cycle.len(), cycle);

            Some(Deadlock {
                cycle,
                detected_at: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Resolve deadlock by selecting victim session
    pub fn select_victim(&self, deadlock: &Deadlock) -> SessionId {
        // Simple strategy: abort the youngest transaction (last in cycle)
        // More sophisticated strategies could consider:
        // - Work done (abort least work)
        // - Priority levels
        // - Resource usage

        deadlock.cycle.last()
            .cloned()
            .unwrap_or_else(|| deadlock.cycle[0].clone())
    }

    /// Get deadlock statistics
    pub fn statistics(&self) -> DeadlockStatistics {
        self.stats.read().clone()
    }

    /// Get wait-for graph statistics
    pub fn graph_statistics(&self) -> WaitGraphStatistics {
        let graph = self.graph.read();
        graph.statistics()
    }

    /// Get detection interval
    pub fn detection_interval(&self) -> Duration {
        self.detection_interval
    }
}

// ==============================================================================
// Lock Compatibility Matrix
// ==============================================================================

/// Check if two lock types are compatible
pub fn is_compatible(held: &LockType, requested: &LockType) -> bool {
    // Lock compatibility matrix:
    //              | Read | Write | Intent |
    // Read         |  ✓   |   ✗   |   ✓    |
    // Write        |  ✗   |   ✗   |   ✗    |
    // Intent       |  ✓   |   ✗   |   ✓    |

    match (held, requested) {
        (LockType::Read, LockType::Read) => true,
        (LockType::Read, LockType::Intent) => true,
        (LockType::Intent, LockType::Read) => true,
        (LockType::Intent, LockType::Intent) => true,
        _ => false,
    }
}

/// Check if a lock request is compatible with existing locks
pub fn check_compatibility(
    existing_locks: &[&EntityLock],
    requested_type: &LockType,
    requesting_session: &SessionId,
) -> bool {
    for lock in existing_locks {
        // Same session can hold multiple locks
        if &lock.holder_session == requesting_session {
            continue;
        }

        // Check compatibility
        if !is_compatible(&lock.lock_type, requested_type) {
            return false;
        }
    }

    true
}

// ==============================================================================
// Lock Manager State
// ==============================================================================

/// Internal lock manager state
#[derive(Debug)]
struct LockManagerState {
    /// All active locks: lock_id -> Arc<EntityLock>
    /// Using Arc to avoid clones on lookups
    locks: DashMap<LockId, Arc<EntityLock>>,
    /// Locks by entity: entity_id -> set of lock_ids
    locks_by_entity: DashMap<String, HashSet<LockId>>,
    /// Locks by session: session_id -> set of lock_ids
    locks_by_session: DashMap<SessionId, HashSet<LockId>>,
    /// Lock ID counter
    lock_counter: AtomicU64,
    /// Statistics
    stats: Arc<RwLock<LockStatistics>>,
}

impl LockManagerState {
    fn new() -> Self {
        Self {
            locks: DashMap::new(),
            locks_by_entity: DashMap::new(),
            locks_by_session: DashMap::new(),
            lock_counter: AtomicU64::new(0),
            stats: Arc::new(RwLock::new(LockStatistics::default())),
        }
    }

    fn generate_lock_id(&self) -> LockId {
        let count = self.lock_counter.fetch_add(1, Ordering::SeqCst);
        format!("lock_{}", count)
    }

    fn add_lock(&self, lock: EntityLock) {
        let lock_id = lock.lock_id.clone();
        let entity_id = lock.entity_id.clone();
        let session_id = lock.holder_session.clone();

        // Add to main locks map using Arc to avoid clones
        let lock_arc = Arc::new(lock);
        self.locks.insert(lock_id.clone(), lock_arc);

        // Add to entity index
        self.locks_by_entity
            .entry(entity_id)
            .or_insert_with(HashSet::new)
            .insert(lock_id.clone());

        // Add to session index
        self.locks_by_session
            .entry(session_id)
            .or_insert_with(HashSet::new)
            .insert(lock_id);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_acquired += 1;
        stats.active_locks = self.locks.len();
    }

    fn remove_lock(&self, lock_id: &LockId) -> Option<Arc<EntityLock>> {
        if let Some((_, lock_arc)) = self.locks.remove(lock_id) {
            // Remove from entity index
            if let Some(mut entity_locks) = self.locks_by_entity.get_mut(&lock_arc.entity_id) {
                entity_locks.remove(lock_id);
            }

            // Remove from session index
            if let Some(mut session_locks) = self.locks_by_session.get_mut(&lock_arc.holder_session) {
                session_locks.remove(lock_id);
            }

            // Update stats
            let mut stats = self.stats.write();
            stats.total_released += 1;
            stats.active_locks = self.locks.len();

            Some(lock_arc)
        } else {
            None
        }
    }

    fn get_entity_locks(&self, entity_id: &str) -> Vec<Arc<EntityLock>> {
        if let Some(lock_ids) = self.locks_by_entity.get(entity_id) {
            lock_ids
                .iter()
                .filter_map(|lock_id| self.locks.get(lock_id).map(|l| l.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn get_session_locks(&self, session_id: &SessionId) -> Vec<Arc<EntityLock>> {
        if let Some(lock_ids) = self.locks_by_session.get(session_id) {
            lock_ids
                .iter()
                .filter_map(|lock_id| self.locks.get(lock_id).map(|l| l.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn find_expired_locks(&self, now: DateTime<Utc>) -> Vec<Arc<EntityLock>> {
        self.locks
            .iter()
            .filter(|entry| entry.value().expires_at < now)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

// ==============================================================================
// Lock Manager
// ==============================================================================

/// Lock manager for entity locking with deadlock detection
pub struct LockManager {
    /// Internal state
    state: Arc<LockManagerState>,
    /// Deadlock detector
    deadlock_detector: Arc<DeadlockDetector>,
    /// Default lock timeout
    default_timeout: Duration,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new(default_timeout: Duration, detection_interval: Duration) -> Self {
        Self {
            state: Arc::new(LockManagerState::new()),
            deadlock_detector: Arc::new(DeadlockDetector::new(detection_interval)),
            default_timeout,
        }
    }

    /// Try to acquire a lock (non-blocking)
    pub fn try_acquire_lock(
        &self,
        session: &SessionId,
        request: LockRequest,
    ) -> Result<LockAcquisition> {
        let entity_locks = self.state.get_entity_locks(&request.entity_id);
        let entity_lock_refs: Vec<&EntityLock> = entity_locks.iter().map(|arc| arc.as_ref()).collect();

        // Check compatibility
        if !check_compatibility(&entity_lock_refs, &request.lock_type, session) {
            // Find blocking lock
            let blocking_lock = entity_locks
                .iter()
                .find(|lock| {
                    &lock.holder_session != session
                        && !is_compatible(&lock.lock_type, &request.lock_type)
                })
                .unwrap();

            // Add wait edge
            self.deadlock_detector
                .add_wait(session.clone(), blocking_lock.holder_session.clone());

            // Check for deadlock
            if let Some(deadlock) = self.deadlock_detector.check_deadlock() {
                self.deadlock_detector.remove_wait(session, &blocking_lock.holder_session);

                let mut stats = self.state.stats.write();
                stats.total_deadlocks += 1;

                return Ok(LockAcquisition::Deadlock {
                    cycle: deadlock.cycle,
                });
            }

            return Ok(LockAcquisition::WouldBlock {
                blocking_lock_id: blocking_lock.lock_id.clone(),
                holder_session: blocking_lock.holder_session.clone(),
            });
        }

        // Acquire lock
        let lock_id = self.state.generate_lock_id();
        let now = Utc::now();
        let timeout_duration = if request.timeout.as_secs() > 0 {
            request.timeout
        } else {
            self.default_timeout
        };

        let expires_at = now
            + ChronoDuration::from_std(timeout_duration)
                .map_err(|e| CortexError::InvalidInput(e.to_string()))?;

        let lock = EntityLock {
            lock_id,
            entity_id: request.entity_id,
            entity_type: request.entity_type,
            lock_type: request.lock_type,
            holder_session: session.clone(),
            acquired_at: now,
            expires_at,
            metadata: request.metadata.unwrap_or_default(),
        };

        self.state.add_lock(lock.clone());

        debug!(
            "Lock acquired: {} on entity {} by session {}",
            lock.lock_id, lock.entity_id, session
        );

        Ok(LockAcquisition::Acquired(lock))
    }

    /// Acquire lock with timeout (blocking)
    pub async fn acquire_lock(
        &self,
        session: &SessionId,
        request: LockRequest,
    ) -> Result<EntityLock> {
        let timeout_duration = if request.timeout.as_secs() > 0 {
            request.timeout
        } else {
            self.default_timeout
        };

        let start = std::time::Instant::now();

        loop {
            match self.try_acquire_lock(session, request.clone())? {
                LockAcquisition::Acquired(lock) => {
                    return Ok(lock);
                }
                LockAcquisition::WouldBlock { .. } => {
                    // Check timeout
                    if start.elapsed() >= timeout_duration {
                        let mut stats = self.state.stats.write();
                        stats.total_timeouts += 1;

                        return Err(CortexError::Timeout(format!(
                            "Lock acquisition timed out after {:?}",
                            timeout_duration
                        )));
                    }

                    // Wait a bit before retrying
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                LockAcquisition::Deadlock { cycle } => {
                    return Err(CortexError::Deadlock(format!(
                        "Deadlock detected involving sessions: {:?}",
                        cycle
                    )));
                }
                LockAcquisition::Timeout => {
                    return Err(CortexError::Timeout("Lock acquisition timed out".to_string()));
                }
            }
        }
    }

    /// Release a lock
    pub fn release_lock(&self, lock_id: &LockId) -> Result<()> {
        if let Some(lock) = self.state.remove_lock(lock_id) {
            // Remove from wait-for graph
            self.deadlock_detector.remove_session(&lock.holder_session);

            debug!(
                "Lock released: {} on entity {} by session {}",
                lock.lock_id, lock.entity_id, lock.holder_session
            );

            Ok(())
        } else {
            Err(CortexError::not_found("lock", lock_id))
        }
    }

    /// Release all locks for a session
    pub fn release_session_locks(&self, session: &SessionId) -> Result<usize> {
        let locks = self.state.get_session_locks(session);
        let count = locks.len();

        for lock in locks {
            self.state.remove_lock(&lock.lock_id);
        }

        // Remove from wait-for graph
        self.deadlock_detector.remove_session(session);

        info!("Released {} locks for session {}", count, session);

        Ok(count)
    }

    /// List all active locks
    pub fn list_locks(&self) -> Result<Vec<Arc<EntityLock>>> {
        Ok(self
            .state
            .locks
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// List locks for a specific session
    pub fn list_session_locks(&self, session: &SessionId) -> Result<Vec<Arc<EntityLock>>> {
        Ok(self.state.get_session_locks(session))
    }

    /// List locks for a specific entity
    pub fn list_entity_locks(&self, entity_id: &str) -> Result<Vec<Arc<EntityLock>>> {
        Ok(self.state.get_entity_locks(entity_id))
    }

    /// Check if an entity is locked
    pub fn is_locked(&self, entity_id: &str) -> Result<bool> {
        Ok(!self.state.get_entity_locks(entity_id).is_empty())
    }

    /// Get a specific lock by ID
    pub fn get_lock(&self, lock_id: &LockId) -> Result<Arc<EntityLock>> {
        self.state
            .locks
            .get(lock_id)
            .map(|l| l.clone())
            .ok_or_else(|| CortexError::not_found("lock", lock_id))
    }

    /// Find expired locks
    pub fn find_expired_locks(&self, now: DateTime<Utc>) -> Vec<Arc<EntityLock>> {
        self.state.find_expired_locks(now)
    }

    /// Cleanup expired locks
    pub async fn cleanup_expired_locks(&self) -> usize {
        let now = Utc::now();
        let expired = self.find_expired_locks(now);
        let count = expired.len();

        for lock in expired {
            if let Err(e) = self.release_lock(&lock.lock_id) {
                warn!("Failed to release expired lock {}: {}", lock.lock_id, e);
            } else {
                info!("Expired lock released: {} (expired at {})", lock.lock_id, lock.expires_at);
            }
        }

        count
    }

    /// Run cleanup loop (background task)
    pub async fn run_cleanup_loop(&self, interval: Duration) {
        info!("Starting lock cleanup loop with interval {:?}", interval);

        loop {
            tokio::time::sleep(interval).await;

            let cleaned = self.cleanup_expired_locks().await;
            if cleaned > 0 {
                debug!("Cleaned up {} expired locks", cleaned);
            }
        }
    }

    /// Run deadlock detection loop (background task)
    pub async fn run_deadlock_detection_loop(&self) {
        let interval = self.deadlock_detector.detection_interval();
        info!("Starting deadlock detection loop with interval {:?}", interval);

        loop {
            tokio::time::sleep(interval).await;

            if let Some(deadlock) = self.deadlock_detector.check_deadlock() {
                warn!(
                    "Deadlock detected involving {} sessions: {:?}",
                    deadlock.cycle.len(),
                    deadlock.cycle
                );

                // Select victim and abort
                let victim = self.deadlock_detector.select_victim(&deadlock);
                info!("Aborting session {} to resolve deadlock", victim);

                if let Err(e) = self.release_session_locks(&victim) {
                    warn!("Failed to abort victim session {}: {}", victim, e);
                }
            }
        }
    }

    /// Get lock statistics
    pub fn statistics(&self) -> LockStatistics {
        let mut stats = self.state.stats.read().clone();
        stats.active_locks = self.state.locks.len();
        stats
    }

    /// Get deadlock statistics
    pub fn deadlock_statistics(&self) -> DeadlockStatistics {
        self.deadlock_detector.statistics()
    }

    /// Get wait-for graph statistics
    pub fn wait_graph_statistics(&self) -> WaitGraphStatistics {
        self.deadlock_detector.graph_statistics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_compatibility() {
        // Read-Read: compatible
        assert!(is_compatible(&LockType::Read, &LockType::Read));

        // Read-Write: incompatible
        assert!(!is_compatible(&LockType::Read, &LockType::Write));

        // Write-Read: incompatible
        assert!(!is_compatible(&LockType::Write, &LockType::Read));

        // Write-Write: incompatible
        assert!(!is_compatible(&LockType::Write, &LockType::Write));

        // Intent-Intent: compatible
        assert!(is_compatible(&LockType::Intent, &LockType::Intent));

        // Read-Intent: compatible
        assert!(is_compatible(&LockType::Read, &LockType::Intent));

        // Intent-Read: compatible
        assert!(is_compatible(&LockType::Intent, &LockType::Read));

        // Write-Intent: incompatible
        assert!(!is_compatible(&LockType::Write, &LockType::Intent));
    }

    #[test]
    fn test_wait_for_graph_cycle_detection() {
        let mut graph = WaitForGraph::new();

        // No cycle
        graph.add_wait_edge("A".to_string(), "B".to_string());
        graph.add_wait_edge("B".to_string(), "C".to_string());
        assert!(graph.detect_cycle().is_none());

        // Create cycle: A -> B -> C -> A
        graph.add_wait_edge("C".to_string(), "A".to_string());
        let cycle = graph.detect_cycle();
        assert!(cycle.is_some());
        assert_eq!(cycle.unwrap().len(), 3);
    }

    #[test]
    fn test_wait_for_graph_path_detection() {
        let mut graph = WaitForGraph::new();

        graph.add_wait_edge("A".to_string(), "B".to_string());
        graph.add_wait_edge("B".to_string(), "C".to_string());

        assert!(graph.has_path(&"A".to_string(), &"C".to_string()));
        assert!(!graph.has_path(&"C".to_string(), &"A".to_string()));
    }

    #[test]
    fn test_lock_manager_basic() {
        let manager = LockManager::new(
            Duration::from_secs(300),
            Duration::from_millis(100),
        );

        let session = "session1".to_string();
        let request = LockRequest {
            entity_id: "entity1".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };

        // Acquire lock
        let result = manager.try_acquire_lock(&session, request).unwrap();
        match result {
            LockAcquisition::Acquired(lock) => {
                assert_eq!(lock.entity_id, "entity1");
                assert_eq!(lock.lock_type, LockType::Write);
            }
            _ => panic!("Expected lock to be acquired"),
        }

        // Check if locked
        assert!(manager.is_locked("entity1").unwrap());

        // List locks
        let locks = manager.list_locks().unwrap();
        assert_eq!(locks.len(), 1);
    }

    #[test]
    fn test_lock_manager_conflict() {
        let manager = LockManager::new(
            Duration::from_secs(300),
            Duration::from_millis(100),
        );

        let session1 = "session1".to_string();
        let session2 = "session2".to_string();

        // Session 1 acquires write lock
        let request1 = LockRequest {
            entity_id: "entity1".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };

        manager.try_acquire_lock(&session1, request1).unwrap();

        // Session 2 tries to acquire read lock (should block)
        let request2 = LockRequest {
            entity_id: "entity1".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Read,
            timeout: Duration::from_secs(1),
            metadata: None,
        };

        let result = manager.try_acquire_lock(&session2, request2).unwrap();
        match result {
            LockAcquisition::WouldBlock { holder_session, .. } => {
                assert_eq!(holder_session, session1);
            }
            _ => panic!("Expected lock to block"),
        }
    }

    #[test]
    fn test_lock_manager_shared_locks() {
        let manager = LockManager::new(
            Duration::from_secs(300),
            Duration::from_millis(100),
        );

        let session1 = "session1".to_string();
        let session2 = "session2".to_string();

        // Session 1 acquires read lock
        let request1 = LockRequest {
            entity_id: "entity1".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Read,
            timeout: Duration::from_secs(1),
            metadata: None,
        };

        manager.try_acquire_lock(&session1, request1).unwrap();

        // Session 2 acquires read lock (should succeed)
        let request2 = LockRequest {
            entity_id: "entity1".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Read,
            timeout: Duration::from_secs(1),
            metadata: None,
        };

        let result = manager.try_acquire_lock(&session2, request2).unwrap();
        match result {
            LockAcquisition::Acquired(_) => {}
            _ => panic!("Expected lock to be acquired"),
        }

        // Should have 2 locks
        let locks = manager.list_locks().unwrap();
        assert_eq!(locks.len(), 2);
    }
}
