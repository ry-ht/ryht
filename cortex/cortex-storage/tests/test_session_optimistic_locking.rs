//! Comprehensive tests for session state transitions with optimistic locking.
//!
//! These tests verify that the version-based optimistic locking mechanism
//! correctly prevents race conditions during concurrent state transitions.

use cortex_storage::prelude::*;
use cortex_storage::{ConnectionManager, DatabaseConfig, PoolConnectionMode, SessionManager};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

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

/// Create a test session manager with in-memory database
async fn create_test_session_manager() -> SessionManager {
    let conn = create_test_connection_manager().await;
    let db = conn.acquire().await.expect("Failed to acquire connection");

    SessionManager::new(
        Arc::new(db.connection().clone()),
        "main".to_string(),
        "test".to_string(),
    )
}

/// Create a test session
async fn create_test_session(manager: &SessionManager) -> AgentSession {
    manager
        .create_session(
            "test-agent".to_string(),
            cortex_core::id::CortexId::new(),
            SessionMetadata::default(),
            Some(chrono::Duration::hours(1)),
        )
        .await
        .expect("Failed to create session")
}

// ==============================================================================
// Unit Tests - State Transition Validation
// ==============================================================================

#[tokio::test]
async fn test_valid_state_transitions() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Active -> Committing
    manager
        .set_session_state(&session.id, SessionState::Committing)
        .await
        .expect("Active -> Committing should succeed");

    // Committing -> Committed
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .expect("Committing -> Committed should succeed");
}

#[tokio::test]
async fn test_invalid_state_transitions() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // First transition to committed
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .expect("Active -> Committed should succeed");

    // Committed -> Active (invalid)
    let result = manager
        .set_session_state(&session.id, SessionState::Active)
        .await;

    assert!(
        result.is_err(),
        "Should not allow transition from Committed to Active"
    );
}

#[tokio::test]
async fn test_state_transition_from_abandoned() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Active -> Abandoned
    manager
        .set_session_state(&session.id, SessionState::Abandoned)
        .await
        .expect("Active -> Abandoned should succeed");

    // Abandoned -> Active (invalid - terminal state)
    let result = manager
        .set_session_state(&session.id, SessionState::Active)
        .await;

    assert!(
        result.is_err(),
        "Should not allow transition from terminal state Abandoned"
    );
}

#[tokio::test]
async fn test_committing_rollback_to_active() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Active -> Committing
    manager
        .set_session_state(&session.id, SessionState::Committing)
        .await
        .expect("Active -> Committing should succeed");

    // Committing -> Active (valid - rollback on failure)
    manager
        .set_session_state(&session.id, SessionState::Active)
        .await
        .expect("Committing -> Active should succeed (rollback)");
}

// ==============================================================================
// Integration Tests - Optimistic Locking
// ==============================================================================

#[tokio::test]
async fn test_concurrent_state_transition_with_version_check() {
    let manager = Arc::new(create_test_session_manager().await);
    let session = create_test_session(&manager).await;
    let session_id = session.id;

    // Get initial version
    let initial_session = manager
        .get_session(&session_id)
        .await
        .expect("Should retrieve session");
    assert_eq!(initial_session.version, 1, "Initial version should be 1");

    // First transition succeeds
    manager
        .set_session_state(&session_id, SessionState::Committing)
        .await
        .expect("First transition should succeed");

    // Verify version was incremented
    let updated_session = manager
        .get_session(&session_id)
        .await
        .expect("Should retrieve session");
    assert_eq!(
        updated_session.version, 2,
        "Version should be incremented to 2"
    );

    // Simulate concurrent modification by trying to transition again
    let result = manager
        .set_session_state(&session_id, SessionState::Committed)
        .await;

    assert!(
        result.is_ok(),
        "Second transition from updated state should succeed"
    );

    // Verify final version
    let final_session = manager
        .get_session(&session_id)
        .await
        .expect("Should retrieve session");
    assert_eq!(final_session.version, 3, "Version should be incremented to 3");
}

#[tokio::test]
async fn test_concurrent_state_transitions_multiple_tasks() {
    let manager = Arc::new(create_test_session_manager().await);
    let session = create_test_session(&manager).await;
    let session_id = session.id;

    // Spawn multiple concurrent tasks trying to transition state
    let mut tasks = JoinSet::new();

    for i in 0..5 {
        let manager_clone = Arc::clone(&manager);
        let id = session_id;

        tasks.spawn(async move {
            tokio::time::sleep(Duration::from_millis(i * 10)).await;
            manager_clone.set_session_state(&id, SessionState::Committing).await
        });
    }

    // Collect results
    let mut success_count = 0;
    let mut conflict_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => {
                // Check if it's a concurrency error
                if e.to_string().contains("concurrent") || e.to_string().contains("version") {
                    conflict_count += 1;
                } else {
                    panic!("Unexpected error: {}", e);
                }
            }
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    // Only one transition should succeed due to version-based locking
    assert_eq!(
        success_count, 1,
        "Only one concurrent transition should succeed"
    );
    assert!(
        conflict_count >= 4,
        "At least 4 transitions should fail due to version conflicts"
    );
}

