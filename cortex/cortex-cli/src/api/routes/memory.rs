//! Memory management endpoints

use crate::api::{
    error::ApiResult,
    types::{
        ApiResponse, ConsolidateMemoryRequest, MemoryEpisode,
        EpisodeSearchRequest, LearnedPattern,
    },
};
use crate::services::MemoryService;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;

/// Memory context
#[derive(Clone)]
pub struct MemoryContext {
    pub memory_service: Arc<MemoryService>,
}

/// Create memory routes
pub fn memory_routes(context: MemoryContext) -> Router {
    Router::new()
        .route("/api/v1/memory/episodes", get(list_episodes))
        .route("/api/v1/memory/consolidate", post(consolidate_memory))
        .route("/api/v1/memory/episodes/{episode_id}", get(get_episode))
        .route("/api/v1/memory/search", post(search_episodes))
        .route("/api/v1/memory/patterns", get(get_patterns))
        .with_state(context)
}

/// GET /api/v1/memory/episodes - List memory episodes
async fn list_episodes(
    State(ctx): State<MemoryContext>,
) -> ApiResult<Json<ApiResponse<Vec<MemoryEpisode>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use MemoryService to recall recent episodes
    let service_request = crate::services::memory::RecallEpisodesRequest {
        query: String::new(), // Empty query to get all
        episode_type: None,
        limit: Some(100),
        min_importance: None,
    };

    let service_episodes = ctx.memory_service
        .recall_episodes(service_request)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert service episodes to API episodes
    let episodes: Vec<MemoryEpisode> = service_episodes
        .into_iter()
        .map(|ep| MemoryEpisode {
            id: ep.id,
            content: ep.task_description,
            episode_type: ep.episode_type,
            importance: ep.importance,
            created_at: ep.created_at,
        })
        .collect();

    tracing::debug!(count = episodes.len(), "Listed memory episodes");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(episodes, request_id, duration)))
}

/// GET /api/v1/memory/episodes/:episode_id - Get episode details
async fn get_episode(
    State(ctx): State<MemoryContext>,
    Path(episode_id): Path<String>,
) -> ApiResult<Json<ApiResponse<MemoryEpisode>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use MemoryService to get episode
    let service_episode = ctx.memory_service
        .get_episode(&episode_id)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?
        .ok_or_else(|| crate::api::error::ApiError::NotFound(format!("Episode {} not found", episode_id)))?;

    // Convert service episode to API episode
    let episode = MemoryEpisode {
        id: service_episode.id,
        content: service_episode.task_description,
        episode_type: service_episode.episode_type,
        importance: service_episode.importance,
        created_at: service_episode.created_at,
    };

    tracing::debug!(episode_id = %episode_id, "Retrieved episode details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(episode, request_id, duration)))
}

/// POST /api/v1/memory/consolidate - Consolidate memory
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

    // Use MemoryService to consolidate memory
    let consolidation_result = ctx.memory_service
        .consolidate()
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

/// POST /api/v1/memory/search - Search similar episodes using embeddings
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

    // Use MemoryService to recall episodes
    let service_request = crate::services::memory::RecallEpisodesRequest {
        query: payload.query.clone(),
        episode_type: payload.episode_type.clone(),
        limit: Some(limit),
        min_importance: Some(min_importance),
    };

    let service_episodes = ctx.memory_service
        .recall_episodes(service_request)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert service episodes to API episodes
    let episodes: Vec<MemoryEpisode> = service_episodes
        .into_iter()
        .map(|ep| MemoryEpisode {
            id: ep.id,
            content: ep.task_description,
            episode_type: ep.episode_type,
            importance: ep.importance,
            created_at: ep.created_at,
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

/// GET /api/v1/memory/patterns - Get learned patterns from memory
async fn get_patterns(
    State(ctx): State<MemoryContext>,
) -> ApiResult<Json<ApiResponse<Vec<LearnedPattern>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::debug!("Fetching learned patterns from memory");

    // Use MemoryService to get patterns
    let filters = crate::services::memory::PatternFilters {
        pattern_type: None,
        min_confidence: None,
        limit: Some(100),
    };

    let service_patterns = ctx.memory_service
        .get_patterns(filters)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Convert service patterns to API patterns
    let patterns: Vec<LearnedPattern> = service_patterns
        .into_iter()
        .map(|p| LearnedPattern {
            id: p.id,
            pattern_name: p.pattern_name,
            description: p.description,
            pattern_type: p.pattern_type,
            occurrences: p.occurrences,
            confidence: p.confidence,
            created_at: p.created_at,
            last_seen: p.last_seen,
            examples: Vec::new(), // Service type doesn't include examples yet
        })
        .collect();

    tracing::debug!(
        patterns_count = patterns.len(),
        "Retrieved learned patterns"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(patterns, request_id, duration)))
}
