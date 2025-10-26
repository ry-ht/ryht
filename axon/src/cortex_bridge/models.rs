//! Data models for Cortex API integration
//!
//! This module defines all data structures used for communication with the Cortex REST API.
//! These models match the Cortex database schema and API specifications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::collections::HashMap;

// ============================================================================
// Identity Types
// ============================================================================

/// Session identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        SessionId(s)
    }
}

/// Agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        AgentId(s)
    }
}

/// Workspace identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub String);

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WorkspaceId {
    fn from(s: String) -> Self {
        WorkspaceId(s)
    }
}

/// Episode identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpisodeId(pub String);

impl fmt::Display for EpisodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for EpisodeId {
    fn from(s: String) -> Self {
        EpisodeId(s)
    }
}

/// Task identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TaskId {
    fn from(s: String) -> Self {
        TaskId(s)
    }
}

/// Lock identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockId(pub String);

impl fmt::Display for LockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for LockId {
    fn from(s: String) -> Self {
        LockId(s)
    }
}

// ============================================================================
// Session Models
// ============================================================================

/// Session scope definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScope {
    /// Paths accessible in the session
    pub paths: Vec<String>,
    /// Read-only paths
    pub read_only_paths: Vec<String>,
}

/// Session status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    /// Session ID
    pub session_id: SessionId,
    /// Agent ID
    pub agent_id: AgentId,
    /// Workspace ID
    pub workspace_id: WorkspaceId,
    /// Current status
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Number of changes in session
    pub change_count: u32,
}

/// Health status response
#[derive(Debug, Clone, Deserialize)]
pub struct HealthStatus {
    /// Service status
    pub status: String,
    /// Version
    pub version: String,
    /// Database status
    pub database: String,
}

// ============================================================================
// File Operations Models
// ============================================================================

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path
    pub path: String,
    /// File type (file, directory, symlink)
    pub file_type: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Last modified timestamp
    pub modified_at: String,
}

// ============================================================================
// Merge Models
// ============================================================================

/// Merge strategy for session merging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Automatic conflict resolution
    Auto,
    /// Manual conflict resolution
    Manual,
    /// Take their changes
    Theirs,
    /// Take my changes
    Mine,
}

impl fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeStrategy::Auto => write!(f, "auto"),
            MergeStrategy::Manual => write!(f, "manual"),
            MergeStrategy::Theirs => write!(f, "theirs"),
            MergeStrategy::Mine => write!(f, "mine"),
        }
    }
}

/// Merge report result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReport {
    /// Number of changes merged
    pub changes_merged: u32,
    /// Number of conflicts resolved
    pub conflicts_resolved: u32,
    /// New version number
    pub new_version: u64,
}

// ============================================================================
// Episode Models
// ============================================================================

/// Episode type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeType {
    /// Task execution
    Task,
    /// Refactoring operation
    Refactor,
    /// Bug fix
    Bugfix,
    /// Feature implementation
    Feature,
    /// Code exploration
    Exploration,
}

/// Episode outcome classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeOutcome {
    /// Successful completion
    Success,
    /// Partial completion
    Partial,
    /// Failed execution
    Failure,
    /// Abandoned task
    Abandoned,
}

/// Tool usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    /// Tool name
    pub tool_name: String,
    /// Number of invocations
    pub invocations: u32,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens
    pub input: u64,
    /// Output tokens
    pub output: u64,
    /// Total tokens
    pub total: u64,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            input: 0,
            output: 0,
            total: 0,
        }
    }
}

/// Episode data structure - matches Cortex database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Episode ID
    pub id: String,
    /// Episode type
    pub episode_type: EpisodeType,
    /// Task description
    pub task_description: String,
    /// Agent ID
    pub agent_id: String,
    /// Session ID (optional)
    pub session_id: Option<String>,
    /// Workspace ID
    pub workspace_id: String,
    /// Entities created
    pub entities_created: Vec<String>,
    /// Entities modified
    pub entities_modified: Vec<String>,
    /// Entities deleted
    pub entities_deleted: Vec<String>,
    /// Files touched
    pub files_touched: Vec<String>,
    /// Queries made
    pub queries_made: Vec<String>,
    /// Tools used
    pub tools_used: Vec<ToolUsage>,
    /// Solution summary
    pub solution_summary: String,
    /// Outcome
    pub outcome: EpisodeOutcome,
    /// Success metrics (JSON object)
    pub success_metrics: serde_json::Value,
    /// Errors encountered
    pub errors_encountered: Vec<String>,
    /// Lessons learned
    pub lessons_learned: Vec<String>,
    /// Duration in seconds
    pub duration_seconds: i32,
    /// Token usage
    pub tokens_used: TokenUsage,
    /// Semantic embedding (generated by Cortex)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub embedding: Vec<f32>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Pattern Models
// ============================================================================

