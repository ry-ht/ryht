//! Multi-agent search orchestration.
//!
//! This module implements federated search across multiple agent namespaces with:
//! - Concurrent search across multiple agents
//! - Result aggregation and deduplication
//! - Cross-agent context passing
//! - Load balancing and failover
//!
//! Based on 2025 research in distributed search systems and multi-agent coordination.

use crate::agent::{AgentContext, AgentCoordinator, AgentId, Namespace, SearchPriority};
use crate::error::{Result, SemanticError};
use crate::search::{SearchFilter, SearchResult, SemanticSearchEngine};
use crate::types::{AgentSearchResult, DocumentId, FederatedSearchConfig, MultiAgentSearchStats};
use dashmap::DashMap;
use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};

/// Search orchestrator for multi-agent coordination.
///
/// The orchestrator manages:
/// - Federated search across agent namespaces
/// - Result aggregation from multiple agents
/// - Deduplication of cross-agent results
/// - Load balancing and concurrency control
pub struct SearchOrchestrator {
    /// Reference to agent coordinator
    coordinator: Arc<AgentCoordinator>,
    /// Per-agent search engines
    engines: Arc<DashMap<AgentId, Arc<SemanticSearchEngine>>>,
    /// Federated search configuration
    config: FederatedSearchConfig,
    /// Search statistics
    stats: Arc<RwLock<SearchOrchestratorStats>>,
    /// Rate limiter for concurrent searches (prevents DoS)
    rate_limiter: Arc<Semaphore>,
}

/// Orchestrator statistics.
#[derive(Debug, Default, Clone)]
pub struct SearchOrchestratorStats {
    pub total_federated_searches: u64,
    pub total_namespaces_searched: u64,
    pub total_results_deduplicated: u64,
    pub avg_search_latency_ms: f64,
}

impl SearchOrchestrator {
    /// Create a new search orchestrator.
    pub fn new(coordinator: Arc<AgentCoordinator>) -> Self {
        Self::with_config(coordinator, FederatedSearchConfig::default())
    }

    /// Create orchestrator with custom configuration.
    pub fn with_config(
        coordinator: Arc<AgentCoordinator>,
        config: FederatedSearchConfig,
    ) -> Self {
        info!("Initializing SearchOrchestrator with rate limiting");

        // Default to 10 concurrent searches to prevent DoS
        let max_concurrent_searches = config.max_concurrent_searches.unwrap_or(10);

        Self {
            coordinator,
            engines: Arc::new(DashMap::new()),
            config,
            stats: Arc::new(RwLock::new(SearchOrchestratorStats::default())),
            rate_limiter: Arc::new(Semaphore::new(max_concurrent_searches)),
        }
    }

    /// Register a search engine for an agent.
    pub fn register_engine(&self, agent_id: impl Into<AgentId>, engine: Arc<SemanticSearchEngine>) {
        let agent_id = agent_id.into();
        debug!("Registering search engine for agent: {}", agent_id);
        self.engines.insert(agent_id, engine);
    }

    /// Unregister an agent's search engine.
    pub fn unregister_engine(&self, agent_id: &AgentId) {
        debug!("Unregistering search engine for agent: {}", agent_id);
        self.engines.remove(agent_id);
    }

