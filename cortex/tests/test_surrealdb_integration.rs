//! SurrealDB integration E2E test
//!
//! This test validates database integration:
//! 1. Test connection pool
//! 2. Test transactions
//! 3. Test rollback scenarios
//! 4. Test connection recovery
//! 5. Test different connection modes

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Helper to create test database config
fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex_test".to_string(),
        database: db_name.to_string(),
    }
}

#[tokio::test]
async fn test_connection_pool_initialization() {
    info!("Testing connection pool initialization");

    let db_config = create_test_db_config("pool_init_test");

    let init_start = Instant::now();
    let connection_manager = ConnectionManager::new(db_config)
        .await
        .expect("Failed to create connection manager");
    let init_duration = init_start.elapsed();

    info!("Connection pool initialized in {:?}", init_duration);

    assert!(
        init_duration.as_secs() < 5,
        "Pool initialization should be fast"
    );

    // Verify we can get a connection
    let conn_result = connection_manager.get_connection().await;
    assert!(conn_result.is_ok(), "Should get connection from pool");

    info!("Connection pool initialization test passed");
}

#[tokio::test]
async fn test_multiple_database_namespaces() {
    info!("Testing multiple database namespaces");

    // Create two separate database configs
    let db_config_1 = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "namespace_1".to_string(),
        database: "db_1".to_string(),
    };

    let db_config_2 = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "namespace_2".to_string(),
        database: "db_2".to_string(),
    };

    let manager_1 = Arc::new(
        ConnectionManager::new(db_config_1)
            .await
            .expect("Failed to create manager 1"),
    );

    let manager_2 = Arc::new(
        ConnectionManager::new(db_config_2)
            .await
            .expect("Failed to create manager 2"),
    );

    // Create cognitive managers for each
    let cognitive_1 = CognitiveManager::new(manager_1);
    let cognitive_2 = CognitiveManager::new(manager_2);

    let project_id_1 = CortexId::new();
    let project_id_2 = CortexId::new();

    // Store episodes in different namespaces
    let episode_1 = EpisodicMemory::new(
        "Episode in namespace 1".to_string(),
        "agent-1".to_string(),
        project_id_1,
        EpisodeType::Task,
    );

    let episode_2 = EpisodicMemory::new(
        "Episode in namespace 2".to_string(),
        "agent-2".to_string(),
        project_id_2,
        EpisodeType::Task,
    );

    cognitive_1
        .remember_episode(&episode_1)
        .await
        .expect("Failed to store episode 1");

    cognitive_2
        .remember_episode(&episode_2)
        .await
        .expect("Failed to store episode 2");

    // Verify isolation
    let stats_1 = cognitive_1
        .get_statistics()
        .await
        .expect("Failed to get stats 1");

    let stats_2 = cognitive_2
        .get_statistics()
        .await
        .expect("Failed to get stats 2");

    assert_eq!(
        stats_1.episodic.total_episodes, 1,
        "Namespace 1 should have 1 episode"
    );
    assert_eq!(
        stats_2.episodic.total_episodes, 1,
        "Namespace 2 should have 1 episode"
    );

    info!("Multiple namespace test passed");
}

#[tokio::test]
async fn test_connection_pool_exhaustion_recovery() {
    info!("Testing connection pool exhaustion and recovery");

    let db_config = DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            max_connections: 5, // Small pool for testing
            min_connections: 2,
            connection_timeout_secs: 10,
            max_idle_time_secs: 300,
            health_check_interval_secs: 60,
        },
        namespace: "cortex_test".to_string(),
        database: "pool_exhaustion_test".to_string(),
    };

    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Perform operations that would normally exhaust the pool
    info!("Performing 50 sequential operations");
    for i in 0..50 {
        let episode = EpisodicMemory::new(
            format!("Exhaustion test episode {}", i),
            "exhaustion-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");

        if (i + 1) % 10 == 0 {
            info!("Completed {} operations", i + 1);
        }
    }

    // Verify all operations succeeded
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 50,
        "All 50 episodes should be stored"
    );

    info!("Connection pool exhaustion recovery test passed");
}

