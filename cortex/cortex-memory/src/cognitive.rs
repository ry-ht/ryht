//! Cognitive manager orchestrating all memory operations.

use crate::types::*;
use crate::{
    EpisodicMemorySystem, MemoryConsolidator, ProceduralMemorySystem, SemanticMemorySystem,
    WorkingMemorySystem,
};
use cortex_core::error::Result;
use cortex_core::id::CortexId;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use tracing::{info, instrument};

/// Cognitive manager coordinating all memory systems
pub struct CognitiveManager {
    episodic: Arc<EpisodicMemorySystem>,
    semantic: Arc<SemanticMemorySystem>,
    working: Arc<WorkingMemorySystem>,
    procedural: Arc<ProceduralMemorySystem>,
    consolidator: Arc<MemoryConsolidator>,
}

impl CognitiveManager {
    /// Create a new cognitive manager
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(connection_manager.clone()));
        let semantic = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));
        let working = Arc::new(WorkingMemorySystem::new(1000, 100 * 1024 * 1024)); // 100MB
        let procedural = Arc::new(ProceduralMemorySystem::new(connection_manager));

        let consolidator = Arc::new(MemoryConsolidator::new(
            episodic.clone(),
            semantic.clone(),
            procedural.clone(),
            working.clone(),
        ));

        Self {
            episodic,
            semantic,
            working,
            procedural,
            consolidator,
        }
    }

    /// Create with custom configuration
    pub fn with_config(connection_manager: Arc<ConnectionManager>, max_items: usize, max_bytes: usize) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(connection_manager.clone()));
        let semantic = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));
        let working = Arc::new(WorkingMemorySystem::new(max_items, max_bytes));
        let procedural = Arc::new(ProceduralMemorySystem::new(connection_manager));

        let consolidator = Arc::new(MemoryConsolidator::new(
            episodic.clone(),
            semantic.clone(),
            procedural.clone(),
            working.clone(),
        ));

        Self {
            episodic,
            semantic,
            working,
            procedural,
            consolidator,
        }
    }

    // ========================================================================
    // Cognitive Operations (Remember, Recall, Associate, Forget, Dream)
    // ========================================================================

    /// Remember: Store a new episode
    #[instrument(skip(self, episode))]
    pub async fn remember_episode(&self, episode: &EpisodicMemory) -> Result<CortexId> {
        info!(episode_id = %episode.id, "Remembering episode");
        self.episodic.store_episode(episode).await
    }

    /// Remember: Store a semantic unit
    #[instrument(skip(self, unit))]
    pub async fn remember_unit(&self, unit: &SemanticUnit) -> Result<CortexId> {
        info!(unit_id = %unit.id, "Remembering semantic unit");
        self.semantic.store_unit(unit).await
    }

    /// Remember: Store a learned pattern
    #[instrument(skip(self, pattern))]
    pub async fn remember_pattern(&self, pattern: &LearnedPattern) -> Result<CortexId> {
        info!(pattern_id = %pattern.id, "Remembering learned pattern");
        self.procedural.store_pattern(pattern).await
    }

    /// Recall: Retrieve similar episodes
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_episodes(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<EpisodicMemory>>> {
        info!(query = %query.query_text, "Recalling episodes");
        self.episodic.retrieve_similar(query, embedding).await
    }

    /// Recall: Retrieve similar code units
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_units(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<SemanticUnit>>> {
        info!(query = %query.query_text, "Recalling semantic units");
        self.semantic.search_units(query, embedding).await
    }

    /// Recall: Retrieve similar patterns
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_patterns(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<LearnedPattern>>> {
        info!(query = %query.query_text, "Recalling patterns");
        self.procedural.search_patterns(query, embedding).await
    }

    /// Associate: Link related memories
    #[instrument(skip(self))]
    pub async fn associate(
        &self,
        source_id: CortexId,
        target_id: CortexId,
        dependency_type: DependencyType,
    ) -> Result<()> {
        info!(%source_id, %target_id, "Creating association");

        let dependency = Dependency {
            id: CortexId::new(),
            source_id,
            target_id,
            dependency_type,
            is_direct: true,
            is_runtime: false,
            is_dev: false,
            metadata: std::collections::HashMap::new(),
        };

        self.semantic.store_dependency(&dependency).await?;
        Ok(())
    }

    /// Forget: Remove low-importance memories
    #[instrument(skip(self))]
    pub async fn forget(&self, threshold: f32) -> Result<usize> {
        info!(threshold, "Forgetting low-importance memories");
        self.episodic.forget_unimportant(threshold).await
    }

    /// Dream: Offline consolidation and pattern extraction
    #[instrument(skip(self))]
    pub async fn dream(&self) -> Result<Vec<LearnedPattern>> {
        info!("Starting dream consolidation");
        self.consolidator.dream().await
    }

    /// Consolidate: Transfer from working to long-term memory
    #[instrument(skip(self))]
    pub async fn consolidate(&self) -> Result<crate::consolidation::ConsolidationReport> {
        info!("Starting memory consolidation");
        self.consolidator.consolidate().await
    }

    /// Perform incremental consolidation with batch size
    #[instrument(skip(self))]
    pub async fn consolidate_incremental(&self, batch_size: usize) -> Result<crate::consolidation::ConsolidationReport> {
        info!(batch_size, "Starting incremental consolidation");
        self.consolidator.incremental_consolidate(batch_size).await
    }

    // ========================================================================
    // Access to Memory Systems
    // ========================================================================

    pub fn episodic(&self) -> &Arc<EpisodicMemorySystem> {
        &self.episodic
    }

    pub fn semantic(&self) -> &Arc<SemanticMemorySystem> {
        &self.semantic
    }

    pub fn working(&self) -> &Arc<WorkingMemorySystem> {
        &self.working
    }

    pub fn procedural(&self) -> &Arc<ProceduralMemorySystem> {
        &self.procedural
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get comprehensive memory statistics
    pub async fn get_statistics(&self) -> Result<MemoryStats> {
        Ok(MemoryStats {
            episodic: self.episodic.get_statistics().await?,
            semantic: self.semantic.get_statistics().await?,
            working: self.working.get_statistics(),
            procedural: self.procedural.get_statistics().await?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::{DatabaseConfig, PoolConfig};

    async fn create_test_manager() -> CognitiveManager {
        let config = ConnectionConfig::memory();
        let pool_config = PoolConfig::default();
        let manager = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager"),
        );
        CognitiveManager::new(manager)
    }

    #[tokio::test]
    async fn test_remember_and_recall() {
        let manager = create_test_manager().await;

        let episode = EpisodicMemory::new(
            "Test episode".to_string(),
            "test-agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        let id = manager
            .remember_episode(&episode)
            .await
            .expect("Failed to remember episode");

        assert_eq!(id, episode.id);
    }

    #[tokio::test]
    async fn test_working_memory() {
        let manager = create_test_manager().await;

        let key = "test_key".to_string();
        let value = vec![1, 2, 3];

        assert!(manager.working().store(key.clone(), value.clone(), Priority::Medium));
        assert_eq!(manager.working().retrieve(&key), Some(value));
    }

    #[tokio::test]
    async fn test_statistics() {
        let manager = create_test_manager().await;

        let stats = manager
            .get_statistics()
            .await
            .expect("Failed to get statistics");

        assert_eq!(stats.episodic.total_episodes, 0);
        assert_eq!(stats.semantic.total_units, 0);
        assert_eq!(stats.working.current_items, 0);
        assert_eq!(stats.procedural.total_patterns, 0);
    }
}
