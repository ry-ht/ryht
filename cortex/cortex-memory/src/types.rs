//! Memory system types following the cognitive architecture from the specification.

use chrono::{DateTime, Utc};
use cortex_core::id::CortexId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Episodic Memory Types
// ============================================================================

/// Type of episode being recorded
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeType {
    Task,
    Refactor,
    Bugfix,
    Feature,
    Exploration,
}

/// Outcome of an episode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeOutcome {
    Success,
    Partial,
    Failure,
    Abandoned,
}

/// Tool usage during an episode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUsage {
    pub tool_name: String,
    pub usage_count: u32,
    pub total_duration_ms: u64,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Token usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
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

/// A complete episodic memory record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodicMemory {
    pub id: CortexId,
    pub episode_type: EpisodeType,

    // Context
    pub task_description: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub workspace_id: CortexId,

    // Work performed
    pub entities_created: Vec<String>,
    pub entities_modified: Vec<String>,
    pub entities_deleted: Vec<String>,
    pub files_touched: Vec<String>,
    pub queries_made: Vec<String>,
    pub tools_used: Vec<ToolUsage>,

    // Outcome
    pub solution_summary: String,
    pub outcome: EpisodeOutcome,
    pub success_metrics: HashMap<String, f64>,
    pub errors_encountered: Vec<String>,
    pub lessons_learned: Vec<String>,

    // Performance
    pub duration_seconds: u64,
    pub tokens_used: TokenUsage,

    // Semantic representation
    pub embedding: Option<Vec<f32>>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl EpisodicMemory {
    pub fn new(
        task_description: String,
        agent_id: String,
        workspace_id: CortexId,
        episode_type: EpisodeType,
    ) -> Self {
        Self {
            id: CortexId::new(),
            episode_type,
            task_description,
            agent_id,
            session_id: None,
            workspace_id,
            entities_created: Vec::new(),
            entities_modified: Vec::new(),
            entities_deleted: Vec::new(),
            files_touched: Vec::new(),
            queries_made: Vec::new(),
            tools_used: Vec::new(),
            solution_summary: String::new(),
            outcome: EpisodeOutcome::Success,
            success_metrics: HashMap::new(),
            errors_encountered: Vec::new(),
            lessons_learned: Vec::new(),
            duration_seconds: 0,
            tokens_used: TokenUsage::default(),
            embedding: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }
}

// ============================================================================
// Semantic Memory Types
// ============================================================================

/// Type of code entity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CodeUnitType {
    Function,
    Method,
    AsyncFunction,
    Generator,
    Lambda,
    Class,
    Struct,
    Enum,
    Union,
    Interface,
    Trait,
    TypeAlias,
    Typedef,
    Const,
    Static,
    Variable,
    Module,
    Namespace,
    Package,
    ImplBlock,
    Decorator,
    Macro,
    Template,
    Test,
    Benchmark,
    Example,
}

/// Code complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub lines: u32,
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            lines: 0,
        }
    }
}

/// Semantic code unit information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticUnit {
    pub id: CortexId,
    pub unit_type: CodeUnitType,

    // Identification
    pub name: String,
    pub qualified_name: String,
    pub display_name: String,

    // Location
    pub file_path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,

    // Code content
    pub signature: String,
    pub body: String,
    pub docstring: Option<String>,

    // Semantic information
    pub visibility: String,
    pub modifiers: Vec<String>,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,

    // Analysis
    pub summary: String,
    pub purpose: String,
    pub complexity: ComplexityMetrics,

    // Quality
    pub test_coverage: Option<f32>,
    pub has_tests: bool,
    pub has_documentation: bool,

    // Embedding
    pub embedding: Option<Vec<f32>>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Type of dependency relationship
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Imports,
    Requires,
    Includes,
    Calls,
    Invokes,
    Instantiates,
    Extends,
    Implements,
    Inherits,
    UsesType,
    UsesTrait,
    UsesInterface,
    Reads,
    Writes,
    Modifies,
}

/// Dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    pub id: CortexId,
    pub source_id: CortexId,
    pub target_id: CortexId,
    pub dependency_type: DependencyType,
    pub is_direct: bool,
    pub is_runtime: bool,
    pub is_dev: bool,
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// Working Memory Types
// ============================================================================

/// Priority level for working memory items
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Low = 1,
}

/// Working memory item with priority and recency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub key: String,
    pub value: Vec<u8>,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub size_bytes: usize,
}

impl WorkingMemoryItem {
    pub fn new(key: String, value: Vec<u8>, priority: Priority) -> Self {
        let now = Utc::now();
        let size_bytes = value.len();
        Self {
            key,
            value,
            priority,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            size_bytes,
        }
    }

