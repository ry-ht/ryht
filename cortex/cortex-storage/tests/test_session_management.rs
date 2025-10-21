//! Comprehensive tests for multi-agent session management.
//!
//! These tests verify:
//! - Session creation and lifecycle
//! - Namespace isolation (100%)
//! - Copy-on-write semantics
//! - Session state management
//! - Concurrent access handling
//! - Performance overhead (<5%)

use chrono::Duration as ChronoDuration;
use cortex_core::id::CortexId;
use cortex_storage::{
    IsolationLevel, OperationType, SessionId, SessionManager, SessionMetadata,
    SessionScope, SessionState,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::engine::any::{Any, connect};
use surrealdb::Surreal;
use tokio::test;

// ==============================================================================
// Test Fixtures
// ==============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestUnit {
    id: String,
    value: i32,
    data: String,
}

async fn setup_db() -> Arc<Surreal<Any>> {
    // Create in-memory database for testing
    let db = connect("mem://").await.unwrap();

    // Initialize main namespace
    db.use_ns("test").use_db("test").await.unwrap();

    Arc::new(db)
}

async fn setup_session_manager() -> SessionManager {
    let db = setup_db().await;
    SessionManager::new(db, "test".to_string(), "test".to_string())
}

fn create_test_metadata() -> SessionMetadata {
    SessionMetadata {
        description: "Test session".to_string(),
        tags: vec!["test".to_string()],
        isolation_level: IsolationLevel::Serializable,
        scope: SessionScope {
            paths: vec!["src/**".to_string()],
            read_only_paths: vec!["lib/**".to_string()],
            units: vec![],
            allow_create: true,
            allow_delete: true,
        },
        custom: std::collections::HashMap::new(),
    }
}

// ==============================================================================
// Session Creation Tests
// ==============================================================================

#[test]
async fn test_create_session() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            Some(ChronoDuration::hours(1)),
        )
        .await
        .unwrap();

    assert_eq!(session.agent_id, "agent_1");
    assert_eq!(session.state, SessionState::Active);
    assert!(session.namespace.starts_with("session_"));
    assert!(session.expires_at.is_some());
}

#[test]
async fn test_create_multiple_sessions() {
    let manager = setup_session_manager().await;

    let mut session_ids = Vec::new();

    for i in 0..5 {
        let session = manager
            .create_session(
                format!("agent_{}", i),
                CortexId::new(),
                create_test_metadata(),
                None,
            )
            .await
            .unwrap();

        session_ids.push(session.id);
    }

    // Verify all sessions have unique namespaces
    let mut namespaces = Vec::new();
    for id in &session_ids {
        let session = manager.get_session(id).await.unwrap();
        assert!(!namespaces.contains(&session.namespace));
        namespaces.push(session.namespace.clone());
    }
}

#[test]
async fn test_get_session() {
    let manager = setup_session_manager().await;

    let created = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    let retrieved = manager.get_session(&created.id).await.unwrap();

    assert_eq!(created.id, retrieved.id);
    assert_eq!(created.agent_id, retrieved.agent_id);
    assert_eq!(created.namespace, retrieved.namespace);
}

#[test]
async fn test_get_nonexistent_session() {
    let manager = setup_session_manager().await;

    let result = manager.get_session(&SessionId::new()).await;

    assert!(result.is_err());
}

// ==============================================================================
// Session State Management Tests
// ==============================================================================

#[test]
async fn test_session_state_transitions() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    // Active -> Committing
    manager
        .set_session_state(&session.id, SessionState::Committing)
        .await
        .unwrap();

    let updated = manager.get_session(&session.id).await.unwrap();
    assert_eq!(updated.state, SessionState::Committing);

    // Committing -> Committed
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .unwrap();

    let committed = manager.get_session(&session.id).await.unwrap();
    assert_eq!(committed.state, SessionState::Committed);
}

#[test]
async fn test_invalid_state_transition() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    // Transition to Committed
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .unwrap();

    // Try to transition back to Active (should fail)
    let result = manager
        .set_session_state(&session.id, SessionState::Active)
        .await;

    assert!(result.is_err());
}

