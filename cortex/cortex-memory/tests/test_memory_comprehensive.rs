//! Comprehensive tests for the Cortex Cognitive Memory System
//!
//! This test suite validates:
//! - All 5 memory tiers (Working, Episodic, Semantic, Procedural, Consolidation)
//! - Memory consolidation with decay simulation
//! - Pattern extraction from episodes
//! - Cross-session knowledge transfer
//! - Realistic development scenarios (50+ episodes)
//! - Performance metrics and latency measurements

use cortex_core::id::CortexId;
use cortex_memory::prelude::*;
use cortex_storage::connection_pool::{self, ConnectionManager, DatabaseConfig, Credentials, PoolConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use chrono::Utc;

// ============================================================================
// Test Utilities
// ============================================================================

async fn create_test_cognitive_manager() -> CognitiveManager {
    let config = DatabaseConfig {
        connection_mode: connection_pool::ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
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

/// Create a realistic episode simulating development work
fn create_realistic_episode(
    task_type: &str,
    task_number: usize,
    outcome: EpisodeOutcome,
) -> EpisodicMemory {
    let episode_type = match task_type {
        "bugfix" => EpisodeType::Bugfix,
        "feature" => EpisodeType::Feature,
        "refactor" => EpisodeType::Refactor,
        "test" => EpisodeType::Task,
        _ => EpisodeType::Exploration,
    };

    let mut episode = EpisodicMemory::new(
        format!("{} #{}: {}", task_type, task_number, get_task_description(task_type, task_number)),
        format!("agent-{}", task_number % 3),
        CortexId::new(),
        episode_type,
    );

    // Add realistic work performed
    episode.files_touched = get_files_for_task(task_type, task_number);
    episode.tools_used = get_tools_for_task(task_type);
    episode.outcome = outcome;
    episode.duration_seconds = 60 + (task_number as u64 % 600);
    episode.tokens_used = TokenUsage {
        input: 1000 + (task_number as u64 * 100),
        output: 500 + (task_number as u64 * 50),
        total: 1500 + (task_number as u64 * 150),
    };

    // Add entities modified based on task type
    match task_type {
        "bugfix" => {
            episode.entities_modified = vec![
                format!("function::fix_bug_{}", task_number),
                format!("test::test_bug_fix_{}", task_number),
            ];
            episode.lessons_learned = vec![
                format!("Always add tests when fixing bugs"),
                format!("Check edge cases for similar bugs"),
            ];
        }
        "feature" => {
            episode.entities_created = vec![
                format!("function::new_feature_{}", task_number),
                format!("test::test_feature_{}", task_number),
                format!("docs::feature_{}_docs", task_number),
            ];
            episode.lessons_learned = vec![
                format!("When implementing features, also add tests"),
                format!("Document public APIs immediately"),
            ];
        }
        "refactor" => {
            episode.entities_modified = vec![
                format!("function::refactored_{}", task_number),
            ];
            episode.lessons_learned = vec![
                format!("Ensure tests pass after refactoring"),
            ];
        }
        _ => {}
    }

    // Add success metrics
    if outcome == EpisodeOutcome::Success {
        episode.success_metrics.insert("tests_passing".to_string(), 1.0);
        episode.success_metrics.insert("code_quality".to_string(), 0.85);
    }

    episode.solution_summary = format!("Completed {} task successfully", task_type);
    episode.completed_at = Some(Utc::now());

    episode
}

fn get_task_description(task_type: &str, _task_number: usize) -> String {
    match task_type {
        "bugfix" => format!("Fix null pointer in authentication module"),
        "feature" => format!("Add new caching layer to API"),
        "refactor" => format!("Simplify database connection pool"),
        "test" => format!("Add integration tests for user service"),
        _ => format!("Explore performance optimization"),
    }
}

fn get_files_for_task(task_type: &str, _task_number: usize) -> Vec<String> {
    match task_type {
        "bugfix" => vec![
            "src/auth/login.rs".to_string(),
            "tests/auth_tests.rs".to_string(),
        ],
        "feature" => vec![
            "src/cache/layer.rs".to_string(),
            "src/api/handler.rs".to_string(),
            "tests/cache_tests.rs".to_string(),
        ],
        "refactor" => vec![
            "src/db/pool.rs".to_string(),
        ],
        "test" => vec![
            "tests/integration/user_service.rs".to_string(),
        ],
        _ => vec![
            "src/performance/profiler.rs".to_string(),
        ],
    }
}

fn get_tools_for_task(task_type: &str) -> Vec<ToolUsage> {
    let mut tools = vec![
        ToolUsage {
            tool_name: "code_search".to_string(),
            usage_count: 5,
            total_duration_ms: 1000,
            parameters: HashMap::new(),
        },
    ];

    match task_type {
        "bugfix" | "feature" => {
            tools.push(ToolUsage {
                tool_name: "run_tests".to_string(),
                usage_count: 10,
                total_duration_ms: 5000,
                parameters: HashMap::new(),
            });
        }
        "refactor" => {
            tools.push(ToolUsage {
                tool_name: "static_analysis".to_string(),
                usage_count: 3,
                total_duration_ms: 2000,
                parameters: HashMap::new(),
            });
        }
        _ => {}
    }

    tools
}

// ============================================================================
// Tier 1: Working Memory Tests
// ============================================================================

#[tokio::test]
async fn test_working_memory_basic_operations() {
    println!("\n=== Testing Working Memory Basic Operations ===");

    let manager = create_test_cognitive_manager().await;
    let working = manager.working();

    // Test store and retrieve
    let key = "test_context".to_string();
    let value = b"Current task context data".to_vec();

    let start = Instant::now();
    assert!(working.store(key.clone(), value.clone(), Priority::High));
    let store_latency = start.elapsed();

    let start = Instant::now();
    let retrieved = working.retrieve(&key);
    let retrieve_latency = start.elapsed();

    assert_eq!(retrieved, Some(value));

    println!("  ✓ Store latency: {:?}", store_latency);
    println!("  ✓ Retrieve latency: {:?}", retrieve_latency);
    println!("  ✓ Basic operations working correctly");
}

#[tokio::test]
async fn test_working_memory_capacity_and_eviction() {
    println!("\n=== Testing Working Memory Capacity and Eviction ===");

    let manager = create_test_cognitive_manager().await;
    let working = manager.working();

    // Fill memory with items of different priorities
    for i in 0..15 {
        let priority = match i % 4 {
            0 => Priority::Critical,
            1 => Priority::High,
            2 => Priority::Medium,
            _ => Priority::Low,
        };
        working.store(
            format!("item_{}", i),
            format!("data_{}", i).into_bytes(),
            priority,
        );
    }

    let stats = working.get_statistics();
    println!("  ✓ Items stored: {}", stats.current_items);
    println!("  ✓ Evictions occurred: {}", stats.total_evictions);
    println!("  ✓ Cache hit rate: {:.2}%", stats.cache_hit_rate * 100.0);

    // Verify high priority items are retained
    assert!(working.retrieve("item_0").is_some() || working.retrieve("item_1").is_some());
}

#[tokio::test]
async fn test_working_memory_priority_based_retention() {
    println!("\n=== Testing Priority-Based Retention ===");

    let config = DatabaseConfig {
        connection_mode: connection_pool::ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex".to_string(),
        database: "test".to_string(),
    };

    let conn_manager = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager"),
    );

    let manager = CognitiveManager::with_config(
        conn_manager,
        5, // Small capacity to force eviction
        10240,
    );

    let working = manager.working();

    // Store items with different priorities
    working.store("critical".to_string(), b"critical data".to_vec(), Priority::Critical);
    working.store("low1".to_string(), b"low data 1".to_vec(), Priority::Low);
    working.store("low2".to_string(), b"low data 2".to_vec(), Priority::Low);
    working.store("low3".to_string(), b"low data 3".to_vec(), Priority::Low);
    working.store("low4".to_string(), b"low data 4".to_vec(), Priority::Low);

    // Add another critical item - should evict low priority
    working.store("critical2".to_string(), b"critical data 2".to_vec(), Priority::Critical);

    // Critical items should still be present
    assert!(working.retrieve("critical").is_some(), "Critical item should be retained");
    assert!(working.retrieve("critical2").is_some(), "New critical item should be stored");

    println!("  ✓ Priority-based retention working correctly");
}

// ============================================================================
// Tier 2: Episodic Memory Tests
// ============================================================================

#[tokio::test]
async fn test_episodic_memory_storage_and_retrieval() {
    println!("\n=== Testing Episodic Memory Storage and Retrieval ===");

    let manager = create_test_cognitive_manager().await;

    let episode = create_realistic_episode("bugfix", 1, EpisodeOutcome::Success);

    let start = Instant::now();
    let id = manager.remember_episode(&episode).await.unwrap();
    let store_latency = start.elapsed();

    let start = Instant::now();
    let retrieved = manager.episodic().get_episode(id).await.unwrap();
    let retrieve_latency = start.elapsed();

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, episode.id);
    assert_eq!(retrieved.outcome, EpisodeOutcome::Success);

    println!("  ✓ Store latency: {:?}", store_latency);
    println!("  ✓ Retrieve latency: {:?}", retrieve_latency);
    println!("  ✓ Episode stored and retrieved successfully");
}

#[tokio::test]
async fn test_episodic_memory_importance_calculation() {
    println!("\n=== Testing Episodic Memory Importance Calculation ===");

    let manager = create_test_cognitive_manager().await;

    // Create episodes with different characteristics
    let successful_complex = create_realistic_episode("feature", 1, EpisodeOutcome::Success);
    let failed_simple = create_realistic_episode("refactor", 2, EpisodeOutcome::Failure);

    let importance_success = manager.episodic().calculate_importance(&successful_complex);
    let importance_failure = manager.episodic().calculate_importance(&failed_simple);

    println!("  ✓ Successful complex episode importance: {:.3}", importance_success);
    println!("  ✓ Failed simple episode importance: {:.3}", importance_failure);

    // Successful episodes should generally have higher importance
    assert!(importance_success > importance_failure,
        "Successful episodes should have higher importance than failed ones");
}

#[tokio::test]
async fn test_episodic_memory_by_outcome() {
    println!("\n=== Testing Episodic Memory Retrieval by Outcome ===");

    let manager = create_test_cognitive_manager().await;

    // Store episodes with different outcomes
    for i in 0..10 {
        let outcome = if i % 3 == 0 {
            EpisodeOutcome::Success
        } else if i % 3 == 1 {
            EpisodeOutcome::Failure
        } else {
            EpisodeOutcome::Partial
        };

        let episode = create_realistic_episode("bugfix", i, outcome);
        manager.remember_episode(&episode).await.unwrap();
    }

    let successful = manager.episodic()
        .retrieve_by_outcome(EpisodeOutcome::Success, 20)
        .await
        .unwrap();

    let failed = manager.episodic()
        .retrieve_by_outcome(EpisodeOutcome::Failure, 20)
        .await
        .unwrap();

    println!("  ✓ Successful episodes retrieved: {}", successful.len());
    println!("  ✓ Failed episodes retrieved: {}", failed.len());

    assert!(successful.len() >= 3, "Should retrieve successful episodes");
    assert!(failed.len() >= 3, "Should retrieve failed episodes");
}

// ============================================================================
// Tier 3: Semantic Memory Tests
// ============================================================================

#[tokio::test]
async fn test_semantic_memory_code_units() {
    println!("\n=== Testing Semantic Memory Code Units ===");

    let manager = create_test_cognitive_manager().await;

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
        signature: "pub fn authenticate_user(credentials: &Credentials) -> Result<Session>".to_string(),
        body: "// Authentication logic".to_string(),
        docstring: Some("Authenticates a user with provided credentials".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec!["credentials".to_string()],
        return_type: Some("Result<Session>".to_string()),
        summary: "User authentication function".to_string(),
        purpose: "Validates credentials and creates session".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 5,
            cognitive: 7,
            nesting: 2,
            lines: 20,
        },
        test_coverage: Some(0.95),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let start = Instant::now();
    let id = manager.remember_unit(&unit).await.unwrap();
    let store_latency = start.elapsed();

    let start = Instant::now();
    let retrieved = manager.semantic().get_semantic_unit(id).await.unwrap();
    let retrieve_latency = start.elapsed();

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "authenticate_user");
    assert_eq!(retrieved.complexity.cyclomatic, 5);

    println!("  ✓ Store latency: {:?}", store_latency);
    println!("  ✓ Retrieve latency: {:?}", retrieve_latency);
    println!("  ✓ Code unit stored and retrieved successfully");
}