    /// Calculate a score for eviction (lower = more likely to be evicted)
    pub fn retention_score(&self) -> f64 {
        let priority_score = self.priority as u32 as f64 * 100.0;
        let age_seconds = (Utc::now() - self.last_accessed).num_seconds() as f64;
        let recency_score = 1000.0 / (1.0 + age_seconds);
        // Use ln(1 + access_count) to avoid -inf for access_count=0
        let access_score = ((self.access_count + 1) as f64).ln() * 10.0;

        priority_score + recency_score + access_score
    }
}

// ============================================================================
// Procedural Memory Types
// ============================================================================

/// Type of learned pattern
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    Code,
    Architecture,
    Refactor,
    Optimization,
    ErrorRecovery,
}

/// A learned pattern or procedure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LearnedPattern {
    pub id: CortexId,
    pub pattern_type: PatternType,

    // Description
    pub name: String,
    pub description: String,
    pub context: String,

    // Pattern definition
    pub before_state: serde_json::Value,
    pub after_state: serde_json::Value,
    pub transformation: serde_json::Value,

    // Usage statistics
    pub times_applied: u32,
    pub success_rate: f32,
    pub average_improvement: HashMap<String, f64>,

    // Examples
    pub example_episodes: Vec<CortexId>,

    // Semantic search
    pub embedding: Option<Vec<f32>>,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LearnedPattern {
    pub fn new(
        pattern_type: PatternType,
        name: String,
        description: String,
        context: String,
    ) -> Self {
        Self {
            id: CortexId::new(),
            pattern_type,
            name,
            description,
            context,
            before_state: serde_json::Value::Null,
            after_state: serde_json::Value::Null,
            transformation: serde_json::Value::Null,
            times_applied: 0,
            success_rate: 0.0,
            average_improvement: HashMap::new(),
            example_episodes: Vec::new(),
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Record a successful application
    pub fn record_success(&mut self) {
        self.times_applied += 1;
        let successes = (self.success_rate * (self.times_applied - 1) as f32) + 1.0;
        self.success_rate = successes / self.times_applied as f32;
        self.updated_at = Utc::now();
    }

    /// Record a failed application
    pub fn record_failure(&mut self) {
        self.times_applied += 1;
        let successes = self.success_rate * (self.times_applied - 1) as f32;
        self.success_rate = successes / self.times_applied as f32;
        self.updated_at = Utc::now();
    }
}

// ============================================================================
// Memory Consolidation Types
// ============================================================================

/// Importance factors for memory consolidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceFactors {
    pub recency_score: f32,
    pub frequency_score: f32,
    pub outcome_score: f32,
    pub complexity_score: f32,
    pub novelty_score: f32,
    pub relevance_score: f32,
}

impl ImportanceFactors {
    /// Calculate combined importance score
    pub fn combined_score(&self) -> f32 {
        let weights = [0.2, 0.2, 0.25, 0.1, 0.15, 0.1];
        let scores = [
            self.recency_score,
            self.frequency_score,
            self.outcome_score,
            self.complexity_score,
            self.novelty_score,
            self.relevance_score,
        ];

        scores
            .iter()
            .zip(weights.iter())
            .map(|(s, w)| s * w)
            .sum()
    }
}

/// Memory decay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    pub half_life_days: f64,
    pub minimum_importance: f32,
    pub consolidation_threshold: f32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            half_life_days: 30.0,
            minimum_importance: 0.1,
            consolidation_threshold: 0.5,
        }
    }
}

// ============================================================================
// Query Types
// ============================================================================

/// Parameters for memory search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query_text: String,
    pub limit: usize,
    pub similarity_threshold: f32,
    pub filters: HashMap<String, String>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl MemoryQuery {
    pub fn new(query_text: String) -> Self {
        Self {
            query_text,
            limit: 10,
            similarity_threshold: 0.7,
            filters: HashMap::new(),
            time_range: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    pub fn with_time_range(
        mut self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        self.time_range = Some((start, end));
        self
    }
}

/// Search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult<T> {
    pub item: T,
    pub similarity_score: f32,
    pub relevance_score: f32,
}

// ============================================================================
// Statistics Types
// ============================================================================

/// Memory system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub episodic: EpisodicStats,
    pub semantic: SemanticStats,
    pub working: WorkingStats,
    pub procedural: ProceduralStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicStats {
    pub total_episodes: u64,
    pub successful_episodes: u64,
    pub failed_episodes: u64,
    pub average_duration_seconds: f64,
    pub total_tokens_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticStats {
    pub total_units: u64,
    pub total_dependencies: u64,
    pub average_complexity: f64,
    pub coverage_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingStats {
    pub current_items: usize,
    pub capacity: usize,
    pub total_evictions: u64,
    pub cache_hit_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralStats {
    pub total_patterns: u64,
    pub average_success_rate: f32,
    pub total_applications: u64,
}