/// Pattern type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    /// Code pattern
    Code,
    /// Architecture pattern
    Architecture,
    /// Refactoring pattern
    Refactor,
    /// Optimization pattern
    Optimization,
    /// Error recovery pattern
    ErrorRecovery,
}

/// Pattern data structure - matches Cortex database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Pattern ID
    pub id: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Pattern name
    pub name: String,
    /// Pattern description
    pub description: String,
    /// Context where pattern applies
    pub context: String,
    /// Before state (JSON object)
    pub before_state: serde_json::Value,
    /// After state (JSON object)
    pub after_state: serde_json::Value,
    /// Transformation steps (JSON object)
    pub transformation: serde_json::Value,
    /// Number of times applied
    pub times_applied: i32,
    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
    /// Average improvement metrics (JSON object)
    pub average_improvement: serde_json::Value,
    /// Example episode IDs
    pub example_episodes: Vec<String>,
    /// Semantic embedding (generated by Cortex)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub embedding: Vec<f32>,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            id: String::new(),
            pattern_type: PatternType::Code,
            name: String::new(),
            description: String::new(),
            context: String::new(),
            before_state: serde_json::Value::Object(Default::default()),
            after_state: serde_json::Value::Object(Default::default()),
            transformation: serde_json::Value::Object(Default::default()),
            times_applied: 0,
            success_rate: 0.0,
            average_improvement: serde_json::Value::Object(Default::default()),
            example_episodes: Vec::new(),
            embedding: Vec::new(),
        }
    }
}

// ============================================================================
// Search Models
// ============================================================================

/// Search filters for semantic search
#[derive(Debug, Clone)]
pub struct SearchFilters {
    /// Unit types to filter
    pub types: Vec<String>,
    /// Languages to filter
    pub languages: Vec<String>,
    /// Visibility filter
    pub visibility: Option<String>,
    /// Minimum relevance score
    pub min_relevance: f32,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            types: Vec::new(),
            languages: Vec::new(),
            visibility: None,
            min_relevance: 0.7,
        }
    }
}

/// Code search result
#[derive(Debug, Clone, Deserialize)]
pub struct CodeSearchResult {
    /// Code unit ID
    pub unit_id: String,
    /// Unit type (function, class, etc.)
    pub unit_type: String,
    /// Unit name
    pub name: String,
    /// Qualified name
    pub qualified_name: String,
    /// Signature
    pub signature: String,
    /// Relevance score
    pub relevance_score: f32,
    /// File path
    pub file: String,
    /// Code snippet
    pub snippet: String,
}

/// Unit filters for code unit queries
#[derive(Debug, Clone, Default)]
pub struct UnitFilters {
    /// Unit type filter
    pub unit_type: Option<String>,
    /// Language filter
    pub language: Option<String>,
    /// Visibility filter
    pub visibility: Option<String>,
}

/// Code unit information
#[derive(Debug, Clone, Deserialize)]
pub struct CodeUnit {
    /// Unit ID
    pub id: String,
    /// Unit type
    pub unit_type: String,
    /// Name
    pub name: String,
    /// Qualified name
    pub qualified_name: String,
    /// Signature
    pub signature: String,
    /// File path
    pub file: String,
    /// Line range
    pub lines: LineRange,
    /// Visibility
    pub visibility: String,
    /// Complexity metrics
    pub complexity: Complexity,
}

/// Line range in a file
#[derive(Debug, Clone, Deserialize)]
pub struct LineRange {
    /// Start line
    pub start: u32,
    /// End line
    pub end: u32,
}

/// Complexity metrics
#[derive(Debug, Clone, Deserialize)]
pub struct Complexity {
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Cognitive complexity
    pub cognitive: u32,
}

// ============================================================================
// Task Models
// ============================================================================

/// Task definition
#[derive(Debug, Clone)]
pub struct TaskDefinition {
    /// Task title
    pub title: String,
    /// Task description
    pub description: String,
    /// Workspace ID
    pub workspace_id: String,
    /// Estimated hours
    pub estimated_hours: f64,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// Pending execution
    Pending,
    /// In progress
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed execution
    Failed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "done"),
            TaskStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Task metadata
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Task duration
    pub duration: std::time::Duration,
    /// Completion notes
    pub notes: Option<String>,
}

// ============================================================================
// Lock Models
// ============================================================================

/// Lock type for coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared lock (multiple readers)
    Shared,
    /// Exclusive lock (single writer)
    Exclusive,
}

impl fmt::Display for LockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockType::Shared => write!(f, "shared"),
            LockType::Exclusive => write!(f, "exclusive"),
        }
    }
}

// ============================================================================
// WebSocket Event Models
// ============================================================================

