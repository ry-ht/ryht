use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::types::{Outcome, TaskEpisode};

/// Enhanced episodic memory using SurrealDB's native graph and vector capabilities
pub struct EpisodicMemorySurreal {
    db: Arc<Surreal<Db>>,
    retention_days: u32,
}

/// Episode record for SurrealDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpisodeRecord {
    id: String,
    task_description: String,
    solution_summary: String,
    solution_detail: Option<String>,
    files_touched: Vec<String>,
    queries_made: Vec<String>,
    tools_used: Vec<String>,
    success_score: f32,
    duration_seconds: Option<i64>,
    commit_hash: Option<String>,
    embedding: Option<Vec<f32>>,
    created_at: DateTime<Utc>,
}

impl From<&TaskEpisode> for EpisodeRecord {
    fn from(episode: &TaskEpisode) -> Self {
        let success_score = match episode.outcome {
            Outcome::Success => 1.0,
            Outcome::Partial => 0.5,
            Outcome::Failure => 0.0,
        };

        Self {
            id: episode.id.0.clone(),
            task_description: episode.task_description.clone(),
            solution_summary: episode.solution_path.clone(),
            solution_detail: None,
            files_touched: episode.files_touched.clone(),
            queries_made: episode.queries_made.clone(),
            tools_used: Vec::new(),
            success_score,
            duration_seconds: None,
            commit_hash: None,
            embedding: None,
            created_at: episode.timestamp,
        }
    }
}

/// Search result with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSearchResult {
    pub episode: TaskEpisode,
    pub similarity_score: f32,
}

impl EpisodicMemorySurreal {
    /// Create a new SurrealDB-backed episodic memory
    pub fn new(db: Arc<Surreal<Db>>, retention_days: u32) -> Self {
        Self {
            db,
            retention_days,
        }
    }

    /// Initialize the schema (called once at startup)
    pub async fn initialize_schema(&self) -> Result<()> {
        // Schema is initialized via schema.surql file
        // This is a no-op but kept for API compatibility
        Ok(())
    }

    /// Record a new episode with optional embedding
    pub async fn record_episode(&self, episode: TaskEpisode, embedding: Option<Vec<f32>>) -> Result<()> {
        let mut record = EpisodeRecord::from(&episode);
        record.embedding = embedding;

        let episode_id = episode.id.0.clone();

        // Create episode record
        let _: Option<EpisodeRecord> = self
            .db
            .create(("episode", &episode_id))
            .content(record)
            .await
            .with_context(|| format!("Failed to create episode {}", episode_id))?;

        tracing::debug!(
            episode_id = %episode_id,
            task = %episode.task_description,
            "Recorded episode in SurrealDB"
        );

        Ok(())
    }

    /// Find similar episodes using vector similarity search
    /// This uses SurrealDB's native vector capabilities if available
    pub async fn find_similar(&self, task_description: &str, embedding: Option<&[f32]>, limit: usize) -> Result<Vec<EpisodeSearchResult>> {
        if let Some(query_embedding) = embedding {
            // Use vector similarity search
            self.vector_similarity_search(query_embedding, limit).await
        } else {
            // Fallback to keyword search
            self.keyword_search(task_description, limit).await
        }
    }

    /// Vector similarity search using SurrealDB's native capabilities
    async fn vector_similarity_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<EpisodeSearchResult>> {
        #[derive(Deserialize)]
        struct SimilarityResult {
            id: String,
            task_description: String,
            solution_summary: String,
            files_touched: Vec<String>,
            queries_made: Vec<String>,
            success_score: f32,
            created_at: DateTime<Utc>,
            similarity: f32,
        }

        // Query using vector similarity (use native function approach for compatibility)
        let query = r#"
            LET $results = (
                SELECT *,
                    vector::similarity::cosine(embedding, $embedding) AS similarity
                FROM episode
                WHERE embedding IS NOT NONE
                    AND success_score >= 0.5
            );
            RETURN $results ORDER BY similarity DESC LIMIT $limit;
        "#;

        let mut response = self
            .db
            .query(query)
            .bind(("embedding", query_embedding.to_vec()))
            .bind(("limit", limit))
            .await
            .context("Failed to execute vector similarity search")?;

        let results: Vec<SimilarityResult> = response.take(0).unwrap_or_default();

        let episodes: Vec<EpisodeSearchResult> = results
            .into_iter()
            .map(|r| {
                let outcome = if r.success_score >= 0.9 {
                    Outcome::Success
                } else if r.success_score >= 0.4 {
                    Outcome::Partial
                } else {
                    Outcome::Failure
                };

                EpisodeSearchResult {
                    episode: TaskEpisode {
                        schema_version: 1,
                        id: crate::types::EpisodeId(r.id),
                        timestamp: r.created_at,
                        task_description: r.task_description,
                        initial_context: crate::types::ContextSnapshot::default(),
                        queries_made: r.queries_made,
                        files_touched: r.files_touched,
                        solution_path: r.solution_summary,
                        outcome,
                        tokens_used: crate::types::TokenCount::zero(),
                        access_count: 0,
                        pattern_value: r.success_score,
                    },
                    similarity_score: r.similarity,
                }
            })
            .collect();

        tracing::debug!(
            "Vector similarity search found {} episodes",
            episodes.len()
        );

        Ok(episodes)
    }

