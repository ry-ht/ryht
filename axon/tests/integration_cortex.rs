//! Integration tests for Cortex API (with mocks)
//!
//! Tests Cortex integration including sessions, memory, and search

mod common;

use axon::cortex_bridge::*;
use common::MockCortexServer;

// ============================================================================
// Mock Cortex Integration Tests
// ============================================================================

#[tokio::test]
async fn test_mock_cortex_session_creation() {
    let server = MockCortexServer::new();
    let session_id = server.create_session("test-session".to_string()).await;

    assert_eq!(session_id, "test-session");

    let session = server.get_session(&session_id).await;
    assert!(session.is_some());
}

#[tokio::test]
async fn test_mock_cortex_multiple_sessions() {
    let server = MockCortexServer::new();

    let session1 = server.create_session("session-1".to_string()).await;
    let session2 = server.create_session("session-2".to_string()).await;

    assert_ne!(session1, session2);

    assert!(server.get_session(&session1).await.is_some());
    assert!(server.get_session(&session2).await.is_some());
}

#[tokio::test]
async fn test_mock_cortex_episode_storage() {
    use chrono::Utc;

    let server = MockCortexServer::new();

    let episode = Episode {
        id: EpisodeId::from("ep-001".to_string()),
        agent_id: AgentId::from("agent-1".to_string()),
        session_id: Some(SessionId::from("session-1".to_string())),
        task_type: "test".to_string(),
        task_description: "Test task".to_string(),
        context: serde_json::json!({}),
        action_taken: "Action".to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({}),
        learned_patterns: vec![],
        timestamp: Utc::now(),
    };

    let episode_id = server.store_episode(episode).await;
    assert!(!episode_id.is_empty());
}

