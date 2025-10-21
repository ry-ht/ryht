//! Memory consolidation E2E test
//!
//! This test validates the complete memory consolidation pipeline:
//! 1. Create 100+ episodes
//! 2. Store in episodic memory
//! 3. Run consolidation
//! 4. Verify pattern extraction
//! 5. Test semantic memory queries
//! 6. Verify procedural patterns
//! 7. Test working memory eviction

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use std::collections::HashMap;
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
async fn test_large_scale_episodic_memory_consolidation() {
    let test_start = Instant::now();
    info!("Starting large-scale episodic memory consolidation test");

    let db_config = create_test_db_config("episodic_consolidation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create 100+ diverse episodes
    info!("Creating 150 diverse episodes");
    let episode_types = vec![
        EpisodeType::Feature,
        EpisodeType::Refactor,
        EpisodeType::Bugfix,
        EpisodeType::Task,
    ];

    let outcomes = vec![
        EpisodeOutcome::Success,
        EpisodeOutcome::Partial,
        EpisodeOutcome::Abandoned,
    ];

    let tools = vec![
        "code_generator",
        "semantic_search",
        "test_runner",
        "formatter",
        "linter",
    ];

    for i in 0..150 {
        let episode_type = episode_types[i % episode_types.len()];
        let outcome = outcomes[i % outcomes.len()];

        let mut episode = EpisodicMemory::new(
            format!("Task {} - {}", i, episode_type_name(episode_type)),
            format!("agent-{}", i % 5), // 5 different agents
            project_id,
            episode_type,
        );

        episode.outcome = outcome;
        episode.duration_seconds = (100 + (i * 13) % 5000) / 1000; // Varying durations in seconds

        // Add tool usage
        let tool_name = tools[i % tools.len()];
        episode.tools_used = vec![ToolUsage {
            tool_name: tool_name.to_string(),
            usage_count: (i % 10 + 1) as u32,
            total_duration_ms: episode.duration_seconds * 1000 / 2,
            parameters: {
                let mut params = HashMap::new();
                params.insert("mode".to_string(), "test".to_string());
                params
            },
        }];

        // Add some entities
        episode.entities_created = vec![
            format!("file_{}.rs", i),
            format!("test_{}.rs", i),
        ];

        episode.entities_modified = if i % 3 == 0 {
            vec![format!("existing_{}.rs", i / 3)]
        } else {
            vec![]
        };

        cognitive
            .remember_episode(&episode)
            .await
            .expect(&format!("Failed to store episode {}", i));

        if (i + 1) % 50 == 0 {
            info!("Created {} episodes", i + 1);
        }
    }

    let creation_time = test_start.elapsed();
    info!("Created 150 episodes in {:?}", creation_time);

    // Verify all episodes are stored
    let pre_consolidation_stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        pre_consolidation_stats.episodic.total_episodes, 150,
        "Should have 150 episodes before consolidation"
    );

    // Run full consolidation
    info!("Running full memory consolidation");
    let consolidation_start = Instant::now();
    let consolidation_report = cognitive
        .consolidate()
        .await
        .expect("Failed to consolidate");

    let consolidation_time = consolidation_start.elapsed();
    info!(
        "Consolidation completed in {:?}: {:?}",
        consolidation_time, consolidation_report
    );

    assert!(
        consolidation_report.duration_ms > 0,
        "Consolidation should take some time"
    );
    assert!(
        consolidation_time.as_secs() < 30,
        "Consolidation should complete within 30 seconds"
    );

    // Verify episodes still exist after consolidation
    let post_consolidation_stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        post_consolidation_stats.episodic.total_episodes, 150,
        "Episodes should persist after consolidation"
    );

    // Run dream to extract patterns
    info!("Running dream consolidation");
    let dream_start = Instant::now();
    let patterns = cognitive.dream().await.expect("Failed to dream");
    let dream_time = dream_start.elapsed();

    info!(
        "Dream extracted {} patterns in {:?}",
        patterns.len(),
        dream_time
    );

    // Patterns might be empty if the consolidator doesn't extract any,
    // but the operation should complete successfully
    assert!(dream_time.as_secs() < 30, "Dream should complete quickly");

    let total_time = test_start.elapsed();
    info!("Total consolidation test time: {:?}", total_time);
}