/// WebSocket event from Cortex
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CortexEvent {
    /// Session created event
    SessionCreated {
        /// Session ID
        session_id: String,
    },
    /// Session merged event
    SessionMerged {
        /// Session ID
        session_id: String,
        /// Number of conflicts
        conflicts: u32,
    },
    /// Session closed event
    SessionClosed {
        /// Session ID
        session_id: String,
    },
    /// Lock acquired event
    LockAcquired {
        /// Lock ID
        lock_id: String,
        /// Entity ID
        entity_id: String,
    },
    /// Lock released event
    LockReleased {
        /// Lock ID
        lock_id: String,
    },
    /// Lock deadlock detected
    LockDeadlock {
        /// Entity ID
        entity_id: String,
        /// Agent IDs involved
        agents: Vec<String>,
    },
    /// Conflict detected
    ConflictDetected {
        /// Session ID
        session_id: String,
        /// Files with conflicts
        files: Vec<String>,
    },
    /// File changed event
    FileChanged {
        /// File path
        path: String,
        /// Workspace ID
        workspace_id: String,
    },
    /// Pattern detected event
    PatternDetected {
        /// Pattern name
        pattern: String,
        /// Confidence score
        confidence: f32,
    },
}

/// Event filter for subscription
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventFilter {
    /// All events
    All,
    /// Session events only
    Sessions,
    /// Lock events only
    Locks,
    /// Conflict events only
    Conflicts,
    /// File change events only
    FileChanges,
    /// Pattern events only
    Patterns,
}

// ============================================================================
// Working Memory Models
// ============================================================================

/// Working memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    /// Item ID
    pub id: String,
    /// Item type (code_snippet, task, note, etc.)
    pub item_type: String,
    /// Content
    pub content: String,
    /// Context information
    pub context: serde_json::Value,
    /// Priority (0.0 - 1.0)
    pub priority: f32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
}

/// Working memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryStats {
    /// Total items in working memory
    pub total_items: usize,
    /// Total memory usage in bytes
    pub total_bytes: usize,
    /// Capacity limit (items)
    pub capacity_items: usize,
    /// Capacity limit (bytes)
    pub capacity_bytes: usize,
    /// Items by type
    pub items_by_type: HashMap<String, usize>,
}

// ============================================================================
// Consolidation Models
// ============================================================================

/// Memory consolidation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    /// Number of working memory items consolidated
    pub items_consolidated: usize,
    /// Number of patterns extracted
    pub patterns_extracted: usize,
    /// Number of semantic units created
    pub semantic_units_created: usize,
    /// Consolidation duration in ms
    pub duration_ms: u64,
    /// Memory freed in bytes
    pub memory_freed_bytes: usize,
}

/// Dream consolidation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamReport {
    /// New patterns discovered
    pub new_patterns: usize,
    /// Patterns refined
    pub patterns_refined: usize,
    /// Low-importance memories forgotten
    pub memories_forgotten: usize,
    /// Processing duration in ms
    pub duration_ms: u64,
}

/// Pattern version (for evolution tracking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVersion {
    /// Version ID
    pub version_id: String,
    /// Pattern ID
    pub pattern_id: String,
    /// Version number
    pub version: u32,
    /// Changes made
    pub changes: String,
    /// Performance metrics
    pub metrics: serde_json::Value,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Pattern application result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternApplication {
    /// Application ID
    pub id: String,
    /// Pattern ID
    pub pattern_id: String,
    /// Context applied to
    pub context: serde_json::Value,
    /// Result of application
    pub result: serde_json::Value,
    /// Success flag
    pub success: bool,
    /// Applied timestamp
    pub applied_at: DateTime<Utc>,
}

// ============================================================================
// Collaborative Memory Models
// ============================================================================

/// Collaborative insight (pattern/knowledge shared across agents)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborativeInsight {
    /// Insight ID
    pub id: String,
    /// Insight type
    pub insight_type: String,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Contributing agents
    pub contributing_agents: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Supporting episodes
    pub supporting_episodes: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Code Materialization Models
// ============================================================================

/// Code representation in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRepresentation {
    /// Representation type (ast, semantic, template)
    pub repr_type: String,
    /// Language
    pub language: String,
    /// Semantic description
    pub description: String,
    /// Structure/AST representation
    pub structure: serde_json::Value,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Materialized code result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedCode {
    /// File path
    pub path: String,
    /// Generated code content
    pub content: String,
    /// Language
    pub language: String,
    /// Analysis results
    pub analysis: CodeAnalysisResult,
}

/// Code analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    /// Units extracted
    pub units_extracted: usize,
    /// Dependencies found
    pub dependencies_found: usize,
    /// Complexity metrics
    pub complexity: serde_json::Value,
    /// Issues found
    pub issues: Vec<String>,
}

/// Sync report for bidirectional sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    /// Files synced
    pub files_synced: usize,
    /// Units updated
    pub units_updated: usize,
    /// Dependencies updated
    pub dependencies_updated: usize,
    /// Conflicts detected
    pub conflicts: usize,
    /// Sync duration in ms
    pub duration_ms: u64,
}
