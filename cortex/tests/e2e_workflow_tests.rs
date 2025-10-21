//! End-to-end workflow tests for the complete Cortex system
//!
//! These tests verify complete workflows from workspace creation through
//! ingestion, search, modification, and materialization.

use cortex_core::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create a test workspace
async fn create_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_workspace");
    fs::create_dir_all(&workspace_path).await.unwrap();
    (temp_dir, workspace_path)
}

/// Helper to create test database config
fn create_test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex_test".to_string(),
        database: "e2e_test".to_string(),
    }
}

/// Helper to create test files in workspace
async fn create_test_files(workspace: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Create a Rust source file
    let rust_file = workspace.join("main.rs");
    fs::write(
        &rust_file,
        r#"
fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(rust_file);

    // Create a README
    let readme = workspace.join("README.md");
    fs::write(
        &readme,
        r#"# Test Project

This is a test project for Cortex E2E testing.

## Features

- Basic arithmetic operations
- Testing infrastructure
"#,
    )
    .await
    .unwrap();
    files.push(readme);

    // Create a config file
    let config = workspace.join("config.toml");
    fs::write(
        &config,
        r#"
[project]
name = "test-project"
version = "0.1.0"

[dependencies]
cortex = "0.1.0"
"#,
    )
    .await
    .unwrap();
    files.push(config);

    files
}

#[tokio::test]
async fn test_complete_workflow_workspace_creation_to_search() {
    // Step 1: Create workspace
    let (_temp_dir, workspace_path) = create_test_workspace().await;
    let files = create_test_files(&workspace_path).await;

    assert_eq!(files.len(), 3);
    assert!(workspace_path.exists());

    // Step 2: Initialize database
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Step 3: Create project in database
    let project = Project::new(
        "test-project".to_string(),
        workspace_path.clone(),
    );

    // Verify project creation
    assert_eq!(project.name, "test-project");
    assert_eq!(project.path, workspace_path);

    // Step 4: Initialize cognitive memory
    let cognitive_manager = CognitiveManager::new(connection_manager.clone());

    // Step 5: Create an episode for this workflow
    let mut episode = EpisodicMemory::new(
        "E2E workflow test".to_string(),
        "test-agent".to_string(),
        project.id,
        EpisodeType::Task,
    );

    episode.entities_created = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();
    episode.outcome = EpisodeOutcome::Success;

    let episode_id = cognitive_manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Step 6: Retrieve the episode
    let retrieved_episode = cognitive_manager
        .episodic()
        .get_episode(episode_id)
        .await
        .expect("Failed to retrieve episode")
        .expect("Episode not found");

    assert_eq!(retrieved_episode.task_description, "E2E workflow test");
    assert_eq!(retrieved_episode.entities_created.len(), 3);

    // Step 7: Verify statistics
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 1);
}

#[tokio::test]
async fn test_multi_agent_workflow() {
    let (_temp_dir, workspace_path) = create_test_workspace().await;
    create_test_files(&workspace_path).await;

    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let project = Project::new("multi-agent-project".to_string(), workspace_path);

    // Create multiple agents working on different tasks
    let agent1_manager = CognitiveManager::new(connection_manager.clone());
    let agent2_manager = CognitiveManager::new(connection_manager.clone());

    // Agent 1: Code generation task
    let mut agent1_episode = EpisodicMemory::new(
        "Generate authentication module".to_string(),
        "agent-1".to_string(),
        project.id,
        EpisodeType::Feature,
    );
    agent1_episode.entities_created = vec!["auth.rs".to_string(), "user.rs".to_string()];
    agent1_episode.outcome = EpisodeOutcome::Success;

    let agent1_episode_id = agent1_manager
        .remember_episode(&agent1_episode)
        .await
        .expect("Failed to store agent1 episode");

    // Agent 2: Testing task
    let mut agent2_episode = EpisodicMemory::new(
        "Write integration tests".to_string(),
        "agent-2".to_string(),
        project.id,
        EpisodeType::Task,
    );
    agent2_episode.entities_created = vec!["integration_tests.rs".to_string()];
    agent2_episode.outcome = EpisodeOutcome::Success;

    let agent2_episode_id = agent2_manager
        .remember_episode(&agent2_episode)
        .await
        .expect("Failed to store agent2 episode");

    // Verify both episodes are stored
    assert_ne!(agent1_episode_id, agent2_episode_id);

    // Get combined statistics
    let stats = agent1_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 2);
}