    /// Perform federated search across multiple agent namespaces.
    ///
    /// This searches across all registered agents (or a subset) and aggregates results.
    pub async fn federated_search(
        &self,
        requesting_agent: &AgentId,
        query: &str,
        limit: usize,
        namespaces: Option<Vec<Namespace>>,
        priority: SearchPriority,
    ) -> Result<(Vec<AgentSearchResult>, MultiAgentSearchStats)> {
        let start = Instant::now();

        info!(
            "Starting federated search for agent {} with priority {:?}",
            requesting_agent, priority
        );

        // Acquire concurrency permit
        let _permit = self.coordinator.acquire_permit().await?;

        // Determine which namespaces to search
        let target_namespaces = self.determine_namespaces(namespaces).await?;

        if target_namespaces.is_empty() {
            return Ok((vec![], MultiAgentSearchStats::default()));
        }

        // Track statistics
        let mut stats = MultiAgentSearchStats {
            agents_queried: target_namespaces.len(),
            namespaces_searched: target_namespaces.clone(),
            results_per_agent: HashMap::new(),
            total_search_time_ms: 0,
            deduplicated_count: 0,
            communication_overhead_ms: 0,
        };

        // Perform concurrent searches across namespaces with rate limiting
        let search_futures: Vec<_> = target_namespaces
            .iter()
            .filter_map(|namespace| {
                // Extract agent_id from namespace (format: "agent::{agent_id}")
                let agent_id = namespace.strip_prefix("agent::").unwrap_or(namespace);

                self.engines.get(agent_id).map(|engine| {
                    let engine = engine.clone();
                    let query = query.to_string();
                    let namespace = namespace.clone();
                    let agent_id = agent_id.to_string();
                    let rate_limiter = self.rate_limiter.clone();

                    async move {
                        // Acquire permit from rate limiter (blocks if limit reached)
                        let _permit = rate_limiter.acquire().await.expect("Semaphore closed unexpectedly");

                        let search_start = Instant::now();

                        let results = engine
                            .search(&query, limit)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Search failed for namespace {}: {}", namespace, e);
                                vec![]
                            });

                        let search_time = search_start.elapsed().as_millis() as u64;

                        (agent_id, namespace, results, search_time)
                    }
                })
            })
            .collect();

        // Execute all searches concurrently
        let search_results = join_all(search_futures).await;

        // Aggregate results
        let mut all_results: Vec<AgentSearchResult> = Vec::new();

        for (agent_id, namespace, results, search_time) in search_results {
            stats.total_search_time_ms += search_time;
            stats.results_per_agent.insert(agent_id.clone(), results.len());

            for result in results {
                all_results.push(AgentSearchResult {
                    id: result.id,
                    entity_type: result.entity_type,
                    content: result.content,
                    score: result.score,
                    metadata: result.metadata,
                    explanation: result.explanation,
                    indexed_by: Some(agent_id.clone()),
                    namespace: Some(namespace.clone()),
                    cross_agent_score: None,
                    embedding: result.embedding,  // Pass through embedding for deduplication
                });
            }
        }

        // Deduplicate results if enabled
        if self.config.deduplicate_results {
            let before_dedup = all_results.len();
            all_results = self.deduplicate_results(all_results).await;
            stats.deduplicated_count = before_dedup - all_results.len();
        }

        // Rerank results with cross-agent awareness
        if self.config.aggregate_results {
            self.rerank_cross_agent(&mut all_results);
        }

        // Sort by score and limit
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(limit);

        // Update orchestrator stats
        let elapsed = start.elapsed().as_millis() as u64;
        stats.communication_overhead_ms = elapsed.saturating_sub(stats.total_search_time_ms);

        {
            let mut orchestrator_stats = self.stats.write().await;
            orchestrator_stats.total_federated_searches += 1;
            orchestrator_stats.total_namespaces_searched += target_namespaces.len() as u64;
            orchestrator_stats.total_results_deduplicated += stats.deduplicated_count as u64;

            // Update rolling average
            let count = orchestrator_stats.total_federated_searches as f64;
            orchestrator_stats.avg_search_latency_ms =
                (orchestrator_stats.avg_search_latency_ms * (count - 1.0) + elapsed as f64) / count;
        }

        // Update requesting agent metrics
        if let Some(metrics) = self.coordinator.get_metrics(requesting_agent) {
            metrics.cross_agent_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        info!(
            "Federated search completed: {} results from {} namespaces in {}ms",
            all_results.len(),
            target_namespaces.len(),
            elapsed
        );

        Ok((all_results, stats))
    }

    /// Search within a specific agent's namespace only.
    pub async fn namespace_search(
        &self,
        agent_id: &AgentId,
        query: &str,
        limit: usize,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>> {
        let engine = self
            .engines
            .get(agent_id)
            .ok_or_else(|| SemanticError::Search(format!("No engine found for agent {}", agent_id)))?;

        let results = if let Some(filter) = filter {
            engine.search_with_filter(query, limit, filter).await?
        } else {
            engine.search(query, limit).await?
        };

        Ok(results)
    }

    /// Broadcast search to all registered agents.
    pub async fn broadcast_search(
        &self,
        query: &str,
        limit_per_agent: usize,
    ) -> Result<HashMap<AgentId, Vec<SearchResult>>> {
        let mut results = HashMap::new();

        let search_futures: Vec<_> = self
            .engines
            .iter()
            .map(|entry| {
                let agent_id = entry.key().clone();
                let engine = entry.value().clone();
                let query = query.to_string();

                async move {
                    let search_results = engine
                        .search(&query, limit_per_agent)
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Broadcast search failed for agent {}: {}", agent_id, e);
                            vec![]
                        });

                    (agent_id, search_results)
                }
            })
            .collect();

        let all_results = join_all(search_futures).await;

        for (agent_id, agent_results) in all_results {
            results.insert(agent_id, agent_results);
        }

        Ok(results)
    }

    /// Get orchestrator statistics.
    pub async fn stats(&self) -> SearchOrchestratorStats {
        self.stats.read().await.clone()
    }

    /// Determine which namespaces to search.
    async fn determine_namespaces(
        &self,
        requested: Option<Vec<Namespace>>,
    ) -> Result<Vec<Namespace>> {
        let namespaces = if let Some(requested_namespaces) = requested {
            // Use specified namespaces
            requested_namespaces
                .into_iter()
                .take(self.config.max_namespaces)
                .collect()
        } else {
            // Search all registered agent namespaces
            let agents = self.coordinator.list_agents();

            agents
                .into_iter()
                .take(self.config.max_namespaces)
                .map(|agent_id| format!("agent::{}", agent_id))
                .collect()
        };

        Ok(namespaces)
    }

    /// Deduplicate results based on embedding similarity (semantic deduplication).
    async fn deduplicate_results(&self, results: Vec<AgentSearchResult>) -> Vec<AgentSearchResult> {
        if results.len() <= 1 {
            return results;
        }

        let mut deduplicated: Vec<AgentSearchResult> = Vec::new();
        let mut seen_ids = HashSet::new();

        for result in results {
            // Skip exact duplicate IDs
            if seen_ids.contains(&result.id) {
                continue;
            }

            // Check semantic similarity with existing results using embeddings
            let mut is_duplicate = false;

            if let Some(ref result_embedding) = result.embedding {
                for existing in &deduplicated {
                    // Use embedding-based semantic similarity when available
                    let similarity = if let Some(ref existing_embedding) = existing.embedding {
                        crate::types::cosine_similarity(
                            result_embedding.as_slice(),
                            existing_embedding.as_slice()
                        )
                    } else {
                        // Fallback to content comparison if no embedding
                        self.calculate_content_similarity(&result.content, &existing.content)
                    };

                    if similarity >= self.config.dedup_threshold {
                        is_duplicate = true;
                        debug!("Deduplicating result {} (similarity: {:.3})", result.id, similarity);
                        break;
                    }
                }
            } else {
                // If no embedding, check with basic content similarity
                for existing in &deduplicated {
                    let similarity = self.calculate_content_similarity(&result.content, &existing.content);

                    if similarity >= self.config.dedup_threshold {
                        is_duplicate = true;
                        break;
                    }
                }
            }

            if !is_duplicate {
                seen_ids.insert(result.id.clone());
                deduplicated.push(result);
            }
        }

        deduplicated
    }

    /// Calculate content similarity as fallback when embeddings not available.
    /// Uses normalized character overlap for better accuracy than Jaccard.
    fn calculate_content_similarity(&self, text1: &str, text2: &str) -> f32 {
        if text1 == text2 {
            return 1.0;
        }

        // Use normalized character overlap as basic similarity metric
        let max_len = text1.len().max(text2.len());
        if max_len == 0 {
            return 1.0;
        }

        // Calculate common character count
        let common_chars = text1.chars()
            .filter(|c| text2.contains(*c))
            .count();

        common_chars as f32 / max_len as f32
    }

    /// Rerank results with cross-agent awareness.
    fn rerank_cross_agent(&self, results: &mut [AgentSearchResult]) {
        for result in results.iter_mut() {
            // Adjust score based on cross-namespace weight
            if result.namespace.is_some() {
                let adjusted_score = result.score * self.config.cross_namespace_weight;
                result.cross_agent_score = Some(result.score);
                result.score = adjusted_score;
            }
        }
    }
}

