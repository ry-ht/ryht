//! Memory management endpoints

use crate::api::{
    error::ApiResult,
    types::{
        ApiResponse, ConsolidateMemoryRequest, MemoryEpisode,
        EpisodeSearchRequest, LearnedPattern,
    },
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use cortex_memory::CognitiveManager;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;

/// Memory context
#[derive(Clone)]
pub struct MemoryContext {
    pub storage: Arc<ConnectionManager>,
    pub memory: Arc<CognitiveManager>,
}

/// Create memory routes
pub fn memory_routes(context: MemoryContext) -> Router {
    Router::new()
        .route("/api/v3/memory/episodes", get(list_episodes))
        .route("/api/v3/memory/consolidate", post(consolidate_memory))
        .route("/api/v3/memory/episodes/:episode_id", get(get_episode))
        .route("/api/v3/memory/search", post(search_episodes))
        .route("/api/v3/memory/patterns", get(get_patterns))
        .with_state(context)
}

/// GET /api/v3/memory/episodes - List memory episodes
async fn list_episodes(
    State(ctx): State<MemoryContext>,
) -> ApiResult<Json<ApiResponse<Vec<MemoryEpisode>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query episodes from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let query = "SELECT
        cortex_id,
        type::string(episode_type) as episode_type,
        task_description,
        created_at,
        duration_seconds,
        type::string(outcome) as outcome,
        success_metrics
        FROM episode
        ORDER BY created_at DESC
        LIMIT 100";

    let mut response = conn.connection()
        .query(query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let episodes_raw: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert to API response format
    let episodes: Vec<MemoryEpisode> = episodes_raw
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

            Some(MemoryEpisode {
                id: ep.get("cortex_id")?.as_str()?.to_string(),
                content: ep.get("task_description")?.as_str()?.to_string(),
                episode_type: ep.get("episode_type")?.as_str()?.to_string(),
                importance,
                created_at: serde_json::from_value(ep.get("created_at")?.clone()).ok()?,
            })
        })
        .collect();

    tracing::debug!(count = episodes.len(), "Listed memory episodes");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(episodes, request_id, duration)))
}

/// GET /api/v3/memory/episodes/:episode_id - Get episode details
async fn get_episode(
    State(ctx): State<MemoryContext>,
    Path(episode_id): Path<String>,
) -> ApiResult<Json<ApiResponse<MemoryEpisode>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query specific episode from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let query = "SELECT
        cortex_id,
        type::string(episode_type) as episode_type,
        task_description,
        created_at,
        duration_seconds,
        type::string(outcome) as outcome,
        success_metrics,
        solution_summary,
        errors_encountered,
        lessons_learned
        FROM episode
        WHERE cortex_id = $episode_id
        LIMIT 1";

    let mut response = conn.connection()
        .query(query)
        .bind(("episode_id", episode_id.clone()))
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let episodes_raw: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let episode_json = episodes_raw.into_iter().next()
        .ok_or_else(|| crate::api::error::ApiError::NotFound(format!("Episode {} not found", episode_id)))?;

    // Calculate importance
    let importance = if let Some(metrics) = episode_json.get("success_metrics") {
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

    // Build detailed content from multiple fields
    let mut content_parts = vec![
        episode_json.get("task_description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    ];

    if let Some(summary) = episode_json.get("solution_summary").and_then(|v| v.as_str()) {
        if !summary.is_empty() {
            content_parts.push(format!("Solution: {}", summary));
        }
    }

    if let Some(lessons) = episode_json.get("lessons_learned").and_then(|v| v.as_array()) {
        if !lessons.is_empty() {
            let lessons_str: Vec<String> = lessons
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !lessons_str.is_empty() {
                content_parts.push(format!("Lessons: {}", lessons_str.join(", ")));
            }
        }
    }

    let episode = MemoryEpisode {
        id: episode_json.get("cortex_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&episode_id)
            .to_string(),
        content: content_parts.join("\n\n"),
        episode_type: episode_json.get("episode_type")
            .and_then(|v| v.as_str())
            .unwrap_or("task")
            .to_string(),
        importance,
        created_at: serde_json::from_value(
            episode_json.get("created_at")
                .cloned()
                .unwrap_or(serde_json::json!(chrono::Utc::now()))
        ).unwrap_or_else(|_| chrono::Utc::now()),
    };

    tracing::debug!(episode_id = %episode_id, "Retrieved episode details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(episode, request_id, duration)))
}

/// POST /api/v3/memory/consolidate - Consolidate memory
async fn consolidate_memory(
    State(ctx): State<MemoryContext>,
    Json(payload): Json<ConsolidateMemoryRequest>,
) -> ApiResult<Json<ApiResponse<serde_json::Value>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::info!(
        workspace_id = ?payload.workspace_id,
        "Starting memory consolidation"
    );

    // Run memory consolidation using the cognitive manager
    let consolidation_result = ctx.memory.consolidate()
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(format!("Consolidation failed: {}", e)))?;

    // Build response with consolidation statistics
    let response_data = serde_json::json!({
        "episodes_processed": consolidation_result.episodes_processed,
        "patterns_extracted": consolidation_result.patterns_extracted,
        "memories_decayed": consolidation_result.memories_decayed,
        "duplicates_merged": consolidation_result.duplicates_merged,
        "knowledge_links_created": consolidation_result.knowledge_links_created,
        "duration_ms": consolidation_result.duration_ms,
        "timestamp": chrono::Utc::now(),
    });

    tracing::info!(
        patterns = consolidation_result.patterns_extracted,
        knowledge_links = consolidation_result.knowledge_links_created,
        episodes = consolidation_result.episodes_processed,
        "Memory consolidation completed"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response_data, request_id, duration)))
}

