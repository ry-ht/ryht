//! Bridge to Cortex memory and learning systems
//!
//! This module provides direct integration with Cortex for:
//! - Episodic memory (development sessions and episodes)
//! - Semantic search (code search and similarity)
//! - Virtual filesystem operations
//! - Storage and sessions
//!
//! **IMPORTANT**: This bridge now uses direct API calls to Cortex modules
//! instead of HTTP requests, eliminating serialization overhead and network latency.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use cortex_types::{SessionId, WorkspaceId};
use cortex_core::types::{CodeUnit, SearchResult};
use cortex_core::error::{CortexError, Result};
use cortex_memory::{EpisodicMemorySystem, CognitiveManager};
use cortex_semantic::{SemanticSearchEngine, SearchFilter as SemanticSearchFilter};
use cortex_vfs::VirtualFileSystem;
use cortex_storage::{ConnectionManager, SessionManager};
use tracing::{debug, info, warn};

/// Session scope configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScope {
    pub paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}

/// Search filters for code search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub workspace_id: Option<String>,
    pub limit: Option<usize>,
    pub types: Option<Vec<String>>,
    pub min_relevance: Option<f32>,
    pub languages: Option<Vec<String>>,
    pub visibility: Option<String>,
}

/// Unit filters for code units
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitFilters {
    pub language: Option<String>,
    pub limit: Option<usize>,
    pub unit_type: Option<String>,
    pub visibility: Option<String>,
}

/// Bridge to Cortex systems with direct API access
///
/// This provides a lightweight wrapper around Cortex modules, enabling
/// agents to access memory, search, and filesystem operations without
/// HTTP overhead. All operations are in-process function calls.
pub struct CortexBridge {
    /// Episodic memory system
    memory: Arc<EpisodicMemorySystem>,

    /// Cognitive manager for memory operations
    cognitive: Arc<CognitiveManager>,

    /// Semantic search engine
    semantic: Option<Arc<SemanticSearchEngine>>,

    /// Virtual filesystem
    vfs: Arc<VirtualFileSystem>,

    /// Session manager
    session_manager: Arc<SessionManager>,

    /// Connection manager for direct storage access
    storage: Arc<ConnectionManager>,
}

impl CortexBridge {
    /// Create a new CortexBridge with direct Cortex module access
    ///
    /// # Arguments
    ///
    /// * `memory` - Episodic memory system
    /// * `cognitive` - Cognitive manager
    /// * `semantic` - Optional semantic search engine
    /// * `vfs` - Virtual filesystem
    /// * `session_manager` - Session manager
    /// * `storage` - Connection manager
    pub fn new(
        memory: Arc<EpisodicMemorySystem>,
        cognitive: Arc<CognitiveManager>,
        semantic: Option<Arc<SemanticSearchEngine>>,
        vfs: Arc<VirtualFileSystem>,
        session_manager: Arc<SessionManager>,
        storage: Arc<ConnectionManager>,
    ) -> Self {
        info!("Creating CortexBridge with direct API access");
        Self {
            memory,
            cognitive,
            semantic,
            vfs,
            session_manager,
            storage,
        }
    }

    /// Check if bridge has semantic search capabilities
    pub fn has_semantic_search(&self) -> bool {
        self.semantic.is_some()
    }

    /// Get reference to episodic memory system
    pub fn memory(&self) -> &Arc<EpisodicMemorySystem> {
        &self.memory
    }

    /// Get reference to cognitive manager
    pub fn cognitive(&self) -> &Arc<CognitiveManager> {
        &self.cognitive
    }

    /// Get reference to virtual filesystem
    pub fn vfs(&self) -> &Arc<VirtualFileSystem> {
        &self.vfs
    }

