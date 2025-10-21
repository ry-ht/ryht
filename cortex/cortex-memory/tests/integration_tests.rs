//! Comprehensive integration tests for the cognitive memory system.

use chrono::Utc;
use cortex_core::id::CortexId;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};
use std::sync::Arc;
use std::time::Duration;

/// Create a test cognitive manager
async fn create_test_manager() -> CognitiveManager {
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::Local { endpoint: "mem://".to_string() },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 2,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(30)),
            max_lifetime: Some(Duration::from_secs(60)),
            retry_policy: cortex_storage::connection_pool::RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: false,
            recycle_after_uses: Some(1000),
            shutdown_grace_period: Duration::from_secs(5),
        },
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
async fn test_episodic_memory_workflow() {
    let manager = create_test_manager().await;

    // Create an episode
    let mut episode = EpisodicMemory::new(
        "Implement user authentication".to_string(),
        "agent-001".to_string(),
        CortexId::new(),
        EpisodeType::Feature,
    );

    episode.entities_created = vec!["auth.rs".to_string(), "user.rs".to_string()];
    episode.entities_modified = vec!["main.rs".to_string()];
    episode.tools_used = vec![ToolUsage {
        tool_name: "code_generator".to_string(),
        usage_count: 3,
        total_duration_ms: 1500,
        parameters: std::collections::HashMap::new(),
    }];
    episode.outcome = EpisodeOutcome::Success;
    episode.duration_seconds = 300;
    episode.solution_summary = "Successfully implemented authentication".to_string();
    episode.lessons_learned = vec!["Use JWT tokens for stateless auth".to_string()];

    // Store the episode
    let episode_id = manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Retrieve the episode
    let retrieved = manager
        .episodic()
        .get_episode(episode_id)
        .await
        .expect("Failed to retrieve episode")
        .expect("Episode not found");

    assert_eq!(retrieved.task_description, "Implement user authentication");
    assert_eq!(retrieved.outcome, EpisodeOutcome::Success);
    assert_eq!(retrieved.entities_created.len(), 2);
}

#[tokio::test]
async fn test_semantic_memory_workflow() {
    let manager = create_test_manager().await;

    // Create a semantic unit
    let unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "authenticate_user".to_string(),
        qualified_name: "auth::authenticate_user".to_string(),
        display_name: "authenticate_user".to_string(),
        file_path: "src/auth.rs".to_string(),
        start_line: 10,
        start_column: 0,
        end_line: 30,
        end_column: 1,
        signature: "pub fn authenticate_user(username: &str, password: &str) -> Result<Token>".to_string(),
        body: "// Authentication logic".to_string(),
        docstring: Some("Authenticate a user with username and password".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec!["username".to_string(), "password".to_string()],
        return_type: Some("Result<Token>".to_string()),
        summary: "Authenticates user credentials".to_string(),
        purpose: "User authentication".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 5,
            cognitive: 7,
            nesting: 2,
            lines: 20,
        },
        test_coverage: Some(0.9),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Store the unit
    let unit_id = manager
        .remember_unit(&unit)
        .await
        .expect("Failed to store unit");

    // Retrieve the unit
    let retrieved = manager
        .semantic()
        .get_semantic_unit(unit_id)
        .await
        .expect("Failed to retrieve unit")
        .expect("Unit not found");

    assert_eq!(retrieved.name, "authenticate_user");
    assert_eq!(retrieved.unit_type, CodeUnitType::Function);
    assert!(retrieved.has_tests);
    assert!(retrieved.has_documentation);
}

#[tokio::test]
async fn test_dependency_tracking() {
    let manager = create_test_manager().await;

    // Create two related units
    let caller_id = CortexId::new();
    let callee_id = CortexId::new();

    // Create an association (dependency)
    manager
        .associate(caller_id, callee_id, DependencyType::Calls)
        .await
        .expect("Failed to create association");

    // Retrieve dependencies
    let deps = manager
        .semantic()
        .get_dependencies(caller_id)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].source_id, caller_id);
    assert_eq!(deps[0].target_id, callee_id);
    assert_eq!(deps[0].dependency_type, DependencyType::Calls);
}

