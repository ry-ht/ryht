//! Load tests for the connection pool.
//!
//! These tests simulate high-concurrency scenarios to verify pool behavior under load.
//! Run with: `cargo test --test connection_pool_load_tests --release -- --test-threads=1`

use cortex_storage::connection_pool::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Helper to create a test database config
fn load_test_config(max_connections: usize) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials {
            username: Some("root".to_string()),
            password: Some("root".to_string()),
        },
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(60)),
            max_lifetime: Some(Duration::from_secs(300)),
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
            validate_on_checkout: true,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(30),
        },
        namespace: "load_test".to_string(),
        database: "load_test".to_string(),
    }
}

#[tokio::test]
async fn test_high_concurrency_reads() {
    let config = load_test_config(20);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let num_tasks = 100;
    let success_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut handles = Vec::new();

    for _i in 0..num_tasks {
        let manager = manager.clone();
        let success_count = success_count.clone();
        let error_count = error_count.clone();

        let handle = tokio::spawn(async move {
            match manager.acquire().await {
                Ok(_conn) => {
                    // Simulate read operation
                    sleep(Duration::from_millis(10)).await;
                    success_count.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    error_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let successes = success_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("High Concurrency Reads:");
    println!("  Tasks: {}", num_tasks);
    println!("  Successes: {}", successes);
    println!("  Errors: {}", errors);
    println!("  Duration: {:?}", elapsed);
    println!("  Throughput: {:.2} ops/sec", num_tasks as f64 / elapsed.as_secs_f64());

    assert_eq!(successes, num_tasks);
    assert_eq!(errors, 0);
}

#[tokio::test]
async fn test_sustained_load() {
    let config = load_test_config(15);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let duration = Duration::from_secs(5);
    let operation_count = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut handles = Vec::new();

    // Spawn 20 workers that continuously perform operations
    for _ in 0..20 {
        let manager = manager.clone();
        let operation_count = operation_count.clone();
        let error_count = error_count.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                match manager.acquire().await {
                    Ok(_conn) => {
                        // Simulate some work
                        sleep(Duration::from_millis(5)).await;
                        operation_count.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        error_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let operations = operation_count.load(Ordering::Relaxed);
    let errors = error_count.load(Ordering::Relaxed);

    println!("\nSustained Load:");
    println!("  Duration: {:?}", elapsed);
    println!("  Operations: {}", operations);
    println!("  Errors: {}", errors);
    println!("  Throughput: {:.2} ops/sec", operations as f64 / elapsed.as_secs_f64());

    assert!(operations > 0);
    // Allow some errors under extreme load
    assert!(errors < operations / 10); // Less than 10% error rate
}

#[tokio::test]
async fn test_burst_traffic() {
    let config = load_test_config(25);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let num_bursts = 5;
    let tasks_per_burst = 50;
    let total_operations = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    for burst in 0..num_bursts {
        println!("Burst {} of {}", burst + 1, num_bursts);

        let mut handles = Vec::new();

        for _ in 0..tasks_per_burst {
            let manager = manager.clone();
            let total_operations = total_operations.clone();

            let handle = tokio::spawn(async move {
                if let Ok(_conn) = manager.acquire().await {
                    sleep(Duration::from_millis(5)).await;
                    total_operations.fetch_add(1, Ordering::Relaxed);
                }
            });

            handles.push(handle);
        }

        futures::future::join_all(handles).await;

        // Short pause between bursts
        sleep(Duration::from_millis(100)).await;
    }

    let elapsed = start.elapsed();
    let operations = total_operations.load(Ordering::Relaxed);

    println!("\nBurst Traffic:");
    println!("  Bursts: {}", num_bursts);
    println!("  Tasks per burst: {}", tasks_per_burst);
    println!("  Total operations: {}", operations);
    println!("  Duration: {:?}", elapsed);

    assert!(operations >= (num_bursts * tasks_per_burst * 90 / 100) as u64); // At least 90% success
}

#[tokio::test]
async fn test_connection_churn() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            min_connections: 2,
            max_connections: 10,
            idle_timeout: Some(Duration::from_millis(500)),
            max_lifetime: Some(Duration::from_secs(2)),
            ..PoolConfig::default()
        },
        ..load_test_config(10)
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());
    let duration = Duration::from_secs(5);
    let start = Instant::now();

    let mut handles = Vec::new();

    // Spawn tasks that acquire and release connections rapidly
    for _ in 0..10 {
        let manager = manager.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                if let Ok(_conn) = manager.acquire().await {
                    // Very short hold time
                    sleep(Duration::from_millis(1)).await;
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let metrics = manager.metrics().snapshot();

    println!("\nConnection Churn:");
    println!("  Connections created: {}", metrics.connections_created);
    println!("  Connections reused: {}", metrics.connections_reused);
    println!("  Reuse ratio: {:.2}%",
        (metrics.connections_reused as f64 / (metrics.connections_created + metrics.connections_reused) as f64) * 100.0
    );

    // Should see significant reuse
    assert!(metrics.connections_reused > metrics.connections_created);
}

#[tokio::test]
async fn test_multi_agent_concurrent_sessions() {
    let config = load_test_config(30);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let num_agents = 50;
    let operations_per_agent = 10;

    let start = Instant::now();
    let mut handles = Vec::new();

    for agent_id in 0..num_agents {
        let manager = manager.clone();

        let handle = tokio::spawn(async move {
            let session = AgentSession::create(
                format!("agent-{}", agent_id),
                manager.clone(),
                format!("namespace-{}", agent_id),
            )
            .await
            .unwrap();

            for op in 0..operations_per_agent {
                match session.acquire().await {
                    Ok(_conn) => {
                        sleep(Duration::from_millis(2)).await;

                        session.record_transaction(TransactionOperation::Write {
                            path: format!("/agent-{}/file-{}.rs", agent_id, op),
                            content_hash: format!("hash-{}-{}", agent_id, op),
                        });
                    }
                    Err(_) => {}
                }
            }

            session.transaction_history().len()
        });

        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    let elapsed = start.elapsed();
    let total_transactions: usize = results.iter().sum();

    println!("\nMulti-Agent Concurrent Sessions:");
    println!("  Agents: {}", num_agents);
    println!("  Operations per agent: {}", operations_per_agent);
    println!("  Total transactions: {}", total_transactions);
    println!("  Duration: {:?}", elapsed);
    println!("  Throughput: {:.2} txn/sec", total_transactions as f64 / elapsed.as_secs_f64());

    assert_eq!(results.len(), num_agents);
}

#[tokio::test]
async fn test_pool_saturation_recovery() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            max_connections: 5,
            connection_timeout: Duration::from_secs(2),
            ..PoolConfig::default()
        },
        ..load_test_config(5)
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Phase 1: Saturate the pool
    println!("Phase 1: Saturating pool...");
    let connections: Vec<_> = (0..5)
        .map(|_| manager.acquire())
        .collect::<futures::future::JoinAll<_>>()
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(connections.len(), 5);

    // Phase 2: Try to acquire while saturated
    println!("Phase 2: Testing saturation...");
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        manager.acquire()
    ).await;
    assert!(result.is_err()); // Should timeout

    // Phase 3: Release all connections
    println!("Phase 3: Releasing connections...");
    drop(connections);
    sleep(Duration::from_millis(100)).await;

    // Phase 4: Verify recovery
    println!("Phase 4: Testing recovery...");
    let new_conn = manager.acquire().await;
    assert!(new_conn.is_ok());

    println!("Pool recovered successfully!");
}

#[tokio::test]
async fn test_retry_under_load() {
    let config = load_test_config(10);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let num_tasks = 50;
    let success_count = Arc::new(AtomicU64::new(0));
    let failure_count = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut handles = Vec::new();

    for i in 0..num_tasks {
        let manager = manager.clone();
        let success_count = success_count.clone();
        let failure_count = failure_count.clone();

        let handle = tokio::spawn(async move {
            let mut attempts = 0;

            let result = manager
                .execute_with_retry(|| {
                    attempts += 1;
                    Box::pin(async move {
                        // Simulate 30% transient failure rate
                        if attempts == 1 && i % 3 == 0 {
                            Err(cortex_core::error::CortexError::database("Transient error"))
                        } else {
                            Ok(())
                        }
                    })
                })
                .await;

            if result.is_ok() {
                success_count.fetch_add(1, Ordering::Relaxed);
            } else {
                failure_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let successes = success_count.load(Ordering::Relaxed);
    let failures = failure_count.load(Ordering::Relaxed);

    println!("\nRetry Under Load:");
    println!("  Tasks: {}", num_tasks);
    println!("  Successes: {}", successes);
    println!("  Failures: {}", failures);
    println!("  Duration: {:?}", elapsed);

    let metrics = manager.metrics().snapshot();
    println!("  Retries: {}", metrics.retries);

    // All should succeed with retry
    assert_eq!(successes, num_tasks);
    assert!(metrics.retries > 0);
}

#[tokio::test]
async fn test_health_monitoring_under_load() {
    let config = load_test_config(15);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let duration = Duration::from_secs(3);
    let start = Instant::now();

    // Generate load while health monitoring is running
    let mut handles = Vec::new();

    for _ in 0..20 {
        let manager = manager.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                if let Ok(_conn) = manager.acquire().await {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let metrics = manager.metrics().snapshot();
    let health = manager.health_status();

    println!("\nHealth Monitoring Under Load:");
    println!("  Health checks passed: {}", metrics.health_checks_passed);
    println!("  Health checks failed: {}", metrics.health_checks_failed);
    println!("  Pool healthy: {}", health.healthy);
    println!("  Circuit breaker: {:?}", health.circuit_breaker_state);

    assert!(health.healthy);
    assert_eq!(health.circuit_breaker_state, CircuitBreakerState::Closed);
}

#[tokio::test]
async fn test_connection_lifetime_rotation() {
    let config = DatabaseConfig {
        pool_config: PoolConfig {
            max_connections: 5,
            max_lifetime: Some(Duration::from_secs(1)),
            ..PoolConfig::default()
        },
        ..load_test_config(5)
    };

    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Acquire initial connections and note their IDs
    let initial_ids: Vec<_> = (0..3)
        .map(|_| {
            let manager = manager.clone();
            async move {
                let conn = manager.acquire().await.unwrap();
                conn.id()
            }
        })
        .collect::<futures::future::JoinAll<_>>()
        .await;

    // Wait for max lifetime to expire
    sleep(Duration::from_secs(2)).await;

    // Acquire new connections
    let new_ids: Vec<_> = (0..3)
        .map(|_| {
            let manager = manager.clone();
            async move {
                let conn = manager.acquire().await.unwrap();
                conn.id()
            }
        })
        .collect::<futures::future::JoinAll<_>>()
        .await;

    println!("\nConnection Lifetime Rotation:");
    println!("  Initial IDs: {:?}", initial_ids);
    println!("  New IDs: {:?}", new_ids);

    let metrics = manager.metrics().snapshot();
    println!("  Total connections created: {}", metrics.connections_created);

    // Should see some new connections created due to lifetime expiration
    assert!(metrics.connections_created > 3);
}

#[tokio::test]
async fn test_mixed_workload() {
    let config = load_test_config(20);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    let duration = Duration::from_secs(5);
    let start = Instant::now();

    let read_count = Arc::new(AtomicU64::new(0));
    let write_count = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();

    // Read-heavy workers (70%)
    for _ in 0..14 {
        let manager = manager.clone();
        let read_count = read_count.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                if let Ok(_conn) = manager.acquire().await {
                    sleep(Duration::from_millis(5)).await; // Fast reads
                    read_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    // Write-heavy workers (30%)
    for _ in 0..6 {
        let manager = manager.clone();
        let write_count = write_count.clone();
        let end_time = start + duration;

        let handle = tokio::spawn(async move {
            while Instant::now() < end_time {
                if let Ok(_conn) = manager.acquire().await {
                    sleep(Duration::from_millis(20)).await; // Slower writes
                    write_count.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let reads = read_count.load(Ordering::Relaxed);
    let writes = write_count.load(Ordering::Relaxed);
    let total = reads + writes;

    println!("\nMixed Workload:");
    println!("  Duration: {:?}", elapsed);
    println!("  Reads: {} ({:.1}%)", reads, (reads as f64 / total as f64) * 100.0);
    println!("  Writes: {} ({:.1}%)", writes, (writes as f64 / total as f64) * 100.0);
    println!("  Total ops: {}", total);
    println!("  Throughput: {:.2} ops/sec", total as f64 / elapsed.as_secs_f64());

    assert!(reads > writes); // Read-heavy workload
}

#[tokio::test]
async fn test_graceful_shutdown_under_load() {
    let config = load_test_config(10);
    let manager = Arc::new(ConnectionManager::new(config).await.unwrap());

    // Start some background work
    let manager_clone = manager.clone();
    let work_handle = tokio::spawn(async move {
        for _ in 0..10 {
            if let Ok(_conn) = manager_clone.acquire().await {
                sleep(Duration::from_millis(100)).await;
            }
        }
    });

    // Let some work start
    sleep(Duration::from_millis(200)).await;

    // Initiate shutdown
    let shutdown_result = manager.shutdown().await;

    // Wait for background work to complete
    let _ = work_handle.await;

    println!("\nGraceful Shutdown Under Load:");
    println!("  Shutdown result: {:?}", shutdown_result);

    assert!(shutdown_result.is_ok());
}
