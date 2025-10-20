use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::embeddings::EmbeddingEngine;
use crate::types::TaskEpisode;

use super::cognitive_manager::Memory;
use super::episodic::EpisodicMemory;

/// Memory retrieval system with multiple strategies
pub struct MemoryRetrieval {
    db: Arc<Surreal<Db>>,
    episodic_memory: Arc<EpisodicMemory>,
    embedding_engine: Option<EmbeddingEngine>,
}

/// Retrieval strategy for memory search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetrievalStrategy {
    /// Most recent memories first
    Recency,

    /// Most relevant to query (semantic similarity)
    Relevance,

    /// Most important memories (based on access count and pattern value)
    Importance,

    /// Combined strategy with weights
    Hybrid {
        recency_weight: f32,
        relevance_weight: f32,
        importance_weight: f32,
    },
}

impl Default for RetrievalStrategy {
    fn default() -> Self {
        Self::Hybrid {
            recency_weight: 0.3,
            relevance_weight: 0.5,
            importance_weight: 0.2,
        }
    }
}

/// Retrieved memory with scoring information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub memory: Memory,
    pub scores: MemoryScores,
    pub combined_score: f32,
}

/// Individual scoring components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryScores {
    pub recency_score: f32,
    pub relevance_score: f32,
    pub importance_score: f32,
}

/// Context for finding similar episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub task_description: String,
    pub active_files: Vec<String>,
    pub active_symbols: Vec<String>,
    pub tags: Vec<String>,
}

impl MemoryRetrieval {
    /// Create a new memory retrieval system
    pub fn new(db: Arc<Surreal<Db>>, episodic_memory: Arc<EpisodicMemory>) -> Result<Self> {
        // Initialize embedding engine for semantic search
        let embedding_engine = match EmbeddingEngine::new() {
            Ok(engine) => {
                tracing::info!("Memory retrieval: embedding engine initialized ({})", engine.model_name());
                Some(engine)
            }
            Err(e) => {
                tracing::warn!("Memory retrieval: failed to init embeddings: {}", e);
                None
            }
        };

        Ok(Self {
            db,
            episodic_memory,
            embedding_engine,
        })
    }

    /// Retrieve memories using specified strategy
    pub async fn retrieve(
        &self,
        query: &str,
        strategy: RetrievalStrategy,
        limit: usize,
    ) -> Result<Vec<ScoredMemory>> {
        tracing::debug!(
            query = %query,
            strategy = ?strategy,
            limit = %limit,
            "Retrieving memories"
        );

        match strategy {
            RetrievalStrategy::Recency => self.retrieve_by_recency(limit).await,
            RetrievalStrategy::Relevance => self.retrieve_by_relevance(query, limit).await,
            RetrievalStrategy::Importance => self.retrieve_by_importance(limit).await,
            RetrievalStrategy::Hybrid {
                recency_weight,
                relevance_weight,
                importance_weight,
            } => {
                self.retrieve_hybrid(
                    query,
                    limit,
                    recency_weight,
                    relevance_weight,
                    importance_weight,
                )
                .await
            }
        }
    }