#[tokio::test]
async fn test_working_memory_eviction() {
    let manager = create_test_manager().await;

    // Store items until we hit capacity
    for i in 0..15 {
        let key = format!("item_{}", i);
        let value = vec![i as u8; 1024]; // 1KB each
        let priority = if i < 5 {
            Priority::Low
        } else if i < 10 {
            Priority::Medium
        } else {
            Priority::High
        };

        manager.working().store(key, value, priority);
    }

    // High priority items should still be in memory
    assert!(manager.working().retrieve("item_14").is_some());
    assert!(manager.working().retrieve("item_13").is_some());

    // Check statistics
    let stats = manager.working().get_statistics();
    assert!(stats.current_items > 0);
    assert!(stats.total_evictions > 0 || stats.current_items < 15);
}

#[tokio::test]
async fn test_pattern_learning() {
    let manager = create_test_manager().await;

    // Create a learned pattern
    let pattern = LearnedPattern::new(
        PatternType::Refactor,
        "Extract method refactoring".to_string(),
        "Common pattern for extracting long methods into smaller functions".to_string(),
        "Code complexity reduction".to_string(),
    );

    // Store the pattern
    let pattern_id = manager
        .remember_pattern(&pattern)
        .await
        .expect("Failed to store pattern");

    // Retrieve the pattern
    let retrieved = manager
        .procedural()
        .get_pattern(pattern_id)
        .await
        .expect("Failed to retrieve pattern")
        .expect("Pattern not found");

    assert_eq!(retrieved.name, "Extract method refactoring");
    assert_eq!(retrieved.pattern_type, PatternType::Refactor);
}

#[tokio::test]
async fn test_pattern_success_tracking() {
    let manager = create_test_manager().await;

    let pattern = LearnedPattern::new(
        PatternType::Optimization,
        "Cache query results".to_string(),
        "Pattern for caching expensive queries".to_string(),
        "Performance optimization".to_string(),
    );

    let pattern_id = manager
        .remember_pattern(&pattern)
        .await
        .expect("Failed to store pattern");

    // Record successful applications
    for _ in 0..3 {
        manager
            .procedural()
            .record_success(pattern_id)
            .await
            .expect("Failed to record success");
    }

    // Retrieve updated pattern
    let updated = manager
        .procedural()
        .get_pattern(pattern_id)
        .await
        .expect("Failed to retrieve pattern")
        .expect("Pattern not found");

    assert_eq!(updated.times_applied, 3);
    assert_eq!(updated.success_rate, 1.0);
}

#[tokio::test]
async fn test_memory_consolidation() {
    let manager = create_test_manager().await;

    // Create several episodes
    for i in 0..5 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );
        episode.outcome = if i % 2 == 0 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Partial
        };

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Perform consolidation
    let report = manager
        .consolidate()
        .await
        .expect("Failed to consolidate memories");

    // consolidation should run without errors
    assert!(report.duration_ms > 0);
}

#[tokio::test]
async fn test_dream_pattern_extraction() {
    let manager = create_test_manager().await;

    // Create successful episodes with common tools
    for i in 0..5 {
        let mut episode = EpisodicMemory::new(
            format!("Feature {}", i),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Feature,
        );
        episode.outcome = EpisodeOutcome::Success;
        episode.tools_used = vec![ToolUsage {
            tool_name: "code_analyzer".to_string(),
            usage_count: 1,
            total_duration_ms: 500,
            parameters: std::collections::HashMap::new(),
        }];

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Extract patterns through dreaming
    let patterns = manager.dream().await.expect("Failed to dream");

    // Should have extracted at least one pattern
    assert!(patterns.len() > 0);
}

#[tokio::test]
async fn test_forget_low_importance() {
    let manager = create_test_manager().await;

    // Create episodes with varying importance
    for i in 0..5 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "agent-001".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        // Make some episodes more important
        episode.outcome = if i < 2 {
            EpisodeOutcome::Abandoned // Low importance
        } else {
            EpisodeOutcome::Success // High importance
        };

        manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Get initial stats
    let _initial_stats = manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    // Forget low-importance episodes
    let forgotten = manager
        .forget(0.5)
        .await
        .expect("Failed to forget episodes");

    // Should have forgotten some episodes
    assert!(forgotten > 0);
}

#[tokio::test]
async fn test_comprehensive_statistics() {
    let manager = create_test_manager().await;

    // Add some data
    let episode = EpisodicMemory::new(
        "Test task".to_string(),
        "agent-001".to_string(),
        CortexId::new(),
        EpisodeType::Task,
    );
    manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    let unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "test_function".to_string(),
        qualified_name: "test::test_function".to_string(),
        display_name: "test_function".to_string(),
        file_path: "test.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 10,
        end_column: 1,
        signature: "fn test()".to_string(),
        body: "// test".to_string(),
        docstring: None,
        visibility: "private".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "Test".to_string(),
        purpose: "Testing".to_string(),
        complexity: ComplexityMetrics::default(),
        test_coverage: None,
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    manager
        .remember_unit(&unit)
        .await
        .expect("Failed to store unit");

    manager
        .working()
        .store("test".to_string(), vec![1, 2, 3], Priority::Medium);

    // Get statistics
    let stats = manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 1);
    assert_eq!(stats.semantic.total_units, 1);
    assert_eq!(stats.working.current_items, 1);
}