/// POST /api/v3/memory/search - Search similar episodes using embeddings
async fn search_episodes(
    State(ctx): State<MemoryContext>,
    Json(payload): Json<EpisodeSearchRequest>,
) -> ApiResult<Json<ApiResponse<Vec<MemoryEpisode>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let limit = payload.limit.unwrap_or(20);
    let min_importance = payload.min_importance.unwrap_or(0.0);

    tracing::debug!(
        query = %payload.query,
        episode_type = ?payload.episode_type,
        min_importance = min_importance,
        limit = limit,
        "Searching for similar episodes"
    );

    // In a real implementation, this would:
    // 1. Generate embedding for the query
    // 2. Search semantic memory using vector similarity
    // 3. Return ranked results by similarity score

    // For now, we'll do a text-based search
    let conn = ctx.storage.acquire().await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

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

    if let Some(ref ep_type) = payload.episode_type {
        query_str.push_str(" AND type::string(episode_type) = $episode_type");
    }

    query_str.push_str(" ORDER BY created_at DESC LIMIT $limit");

    let mut query = conn.connection()
        .query(&query_str)
        .bind(("query", payload.query.clone()))
        .bind(("limit", limit));

    if let Some(ref ep_type) = payload.episode_type {
        query = query.bind(("episode_type", ep_type.clone()));
    }

    let mut response = query
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let episodes_raw: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert to API response format and filter by importance
    let episodes: Vec<MemoryEpisode> = episodes_raw
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

            Some(MemoryEpisode {
                id: ep.get("cortex_id")?.as_str()?.to_string(),
                content: ep.get("task_description")?.as_str()?.to_string(),
                episode_type: ep.get("episode_type")?.as_str()?.to_string(),
                importance,
                created_at: serde_json::from_value(ep.get("created_at")?.clone()).ok()?,
            })
        })
        .collect();

    tracing::debug!(
        query = %payload.query,
        results = episodes.len(),
        "Episode search completed"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(episodes, request_id, duration)))
}

/// GET /api/v3/memory/patterns - Get learned patterns from memory
async fn get_patterns(
    State(ctx): State<MemoryContext>,
) -> ApiResult<Json<ApiResponse<Vec<LearnedPattern>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::debug!("Fetching learned patterns from memory");

    // Query learned patterns from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let query = "SELECT
        id,
        pattern_name,
        description,
        type::string(pattern_type) as pattern_type,
        occurrences,
        confidence,
        created_at,
        last_seen,
        examples
        FROM learned_pattern
        ORDER BY confidence DESC, occurrences DESC
        LIMIT 100";

    let mut response = conn.connection()
        .query(query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let patterns_raw: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert to API response format
    let patterns: Vec<LearnedPattern> = patterns_raw
        .into_iter()
        .filter_map(|p| {
            let examples = if let Some(examples_val) = p.get("examples") {
                if let Some(arr) = examples_val.as_array() {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            Some(LearnedPattern {
                id: p.get("id")?.as_str()?.to_string(),
                pattern_name: p.get("pattern_name")?.as_str()?.to_string(),
                description: p.get("description")?.as_str().unwrap_or("").to_string(),
                pattern_type: p.get("pattern_type")?.as_str()?.to_string(),
                occurrences: p.get("occurrences")?.as_u64()? as usize,
                confidence: p.get("confidence")?.as_f64()?,
                created_at: serde_json::from_value(p.get("created_at")?.clone()).ok()?,
                last_seen: serde_json::from_value(p.get("last_seen")?.clone()).ok()?,
                examples,
            })
        })
        .collect();

    tracing::debug!(
        patterns_count = patterns.len(),
        "Retrieved learned patterns"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(patterns, request_id, duration)))
}
