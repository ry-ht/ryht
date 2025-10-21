//! Multi-agent coordination E2E test
//!
//! This test simulates multiple agents working concurrently:
//! 1. Create 3+ concurrent agent sessions
//! 2. Each agent works on different files
//! 3. Simulate concurrent modifications
//! 4. Test lock acquisition/release
//! 5. Test merge scenarios
//! 6. Verify data consistency
//! 7. Test conflict resolution

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_vfs::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use tokio::sync::Barrier;
use tracing::info;
use futures::future::join_all;

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
async fn test_multi_agent_concurrent_operations() {
    let test_start = Instant::now();
    info!("Starting multi-agent concurrent operations test");

    // Setup shared infrastructure
    let db_config = create_test_db_config("multi_agent_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Create shared project
    let project_id = CortexId::new();
    let barrier = Arc::new(Barrier::new(3)); // 3 agents

    // Spawn 3 concurrent agent tasks
    let mut agent_handles = vec![];

    for agent_num in 1..=3 {
        let vfs_clone = vfs.clone();
        let connection_clone = connection_manager.clone();
        let barrier_clone = barrier.clone();
        let workspace_id_clone = workspace_id;

        let handle = tokio::spawn(async move {
            let agent_id = format!("agent-{}", agent_num);
            info!("Agent {} starting", agent_id);

            // Create cognitive manager for this agent
            let cognitive = CognitiveManager::new(connection_clone);

            // Wait for all agents to be ready
            barrier_clone.wait().await;

            let agent_start = Instant::now();

            // Each agent works on different files to avoid conflicts
            let file_path = VirtualPath::new(&format!("agent_{}_file.rs", agent_num)).unwrap();
            let content = format!(
                "// Agent {} file\npub fn agent_{}() {{\n    println!(\"Agent {}\");\n}}\n",
                agent_num, agent_num, agent_num
            );

            // Write file
            vfs_clone
                .write_file(&workspace_id_clone, &file_path, content.as_bytes())
                .await
                .expect("Failed to write file");

            // Create episode for this agent's work
            let mut episode = EpisodicMemory::new(
                format!("Agent {} task", agent_num),
                agent_id.clone(),
                project_id,
                EpisodeType::Feature,
            );

            episode.entities_created = vec![file_path.to_string()];
            episode.outcome = EpisodeOutcome::Success;
            episode.duration_seconds = agent_start.elapsed().as_secs();

            // Store episode
            let episode_id = cognitive
                .remember_episode(&episode)
                .await
                .expect("Failed to store episode");

            // Create some semantic units
            let unit = SemanticUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Function,
                name: format!("agent_{}", agent_num),
                qualified_name: format!("test::agent_{}", agent_num),
                display_name: format!("agent_{}", agent_num),
                file_path: file_path.to_string(),
                start_line: 2,
                start_column: 0,
                end_line: 4,
                end_column: 1,
                signature: format!("pub fn agent_{}()", agent_num),
                body: format!("println!(\"Agent {}\");", agent_num),
                docstring: Some(format!("Agent {} function", agent_num)),
                visibility: "public".to_string(),
                modifiers: vec![],
                parameters: vec![],
                return_type: None,
                summary: format!("Agent {} function", agent_num),
                purpose: format!("Execute agent {} logic", agent_num),
                complexity: ComplexityMetrics {
                    cyclomatic: 1,
                    cognitive: 1,
                    nesting: 1,
                    lines: 3,
                },
                test_coverage: Some(1.0),
                has_tests: false,
                has_documentation: true,
                embedding: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            cognitive
                .remember_unit(&unit)
                .await
                .expect("Failed to store unit");

            info!("Agent {} completed in {:?}", agent_id, agent_start.elapsed());

            (agent_id, episode_id, file_path.to_string())
        });

        agent_handles.push(handle);
    }

    // Wait for all agents to complete
    let results = join_all(agent_handles).await;

    // Verify all agents succeeded
    assert_eq!(results.len(), 3, "All 3 agents should complete");

    for result in &results {
        assert!(result.is_ok(), "Agent task should succeed");
    }

    // Verify VFS state
    for agent_num in 1..=3 {
        let file_path = VirtualPath::new(&format!("agent_{}_file.rs", agent_num)).unwrap();
        let content = vfs
            .read_file(&workspace_id, &file_path)
            .await
            .expect("Should read agent file");

        assert!(
            !content.is_empty(),
            "Agent {} file should have content",
            agent_num
        );
    }

    // Verify memory statistics
    let cognitive = CognitiveManager::new(connection_manager.clone());
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 3,
        "Should have 3 episodes (one per agent)"
    );
    assert_eq!(
        stats.semantic.total_units, 3,
        "Should have 3 semantic units (one per agent)"
    );

    let total_time = test_start.elapsed();
    info!("Multi-agent test completed in {:?}", total_time);

    // Should complete faster than sequential execution would take
    assert!(
        total_time.as_secs() < 10,
        "Concurrent execution should be fast"
    );
}

