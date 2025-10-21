//! Edge case and stress tests for the cognitive memory system.

use chrono::Utc;
use cortex_core::id::CortexId;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};
use std::sync::Arc;

/// Create a test cognitive manager
async fn create_test_manager() -> CognitiveManager {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Local { endpoint: "mem://".to_string() },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex".to_string(),
        database: "test".to_string(),
    };
    let manager = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    );
    CognitiveManager::new(manager)
}

#[tokio::test]
async fn test_empty_working_memory() {
    let manager = create_test_manager().await;

    // Retrieve from empty memory
    assert_eq!(manager.working().retrieve("nonexistent"), None);

    // Check statistics
    let stats = manager.working().get_statistics();
    assert_eq!(stats.current_items, 0);
    assert_eq!(stats.cache_hit_rate, 0.0);
}

#[tokio::test]
async fn test_working_memory_byte_limit() {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Local { endpoint: "mem://".to_string() },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex".to_string(),
        database: "test".to_string(),
    };

    let manager = CognitiveManager::with_config(
        Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create manager"),
        ),
        1000,
        1024, // Only 1KB total
    );

    // Try to store items larger than capacity
    let large_item = vec![0u8; 512]; // 512 bytes

    // Should store first two items
    assert!(manager.working().store("item1".to_string(), large_item.clone(), Priority::Medium));
    assert!(manager.working().store("item2".to_string(), large_item.clone(), Priority::Medium));

    // Third item should trigger eviction
    assert!(manager.working().store("item3".to_string(), large_item.clone(), Priority::High));

    // High priority item should be in memory
    assert!(manager.working().retrieve("item3").is_some());

    // At least one of the earlier items should have been evicted
    let stats = manager.working().get_statistics();
    assert!(stats.total_evictions > 0);
}

#[tokio::test]
async fn test_working_memory_priority_preservation() {
    let manager = create_test_manager().await;

    // Store items with different priorities
    for i in 0..20 {
        let priority = match i % 4 {
            0 => Priority::Critical,
            1 => Priority::High,
            2 => Priority::Medium,
            _ => Priority::Low,
        };

        manager.working().store(
            format!("item_{}", i),
            vec![i as u8; 1024],
            priority,
        );
    }

    // Critical priority items should still be accessible
    assert!(manager.working().retrieve("item_0").is_some());
    assert!(manager.working().retrieve("item_4").is_some());
}

#[tokio::test]
async fn test_episode_with_minimal_data() {
    let manager = create_test_manager().await;

    let minimal_episode = EpisodicMemory::new(
        "Minimal task".to_string(),
        "agent".to_string(),
        CortexId::new(),
        EpisodeType::Task,
    );

    let id = manager
        .remember_episode(&minimal_episode)
        .await
        .expect("Failed to store minimal episode");

    let retrieved = manager
        .episodic()
        .get_episode(id)
        .await
        .expect("Failed to retrieve")
        .expect("Episode not found");

    assert_eq!(retrieved.task_description, "Minimal task");
    assert_eq!(retrieved.entities_created.len(), 0);
    assert_eq!(retrieved.tools_used.len(), 0);
}

#[tokio::test]
async fn test_episode_with_maximum_data() {
    let manager = create_test_manager().await;

    let mut max_episode = EpisodicMemory::new(
        "Maximum data task".to_string(),
        "agent-001".to_string(),
        CortexId::new(),
        EpisodeType::Feature,
    );

    // Fill with maximum data
    max_episode.entities_created = (0..100).map(|i| format!("file_{}.rs", i)).collect();
    max_episode.entities_modified = (0..100).map(|i| format!("mod_{}.rs", i)).collect();
    max_episode.queries_made = (0..50).map(|i| format!("query_{}", i)).collect();
    max_episode.tools_used = (0..20)
        .map(|i| ToolUsage {
            tool_name: format!("tool_{}", i),
            usage_count: i,
            total_duration_ms: (i * 100) as u64,
            parameters: std::collections::HashMap::new(),
        })
        .collect();
    max_episode.lessons_learned = (0..10).map(|i| format!("Lesson {}", i)).collect();

    let id = manager
        .remember_episode(&max_episode)
        .await
        .expect("Failed to store maximum episode");

    let retrieved = manager
        .episodic()
        .get_episode(id)
        .await
        .expect("Failed to retrieve")
        .expect("Episode not found");

    assert_eq!(retrieved.entities_created.len(), 100);
    assert_eq!(retrieved.tools_used.len(), 20);
}

#[tokio::test]
async fn test_semantic_unit_with_high_complexity() {
    let manager = create_test_manager().await;

    let complex_unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "ultra_complex_function".to_string(),
        qualified_name: "module::ultra_complex_function".to_string(),
        display_name: "ultra_complex_function".to_string(),
        file_path: "src/complex.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 1000,
        end_column: 1,
        signature: "fn ultra_complex_function() -> Result<()>".to_string(),
        body: "// Very long body".to_string(),
        docstring: None,
        visibility: "public".to_string(),
        modifiers: vec!["async".to_string()],
        parameters: (0..20).map(|i| format!("param_{}", i)).collect(),
        return_type: Some("Result<()>".to_string()),
        summary: "Ultra complex".to_string(),
        purpose: "Testing complexity limits".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 100,
            cognitive: 150,
            nesting: 10,
            lines: 1000,
        },
        test_coverage: Some(0.1),
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let id = manager
        .remember_unit(&complex_unit)
        .await
        .expect("Failed to store complex unit");

    let retrieved = manager
        .semantic()
        .get_semantic_unit(id)
        .await
        .expect("Failed to retrieve")
        .expect("Unit not found");

    assert_eq!(retrieved.complexity.cyclomatic, 100);
    assert_eq!(retrieved.parameters.len(), 20);
}