    /// Get reference to session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get reference to storage
    pub fn storage(&self) -> &Arc<ConnectionManager> {
        &self.storage
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        agent_id: String,
        workspace_id: WorkspaceId,
        scope: SessionScope,
    ) -> Result<SessionId> {
        use cortex_core::id::CortexId;

        info!(agent_id = %agent_id, workspace_id = %workspace_id, "Creating session");

        // Convert WorkspaceId to CortexId
        let workspace_cortex_id = CortexId::from_uuid(workspace_id.as_uuid());

        // Convert scope to session metadata with scope
        let session_scope = cortex_storage::SessionScope {
            paths: scope.paths,
            read_only_paths: scope.read_only_paths,
            units: Vec::new(),
            allow_create: true,
            allow_delete: true,
        };

        let metadata = cortex_storage::SessionMetadata {
            description: format!("Agent {} session", agent_id),
            tags: Vec::new(),
            isolation_level: cortex_storage::IsolationLevel::ReadCommitted,
            scope: session_scope,
            custom: std::collections::HashMap::new(),
        };

        // Create session via session manager
        let session = self
            .session_manager
            .create_session(
                agent_id,
                workspace_cortex_id,
                metadata,
                None, // No TTL
            )
            .await?;

        // Convert CortexId back to SessionId
        Ok(SessionId::from_uuid(*session.id.as_uuid()))
    }

    /// Close a session (abandon it without merging)
    pub async fn close_session(&self, session_id: SessionId) -> Result<()> {
        use cortex_core::id::CortexId;

        debug!(session_id = %session_id, "Closing session");

        // Convert SessionId to CortexId
        let session_cortex_id = CortexId::from_uuid(session_id.as_uuid());

        // Abandon the session without merging
        self.session_manager
            .abandon_session(&session_cortex_id)
            .await?;

        Ok(())
    }

    /// Merge a session back to workspace
    pub async fn merge_session(
        &self,
        session_id: SessionId,
        merge_strategy: String,
    ) -> Result<()> {
        use cortex_core::id::CortexId;

        info!(session_id = %session_id, strategy = %merge_strategy, "Merging session");

        // Convert SessionId to CortexId
        let session_cortex_id = CortexId::from_uuid(session_id.as_uuid());

        // Convert merge strategy string to enum
        let strategy = match merge_strategy.as_str() {
            "auto" | "Auto" => cortex_storage::ResolutionStrategy::AutoMerge,
            "manual" | "Manual" => cortex_storage::ResolutionStrategy::Manual,
            "use_mine" | "UseMine" => cortex_storage::ResolutionStrategy::UseMine,
            "use_theirs" | "UseTheirs" => cortex_storage::ResolutionStrategy::UseTheirs,
            "force" | "Force" => cortex_storage::ResolutionStrategy::Force,
            _ => cortex_storage::ResolutionStrategy::AutoMerge,
        };

        let _result = self.session_manager
            .merge_session(&session_cortex_id, strategy)
            .await?;

        Ok(())
    }

    /// Search for relevant episodes using embeddings
    pub async fn search_episodes(&self, query: &str, limit: usize) -> Result<Vec<Episode>> {
        debug!(query = %query, limit = %limit, "Searching episodes");

        // For now, return empty results since we need embedding integration
        // TODO: Integrate with embedding provider to generate query embedding
        warn!("Episode search requires embedding integration - returning empty results");
        Ok(Vec::new())
    }

    /// Store an episode
    pub async fn store_episode(&self, episode: Episode) -> Result<()> {
        info!(episode_id = %episode.id, agent_id = %episode.agent_id, "Storing episode");

        // Convert Episode to EpisodicMemory
        let episodic_memory = convert_episode_to_memory(episode)?;

        // Store via memory system
        self.memory.store_episode(&episodic_memory).await?;

        Ok(())
    }

    /// Get learned patterns
    pub async fn get_patterns(&self) -> Result<Vec<Pattern>> {
        debug!("Retrieving learned patterns");

        // Use cognitive manager to get patterns
        // For now, return empty results as pattern extraction is done during consolidation
        warn!("Pattern retrieval requires consolidation - returning empty results");
        Ok(Vec::new())
    }

