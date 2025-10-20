//! Episodic memory implementation for storing development episodes.
//!
//! Episodic memory stores experiences and events with full context, allowing
//! the system to learn from past work and extract successful patterns.

use crate::types::*;
use chrono::Utc;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use tracing::{debug, info};

/// Episodic memory system for storing and retrieving development episodes
pub struct EpisodicMemorySystem {
    connection_manager: Arc<ConnectionManager>,
    importance_threshold: f32,
}

impl EpisodicMemorySystem {
    /// Create a new episodic memory system
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self {
            connection_manager,
            importance_threshold: 0.3,
        }
    }

    /// Set the importance threshold for forgetting
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.importance_threshold = threshold;
        self
    }

    /// Store a new episode
    pub async fn store_episode(&self, episode: &EpisodicMemory) -> Result<CortexId> {
        info!(episode_id = %episode.id, "Storing episodic memory");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Convert to SurrealDB format
        let query = "
            CREATE episode CONTENT {
                id: $id,
                episode_type: $episode_type,
                task_description: $task_description,
                agent_id: $agent_id,
                session_id: $session_id,
                workspace_id: $workspace_id,
                entities_created: $entities_created,
                entities_modified: $entities_modified,
                entities_deleted: $entities_deleted,
                files_touched: $files_touched,
                queries_made: $queries_made,
                tools_used: $tools_used,
                solution_summary: $solution_summary,
                outcome: $outcome,
                success_metrics: $success_metrics,
                errors_encountered: $errors_encountered,
                lessons_learned: $lessons_learned,
                duration_seconds: $duration_seconds,
                tokens_used: $tokens_used,
                embedding: $embedding,
                created_at: $created_at,
                completed_at: $completed_at
            }
        ";

        conn.connection().query(query)
            .bind(("id", episode.id.to_string()))
            .bind(("episode_type", episode.episode_type))
            .bind(("task_description", episode.task_description.clone()))
            .bind(("agent_id", episode.agent_id.clone()))
            .bind(("session_id", episode.session_id.clone()))
            .bind(("workspace_id", episode.workspace_id.to_string()))
            .bind(("entities_created", episode.entities_created.clone()))
            .bind(("entities_modified", episode.entities_modified.clone()))
            .bind(("entities_deleted", episode.entities_deleted.clone()))
            .bind(("files_touched", episode.files_touched.clone()))
            .bind(("queries_made", episode.queries_made.clone()))
            .bind(("tools_used", episode.tools_used.clone()))
            .bind(("solution_summary", episode.solution_summary.clone()))
            .bind(("outcome", episode.outcome))
            .bind(("success_metrics", episode.success_metrics.clone()))
            .bind(("errors_encountered", episode.errors_encountered.clone()))
            .bind(("lessons_learned", episode.lessons_learned.clone()))
            .bind(("duration_seconds", episode.duration_seconds))
            .bind(("tokens_used", episode.tokens_used.clone()))
            .bind(("embedding", episode.embedding.clone()))
            .bind(("created_at", episode.created_at))
            .bind(("completed_at", episode.completed_at.clone()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        debug!(episode_id = %episode.id, "Episode stored successfully");
        Ok(episode.id)
    }

    /// Retrieve an episode by ID
    pub async fn get_episode(&self, id: CortexId) -> Result<Option<EpisodicMemory>> {
        debug!(episode_id = %id, "Retrieving episode");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM episode WHERE id = $id")
            .bind(("id", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let episode: Option<EpisodicMemory> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(episode)
    }

    /// Retrieve episodes by similarity search using embeddings
    pub async fn retrieve_similar(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<EpisodicMemory>>> {
        info!(query = %query.query_text, limit = query.limit, "Searching for similar episodes");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Use SurrealDB's vector search capabilities
        let query_str = "
            SELECT *,
                   vector::distance::cosine(embedding, $query_embedding) AS similarity
            FROM episode
            WHERE embedding IS NOT NONE
              AND vector::distance::cosine(embedding, $query_embedding) <= $threshold
            ORDER BY similarity ASC
            LIMIT $limit
        ";

        let mut result = conn
            .connection()
            .query(query_str)
            .bind(("query_embedding", embedding.to_vec()))
            .bind(("threshold", 1.0 - query.similarity_threshold))
            .bind(("limit", query.limit))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let episodes: Vec<(EpisodicMemory, f32)> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        let results = episodes
            .into_iter()
            .map(|(episode, similarity)| {
                let relevance = self.calculate_relevance(&episode);
                MemorySearchResult {
                    item: episode,
                    similarity_score: 1.0 - similarity, // Convert distance to similarity
                    relevance_score: relevance,
                }
            })
            .collect();

        Ok(results)
    }

    /// Retrieve episodes by outcome (successful, failed, etc.)
    pub async fn retrieve_by_outcome(
        &self,
        outcome: EpisodeOutcome,
        limit: usize,
    ) -> Result<Vec<EpisodicMemory>> {
        debug!(outcome = ?outcome, limit, "Retrieving episodes by outcome");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM episode WHERE outcome = $outcome ORDER BY created_at DESC LIMIT $limit")
            .bind(("outcome", outcome))
            .bind(("limit", limit))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let episodes: Vec<EpisodicMemory> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(episodes)
    }

    /// Extract patterns from successful episodes
    pub async fn extract_patterns(&self, min_success_rate: f32) -> Result<Vec<LearnedPattern>> {
        info!(min_success_rate, "Extracting patterns from successful episodes");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Query successful episodes
        let mut result = conn
            .connection()
            .query("SELECT * FROM episode WHERE outcome = $outcome ORDER BY created_at DESC")
            .bind(("outcome", EpisodeOutcome::Success))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let episodes: Vec<EpisodicMemory> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        // Group episodes by similar characteristics
        let patterns = self.cluster_and_extract_patterns(&episodes, min_success_rate)?;

        info!(patterns_found = patterns.len(), "Pattern extraction complete");
        Ok(patterns)
    }

    /// Calculate importance score for an episode
    pub fn calculate_importance(&self, episode: &EpisodicMemory) -> f32 {
        let mut factors = ImportanceFactors {
            recency_score: 0.0,
            frequency_score: 0.0,
            outcome_score: 0.0,
            complexity_score: 0.0,
            novelty_score: 0.0,
            relevance_score: 0.0,
        };

        // Recency: exponential decay based on time
        let age_days = (Utc::now() - episode.created_at).num_days() as f64;
        factors.recency_score = (-age_days / 30.0).exp() as f32;

        // Outcome: weight based on success/failure
        factors.outcome_score = match episode.outcome {
            EpisodeOutcome::Success => 1.0,
            EpisodeOutcome::Partial => 0.6,
            EpisodeOutcome::Failure => 0.3,
            EpisodeOutcome::Abandoned => 0.1,
        };

        // Complexity: based on duration and entities modified
        let entities_modified = (episode.entities_created.len()
            + episode.entities_modified.len()
            + episode.entities_deleted.len()) as f32;
        factors.complexity_score = (entities_modified / 10.0).min(1.0);

        // Novelty: if there are lessons learned
        factors.novelty_score = if !episode.lessons_learned.is_empty() {
            0.8
        } else {
            0.2
        };

        // Relevance: based on recent access and tools used
        factors.relevance_score = (episode.tools_used.len() as f32 / 5.0).min(1.0);

        factors.combined_score()
    }

    /// Calculate relevance score for search results
    fn calculate_relevance(&self, episode: &EpisodicMemory) -> f32 {
        let importance = self.calculate_importance(episode);
        let outcome_bonus = match episode.outcome {
            EpisodeOutcome::Success => 0.2,
            EpisodeOutcome::Partial => 0.1,
            _ => 0.0,
        };

        (importance + outcome_bonus).min(1.0)
    }

    /// Cluster episodes and extract common patterns
    fn cluster_and_extract_patterns(
        &self,
        episodes: &[EpisodicMemory],
        min_success_rate: f32,
    ) -> Result<Vec<LearnedPattern>> {
        let mut patterns = Vec::new();

        // Group by episode type
        let mut type_groups: std::collections::HashMap<EpisodeType, Vec<&EpisodicMemory>> =
            std::collections::HashMap::new();

        for episode in episodes {
            type_groups
                .entry(episode.episode_type)
                .or_default()
                .push(episode);
        }

        // Extract patterns from each group
        for (episode_type, group) in type_groups {
            if group.len() < 3 {
                continue; // Need at least 3 episodes to extract a pattern
            }

            // Analyze common tools and approaches
            let mut tool_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            for episode in &group {
                for tool in &episode.tools_used {
                    *tool_counts.entry(tool.tool_name.clone()).or_default() += 1;
                }
            }

            // Create pattern if tools are common enough
            let total_episodes = group.len();
            for (tool_name, count) in tool_counts {
                let frequency = count as f32 / total_episodes as f32;
                if frequency >= min_success_rate {
                    let pattern = LearnedPattern::new(
                        PatternType::Code, // Map from episode type
                        format!("{:?} pattern using {}", episode_type, tool_name),
                        format!("Common pattern extracted from {} episodes", total_episodes),
                        format!("Episode type: {:?}", episode_type),
                    );
                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }

    /// Forget episodes below importance threshold
    pub async fn forget_unimportant(&self, threshold: f32) -> Result<usize> {
        info!(threshold, "Forgetting episodes below importance threshold");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Get all episodes and calculate importance
        let mut result = conn
            .connection()
            .query("SELECT * FROM episode")
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let episodes: Vec<EpisodicMemory> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        let mut deleted_count = 0;
        for episode in episodes {
            let importance = self.calculate_importance(&episode);
            if importance < threshold {
                conn.connection()
                    .query("DELETE episode WHERE id = $id")
                    .bind(("id", episode.id.to_string()))
                    .await
                    .map_err(|e| CortexError::storage(e.to_string()))?;
                deleted_count += 1;
            }
        }

        info!(deleted_count, "Forgotten episodes below threshold");
        Ok(deleted_count)
    }

    /// Get statistics about episodic memory
    pub async fn get_statistics(&self) -> Result<EpisodicStats> {
        debug!("Retrieving episodic memory statistics");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let query = "
            SELECT
                count() AS total,
                count(outcome = 'success') AS successful,
                count(outcome = 'failure') AS failed,
                math::mean(duration_seconds) AS avg_duration,
                math::sum(tokens_used.total) AS total_tokens
            FROM episode
            GROUP ALL
        ";

        let mut result = conn
            .connection()
            .query(query)
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let stats: Option<serde_json::Value> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        if let Some(stats) = stats {
            Ok(EpisodicStats {
                total_episodes: stats["total"].as_u64().unwrap_or(0),
                successful_episodes: stats["successful"].as_u64().unwrap_or(0),
                failed_episodes: stats["failed"].as_u64().unwrap_or(0),
                average_duration_seconds: stats["avg_duration"].as_f64().unwrap_or(0.0),
                total_tokens_used: stats["total_tokens"].as_u64().unwrap_or(0),
            })
        } else {
            Ok(EpisodicStats {
                total_episodes: 0,
                successful_episodes: 0,
                failed_episodes: 0,
                average_duration_seconds: 0.0,
                total_tokens_used: 0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};

    async fn create_test_memory() -> EpisodicMemorySystem {
        let config = ConnectionConfig::memory();
        let pool_config = PoolConfig::default();
        let manager = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager"),
        );
        EpisodicMemorySystem::new(manager)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_episode() {
        let memory = create_test_memory().await;

        let mut episode = EpisodicMemory::new(
            "Test task".to_string(),
            "test-agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );
        episode.outcome = EpisodeOutcome::Success;
        episode.duration_seconds = 120;

        let id = memory
            .store_episode(&episode)
            .await
            .expect("Failed to store episode");

        let retrieved = memory
            .get_episode(id)
            .await
            .expect("Failed to retrieve episode");

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.task_description, "Test task");
        assert_eq!(retrieved.agent_id, "test-agent");
    }

    #[tokio::test]
    async fn test_importance_calculation() {
        let memory = create_test_memory().await;

        let mut episode = EpisodicMemory::new(
            "Important task".to_string(),
            "test-agent".to_string(),
            CortexId::new(),
            EpisodeType::Feature,
        );
        episode.outcome = EpisodeOutcome::Success;
        episode.lessons_learned = vec!["Learned something".to_string()];
        episode.entities_modified = vec!["file1.rs".to_string(), "file2.rs".to_string()];

        let importance = memory.calculate_importance(&episode);
        assert!(importance > 0.5, "Importance should be high for successful, complex episodes");
    }

    #[tokio::test]
    async fn test_retrieve_by_outcome() {
        let memory = create_test_memory().await;

        // Store multiple episodes with different outcomes
        for i in 0..5 {
            let mut episode = EpisodicMemory::new(
                format!("Task {}", i),
                "test-agent".to_string(),
                CortexId::new(),
                EpisodeType::Task,
            );
            episode.outcome = if i % 2 == 0 {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Failure
            };

            memory
                .store_episode(&episode)
                .await
                .expect("Failed to store episode");
        }

        let successful = memory
            .retrieve_by_outcome(EpisodeOutcome::Success, 10)
            .await
            .expect("Failed to retrieve successful episodes");

        assert_eq!(successful.len(), 3);
    }

    #[tokio::test]
    async fn test_statistics() {
        let memory = create_test_memory().await;

        // Store some episodes
        for i in 0..3 {
            let mut episode = EpisodicMemory::new(
                format!("Task {}", i),
                "test-agent".to_string(),
                CortexId::new(),
                EpisodeType::Task,
            );
            episode.outcome = EpisodeOutcome::Success;
            episode.duration_seconds = 100 + i * 10;
            episode.tokens_used.total = 1000;

            memory
                .store_episode(&episode)
                .await
                .expect("Failed to store episode");
        }

        let stats = memory
            .get_statistics()
            .await
            .expect("Failed to get statistics");

        assert_eq!(stats.total_episodes, 3);
        assert_eq!(stats.successful_episodes, 3);
        assert!(stats.average_duration_seconds > 0.0);
    }
}