#[tokio::test]
async fn test_complex_query_workflow() {
    let manager = create_test_manager().await;

    // Create a workspace with multiple related units
    let file_path = "src/user_service.rs";

    // Create several related functions
    let authenticate_id = CortexId::new();
    let validate_id = CortexId::new();
    let hash_id = CortexId::new();

    let authenticate = SemanticUnit {
        id: authenticate_id,
        unit_type: CodeUnitType::Function,
        name: "authenticate".to_string(),
        qualified_name: "user_service::authenticate".to_string(),
        display_name: "authenticate".to_string(),
        file_path: file_path.to_string(),
        start_line: 10,
        start_column: 0,
        end_line: 20,
        end_column: 1,
        signature: "pub fn authenticate(user: &str) -> Result<Token>".to_string(),
        body: "// auth logic".to_string(),
        docstring: Some("Authenticate user".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec!["user".to_string()],
        return_type: Some("Result<Token>".to_string()),
        summary: "Authenticates user".to_string(),
        purpose: "Authentication".to_string(),
        complexity: ComplexityMetrics::default(),
        test_coverage: Some(0.85),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    manager
        .remember_unit(&authenticate)
        .await
        .expect("Failed to store unit");

    // Create dependencies
    manager
        .associate(authenticate_id, validate_id, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    manager
        .associate(authenticate_id, hash_id, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    // Query units in the file
    let units = manager
        .semantic()
        .get_units_in_file(file_path)
        .await
        .expect("Failed to get units in file");

    assert_eq!(units.len(), 1);
    assert_eq!(units[0].name, "authenticate");

    // Query dependencies
    let deps = manager
        .semantic()
        .get_dependencies(authenticate_id)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps.len(), 2);
}

#[tokio::test]
async fn test_code_quality_analysis() {
    let manager = create_test_manager().await;

    // Create units with varying complexity
    let complex_unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "complex_algorithm".to_string(),
        qualified_name: "algorithms::complex_algorithm".to_string(),
        display_name: "complex_algorithm".to_string(),
        file_path: "src/algorithms.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 100,
        end_column: 1,
        signature: "fn complex_algorithm()".to_string(),
        body: "// complex code".to_string(),
        docstring: None,
        visibility: "private".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "Complex algorithm".to_string(),
        purpose: "Data processing".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 25,
            cognitive: 30,
            nesting: 5,
            lines: 100,
        },
        test_coverage: None,
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    manager
        .remember_unit(&complex_unit)
        .await
        .expect("Failed to store unit");

    // Find complex units
    let complex_units = manager
        .semantic()
        .find_complex_units(20)
        .await
        .expect("Failed to find complex units");

    assert_eq!(complex_units.len(), 1);
    assert_eq!(complex_units[0].name, "complex_algorithm");

    // Find untested units
    let untested = manager
        .semantic()
        .find_untested_units()
        .await
        .expect("Failed to find untested units");

    assert!(untested.len() > 0);

    // Find undocumented units (won't find this one as it's private)
    let undocumented = manager
        .semantic()
        .find_undocumented_units()
        .await
        .expect("Failed to find undocumented units");

    // Private function won't be in undocumented list
    assert_eq!(undocumented.len(), 0);
}
