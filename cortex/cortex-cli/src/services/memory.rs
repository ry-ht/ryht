//! Memory service layer
//!
//! Provides unified cognitive memory operations for both API and MCP modules.

use anyhow::Result;
use chrono::{DateTime, Utc};
use cortex_memory::CognitiveManager;
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// Memory service for cognitive memory operations
#[derive(Clone)]
pub struct MemoryService {
    storage: Arc<ConnectionManager>,
    cognitive_manager: Arc<CognitiveManager>,
}

impl MemoryService {
    /// Create a new memory service
    pub fn new(storage: Arc<ConnectionManager>, cognitive_manager: Arc<CognitiveManager>) -> Self {
        Self {
            storage,
            cognitive_manager,
        }
    }

    /// Store a memory episode
    pub async fn store_episode(&self, request: StoreEpisodeRequest) -> Result<EpisodeDetails> {
        info!("Storing memory episode: {}", request.task_description);

        // In a real implementation, this would use the cognitive manager
        // to store the episode with proper encoding and relationships
        let episode_id = uuid::Uuid::new_v4().to_string();

        Ok(EpisodeDetails {
            id: episode_id,
            task_description: request.task_description,
            episode_type: request.episode_type,
            outcome: request.outcome,
            importance: request.importance.unwrap_or(0.5),
            created_at: Utc::now(),
        })
    }

