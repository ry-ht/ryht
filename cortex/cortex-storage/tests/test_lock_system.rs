//! Comprehensive Lock System Tests
//!
//! This test suite covers:
//! - Basic locking (acquire, release)
//! - Lock compatibility (read/write/intent)
//! - Deadlock detection and resolution
//! - Lock timeout handling
//! - Session cleanup
//! - Performance benchmarks

use cortex_storage::locks::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ==============================================================================
// Basic Lock Tests
// ==============================================================================

#[tokio::test]
async fn test_acquire_write_lock() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let lock = manager.acquire_lock(&session, request).await.unwrap();
    assert_eq!(lock.entity_id, "entity1");
    assert_eq!(lock.lock_type, LockType::Write);
    assert_eq!(lock.holder_session, session);
}

#[tokio::test]
async fn test_acquire_read_lock() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::VNode,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let lock = manager.acquire_lock(&session, request).await.unwrap();
    assert_eq!(lock.lock_type, LockType::Read);
}

#[tokio::test]
async fn test_acquire_intent_lock() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::Workspace,
        lock_type: LockType::Intent,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let lock = manager.acquire_lock(&session, request).await.unwrap();
    assert_eq!(lock.lock_type, LockType::Intent);
}

#[tokio::test]
async fn test_release_lock() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let lock = manager.acquire_lock(&session, request).await.unwrap();
    let lock_id = lock.lock_id.clone();

    manager.release_lock(&lock_id).unwrap();
    assert!(!manager.is_locked("entity1").unwrap());
}

#[tokio::test]
async fn test_is_locked() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    assert!(!manager.is_locked("entity1").unwrap());

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session, request).await.unwrap();
    assert!(manager.is_locked("entity1").unwrap());
}

#[tokio::test]
async fn test_list_locks() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    assert_eq!(manager.list_locks().unwrap().len(), 0);

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session, request).await.unwrap();
    assert_eq!(manager.list_locks().unwrap().len(), 1);
}

// ==============================================================================
// Lock Compatibility Tests
// ==============================================================================

#[tokio::test]
async fn test_multiple_read_locks() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    manager.acquire_lock(&session2, request2).await.unwrap();

    assert_eq!(manager.list_locks().unwrap().len(), 2);
}

#[tokio::test]
async fn test_read_write_conflict() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    let result = manager.acquire_lock(&session2, request2).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_write_read_conflict() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    let result = manager.acquire_lock(&session2, request2).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_write_write_conflict() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    let result = manager.acquire_lock(&session2, request2).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_intent_compatible() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Intent,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    manager.acquire_lock(&session2, request2).await.unwrap();

    assert_eq!(manager.list_locks().unwrap().len(), 2);
}

#[tokio::test]
async fn test_intent_intent_compatible() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Intent,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Intent,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    manager.acquire_lock(&session2, request2).await.unwrap();

    assert_eq!(manager.list_locks().unwrap().len(), 2);
}

#[tokio::test]
async fn test_write_intent_conflict() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Intent,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    manager.acquire_lock(&session1, request1).await.unwrap();
    let result = manager.acquire_lock(&session2, request2).await;

    assert!(result.is_err());
}

// ==============================================================================
// Deadlock Detection Tests
// ==============================================================================

#[tokio::test]
async fn test_simple_deadlock_detection() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(50),
    ));

    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    // Session 1 locks entity A
    let request1_a = LockRequest {
        entity_id: "entityA".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session1, request1_a).await.unwrap();

    // Session 2 locks entity B
    let request2_b = LockRequest {
        entity_id: "entityB".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session2, request2_b).await.unwrap();

    // Create deadlock: session1 tries to lock B, session2 tries to lock A
    let manager1 = manager.clone();
    let manager2 = manager.clone();

    let handle1 = tokio::spawn(async move {
        let request = LockRequest {
            entity_id: "entityB".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_millis(500),
            metadata: None,
        };
        manager1.acquire_lock(&session1, request).await
    });

    let handle2 = tokio::spawn(async move {
        sleep(Duration::from_millis(50)).await;
        let request = LockRequest {
            entity_id: "entityA".to_string(),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_millis(500),
            metadata: None,
        };
        manager2.acquire_lock(&session2, request).await
    });

    let result1 = handle1.await.unwrap();
    let result2 = handle2.await.unwrap();

    // At least one should fail due to deadlock or timeout
    assert!(result1.is_err() || result2.is_err());
}

#[test]
fn test_wait_for_graph_cycle() {
    let mut graph = WaitForGraph::new();

    // Create cycle: A -> B -> C -> A
    graph.add_wait_edge("A".to_string(), "B".to_string());
    graph.add_wait_edge("B".to_string(), "C".to_string());
    graph.add_wait_edge("C".to_string(), "A".to_string());

    let cycle = graph.detect_cycle();
    assert!(cycle.is_some());
    assert_eq!(cycle.unwrap().len(), 3);
}

