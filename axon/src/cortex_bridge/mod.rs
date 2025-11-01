//! Cortex Bridge - Integration with Cortex Data Layer
//!
//! This module provides a comprehensive bridge between Axon's multi-agent orchestration
//! and Cortex's persistent memory and data layer. It handles sessions, episodic memory,
//! semantic search, and distributed coordination.
//!
//! # Architecture
//!
//! The CortexBridge acts as the central nervous system for all agents, providing:
//! - **Session Isolation**: Each agent works in an isolated session
//! - **Episodic Memory**: Shared learning across all agents
//! - **Semantic Search**: Context-aware code discovery
//! - **Distributed Locks**: Safe coordination between agents
//!
//! # Example
//!
//! ```no_run
//! use axon::cortex_bridge::{CortexBridge, CortexConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = CortexConfig::default();
//!     let bridge = CortexBridge::new(config).await?;
//!
//!     // Check health
//!     let health = bridge.health_check().await?;
//!     println!("Cortex status: {}", health.status);
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// Module declarations
pub mod client;
pub mod locks;
pub mod memory;
pub mod models;
pub mod search;
pub mod session;
pub mod working_memory;
pub mod consolidation;

// Re-export key types
pub use client::{CortexConfig, CortexError, Result};
pub use locks::{LockGuard, LockManager};
pub use memory::MemoryManager;
pub use models::*;
pub use search::SearchManager;
pub use session::SessionManager;
pub use working_memory::WorkingMemoryManager;
pub use consolidation::ConsolidationManager;

use client::CortexClient;

/// Main CortexBridge structure
///
/// Provides high-level API for all Cortex operations with connection pooling,
/// caching, and automatic retry logic.
pub struct CortexBridge {
    /// HTTP client for Cortex API
    client: Arc<CortexClient>,

    /// Session manager
    session_manager: SessionManager,

    /// Memory manager
    memory_manager: MemoryManager,

    /// Search manager
    search_manager: SearchManager,

    /// Lock manager
    lock_manager: LockManager,

    /// Working memory manager
    working_memory_manager: WorkingMemoryManager,

    /// Consolidation manager
    consolidation_manager: ConsolidationManager,

    /// Active sessions tracking
    active_sessions: Arc<RwLock<HashMap<AgentId, SessionId>>>,

    /// Configuration
    config: CortexConfig,
}

impl CortexBridge {
    /// Create a new CortexBridge and connect to Cortex
    ///
    /// This will perform a health check to ensure Cortex is reachable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use axon::cortex_bridge::{CortexBridge, CortexConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = CortexConfig {
    ///         base_url: "http://localhost:8080".to_string(),
    ///         ..Default::default()
    ///     };
    ///
    ///     let bridge = CortexBridge::new(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: CortexConfig) -> Result<Self> {
        info!("Initializing CortexBridge with base_url: {}", config.base_url);

        let client = Arc::new(CortexClient::new(config.clone())?);

        // Verify connection
        client.health_check().await?;
        info!("Cortex health check passed");

        // Create managers
        let session_manager = SessionManager::new(client.as_ref().clone());
        let memory_manager = MemoryManager::new(client.as_ref().clone());
        let search_manager = SearchManager::new(client.as_ref().clone());
        let lock_manager = LockManager::new(client.as_ref().clone());
        let working_memory_manager = WorkingMemoryManager::new(client.as_ref().clone());
        let consolidation_manager = ConsolidationManager::new(client.as_ref().clone());