#[tokio::test]
async fn test_mock_cortex_pattern_storage() {
    use chrono::Utc;

    let server = MockCortexServer::new();

    let pattern = Pattern {
        id: "pattern-001".to_string(),
        name: "Test Pattern".to_string(),
        pattern_type: PatternType::CodePattern,
        description: "Test".to_string(),
        context: "Test context".to_string(),
        transformation: serde_json::json!({}),
        success_rate: 0.9,
        times_applied: 10,
        average_improvement: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let pattern_id = server.store_pattern(pattern).await;
    assert!(!pattern_id.is_empty());
}

// ============================================================================
// Type Integration Tests
// ============================================================================

#[test]
fn test_agent_id_session_id_compatibility() {
    let agent_id = AgentId::from("agent-123".to_string());
    let session_id = SessionId::from("session-456".to_string());

    assert_eq!(agent_id.to_string(), "agent-123");
    assert_eq!(session_id.to_string(), "session-456");
}

#[test]
fn test_workspace_session_relationship() {
    let workspace_id = WorkspaceId::from("workspace-abc".to_string());
    let session_id = SessionId::from("session-xyz".to_string());

    // Both IDs should be valid
    assert!(!workspace_id.to_string().is_empty());
    assert!(!session_id.to_string().is_empty());
}

// ============================================================================
// Episode Integration Tests
// ============================================================================

#[test]
fn test_episode_with_session() {
    use chrono::Utc;

    let episode = Episode {
        id: EpisodeId::from("ep-001".to_string()),
        agent_id: AgentId::from("agent-1".to_string()),
        session_id: Some(SessionId::from("session-1".to_string())),
        task_type: "code_generation".to_string(),
        task_description: "Generate code".to_string(),
        context: serde_json::json!({"lang": "rust"}),
        action_taken: "Generated code".to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({"lines": 100}),
        learned_patterns: vec!["pattern-1".to_string()],
        timestamp: Utc::now(),
    };

    assert!(episode.session_id.is_some());
    assert_eq!(episode.learned_patterns.len(), 1);
}

#[test]
fn test_episode_without_session() {
    use chrono::Utc;

    let episode = Episode {
        id: EpisodeId::from("ep-002".to_string()),
        agent_id: AgentId::from("agent-2".to_string()),
        session_id: None,
        task_type: "analysis".to_string(),
        task_description: "Analyze code".to_string(),
        context: serde_json::json!({}),
        action_taken: "Performed analysis".to_string(),
        outcome: EpisodeOutcome::Success,
        success_metrics: serde_json::json!({}),
        learned_patterns: vec![],
        timestamp: Utc::now(),
    };

    assert!(episode.session_id.is_none());
}

// ============================================================================
// Pattern Integration Tests
// ============================================================================

#[test]
fn test_pattern_type_classification() {
    use chrono::Utc;

    let patterns = vec![
        Pattern {
            id: "p1".to_string(),
            name: "Code Pattern".to_string(),
            pattern_type: PatternType::CodePattern,
            description: "".to_string(),
            context: "".to_string(),
            transformation: serde_json::json!({}),
            success_rate: 0.9,
            times_applied: 1,
            average_improvement: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        Pattern {
            id: "p2".to_string(),
            name: "Arch Pattern".to_string(),
            pattern_type: PatternType::ArchitecturePattern,
            description: "".to_string(),
            context: "".to_string(),
            transformation: serde_json::json!({}),
            success_rate: 0.8,
            times_applied: 1,
            average_improvement: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    assert_eq!(patterns.len(), 2);
    assert!(matches!(patterns[0].pattern_type, PatternType::CodePattern));
    assert!(matches!(
        patterns[1].pattern_type,
        PatternType::ArchitecturePattern
    ));
}

// ============================================================================
// Search Integration Tests
// ============================================================================

#[test]
fn test_search_filters_integration() {
    let filters = SearchFilters {
        file_types: Some(vec!["rs".to_string(), "toml".to_string()]),
        min_score: Some(0.7),
        max_results: Some(20),
        include_archived: false,
    };

    assert_eq!(filters.file_types.as_ref().unwrap()[0], "rs");
    assert_eq!(filters.max_results, Some(20));
}

#[test]
fn test_code_search_result_creation() {
    let result = CodeSearchResult {
        file_path: "/src/main.rs".to_string(),
        line_number: 100,
        snippet: "fn main() {}".to_string(),
        score: 0.95,
        context: "Entry point".to_string(),
    };

    assert!(result.score > 0.9);
    assert_eq!(result.line_number, 100);
}

// ============================================================================
// File Operations Integration Tests
// ============================================================================

#[test]
fn test_file_info_with_types() {
    use chrono::Utc;

    let files = vec![
        FileInfo {
            path: "/src/lib.rs".to_string(),
            size: 2048,
            modified: Utc::now(),
            file_type: FileType::File,
        },
        FileInfo {
            path: "/src".to_string(),
            size: 0,
            modified: Utc::now(),
            file_type: FileType::Directory,
        },
    ];

    assert!(matches!(files[0].file_type, FileType::File));
    assert!(matches!(files[1].file_type, FileType::Directory));
}

// ============================================================================
// Merge Integration Tests
// ============================================================================

#[test]
fn test_merge_report_with_strategy() {
    let report = MergeReport {
        success: true,
        files_merged: 10,
        conflicts: vec![],
        message: "Auto-merge successful".to_string(),
    };

    let strategy = MergeStrategy::Auto;

    assert!(report.success);
    assert!(matches!(strategy, MergeStrategy::Auto));
}

#[test]
fn test_conflict_resolution() {
    let conflicts = vec![
        MergeConflict {
            file_path: "/src/main.rs".to_string(),
            conflict_type: ConflictType::ContentConflict,
            description: "Both modified".to_string(),
        },
        MergeConflict {
            file_path: "/src/lib.rs".to_string(),
            conflict_type: ConflictType::DeleteModifyConflict,
            description: "Deleted in one branch".to_string(),
        },
    ];

    assert_eq!(conflicts.len(), 2);
    assert!(matches!(
        conflicts[0].conflict_type,
        ConflictType::ContentConflict
    ));
}