#[tokio::test]
async fn test_semantic_memory_dependencies() {
    println!("\n=== Testing Semantic Memory Dependencies ===");

    let manager = create_test_cognitive_manager().await;

    let source_id = CortexId::new();
    let target_id = CortexId::new();

    let start = Instant::now();
    manager.associate(source_id, target_id, DependencyType::Calls).await.unwrap();
    let associate_latency = start.elapsed();

    let start = Instant::now();
    let deps = manager.semantic().get_dependencies(source_id).await.unwrap();
    let retrieve_latency = start.elapsed();

    println!("  ✓ Associate latency: {:?}", associate_latency);
    println!("  ✓ Retrieve latency: {:?}", retrieve_latency);
    println!("  ✓ Dependencies: {}", deps.len());
}

#[tokio::test]
async fn test_semantic_memory_complexity_analysis() {
    println!("\n=== Testing Semantic Memory Complexity Analysis ===");

    let manager = create_test_cognitive_manager().await;

    // Create units with varying complexity
    for i in 0..5 {
        let complexity = (i + 1) * 3;
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("function_{}", i),
            qualified_name: format!("module::function_{}", i),
            display_name: format!("function_{}", i),
            file_path: "src/test.rs".to_string(),
            start_line: i * 20,
            start_column: 0,
            end_line: (i + 1) * 20,
            end_column: 1,
            signature: format!("fn function_{}()", i),
            body: "// body".to_string(),
            docstring: None,
            visibility: "public".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: "Test function".to_string(),
            purpose: "Testing".to_string(),
            complexity: ComplexityMetrics {
                cyclomatic: complexity,
                cognitive: complexity + 2,
                nesting: i,
                lines: (i + 1) * 10,
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.remember_unit(&unit).await.unwrap();
    }

    // Find complex units
    let complex_units = manager.semantic().find_complex_units(10).await.unwrap();

    println!("  ✓ Complex units found (>10 cyclomatic): {}", complex_units.len());
    assert!(complex_units.len() >= 2, "Should find units with high complexity");
}

// ============================================================================
// Tier 4: Procedural Memory Tests
// ============================================================================

#[tokio::test]
async fn test_procedural_memory_pattern_storage() {
    println!("\n=== Testing Procedural Memory Pattern Storage ===");

    let manager = create_test_cognitive_manager().await;

    let pattern = LearnedPattern::new(
        PatternType::Code,
        "Add tests when fixing bugs".to_string(),
        "Pattern extracted from successful bug fix episodes".to_string(),
        "Bug fixing context".to_string(),
    );

    let start = Instant::now();
    let id = manager.remember_pattern(&pattern).await.unwrap();
    let store_latency = start.elapsed();

    let start = Instant::now();
    let retrieved = manager.procedural().get_pattern(id).await.unwrap();
    let retrieve_latency = start.elapsed();

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Add tests when fixing bugs");

    println!("  ✓ Store latency: {:?}", store_latency);
    println!("  ✓ Retrieve latency: {:?}", retrieve_latency);
    println!("  ✓ Pattern stored and retrieved successfully");
}

#[tokio::test]
async fn test_procedural_memory_pattern_application() {
    println!("\n=== Testing Procedural Memory Pattern Application ===");

    let manager = create_test_cognitive_manager().await;

    let pattern = LearnedPattern::new(
        PatternType::Code,
        "Feature implementation pattern".to_string(),
        "When implementing features, also add tests and documentation".to_string(),
        "Feature development".to_string(),
    );

    let id = manager.remember_pattern(&pattern).await.unwrap();

    // Record successful applications
    for _ in 0..10 {
        manager.procedural().record_success(id).await.unwrap();
    }

    let updated = manager.procedural().get_pattern(id).await.unwrap().unwrap();

    println!("  ✓ Times applied: {}", updated.times_applied);
    println!("  ✓ Success rate: {:.2}%", updated.success_rate * 100.0);

    assert_eq!(updated.times_applied, 10);
    assert!(updated.success_rate > 0.9, "Success rate should be high");
}

// ============================================================================
// Tier 5: Memory Consolidation Tests
// ============================================================================

#[tokio::test]
async fn test_memory_consolidation_basic() {
    println!("\n=== Testing Basic Memory Consolidation ===");

    let manager = create_test_cognitive_manager().await;

    // Store some episodes
    for i in 0..10 {
        let episode = create_realistic_episode("bugfix", i, EpisodeOutcome::Success);
        manager.remember_episode(&episode).await.unwrap();
    }

    let start = Instant::now();
    let report = manager.consolidate().await.unwrap();
    let consolidation_latency = start.elapsed();

    println!("  ✓ Consolidation latency: {:?}", consolidation_latency);
    println!("  ✓ Episodes processed: {}", report.episodes_processed);
    println!("  ✓ Patterns extracted: {}", report.patterns_extracted);
    println!("  ✓ Memories decayed: {}", report.memories_decayed);
    println!("  ✓ Knowledge links created: {}", report.knowledge_links_created);
}

#[tokio::test]
async fn test_memory_decay_simulation() {
    println!("\n=== Testing Memory Decay Simulation ===");

    let manager = create_test_cognitive_manager().await;

    // Store episodes with varying importance
    let mut episode_ids = Vec::new();
    for i in 0..20 {
        let outcome = if i % 2 == 0 {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Failure
        };

        let episode = create_realistic_episode("refactor", i, outcome);
        let id = manager.remember_episode(&episode).await.unwrap();
        episode_ids.push(id);
    }

    // Check importance before decay
    let stats_before = manager.episodic().get_statistics().await.unwrap();
    println!("  ✓ Episodes before decay: {}", stats_before.total_episodes);

    // Apply decay (forget unimportant memories)
    let forgotten = manager.forget(0.3).await.unwrap();

    let stats_after = manager.episodic().get_statistics().await.unwrap();
    println!("  ✓ Episodes after decay: {}", stats_after.total_episodes);
    println!("  ✓ Memories decayed: {}", forgotten);

    assert!(forgotten > 0, "Some memories should be decayed");
    assert!(stats_after.total_episodes < stats_before.total_episodes, "Total episodes should decrease");
}

#[tokio::test]
async fn test_pattern_extraction_from_episodes() {
    println!("\n=== Testing Pattern Extraction from Episodes ===");

    let manager = create_test_cognitive_manager().await;

    // Store multiple successful episodes of the same type
    for i in 0..15 {
        let episode = create_realistic_episode("bugfix", i, EpisodeOutcome::Success);
        manager.remember_episode(&episode).await.unwrap();
    }

    let start = Instant::now();
    let patterns = manager.episodic().extract_patterns(0.6).await.unwrap();
    let extraction_latency = start.elapsed();

    println!("  ✓ Pattern extraction latency: {:?}", extraction_latency);
    println!("  ✓ Patterns extracted: {}", patterns.len());

    for (idx, pattern) in patterns.iter().enumerate() {
        println!("    - Pattern {}: {}", idx + 1, pattern.name);
        println!("      Context: {}", pattern.context);
    }

    assert!(patterns.len() > 0, "Should extract patterns from similar episodes");
}

// ============================================================================
// Realistic Scenario: 50 Development Episodes
// ============================================================================

#[tokio::test]
async fn test_realistic_50_episode_scenario() {
    println!("\n=== Testing Realistic 50 Development Episodes ===");

    let manager = create_test_cognitive_manager().await;

    let task_types = vec!["bugfix", "feature", "refactor", "test", "explore"];
    let mut total_store_time = Duration::ZERO;

    // Simulate 50 development episodes
    for i in 0..50 {
        let task_type = task_types[i % task_types.len()];
        let outcome = if i % 5 == 4 {
            EpisodeOutcome::Failure
        } else if i % 7 == 0 {
            EpisodeOutcome::Partial
        } else {
            EpisodeOutcome::Success
        };

        let episode = create_realistic_episode(task_type, i, outcome);

        let start = Instant::now();
        manager.remember_episode(&episode).await.unwrap();
        total_store_time += start.elapsed();

        // Periodically consolidate (simulate dream/background consolidation)
        if i > 0 && i % 10 == 0 {
            let consolidation_start = Instant::now();
            let report = manager.consolidate_incremental(10).await.unwrap();
            let consolidation_time = consolidation_start.elapsed();

            println!("\n  Consolidation at episode {}:", i);
            println!("    Duration: {:?}", consolidation_time);
            println!("    Patterns extracted: {}", report.patterns_extracted);
        }
    }

    // Get final statistics
    let stats = manager.get_statistics().await.unwrap();

    println!("\n  === Final Statistics ===");
    println!("  Episodes stored: {}", stats.episodic.total_episodes);
    println!("  Successful episodes: {}", stats.episodic.successful_episodes);
    println!("  Failed episodes: {}", stats.episodic.failed_episodes);
    println!("  Average duration: {:.2}s", stats.episodic.average_duration_seconds);
    println!("  Total tokens used: {}", stats.episodic.total_tokens_used);
    println!("  Semantic units: {}", stats.semantic.total_units);
    println!("  Learned patterns: {}", stats.procedural.total_patterns);
    println!("  Working memory items: {}", stats.working.current_items);
    println!("  Average store latency: {:?}", total_store_time / 50);

    assert_eq!(stats.episodic.total_episodes, 50, "Should have 50 episodes");
    assert!(stats.episodic.successful_episodes > 30, "Should have many successful episodes");
}

#[tokio::test]
async fn test_knowledge_transfer_across_sessions() {
    println!("\n=== Testing Knowledge Transfer Across Sessions ===");

    let manager = create_test_cognitive_manager().await;

    // Session 1: Learn from authentication-related work
    println!("\n  Session 1: Learning from authentication work");
    for i in 0..10 {
        let mut episode = create_realistic_episode("bugfix", i, EpisodeOutcome::Success);
        episode.task_description = format!("Fix authentication bug #{}", i);
        episode.files_touched = vec!["src/auth.rs".to_string(), "tests/auth_tests.rs".to_string()];
        episode.lessons_learned = vec![
            "Always validate tokens before use".to_string(),
            "Add tests for edge cases in auth".to_string(),
        ];
        manager.remember_episode(&episode).await.unwrap();
    }

    // Consolidate and extract patterns
    let report1 = manager.consolidate().await.unwrap();
    println!("    Patterns extracted: {}", report1.patterns_extracted);

    // Session 2: Apply learned patterns to new authentication work
    println!("\n  Session 2: Applying learned patterns to new work");

    // The system should have learned to add tests when working on auth
    let patterns = manager.episodic().extract_patterns(0.5).await.unwrap();

    println!("    Available patterns for new work: {}", patterns.len());
    for pattern in &patterns {
        println!("      - {}", pattern.name);
    }

    // Store patterns in procedural memory
    for pattern in patterns {
        manager.remember_pattern(&pattern).await.unwrap();
    }

    let final_stats = manager.get_statistics().await.unwrap();
    println!("\n  Knowledge Transfer Results:");
    println!("    Total episodes: {}", final_stats.episodic.total_episodes);
    println!("    Learned patterns: {}", final_stats.procedural.total_patterns);

    assert!(final_stats.procedural.total_patterns > 0,
        "Should have learned patterns from first session");
}

// ============================================================================
// Performance Benchmarks
// ============================================================================

#[tokio::test]
async fn test_memory_performance_benchmarks() {
    println!("\n=== Memory System Performance Benchmarks ===");

    let manager = create_test_cognitive_manager().await;

    // Benchmark 1: Episode storage throughput
    let mut episode_store_times = Vec::new();
    for i in 0..100 {
        let episode = create_realistic_episode("feature", i, EpisodeOutcome::Success);
        let start = Instant::now();
        manager.remember_episode(&episode).await.unwrap();
        episode_store_times.push(start.elapsed());
    }

    let avg_episode_store = episode_store_times.iter().sum::<Duration>() / 100;
    let max_episode_store = episode_store_times.iter().max().unwrap();
    let min_episode_store = episode_store_times.iter().min().unwrap();

    println!("\n  Episode Storage:");
    println!("    Average: {:?}", avg_episode_store);
    println!("    Min: {:?}", min_episode_store);
    println!("    Max: {:?}", max_episode_store);

    // Benchmark 2: Working memory operations
    let working = manager.working();
    let mut working_store_times = Vec::new();
    let mut working_retrieve_times = Vec::new();

    for i in 0..1000 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i).into_bytes();

        let start = Instant::now();
        working.store(key.clone(), value, Priority::Medium);
        working_store_times.push(start.elapsed());

        let start = Instant::now();
        working.retrieve(&key);
        working_retrieve_times.push(start.elapsed());
    }

    let avg_working_store = working_store_times.iter().sum::<Duration>() / 1000;
    let avg_working_retrieve = working_retrieve_times.iter().sum::<Duration>() / 1000;

    println!("\n  Working Memory:");
    println!("    Average store: {:?}", avg_working_store);
    println!("    Average retrieve: {:?}", avg_working_retrieve);

    // Benchmark 3: Pattern extraction
    let start = Instant::now();
    let patterns = manager.episodic().extract_patterns(0.6).await.unwrap();
    let pattern_extraction_time = start.elapsed();

    println!("\n  Pattern Extraction:");
    println!("    Time: {:?}", pattern_extraction_time);
    println!("    Patterns found: {}", patterns.len());

    // Benchmark 4: Memory consolidation
    let start = Instant::now();
    let report = manager.consolidate().await.unwrap();
    let consolidation_time = start.elapsed();

    println!("\n  Memory Consolidation:");
    println!("    Time: {:?}", consolidation_time);
    println!("    Episodes processed: {}", report.episodes_processed);
    println!("    Patterns extracted: {}", report.patterns_extracted);

    // Performance assertions
    assert!(avg_episode_store < Duration::from_millis(100),
        "Episode storage should be fast (< 100ms)");
    assert!(avg_working_store < Duration::from_micros(100),
        "Working memory store should be very fast (< 100μs)");
    assert!(avg_working_retrieve < Duration::from_micros(50),
        "Working memory retrieve should be extremely fast (< 50μs)");
}

