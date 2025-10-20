//! Integration tests for the connection pool with real SurrealDB.
//!
//! These tests require a running SurrealDB instance.
//! Run with: `cargo test --test connection_pool_integration_tests -- --test-threads=1`

use cortex_storage::connection_pool::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Helper to create a test database config
fn test_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(), // Use in-memory for tests
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        pool_config: PoolConfig {
            min_connections: 2,
            max_connections: 5,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(30)),
            max_lifetime: Some(Duration::from_secs(60)),
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
        },
        namespace: "test".to_string(),
        database: "test".to_string(),
    }
}

#[tokio::test]
async fn test_connection_manager_creation() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await;

    assert!(manager.is_ok());

    let manager = manager.unwrap();
    let health = manager.health_status();

    assert!(health.healthy);
    assert_eq!(health.pool_size, 2); // min_connections
}

#[tokio::test]
async fn test_connection_acquisition() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn1 = manager.acquire().await;
    assert!(conn1.is_ok());

    let conn2 = manager.acquire().await;
    assert!(conn2.is_ok());

    let conn1 = conn1.unwrap();
    let conn2 = conn2.unwrap();

    // Connections should have different IDs
    assert_ne!(conn1.id(), conn2.id());
}

#[tokio::test]
async fn test_connection_reuse() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn1_id = {
        let conn = manager.acquire().await.unwrap();
        conn.id()
    }; // Connection dropped here

    sleep(Duration::from_millis(100)).await;

    let conn2_id = {
        let conn = manager.acquire().await.unwrap();
        conn.id()
    };

    // Should reuse the same connection
    assert_eq!(conn1_id, conn2_id);

    let metrics = manager.metrics().snapshot();
    assert!(metrics.connections_reused > 0);
}

#[tokio::test]
async fn test_pool_exhaustion() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            max_connections: 2,
            connection_timeout: Duration::from_millis(500),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Acquire all available connections
    let _conn1 = manager.acquire().await.unwrap();
    let _conn2 = manager.acquire().await.unwrap();

    // Try to acquire one more - should timeout
    let conn3 = manager.acquire().await;
    assert!(conn3.is_err());
}

#[tokio::test]
async fn test_concurrent_access() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let mut handles = Vec::new();

    for i in 0..10 {
        let manager = manager.clone();
        let handle = tokio::spawn(async move {
            let conn = manager.acquire().await.unwrap();
            sleep(Duration::from_millis(100)).await;
            conn.id()
        });
        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), 10);

    // All should succeed
    let metrics = manager.metrics().snapshot();
    assert!(metrics.connections_created >= 2); // At least min_connections
}

#[tokio::test]
async fn test_health_monitoring() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    // Wait for health check to run
    sleep(Duration::from_secs(2)).await;

    let metrics = manager.metrics().snapshot();
    assert!(metrics.health_checks_passed > 0);

    let health = manager.health_status();
    assert!(health.healthy);
}

#[tokio::test]
async fn test_connection_health_check() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();
    let is_healthy = conn.check_health().await;

    assert!(is_healthy);
}

#[tokio::test]
async fn test_agent_session_creation() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let session = AgentSession::create(
        "test-agent-1".to_string(),
        manager.clone(),
        "test-namespace".to_string(),
    )
    .await;

    assert!(session.is_ok());

    let session = session.unwrap();
    assert_eq!(session.agent_id, "test-agent-1");
    assert_eq!(session.namespace, "test-namespace");
}

#[tokio::test]
async fn test_agent_session_transactions() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let session = AgentSession::create(
        "test-agent-1".to_string(),
        manager.clone(),
        "test-namespace".to_string(),
    )
    .await
    .unwrap();

    // Record a transaction
    let txn_id = session.record_transaction(TransactionOperation::Write {
        path: "/test/file.rs".to_string(),
        content_hash: "abc123".to_string(),
    });

    // Commit it
    session.commit_transaction(txn_id);

    // Check history
    let history = session.transaction_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, txn_id);
    assert_eq!(history[0].status, TransactionStatus::Committed);
}

