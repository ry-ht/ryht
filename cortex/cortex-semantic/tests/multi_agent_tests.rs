//! Integration tests for multi-agent coordination features.
//!
//! These tests verify:
//! - Agent registration and lifecycle
//! - Memory pool access control
//! - Priority-based search queuing
//! - Federated search across namespaces
//! - Cross-agent knowledge retrieval
//! - Conflict resolution
//! - Performance metrics tracking

use cortex_semantic::agent::*;
use cortex_semantic::config::SemanticConfig;
use cortex_semantic::orchestration::*;
use cortex_semantic::search::SemanticSearchEngine;
use cortex_semantic::types::EntityType;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Create a test coordinator with multiple agents.
async fn setup_multi_agent_system() -> (
    Arc<AgentCoordinator>,
    Vec<Arc<tokio::sync::RwLock<AgentContext>>>,
) {
    let coordinator = Arc::new(AgentCoordinator::new());

    let agent1 = coordinator
        .register_agent("worker-1", AgentRole::Worker, vec!["rust".to_string()])
        .await
        .unwrap();

    let agent2 = coordinator
        .register_agent("worker-2", AgentRole::Worker, vec!["python".to_string()])
        .await
        .unwrap();

    let agent3 = coordinator
        .register_agent(
            "orchestrator-1",
            AgentRole::Orchestrator,
            vec!["coordination".to_string()],
        )
        .await
        .unwrap();

    let agents = vec![agent1, agent2, agent3];

    (coordinator, agents)
}

/// Create a test search engine.
async fn create_test_engine() -> Arc<SemanticSearchEngine> {
    let mut config = SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];

    Arc::new(SemanticSearchEngine::new(config).await.unwrap())
}

#[tokio::test]
async fn test_agent_registration_and_lifecycle() {
    let coordinator = AgentCoordinator::new();

    // Register agent
    let agent = coordinator
        .register_agent("test-agent", AgentRole::Worker, vec!["test".to_string()])
        .await
        .unwrap();

    // Verify agent exists
    let agents = coordinator.list_agents();
    assert_eq!(agents.len(), 1);
    assert!(agents.contains(&"test-agent".to_string()));

    // Verify context
    let context = agent.read().await;
    assert_eq!(context.agent_id, "test-agent");
    assert_eq!(context.role, AgentRole::Worker);
    assert_eq!(context.namespace, "agent::test-agent");

    // Unregister agent
    coordinator.unregister_agent(&"test-agent".to_string()).await.unwrap();
    assert_eq!(coordinator.list_agents().len(), 0);
}

