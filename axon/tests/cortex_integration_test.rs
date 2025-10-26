//! Comprehensive integration tests for Axon-Cortex cognitive memory integration
//!
//! These tests verify that agents properly utilize Cortex's cognitive memory system:
//! - Episodic memory for storing agent experiences
//! - Semantic memory for code analysis and relationships
//! - Pattern learning from successful agent operations
//! - Knowledge graph for code understanding
//! - Working memory for active tasks
//! - Memory consolidation
//! - Collaborative memory sharing

use axon::agents::{
    developer::{CodeSpec, DeveloperAgent, RefactoringType},
    reviewer::ReviewerAgent,
    tester::{TestSpec, TestType, TesterAgent},
    Agent,
};
use axon::cortex_bridge::{
    CortexBridge, CortexConfig, EpisodeType, PatternType, SessionScope, WorkspaceId,
    WorkingMemoryItem,
};
use std::sync::Arc;

/// Helper to create a test CortexBridge (assumes Cortex is running)
async fn create_cortex_bridge() -> Arc<CortexBridge> {
    let config = CortexConfig {
        base_url: std::env::var("CORTEX_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
        ..Default::default()
    };

    Arc::new(
        CortexBridge::new(config)
            .await
            .expect("Failed to create CortexBridge"),
    )
}

#[tokio::test]
#[ignore] // Only run when Cortex is available
async fn test_developer_agent_with_episodic_memory() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    let developer = DeveloperAgent::with_cortex("dev-001".to_string(), cortex.clone());

    let spec = CodeSpec {
        description: "Create a user authentication function".to_string(),
        target_path: "src/auth.rs".to_string(),
        language: "rust".to_string(),
        workspace_id: workspace_id.clone(),
        feature_type: "function".to_string(),
    };

    // Generate code - this should store an episode
    let result = developer.generate_code(spec).await;
    assert!(result.is_ok(), "Code generation failed: {:?}", result.err());

    let generated = result.unwrap();
    assert!(!generated.content.is_empty());
    assert!(generated.metadata.generation_time_ms > 0);

    // Verify episode was stored by searching for it
    let episodes = cortex
        .search_episodes("authentication function", 10)
        .await
        .expect("Failed to search episodes");

    assert!(
        !episodes.is_empty(),
        "Episode should have been stored in episodic memory"
    );
}

#[tokio::test]
#[ignore]
async fn test_reviewer_agent_with_pattern_learning() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    let reviewer = ReviewerAgent::with_cortex("reviewer-001".to_string(), cortex.clone());

    // Create a session
    let session_id = cortex
        .create_session(
            "reviewer-001".to_string().into(),
            workspace_id.clone(),
            SessionScope {
                paths: vec!["src/".to_string()],
                read_only_paths: vec![],
            },
        )
        .await
        .expect("Failed to create session");

    // Write test file to session
    cortex
        .write_file(&session_id, "src/test.rs", "fn test() { unwrap() }")
        .await
        .expect("Failed to write file");

    // Review code - should detect patterns and store episode
    let review = reviewer
        .review_code(&workspace_id, &session_id, "src/test.rs")
        .await;

    assert!(review.is_ok(), "Review failed: {:?}", review.err());

    let report = review.unwrap();
    assert!(
        !report.issues.is_empty(),
        "Should detect unwrap() as an issue"
    );

    // Close session
    cortex
        .close_session(&session_id, &"reviewer-001".to_string().into())
        .await
        .expect("Failed to close session");

    // Verify episode was stored
    let episodes = cortex
        .search_episodes("code review", 10)
        .await
        .expect("Failed to search episodes");

    assert!(
        !episodes.is_empty(),
        "Review episode should have been stored"
    );
}

#[tokio::test]
#[ignore]
async fn test_tester_agent_with_semantic_memory() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    let tester = TesterAgent::with_cortex("tester-001".to_string(), cortex.clone());

    let spec = TestSpec {
        target_path: "src/lib.rs".to_string(),
        test_type: TestType::Unit,
        coverage_target: 0.8,
        workspace_id: workspace_id.clone(),
    };

    // Generate tests - should use semantic memory to understand code structure
    let result = tester.generate_tests(spec).await;
    assert!(result.is_ok(), "Test generation failed: {:?}", result.err());

    let test_suite = result.unwrap();
    assert!(test_suite.test_count > 0);
    assert!(!test_suite.content.is_empty());

    // Verify episode was stored
    let episodes = cortex
        .search_episodes("generate Unit tests", 10)
        .await
        .expect("Failed to search episodes");

    assert!(
        !episodes.is_empty(),
        "Test generation episode should have been stored"
    );
}