#[tokio::test]
async fn test_version_increment_on_each_transition() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Track version across multiple transitions
    let versions = vec![
        (SessionState::Active, 1),
        (SessionState::Committing, 2),
        (SessionState::Active, 3),    // rollback
        (SessionState::Abandoned, 4), // final state
    ];

    for (expected_state, expected_version) in versions {
        if expected_state == SessionState::Active && expected_version == 1 {
            // Initial state, skip
            continue;
        }

        manager
            .set_session_state(&session.id, expected_state)
            .await
            .expect("State transition should succeed");

        let updated = manager
            .get_session(&session.id)
            .await
            .expect("Should retrieve session");

        assert_eq!(
            updated.state, expected_state,
            "State should match expected"
        );
        assert_eq!(
            updated.version, expected_version,
            "Version should be {}",
            expected_version
        );
    }
}

// ==============================================================================
// Stress Tests - High Concurrency
// ==============================================================================

#[tokio::test]
async fn test_high_concurrency_state_transitions() {
    let manager = Arc::new(create_test_session_manager().await);
    let session = create_test_session(&manager).await;
    let session_id = session.id;

    // Spawn 50 concurrent tasks
    let mut tasks = JoinSet::new();

    for i in 0..50 {
        let manager_clone = Arc::clone(&manager);
        let id = session_id;

        tasks.spawn(async move {
            tokio::time::sleep(Duration::from_micros(i * 100)).await;

            // Try to transition state
            manager_clone.set_session_state(&id, SessionState::Committing).await
        });
    }

    // Collect results
    let mut success_count = 0;
    let mut error_count = 0;

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) => error_count += 1,
            Err(e) => panic!("Task panicked: {}", e),
        }
    }

    // Exactly one should succeed
    assert_eq!(success_count, 1, "Exactly one task should succeed");
    assert_eq!(
        error_count, 49,
        "All other tasks should fail due to version conflicts"
    );

    // Verify final state
    let final_session = manager
        .get_session(&session_id)
        .await
        .expect("Should retrieve session");

    assert_eq!(
        final_session.state,
        SessionState::Committing,
        "Final state should be Committing"
    );
    assert_eq!(final_session.version, 2, "Final version should be 2");
}

#[tokio::test]
async fn test_sequential_transitions_with_version_tracking() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Perform a series of valid transitions sequentially
    let transitions = vec![
        SessionState::Committing,
        SessionState::Active,
        SessionState::Committing,
        SessionState::Committed,
    ];

    let mut expected_version = 1;

    for new_state in transitions {
        expected_version += 1;

        manager
            .set_session_state(&session.id, new_state)
            .await
            .expect("Transition should succeed");

        let updated = manager
            .get_session(&session.id)
            .await
            .expect("Should retrieve session");

        assert_eq!(
            updated.version, expected_version,
            "Version should increment correctly"
        );
        assert_eq!(updated.state, new_state, "State should be updated");
    }
}

// ==============================================================================
// Edge Cases
// ==============================================================================

#[tokio::test]
async fn test_nonexistent_session_state_transition() {
    let manager = create_test_session_manager().await;
    let fake_id = cortex_core::id::CortexId::new();

    let result = manager
        .set_session_state(&fake_id, SessionState::Committed)
        .await;

    assert!(
        result.is_err(),
        "Should fail when session doesn't exist"
    );
}

#[tokio::test]
async fn test_version_stays_consistent_after_failed_transition() {
    let manager = create_test_session_manager().await;
    let session = create_test_session(&manager).await;

    // Transition to committed (terminal state)
    manager
        .set_session_state(&session.id, SessionState::Committed)
        .await
        .expect("Should transition to Committed");

    let before = manager
        .get_session(&session.id)
        .await
        .expect("Should retrieve session");

    // Try invalid transition (should fail)
    let _ = manager
        .set_session_state(&session.id, SessionState::Active)
        .await;

    let after = manager
        .get_session(&session.id)
        .await
        .expect("Should retrieve session");

    // Version should not change on failed transition
    assert_eq!(
        before.version, after.version,
        "Version should not change on failed transition"
    );
    assert_eq!(
        after.state,
        SessionState::Committed,
        "State should remain unchanged"
    );
}

#[tokio::test]
async fn test_multiple_sessions_independent_versions() {
    let manager = Arc::new(create_test_session_manager().await);

    // Create two sessions
    let session1 = create_test_session(&manager).await;
    let session2 = create_test_session(&manager).await;

    // Transition session1
    manager
        .set_session_state(&session1.id, SessionState::Committing)
        .await
        .expect("Session1 transition should succeed");

    // Transition session2
    manager
        .set_session_state(&session2.id, SessionState::Abandoned)
        .await
        .expect("Session2 transition should succeed");

    // Verify both have independent versions
    let s1 = manager
        .get_session(&session1.id)
        .await
        .expect("Should retrieve session1");
    let s2 = manager
        .get_session(&session2.id)
        .await
        .expect("Should retrieve session2");

    assert_eq!(s1.version, 2, "Session1 version should be 2");
    assert_eq!(s2.version, 2, "Session2 version should be 2");
    assert_eq!(s1.state, SessionState::Committing);
    assert_eq!(s2.state, SessionState::Abandoned);
}
