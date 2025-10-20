//! Procedural memory for learned patterns and procedures.

use crate::types::*;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use tracing::info;

/// Procedural memory system for storing and applying learned patterns
pub struct ProceduralMemorySystem {
    connection_manager: Arc<ConnectionManager>,
}

impl ProceduralMemorySystem {
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    /// Store a learned pattern
    pub async fn store_pattern(&self, pattern: &LearnedPattern) -> Result<CortexId> {
        info!(pattern_id = %pattern.id, name = %pattern.name, "Storing learned pattern");

        let conn = self.connection_manager.acquire().await?;

        let query = "
            CREATE pattern CONTENT {
                id: $id, pattern_type: $pattern_type, name: $name,
                description: $description, context: $context,
                before_state: $before_state, after_state: $after_state,
                transformation: $transformation, times_applied: $times_applied,
                success_rate: $success_rate, average_improvement: $average_improvement,
                example_episodes: $example_episodes, embedding: $embedding,
                created_at: $created_at, updated_at: $updated_at
            }
        ";

        conn.connection().query(query)
            .bind(("id", pattern.id.to_string()))
            .bind(("pattern_type", pattern.pattern_type))
            .bind(("name", pattern.name.clone()))
            .bind(("description", pattern.description.clone()))
            .bind(("context", pattern.context.clone()))
            .bind(("before_state", pattern.before_state.clone()))
            .bind(("after_state", pattern.after_state.clone()))
            .bind(("transformation", pattern.transformation.clone()))
            .bind(("times_applied", pattern.times_applied))
            .bind(("success_rate", pattern.success_rate))
            .bind(("average_improvement", pattern.average_improvement.clone()))
            .bind(("example_episodes", pattern.example_episodes.clone()))
            .bind(("embedding", pattern.embedding.clone()))
            .bind(("created_at", pattern.created_at))
            .bind(("updated_at", pattern.updated_at))
            .await
            .map_err(|e| CortexError::database(e.to_string()))?;

        Ok(pattern.id)
    }

    /// Retrieve a pattern by ID
    pub async fn get_pattern(&self, id: CortexId) -> Result<Option<LearnedPattern>> {
        let conn = self.connection_manager.acquire().await?;
        let mut result = conn
            .connection()
            .query("SELECT * FROM pattern WHERE id = $id")
            .bind(("id", id.to_string()))
            .await
            .map_err(|e| CortexError::database(e.to_string()))?;
        Ok(result.take(0).map_err(|e| CortexError::database(e.to_string()))?)
    }

    /// Search for similar patterns using embeddings
    pub async fn search_patterns(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<LearnedPattern>>> {
        info!(query = %query.query_text, "Searching for similar patterns");

        let conn = self.connection_manager.acquire().await?;

        let query_str = "
            SELECT *, vector::distance::cosine(embedding, $query_embedding) AS similarity
            FROM pattern
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
            .map_err(|e| CortexError::database(e.to_string()))?;

        let patterns: Vec<(LearnedPattern, f32)> = result.take(0)
            .map_err(|e| CortexError::database(e.to_string()))?;
        Ok(patterns
            .into_iter()
            .map(|(pattern, similarity)| MemorySearchResult {
                item: pattern.clone(),
                similarity_score: 1.0 - similarity,
                relevance_score: pattern.success_rate,
            })
            .collect())
    }

    /// Record a successful pattern application
    pub async fn record_success(&self, pattern_id: CortexId) -> Result<()> {
        let conn = self.connection_manager.acquire().await?;

        conn.connection().query("
            UPDATE pattern SET
                times_applied = times_applied + 1,
                success_rate = (success_rate * times_applied + 1) / (times_applied + 1),
                updated_at = time::now()
            WHERE id = $id
        ")
        .bind(("id", pattern_id.to_string()))
        .await
        .map_err(|e| CortexError::database(e.to_string()))?;

        Ok(())
    }

    /// Get statistics about procedural memory
    pub async fn get_statistics(&self) -> Result<ProceduralStats> {
        let conn = self.connection_manager.acquire().await?;

        let query = "
            SELECT count() AS total, math::mean(success_rate) AS avg_rate,
                   math::sum(times_applied) AS total_apps
            FROM pattern GROUP ALL
        ";

        let mut result = conn.connection().query(query).await
            .map_err(|e| CortexError::database(e.to_string()))?;
        let stats: Option<serde_json::Value> = result.take(0)
            .map_err(|e| CortexError::database(e.to_string()))?;

        if let Some(stats) = stats {
            Ok(ProceduralStats {
                total_patterns: stats["total"].as_u64().unwrap_or(0),
                average_success_rate: stats["avg_rate"].as_f64().unwrap_or(0.0) as f32,
                total_applications: stats["total_apps"].as_u64().unwrap_or(0),
            })
        } else {
            Ok(ProceduralStats {
                total_patterns: 0,
                average_success_rate: 0.0,
                total_applications: 0,
            })
        }
    }
}
