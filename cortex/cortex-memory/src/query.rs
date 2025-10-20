//! Cross-memory query capabilities for unified search across all memory systems.

use crate::types::*;
use crate::CognitiveManager;
use cortex_core::error::Result;
use cortex_core::id::CortexId;
use std::sync::Arc;
use tracing::{debug, info};

/// Unified query result combining different memory types
#[derive(Debug, Clone)]
pub enum UnifiedMemoryResult {
    Episode(MemorySearchResult<EpisodicMemory>),
    SemanticUnit(MemorySearchResult<SemanticUnit>),
    Pattern(MemorySearchResult<LearnedPattern>),
}

impl UnifiedMemoryResult {
    /// Get the combined relevance score
    pub fn relevance(&self) -> f32 {
        match self {
            Self::Episode(r) => r.relevance_score,
            Self::SemanticUnit(r) => r.relevance_score,
            Self::Pattern(r) => r.relevance_score,
        }
    }

    /// Get the similarity score
    pub fn similarity(&self) -> f32 {
        match self {
            Self::Episode(r) => r.similarity_score,
            Self::SemanticUnit(r) => r.similarity_score,
            Self::Pattern(r) => r.similarity_score,
        }
    }

    /// Get a combined score for ranking
    pub fn combined_score(&self) -> f32 {
        (self.relevance() * 0.6) + (self.similarity() * 0.4)
    }
}

/// Cross-memory query executor
pub struct CrossMemoryQuery {
    cognitive_manager: Arc<CognitiveManager>,
}

impl CrossMemoryQuery {
    pub fn new(cognitive_manager: Arc<CognitiveManager>) -> Self {
        Self { cognitive_manager }
    }