    /// Recall similar episodes
    pub async fn recall_episodes(&self, request: RecallEpisodesRequest) -> Result<Vec<EpisodeDetails>> {
        debug!("Recalling episodes for query: {}", request.query);

        let conn = self.storage.acquire().await?;

        let limit = request.limit.unwrap_or(20);
        let min_importance = request.min_importance.unwrap_or(0.0);

        let mut query_str = String::from(
            "SELECT
                cortex_id,
                type::string(episode_type) as episode_type,
                task_description,
                created_at,
                duration_seconds,
                type::string(outcome) as outcome,
                success_metrics
            FROM episode
            WHERE task_description CONTAINS $query"
        );

        if let Some(ref ep_type) = request.episode_type {
            query_str.push_str(" AND type::string(episode_type) = $episode_type");
        }

        query_str.push_str(" ORDER BY created_at DESC LIMIT $limit");

        let mut query = conn
            .connection()
            .query(&query_str)
            .bind(("query", request.query.clone()))
            .bind(("limit", limit));

        if let Some(ref ep_type) = request.episode_type {
            query = query.bind(("episode_type", ep_type.clone()));
        }

        let mut response = query.await?;
        let episodes_raw: Vec<serde_json::Value> = response.take(0)?;

        let episodes: Vec<EpisodeDetails> = episodes_raw
            .into_iter()
            .filter_map(|ep| {
                // Calculate importance based on success metrics
                let importance = if let Some(metrics) = ep.get("success_metrics") {
                    if let Some(obj) = metrics.as_object() {
                        obj.values()
                            .filter_map(|v| v.as_f64())
                            .sum::<f64>() / obj.len().max(1) as f64
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };

                // Filter by minimum importance
                if importance < min_importance {
                    return None;
                }

                Some(EpisodeDetails {
                    id: ep.get("cortex_id")?.as_str()?.to_string(),
                    task_description: ep.get("task_description")?.as_str()?.to_string(),
                    episode_type: ep.get("episode_type")?.as_str()?.to_string(),
                    outcome: ep.get("outcome")?.as_str()?.to_string(),
                    importance,
                    created_at: serde_json::from_value(ep.get("created_at")?.clone()).ok()?,
                })
            })
            .collect();

        Ok(episodes)
    }

    /// Get episode details
    pub async fn get_episode(&self, episode_id: &str) -> Result<Option<EpisodeDetails>> {
        debug!("Getting episode: {}", episode_id);

        let conn = self.storage.acquire().await?;

        let query = "SELECT
            cortex_id,
            type::string(episode_type) as episode_type,
            task_description,
            created_at,
            type::string(outcome) as outcome,
            success_metrics
            FROM episode
            WHERE cortex_id = $episode_id
            LIMIT 1";

        let mut response = conn
            .connection()
            .query(query)
            .bind(("episode_id", episode_id.to_string()))
            .await?;

        let episodes_raw: Vec<serde_json::Value> = response.take(0)?;

        let episode = episodes_raw.into_iter().next().map(|ep| {
            let importance = if let Some(metrics) = ep.get("success_metrics") {
                if let Some(obj) = metrics.as_object() {
                    obj.values()
                        .filter_map(|v| v.as_f64())
                        .sum::<f64>() / obj.len().max(1) as f64
                } else {
                    0.5
                }
            } else {
                0.5
            };

            EpisodeDetails {
                id: ep
                    .get("cortex_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(episode_id)
                    .to_string(),
                task_description: ep
                    .get("task_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                episode_type: ep
                    .get("episode_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("task")
                    .to_string(),
                outcome: ep
                    .get("outcome")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                importance,
                created_at: serde_json::from_value(
                    ep.get("created_at")
                        .cloned()
                        .unwrap_or(serde_json::json!(Utc::now())),
                )
                .unwrap_or_else(|_| Utc::now()),
            }
        });

        Ok(episode)
    }

    /// Consolidate memory
    pub async fn consolidate(&self) -> Result<ConsolidationResult> {
        info!("Running memory consolidation");

        let result = self.cognitive_manager.consolidate().await?;

        Ok(ConsolidationResult {
            episodes_processed: result.episodes_processed,
            patterns_extracted: result.patterns_extracted,
            memories_decayed: result.memories_decayed,
            duplicates_merged: result.duplicates_merged,
            knowledge_links_created: result.knowledge_links_created,
            duration_ms: result.duration_ms,
        })
    }

    /// Get learned patterns
    pub async fn get_patterns(&self, filters: PatternFilters) -> Result<Vec<PatternDetails>> {
        debug!("Getting learned patterns with filters: {:?}", filters);

        let conn = self.storage.acquire().await?;

        let mut query_str = String::from(
            "SELECT
                id,
                pattern_name,
                description,
                type::string(pattern_type) as pattern_type,
                occurrences,
                confidence,
                created_at,
                last_seen
            FROM learned_pattern
            WHERE 1=1"
        );

        if let Some(ref pattern_type) = filters.pattern_type {
            query_str.push_str(&format!(" AND type::string(pattern_type) = '{}'", pattern_type));
        }

        if let Some(min_confidence) = filters.min_confidence {
            query_str.push_str(&format!(" AND confidence >= {}", min_confidence));
        }

        query_str.push_str(" ORDER BY confidence DESC, occurrences DESC");

        if let Some(limit) = filters.limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }

        let mut response = conn.connection().query(&query_str).await?;
        let patterns_raw: Vec<serde_json::Value> = response.take(0)?;

        let patterns = patterns_raw
            .into_iter()
            .filter_map(|p| {
                Some(PatternDetails {
                    id: p.get("id")?.as_str()?.to_string(),
                    pattern_name: p.get("pattern_name")?.as_str()?.to_string(),
                    description: p.get("description")?.as_str().unwrap_or("").to_string(),
                    pattern_type: p.get("pattern_type")?.as_str()?.to_string(),
                    occurrences: p.get("occurrences")?.as_u64()? as usize,
                    confidence: p.get("confidence")?.as_f64()?,
                    created_at: serde_json::from_value(p.get("created_at")?.clone()).ok()?,
                    last_seen: serde_json::from_value(p.get("last_seen")?.clone()).ok()?,
                })
            })
            .collect();

        Ok(patterns)
    }

    /// Get context for a task
    pub async fn get_context(&self, request: GetContextRequest) -> Result<ContextDetails> {
        debug!("Getting context for: {}", request.description);

        // Recall relevant episodes
        let episodes = self
            .recall_episodes(RecallEpisodesRequest {
                query: request.description.clone(),
                episode_type: None,
                limit: Some(5),
                min_importance: Some(0.6),
            })
            .await?;

        // Get relevant patterns
        let patterns = self
            .get_patterns(PatternFilters {
                pattern_type: None,
                min_confidence: Some(0.7),
                limit: Some(5),
            })
            .await?;

        Ok(ContextDetails {
            relevant_episodes: episodes,
            relevant_patterns: patterns,
            context_score: 0.8, // Simplified scoring
        })
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct StoreEpisodeRequest {
    pub task_description: String,
    pub episode_type: String,
    pub outcome: String,
    pub importance: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecallEpisodesRequest {
    pub query: String,
    pub episode_type: Option<String>,
    pub limit: Option<usize>,
    pub min_importance: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetContextRequest {
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct PatternFilters {
    pub pattern_type: Option<String>,
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeDetails {
    pub id: String,
    pub task_description: String,
    pub episode_type: String,
    pub outcome: String,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternDetails {
    pub id: String,
    pub pattern_name: String,
    pub description: String,
    pub pattern_type: String,
    pub occurrences: usize,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationResult {
    pub episodes_processed: usize,
    pub patterns_extracted: usize,
    pub memories_decayed: usize,
    pub duplicates_merged: usize,
    pub knowledge_links_created: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextDetails {
    pub relevant_episodes: Vec<EpisodeDetails>,
    pub relevant_patterns: Vec<PatternDetails>,
    pub context_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_details_serialization() {
        let episode = EpisodeDetails {
            id: "test-id".to_string(),
            task_description: "Test task".to_string(),
            episode_type: "development".to_string(),
            outcome: "success".to_string(),
            importance: 0.8,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&episode).unwrap();
        assert!(json.contains("Test task"));
    }

    #[test]
    fn test_pattern_details_serialization() {
        let pattern = PatternDetails {
            id: "pattern-id".to_string(),
            pattern_name: "Test Pattern".to_string(),
            description: "A test pattern".to_string(),
            pattern_type: "code".to_string(),
            occurrences: 10,
            confidence: 0.9,
            created_at: Utc::now(),
            last_seen: Utc::now(),
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("Test Pattern"));
    }

    #[test]
    fn test_consolidation_result_serialization() {
        let result = ConsolidationResult {
            episodes_processed: 100,
            patterns_extracted: 10,
            memories_decayed: 5,
            duplicates_merged: 3,
            knowledge_links_created: 20,
            duration_ms: 1500,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"episodes_processed\":100"));
    }
}