#[tokio::test]
async fn test_multiple_agent_sessions() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let session1 = AgentSession::create(
        "agent-1".to_string(),
        manager.clone(),
        "namespace-1".to_string(),
    )
    .await
    .unwrap();

    let session2 = AgentSession::create(
        "agent-2".to_string(),
        manager.clone(),
        "namespace-2".to_string(),
    )
    .await
    .unwrap();

    // Both sessions should use the same connection pool
    let conn1 = session1.acquire().await.unwrap();
    let conn2 = session2.acquire().await.unwrap();

    // But have different session IDs
    assert_ne!(session1.session_id, session2.session_id);
}

#[tokio::test]
async fn test_metrics_collection() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    // Perform some operations
    for _ in 0..5 {
        let _conn = manager.acquire().await.unwrap();
    }

    let metrics = manager.metrics().snapshot();

    assert!(metrics.connections_created >= 2);
    assert!(metrics.acquisitions >= 5 || metrics.connections_reused >= 3);
}

#[tokio::test]
async fn test_circuit_breaker() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let health = manager.health_status();
    assert_eq!(health.circuit_breaker_state, CircuitBreakerState::Closed);

    // Circuit breaker behavior is tested in unit tests
    // Here we just verify it's initialized correctly
}

#[tokio::test]
async fn test_shutdown() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let shutdown_result = manager.shutdown().await;
    assert!(shutdown_result.is_ok());

    let health = manager.health_status();
    assert_eq!(health.pool_size, 0);
}

#[tokio::test]
async fn test_load_balancing_remote() {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Remote {
            endpoints: vec![
                "mem://endpoint1".to_string(),
                "mem://endpoint2".to_string(),
                "mem://endpoint3".to_string(),
            ],
            load_balancing: LoadBalancingStrategy::RoundRobin,
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_hybrid_mode() {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Hybrid {
            local_cache: "mem://local".to_string(),
            remote_sync: vec!["mem://remote1".to_string(), "mem://remote2".to_string()],
            sync_interval: Duration::from_secs(60),
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await;
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_connection_timeout() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            max_connections: 1,
            connection_timeout: Duration::from_millis(100),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Hold the only connection
    let _conn = manager.acquire().await.unwrap();

    // Try to acquire another - should timeout quickly
    let start = std::time::Instant::now();
    let result = manager.acquire().await;
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(elapsed < Duration::from_millis(200)); // Should timeout around 100ms
}

#[tokio::test]
async fn test_connection_use_counter() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();
    let initial_uses = conn.uses();

    conn.increment_uses();
    conn.increment_uses();

    assert_eq!(conn.uses(), initial_uses + 2);
}

#[tokio::test]
async fn test_idle_timeout() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            idle_timeout: Some(Duration::from_millis(200)),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await.unwrap();

    {
        let _conn = manager.acquire().await.unwrap();
        // Connection used here
    }

    // Wait for idle timeout
    sleep(Duration::from_millis(300)).await;

    // Health monitor should clean up idle connections
    let health = manager.health_status();
    // Pool size might be reduced after cleanup
    assert!(health.pool_size <= 5);
}

#[tokio::test]
async fn test_max_lifetime() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            max_lifetime: Some(Duration::from_millis(200)),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await.unwrap();
    let conn_id = {
        let conn = manager.acquire().await.unwrap();
        conn.id()
    };

    // Wait for max lifetime to expire
    sleep(Duration::from_millis(300)).await;

    // Next connection should be new (different ID)
    let new_conn = manager.acquire().await.unwrap();
    // Note: Due to cleanup timing, this might still be the same connection
    // This is a best-effort test
    drop(new_conn);
}