    /// Search across all memory systems with a unified query
    pub async fn search_all(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<UnifiedMemoryResult>> {
        info!(query = %query.query_text, "Searching across all memory systems");

        let mut results = Vec::new();

        // Search episodic memory
        match self
            .cognitive_manager
            .recall_episodes(query, embedding)
            .await
        {
            Ok(episodes) => {
                for episode in episodes {
                    results.push(UnifiedMemoryResult::Episode(episode));
                }
                debug!(count = results.len(), "Found episodic memories");
            }
            Err(e) => {
                debug!(error = %e, "Failed to search episodic memory");
            }
        }

        // Search semantic memory
        match self
            .cognitive_manager
            .recall_units(query, embedding)
            .await
        {
            Ok(units) => {
                for unit in units {
                    results.push(UnifiedMemoryResult::SemanticUnit(unit));
                }
                debug!(count = results.len(), "Found semantic units");
            }
            Err(e) => {
                debug!(error = %e, "Failed to search semantic memory");
            }
        }

        // Search procedural memory
        match self
            .cognitive_manager
            .recall_patterns(query, embedding)
            .await
        {
            Ok(patterns) => {
                for pattern in patterns {
                    results.push(UnifiedMemoryResult::Pattern(pattern));
                }
                debug!(count = results.len(), "Found patterns");
            }
            Err(e) => {
                debug!(error = %e, "Failed to search procedural memory");
            }
        }

        // Sort by combined score
        results.sort_by(|a, b| {
            b.combined_score()
                .partial_cmp(&a.combined_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(query.limit);

        info!(total_results = results.len(), "Cross-memory search complete");
        Ok(results)
    }

    /// Find related memories for a given episode
    pub async fn find_related_episodes(
        &self,
        episode_id: CortexId,
    ) -> Result<Vec<EpisodicMemory>> {
        info!(episode_id = %episode_id, "Finding related episodes");

        // Get the source episode
        let episode = self
            .cognitive_manager
            .episodic()
            .get_episode(episode_id)
            .await?
            .ok_or_else(|| {
                cortex_core::error::CortexError::not_found("episode", episode_id.to_string())
            })?;

        // Search for similar episodes using embedding
        if let Some(embedding) = &episode.embedding {
            let query = MemoryQuery::new(episode.task_description.clone())
                .with_limit(10)
                .with_threshold(0.7);

            let results = self
                .cognitive_manager
                .recall_episodes(&query, embedding)
                .await?;

            let related: Vec<EpisodicMemory> = results
                .into_iter()
                .filter(|r| r.item.id != episode_id) // Exclude self
                .map(|r| r.item)
                .collect();

            info!(count = related.len(), "Found related episodes");
            Ok(related)
        } else {
            // No embedding available, return empty
            Ok(Vec::new())
        }
    }

    /// Find code units related to an episode
    pub async fn find_related_code(
        &self,
        episode: &EpisodicMemory,
    ) -> Result<Vec<SemanticUnit>> {
        info!(episode_id = %episode.id, "Finding related code units");

        let mut related = Vec::new();

        // Search by files touched
        for file_path in &episode.files_touched {
            if let Ok(units) = self
                .cognitive_manager
                .semantic()
                .get_units_in_file(file_path)
                .await
            {
                related.extend(units);
            }
        }

        info!(count = related.len(), "Found related code units");
        Ok(related)
    }

    /// Find patterns applicable to a code unit
    pub async fn find_applicable_patterns(
        &self,
        unit: &SemanticUnit,
        embedding: &[f32],
    ) -> Result<Vec<LearnedPattern>> {
        info!(unit_id = %unit.id, "Finding applicable patterns");

        let query = MemoryQuery::new(format!("{} {}", unit.name, unit.purpose))
            .with_limit(5)
            .with_threshold(0.75);

        let results = self
            .cognitive_manager
            .recall_patterns(&query, embedding)
            .await?;

        let patterns: Vec<LearnedPattern> = results.into_iter().map(|r| r.item).collect();

        info!(count = patterns.len(), "Found applicable patterns");
        Ok(patterns)
    }

    /// Get comprehensive context for a code unit
    pub async fn get_unit_context(&self, unit_id: CortexId) -> Result<UnitContext> {
        info!(unit_id = %unit_id, "Getting comprehensive unit context");

        // Get the unit
        let unit = self
            .cognitive_manager
            .semantic()
            .get_unit(unit_id)
            .await?
            .ok_or_else(|| {
                cortex_core::error::CortexError::not_found("code_unit", unit_id.to_string())
            })?;

        // Get dependencies
        let dependencies = self
            .cognitive_manager
            .semantic()
            .get_dependencies(unit_id)
            .await?;

        // Get dependents
        let dependents = self
            .cognitive_manager
            .semantic()
            .get_dependents(unit_id)
            .await?;

        // Get related units in the same file
        let file_units = self
            .cognitive_manager
            .semantic()
            .get_units_in_file(&unit.file_path)
            .await?;

        Ok(UnitContext {
            unit,
            dependencies,
            dependents,
            file_units,
        })
    }

    /// Query by multiple filters
    pub async fn complex_query(&self, filters: QueryFilters) -> Result<QueryResults> {
        info!("Executing complex query with filters");

        let mut results = QueryResults::default();

        // Apply episode filters
        if let Some(outcome) = filters.episode_outcome {
            let episodes = self
                .cognitive_manager
                .episodic()
                .retrieve_by_outcome(outcome, filters.limit.unwrap_or(10))
                .await?;
            results.episodes = episodes;
        }

        // Apply complexity filters for code units
        if let Some(complexity_threshold) = filters.complexity_threshold {
            let complex_units = self
                .cognitive_manager
                .semantic()
                .find_complex_units(complexity_threshold)
                .await?;
            results.code_units = complex_units;
        }

        // Find untested units if requested
        if filters.untested_only {
            let untested = self
                .cognitive_manager
                .semantic()
                .find_untested_units()
                .await?;
            results.code_units.extend(untested);
        }

        // Find undocumented units if requested
        if filters.undocumented_only {
            let undocumented = self
                .cognitive_manager
                .semantic()
                .find_undocumented_units()
                .await?;
            results.code_units.extend(undocumented);
        }

        info!(
            episodes = results.episodes.len(),
            units = results.code_units.len(),
            "Complex query complete"
        );

        Ok(results)
    }
}

/// Context information for a code unit
#[derive(Debug, Clone)]
pub struct UnitContext {
    pub unit: SemanticUnit,
    pub dependencies: Vec<Dependency>,
    pub dependents: Vec<Dependency>,
    pub file_units: Vec<SemanticUnit>,
}

/// Filters for complex queries
#[derive(Debug, Clone, Default)]
pub struct QueryFilters {
    pub episode_outcome: Option<EpisodeOutcome>,
    pub complexity_threshold: Option<u32>,
    pub untested_only: bool,
    pub undocumented_only: bool,
    pub limit: Option<usize>,
}

/// Results from complex queries
#[derive(Debug, Clone, Default)]
pub struct QueryResults {
    pub episodes: Vec<EpisodicMemory>,
    pub code_units: Vec<SemanticUnit>,
    pub patterns: Vec<LearnedPattern>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::connection::ConnectionConfig;
    use cortex_storage::pool::ConnectionPool;

    async fn create_test_query_engine() -> CrossMemoryQuery {
        let config = ConnectionConfig::memory();
        let pool = Arc::new(ConnectionPool::new(config));
        pool.initialize().await.unwrap();

        let cognitive = Arc::new(CognitiveManager::new(pool).await.unwrap());
        CrossMemoryQuery::new(cognitive)
    }

    #[tokio::test]
    async fn test_complex_query() {
        let query_engine = create_test_query_engine().await;

        let filters = QueryFilters {
            episode_outcome: Some(EpisodeOutcome::Success),
            limit: Some(10),
            ..Default::default()
        };

        let results = query_engine
            .complex_query(filters)
            .await
            .expect("Failed to execute complex query");

        assert_eq!(results.episodes.len(), 0); // No episodes in fresh database
    }

    #[tokio::test]
    async fn test_unified_memory_result_scoring() {
        let episode = EpisodicMemory::new(
            "Test".to_string(),
            "agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        let result = UnifiedMemoryResult::Episode(MemorySearchResult {
            item: episode,
            similarity_score: 0.9,
            relevance_score: 0.8,
        });

        assert!(result.combined_score() > 0.0);
        assert!(result.combined_score() <= 1.0);
    }
}