#[test]
fn test_wait_for_graph_no_cycle() {
    let mut graph = WaitForGraph::new();

    // No cycle: A -> B -> C
    graph.add_wait_edge("A".to_string(), "B".to_string());
    graph.add_wait_edge("B".to_string(), "C".to_string());

    let cycle = graph.detect_cycle();
    assert!(cycle.is_none());
}

#[test]
fn test_wait_for_graph_remove_edge() {
    let mut graph = WaitForGraph::new();

    graph.add_wait_edge("A".to_string(), "B".to_string());
    graph.add_wait_edge("B".to_string(), "C".to_string());
    graph.add_wait_edge("C".to_string(), "A".to_string());

    // Has cycle
    assert!(graph.detect_cycle().is_some());

    // Remove edge to break cycle
    graph.remove_wait_edge(&"C".to_string(), &"A".to_string());

    // No cycle
    assert!(graph.detect_cycle().is_none());
}

#[test]
fn test_wait_for_graph_path_detection() {
    let mut graph = WaitForGraph::new();

    graph.add_wait_edge("A".to_string(), "B".to_string());
    graph.add_wait_edge("B".to_string(), "C".to_string());

    assert!(graph.has_path(&"A".to_string(), &"C".to_string()));
    assert!(!graph.has_path(&"C".to_string(), &"A".to_string()));
}

// ==============================================================================
// Lock Timeout Tests
// ==============================================================================

#[tokio::test]
async fn test_lock_timeout() {
    let manager = LockManager::new(Duration::from_secs(1), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    // Session 1 acquires lock
    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session1, request1).await.unwrap();

    // Session 2 tries to acquire same lock with short timeout
    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    let result = manager.acquire_lock(&session2, request2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lock_expiration() {
    let manager = LockManager::new(Duration::from_secs(1), Duration::from_millis(100));
    let session = "session1".to_string();

    // Acquire lock with 1 second expiration
    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session, request).await.unwrap();

    // Wait for expiration
    sleep(Duration::from_millis(1100)).await;

    // Cleanup expired locks
    let cleaned = manager.cleanup_expired_locks().await;
    assert_eq!(cleaned, 1);

    // Lock should be released
    assert!(!manager.is_locked("entity1").unwrap());
}

// ==============================================================================
// Session Management Tests
// ==============================================================================

#[tokio::test]
async fn test_release_session_locks() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    // Acquire multiple locks
    for i in 0..5 {
        let request = LockRequest {
            entity_id: format!("entity{}", i),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };
        manager.acquire_lock(&session, request).await.unwrap();
    }

    assert_eq!(manager.list_locks().unwrap().len(), 5);

    // Release all locks for session
    let released = manager.release_session_locks(&session).unwrap();
    assert_eq!(released, 5);
    assert_eq!(manager.list_locks().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_session_locks() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    // Session 1 acquires 3 locks
    for i in 0..3 {
        let request = LockRequest {
            entity_id: format!("entity{}", i),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };
        manager.acquire_lock(&session1, request).await.unwrap();
    }

    // Session 2 acquires 2 locks
    for i in 3..5 {
        let request = LockRequest {
            entity_id: format!("entity{}", i),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };
        manager.acquire_lock(&session2, request).await.unwrap();
    }

    let session1_locks = manager.list_session_locks(&session1).unwrap();
    let session2_locks = manager.list_session_locks(&session2).unwrap();

    assert_eq!(session1_locks.len(), 3);
    assert_eq!(session2_locks.len(), 2);
}

#[tokio::test]
async fn test_list_entity_locks() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    // Both sessions acquire read locks on same entity
    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session1, request1).await.unwrap();

    let request2 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session2, request2).await.unwrap();

    let entity_locks = manager.list_entity_locks("entity1").unwrap();
    assert_eq!(entity_locks.len(), 2);
}

// ==============================================================================
// Statistics Tests
// ==============================================================================

#[tokio::test]
async fn test_lock_statistics() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let initial_stats = manager.statistics();
    assert_eq!(initial_stats.total_acquired, 0);
    assert_eq!(initial_stats.total_released, 0);

    // Acquire lock
    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    let lock = manager.acquire_lock(&session, request).await.unwrap();

    let stats = manager.statistics();
    assert_eq!(stats.total_acquired, 1);
    assert_eq!(stats.active_locks, 1);

    // Release lock
    manager.release_lock(&lock.lock_id).unwrap();

    let stats = manager.statistics();
    assert_eq!(stats.total_released, 1);
    assert_eq!(stats.active_locks, 0);
}