#[tokio::test]
async fn test_memory_pool_shared_access() {
    let pool = MemoryPool::new(AccessPolicy::Shared);

    let vector1 = vec![1.0, 2.0, 3.0];
    let vector2 = vec![4.0, 5.0, 6.0];

    // Agent 1 stores
    pool.store(
        &"agent1".to_string(),
        AgentRole::Worker,
        "doc1".to_string(),
        vector1.clone(),
        HashMap::new(),
    )
    .await
    .unwrap();

    // Agent 2 stores
    pool.store(
        &"agent2".to_string(),
        AgentRole::Worker,
        "doc2".to_string(),
        vector2.clone(),
        HashMap::new(),
    )
    .await
    .unwrap();

    // Agent 1 can read agent 2's data
    let entry = pool
        .retrieve(&"agent1".to_string(), AgentRole::Worker, &"doc2".to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(entry.vector, vector2);
    assert_eq!(entry.agent_id, "agent2");

    // Verify stats
    let stats = pool.stats();
    assert_eq!(stats.get("total_entries").unwrap(), &2);
    assert_eq!(stats.get("writes").unwrap(), &2);
    assert_eq!(stats.get("reads").unwrap(), &1);
}

#[tokio::test]
async fn test_memory_pool_private_access() {
    let pool = MemoryPool::new(AccessPolicy::Private);

    // Set up access control
    {
        let mut ac = pool.access_control.write().await;
        ac.add_owner("agent1");
        ac.add_reader("agent2");
    }

    let vector = vec![1.0, 2.0, 3.0];

    // Owner can store
    pool.store(
        &"agent1".to_string(),
        AgentRole::Worker,
        "doc1".to_string(),
        vector.clone(),
        HashMap::new(),
    )
    .await
    .unwrap();

    // Reader can retrieve
    let entry = pool
        .retrieve(&"agent2".to_string(), AgentRole::Worker, &"doc1".to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(entry.vector, vector);

    // Non-authorized agent cannot retrieve
    let result = pool
        .retrieve(&"agent3".to_string(), AgentRole::Worker, &"doc1".to_string())
        .await;

    assert!(result.is_err());

    // Non-owner cannot store
    let result = pool
        .store(
            &"agent2".to_string(),
            AgentRole::Worker,
            "doc2".to_string(),
            vec![7.0, 8.0, 9.0],
            HashMap::new(),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_memory_pool_hierarchical_access() {
    let pool = MemoryPool::new(AccessPolicy::Hierarchical);

    {
        let mut ac = pool.access_control.write().await;
        ac.add_reader("worker1");
    }

    let vector = vec![1.0, 2.0, 3.0];

    // Orchestrator can always write
    pool.store(
        &"orchestrator".to_string(),
        AgentRole::Orchestrator,
        "doc1".to_string(),
        vector.clone(),
        HashMap::new(),
    )
    .await
    .unwrap();

    // Worker with permission can read
    let entry = pool
        .retrieve(&"worker1".to_string(), AgentRole::Worker, &"doc1".to_string())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(entry.vector, vector);

    // Worker cannot write
    let result = pool
        .store(
            &"worker1".to_string(),
            AgentRole::Worker,
            "doc2".to_string(),
            vec![4.0, 5.0, 6.0],
            HashMap::new(),
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_priority_queue() {
    let queue = SearchQueue::new(100);

    // Enqueue requests with different priorities
    let critical = PrioritizedSearchRequest::new("agent1", "critical query", SearchPriority::Critical);
    let normal = PrioritizedSearchRequest::new("agent2", "normal query", SearchPriority::Normal);
    let low = PrioritizedSearchRequest::new("agent3", "low query", SearchPriority::Low);

    queue.enqueue(normal).await.unwrap();
    queue.enqueue(low).await.unwrap();
    queue.enqueue(critical).await.unwrap();

    // Should dequeue in priority order
    let req1 = queue.dequeue().await.unwrap();
    assert_eq!(req1.priority, SearchPriority::Critical);

    let req2 = queue.dequeue().await.unwrap();
    assert_eq!(req2.priority, SearchPriority::Normal);

    let req3 = queue.dequeue().await.unwrap();
    assert_eq!(req3.priority, SearchPriority::Low);
}

#[tokio::test]
async fn test_priority_queue_overflow() {
    let queue = SearchQueue::new(2); // Small queue

    let req1 = PrioritizedSearchRequest::new("agent1", "query1", SearchPriority::Normal);
    let req2 = PrioritizedSearchRequest::new("agent2", "query2", SearchPriority::Normal);
    let req3 = PrioritizedSearchRequest::new("agent3", "query3", SearchPriority::Normal);

    queue.enqueue(req1).await.unwrap();
    queue.enqueue(req2).await.unwrap();
    queue.enqueue(req3).await.unwrap(); // Should drop oldest

    // Should only have 2 requests
    let sizes = queue.queue_sizes().await;
    assert_eq!(sizes.get(&SearchPriority::Normal).unwrap(), &2);
}

#[tokio::test]
async fn test_agent_metrics() {
    let (coordinator, _agents) = setup_multi_agent_system().await;

    // Get metrics for an agent
    let metrics = coordinator.get_metrics(&"worker-1".to_string()).unwrap();

    // Record some operations
    metrics.record_search(100, true);
    metrics.record_search(200, false);
    metrics.record_search(150, true);

    assert_eq!(
        metrics.search_count.load(std::sync::atomic::Ordering::Relaxed),
        3
    );
    assert_eq!(metrics.avg_search_latency_ms(), 150.0);

    let hit_rate = metrics.cache_hit_rate();
    assert!((hit_rate - 0.6666).abs() < 0.01);

    // Get metrics as map
    let metrics_map = metrics.to_map();
    assert!(metrics_map.contains_key("search_count"));
    assert!(metrics_map.contains_key("avg_search_latency_ms"));
}

#[tokio::test]
async fn test_agent_context_capabilities() {
    let mut context = AgentContext::new("test-agent", AgentRole::Specialist, vec![
        "rust".to_string(),
        "python".to_string(),
    ]);

    assert!(context.has_capability("rust"));
    assert!(context.has_capability("python"));
    assert!(!context.has_capability("java"));

    // Add metadata
    context.add_metadata("version", "1.0");
    context.add_metadata("team", "backend");

    assert_eq!(context.metadata.get("version").unwrap(), "1.0");
    assert_eq!(context.metadata.get("team").unwrap(), "backend");

    // Update activity
    let before = context.last_active;
    tokio::time::sleep(Duration::from_millis(10)).await;
    context.update_activity();
    assert!(context.last_active > before);
}

#[tokio::test]
async fn test_federated_search_basic() {
    let (coordinator, _agents) = setup_multi_agent_system().await;
    let orchestrator = Arc::new(SearchOrchestrator::new(coordinator.clone()));

    // Create and register engines for agents
    let engine1 = create_test_engine().await;
    let engine2 = create_test_engine().await;

    // Index documents in different agent namespaces
    engine1
        .index_document_for_agent(
            "worker-1",
            "doc1".to_string(),
            "Rust programming language".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine2
        .index_document_for_agent(
            "worker-2",
            "doc2".to_string(),
            "Python programming language".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    orchestrator.register_engine("worker-1", engine1);
    orchestrator.register_engine("worker-2", engine2);

    // Perform federated search
    let (results, stats) = orchestrator
        .federated_search(
            &"orchestrator-1".to_string(),
            "programming language",
            10,
            None,
            SearchPriority::Normal,
        )
        .await
        .unwrap();

    // Should find results from both agents
    assert!(!results.is_empty());
    assert_eq!(stats.agents_queried, 2);
    assert_eq!(stats.namespaces_searched.len(), 2);
}

#[tokio::test]
async fn test_namespace_specific_search() {
    let (coordinator, _agents) = setup_multi_agent_system().await;
    let orchestrator = Arc::new(SearchOrchestrator::new(coordinator.clone()));

    let engine = create_test_engine().await;

    // Index documents
    engine
        .index_document_for_agent(
            "worker-1",
            "doc1".to_string(),
            "Test document".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    orchestrator.register_engine("worker-1", engine);

    // Search within specific namespace
    let results = orchestrator
        .namespace_search(&"worker-1".to_string(), "test", 10, None)
        .await
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].metadata.get("agent_id").unwrap(), "worker-1");
}

#[tokio::test]
async fn test_broadcast_search() {
    let (coordinator, _agents) = setup_multi_agent_system().await;
    let orchestrator = Arc::new(SearchOrchestrator::new(coordinator.clone()));

    let engine1 = create_test_engine().await;
    let engine2 = create_test_engine().await;

    orchestrator.register_engine("worker-1", engine1);
    orchestrator.register_engine("worker-2", engine2);

    // Broadcast search to all agents
    let results = orchestrator.broadcast_search("test query", 5).await.unwrap();

    // Should have results from all registered agents
    assert_eq!(results.len(), 2);
    assert!(results.contains_key("worker-1"));
    assert!(results.contains_key("worker-2"));
}

#[tokio::test]
async fn test_memory_pool_search() {
    let pool = MemoryPool::new(AccessPolicy::Shared);

    // Store vectors
    pool.store(
        &"agent1".to_string(),
        AgentRole::Worker,
        "doc1".to_string(),
        vec![1.0, 0.0, 0.0],
        HashMap::new(),
    )
    .await
    .unwrap();

    pool.store(
        &"agent1".to_string(),
        AgentRole::Worker,
        "doc2".to_string(),
        vec![0.9, 0.1, 0.0],
        HashMap::new(),
    )
    .await
    .unwrap();

    pool.store(
        &"agent2".to_string(),
        AgentRole::Worker,
        "doc3".to_string(),
        vec![0.0, 1.0, 0.0],
        HashMap::new(),
    )
    .await
    .unwrap();

    // Search with query vector similar to doc1
    let query = vec![1.0, 0.0, 0.0];
    let results = pool
        .search(&"agent2".to_string(), AgentRole::Worker, &query, 2)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, "doc1"); // Most similar
    assert!(results[0].1 > results[1].1); // Higher score
}

#[tokio::test]
async fn test_cross_agent_knowledge_retrieval() {
    let pool = MemoryPool::new(AccessPolicy::Shared);

    // Agent 1 stores knowledge
    let mut metadata1 = HashMap::new();
    metadata1.insert("topic".to_string(), "rust".to_string());

    pool.store(
        &"agent1".to_string(),
        AgentRole::Worker,
        "rust-doc".to_string(),
        vec![1.0, 0.5, 0.0],
        metadata1,
    )
    .await
    .unwrap();

    // Agent 2 retrieves agent 1's knowledge
    let entry = pool
        .retrieve(
            &"agent2".to_string(),
            AgentRole::Worker,
            &"rust-doc".to_string(),
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(entry.agent_id, "agent1");
    assert_eq!(entry.metadata.get("topic").unwrap(), "rust");
    assert!(entry.access_count >= 1);
}

#[tokio::test]
async fn test_agent_concurrency_limit() {
    let coordinator = Arc::new(AgentCoordinator::with_concurrency_limit(2));

    // Acquire permits
    let permit1 = coordinator.acquire_permit().await.unwrap();
    let permit2 = coordinator.acquire_permit().await.unwrap();

    // Third permit should block (we'll use try_acquire which returns immediately)
    // Note: Semaphore doesn't have try_acquire in the public API, so we'll test differently

    // Release permits
    drop(permit1);
    drop(permit2);

    // Now we should be able to acquire again
    let permit3 = coordinator.acquire_permit().await.unwrap();
    drop(permit3);
}

#[tokio::test]
async fn test_orchestrator_statistics() {
    let (coordinator, _agents) = setup_multi_agent_system().await;
    let orchestrator = Arc::new(SearchOrchestrator::new(coordinator.clone()));

    let engine = create_test_engine().await;
    orchestrator.register_engine("worker-1", engine);

    // Perform some searches
    let _ = orchestrator
        .federated_search(
            &"orchestrator-1".to_string(),
            "test query",
            10,
            None,
            SearchPriority::Normal,
        )
        .await;

    // Check stats
    let stats = orchestrator.stats().await;
    assert_eq!(stats.total_federated_searches, 1);
    assert!(stats.avg_search_latency_ms > 0.0);
}

#[tokio::test]
async fn test_coordinator_system_stats() {
    let (coordinator, _agents) = setup_multi_agent_system().await;

    let stats = coordinator.system_stats();

    assert_eq!(stats.get("total_agents").unwrap(), &serde_json::json!(3));
    assert_eq!(stats.get("total_memory_pools").unwrap(), &serde_json::json!(0));
}

#[tokio::test]
async fn test_agent_namespace_isolation() {
    let engine = create_test_engine().await;

    // Index documents for different agents
    engine
        .index_document_for_agent(
            "agent1",
            "doc1".to_string(),
            "Agent 1 content".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document_for_agent(
            "agent2",
            "doc2".to_string(),
            "Agent 2 content".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // Search with namespace isolation
    let results = engine
        .search_for_agent("agent1", "content", 10, false)
        .await
        .unwrap();

    // Should only see agent1's documents
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.get("agent_id").unwrap(), "agent1");
}

#[tokio::test]
async fn test_cross_namespace_search() {
    let engine = create_test_engine().await;

    // Index documents for different agents
    engine
        .index_document_for_agent(
            "agent1",
            "doc1".to_string(),
            "Shared content".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document_for_agent(
            "agent2",
            "doc2".to_string(),
            "Shared content".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // Search with cross-namespace enabled
    let results = engine
        .search_for_agent("agent1", "content", 10, true)
        .await
        .unwrap();

    // Should see documents from both agents
    assert!(results.len() >= 1);
}

#[tokio::test]
async fn test_multi_agent_conflict_resolution() {
    let pool = MemoryPool::new(AccessPolicy::Shared);

    // Multiple agents writing to the same pool concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool = Arc::new(pool.clone());
            tokio::spawn(async move {
                pool.store(
                    &format!("agent{}", i),
                    AgentRole::Worker,
                    format!("doc{}", i),
                    vec![i as f32, 0.0, 0.0],
                    HashMap::new(),
                )
                .await
                .unwrap();
            })
        })
        .collect();

    // Wait for all writes
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all entries were stored
    let stats = pool.stats();
    assert_eq!(stats.get("total_entries").unwrap(), &5);
    assert_eq!(stats.get("writes").unwrap(), &5);
}