#[tokio::test]
#[ignore]
async fn test_working_memory_operations() {
    let cortex = create_cortex_bridge().await;
    let agent_id = "test-agent".to_string().into();
    let session_id = cortex
        .create_session(
            agent_id.clone(),
            WorkspaceId::from("test-workspace".to_string()),
            SessionScope {
                paths: vec!["src/".to_string()],
                read_only_paths: vec![],
            },
        )
        .await
        .expect("Failed to create session");

    // Add items to working memory
    let item = WorkingMemoryItem {
        id: "item-1".to_string(),
        item_type: "code_snippet".to_string(),
        content: "fn test() {}".to_string(),
        context: serde_json::json!({"file": "test.rs"}),
        priority: 0.8,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 1,
    };

    cortex
        .add_to_working_memory(&agent_id, &session_id, item)
        .await
        .expect("Failed to add to working memory");

    // Retrieve working memory
    let items = cortex
        .get_working_memory(&agent_id, &session_id)
        .await
        .expect("Failed to get working memory");

    assert!(!items.is_empty(), "Should have working memory items");

    // Clear working memory
    cortex
        .clear_working_memory(&agent_id, &session_id)
        .await
        .expect("Failed to clear working memory");

    // Close session
    cortex
        .close_session(&session_id, &agent_id)
        .await
        .expect("Failed to close session");
}

#[tokio::test]
#[ignore]
async fn test_memory_consolidation() {
    let cortex = create_cortex_bridge().await;
    let agent_id = "test-agent".to_string().into();
    let workspace_id = WorkspaceId::from("test-workspace".to_string());
    let session_id = cortex
        .create_session(
            agent_id.clone(),
            workspace_id.clone(),
            SessionScope {
                paths: vec!["src/".to_string()],
                read_only_paths: vec![],
            },
        )
        .await
        .expect("Failed to create session");

    // Add working memory items
    for i in 0..5 {
        let item = WorkingMemoryItem {
            id: format!("item-{}", i),
            item_type: "task".to_string(),
            content: format!("Task {}", i),
            context: serde_json::json!({"index": i}),
            priority: 0.5,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
        };

        cortex
            .add_to_working_memory(&agent_id, &session_id, item)
            .await
            .expect("Failed to add item");
    }

    // Trigger consolidation
    let report = cortex
        .consolidate_memory(&agent_id, &session_id)
        .await
        .expect("Failed to consolidate memory");

    assert!(
        report.items_consolidated > 0,
        "Should consolidate some items"
    );

    // Close session
    cortex
        .close_session(&session_id, &agent_id)
        .await
        .expect("Failed to close session");
}

#[tokio::test]
#[ignore]
async fn test_pattern_extraction() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    // Extract patterns from episodes
    let patterns = cortex
        .extract_patterns(&workspace_id, 2)
        .await
        .expect("Failed to extract patterns");

    // May be empty if no episodes exist, but should not error
    println!("Extracted {} patterns", patterns.len());

    // Search for patterns
    let search_results = cortex
        .search_patterns("refactoring", Some(PatternType::Refactor), 10)
        .await
        .expect("Failed to search patterns");

    println!("Found {} refactoring patterns", search_results.len());
}