    /// Retrieve most recent memories
    async fn retrieve_by_recency(&self, limit: usize) -> Result<Vec<ScoredMemory>> {
        let episodes = self.episodic_memory.episodes();
        let now = Utc::now();

        let mut scored: Vec<_> = episodes
            .iter()
            .map(|episode| {
                let age = (now - episode.timestamp).num_seconds() as f32;
                let recency_score = calculate_recency_score(age);

                ScoredMemory {
                    memory: episode_to_memory(episode),
                    scores: MemoryScores {
                        recency_score,
                        relevance_score: 0.0,
                        importance_score: 0.0,
                    },
                    combined_score: recency_score,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored.truncate(limit);
        Ok(scored)
    }

    /// Retrieve most relevant memories (semantic similarity)
    async fn retrieve_by_relevance(&self, query: &str, limit: usize) -> Result<Vec<ScoredMemory>> {
        // Use episodic memory's similarity search
        let similar_episodes = self.episodic_memory.find_similar(query, limit * 2).await;

        let scored: Vec<_> = similar_episodes
            .iter()
            .enumerate()
            .map(|(idx, episode)| {
                // Higher rank = lower score
                let relevance_score = 1.0 - (idx as f32 / (limit * 2) as f32);

                ScoredMemory {
                    memory: episode_to_memory(episode),
                    scores: MemoryScores {
                        recency_score: 0.0,
                        relevance_score,
                        importance_score: 0.0,
                    },
                    combined_score: relevance_score,
                }
            })
            .take(limit)
            .collect();

        Ok(scored)
    }

    /// Retrieve most important memories
    async fn retrieve_by_importance(&self, limit: usize) -> Result<Vec<ScoredMemory>> {
        let episodes = self.episodic_memory.episodes();

        let mut scored: Vec<_> = episodes
            .iter()
            .map(|episode| {
                let importance_score = calculate_importance_score(episode);

                ScoredMemory {
                    memory: episode_to_memory(episode),
                    scores: MemoryScores {
                        recency_score: 0.0,
                        relevance_score: 0.0,
                        importance_score,
                    },
                    combined_score: importance_score,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored.truncate(limit);
        Ok(scored)
    }

    /// Retrieve using hybrid strategy (combines all factors)
    async fn retrieve_hybrid(
        &self,
        query: &str,
        limit: usize,
        recency_weight: f32,
        relevance_weight: f32,
        importance_weight: f32,
    ) -> Result<Vec<ScoredMemory>> {
        // Get similar episodes
        let similar_episodes = self.episodic_memory.find_similar(query, limit * 3).await;
        let now = Utc::now();

        let mut scored: Vec<_> = similar_episodes
            .iter()
            .enumerate()
            .map(|(idx, episode)| {
                let age = (now - episode.timestamp).num_seconds() as f32;
                let recency_score = calculate_recency_score(age);
                let relevance_score = 1.0 - (idx as f32 / (limit * 3) as f32);
                let importance_score = calculate_importance_score(episode);

                let combined_score = (recency_score * recency_weight)
                    + (relevance_score * relevance_weight)
                    + (importance_score * importance_weight);

                ScoredMemory {
                    memory: episode_to_memory(episode),
                    scores: MemoryScores {
                        recency_score,
                        relevance_score,
                        importance_score,
                    },
                    combined_score,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored.truncate(limit);
        Ok(scored)
    }

    /// Find similar episodes based on context
    pub async fn find_similar_episodes(&self, context: &Context) -> Result<Vec<TaskEpisode>> {
        tracing::debug!(
            task = %context.task_description,
            files = ?context.active_files,
            "Finding similar episodes"
        );

        // Primary search by task description
        let mut results = self
            .episodic_memory
            .find_similar(&context.task_description, 10)
            .await;

        // Filter by file overlap if specified
        if !context.active_files.is_empty() {
            results.retain(|episode| {
                let overlap = episode
                    .files_touched
                    .iter()
                    .filter(|f| context.active_files.contains(f))
                    .count();

                overlap > 0
            });
        }

        Ok(results)
    }

    /// Get memories by time range
    pub async fn get_temporal_memories(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Memory>> {
        let episodes = self.episodic_memory.episodes();

        let memories: Vec<_> = episodes
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .map(episode_to_memory)
            .collect();

        tracing::debug!(
            "Found {} memories between {} and {}",
            memories.len(),
            start,
            end
        );

        Ok(memories)
    }

    /// Get memories related to specific files
    pub async fn get_file_related_memories(&self, file_path: &str) -> Result<Vec<Memory>> {
        let episodes = self.episodic_memory.episodes();

        let memories: Vec<_> = episodes
            .iter()
            .filter(|e| e.files_touched.iter().any(|f| f.contains(file_path)))
            .map(episode_to_memory)
            .collect();

        tracing::debug!(
            "Found {} memories related to file: {}",
            memories.len(),
            file_path
        );

        Ok(memories)
    }

    /// Search memories by keyword
    pub async fn search_by_keyword(&self, keyword: &str) -> Result<Vec<Memory>> {
        let episodes = self.episodic_memory.episodes();

        let keyword_lower = keyword.to_lowercase();
        let memories: Vec<_> = episodes
            .iter()
            .filter(|e| {
                e.task_description.to_lowercase().contains(&keyword_lower)
                    || e.solution_path.to_lowercase().contains(&keyword_lower)
            })
            .map(episode_to_memory)
            .collect();

        tracing::debug!(
            "Found {} memories matching keyword: {}",
            memories.len(),
            keyword
        );

        Ok(memories)
    }

    /// Get retrieval statistics
    pub async fn get_statistics(&self) -> Result<RetrievalStats> {
        let episodes = self.episodic_memory.episodes();
        let now = Utc::now();

        let total_memories = episodes.len();
        let recent_memories = episodes
            .iter()
            .filter(|e| (now - e.timestamp) < Duration::days(7))
            .count();

        let high_value_memories = episodes.iter().filter(|e| e.pattern_value > 0.7).count();

        let avg_access_count = if total_memories > 0 {
            episodes.iter().map(|e| e.access_count as f32).sum::<f32>() / total_memories as f32
        } else {
            0.0
        };

        Ok(RetrievalStats {
            total_memories,
            recent_memories,
            high_value_memories,
            avg_access_count,
        })
    }
}

/// Retrieval statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStats {
    pub total_memories: usize,
    pub recent_memories: usize,
    pub high_value_memories: usize,
    pub avg_access_count: f32,
}

/// Calculate recency score (exponential decay)
fn calculate_recency_score(age_seconds: f32) -> f32 {
    // Use exponential decay: score = e^(-age/half_life)
    // Half-life of 7 days (604800 seconds)
    let half_life = 604800.0;
    (-age_seconds / half_life).exp()
}

/// Calculate importance score based on episode attributes
fn calculate_importance_score(episode: &TaskEpisode) -> f32 {
    // Combine pattern value and access count
    let access_score = (episode.access_count as f32).min(100.0) / 100.0;
    let pattern_score = episode.pattern_value;

    // Weight: 60% pattern value, 40% access count
    (pattern_score * 0.6) + (access_score * 0.4)
}

/// Convert TaskEpisode to Memory
fn episode_to_memory(episode: &TaskEpisode) -> Memory {
    Memory {
        id: episode.id.0.clone(),
        content: format!(
            "Task: {}\nSolution: {}\nFiles: {:?}\nQueries: {:?}",
            episode.task_description,
            episode.solution_path,
            episode.files_touched,
            episode.queries_made
        ),
        memory_type: super::cognitive_manager::MemoryType::Episodic,
        relevance_score: episode.pattern_value,
        timestamp: episode.timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::storage::Storage;
    use crate::types::{ContextSnapshot, EpisodeId, Outcome, TokenCount};
    use tempfile::TempDir;

    async fn create_test_setup() -> (Arc<Surreal<Db>>, Arc<EpisodicMemory>, TempDir, TempDir) {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let db = Surreal::new::<surrealdb::engine::local::RocksDb>(temp_dir1.path())
            .await
            .unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
        let mut episodic = EpisodicMemory::new(storage, 30).unwrap();
        episodic.load().await.unwrap();

        (Arc::new(db), Arc::new(episodic), temp_dir1, temp_dir2)
    }

    #[tokio::test]
    async fn test_retrieve_by_recency() {
        let (db, episodic, _temp1, _temp2) = create_test_setup().await;
        let retrieval = MemoryRetrieval::new(db, episodic.clone()).unwrap();

        // Add some episodes
        let mut ep_mem = Arc::try_unwrap(episodic).unwrap();
        for i in 0..5 {
            let episode = TaskEpisode {
                schema_version: 1,
                id: EpisodeId::new(),
                timestamp: Utc::now() - Duration::days(i),
                task_description: format!("Task {}", i),
                initial_context: ContextSnapshot::default(),
                queries_made: Vec::new(),
                files_touched: Vec::new(),
                solution_path: String::new(),
                outcome: Outcome::Success,
                tokens_used: TokenCount::zero(),
                access_count: 0,
                pattern_value: 0.5,
            };
            ep_mem.record_episode(episode).await.unwrap();
        }

        let episodic = Arc::new(ep_mem);
        let retrieval = MemoryRetrieval::new(retrieval.db.clone(), episodic).unwrap();

        let results = retrieval
            .retrieve("", RetrievalStrategy::Recency, 3)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        // Most recent should have highest score
        assert!(results[0].scores.recency_score > results[1].scores.recency_score);
    }

    #[tokio::test]
    async fn test_retrieve_by_importance() {
        let (db, episodic, _temp1, _temp2) = create_test_setup().await;

        let mut ep_mem = Arc::try_unwrap(episodic).unwrap();

        // Add episodes with different importance
        let high_importance = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "High importance task".to_string(),
            initial_context: ContextSnapshot::default(),
            queries_made: Vec::new(),
            files_touched: Vec::new(),
            solution_path: String::new(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::zero(),
            access_count: 50,
            pattern_value: 0.9,
        };

        let low_importance = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Low importance task".to_string(),
            initial_context: ContextSnapshot::default(),
            queries_made: Vec::new(),
            files_touched: Vec::new(),
            solution_path: String::new(),
            outcome: Outcome::Success,
            tokens_used: TokenCount::zero(),
            access_count: 1,
            pattern_value: 0.2,
        };

        ep_mem.record_episode(high_importance).await.unwrap();
        ep_mem.record_episode(low_importance).await.unwrap();

        let episodic = Arc::new(ep_mem);
        let retrieval = MemoryRetrieval::new(db, episodic).unwrap();

        let results = retrieval
            .retrieve("", RetrievalStrategy::Importance, 2)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        // High importance should be first
        assert!(results[0].scores.importance_score > results[1].scores.importance_score);
    }

    #[test]
    fn test_recency_score_calculation() {
        let recent = calculate_recency_score(3600.0); // 1 hour
        let old = calculate_recency_score(604800.0); // 7 days

        assert!(recent > old);
        assert!(recent > 0.9);
        assert!(old < 0.6);
    }
}