    /// Keyword-based search fallback
    async fn keyword_search(&self, task_description: &str, limit: usize) -> Result<Vec<EpisodeSearchResult>> {
        #[derive(Deserialize)]
        struct KeywordResult {
            id: String,
            task_description: String,
            solution_summary: String,
            files_touched: Vec<String>,
            queries_made: Vec<String>,
            success_score: f32,
            created_at: DateTime<Utc>,
        }

        // Extract keywords from task description
        let keywords: Vec<&str> = task_description
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .take(5)
            .collect();

        let pattern = keywords.join("|");

        let query = r#"
            SELECT *
            FROM episode
            WHERE task_description ~ $pattern
                AND success_score >= 0.5
            ORDER BY success_score DESC, created_at DESC
            LIMIT $limit
        "#;

        let mut response = self
            .db
            .query(query)
            .bind(("pattern", pattern))
            .bind(("limit", limit))
            .await
            .context("Failed to execute keyword search")?;

        let results: Vec<KeywordResult> = response.take(0).unwrap_or_default();

        let episodes: Vec<EpisodeSearchResult> = results
            .into_iter()
            .map(|r| {
                let outcome = if r.success_score >= 0.9 {
                    Outcome::Success
                } else if r.success_score >= 0.4 {
                    Outcome::Partial
                } else {
                    Outcome::Failure
                };

                EpisodeSearchResult {
                    episode: TaskEpisode {
                        schema_version: 1,
                        id: crate::types::EpisodeId(r.id),
                        timestamp: r.created_at,
                        task_description: r.task_description,
                        initial_context: crate::types::ContextSnapshot::default(),
                        queries_made: r.queries_made,
                        files_touched: r.files_touched,
                        solution_path: r.solution_summary,
                        outcome,
                        tokens_used: crate::types::TokenCount::zero(),
                        access_count: 0,
                        pattern_value: r.success_score,
                    },
                    similarity_score: 0.5, // Default score for keyword search
                }
            })
            .collect();

        tracing::debug!("Keyword search found {} episodes", episodes.len());

        Ok(episodes)
    }

    /// Get episode by ID
    pub async fn get_episode(&self, episode_id: &str) -> Result<Option<TaskEpisode>> {
        let result: Option<EpisodeRecord> = self
            .db
            .select(("episode", episode_id))
            .await
            .context("Failed to get episode")?;

        Ok(result.map(|r| {
            let outcome = if r.success_score >= 0.9 {
                Outcome::Success
            } else if r.success_score >= 0.4 {
                Outcome::Partial
            } else {
                Outcome::Failure
            };

            TaskEpisode {
                schema_version: 1,
                id: crate::types::EpisodeId(r.id),
                timestamp: r.created_at,
                task_description: r.task_description,
                initial_context: crate::types::ContextSnapshot::default(),
                queries_made: r.queries_made,
                files_touched: r.files_touched,
                solution_path: r.solution_summary,
                outcome,
                tokens_used: crate::types::TokenCount::zero(),
                access_count: 0,
                pattern_value: r.success_score,
            }
        }))
    }