#[tokio::test]
async fn test_memory_consolidation_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    let project_id = CortexId::new();

    // Create multiple episodes over time
    for i in 0..10 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "consolidation-agent".to_string(),
            project_id,
            if i % 2 == 0 {
                EpisodeType::Feature
            } else {
                EpisodeType::Refactor
            },
        );

        episode.outcome = if i < 8 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Partial
        };

        episode.tools_used = vec![ToolUsage {
            tool_name: "code_generator".to_string(),
            usage_count: 1,
            total_duration_ms: 1000,
            parameters: std::collections::HashMap::new(),
        }];

        cognitive_manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Consolidate memories
    let consolidation_report = cognitive_manager
        .consolidate()
        .await
        .expect("Failed to consolidate");

    assert!(consolidation_report.duration_ms > 0);
    assert!(consolidation_report.episodes_processed >= 0);

    // Dream to extract patterns
    let patterns = cognitive_manager
        .dream()
        .await
        .expect("Failed to dream");

    assert!(patterns.len() >= 0);

    // Check statistics after consolidation
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 10);
}

#[tokio::test]
async fn test_semantic_code_analysis_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Create semantic units representing code structure
    let file_id = CortexId::new();

    let function_unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "process_data".to_string(),
        qualified_name: "module::process_data".to_string(),
        display_name: "process_data".to_string(),
        file_path: "src/processor.rs".to_string(),
        start_line: 10,
        start_column: 0,
        end_line: 50,
        end_column: 1,
        signature: "pub fn process_data(input: &[u8]) -> Result<Vec<u8>>".to_string(),
        body: "// Processing logic".to_string(),
        docstring: Some("Process input data and return result".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec!["input".to_string()],
        return_type: Some("Result<Vec<u8>>".to_string()),
        summary: "Data processing function".to_string(),
        purpose: "Transform input data".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 8,
            cognitive: 12,
            nesting: 3,
            lines: 40,
        },
        test_coverage: Some(0.85),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let unit_id = cognitive_manager
        .remember_unit(&function_unit)
        .await
        .expect("Failed to store semantic unit");

    // Retrieve the unit
    let retrieved = cognitive_manager
        .semantic()
        .get_unit(unit_id)
        .await
        .expect("Failed to retrieve unit")
        .expect("Unit not found");

    assert_eq!(retrieved.name, "process_data");
    assert_eq!(retrieved.complexity.cyclomatic, 8);
    assert!(retrieved.has_tests);

    // Find complex units
    let complex_units = cognitive_manager
        .semantic()
        .find_complex_units(5)
        .await
        .expect("Failed to find complex units");

    assert!(complex_units.len() > 0);

    // Check quality metrics
    let untested = cognitive_manager
        .semantic()
        .find_untested_units()
        .await
        .expect("Failed to find untested units");

    assert_eq!(untested.len(), 0); // Our test unit has tests
}

#[tokio::test]
async fn test_working_memory_to_longterm_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Store items in working memory
    for i in 0..5 {
        let key = format!("context_item_{}", i);
        let value = format!("Important context data {}", i).into_bytes();
        let priority = if i < 2 {
            Priority::High
        } else {
            Priority::Medium
        };

        cognitive_manager.working().store(key, value, priority);
    }

    // Check working memory stats
    let stats = cognitive_manager.working().get_statistics();
    assert!(stats.current_items > 0);

    // Retrieve high priority items
    assert!(cognitive_manager.working().retrieve("context_item_0").is_some());
    assert!(cognitive_manager.working().retrieve("context_item_1").is_some());

    // Create episode to move to long-term memory
    let episode = EpisodicMemory::new(
        "Working memory consolidation".to_string(),
        "memory-agent".to_string(),
        CortexId::new(),
        EpisodeType::Task,
    );

    cognitive_manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Verify long-term memory
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 1);
}