        Ok(Self {
            client,
            session_manager,
            memory_manager,
            search_manager,
            lock_manager,
            working_memory_manager,
            consolidation_manager,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &CortexConfig {
        &self.config
    }

    // ========================================================================
    // Health & Status
    // ========================================================================

    /// Perform health check on Cortex
    pub async fn health_check(&self) -> Result<HealthStatus> {
        self.client.health_check().await
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new session for an agent
    pub async fn create_session(
        &self,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        scope: SessionScope,
    ) -> Result<SessionId> {
        let session_id = self
            .session_manager
            .create_session(agent_id.clone(), workspace_id, scope)
            .await?;

        // Track active session
        self.active_sessions
            .write()
            .await
            .insert(agent_id, session_id.clone());

        Ok(session_id)
    }

    /// Get session status
    pub async fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus> {
        self.session_manager.get_session_status(session_id).await
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &SessionId, agent_id: &AgentId) -> Result<()> {
        self.session_manager.close_session(session_id).await?;

        // Remove from tracking
        self.active_sessions.write().await.remove(agent_id);

        Ok(())
    }

    /// Get active session for an agent
    pub async fn get_agent_session(&self, agent_id: &AgentId) -> Option<SessionId> {
        self.active_sessions.read().await.get(agent_id).cloned()
    }

    /// Close all sessions for an agent
    pub async fn close_agent_sessions(&self, agent_id: &AgentId) -> Result<()> {
        if let Some(session_id) = self.active_sessions.write().await.remove(agent_id) {
            self.session_manager.close_session(&session_id).await?;
        }
        Ok(())
    }

    // ========================================================================
    // File Operations
    // ========================================================================

    /// Read a file from a session
    pub async fn read_file(&self, session_id: &SessionId, path: &str) -> Result<String> {
        self.session_manager.read_file(session_id, path).await
    }

    /// Write a file to a session
    pub async fn write_file(
        &self,
        session_id: &SessionId,
        path: &str,
        content: &str,
    ) -> Result<()> {
        self.session_manager
            .write_file(session_id, path, content)
            .await
    }

    /// List files in a session
    pub async fn list_files(&self, session_id: &SessionId, path: &str) -> Result<Vec<FileInfo>> {
        self.session_manager.list_files(session_id, path).await
    }

    // ========================================================================
    // Session Merging
    // ========================================================================

    /// Merge session changes back to workspace
    pub async fn merge_session(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeReport> {
        self.session_manager
            .merge_session(session_id, strategy)
            .await
    }

    // ========================================================================
    // Episodic Memory
    // ========================================================================

    /// Store an episode for learning
    pub async fn store_episode(&self, episode: Episode) -> Result<EpisodeId> {
        self.memory_manager.store_episode(episode).await
    }

    /// Search for similar episodes
    pub async fn search_episodes(&self, query: &str, limit: usize) -> Result<Vec<Episode>> {
        self.memory_manager.search_episodes(query, limit).await
    }

    /// Get a specific episode
    pub async fn get_episode(&self, episode_id: &EpisodeId) -> Result<Episode> {
        self.memory_manager.get_episode(episode_id).await
    }

    // ========================================================================
    // Patterns
    // ========================================================================

    /// Get learned patterns
    pub async fn get_patterns(&self) -> Result<Vec<Pattern>> {
        self.memory_manager.get_patterns().await
    }

    /// Store a new pattern
    pub async fn store_pattern(&self, pattern: Pattern) -> Result<String> {
        self.memory_manager.store_pattern(pattern).await
    }

    /// Get a specific pattern
    pub async fn get_pattern(&self, pattern_id: &str) -> Result<Pattern> {
        self.memory_manager.get_pattern(pattern_id).await
    }

    /// Update pattern statistics
    pub async fn update_pattern_stats(
        &self,
        pattern_id: &str,
        success: bool,
        improvement: serde_json::Value,
    ) -> Result<()> {
        self.memory_manager
            .update_pattern_stats(pattern_id, success, improvement)
            .await
    }

    // ========================================================================
    // Semantic Search
    // ========================================================================

    /// Perform semantic code search
    pub async fn semantic_search(
        &self,
        query: &str,
        workspace_id: &WorkspaceId,
        filters: SearchFilters,
    ) -> Result<Vec<CodeSearchResult>> {
        self.search_manager
            .semantic_search(query, workspace_id, filters)
            .await
    }

    /// Get code units from workspace
    pub async fn get_code_units(
        &self,
        workspace_id: &WorkspaceId,
        filters: UnitFilters,
    ) -> Result<Vec<CodeUnit>> {
        self.search_manager
            .get_code_units(workspace_id, filters)
            .await
    }

    /// Get a specific code unit
    pub async fn get_code_unit(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<CodeUnit> {
        self.search_manager
            .get_code_unit(workspace_id, unit_id)
            .await
    }

    /// Find references to a code unit
    pub async fn find_references(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<Vec<CodeSearchResult>> {
        self.search_manager
            .find_references(workspace_id, unit_id)
            .await
    }

    /// Query the knowledge graph
    pub async fn query_graph(
        &self,
        query: &str,
        parameters: serde_json::Value,
    ) -> Result<search::GraphQueryResponse> {
        self.search_manager.query_graph(query, parameters).await
    }

    // ========================================================================
    // Lock Management
    // ========================================================================

    /// Acquire a lock on an entity
    pub async fn acquire_lock(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<LockId> {
        self.lock_manager
            .acquire_lock(entity_id, lock_type, agent_id, session_id)
            .await
    }

    /// Try to acquire a lock without waiting
    pub async fn try_acquire_lock(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<Option<LockId>> {
        self.lock_manager
            .try_acquire_lock(entity_id, lock_type, agent_id, session_id)
            .await
    }

    /// Release a lock
    pub async fn release_lock(&self, lock_id: &LockId) -> Result<()> {
        self.lock_manager.release_lock(lock_id).await
    }

    /// Get lock status
    pub async fn get_lock_status(&self, lock_id: &LockId) -> Result<locks::LockStatus> {
        self.lock_manager.get_lock_status(lock_id).await
    }

    /// Check if an entity is locked
    pub async fn is_locked(&self, entity_id: &str) -> Result<bool> {
        self.lock_manager.is_locked(entity_id).await
    }

    /// Release all locks for an agent
    pub async fn release_agent_locks(&self, agent_id: &AgentId) -> Result<u32> {
        self.lock_manager.release_agent_locks(agent_id).await
    }

    /// Release all locks for a session
    pub async fn release_session_locks(&self, session_id: &SessionId) -> Result<u32> {
        self.lock_manager.release_session_locks(session_id).await
    }

    // ========================================================================
    // Working Memory Operations
    // ========================================================================

    /// Add an item to working memory
    pub async fn add_to_working_memory(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
        item: WorkingMemoryItem,
    ) -> Result<()> {
        self.working_memory_manager
            .add_item(agent_id, session_id, item)
            .await
    }

    /// Get working memory for an agent session
    pub async fn get_working_memory(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<Vec<WorkingMemoryItem>> {
        self.working_memory_manager
            .get_items(agent_id, session_id)
            .await
    }

    /// Clear working memory for a session
    pub async fn clear_working_memory(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<()> {
        self.working_memory_manager
            .clear_session(agent_id, session_id)
            .await
    }

    /// Get working memory statistics
    pub async fn get_working_memory_stats(
        &self,
        agent_id: &AgentId,
    ) -> Result<WorkingMemoryStats> {
        self.working_memory_manager.get_stats(agent_id).await
    }

    // ========================================================================
    // Memory Consolidation
    // ========================================================================

    /// Trigger memory consolidation for an agent session
    ///
    /// This consolidates working memory into long-term episodic/semantic memory
    pub async fn consolidate_memory(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<ConsolidationReport> {
        self.consolidation_manager
            .consolidate_session(agent_id, session_id)
            .await
    }

    /// Trigger pattern extraction from episodes
    ///
    /// This analyzes recent episodes to extract reusable patterns
    pub async fn extract_patterns(
        &self,
        workspace_id: &WorkspaceId,
        min_occurrences: u32,
    ) -> Result<Vec<Pattern>> {
        self.consolidation_manager
            .extract_patterns(workspace_id, min_occurrences)
            .await
    }

    /// Perform dream-like consolidation (offline learning)
    ///
    /// This runs advanced pattern recognition and memory optimization
    pub async fn dream_consolidation(&self) -> Result<DreamReport> {
        self.consolidation_manager.dream().await
    }

    // ========================================================================
    // Collaborative Memory
    // ========================================================================

    /// Share episode with other agents
    pub async fn share_episode(
        &self,
        episode_id: &EpisodeId,
        target_agents: Vec<AgentId>,
    ) -> Result<()> {
        self.memory_manager
            .share_episode(episode_id, target_agents)
            .await
    }

    /// Get shared episodes from other agents
    pub async fn get_shared_episodes(
        &self,
        agent_id: &AgentId,
        limit: usize,
    ) -> Result<Vec<Episode>> {
        self.memory_manager
            .get_shared_episodes(agent_id, limit)
            .await
    }

    /// Get collaborative insights (patterns shared across agents)
    pub async fn get_collaborative_insights(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<CollaborativeInsight>> {
        self.memory_manager
            .get_collaborative_insights(workspace_id)
            .await
    }

    // ========================================================================
    // Advanced Pattern Operations
    // ========================================================================

    /// Search for similar patterns
    pub async fn search_patterns(
        &self,
        query: &str,
        pattern_type: Option<PatternType>,
        limit: usize,
    ) -> Result<Vec<Pattern>> {
        self.memory_manager
            .search_patterns(query, pattern_type, limit)
            .await
    }

    /// Get pattern evolution history
    pub async fn get_pattern_history(&self, pattern_id: &str) -> Result<Vec<PatternVersion>> {
        self.memory_manager.get_pattern_history(pattern_id).await
    }

    /// Apply a pattern and record the outcome
    pub async fn apply_pattern(
        &self,
        pattern_id: &str,
        context: serde_json::Value,
    ) -> Result<PatternApplication> {
        self.memory_manager
            .apply_pattern(pattern_id, context)
            .await
    }

    // ========================================================================
    // Code Materialization (Bidirectional Sync)
    // ========================================================================

    /// Write code to session with automatic semantic analysis
    pub async fn write_code_with_analysis(
        &self,
        session_id: &SessionId,
        workspace_id: &WorkspaceId,
        path: &str,
        content: &str,
    ) -> Result<CodeAnalysisResult> {
        // Write to session
        self.write_file(session_id, path, content).await?;

        // Trigger semantic analysis
        self.search_manager
            .analyze_and_index(workspace_id, path, content)
            .await
    }

    /// Materialize code from memory representation
    ///
    /// This takes a semantic representation and generates actual code
    pub async fn materialize_code(
        &self,
        session_id: &SessionId,
        representation: CodeRepresentation,
    ) -> Result<MaterializedCode> {
        self.consolidation_manager
            .materialize_code(session_id, representation)
            .await
    }

    /// Sync code changes from session to semantic memory
    pub async fn sync_session_to_memory(
        &self,
        session_id: &SessionId,
        workspace_id: &WorkspaceId,
    ) -> Result<SyncReport> {
        self.consolidation_manager
            .sync_session(session_id, workspace_id)
            .await
    }

    // ========================================================================
    // Cleanup & Shutdown
    // ========================================================================

    /// Close all active sessions and release all locks
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down CortexBridge...");

        let sessions: Vec<(AgentId, SessionId)> = self
            .active_sessions
            .read()
            .await
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (agent_id, session_id) in sessions {
            if let Err(e) = self.close_session(&session_id, &agent_id).await {
                warn!("Failed to close session {}: {}", session_id, e);
            }
        }

        info!("CortexBridge shutdown complete");
        Ok(())
    }
}

impl Drop for CortexBridge {
    fn drop(&mut self) {
        if !self.active_sessions.blocking_read().is_empty() {
            warn!(
                "CortexBridge dropped with {} active sessions. Call shutdown() for clean closure.",
                self.active_sessions.blocking_read().len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cortex_config_default() {
        let config = CortexConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.api_version, "v3");
        assert_eq!(config.request_timeout_secs, 30);
    }

    #[test]
    fn test_agent_id() {
        let id = AgentId::from("test-agent".to_string());
        assert_eq!(id.to_string(), "test-agent");
    }

    #[test]
    fn test_session_id() {
        let id = SessionId::from("test-session".to_string());
        assert_eq!(id.to_string(), "test-session");
    }
}