    /// Find episodes by file paths (graph traversal)
    pub async fn find_by_files(&self, file_paths: &[String], limit: usize) -> Result<Vec<TaskEpisode>> {
        #[derive(Deserialize)]
        struct FileResult {
            id: String,
            task_description: String,
            solution_summary: String,
            files_touched: Vec<String>,
            queries_made: Vec<String>,
            success_score: f32,
            created_at: DateTime<Utc>,
        }

        let query = r#"
            SELECT *
            FROM episode
            WHERE files_touched CONTAINS ANY $files
                AND success_score >= 0.5
            ORDER BY success_score DESC, created_at DESC
            LIMIT $limit
        "#;

        let mut response = self
            .db
            .query(query)
            .bind(("files", file_paths.to_vec()))
            .bind(("limit", limit))
            .await
            .context("Failed to find episodes by files")?;

        let results: Vec<FileResult> = response.take(0).unwrap_or_default();

        let episodes = results
            .into_iter()
            .map(|r| {
                let outcome = if r.success_score >= 0.9 {
                    Outcome::Success
                } else if r.success_score >= 0.4 {
                    Outcome::Partial
                } else {
                    Outcome::Failure
                };

                TaskEpisode {
                    schema_version: 1,
                    id: crate::types::EpisodeId(r.id),
                    timestamp: r.created_at,
                    task_description: r.task_description,
                    initial_context: crate::types::ContextSnapshot::default(),
                    queries_made: r.queries_made,
                    files_touched: r.files_touched,
                    solution_path: r.solution_summary,
                    outcome,
                    tokens_used: crate::types::TokenCount::zero(),
                    access_count: 0,
                    pattern_value: r.success_score,
                }
            })
            .collect();

        Ok(episodes)
    }

    /// Clean up old episodes based on retention policy
    pub async fn cleanup_old_episodes(&self) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(self.retention_days as i64);

        let query = r#"
            DELETE episode
            WHERE created_at < $cutoff
                AND success_score < 0.5
            RETURN BEFORE
        "#;

        let mut response = self
            .db
            .query(query)
            .bind(("cutoff", cutoff))
            .await
            .context("Failed to cleanup old episodes")?;

        #[derive(Deserialize)]
        struct DeletedRecord {
            id: String,
        }

        let deleted: Vec<DeletedRecord> = response.take(0).unwrap_or_default();
        let count = deleted.len();

        tracing::info!("Cleaned up {} old episodes", count);

        Ok(count)
    }

    /// Get episode statistics
    pub async fn get_statistics(&self) -> Result<EpisodeStatistics> {
        let query = r#"
            SELECT
                count() AS total_count,
                math::mean(success_score) AS avg_success_score,
                count(success_score >= 0.9) AS successful_count,
                count(success_score < 0.5) AS failed_count
            FROM episode
        "#;

        let mut response = self
            .db
            .query(query)
            .await
            .context("Failed to get episode statistics")?;

        #[derive(Deserialize)]
        struct StatsResult {
            total_count: i64,
            avg_success_score: f32,
            successful_count: i64,
            failed_count: i64,
        }

        let stats: Option<StatsResult> = response.take(0)?;

        let stats = stats.unwrap_or(StatsResult {
            total_count: 0,
            avg_success_score: 0.0,
            successful_count: 0,
            failed_count: 0,
        });

        Ok(EpisodeStatistics {
            total_episodes: stats.total_count as usize,
            successful_episodes: stats.successful_count as usize,
            failed_episodes: stats.failed_count as usize,
            average_success_score: stats.avg_success_score,
        })
    }

    /// Link episode to code symbols (graph relationship)
    /// Optimized to use batch operations instead of individual queries
    pub async fn link_to_symbols(&self, episode_id: &str, symbol_ids: &[String]) -> Result<()> {
        if symbol_ids.is_empty() {
            return Ok(());
        }

        // Build a single batch query for all symbol links
        let mut query_parts = vec!["BEGIN TRANSACTION;".to_string()];

        for symbol_id in symbol_ids {
            query_parts.push(format!(
                r#"RELATE episode:{}->references_symbol->code_symbol:{} SET reference_type = 'used_in_solution', created_at = time::now();"#,
                episode_id, symbol_id
            ));
        }

        query_parts.push("COMMIT TRANSACTION;".to_string());
        let batch_query = query_parts.join("\n");

        self.db
            .query(batch_query)
            .await
            .context("Failed to batch link episode to symbols")?;

        tracing::debug!(
            episode_id,
            symbol_count = symbol_ids.len(),
            "Batch linked episode to symbols in single transaction"
        );

        Ok(())
    }

    /// Find similar episodes using graph traversal
    /// (episodes that touched similar files or used similar symbols)
    pub async fn find_related_by_graph(&self, episode_id: &str, limit: usize) -> Result<Vec<TaskEpisode>> {
        #[derive(Deserialize)]
        struct RelatedResult {
            id: String,
            task_description: String,
            solution_summary: String,
            files_touched: Vec<String>,
            queries_made: Vec<String>,
            success_score: f32,
            created_at: DateTime<Utc>,
        }

        // Find episodes that share symbols or files with this episode
        let query = r#"
            LET $source_episode = SELECT files_touched FROM episode WHERE id = $episode_id;

            SELECT *
            FROM episode
            WHERE id != $episode_id
                AND files_touched CONTAINS ANY $source_episode.files_touched
                AND success_score >= 0.5
            ORDER BY success_score DESC
            LIMIT $limit
        "#;

        let ep_id_owned = episode_id.to_string();

        let mut response = self
            .db
            .query(query)
            .bind(("episode_id", ep_id_owned))
            .bind(("limit", limit))
            .await
            .context("Failed to find related episodes")?;

        let results: Vec<RelatedResult> = response.take(0).unwrap_or_default();

        let episodes = results
            .into_iter()
            .map(|r| {
                let outcome = if r.success_score >= 0.9 {
                    Outcome::Success
                } else if r.success_score >= 0.4 {
                    Outcome::Partial
                } else {
                    Outcome::Failure
                };

                TaskEpisode {
                    schema_version: 1,
                    id: crate::types::EpisodeId(r.id),
                    timestamp: r.created_at,
                    task_description: r.task_description,
                    initial_context: crate::types::ContextSnapshot::default(),
                    queries_made: r.queries_made,
                    files_touched: r.files_touched,
                    solution_path: r.solution_summary,
                    outcome,
                    tokens_used: crate::types::TokenCount::zero(),
                    access_count: 0,
                    pattern_value: r.success_score,
                }
            })
            .collect();

        Ok(episodes)
    }
}