#[tokio::test]
async fn test_pattern_quality_assessment() {
    println!("\n=== Testing Pattern Quality Assessment ===");

    let manager = create_test_cognitive_manager().await;

    // Create consistent episodes that should produce high-quality patterns
    for i in 0..20 {
        let mut episode = create_realistic_episode("feature", i, EpisodeOutcome::Success);

        // Consistent pattern: features always include tests and docs
        episode.entities_created = vec![
            format!("feature::new_feature_{}", i),
            format!("test::test_feature_{}", i),
            format!("docs::feature_{}_docs", i),
        ];
        episode.tools_used = vec![
            ToolUsage {
                tool_name: "code_generator".to_string(),
                usage_count: 1,
                total_duration_ms: 1000,
                parameters: HashMap::new(),
            },
            ToolUsage {
                tool_name: "test_generator".to_string(),
                usage_count: 1,
                total_duration_ms: 500,
                parameters: HashMap::new(),
            },
            ToolUsage {
                tool_name: "doc_generator".to_string(),
                usage_count: 1,
                total_duration_ms: 300,
                parameters: HashMap::new(),
            },
        ];
        episode.lessons_learned = vec![
            "Features should include tests".to_string(),
            "Features should include documentation".to_string(),
        ];

        manager.remember_episode(&episode).await.unwrap();
    }

    // Extract patterns
    let patterns = manager.episodic().extract_patterns(0.8).await.unwrap();

    println!("\n  Pattern Quality Metrics:");
    println!("    Patterns extracted: {}", patterns.len());

    for (idx, pattern) in patterns.iter().enumerate() {
        println!("\n    Pattern {}:", idx + 1);
        println!("      Name: {}", pattern.name);
        println!("      Type: {:?}", pattern.pattern_type);
        println!("      Context: {}", pattern.context);
        println!("      Description: {}", pattern.description);
    }

    // Quality check: Should extract meaningful patterns from consistent behavior
    assert!(patterns.len() > 0, "Should extract patterns from consistent episodes");

    // Store patterns and verify they can be applied
    for pattern in patterns {
        let id = manager.remember_pattern(&pattern).await.unwrap();

        // Simulate applying the pattern
        manager.procedural().record_success(id).await.unwrap();
    }

    let stats = manager.get_statistics().await.unwrap();
    println!("\n  Final Pattern Statistics:");
    println!("    Total patterns: {}", stats.procedural.total_patterns);
    println!("    Average success rate: {:.2}%", stats.procedural.average_success_rate * 100.0);
}