#[tokio::test]
async fn test_circular_dependencies() {
    let manager = create_test_manager().await;

    let unit_a = CortexId::new();
    let unit_b = CortexId::new();
    let unit_c = CortexId::new();

    // Create circular dependency: A -> B -> C -> A
    manager
        .associate(unit_a, unit_b, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    manager
        .associate(unit_b, unit_c, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    manager
        .associate(unit_c, unit_a, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    // Should be able to retrieve all dependencies
    let deps_a = manager
        .semantic()
        .get_dependencies(unit_a)
        .await
        .expect("Failed to get dependencies");

    let deps_b = manager
        .semantic()
        .get_dependencies(unit_b)
        .await
        .expect("Failed to get dependencies");

    let deps_c = manager
        .semantic()
        .get_dependencies(unit_c)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps_a.len(), 1);
    assert_eq!(deps_b.len(), 1);
    assert_eq!(deps_c.len(), 1);
}

#[tokio::test]
async fn test_pattern_with_zero_applications() {
    let manager = create_test_manager().await;

    let pattern = LearnedPattern::new(
        PatternType::Code,
        "Unused pattern".to_string(),
        "Never applied".to_string(),
        "Test context".to_string(),
    );

    let id = manager
        .remember_pattern(&pattern)
        .await
        .expect("Failed to store pattern");

    let retrieved = manager
        .procedural()
        .get_pattern(id)
        .await
        .expect("Failed to retrieve")
        .expect("Pattern not found");

    assert_eq!(retrieved.times_applied, 0);
    assert_eq!(retrieved.success_rate, 0.0);
}

#[tokio::test]
async fn test_pattern_with_all_failures() {
    let manager = create_test_manager().await;

    let mut pattern = LearnedPattern::new(
        PatternType::Optimization,
        "Failed pattern".to_string(),
        "Always fails".to_string(),
        "Test".to_string(),
    );

    // Simulate multiple failures
    for _ in 0..5 {
        pattern.record_failure();
    }

    let id = manager
        .remember_pattern(&pattern)
        .await
        .expect("Failed to store pattern");

    let retrieved = manager
        .procedural()
        .get_pattern(id)
        .await
        .expect("Failed to retrieve")
        .expect("Pattern not found");

    assert_eq!(retrieved.times_applied, 5);
    assert_eq!(retrieved.success_rate, 0.0);
}

#[tokio::test]
async fn test_consolidation_with_no_data() {
    let manager = create_test_manager().await;

    // Consolidate empty memory
    let report = manager
        .consolidate()
        .await
        .expect("Failed to consolidate");

    assert_eq!(report.patterns_extracted, 0);
    assert_eq!(report.memories_decayed, 0);
}

#[tokio::test]
async fn test_forget_all_episodes() {
    let manager = create_test_manager().await;

    // Store some episodes
    for i in 0..10 {
        let episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Forget all by setting very high threshold
    let forgotten = manager
        .forget(1.0)
        .await
        .expect("Failed to forget");

    assert!(forgotten > 0);

    // Get statistics to verify
    let stats = manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    // Most or all should be forgotten
    assert!(stats.episodic.total_episodes < 10);
}

#[tokio::test]
async fn test_concurrent_working_memory_access() {
    let manager = Arc::new(create_test_manager().await);

    // Spawn multiple tasks accessing working memory concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let key = format!("key_{}_{}", i, j);
                mgr.working().store(
                    key.clone(),
                    vec![i as u8, j as u8],
                    Priority::Medium,
                );

                mgr.working().retrieve(&key);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Memory system should still be consistent
    let stats = manager.working().get_statistics();
    assert!(stats.current_items > 0);
}

#[tokio::test]
async fn test_cross_memory_query() {
    let manager = Arc::new(create_test_manager().await);
    let query_engine = CrossMemoryQuery::new(manager.clone());

    // Create some data
    let episode = EpisodicMemory::new(
        "Test task".to_string(),
        "agent".to_string(),
        CortexId::new(),
        EpisodeType::Task,
    );
    manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Execute complex query
    let filters = cortex_memory::query::QueryFilters {
        episode_outcome: Some(EpisodeOutcome::Success),
        limit: Some(10),
        ..Default::default()
    };

    let _results = query_engine
        .complex_query(filters)
        .await
        .expect("Failed to execute query");

    // Should complete without errors (results is valid)
}

#[tokio::test]
async fn test_statistics_accuracy() {
    let manager = create_test_manager().await;

    // Add known quantities
    for i in 0..5 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );
        episode.outcome = EpisodeOutcome::Success;
        episode.duration_seconds = 100;
        episode.tokens_used.total = 1000;

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store");
    }

    let stats = manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 5);
    assert_eq!(stats.episodic.successful_episodes, 5);
    assert!(stats.episodic.average_duration_seconds > 0.0);
}

#[tokio::test]
async fn test_large_batch_consolidation() {
    let manager = create_test_manager().await;

    // Store many episodes
    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store");
    }

    // Perform incremental consolidation
    let report = manager
        .consolidate_incremental(50)
        .await
        .expect("Failed to consolidate");

    assert_eq!(report.episodes_processed, 50);
    assert!(report.duration_ms > 0);
}
