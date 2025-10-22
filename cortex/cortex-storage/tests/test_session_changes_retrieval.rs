//! Comprehensive tests for session change retrieval and tracking.
//!
//! These tests verify that the find_session_changes implementation correctly
//! retrieves and reconstructs all changes made within a session.

use cortex_storage::prelude::*;
use cortex_storage::{ConnectionManager, DatabaseConfig, MergeEngine, PoolConnectionMode};
use std::sync::Arc;
use std::time::Duration;

// ==============================================================================
// Test Helpers
// ==============================================================================

/// Create an in-memory test database connection manager
async fn create_test_connection_manager() -> Arc<ConnectionManager> {
    let config = DatabaseConfig {
        connection_mode: PoolConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: None,
            max_lifetime: None,
            retry_policy: RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: true,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(10),
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    };

    Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    )
}

/// Create a test merge engine
async fn create_test_merge_engine() -> MergeEngine {
    let storage = create_test_connection_manager().await;
    MergeEngine::new(storage)
}

/// Create a test session manager
async fn create_test_session_manager() -> SessionManager {
    let conn = create_test_connection_manager().await;
    let db = conn.acquire().await.expect("Failed to acquire connection");

    SessionManager::new(
        Arc::new(db.connection().clone()),
        "main".to_string(),
        "test".to_string(),
    )
}

// ==============================================================================
// Unit Tests - Basic Change Recording
// ==============================================================================

#[tokio::test]
async fn test_record_create_change() {
    let manager = create_test_session_manager().await;

    // Create a session
    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record a create operation
    manager
        .record_change(
            &session.id,
            "/test.rs".to_string(),
            OperationType::Create,
            None,
            "hash123".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    // Retrieve changes
    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(changes.len(), 1, "Should have one change");
    assert_eq!(changes[0].path, "/test.rs");
    assert_eq!(changes[0].operation, OperationType::Create);
}

#[tokio::test]
async fn test_record_modify_change() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record a modify operation
    manager
        .record_change(
            &session.id,
            "/existing.rs".to_string(),
            OperationType::Modify,
            Some("old_hash".to_string()),
            "new_hash".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].operation, OperationType::Modify);
    assert_eq!(changes[0].old_hash, Some("old_hash".to_string()));
    assert_eq!(changes[0].new_hash, "new_hash".to_string());
}

#[tokio::test]
async fn test_record_delete_change() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record a delete operation
    manager
        .record_change(
            &session.id,
            "/deleted.rs".to_string(),
            OperationType::Delete,
            Some("old_hash".to_string()),
            "deleted".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].operation, OperationType::Delete);
}

#[tokio::test]
async fn test_record_multiple_changes() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record multiple changes
    let operations = vec![
        OperationType::Create,
        OperationType::Modify,
        OperationType::Delete,
        OperationType::CopyOnWrite,
    ];

    for (i, op) in operations.iter().enumerate() {
        manager
            .record_change(
                &session.id,
                format!("/file{}.rs", i),
                *op,
                if *op == OperationType::Create {
                    None
                } else {
                    Some("old_hash".to_string())
                },
                format!("hash{}", i),
                std::collections::HashMap::new(),
            )
            .await
            .expect("Should record change");
    }

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(
        changes.len(),
        operations.len(),
        "Should have all changes"
    );
}

// ==============================================================================
// Integration Tests - Change Retrieval Order
// ==============================================================================

#[tokio::test]
async fn test_changes_ordered_by_timestamp() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record changes with delays to ensure timestamp ordering
    for i in 0..5 {
        tokio::time::sleep(Duration::from_millis(10)).await;

        manager
            .record_change(
                &session.id,
                format!("/file{}.rs", i),
                OperationType::Create,
                None,
                format!("hash{}", i),
                std::collections::HashMap::new(),
            )
            .await
            .expect("Should record change");
    }

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    // Verify ordering
    assert_eq!(changes.len(), 5);
    for i in 0..4 {
        assert!(
            changes[i].timestamp <= changes[i + 1].timestamp,
            "Changes should be ordered by timestamp"
        );
    }
}

#[tokio::test]
async fn test_empty_session_no_changes() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(changes.len(), 0, "Empty session should have no changes");
}

#[tokio::test]
async fn test_find_changes_from_merge_engine() {
    let engine = create_test_merge_engine().await;

    // Create a merge request for a session with no changes
    let request = MergeRequest {
        session_id: "test_session_changes".to_string(),
        target_namespace: "main".to_string(),
        strategy: MergeStrategy::AutoMerge,
        verify_semantics: false,
        allow_partial: false,
        conflict_resolution: None,
    };

    // Merge should handle session with no changes
    let result = engine
        .merge_session(request)
        .await
        .expect("Merge should succeed");

    assert_eq!(
        result.changes_applied, 0,
        "No changes should be found in empty session"
    );
}

// ==============================================================================
// Statistics Update Tests
// ==============================================================================

#[tokio::test]
async fn test_statistics_updated_on_create() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record create operation
    manager
        .record_change(
            &session.id,
            "/new.rs".to_string(),
            OperationType::Create,
            None,
            "hash123".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    // Get updated session
    let updated = manager
        .get_session(&session.id)
        .await
        .expect("Should get session");

    assert_eq!(updated.statistics.creates, 1, "Creates count should be 1");
    assert_eq!(updated.statistics.writes, 1, "Writes count should be 1");
}