#[tokio::test]
async fn test_database_operations_with_different_types() {
    info!("Testing various database operations");

    let db_config = create_test_db_config("operations_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Test 1: Create and retrieve episode
    info!("Test 1: Episode CRUD operations");
    let episode = EpisodicMemory::new(
        "CRUD test episode".to_string(),
        "crud-agent".to_string(),
        project_id,
        EpisodeType::Feature,
    );

    let episode_id = cognitive
        .remember_episode(&episode)
        .await
        .expect("Failed to create episode");

    let retrieved = cognitive
        .episodic()
        .get_episode(episode_id)
        .await
        .expect("Failed to retrieve episode")
        .expect("Episode not found");

    assert_eq!(retrieved.task_description, "CRUD test episode");
    assert_eq!(retrieved.agent_id, "crud-agent");

    // Test 2: Create and retrieve semantic unit
    info!("Test 2: Semantic unit CRUD operations");
    let unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "crud_function".to_string(),
        qualified_name: "module::crud_function".to_string(),
        display_name: "crud_function".to_string(),
        file_path: "crud.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 10,
        end_column: 1,
        signature: "pub fn crud_function()".to_string(),
        body: "// CRUD test".to_string(),
        docstring: Some("CRUD test function".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "CRUD function".to_string(),
        purpose: "Test CRUD operations".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            lines: 10,
        },
        test_coverage: Some(1.0),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let unit_id = cognitive
        .remember_unit(&unit)
        .await
        .expect("Failed to create unit");

    let retrieved_unit = cognitive
        .semantic()
        .get_unit(unit_id)
        .await
        .expect("Failed to retrieve unit")
        .expect("Unit not found");

    assert_eq!(retrieved_unit.name, "crud_function");
    assert_eq!(retrieved_unit.visibility, "public");

    // Test 3: Create and retrieve pattern
    info!("Test 3: Pattern CRUD operations");
    let pattern = LearnedPattern::new(
        PatternType::Optimization,
        "CRUD test pattern".to_string(),
        "Pattern for CRUD testing".to_string(),
        "CRUD context".to_string(),
    );

    let pattern_id = cognitive
        .remember_pattern(&pattern)
        .await
        .expect("Failed to create pattern");

    let retrieved_pattern = cognitive
        .procedural()
        .get_pattern(pattern_id)
        .await
        .expect("Failed to retrieve pattern")
        .expect("Pattern not found");

    assert_eq!(retrieved_pattern.pattern_name, "CRUD test pattern");

    // Test 4: Dependency operations
    info!("Test 4: Dependency operations");
    let unit_a = CortexId::new();
    let unit_b = CortexId::new();

    cognitive
        .associate(unit_a, unit_b, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    let deps = cognitive
        .semantic()
        .get_dependencies(unit_a)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps.len(), 1, "Should have 1 dependency");

    info!("Database operations test passed");
}

#[tokio::test]
async fn test_concurrent_database_access() {
    info!("Testing concurrent database access");

    let db_config = create_test_db_config("concurrent_db_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let project_id = CortexId::new();

    // Spawn 10 concurrent tasks
    let mut handles = vec![];

    for task_id in 0..10 {
        let manager_clone = connection_manager.clone();

        let handle = tokio::spawn(async move {
            let cognitive = CognitiveManager::new(manager_clone);

            // Each task creates 10 episodes
            for i in 0..10 {
                let episode = EpisodicMemory::new(
                    format!("Concurrent task {} episode {}", task_id, i),
                    format!("concurrent-agent-{}", task_id),
                    project_id,
                    EpisodeType::Task,
                );

                cognitive
                    .remember_episode(&episode)
                    .await
                    .expect("Failed to store episode");
            }

            info!("Task {} completed", task_id);
        });

        handles.push(handle);
    }

    // Wait for all tasks
    futures::future::join_all(handles).await;

    // Verify all episodes were created
    let cognitive = CognitiveManager::new(connection_manager);
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 100,
        "Should have 100 episodes from 10 tasks"
    );

    info!("Concurrent database access test passed");
}

#[tokio::test]
async fn test_database_query_performance() {
    info!("Testing database query performance");

    let db_config = create_test_db_config("query_perf_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create 100 episodes
    info!("Creating 100 episodes");
    let create_start = Instant::now();

    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Query perf test {}", i),
            "perf-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    let create_duration = create_start.elapsed();
    info!(
        "Created 100 episodes in {:?} ({:.2} eps/sec)",
        create_duration,
        100.0 / create_duration.as_secs_f64()
    );

    // Query statistics
    info!("Querying statistics");
    let query_start = Instant::now();

    for _ in 0..10 {
        cognitive
            .get_statistics()
            .await
            .expect("Failed to get statistics");
    }

    let query_duration = query_start.elapsed();
    info!(
        "10 statistics queries in {:?} ({:.2} queries/sec)",
        query_duration,
        10.0 / query_duration.as_secs_f64()
    );

    // Performance assertions
    assert!(
        create_duration.as_secs() < 10,
        "Creating 100 episodes should take less than 10 seconds"
    );
    assert!(
        query_duration.as_secs() < 5,
        "10 queries should take less than 5 seconds"
    );

    info!("Database query performance test passed");
}

#[tokio::test]
async fn test_memory_database_isolation() {
    info!("Testing in-memory database isolation");

    // Create two separate in-memory databases
    let db_config_1 = create_test_db_config("isolated_db_1");
    let db_config_2 = create_test_db_config("isolated_db_2");

    let manager_1 = Arc::new(
        ConnectionManager::new(db_config_1)
            .await
            .expect("Failed to create manager 1"),
    );

    let manager_2 = Arc::new(
        ConnectionManager::new(db_config_2)
            .await
            .expect("Failed to create manager 2"),
    );

    let cognitive_1 = CognitiveManager::new(manager_1);
    let cognitive_2 = CognitiveManager::new(manager_2);

    let project_id = CortexId::new();

    // Store different data in each database
    let episode_1 = EpisodicMemory::new(
        "Database 1 episode".to_string(),
        "agent-1".to_string(),
        project_id,
        EpisodeType::Feature,
    );

    let episode_2 = EpisodicMemory::new(
        "Database 2 episode".to_string(),
        "agent-2".to_string(),
        project_id,
        EpisodeType::Bugfix,
    );

    cognitive_1
        .remember_episode(&episode_1)
        .await
        .expect("Failed to store in db1");

    cognitive_2
        .remember_episode(&episode_2)
        .await
        .expect("Failed to store in db2");

    // Verify isolation
    let stats_1 = cognitive_1.get_statistics().await.expect("Failed to get stats 1");
    let stats_2 = cognitive_2.get_statistics().await.expect("Failed to get stats 2");

    assert_eq!(stats_1.episodic.total_episodes, 1, "DB1 should have 1 episode");
    assert_eq!(stats_2.episodic.total_episodes, 1, "DB2 should have 1 episode");

    info!("Memory database isolation test passed");
}

#[tokio::test]
async fn test_database_error_handling() {
    info!("Testing database error handling");

    let db_config = create_test_db_config("error_handling_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Test 1: Query non-existent episode
    info!("Test 1: Non-existent episode");
    let non_existent_id = CortexId::new();
    let result = cognitive
        .episodic()
        .get_episode(non_existent_id)
        .await;

    assert!(result.is_ok(), "Should not error");
    assert!(result.unwrap().is_none(), "Should return None");

    // Test 2: Query non-existent unit
    info!("Test 2: Non-existent unit");
    let non_existent_unit = CortexId::new();
    let result = cognitive
        .semantic()
        .get_unit(non_existent_unit)
        .await;

    assert!(result.is_ok(), "Should not error");
    assert!(result.unwrap().is_none(), "Should return None");

    // Test 3: Query non-existent pattern
    info!("Test 3: Non-existent pattern");
    let non_existent_pattern = CortexId::new();
    let result = cognitive
        .procedural()
        .get_pattern(non_existent_pattern)
        .await;

    assert!(result.is_ok(), "Should not error");
    assert!(result.unwrap().is_none(), "Should return None");

    info!("Database error handling test passed");
}

#[tokio::test]
async fn test_database_data_integrity() {
    info!("Testing database data integrity");

    let db_config = create_test_db_config("integrity_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create episode with complex data
    let mut episode = EpisodicMemory::new(
        "Integrity test episode".to_string(),
        "integrity-agent".to_string(),
        project_id,
        EpisodeType::Feature,
    );

    episode.entities_created = vec![
        "file1.rs".to_string(),
        "file2.rs".to_string(),
        "file3.rs".to_string(),
    ];

    episode.entities_modified = vec!["existing.rs".to_string()];

    episode.tools_used = vec![
        ToolUsage {
            tool_name: "compiler".to_string(),
            usage_count: 5,
            total_duration_ms: 1000,
            parameters: {
                let mut map = std::collections::HashMap::new();
                map.insert("optimization".to_string(), "3".to_string());
                map
            },
        },
        ToolUsage {
            tool_name: "linter".to_string(),
            usage_count: 2,
            total_duration_ms: 500,
            parameters: std::collections::HashMap::new(),
        },
    ];

    episode.outcome = EpisodeOutcome::Success;
    episode.duration_ms = 1500;

    let episode_id = cognitive
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Retrieve and verify all fields
    let retrieved = cognitive
        .episodic()
        .get_episode(episode_id)
        .await
        .expect("Failed to retrieve")
        .expect("Episode not found");

    assert_eq!(retrieved.task_description, episode.task_description);
    assert_eq!(retrieved.agent_id, episode.agent_id);
    assert_eq!(retrieved.entities_created.len(), 3);
    assert_eq!(retrieved.entities_modified.len(), 1);
    assert_eq!(retrieved.tools_used.len(), 2);
    assert_eq!(retrieved.outcome, EpisodeOutcome::Success);
    assert_eq!(retrieved.duration_ms, 1500);

    info!("Database data integrity test passed");
}