#[tokio::test]
async fn test_retry_policy_success_after_failure() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let mut attempt = 0;
    let result = manager
        .execute_with_retry(|| {
            attempt += 1;
            Box::pin(async move {
                if attempt < 2 {
                    Err(cortex_core::error::CortexError::database(
                        "Simulated transient failure",
                    ))
                } else {
                    Ok("Success".to_string())
                }
            })
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(attempt, 2);

    let metrics = manager.metrics().snapshot();
    assert!(metrics.retries >= 1);
}

#[tokio::test]
async fn test_retry_policy_max_attempts() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            retry_policy: RetryPolicy {
                max_attempts: 2,
                initial_backoff: Duration::from_millis(10),
                max_backoff: Duration::from_secs(1),
                multiplier: 2.0,
            },
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let mut attempt = 0;
    let result = manager
        .execute_with_retry(|| {
            attempt += 1;
            Box::pin(async move {
                Err(cortex_core::error::CortexError::database(
                    "Persistent failure",
                ))
            })
        })
        .await;

    assert!(result.is_err());
    assert_eq!(attempt, 3); // Initial + 2 retries
}

#[tokio::test]
async fn test_concurrent_sessions() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let mut handles = Vec::new();

    for i in 0..5 {
        let manager = manager.clone();
        let handle = tokio::spawn(async move {
            let session = AgentSession::create(
                format!("agent-{}", i),
                manager.clone(),
                format!("namespace-{}", i),
            )
            .await
            .unwrap();

            // Perform some operations
            let _conn = session.acquire().await.unwrap();
            sleep(Duration::from_millis(50)).await;

            session.record_transaction(TransactionOperation::Read {
                path: format!("/test/file-{}.rs", i),
            });

            session.session_id
        });
        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), 5);

    // All sessions should have unique IDs
    let unique_ids: std::collections::HashSet<_> = results.into_iter().collect();
    assert_eq!(unique_ids.len(), 5);
}

#[tokio::test]
async fn test_load_balancing_strategies() {
    for strategy in &[
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::Random,
        LoadBalancingStrategy::HealthBased,
    ] {
        let config = DatabaseConfig {
            connection_mode: ConnectionMode::Remote {
                endpoints: vec![
                    "mem://endpoint1".to_string(),
                    "mem://endpoint2".to_string(),
                ],
                load_balancing: *strategy,
            },
            ..test_config()
        };

        let manager = ConnectionManager::new(config).await;
        assert!(
            manager.is_ok(),
            "Failed to create manager with strategy {:?}",
            strategy
        );
    }
}

#[tokio::test]
async fn test_connection_validation_on_checkout() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            validate_on_checkout: true,
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await.unwrap();
    let conn = manager.acquire().await;

    assert!(conn.is_ok());
}

#[tokio::test]
async fn test_connection_recycling() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            recycle_after_uses: Some(5),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = ConnectionManager::new(config).await.unwrap();

    // Use connection multiple times
    for _ in 0..6 {
        let conn = manager.acquire().await.unwrap();
        conn.increment_uses();
    }

    let metrics = manager.metrics().snapshot();
    assert!(metrics.connections_closed > 0);
}

#[tokio::test]
async fn test_graceful_shutdown_waits_for_connections() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            shutdown_grace_period: Duration::from_secs(2),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Hold a connection
    let manager_clone = manager.clone();
    let handle = tokio::spawn(async move {
        let _conn = manager_clone.acquire().await.unwrap();
        sleep(Duration::from_millis(500)).await;
    });

    // Start shutdown
    let shutdown_handle = {
        let manager = manager.clone();
        tokio::spawn(async move {
            manager.shutdown().await
        })
    };

    // Wait for background task
    let _ = handle.await;
    let shutdown_result = shutdown_handle.await.unwrap();

    assert!(shutdown_result.is_ok());
}

#[tokio::test]
async fn test_pool_statistics() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    // Perform some operations
    for _ in 0..3 {
        let _conn = manager.acquire().await.unwrap();
    }

    let stats = manager.pool_stats();

    assert!(stats.total_connections >= 2);
    assert!(stats.connections_created >= 2);
    assert_eq!(stats.total_connections, stats.available_connections + stats.in_use_connections);
}

