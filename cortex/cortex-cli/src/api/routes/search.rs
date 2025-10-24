//! Search endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{
        ApiResponse, SearchRequest, SearchResult,
        ReferencesResponse, CodeReference, PatternSearchRequest,
        PatternSearchResponse, PatternMatch,
    },
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
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
        .route("/api/v1/search", get(search))
        .route("/api/v1/search/references/{unit_id}", get(find_references))
        .route("/api/v1/search/pattern", post(search_pattern))
        .with_state(context)
}

/// GET /api/v1/search - Search across memory
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
        "pattern" => search_pattern_helper(&ctx, &params.query, limit).await?,
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

/// Search for pattern matches in code (helper function for the general search endpoint)
async fn search_pattern_helper(
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

/// GET /api/v1/search/references/{unit_id} - Find references to a code unit
async fn find_references(
    State(ctx): State<SearchContext>,
    Path(unit_id): Path<String>,
) -> ApiResult<Json<ApiResponse<ReferencesResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Get the code unit details first
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let unit_query = "SELECT * FROM code_unit WHERE id = $unit_id LIMIT 1";
    let mut unit_response = conn.connection()
        .query(unit_query)
        .bind(("unit_id", unit_id.clone()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let units: Vec<serde_json::Value> = unit_response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let unit = units.into_iter().next()
        .ok_or_else(|| ApiError::NotFound(format!("Code unit {} not found", unit_id)))?;

    let unit_name = unit.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    // Search for references to this unit
    // In a real implementation, this would query a references table or graph
    let references_query = "SELECT * FROM code_reference WHERE target_unit_id = $unit_id LIMIT 100";
    let mut ref_response = conn.connection()
        .query(references_query)
        .bind(("unit_id", unit_id.clone()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let references_raw: Vec<serde_json::Value> = ref_response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let references: Vec<CodeReference> = references_raw
        .into_iter()
        .filter_map(|r| {
            Some(CodeReference {
                id: r.get("id")?.as_str()?.to_string(),
                file_path: r.get("file_path")?.as_str()?.to_string(),
                line: r.get("line")?.as_u64()? as usize,
                column: r.get("column")?.as_u64().unwrap_or(0) as usize,
                reference_type: r.get("reference_type")?.as_str()?.to_string(),
                context: r.get("context")?.as_str().unwrap_or("").to_string(),
                referencing_unit: r.get("referencing_unit_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .collect();

    let total_references = references.len();

    tracing::debug!(
        unit_id = %unit_id,
        unit_name = %unit_name,
        references = total_references,
        "Found references to code unit"
    );

    let response = ReferencesResponse {
        unit_id,
        unit_name,
        total_references,
        references,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// POST /api/v1/search/pattern - Search using tree-sitter patterns
async fn search_pattern(
    State(ctx): State<SearchContext>,
    Json(payload): Json<PatternSearchRequest>,
) -> ApiResult<Json<ApiResponse<PatternSearchResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let limit = payload.limit.unwrap_or(50);

    tracing::debug!(
        workspace_id = %payload.workspace_id,
        pattern = %payload.pattern,
        language = ?payload.language,
        "Searching for AST pattern"
    );

    // In a real implementation, this would:
    // 1. Parse the tree-sitter pattern
    // 2. Query the AST database for matching nodes
    // 3. Return structured results with context

    // For now, we'll do a simpler text-based search as a placeholder
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut search_query = String::from(
        "SELECT * FROM code_unit WHERE (
            signature CONTAINS $pattern OR
            body CONTAINS $pattern
         )"
    );

    if let Some(ref _lang) = payload.language {
        search_query.push_str(" AND language = $language");
    }

    search_query.push_str(" LIMIT $limit");

    let mut query = conn.connection()
        .query(&search_query)
        .bind(("pattern", payload.pattern.clone()));

    if let Some(ref lang) = payload.language {
        query = query.bind(("language", lang.clone()));
    }

    let mut response = query
        .bind(("limit", limit))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let units: Vec<serde_json::Value> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let matches: Vec<PatternMatch> = units
        .into_iter()
        .filter_map(|unit| {
            let file_path = unit.get("file_path")?.as_str()?.to_string();
            let start_line = unit.get("start_line")?.as_u64()? as usize;
            let start_column = unit.get("start_column")?.as_u64().unwrap_or(0) as usize;
            let body = unit.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let signature = unit.get("signature").and_then(|v| v.as_str()).unwrap_or("");

            // Find the matched text
            let matched_text = if body.contains(&payload.pattern) {
                extract_snippet(body, &payload.pattern, 50)
            } else if signature.contains(&payload.pattern) {
                extract_snippet(signature, &payload.pattern, 50)
            } else {
                payload.pattern.clone()
            };

            Some(PatternMatch {
                file_path,
                line: start_line,
                column: start_column,
                matched_text: matched_text.clone(),
                context: format!("{}\n{}", signature, matched_text),
                unit_id: unit.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
        })
        .collect();

    let total_matches = matches.len();

    tracing::debug!(
        pattern = %payload.pattern,
        matches = total_matches,
        "Pattern search completed"
    );

    let response = PatternSearchResponse {
        pattern: payload.pattern,
        total_matches,
        matches,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}