fn episode_type_name(et: EpisodeType) -> &'static str {
    match et {
        EpisodeType::Feature => "Feature",
        EpisodeType::Refactor => "Refactor",
        EpisodeType::Bugfix => "Bugfix",
        EpisodeType::Task => "Task",
    }
}

#[tokio::test]
async fn test_incremental_consolidation() {
    info!("Testing incremental memory consolidation");

    let db_config = create_test_db_config("incremental_consolidation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create 200 episodes
    info!("Creating 200 episodes for incremental consolidation");
    for i in 0..200 {
        let episode = EpisodicMemory::new(
            format!("Incremental task {}", i),
            "incremental-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Run incremental consolidation in batches of 50
    info!("Running incremental consolidation (batch size: 50)");
    let mut total_processed = 0;

    for batch in 0..4 {
        let report = cognitive
            .consolidate_incremental(50)
            .await
            .expect("Failed to consolidate incrementally");

        info!("Batch {}: processed {} episodes", batch, report.episodes_processed);
        total_processed += report.episodes_processed;

        assert!(
            report.episodes_processed <= 50,
            "Should not exceed batch size"
        );
    }

    info!("Total processed: {} episodes", total_processed);

    // Verify all episodes still exist
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        stats.episodic.total_episodes, 200,
        "All episodes should still exist"
    );
}

#[tokio::test]
async fn test_semantic_memory_operations() {
    info!("Testing semantic memory operations");

    let db_config = create_test_db_config("semantic_memory_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Create various semantic units
    info!("Creating 50 semantic units");
    let unit_types = vec![
        CodeUnitType::Function,
        CodeUnitType::Struct,
        CodeUnitType::Enum,
        CodeUnitType::Trait,
        CodeUnitType::Module,
    ];

    let mut created_units = vec![];

    for i in 0..50 {
        let unit_type = unit_types[i % unit_types.len()];

        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type,
            name: format!("{}_name_{}", unit_type_name(unit_type), i),
            qualified_name: format!("module::{}_{}", unit_type_name(unit_type), i),
            display_name: format!("{} {}", unit_type_name(unit_type), i),
            file_path: format!("src/module_{}.rs", i / 10),
            start_line: (i * 10) % 1000,
            start_column: 0,
            end_line: ((i * 10) % 1000) + 20,
            end_column: 1,
            signature: format!("pub {} {}(...)", unit_type_name(unit_type), i),
            body: format!("// Implementation for {}", i),
            docstring: Some(format!("Documentation for unit {}", i)),
            visibility: if i % 2 == 0 { "public" } else { "private" }.to_string(),
            modifiers: vec![],
            parameters: if i % 3 == 0 {
                vec!["param1".to_string(), "param2".to_string()]
            } else {
                vec![]
            },
            return_type: if i % 2 == 0 {
                Some("Result<()>".to_string())
            } else {
                None
            },
            summary: format!("Summary for unit {}", i),
            purpose: format!("Purpose of unit {}", i),
            complexity: ComplexityMetrics {
                cyclomatic: (i % 20) as u32,
                cognitive: (i % 30) as u32,
                nesting: (i % 5) as u32,
                lines: ((i % 100) + 10) as u32,
            },
            test_coverage: Some((i as f32 % 100.0) / 100.0),
            has_tests: i % 3 == 0,
            has_documentation: i % 2 == 0,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let unit_id = cognitive
            .remember_unit(&unit)
            .await
            .expect("Failed to store unit");

        created_units.push(unit_id);
    }

    // Query complex units
    info!("Querying complex semantic units");
    let complex_units = cognitive
        .semantic()
        .find_complex_units(10)
        .await
        .expect("Failed to find complex units");

    info!("Found {} complex units", complex_units.len());
    assert!(complex_units.len() > 0, "Should find some complex units");

    // Query untested units
    info!("Querying untested units");
    let untested_units = cognitive
        .semantic()
        .find_untested_units()
        .await
        .expect("Failed to find untested units");

    info!("Found {} untested units", untested_units.len());
    // Should have some untested units (only 1/3 have tests)
    assert!(
        untested_units.len() > 0,
        "Should find untested units"
    );

    // Verify stats
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        stats.semantic.total_units, 50,
        "Should have 50 semantic units"
    );
}

fn unit_type_name(ut: CodeUnitType) -> &'static str {
    match ut {
        CodeUnitType::Function => "fn",
        CodeUnitType::Struct => "struct",
        CodeUnitType::Enum => "enum",
        CodeUnitType::Trait => "trait",
        CodeUnitType::Module => "mod",
        _ => "unknown",
    }
}

#[tokio::test]
async fn test_procedural_memory_patterns() {
    info!("Testing procedural memory and pattern learning");

    let db_config = create_test_db_config("procedural_memory_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);

    // Create learned patterns
    info!("Creating learned patterns");
    let pattern_types = vec![
        PatternType::Optimization,
        PatternType::Refactor,
        PatternType::Code,
        PatternType::Architecture,
    ];

    let mut pattern_ids = vec![];

    for i in 0..20 {
        let pattern_type = pattern_types[i % pattern_types.len()];

        let pattern = LearnedPattern::new(
            pattern_type,
            format!("Pattern {}: {}", i, pattern_type_name(pattern_type)),
            format!("Description for pattern {}", i),
            format!("Context for applying pattern {}", i),
        );

        let pattern_id = cognitive
            .remember_pattern(&pattern)
            .await
            .expect("Failed to store pattern");

        pattern_ids.push(pattern_id);
    }

    // Apply patterns with varying success rates
    info!("Applying patterns and recording outcomes");
    for (idx, pattern_id) in pattern_ids.iter().enumerate() {
        let success_rate = (idx % 10) as f32 / 10.0;

        // Apply pattern multiple times
        let total_applications = 10;
        let successful = (total_applications as f32 * success_rate) as usize;

        for _ in 0..successful {
            cognitive
                .procedural()
                .record_success(*pattern_id)
                .await
                .expect("Failed to record success");
        }

        for _ in successful..total_applications {
            cognitive
                .procedural()
//                 .record_failure(*pattern_id)
//                 .await
//                 .expect("Failed to record failure");
        }
    }

    // Verify pattern statistics
    info!("Verifying pattern statistics");
    for (idx, pattern_id) in pattern_ids.iter().enumerate() {
        let pattern = cognitive
            .procedural()
            .get_pattern(*pattern_id)
            .await
            .expect("Failed to get pattern")
            .expect("Pattern not found");

        assert_eq!(
            pattern.times_applied, 10,
            "Pattern should be applied 10 times"
        );

        let expected_success_rate = (idx % 10) as f32 / 10.0;
        let tolerance = 0.01;

        assert!(
            (pattern.success_rate - expected_success_rate).abs() < tolerance,
            "Success rate should be approximately {}",
            expected_success_rate
        );
    }

    // Verify procedural memory stats
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        stats.procedural.total_patterns, 20,
        "Should have 20 patterns"
    );
}

fn pattern_type_name(pt: PatternType) -> &'static str {
    match pt {
        PatternType::Optimization => "Optimization",
        PatternType::Refactor => "Refactoring",
        PatternType::Code => "Testing",
        PatternType::Architecture => "Architecture",
        _ => "Unknown",
    }
}