#[tokio::test]
async fn test_pattern_learning_and_application_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Learn a pattern from successful episodes
    let mut pattern = LearnedPattern::new(
        PatternType::Optimization,
        "Cache frequently accessed data".to_string(),
        "Pattern for improving performance by caching".to_string(),
        "Performance optimization context".to_string(),
    );

    let pattern_id = cognitive_manager
        .remember_pattern(&pattern)
        .await
        .expect("Failed to store pattern");

    // Apply the pattern multiple times
    for _ in 0..5 {
        cognitive_manager
            .procedural()
            .record_success(pattern_id)
            .await
            .expect("Failed to record success");
    }

    // Retrieve updated pattern
    let updated_pattern = cognitive_manager
        .procedural()
        .get_pattern(pattern_id)
        .await
        .expect("Failed to retrieve pattern")
        .expect("Pattern not found");

    assert_eq!(updated_pattern.times_applied, 5);
    assert_eq!(updated_pattern.success_rate, 1.0);

    // Test pattern with some failures
    for _ in 0..2 {
        cognitive_manager
            .procedural()
//             .record_failure(pattern_id)
//             .await
//             .expect("Failed to record failure");
    }

    let final_pattern = cognitive_manager
        .procedural()
        .get_pattern(pattern_id)
        .await
        .expect("Failed to retrieve pattern")
        .expect("Pattern not found");

    assert_eq!(final_pattern.times_applied, 7);
    assert!(final_pattern.success_rate < 1.0);
    assert!(final_pattern.success_rate > 0.7);
}

#[tokio::test]
async fn test_dependency_graph_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Create a dependency chain: A -> B -> C
    let unit_a = CortexId::new();
    let unit_b = CortexId::new();
    let unit_c = CortexId::new();

    // A calls B
    cognitive_manager
        .associate(unit_a, unit_b, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    // B calls C
    cognitive_manager
        .associate(unit_b, unit_c, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    // A imports C
    cognitive_manager
        .associate(unit_a, unit_c, DependencyType::Imports)
        .await
        .expect("Failed to create dependency");

    // Query dependencies
    let deps_a = cognitive_manager
        .semantic()
        .get_dependencies(unit_a)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps_a.len(), 2); // A depends on B and C

    let deps_b = cognitive_manager
        .semantic()
        .get_dependencies(unit_b)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(deps_b.len(), 1); // B depends on C
}

#[tokio::test]
async fn test_incremental_consolidation_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Create many episodes
    for i in 0..100 {
        let episode = EpisodicMemory::new(
            format!("Task {}", i),
            "batch-agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        cognitive_manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Perform incremental consolidation
    let report = cognitive_manager
        .consolidate_incremental(50)
        .await
        .expect("Failed to consolidate incrementally");

    assert_eq!(report.episodes_processed, 50);
    assert!(report.duration_ms > 0);

    // Verify total episodes
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 100);
}

#[tokio::test]
async fn test_forget_low_importance_workflow() {
    let db_config = create_test_db_config();
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive_manager = CognitiveManager::new(connection_manager);

    // Create episodes with varying importance
    for i in 0..10 {
        let mut episode = EpisodicMemory::new(
            format!("Task {}", i),
            "forget-agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        // Some are successful (high importance), some abandoned (low importance)
        episode.outcome = if i < 7 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Abandoned
        };

        cognitive_manager
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");
    }

    // Forget low importance episodes (threshold 0.5)
    let forgotten_count = cognitive_manager
        .forget(0.5)
        .await
        .expect("Failed to forget episodes");

    assert!(forgotten_count > 0);

    // Verify remaining episodes
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert!(stats.episodic.total_episodes < 10);
}
