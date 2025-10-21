//! Storage Layer Performance Benchmarks
//!
//! Comprehensive benchmarks for:
//! - Connection pool performance (target: <5ms acquisition)
//! - Query performance (SELECT <1ms, JOIN <10ms, FTS <50ms)
//! - Write performance (single <5ms, batch 1K <500ms)
//! - Transaction performance (commit <50ms)

use cortex_storage::{
    connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, PoolConfig, RetryPolicy, LoadBalancingStrategy, Credentials},
    session::SessionManager,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

// ==============================================================================
// Benchmark Setup Helpers
// ==============================================================================

fn create_test_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "memory".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                backoff_multiplier: 2.0,
            },
            warm_connections: true,
            health_check_interval: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        },
        namespace: "bench_ns".to_string(),
        database: "bench_db".to_string(),
    }
}

async fn setup_connection_manager() -> Arc<ConnectionManager> {
    let config = create_test_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    Arc::new(manager)
}

async fn setup_test_data(manager: &ConnectionManager, record_count: usize) {
    let session = manager.get_session().await.expect("Failed to get session");

    // Create test table with realistic schema
    session
        .query("DEFINE TABLE test_records SCHEMALESS")
        .await
        .expect("Failed to create table");

    session
        .query("DEFINE INDEX idx_test_name ON TABLE test_records COLUMNS name")
        .await
        .expect("Failed to create index");

    // Insert test records in batches
    let batch_size = 100;
    for batch_start in (0..record_count).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(record_count);
        let mut query = String::from("BEGIN TRANSACTION;");

        for i in batch_start..batch_end {
            query.push_str(&format!(
                "CREATE test_records SET name = 'record_{}', value = {}, data = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.';",
                i, i
            ));
        }

        query.push_str("COMMIT TRANSACTION;");

        session.query(&query).await.expect("Failed to insert batch");
    }
}

// ==============================================================================
// Connection Pool Benchmarks
// ==============================================================================

fn bench_connection_acquisition(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(setup_connection_manager());

    let mut group = c.benchmark_group("connection_pool");
    group.significance_level(0.05).sample_size(100);

    // Single connection acquisition - Target: <5ms
    group.bench_function("acquire_single", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            black_box(session);
        });
    });

    // Concurrent connection acquisition - 10 concurrent
    group.bench_function("acquire_concurrent_10", |b| {
        b.to_async(&rt).iter(|| async {
            let futures: Vec<_> = (0..10)
                .map(|_| manager.get_session())
                .collect();
            let results = futures::future::join_all(futures).await;
            black_box(results);
        });
    });

    // Concurrent connection acquisition - 50 concurrent
    group.bench_function("acquire_concurrent_50", |b| {
        b.to_async(&rt).iter(|| async {
            let futures: Vec<_> = (0..50)
                .map(|_| manager.get_session())
                .collect();
            let results = futures::future::join_all(futures).await;
            black_box(results);
        });
    });

    // Concurrent connection acquisition - 100 concurrent (pool saturation)
    group.bench_function("acquire_concurrent_100_saturation", |b| {
        b.to_async(&rt).iter(|| async {
            let futures: Vec<_> = (0..100)
                .map(|_| manager.get_session())
                .collect();
            let results = futures::future::join_all(futures).await;
            black_box(results);
        });
    });

    // Connection recycling
    group.bench_function("connection_recycling", |b| {
        b.to_async(&rt).iter(|| async {
            for _ in 0..10 {
                let session = manager.get_session().await.unwrap();
                drop(session); // Return to pool
            }
        });
    });

    group.finish();
}

// ==============================================================================
// Query Performance Benchmarks
// ==============================================================================