// ==============================================================================
// Lock Metadata Tests
// ==============================================================================

#[tokio::test]
async fn test_lock_with_metadata() {
    let manager = LockManager::new(Duration::from_secs(300), Duration::from_millis(100));
    let session = "session1".to_string();

    let metadata = LockMetadata {
        agent_id: Some("agent1".to_string()),
        purpose: Some("Testing metadata".to_string()),
        context: Some("Unit test".to_string()),
        renewal_count: 0,
    };

    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: Some(metadata),
    };

    let lock = manager.acquire_lock(&session, request).await.unwrap();
    assert_eq!(lock.metadata.agent_id, Some("agent1".to_string()));
    assert_eq!(lock.metadata.purpose, Some("Testing metadata".to_string()));
}

// ==============================================================================
// Concurrent Access Tests
// ==============================================================================

#[tokio::test]
async fn test_concurrent_lock_acquisitions() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    let mut handles = vec![];

    // 10 concurrent sessions trying to acquire locks
    for i in 0..10 {
        let manager_clone = manager.clone();
        let session = format!("session{}", i);

        let handle = tokio::spawn(async move {
            let request = LockRequest {
                entity_id: format!("entity{}", i),
                entity_type: EntityType::CodeUnit,
                lock_type: LockType::Write,
                timeout: Duration::from_secs(1),
                metadata: None,
            };
            manager_clone.acquire_lock(&session, request).await
        });

        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10);
    assert_eq!(manager.list_locks().unwrap().len(), 10);
}

#[tokio::test]
async fn test_concurrent_read_locks() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    let mut handles = vec![];

    // 20 concurrent read locks on same entity
    for i in 0..20 {
        let manager_clone = manager.clone();
        let session = format!("session{}", i);

        let handle = tokio::spawn(async move {
            let request = LockRequest {
                entity_id: "shared_entity".to_string(),
                entity_type: EntityType::CodeUnit,
                lock_type: LockType::Read,
                timeout: Duration::from_secs(1),
                metadata: None,
            };
            manager_clone.acquire_lock(&session, request).await
        });

        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 20);
    assert_eq!(manager.list_locks().unwrap().len(), 20);
}

// ==============================================================================
// Performance Benchmarks
// ==============================================================================

#[tokio::test]
async fn bench_lock_acquisition_performance() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    let start = std::time::Instant::now();
    let iterations = 100;

    for i in 0..iterations {
        let session = format!("session{}", i);
        let request = LockRequest {
            entity_id: format!("entity{}", i),
            entity_type: EntityType::CodeUnit,
            lock_type: LockType::Write,
            timeout: Duration::from_secs(1),
            metadata: None,
        };
        manager.acquire_lock(&session, request).await.unwrap();
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_micros() / iterations;

    println!("Average lock acquisition time: {} μs", avg_time);
    println!("Total time for {} locks: {:?}", iterations, elapsed);

    // Target: < 10ms per lock (10,000 μs)
    assert!(
        avg_time < 10_000,
        "Lock acquisition too slow: {} μs",
        avg_time
    );
}

#[tokio::test]
async fn bench_concurrent_lock_performance() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    let start = std::time::Instant::now();
    let mut handles = vec![];

    // 100 concurrent lock acquisitions
    for i in 0..100 {
        let manager_clone = manager.clone();
        let session = format!("session{}", i);

        let handle = tokio::spawn(async move {
            let request = LockRequest {
                entity_id: format!("entity{}", i),
                entity_type: EntityType::CodeUnit,
                lock_type: LockType::Write,
                timeout: Duration::from_secs(1),
                metadata: None,
            };
            manager_clone.acquire_lock(&session, request).await.unwrap();
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();
    println!("Time for 100 concurrent locks: {:?}", elapsed);

    // Should complete in reasonable time even under contention
    assert!(elapsed.as_secs() < 5);
}

#[tokio::test]
async fn bench_deadlock_detection_overhead() {
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    let session1 = "session1".to_string();
    let session2 = "session2".to_string();

    // Create potential deadlock scenario
    let request1 = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session1, request1).await.unwrap();

    let request2 = LockRequest {
        entity_id: "entity2".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };
    manager.acquire_lock(&session2, request2).await.unwrap();

    // Measure deadlock detection overhead
    let start = std::time::Instant::now();

    let request3 = LockRequest {
        entity_id: "entity2".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_millis(200),
        metadata: None,
    };

    let _ = manager.acquire_lock(&session1, request3).await;

    let elapsed = start.elapsed();
    println!("Deadlock detection overhead: {:?}", elapsed);

    // Deadlock detection should be fast
    assert!(elapsed.as_millis() < 500);
}