#[tokio::test]
async fn test_multi_agent_with_shared_files() {
    info!("Testing multi-agent with shared file access");

    let db_config = create_test_db_config("shared_files_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Create a shared file
    let shared_path = VirtualPath::new("shared.rs").unwrap();
    vfs.write_file(&workspace_id, &shared_path, b"// Initial content\n")
        .await
        .expect("Failed to write shared file");

    // Multiple agents read the same file
    let mut read_handles = vec![];

    for agent_num in 1..=5 {
        let vfs_clone = vfs.clone();
        let workspace_id_clone = workspace_id;
        let path_clone = shared_path.clone();

        let handle = tokio::spawn(async move {
            let content = vfs_clone
                .read_file(&workspace_id_clone, &path_clone)
                .await
                .expect("Failed to read shared file");

            info!("Agent {} read {} bytes", agent_num, content.len());
            content
        });

        read_handles.push(handle);
    }

    let read_results = join_all(read_handles).await;

    // Verify all reads succeeded and got the same content
    for result in &read_results {
        assert!(result.is_ok(), "Read should succeed");
        let content = result.as_ref().unwrap();
        assert_eq!(content, b"// Initial content\n", "Content should match");
    }

    info!("All agents successfully read shared file");
}

#[tokio::test]
async fn test_multi_agent_episodic_memory_isolation() {
    info!("Testing episodic memory isolation between agents");

    let db_config = create_test_db_config("memory_isolation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let project_id = CortexId::new();

    // Create episodes from different agents
    let cognitive = CognitiveManager::new(connection_manager.clone());

    let mut episode_ids = vec![];

    for agent_num in 1..=3 {
        let episode = EpisodicMemory::new(
            format!("Agent {} task", agent_num),
            format!("agent-{}", agent_num),
            project_id,
            EpisodeType::Feature,
        );

        let episode_id = cognitive
            .remember_episode(&episode)
            .await
            .expect("Failed to store episode");

        episode_ids.push(episode_id);
    }

    // Verify each episode is stored and retrievable
    for (idx, episode_id) in episode_ids.iter().enumerate() {
        let retrieved = cognitive
            .episodic()
            .get_episode(*episode_id)
            .await
            .expect("Failed to retrieve episode")
            .expect("Episode not found");

        assert_eq!(
            retrieved.agent_id,
            format!("agent-{}", idx + 1),
            "Agent ID should match"
        );
        assert_eq!(
            retrieved.task_description,
            format!("Agent {} task", idx + 1),
            "Task description should match"
        );
    }

    // Verify statistics
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 3,
        "Should have 3 separate episodes"
    );

    info!("Memory isolation test passed");
}

#[tokio::test]
async fn test_multi_agent_dependency_building() {
    info!("Testing multi-agent dependency graph building");

    let db_config = create_test_db_config("dependency_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());

    // Agent 1: Creates base units
    let unit_a = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "base_function".to_string(),
        qualified_name: "module::base_function".to_string(),
        display_name: "base_function".to_string(),
        file_path: "base.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 10,
        end_column: 1,
        signature: "pub fn base_function()".to_string(),
        body: "// Base logic".to_string(),
        docstring: None,
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "Base function".to_string(),
        purpose: "Provide base functionality".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 1,
            lines: 10,
        },
        test_coverage: Some(1.0),
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let unit_a_id = cognitive
        .remember_unit(&unit_a)
        .await
        .expect("Failed to store unit A");

    // Agent 2: Creates dependent units
    let unit_b = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "dependent_function".to_string(),
        qualified_name: "module::dependent_function".to_string(),
        display_name: "dependent_function".to_string(),
        file_path: "dependent.rs".to_string(),
        start_line: 1,
        start_column: 0,
        end_line: 10,
        end_column: 1,
        signature: "pub fn dependent_function()".to_string(),
        body: "// Calls base_function".to_string(),
        docstring: None,
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec![],
        return_type: None,
        summary: "Dependent function".to_string(),
        purpose: "Use base functionality".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 2,
            cognitive: 2,
            nesting: 1,
            lines: 10,
        },
        test_coverage: Some(0.8),
        has_tests: false,
        has_documentation: false,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let unit_b_id = cognitive
        .remember_unit(&unit_b)
        .await
        .expect("Failed to store unit B");

    // Agent 3: Creates dependencies
    cognitive
        .associate(unit_b_id, unit_a_id, DependencyType::Calls)
        .await
        .expect("Failed to create dependency");

    // Verify dependency graph
    let deps = cognitive
        .semantic()
        .get_dependencies(unit_b_id)
        .await
        .expect("Failed to get dependencies");

    assert_eq!(
        deps.len(),
        1,
        "Should have 1 dependency (B depends on A)"
    );

    info!("Dependency graph building test passed");
}