/// Episode statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeStatistics {
    pub total_episodes: usize,
    pub successful_episodes: usize,
    pub failed_episodes: usize,
    pub average_success_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> (Arc<Surreal<Db>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = Surreal::new::<surrealdb::engine::local::RocksDb>(temp_dir.path())
            .await
            .unwrap();

        db.use_ns("test").use_db("test").await.unwrap();

        // Initialize schema
        let schema = include_str!("../storage/schema.surql");
        db.query(schema).await.unwrap();

        (Arc::new(db), temp_dir)
    }

    #[tokio::test]
    async fn test_record_and_find_episode() {
        let (db, _temp) = create_test_db().await;
        let memory = EpisodicMemorySurreal::new(db, 30);

        let episode = TaskEpisode {
            schema_version: 1,
            id: crate::types::EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Test authentication fix".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec!["find auth".to_string()],
            files_touched: vec!["auth.rs".to_string()],
            solution_path: "Fixed validation logic".to_string(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::new(500),
            access_count: 0,
            pattern_value: 0.9,
        };

        memory.record_episode(episode.clone(), None).await.unwrap();

        let found = memory.get_episode(&episode.id.0).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().task_description, episode.task_description);
    }

    #[tokio::test]
    async fn test_find_by_files() {
        let (db, _temp) = create_test_db().await;
        let memory = EpisodicMemorySurreal::new(db, 30);

        let episode = TaskEpisode {
            schema_version: 1,
            id: crate::types::EpisodeId::new(),
            timestamp: Utc::now(),
            task_description: "Fix auth bug".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec!["auth.rs".to_string(), "user.rs".to_string()],
            solution_path: "Fixed bug".to_string(),
            outcome: Outcome::Success,
            tokens_used: crate::types::TokenCount::new(300),
            access_count: 0,
            pattern_value: 0.8,
        };

        memory.record_episode(episode.clone(), None).await.unwrap();

        let results = memory
            .find_by_files(&["auth.rs".to_string()], 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_description, episode.task_description);
    }

    #[tokio::test]
    async fn test_cleanup_old_episodes() {
        let (db, _temp) = create_test_db().await;
        let memory = EpisodicMemorySurreal::new(db, 30);

        // Create old failed episode
        let old_episode = TaskEpisode {
            schema_version: 1,
            id: crate::types::EpisodeId::new(),
            timestamp: Utc::now() - chrono::Duration::days(40),
            task_description: "Old task".to_string(),
            initial_context: crate::types::ContextSnapshot::default(),
            queries_made: vec![],
            files_touched: vec![],
            solution_path: String::new(),
            outcome: Outcome::Failure,
            tokens_used: crate::types::TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.1,
        };

        memory.record_episode(old_episode, None).await.unwrap();

        let deleted = memory.cleanup_old_episodes().await.unwrap();
        assert_eq!(deleted, 1);
    }
}