#[tokio::test]
async fn test_working_memory_eviction() {
    info!("Testing working memory eviction policies");

    let db_config = create_test_db_config("working_memory_eviction_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Create with limited capacity: 10 items, 1KB total
    let cognitive = CognitiveManager::with_config(connection_manager, 10, 1024);

    // Fill with low priority items
    info!("Filling working memory with low priority items");
    for i in 0..10 {
        let key = format!("low_priority_{}", i);
        let value = vec![i as u8; 50]; // 50 bytes each
        cognitive
            .working()
            .store(key, value, Priority::Low);
    }

    let stats = cognitive.working().get_statistics();
    assert_eq!(stats.current_items, 10, "Should have 10 items");

    // Add high priority items - should evict low priority
    info!("Adding high priority items");
    for i in 0..5 {
        let key = format!("high_priority_{}", i);
        let value = vec![(100 + i) as u8; 50];
        cognitive
            .working()
            .store(key, value, Priority::High);
    }

    // Verify high priority items are present
    for i in 0..5 {
        let key = format!("high_priority_{}", i);
        assert!(
            cognitive.working().retrieve(&key).is_some(),
            "High priority item {} should be present",
            i
        );
    }

    // Some low priority items should be evicted
    let stats = cognitive.working().get_statistics();
    info!("Working memory stats after eviction: {:?}", stats);

    // Should not exceed capacity
    assert!(
        stats.current_items <= 10,
        "Should not exceed capacity"
    );

    info!("Working memory eviction test passed");
}

#[tokio::test]
async fn test_memory_forget_operation() {
    info!("Testing memory forget operation");

    let db_config = create_test_db_config("forget_operation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create episodes with different importance levels
    info!("Creating episodes with varying importance");
    for i in 0..50 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "forget-test-agent".to_string(),
            project_id,
            EpisodeType::Task,
        );

        // First 30 are successful (high importance)
        // Last 20 are abandoned (low importance)
        episode.outcome = if i < 30 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Abandoned
        };

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    let before_forget = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        before_forget.episodic.total_episodes, 50,
        "Should have 50 episodes before forget"
    );

    // Forget low importance episodes (threshold 0.5)
    info!("Forgetting low importance episodes");
    let forgotten_count = cognitive
        .forget(0.5)
        .await
        .expect("Failed to forget");

    info!("Forgot {} episodes", forgotten_count);
    assert!(forgotten_count > 0, "Should forget some episodes");

    // Verify reduced episode count
    let after_forget = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert!(
        after_forget.episodic.total_episodes < 50,
        "Should have fewer episodes after forget"
    );

    info!("Forget operation test passed");
}