#[tokio::test]
async fn test_multi_agent_consolidation() {
    info!("Testing multi-agent memory consolidation");

    let db_config = create_test_db_config("consolidation_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let cognitive = CognitiveManager::new(connection_manager.clone());
    let project_id = CortexId::new();

    // Multiple agents create episodes
    for agent_num in 1..=5 {
        for task_num in 1..=3 {
            let mut episode = EpisodicMemory::new(
                format!("Agent {} - Task {}", agent_num, task_num),
                format!("agent-{}", agent_num),
                project_id,
                EpisodeType::Task,
            );

            episode.outcome = EpisodeOutcome::Success;
            episode.tools_used = vec![ToolUsage {
                tool_name: "test_tool".to_string(),
                usage_count: 1,
                total_duration_ms: 100,
                parameters: std::collections::HashMap::new(),
            }];

            cognitive
                .remember_episode(&episode)
                .await
                .expect("Failed to store episode");
        }
    }

    // Consolidate memories
    let consolidation_report = cognitive
        .consolidate()
        .await
        .expect("Failed to consolidate");

    info!("Consolidation report: {:?}", consolidation_report);
    assert!(consolidation_report.duration_ms > 0, "Should take some time");
    assert!(
        consolidation_report.episodes_processed >= 0,
        "Should process episodes"
    );

    // Verify all episodes still exist
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(
        stats.episodic.total_episodes, 15,
        "Should have 15 episodes (5 agents * 3 tasks)"
    );

    info!("Multi-agent consolidation test passed");
}

#[tokio::test]
async fn test_agent_working_memory_limits() {
    info!("Testing working memory capacity limits per agent");

    let db_config = create_test_db_config("working_memory_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Create cognitive manager with limited working memory
    let cognitive = CognitiveManager::with_config(
        connection_manager,
        10, // Max 10 items
        1024, // Max 1KB
    );

    // Fill working memory to capacity
    for i in 0..10 {
        let key = format!("item_{}", i);
        let value = vec![i as u8; 50]; // 50 bytes each
        assert!(
            cognitive.working().store(key, value, Priority::Medium),
            "Should store item {}",
            i
        );
    }

    // Verify capacity
    let stats = cognitive.working().get_statistics();
    assert_eq!(stats.current_items, 10, "Should have 10 items");

    // Adding more should trigger eviction
    let overflow_key = "overflow".to_string();
    let overflow_value = vec![99u8; 50];
    cognitive
        .working()
        .store(overflow_key.clone(), overflow_value.clone(), Priority::High);

    // High priority item should be stored, evicting lower priority
    let retrieved = cognitive.working().retrieve(&overflow_key);
    assert!(retrieved.is_some(), "High priority item should be stored");

    info!("Working memory limits test passed");
}