#[tokio::test]
async fn test_memory_consolidation_effectiveness() {
    println!("\n=== Testing Memory Consolidation Effectiveness ===");

    let manager = create_test_cognitive_manager().await;

    // Phase 1: Store many episodes
    println!("\n  Phase 1: Storing episodes");
    for i in 0..30 {
        let outcome = if i % 4 == 0 {
            EpisodeOutcome::Failure
        } else {
            EpisodeOutcome::Success
        };
        let episode = create_realistic_episode("bugfix", i, outcome);
        manager.remember_episode(&episode).await.unwrap();
    }

    let stats_before = manager.get_statistics().await.unwrap();
    println!("    Episodes before consolidation: {}", stats_before.episodic.total_episodes);
    println!("    Patterns before consolidation: {}", stats_before.procedural.total_patterns);

    // Phase 2: Consolidate
    println!("\n  Phase 2: Running consolidation");
    let start = Instant::now();
    let report = manager.consolidate().await.unwrap();
    let consolidation_time = start.elapsed();

    println!("    Consolidation time: {:?}", consolidation_time);
    println!("    Patterns extracted: {}", report.patterns_extracted);
    println!("    Memories decayed: {}", report.memories_decayed);
    println!("    Links created: {}", report.knowledge_links_created);

    // Phase 3: Verify consolidation effectiveness
    println!("\n  Phase 3: Verifying consolidation effectiveness");
    let stats_after = manager.get_statistics().await.unwrap();
    println!("    Episodes after consolidation: {}", stats_after.episodic.total_episodes);
    println!("    Patterns after consolidation: {}", stats_after.procedural.total_patterns);

    // Consolidation should either extract patterns or decay memories
    let consolidation_occurred =
        report.patterns_extracted > 0 ||
        report.memories_decayed > 0 ||
        report.knowledge_links_created > 0;

    assert!(consolidation_occurred, "Consolidation should have produced results");

    println!("\n  ✓ Consolidation effectiveness verified");
}