    /// Semantic search for code
    pub async fn semantic_search(
        &self,
        query: &str,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult<CodeUnit>>> {
        debug!(query = %query, "Performing semantic search");

        // Check if semantic search is available
        let search_engine = self
            .semantic
            .as_ref()
            .ok_or_else(|| CortexError::config("Semantic search not configured"))?;

        // Convert filters
        let search_filter = convert_search_filters(filters);

        // Perform search
        let results = search_engine
            .search(query, search_filter.limit.unwrap_or(10))
            .await
            .map_err(|e| CortexError::semantic(e.to_string()))?;

        // Convert results to SearchResult<CodeUnit>
        // For now, return empty as we need to integrate with code unit storage
        warn!("Code unit search integration pending - returning empty results");
        Ok(Vec::new())
    }

    /// Get code units
    pub async fn get_code_units(
        &self,
        workspace_id: WorkspaceId,
        filters: UnitFilters,
    ) -> Result<Vec<CodeUnit>> {
        debug!(workspace_id = %workspace_id, "Retrieving code units");

        // Query storage for code units
        // For now, return empty as we need to integrate with code unit storage
        warn!("Code unit retrieval integration pending - returning empty results");
        Ok(Vec::new())
    }

    /// Read file content
    pub async fn read_file(
        &self,
        session_id: SessionId,
        path: &str,
    ) -> Result<String> {
        use cortex_core::id::CortexId;

        debug!(session_id = %session_id, path = %path, "Reading file");

        // Convert SessionId to CortexId
        let session_cortex_id = CortexId::from_uuid(session_id.as_uuid());

        // Get workspace from session
        let session = self.session_manager.get_session(&session_cortex_id).await?;
        let workspace_uuid = *session.workspace_id.as_uuid();

        // Read via VFS
        let virtual_path = cortex_vfs::VirtualPath::new(path)
            .map_err(|e| CortexError::vfs(e.to_string()))?;

        let content = self.vfs.read_file(&workspace_uuid, &virtual_path).await?;

        Ok(String::from_utf8_lossy(&content).to_string())
    }

    /// Write file content
    pub async fn write_file(
        &self,
        session_id: SessionId,
        path: &str,
        content: &str,
    ) -> Result<()> {
        use cortex_core::id::CortexId;

        info!(session_id = %session_id, path = %path, "Writing file");

        // Convert SessionId to CortexId
        let session_cortex_id = CortexId::from_uuid(session_id.as_uuid());

        // Get workspace from session
        let session = self.session_manager.get_session(&session_cortex_id).await?;
        let workspace_uuid = *session.workspace_id.as_uuid();

        // Write via VFS
        let virtual_path = cortex_vfs::VirtualPath::new(path)
            .map_err(|e| CortexError::vfs(e.to_string()))?;

        self.vfs
            .write_file(&workspace_uuid, &virtual_path, content.as_bytes())
            .await?;

        Ok(())
    }

    /// Query the knowledge graph (stub implementation)
    pub async fn query_graph(
        &self,
        _query: &str,
        _params: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        // TODO: Implement actual graph query
        warn!("Knowledge graph query not yet implemented");
        Ok(Vec::new())
    }

    /// Search for patterns (stub implementation)
    pub async fn search_patterns(
        &self,
        _query: &str,
        _limit: Option<usize>,
    ) -> Result<Vec<Pattern>> {
        // TODO: Implement actual pattern search
        warn!("Pattern search not yet implemented");
        Ok(Vec::new())
    }
}

/// Convert Episode to EpisodicMemory
fn convert_episode_to_memory(episode: Episode) -> Result<cortex_memory::types::EpisodicMemory> {
    use cortex_memory::types::{EpisodicMemory, EpisodeType, ToolUsage, TokenUsage as MemoryTokenUsage};
    use cortex_core::id::CortexId;

    // Parse IDs
    let workspace_id = CortexId::parse(&episode.workspace_id.unwrap_or_default())
        .unwrap_or_else(|_| CortexId::new());

    // Convert episode type
    let episode_type = match episode.episode_type.as_deref() {
        Some("task") | Some("Task") => EpisodeType::Task,
        Some("refactor") | Some("Refactor") => EpisodeType::Refactor,
        Some("bugfix") | Some("Bugfix") => EpisodeType::Bugfix,
        Some("feature") | Some("Feature") => EpisodeType::Feature,
        _ => EpisodeType::Task,
    };

    // Convert outcome - use the types from this module's namespace
    let outcome = match episode.outcome {
        crate::cortex_bridge::EpisodeOutcome::Success => cortex_memory::types::EpisodeOutcome::Success,
        crate::cortex_bridge::EpisodeOutcome::Failure => cortex_memory::types::EpisodeOutcome::Failure,
        crate::cortex_bridge::EpisodeOutcome::Partial => cortex_memory::types::EpisodeOutcome::Partial,
    };

    // Convert tool usage
    let tools_used: Vec<ToolUsage> = episode
        .tools_used
        .into_iter()
        .map(|tool| ToolUsage {
            tool_name: tool,
            usage_count: 1,
            total_duration_ms: 0,
            parameters: HashMap::new(),
        })
        .collect();

    // Convert token usage
    let tokens_used = MemoryTokenUsage {
        input: episode.tokens_used as u64,
        output: 0,
        total: episode.tokens_used as u64,
    };

    // Convert success metrics
    let success_metrics: HashMap<String, f64> = episode
        .success_metrics
        .into_iter()
        .filter_map(|(k, v)| {
            if let serde_json::Value::Number(n) = v {
                n.as_f64().map(|f| (k, f))
            } else {
                None
            }
        })
        .collect();

    Ok(EpisodicMemory {
        id: CortexId::parse(&episode.id).unwrap_or_else(|_| CortexId::new()),
        episode_type,
        task_description: episode.task_description,
        agent_id: episode.agent_id,
        session_id: episode.session_id,
        workspace_id,
        entities_created: episode.entities_created,
        entities_modified: episode.entities_modified,
        entities_deleted: episode.entities_deleted,
        files_touched: episode.files_touched,
        queries_made: episode.queries_made,
        tools_used,
        solution_summary: episode.solution_summary,
        outcome,
        success_metrics,
        errors_encountered: episode.errors_encountered,
        lessons_learned: episode.lessons_learned,
        duration_seconds: episode.duration_seconds.unwrap_or(0.0) as u64,
        tokens_used,
        embedding: episode.embedding,
        created_at: episode.created_at.unwrap_or_else(chrono::Utc::now),
        completed_at: episode.completed_at,
    })
}

/// Convert SearchFilters to SemanticSearchFilter
fn convert_search_filters(filters: SearchFilters) -> SearchFilters {
    filters // Return as-is for now, will be properly converted when integrated
}

// Note: No Default implementation - CortexBridge requires explicit configuration
// with all Cortex modules. Use CortexBridge::new() with proper dependencies.

/// Episode from episodic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub agent_id: String,
    pub outcome: EpisodeOutcome,
    pub success_metrics: HashMap<String, serde_json::Value>,

    // Additional fields used by agents
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
    pub task_description: String,
    pub solution_summary: String,
    pub files_touched: Vec<String>,
    pub tools_used: Vec<String>,
    pub queries_made: Vec<String>,
    pub errors_encountered: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub tokens_used: usize,
    pub episode_type: Option<String>,
    pub entities_created: Vec<String>,
    pub entities_modified: Vec<String>,
    pub entities_deleted: Vec<String>,
    pub embedding: Option<Vec<f32>>,
    pub duration_seconds: Option<f64>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Episode outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    Failure,
    Partial,
}

/// Pattern type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Code,
    Architecture,
    Bug,
    Optimization,
    Refactor,
}

/// Learned pattern from Cortex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub description: String,
    pub name: String,

    // Fields used by optimizer.rs
    pub context: String,
    pub success_rate: f32,

    // Fields used by router.rs
    pub transformation: HashMap<String, serde_json::Value>,
    pub average_improvement: HashMap<String, serde_json::Value>,
    pub times_applied: usize,
}