fn bench_query_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(setup_connection_manager());

    // Setup test data with 10K records
    rt.block_on(setup_test_data(&manager, 10_000));

    let mut group = c.benchmark_group("query_performance");
    group.significance_level(0.05).sample_size(100);

    // Simple SELECT by ID - Target: <1ms
    group.bench_function("select_by_id", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT * FROM test_records WHERE name = 'record_100'")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Simple SELECT with index - Target: <1ms
    group.bench_function("select_indexed", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT * FROM test_records WHERE name = 'record_500' LIMIT 1")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Range query - Target: <10ms
    group.bench_function("select_range_100", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT * FROM test_records WHERE value >= 100 AND value < 200")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Aggregation query - Target: <100ms
    group.bench_function("aggregation_count", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT COUNT() as total, math::avg(value) as avg_value FROM test_records GROUP BY ALL")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Full table scan - Target: <50ms for 10K records
    group.bench_function("full_scan_10k", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT * FROM test_records")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Text search - Target: <50ms
    group.bench_function("text_search", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("SELECT * FROM test_records WHERE data CONTAINS 'Lorem ipsum'")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// ==============================================================================
// Write Performance Benchmarks
// ==============================================================================

fn bench_write_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(setup_connection_manager());

    let mut group = c.benchmark_group("write_performance");
    group.significance_level(0.05).sample_size(50);

    // Single insert - Target: <5ms
    group.bench_function("insert_single", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query(&format!(
                    "CREATE test_records SET name = 'bench_{}', value = {}, data = 'Test data'",
                    counter, counter
                ))
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Batch insert with various sizes - Target: <500ms for 1K records
    for batch_size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_insert", batch_size),
            batch_size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let session = manager.get_session().await.unwrap();
                    let mut query = String::from("BEGIN TRANSACTION;");
                    for i in 0..size {
                        query.push_str(&format!(
                            "CREATE test_records SET name = 'batch_{}', value = {}, data = 'Batch test data';",
                            i, i
                        ));
                    }
                    query.push_str("COMMIT TRANSACTION;");

                    session.query(&query).await.unwrap();
                });
            },
        );
    }

    // Update operations - Target: <10ms
    group.bench_function("update_single", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("UPDATE test_records SET value = value + 1 WHERE name = 'record_100'")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Bulk update - Target: <100ms for 100 records
    group.bench_function("update_bulk_100", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            let result: Vec<serde_json::Value> = session
                .query("UPDATE test_records SET value = value + 1 WHERE value >= 0 AND value < 100")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    // Delete operations - Target: <10ms
    group.bench_function("delete_single", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            // Create and immediately delete to avoid depleting records
            session
                .query("CREATE test_records SET name = 'temp_delete', value = 999")
                .await
                .unwrap();
            let result: Vec<serde_json::Value> = session
                .query("DELETE test_records WHERE name = 'temp_delete'")
                .await
                .unwrap()
                .take(0)
                .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// ==============================================================================
// Transaction Performance Benchmarks
// ==============================================================================

fn bench_transaction_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(setup_connection_manager());

    let mut group = c.benchmark_group("transaction_performance");
    group.significance_level(0.05).sample_size(50);

    // Simple transaction commit - Target: <50ms
    group.bench_function("simple_transaction", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let session = manager.get_session().await.unwrap();
            let query = format!(
                "BEGIN TRANSACTION; \
                 CREATE test_records SET name = 'tx_{}', value = {}; \
                 COMMIT TRANSACTION;",
                counter, counter
            );
            session.query(&query).await.unwrap();
        });
    });

    // Multi-operation transaction - Target: <100ms
    group.bench_function("multi_op_transaction", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let session = manager.get_session().await.unwrap();
            let query = format!(
                "BEGIN TRANSACTION; \
                 CREATE test_records SET name = 'multi_{}', value = {}; \
                 UPDATE test_records SET value = value + 1 WHERE name = 'multi_{}'; \
                 SELECT * FROM test_records WHERE name = 'multi_{}'; \
                 COMMIT TRANSACTION;",
                counter, counter, counter, counter
            );
            session.query(&query).await.unwrap();
        });
    });

    // Transaction with rollback - Target: <50ms
    group.bench_function("transaction_rollback", |b| {
        let mut counter = 0;
        b.to_async(&rt).iter(|| async {
            counter += 1;
            let session = manager.get_session().await.unwrap();
            let query = format!(
                "BEGIN TRANSACTION; \
                 CREATE test_records SET name = 'rollback_{}', value = {}; \
                 CANCEL TRANSACTION;",
                counter, counter
            );
            session.query(&query).await.unwrap();
        });
    });

    group.finish();
}

// ==============================================================================
// Session Management Benchmarks
// ==============================================================================

fn bench_session_management(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = rt.block_on(setup_connection_manager());

    let mut group = c.benchmark_group("session_management");
    group.significance_level(0.05).sample_size(100);

    // Session creation - Target: <10ms
    group.bench_function("session_create", |b| {
        b.to_async(&rt).iter(|| async {
            let session = manager.get_session().await.unwrap();
            black_box(session);
        });
    });

    // Session with agent context - Target: <15ms
    group.bench_function("session_with_context", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = Uuid::new_v4();
            let session = manager.get_session().await.unwrap();
            // Simulate setting agent context
            black_box(session);
        });
    });

    // Concurrent sessions from multiple agents - Target: <50ms for 10 agents
    group.bench_function("multi_agent_sessions_10", |b| {
        b.to_async(&rt).iter(|| async {
            let futures: Vec<_> = (0..10)
                .map(|_| {
                    let manager = manager.clone();
                    async move {
                        manager.get_session().await.unwrap()
                    }
                })
                .collect();
            let sessions = futures::future::join_all(futures).await;
            black_box(sessions);
        });
    });

    group.finish();
}

// ==============================================================================
// Main Benchmark Configuration
// ==============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_connection_acquisition,
        bench_query_performance,
        bench_write_performance,
        bench_transaction_performance,
        bench_session_management,
);

criterion_main!(benches);