/// Result aggregation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// Take top-K from each agent and merge
    TopK,
    /// Round-robin from each agent
    RoundRobin,
    /// Score-based weighted merge
    WeightedMerge,
    /// Diversity-based selection
    Diverse,
}

/// Result deduplication strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeduplicationStrategy {
    /// Remove exact ID matches
    ExactId,
    /// Content-based similarity
    ContentSimilarity,
    /// Embedding-based similarity
    EmbeddingSimilarity,
    /// No deduplication
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentRole;
    use crate::config::SemanticConfig;

    async fn create_test_coordinator() -> Arc<AgentCoordinator> {
        let coordinator = Arc::new(AgentCoordinator::new());

        // Register test agents
        coordinator
            .register_agent("agent1", AgentRole::Worker, vec!["rust".to_string()])
            .await
            .unwrap();

        coordinator
            .register_agent("agent2", AgentRole::Worker, vec!["python".to_string()])
            .await
            .unwrap();

        coordinator
    }

    async fn create_test_engine() -> Arc<SemanticSearchEngine> {
        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();

        Arc::new(SemanticSearchEngine::new(config).await.unwrap())
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let stats = orchestrator.stats().await;
        assert_eq!(stats.total_federated_searches, 0);
    }

    #[tokio::test]
    async fn test_register_engine() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let engine = create_test_engine().await;
        orchestrator.register_engine("agent1", engine);

        assert!(orchestrator.engines.contains_key("agent1"));
    }

    #[tokio::test]
    async fn test_namespace_search() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let engine = create_test_engine().await;

        // Index some documents
        engine
            .index_document(
                "doc1".to_string(),
                "test content".to_string(),
                crate::types::EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        orchestrator.register_engine("agent1", engine);

        // Search within namespace
        let results = orchestrator
            .namespace_search(&"agent1".to_string(), "test", 10, None)
            .await
            .unwrap();

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_text_similarity() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let sim1 = orchestrator.calculate_content_similarity("hello world", "hello world");
        assert_eq!(sim1, 1.0);

        let sim2 = orchestrator.calculate_content_similarity("hello world", "world hello");
        assert!(sim2 > 0.5);  // Content similarity is less strict than word overlap

        let sim3 = orchestrator.calculate_content_similarity("hello world", "foo bar");
        assert!(sim3 < 0.3);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let results = vec![
            AgentSearchResult {
                id: "doc1".to_string(),
                entity_type: crate::types::EntityType::Document,
                content: "hello world".to_string(),
                score: 0.9,
                metadata: HashMap::new(),
                explanation: None,
                indexed_by: Some("agent1".to_string()),
                namespace: Some("agent::agent1".to_string()),
                cross_agent_score: None,
                embedding: Some(vec![0.1, 0.2, 0.3]),  // Test embedding
            },
            AgentSearchResult {
                id: "doc2".to_string(),
                entity_type: crate::types::EntityType::Document,
                content: "hello world".to_string(), // Duplicate
                score: 0.8,
                metadata: HashMap::new(),
                explanation: None,
                indexed_by: Some("agent2".to_string()),
                namespace: Some("agent::agent2".to_string()),
                cross_agent_score: None,
                embedding: Some(vec![0.1, 0.2, 0.3]),  // Same embedding (duplicate)
            },
            AgentSearchResult {
                id: "doc3".to_string(),
                entity_type: crate::types::EntityType::Document,
                content: "different content".to_string(),
                score: 0.7,
                metadata: HashMap::new(),
                explanation: None,
                indexed_by: Some("agent1".to_string()),
                namespace: Some("agent::agent1".to_string()),
                cross_agent_score: None,
                embedding: Some(vec![0.9, 0.8, 0.7]),  // Different embedding
            },
        ];

        let deduplicated = orchestrator.deduplicate_results(results).await;

        // Should remove one duplicate
        assert_eq!(deduplicated.len(), 2);
    }

    #[tokio::test]
    async fn test_broadcast_search() {
        let coordinator = create_test_coordinator().await;
        let orchestrator = SearchOrchestrator::new(coordinator);

        let engine1 = create_test_engine().await;
        let engine2 = create_test_engine().await;

        orchestrator.register_engine("agent1", engine1);
        orchestrator.register_engine("agent2", engine2);

        let results = orchestrator.broadcast_search("test", 5).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.contains_key("agent1"));
        assert!(results.contains_key("agent2"));
    }
}