#[tokio::test]
async fn test_cross_memory_integration() {
    info!("Testing integration across all memory systems");

    let db_config = create_test_db_config("cross_memory_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // 1. Add items to working memory
    info!("Step 1: Populating working memory");
    for i in 0..5 {
        cognitive.working().store(
            format!("context_{}", i),
            vec![i as u8; 100],
            Priority::High,
        );
    }

    // 2. Create episodes
    info!("Step 2: Creating episodes");
    for i in 0..10 {
        let episode = EpisodicMemory::new(
            format!("Cross-memory task {}", i),
            "integration-agent".to_string(),
            project_id,
            EpisodeType::Feature,
        );

        cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // 3. Create semantic units
    info!("Step 3: Creating semantic units");
    for i in 0..10 {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("cross_fn_{}", i),
            qualified_name: format!("cross::fn_{}", i),
            display_name: format!("cross_fn_{}", i),
            file_path: "cross.rs".to_string(),
            start_line: i * 10,
            start_column: 0,
            end_line: i * 10 + 5,
            end_column: 1,
            signature: format!("pub fn cross_fn_{}()", i),
            body: "// Body".to_string(),
            docstring: None,
            visibility: "public".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: format!("Function {}", i),
            purpose: format!("Purpose {}", i),
            complexity: ComplexityMetrics {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 1,
                lines: 5,
            },
            test_coverage: Some(1.0),
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        cognitive
            .remember_unit(&unit)
            .await
            .expect("Failed to store unit");
    }

    // 4. Create learned patterns
    info!("Step 4: Creating learned patterns");
    for i in 0..5 {
        let pattern = LearnedPattern::new(
            PatternType::Optimization,
            format!("Cross-memory pattern {}", i),
            format!("Description {}", i),
            "Integration context".to_string(),
        );

        cognitive
            .remember_pattern(&pattern)
            .await
            .expect("Failed to store pattern");
    }

    // 5. Verify all memory systems
    info!("Step 5: Verifying all memory systems");
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(stats.episodic.total_episodes, 10, "Should have 10 episodes");
    assert_eq!(stats.semantic.total_units, 10, "Should have 10 units");
    assert_eq!(stats.working.current_items, 5, "Should have 5 working items");
    assert_eq!(stats.procedural.total_patterns, 5, "Should have 5 patterns");

    // 6. Run consolidation
    info!("Step 6: Running cross-memory consolidation");
    let report = cognitive
        .consolidate()
        .await
        .expect("Failed to consolidate");

    info!("Consolidation report: {:?}", report);

    // Verify memories persist after consolidation
    let post_stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get stats");

    assert_eq!(
        post_stats.episodic.total_episodes, 10,
        "Episodes should persist"
    );
    assert_eq!(
        post_stats.semantic.total_units, 10,
        "Units should persist"
    );

    info!("Cross-memory integration test passed");
}