#[tokio::test]
#[ignore]
async fn test_collaborative_memory_sharing() {
    let cortex = create_cortex_bridge().await;
    let agent1_id = "agent-1".to_string().into();
    let agent2_id = "agent-2".to_string().into();
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    // Create a developer agent that stores an episode
    let developer = DeveloperAgent::with_cortex("agent-1".to_string(), cortex.clone());

    let spec = CodeSpec {
        description: "Shared knowledge test".to_string(),
        target_path: "src/shared.rs".to_string(),
        language: "rust".to_string(),
        workspace_id: workspace_id.clone(),
        feature_type: "function".to_string(),
    };

    let result = developer.generate_code(spec).await;
    assert!(result.is_ok());

    // Find episodes from agent1
    let episodes = cortex
        .search_episodes("shared knowledge", 10)
        .await
        .expect("Failed to search episodes");

    if !episodes.is_empty() {
        let episode_id = episodes[0].id.clone().into();

        // Share with agent2
        cortex
            .share_episode(&episode_id, vec![agent2_id.clone()])
            .await
            .expect("Failed to share episode");

        // Agent2 retrieves shared episodes
        let shared = cortex
            .get_shared_episodes(&agent2_id, 10)
            .await
            .expect("Failed to get shared episodes");

        assert!(
            !shared.is_empty(),
            "Agent2 should see shared episodes from Agent1"
        );
    }

    // Get collaborative insights
    let insights = cortex
        .get_collaborative_insights(&workspace_id)
        .await
        .expect("Failed to get collaborative insights");

    println!("Found {} collaborative insights", insights.len());
}

#[tokio::test]
#[ignore]
async fn test_knowledge_graph_queries() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    // Query knowledge graph
    let query = r#"
        MATCH (u:CodeUnit)-[:CALLS]->(called:CodeUnit)
        WHERE u.name CONTAINS 'test'
        RETURN u.name AS caller, called.name AS callee
        LIMIT 10
    "#;

    let result = cortex
        .query_graph(query, serde_json::json!({}))
        .await
        .expect("Failed to query knowledge graph");

    println!(
        "Graph query returned {} nodes and {} edges",
        result.nodes.len(),
        result.edges.len()
    );
}

#[tokio::test]
#[ignore]
async fn test_bidirectional_sync() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());
    let session_id = cortex
        .create_session(
            "sync-agent".to_string().into(),
            workspace_id.clone(),
            SessionScope {
                paths: vec!["src/".to_string()],
                read_only_paths: vec![],
            },
        )
        .await
        .expect("Failed to create session");

    // Write code with automatic analysis
    let code = r#"
        pub fn example_function(x: i32) -> i32 {
            x * 2
        }
    "#;

    let analysis = cortex
        .write_code_with_analysis(&session_id, &workspace_id, "src/example.rs", code)
        .await
        .expect("Failed to write code with analysis");

    assert!(
        analysis.units_extracted > 0,
        "Should extract code units from written code"
    );

    // Sync session to semantic memory
    let sync_report = cortex
        .sync_session_to_memory(&session_id, &workspace_id)
        .await
        .expect("Failed to sync session");

    assert!(sync_report.files_synced > 0, "Should sync files");

    // Close session
    cortex
        .close_session(&session_id, &"sync-agent".to_string().into())
        .await
        .expect("Failed to close session");
}

#[tokio::test]
#[ignore]
async fn test_agent_learning_from_past_experiences() {
    let cortex = create_cortex_bridge().await;
    let workspace_id = WorkspaceId::from("test-workspace".to_string());

    // Create two developer agents
    let developer1 = DeveloperAgent::with_cortex("dev-1".to_string(), cortex.clone());
    let developer2 = DeveloperAgent::with_cortex("dev-2".to_string(), cortex.clone());

    // Developer1 creates code (stores episode)
    let spec1 = CodeSpec {
        description: "Authentication middleware".to_string(),
        target_path: "src/middleware/auth.rs".to_string(),
        language: "rust".to_string(),
        workspace_id: workspace_id.clone(),
        feature_type: "middleware".to_string(),
    };

    developer1
        .generate_code(spec1)
        .await
        .expect("Failed to generate code");

    // Developer2 creates similar code and should benefit from dev1's experience
    let spec2 = CodeSpec {
        description: "Authorization middleware".to_string(),
        target_path: "src/middleware/authz.rs".to_string(),
        language: "rust".to_string(),
        workspace_id: workspace_id.clone(),
        feature_type: "middleware".to_string(),
    };

    let result2 = developer2
        .generate_code(spec2)
        .await
        .expect("Failed to generate code");

    // Developer2 should have consulted episodes
    assert!(
        result2.metadata.episodes_consulted > 0,
        "Should learn from past episodes"
    );
}
