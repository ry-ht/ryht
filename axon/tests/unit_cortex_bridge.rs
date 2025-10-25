//! Unit tests for CortexBridge
//!
//! Tests cover:
//! - Configuration and initialization
//! - Model types and conversions
//! - Error handling
//! - Type safety

mod common;

use axon::cortex_bridge::*;

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_cortex_config_default() {
    let config = CortexConfig::default();
    assert_eq!(config.base_url, "http://localhost:8081");
    assert_eq!(config.api_version, "v3");
    assert_eq!(config.request_timeout_secs, 30);
}

#[test]
fn test_cortex_config_custom() {
    let config = CortexConfig {
        base_url: "http://custom:9000".to_string(),
        api_version: "v4".to_string(),
        request_timeout_secs: 60,
        max_retries: 5,
    };

    assert_eq!(config.base_url, "http://custom:9000");
    assert_eq!(config.api_version, "v4");
    assert_eq!(config.request_timeout_secs, 60);
    assert_eq!(config.max_retries, 5);
}

// ============================================================================
// ID Types Tests
// ============================================================================

#[test]
fn test_agent_id_creation() {
    let id = AgentId::from("test-agent".to_string());
    assert_eq!(id.to_string(), "test-agent");
}

#[test]
fn test_agent_id_equality() {
    let id1 = AgentId::from("agent-1".to_string());
    let id2 = AgentId::from("agent-1".to_string());
    let id3 = AgentId::from("agent-2".to_string());

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn test_session_id_creation() {
    let id = SessionId::from("session-123".to_string());
    assert_eq!(id.to_string(), "session-123");
}

#[test]
fn test_workspace_id_creation() {
    let id = WorkspaceId::from("workspace-abc".to_string());
    assert_eq!(id.to_string(), "workspace-abc");
}

#[test]
fn test_episode_id_creation() {
    let id = EpisodeId::from("episode-xyz".to_string());
    assert_eq!(id.to_string(), "episode-xyz");
}

#[test]
fn test_lock_id_creation() {
    let id = LockId::from("lock-001".to_string());
    assert_eq!(id.to_string(), "lock-001");
}

// ============================================================================
// Session Scope Tests
// ============================================================================

#[test]
fn test_session_scope_variants() {
    let scopes = vec![
        SessionScope::Task,
        SessionScope::Project,
        SessionScope::Workspace,
    ];

    assert_eq!(scopes.len(), 3);
}

// ============================================================================
// Session Status Tests
// ============================================================================

#[test]
fn test_session_status_variants() {
    let statuses = vec![
        SessionStatus::Active,
        SessionStatus::Paused,
        SessionStatus::Closed,
    ];

    assert_eq!(statuses.len(), 3);
}

// ============================================================================
// Lock Type Tests
// ============================================================================

#[test]
fn test_lock_type_variants() {
    let lock_types = vec![LockType::Read, LockType::Write, LockType::Exclusive];

    assert_eq!(lock_types.len(), 3);
}

// ============================================================================
// Merge Strategy Tests
// ============================================================================

#[test]
fn test_merge_strategy_variants() {
    let strategies = vec![
        MergeStrategy::Auto,
        MergeStrategy::ThreeWay,
        MergeStrategy::Manual,
    ];

    assert_eq!(strategies.len(), 3);
}

// ============================================================================
// Episode Tests
// ============================================================================

#[test]
fn test_episode_creation() {
    use chrono::Utc;

    let episode = Episode {
        id: EpisodeId::from("ep-001".to_string()),
        agent_id: AgentId::from("agent-1".to_string()),
        session_id: Some(SessionId::from("session-1".to_string())),
        task_type: "code_generation".to_string(),
        task_description: "Generate function".to_string(),
        context: serde_json::json!({"language": "rust"}),
        action_taken: "Generated async function".to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({"lines": 50}),
        learned_patterns: vec!["async-pattern".to_string()],
        timestamp: Utc::now(),
    };

    assert_eq!(episode.task_type, "code_generation");
    assert!(matches!(episode.outcome, EpisodeOutcome::Success));
}

#[test]
fn test_episode_outcome_variants() {
    let outcomes = vec![
        EpisodeOutcome::Success,
        EpisodeOutcome::Failure,
        EpisodeOutcome::Partial,
    ];

    assert_eq!(outcomes.len(), 3);
}

// ============================================================================
// Pattern Tests
// ============================================================================

#[test]
fn test_pattern_creation() {
    use chrono::Utc;

    let pattern = Pattern {
        id: "pattern-001".to_string(),
        name: "Error Handling Pattern".to_string(),
        pattern_type: PatternType::CodePattern,
        description: "Use Result<T, E>".to_string(),
        context: "Rust error handling".to_string(),
        transformation: serde_json::json!({"approach": "Result"}),
        success_rate: 0.95,
        times_applied: 100,
        average_improvement: serde_json::json!({"quality": 0.8}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    assert_eq!(pattern.name, "Error Handling Pattern");
    assert_eq!(pattern.success_rate, 0.95);
    assert_eq!(pattern.times_applied, 100);
}

#[test]
fn test_pattern_type_variants() {
    let types = vec![
        PatternType::CodePattern,
        PatternType::ArchitecturePattern,
        PatternType::OptimizationPattern,
        PatternType::CommunicationPattern,
    ];

    assert_eq!(types.len(), 4);
}

// ============================================================================
// File Info Tests
// ============================================================================

#[test]
fn test_file_info_creation() {
    use chrono::Utc;

    let file_info = FileInfo {
        path: "/src/main.rs".to_string(),
        size: 1024,
        modified: Utc::now(),
        file_type: FileType::File,
    };

    assert_eq!(file_info.path, "/src/main.rs");
    assert_eq!(file_info.size, 1024);
}

#[test]
fn test_file_type_variants() {
    let types = vec![FileType::File, FileType::Directory, FileType::Symlink];

    assert_eq!(types.len(), 3);
}

// ============================================================================
// Merge Report Tests
// ============================================================================

#[test]
fn test_merge_report_creation() {
    let report = MergeReport {
        success: true,
        files_merged: 5,
        conflicts: vec![],
        message: "Merge successful".to_string(),
    };

    assert!(report.success);
    assert_eq!(report.files_merged, 5);
    assert!(report.conflicts.is_empty());
}

#[test]
fn test_merge_report_with_conflicts() {
    let conflicts = vec![
        MergeConflict {
            file_path: "/src/lib.rs".to_string(),
            conflict_type: ConflictType::ContentConflict,
            description: "Conflicting changes".to_string(),
        },
    ];

    let report = MergeReport {
        success: false,
        files_merged: 3,
        conflicts,
        message: "Merge failed".to_string(),
    };

    assert!(!report.success);
    assert_eq!(report.conflicts.len(), 1);
}

// ============================================================================
// Conflict Type Tests
// ============================================================================

#[test]
fn test_conflict_type_variants() {
    let types = vec![
        ConflictType::ContentConflict,
        ConflictType::DeleteModifyConflict,
        ConflictType::RenameConflict,
    ];

    assert_eq!(types.len(), 3);
}

// ============================================================================
// Search Filter Tests
// ============================================================================

#[test]
fn test_search_filters_default() {
    let filters = SearchFilters::default();
    assert!(filters.file_types.is_none());
    assert!(filters.max_results.is_none());
}

#[test]
fn test_search_filters_custom() {
    let filters = SearchFilters {
        file_types: Some(vec!["rs".to_string(), "toml".to_string()]),
        min_score: Some(0.7),
        max_results: Some(10),
        include_archived: false,
    };

    assert_eq!(filters.file_types.as_ref().unwrap().len(), 2);
    assert_eq!(filters.min_score, Some(0.7));
    assert_eq!(filters.max_results, Some(10));
}

// ============================================================================
// Code Search Result Tests
// ============================================================================

#[test]
fn test_code_search_result() {
    let result = CodeSearchResult {
        file_path: "/src/main.rs".to_string(),
        line_number: 42,
        snippet: "fn main() {}".to_string(),
        score: 0.95,
        context: "Main function".to_string(),
    };

    assert_eq!(result.line_number, 42);
    assert_eq!(result.score, 0.95);
}

// ============================================================================
// Code Unit Tests
// ============================================================================

#[test]
fn test_code_unit_creation() {
    let unit = CodeUnit {
        id: "unit-001".to_string(),
        name: "MyStruct".to_string(),
        kind: CodeUnitKind::Struct,
        file_path: "/src/types.rs".to_string(),
        start_line: 10,
        end_line: 20,
        signature: "pub struct MyStruct {}".to_string(),
        documentation: Some("A test struct".to_string()),
    };

    assert_eq!(unit.name, "MyStruct");
    assert!(matches!(unit.kind, CodeUnitKind::Struct));
}

#[test]
fn test_code_unit_kind_variants() {
    let kinds = vec![
        CodeUnitKind::Function,
        CodeUnitKind::Struct,
        CodeUnitKind::Enum,
        CodeUnitKind::Trait,
        CodeUnitKind::Module,
    ];

    assert_eq!(kinds.len(), 5);
}

// ============================================================================
// Unit Filters Tests
// ============================================================================

#[test]
fn test_unit_filters_default() {
    let filters = UnitFilters::default();
    assert!(filters.kinds.is_none());
    assert!(filters.name_pattern.is_none());
}

#[test]
fn test_unit_filters_custom() {
    let filters = UnitFilters {
        kinds: Some(vec![CodeUnitKind::Function, CodeUnitKind::Struct]),
        name_pattern: Some("test_*".to_string()),
        file_pattern: Some("*.rs".to_string()),
    };

    assert_eq!(filters.kinds.as_ref().unwrap().len(), 2);
    assert_eq!(filters.name_pattern, Some("test_*".to_string()));
}

// ============================================================================
// Health Status Tests
// ============================================================================

#[test]
fn test_health_status() {
    let status = HealthStatus {
        status: "healthy".to_string(),
        version: "v3.0.0".to_string(),
        uptime_seconds: 3600,
        active_sessions: 5,
    };

    assert_eq!(status.status, "healthy");
    assert_eq!(status.active_sessions, 5);
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_cortex_error_network() {
    let error = CortexError::Network("Connection failed".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Network error"));
}

#[test]
fn test_cortex_error_api() {
    let error = CortexError::Api {
        status: 404,
        message: "Not found".to_string(),
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("API error"));
    assert!(error_msg.contains("404"));
}

#[test]
fn test_cortex_error_serialization() {
    let error = CortexError::Serialization("Invalid JSON".to_string());
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("Serialization error"));
}

#[test]
fn test_cortex_error_not_found() {
    let error = CortexError::NotFound {
        resource_type: "session".to_string(),
        resource_id: "sess-123".to_string(),
    };
    let error_msg = format!("{}", error);
    assert!(error_msg.contains("session"));
    assert!(error_msg.contains("sess-123"));
}