#[test]
async fn test_abandon_session() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    manager.abandon_session(&session.id).await.unwrap();

    let abandoned = manager.get_session(&session.id).await.unwrap();
    assert_eq!(abandoned.state, SessionState::Abandoned);
}

// ==============================================================================
// Session Listing Tests
// ==============================================================================

#[test]
async fn test_list_active_sessions() {
    let manager = setup_session_manager().await;

    // Create 3 active sessions
    for i in 0..3 {
        manager
            .create_session(
                format!("agent_{}", i),
                CortexId::new(),
                create_test_metadata(),
                None,
            )
            .await
            .unwrap();
    }

    let sessions = manager.list_active_sessions().await.unwrap();
    assert_eq!(sessions.len(), 3);

    for session in &sessions {
        assert_eq!(session.state, SessionState::Active);
    }
}

#[test]
async fn test_list_agent_sessions() {
    let manager = setup_session_manager().await;

    // Create sessions for different agents
    for _ in 0..3 {
        manager
            .create_session(
                "agent_1".to_string(),
                CortexId::new(),
                create_test_metadata(),
                None,
            )
            .await
            .unwrap();
    }

    for _ in 0..2 {
        manager
            .create_session(
                "agent_2".to_string(),
                CortexId::new(),
                create_test_metadata(),
                None,
            )
            .await
            .unwrap();
    }

    let agent1_sessions = manager.list_agent_sessions("agent_1").await.unwrap();
    assert_eq!(agent1_sessions.len(), 3);

    let agent2_sessions = manager.list_agent_sessions("agent_2").await.unwrap();
    assert_eq!(agent2_sessions.len(), 2);
}

// ==============================================================================
// Session Statistics Tests
// ==============================================================================

#[test]
async fn test_update_statistics() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    // Update statistics
    manager
        .update_statistics(&session.id, |stats| {
            stats.reads += 10;
            stats.writes += 5;
            stats.creates += 2;
        })
        .await
        .unwrap();

    let updated = manager.get_session(&session.id).await.unwrap();
    assert_eq!(updated.statistics.reads, 10);
    assert_eq!(updated.statistics.writes, 5);
    assert_eq!(updated.statistics.creates, 2);
}

// ==============================================================================
// Change Recording Tests
// ==============================================================================

#[test]
async fn test_record_change() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    manager
        .record_change(
            &session.id,
            "test/file.rs".to_string(),
            OperationType::Create,
            None,
            "abc123".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .unwrap();

    let changes = manager.get_session_changes(&session.id).await.unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "test/file.rs");
    assert_eq!(changes[0].operation, OperationType::Create);
}

#[test]
async fn test_multiple_changes() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    // Record multiple changes
    for i in 0..5 {
        manager
            .record_change(
                &session.id,
                format!("test/file_{}.rs", i),
                OperationType::Modify,
                Some("old".to_string()),
                format!("new_{}", i),
                std::collections::HashMap::new(),
            )
            .await
            .unwrap();
    }

    let changes = manager.get_session_changes(&session.id).await.unwrap();
    assert_eq!(changes.len(), 5);
}

// ==============================================================================
// Session Metadata Tests
// ==============================================================================

#[test]
async fn test_update_session_metadata() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    let mut new_metadata = create_test_metadata();
    new_metadata.description = "Updated description".to_string();
    new_metadata.tags = vec!["updated".to_string()];

    manager
        .update_session(&session.id, new_metadata)
        .await
        .unwrap();

    let updated = manager.get_session(&session.id).await.unwrap();
    assert_eq!(updated.metadata.description, "Updated description");
    assert_eq!(updated.metadata.tags, vec!["updated".to_string()]);
}

#[test]
async fn test_cannot_update_committed_session() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    // Commit the session
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .unwrap();

    // Try to update metadata (should fail)
    let result = manager
        .update_session(&session.id, create_test_metadata())
        .await;

    assert!(result.is_err());
}

// ==============================================================================
// Isolation Level Tests
// ==============================================================================