#[tokio::test]
async fn test_statistics_updated_on_modify() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    manager
        .record_change(
            &session.id,
            "/existing.rs".to_string(),
            OperationType::Modify,
            Some("old".to_string()),
            "new".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    let updated = manager
        .get_session(&session.id)
        .await
        .expect("Should get session");

    assert_eq!(updated.statistics.updates, 1, "Updates count should be 1");
    assert_eq!(updated.statistics.writes, 1, "Writes count should be 1");
}

#[tokio::test]
async fn test_statistics_updated_on_delete() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    manager
        .record_change(
            &session.id,
            "/deleted.rs".to_string(),
            OperationType::Delete,
            Some("old".to_string()),
            "deleted".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    let updated = manager
        .get_session(&session.id)
        .await
        .expect("Should get session");

    assert_eq!(updated.statistics.deletes, 1, "Deletes count should be 1");
    assert_eq!(updated.statistics.writes, 1, "Writes count should be 1");
}

#[tokio::test]
async fn test_statistics_cow_operations() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    manager
        .record_change(
            &session.id,
            "/cow.rs".to_string(),
            OperationType::CopyOnWrite,
            Some("original".to_string()),
            "copied".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    let updated = manager
        .get_session(&session.id)
        .await
        .expect("Should get session");

    assert_eq!(
        updated.statistics.cow_operations, 1,
        "COW operations count should be 1"
    );
    assert_eq!(updated.statistics.writes, 1, "Writes count should be 1");
}

// ==============================================================================
// Concurrent Change Recording Tests
// ==============================================================================

#[tokio::test]
async fn test_concurrent_change_recording() {
    let manager = Arc::new(create_test_session_manager().await);

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    let session_id = session.id;

    // Spawn concurrent change recording tasks
    let mut handles = vec![];
    let num_tasks = 10;

    for i in 0..num_tasks {
        let manager_clone = Arc::clone(&manager);
        let sid = session_id;

        let handle = tokio::spawn(async move {
            manager_clone
                .record_change(
                    &sid,
                    format!("/concurrent{}.rs", i),
                    OperationType::Create,
                    None,
                    format!("hash{}", i),
                    std::collections::HashMap::new(),
                )
                .await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle
            .await
            .expect("Task should not panic")
            .expect("Should record change");
    }

    // Verify all changes were recorded
    let changes = manager
        .get_session_changes(&session_id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(
        changes.len(),
        num_tasks,
        "All concurrent changes should be recorded"
    );
}

// ==============================================================================
// Edge Cases and Error Handling
// ==============================================================================

#[tokio::test]
async fn test_get_changes_nonexistent_session() {
    let manager = create_test_session_manager().await;
    let fake_id = cortex_core::id::CortexId::new();

    let result = manager.get_session_changes(&fake_id).await;

    assert!(
        result.is_err(),
        "Should fail for nonexistent session"
    );
}

#[tokio::test]
async fn test_record_change_with_metadata() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("author".to_string(), "test-user".to_string());
    metadata.insert("reason".to_string(), "bug-fix".to_string());

    manager
        .record_change(
            &session.id,
            "/with_metadata.rs".to_string(),
            OperationType::Modify,
            Some("old".to_string()),
            "new".to_string(),
            metadata.clone(),
        )
        .await
        .expect("Should record change with metadata");

    let changes = manager
        .get_session_changes(&session.id)
        .await
        .expect("Should retrieve changes");

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].metadata.len(), metadata.len());
}

#[tokio::test]
async fn test_changes_persist_across_retrievals() {
    let manager = create_test_session_manager().await;

    let session = manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session");

    // Record a change
    manager
        .record_change(
            &session.id,
            "/persist.rs".to_string(),
            OperationType::Create,
            None,
            "hash123".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change");

    // Retrieve multiple times
    for _ in 0..3 {
        let changes = manager
            .get_session_changes(&session.id)
            .await
            .expect("Should retrieve changes");

        assert_eq!(changes.len(), 1, "Changes should persist");
        assert_eq!(changes[0].path, "/persist.rs");
    }
}

#[tokio::test]
async fn test_session_isolation_changes() {
    let manager = Arc::new(create_test_session_manager().await);

    // Create two sessions
    let session1 = manager
        .create_session(
            "agent-1".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session1");

    let session2 = manager
        .create_session(
            "agent-2".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Should create session2");

    // Record changes in each session
    manager
        .record_change(
            &session1.id,
            "/session1.rs".to_string(),
            OperationType::Create,
            None,
            "hash1".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change in session1");

    manager
        .record_change(
            &session2.id,
            "/session2.rs".to_string(),
            OperationType::Create,
            None,
            "hash2".to_string(),
            std::collections::HashMap::new(),
        )
        .await
        .expect("Should record change in session2");

    // Verify isolation
    let changes1 = manager
        .get_session_changes(&session1.id)
        .await
        .expect("Should get session1 changes");

    let changes2 = manager
        .get_session_changes(&session2.id)
        .await
        .expect("Should get session2 changes");

    assert_eq!(changes1.len(), 1, "Session1 should have 1 change");
    assert_eq!(changes2.len(), 1, "Session2 should have 1 change");
    assert_eq!(changes1[0].path, "/session1.rs");
    assert_eq!(changes2[0].path, "/session2.rs");
}
