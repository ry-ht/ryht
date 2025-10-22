//! Search endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{ApiResponse, SearchRequest, SearchResult},
};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use cortex_memory::CognitiveManager;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;

/// Search context
#[derive(Clone)]
pub struct SearchContext {
    pub storage: Arc<ConnectionManager>,
    pub memory: Arc<CognitiveManager>,
}

/// Create search routes
pub fn search_routes(context: SearchContext) -> Router {
    Router::new()
        .route("/api/v3/search", get(search))
        .with_state(context)
}

/// GET /api/v3/search - Search across memory
async fn search(
    State(ctx): State<SearchContext>,
    Query(params): Query<SearchRequest>,
) -> ApiResult<Json<ApiResponse<Vec<SearchResult>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let search_type = params.search_type.as_deref().unwrap_or("semantic");
    let limit = params.limit.unwrap_or(20);

    let results = match search_type {
        "semantic" => search_semantic(&ctx, &params.query, limit).await?,
        "pattern" => search_pattern(&ctx, &params.query, limit).await?,
        "content" => search_content(&ctx, &params.query, limit).await?,
        _ => return Err(ApiError::BadRequest(format!("Invalid search type: {}", search_type))),
    };

    tracing::debug!(
        query = %params.query,
        search_type = search_type,
        result_count = results.len(),
        "Performed search"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(results, request_id, duration)))
}

/// Search semantic memory for code units
async fn search_semantic(
    ctx: &SearchContext,
    query: &str,
    limit: usize,
) -> ApiResult<Vec<SearchResult>> {
    // For semantic search, we would typically:
    // 1. Generate embedding for query
    // 2. Search semantic memory using embedding
    // For now, we'll do a basic text search on code units

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let search_query = format!(
        "SELECT * FROM code_unit WHERE
         name CONTAINS $query OR
         signature CONTAINS $query OR
         summary CONTAINS $query
         LIMIT $limit"
    );

    let query_owned = query.to_string();
    let mut response = conn.connection()
        .query(&search_query)
        .bind(("query", query_owned))
        .bind(("limit", limit))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let code_units: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let results = code_units
        .into_iter()
        .enumerate()
        .map(|(i, unit)| SearchResult {
            id: unit.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            title: unit.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string(),
            content: unit.get("signature")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            score: 1.0 - (i as f64 / limit as f64), // Simple scoring
            result_type: "code_unit".to_string(),
            metadata: unit,
        })
        .collect();

    Ok(results)
}

/// Search for pattern matches in code
async fn search_pattern(
    ctx: &SearchContext,
    query: &str,
    limit: usize,
) -> ApiResult<Vec<SearchResult>> {
    // Pattern search looks for specific patterns in code
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let search_query = format!(
        "SELECT * FROM learned_pattern WHERE
         pattern_name CONTAINS $query OR
         description CONTAINS $query
         LIMIT $limit"
    );

    let query_owned = query.to_string();
    let mut response = conn.connection()
        .query(&search_query)
        .bind(("query", query_owned))
        .bind(("limit", limit))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let patterns: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let results = patterns
        .into_iter()
        .enumerate()
        .map(|(i, pattern)| SearchResult {
            id: pattern.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            title: pattern.get("pattern_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed Pattern")
                .to_string(),
            content: pattern.get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            score: 1.0 - (i as f64 / limit as f64),
            result_type: "pattern".to_string(),
            metadata: pattern,
        })
        .collect();

    Ok(results)
}

/// Search file content
async fn search_content(
    ctx: &SearchContext,
    query: &str,
    limit: usize,
) -> ApiResult<Vec<SearchResult>> {
    // Content search looks in file content stored in VFS
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let search_query = format!(
        "SELECT * FROM file_content WHERE
         content CONTAINS $query
         LIMIT $limit"
    );

    let query_owned = query.to_string();
    let mut response = conn.connection()
        .query(&search_query)
        .bind(("query", query_owned))
        .bind(("limit", limit))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let files: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let results = files
        .into_iter()
        .enumerate()
        .map(|(i, file)| {
            let content_hash = file.get("content_hash")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let content = file.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Extract a snippet around the match
            let snippet = extract_snippet(content, query, 100);

            SearchResult {
                id: content_hash.to_string(),
                title: format!("File content ({})", &content_hash[..8]),
                content: snippet,
                score: 1.0 - (i as f64 / limit as f64),
                result_type: "file_content".to_string(),
                metadata: file,
            }
        })
        .collect();

    Ok(results)
}

/// Extract a snippet around the first occurrence of a query string
fn extract_snippet(content: &str, query: &str, context_chars: usize) -> String {
    if let Some(pos) = content.to_lowercase().find(&query.to_lowercase()) {
        let start = pos.saturating_sub(context_chars);
        let end = (pos + query.len() + context_chars).min(content.len());

        let mut snippet = content[start..end].to_string();

        if start > 0 {
            snippet.insert_str(0, "...");
        }
        if end < content.len() {
            snippet.push_str("...");
        }

        snippet
    } else {
        content.chars().take(context_chars * 2).collect()
    }
}