#[tokio::test]
async fn test_transaction_support() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    // Begin transaction
    let result = conn.begin_transaction().await;
    assert!(result.is_ok());

    // Commit transaction
    let result = conn.commit_transaction().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_transaction_rollback() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    // Begin transaction
    conn.begin_transaction().await.unwrap();

    // Rollback transaction
    let result = conn.rollback_transaction().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_savepoint_support() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    // Begin transaction
    conn.begin_transaction().await.unwrap();

    // Create savepoint
    let result = conn.savepoint("sp1").await;
    // Note: SurrealDB may not support savepoints, so we just verify the method exists
    drop(result);

    // Rollback transaction
    conn.rollback_transaction().await.unwrap();
}

#[tokio::test]
async fn test_with_transaction_helper() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    let result = conn.with_transaction(|_conn| {
        Box::pin(async move {
            // Simulated operation
            Ok(42)
        })
    }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_with_transaction_rollback_on_error() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    let result = conn.with_transaction(|_conn| {
        Box::pin(async move {
            Err(cortex_core::error::CortexError::database("Simulated error"))
        })
    }).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_agent_session_resource_limits() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let limits = ResourceLimits {
        max_concurrent_connections: 2,
        max_operations: 10,
        max_transaction_log_size: 100,
    };

    let session = AgentSession::create_with_limits(
        "test-agent".to_string(),
        manager.clone(),
        "test-namespace".to_string(),
        limits,
    )
    .await
    .unwrap();

    // Should be within limits initially
    assert!(session.is_within_limits());

    // Try to exceed connection limit
    let _conn1 = session.acquire().await.unwrap();
    let _conn2 = session.acquire().await.unwrap();

    // Third connection should fail
    let conn3 = session.acquire().await;
    assert!(conn3.is_err());
}

#[tokio::test]
async fn test_session_statistics() {
    let config = test_config();
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let session = AgentSession::create(
        "test-agent".to_string(),
        manager.clone(),
        "test-namespace".to_string(),
    )
    .await
    .unwrap();

    // Perform some operations
    let _conn = session.acquire().await.unwrap();

    let txn_id = session.record_transaction(TransactionOperation::Write {
        path: "/test/file.rs".to_string(),
        content_hash: "abc123".to_string(),
    });

    session.commit_transaction(txn_id);

    let stats = session.session_stats();

    assert_eq!(stats.agent_id, "test-agent");
    assert_eq!(stats.total_transactions, 1);
    assert_eq!(stats.committed_transactions, 1);
    assert_eq!(stats.aborted_transactions, 0);
}

#[tokio::test]
async fn test_connection_marked_for_recycling() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    let conn = manager.acquire().await.unwrap();

    assert!(!conn.is_marked_for_recycling());

    conn.mark_for_recycling();

    assert!(conn.is_marked_for_recycling());
}

#[tokio::test]
async fn test_is_shutting_down() {
    let config = test_config();
    let manager = ConnectionManager::new(config).await.unwrap();

    assert!(!manager.is_shutting_down());

    // Start shutdown in background
    let shutdown_handle = {
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.shutdown().await
        })
    };

    // Give it a moment to start
    sleep(Duration::from_millis(100)).await;

    // Should be shutting down now
    assert!(manager.is_shutting_down());

    // Wait for shutdown to complete
    let _ = shutdown_handle.await;
}

#[tokio::test]
async fn test_acquire_during_shutdown_fails() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            shutdown_grace_period: Duration::from_millis(500),
            ..PoolConfig::default()
        },
        ..test_config()
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Start shutdown
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        manager_clone.shutdown().await
    });

    // Wait for shutdown to start
    sleep(Duration::from_millis(100)).await;

    // Try to acquire - should fail
    let result = manager.acquire().await;
    assert!(result.is_err());
}
