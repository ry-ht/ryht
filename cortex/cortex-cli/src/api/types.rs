//! API request and response types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub metadata: ApiMetadata,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, request_id: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ApiMetadata {
                request_id,
                timestamp: Utc::now(),
                version: "v3".to_string(),
                duration_ms,
            },
        }
    }

    pub fn error(error: String, request_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata: ApiMetadata {
                request_id,
                timestamp: Utc::now(),
                version: "v3".to_string(),
                duration_ms: 0,
            },
        }
    }
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMetadata {
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub duration_ms: u64,
}

// ============================================================================
// VFS Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListRequest {
    #[serde(default)]
    pub recursive: bool,
    pub file_type: Option<String>,
    pub language: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub file_type: String,
    pub size: u64,
    pub language: Option<String>,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryTreeResponse {
    pub name: String,
    pub path: String,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFileRequest {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateFileRequest {
    pub content: String,
}

// ============================================================================
// Workspace Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub workspace_type: String,
    pub source_type: String,
    pub namespace: String,
    pub source_path: Option<String>,
    pub read_only: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub workspace_type: String,
    pub source_path: Option<String>,
}

// ============================================================================
// Session Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    pub agent_type: String,
    pub workspace_id: Option<String>,
}

// ============================================================================
// Search Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f64,
    pub result_type: String,
    pub metadata: serde_json::Value,
}

// ============================================================================
// Memory Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEpisode {
    pub id: String,
    pub content: String,
    pub episode_type: String,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsolidateMemoryRequest {
    pub workspace_id: Option<String>,
}

// ============================================================================
// Health Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub database: DatabaseHealth,
    pub memory: MemoryHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub response_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryHealth {
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub workspaces: usize,
    pub files: usize,
    pub total_size_bytes: u64,
    pub episodes: usize,
    pub semantic_nodes: usize,
}

// ============================================================================
// Code Units Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeUnitListRequest {
    #[serde(default)]
    pub unit_type: Option<String>,
    pub visibility: Option<String>,
    pub language: Option<String>,
    pub min_complexity: Option<u32>,
    pub max_complexity: Option<u32>,
    pub has_tests: Option<bool>,
    pub has_docs: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeUnitResponse {
    pub id: String,
    pub unit_type: String,
    pub name: String,
    pub qualified_name: String,
    pub display_name: String,
    pub file_path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub signature: String,
    pub body: Option<String>,
    pub docstring: Option<String>,
    pub visibility: String,
    pub is_async: bool,
    pub is_exported: bool,
    pub complexity: ComplexityResponse,
    pub has_tests: bool,
    pub has_documentation: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityResponse {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub lines: u32,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCodeUnitRequest {
    pub body: Option<String>,
    pub docstring: Option<String>,
    pub expected_version: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeUnitListResponse {
    pub units: Vec<CodeUnitResponse>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

// ============================================================================
// Dependencies & Graph Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyGraphRequest {
    pub format: Option<String>, // json, dot, mermaid
    pub level: Option<String>,   // file, unit, package
    pub max_depth: Option<usize>,
    pub include_external: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyGraphResponse {
    pub format: String,
    pub content: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub weight: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_depth: usize,
    pub cycles_detected: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImpactAnalysisRequest {
    pub changed_entity_ids: Vec<String>,
    pub analysis_type: Option<String>, // full, direct, transitive
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImpactAnalysisResponse {
    pub changed_entities: Vec<EntityImpact>,
    pub affected_entities: Vec<EntityImpact>,
    pub risk_assessment: RiskAssessment,
    pub analysis_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityImpact {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub impact_level: String,
    pub affected_by: Vec<String>,
    pub affects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: String,
    pub risk_score: f64,
    pub total_affected: usize,
    pub critical_paths: Vec<Vec<String>>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CycleDetectionResponse {
    pub cycles: Vec<DependencyCycle>,
    pub total_cycles: usize,
    pub max_cycle_length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyCycle {
    pub cycle_id: String,
    pub entities: Vec<String>,
    pub cycle_length: usize,
    pub severity: String,
    pub suggestions: Vec<String>,
}

// ============================================================================
// Workspace Update/Sync Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub workspace_type: Option<String>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncWorkspaceRequest {
    pub force: Option<bool>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub files_added: usize,
    pub files_updated: usize,
    pub files_deleted: usize,
    pub total_processed: usize,
    pub duration_ms: u64,
    pub changes: Vec<SyncChange>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncChange {
    pub path: String,
    pub change_type: String, // added, updated, deleted
    pub size_bytes: Option<u64>,
}

// ============================================================================
// Search Reference Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ReferencesResponse {
    pub unit_id: String,
    pub unit_name: String,
    pub total_references: usize,
    pub references: Vec<CodeReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeReference {
    pub id: String,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub reference_type: String, // call, import, instantiation, etc
    pub context: String,
    pub referencing_unit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternSearchRequest {
    pub workspace_id: String,
    pub pattern: String,
    pub language: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternSearchResponse {
    pub pattern: String,
    pub total_matches: usize,
    pub matches: Vec<PatternMatch>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternMatch {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub matched_text: String,
    pub context: String,
    pub unit_id: Option<String>,
}

// ============================================================================
// Memory Search Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodeSearchRequest {
    pub query: String,
    pub episode_type: Option<String>,
    pub min_importance: Option<f64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: String,
    pub pattern_name: String,
    pub description: String,
    pub pattern_type: String,
    pub occurrences: usize,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub examples: Vec<String>,
}

// ============================================================================
// Build & CI/CD Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildRequest {
    pub workspace_id: String,
    pub build_type: String, // debug, release, test
    pub target: Option<String>,
    pub features: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    pub job_id: String,
    pub workspace_id: String,
    pub build_type: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStatusResponse {
    pub job_id: String,
    pub status: String,
    pub progress: f64,
    pub current_step: Option<String>,
    pub logs_url: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub artifacts: Vec<BuildArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub name: String,
    pub artifact_type: String,
    pub size_bytes: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunRequest {
    pub workspace_id: String,
    pub test_pattern: Option<String>,
    pub coverage: Option<bool>,
    pub test_type: Option<String>, // unit, integration, all
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestRunResponse {
    pub run_id: String,
    pub workspace_id: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestResultsResponse {
    pub run_id: String,
    pub status: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_seconds: f64,
    pub coverage: Option<CoverageReport>,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoverageReport {
    pub lines_covered: usize,
    pub lines_total: usize,
    pub percentage: f64,
    pub by_file: Vec<FileCoverage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCoverage {
    pub file_path: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
}