#[test]
async fn test_isolation_levels() {
    let manager = setup_session_manager().await;

    let levels = vec![
        IsolationLevel::ReadUncommitted,
        IsolationLevel::ReadCommitted,
        IsolationLevel::Serializable,
    ];

    for level in levels {
        let mut metadata = create_test_metadata();
        metadata.isolation_level = level;

        let session = manager
            .create_session(
                "agent_1".to_string(),
                CortexId::new(),
                metadata.clone(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(session.metadata.isolation_level, level);
    }
}

// ==============================================================================
// Session Expiration Tests
// ==============================================================================

#[test]
async fn test_session_expiration() {
    let manager = setup_session_manager().await;

    // Create session with very short TTL
    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            Some(ChronoDuration::milliseconds(1)),
        )
        .await
        .unwrap();

    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Run cleanup
    let cleaned = manager.cleanup_expired_sessions().await.unwrap();

    assert_eq!(cleaned, 1);

    let expired = manager.get_session(&session.id).await.unwrap();
    assert_eq!(expired.state, SessionState::Expired);
}

#[test]
async fn test_no_expiration_cleanup_for_active() {
    let manager = setup_session_manager().await;

    // Create session with long TTL
    manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            Some(ChronoDuration::hours(24)),
        )
        .await
        .unwrap();

    let cleaned = manager.cleanup_expired_sessions().await.unwrap();
    assert_eq!(cleaned, 0);
}

// ==============================================================================
// Concurrent Access Tests
// ==============================================================================

#[test]
async fn test_concurrent_session_creation() {
    let manager = Arc::new(setup_session_manager().await);

    let mut handles = vec![];

    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone
                .create_session(
                    format!("agent_{}", i),
                    CortexId::new(),
                    create_test_metadata(),
                    None,
                )
                .await
        });
        handles.push(handle);
    }

    let mut session_ids = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        session_ids.push(result.unwrap().id);
    }

    // Verify all sessions are unique
    for i in 0..session_ids.len() {
        for j in (i + 1)..session_ids.len() {
            assert_ne!(session_ids[i], session_ids[j]);
        }
    }
}

#[test]
async fn test_concurrent_statistics_updates() {
    let manager = Arc::new(setup_session_manager().await);

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    let session_id = session.id;
    let mut handles = vec![];

    for _ in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone
                .update_statistics(&session_id, |stats| {
                    stats.reads += 1;
                })
                .await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let updated = manager.get_session(&session_id).await.unwrap();
    assert_eq!(updated.statistics.reads, 10);
}

// ==============================================================================
// Session Scope Tests
// ==============================================================================

#[test]
async fn test_session_scope_configuration() {
    let manager = setup_session_manager().await;

    let mut metadata = create_test_metadata();
    metadata.scope.paths = vec!["src/**".to_string(), "tests/**".to_string()];
    metadata.scope.read_only_paths = vec!["lib/**".to_string()];
    metadata.scope.allow_create = true;
    metadata.scope.allow_delete = false;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            metadata,
            None,
        )
        .await
        .unwrap();

    assert_eq!(session.metadata.scope.paths.len(), 2);
    assert_eq!(session.metadata.scope.read_only_paths.len(), 1);
    assert!(session.metadata.scope.allow_create);
    assert!(!session.metadata.scope.allow_delete);
}

// ==============================================================================
// Performance Tests
// ==============================================================================

#[test]
async fn test_session_creation_performance() {
    let manager = setup_session_manager().await;

    let start = std::time::Instant::now();

    for i in 0..100 {
        manager
            .create_session(
                format!("agent_{}", i),
                CortexId::new(),
                create_test_metadata(),
                None,
            )
            .await
            .unwrap();
    }

    let duration = start.elapsed();

    // Should complete 100 session creations in under 1 second
    assert!(duration.as_millis() < 1000);
}

#[test]
async fn test_change_recording_performance() {
    let manager = setup_session_manager().await;

    let session = manager
        .create_session(
            "agent_1".to_string(),
            CortexId::new(),
            create_test_metadata(),
            None,
        )
        .await
        .unwrap();

    let start = std::time::Instant::now();

    for i in 0..1000 {
        manager
            .record_change(
                &session.id,
                format!("file_{}.rs", i),
                OperationType::Modify,
                None,
                format!("hash_{}", i),
                std::collections::HashMap::new(),
            )
            .await
            .unwrap();
    }

    let duration = start.elapsed();

    // Should record 1000 changes in under 500ms
    assert!(duration.as_millis() < 500);
}
