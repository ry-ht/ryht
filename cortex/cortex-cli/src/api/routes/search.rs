//! Search endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{
        ApiResponse, SearchRequest, SearchResult,
        ReferencesResponse, CodeReference, PatternSearchRequest,
        PatternSearchResponse, PatternMatch,
    },
};
use crate::services::SearchService;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;

/// Search context
#[derive(Clone)]
pub struct SearchContext {
    pub search_service: Arc<SearchService>,
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

    let results: Vec<SearchResult> = match search_type {
        "semantic" => {
            // Use SearchService for semantic search
            let service_request = crate::services::search::SearchCodeRequest {
                query: params.query.clone(),
                limit,
                min_similarity: 0.5,
                language: None,
            };

            let service_results = ctx.search_service
                .search_code(service_request)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Convert service results to API results
            service_results.into_iter().map(|r| SearchResult {
                id: r.id,
                title: r.title,
                content: r.content,
                score: r.score as f64,
                result_type: r.result_type,
                metadata: serde_json::to_value(r.metadata).unwrap_or_default(),
            }).collect()
        },
        "pattern" | "content" => {
            // Use text search for non-semantic queries
            let search_type_str = if search_type == "pattern" { "patterns" } else { "code_units" };
            let service_request = crate::services::search::TextSearchRequest {
                query: params.query.clone(),
                search_type: search_type_str.to_string(),
                limit,
            };

            let service_results = ctx.search_service
                .search_text(service_request)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Convert service results to API results
            service_results.into_iter().map(|r| SearchResult {
                id: r.id,
                title: r.title,
                content: r.content,
                score: r.score as f64,
                result_type: r.result_type,
                metadata: serde_json::json!({
                    "file_path": r.file_path,
                    "language": r.language,
                }),
            }).collect()
        },
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

/// GET /api/v1/search/references/{unit_id} - Find references to a code unit
async fn find_references(
    State(ctx): State<SearchContext>,
    Path(unit_id): Path<String>,
) -> ApiResult<Json<ApiResponse<ReferencesResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use SearchService to find references
    let service_references = ctx.search_service
        .find_references(&unit_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::NotFound(format!("Code unit {} not found", unit_id))
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

    // Convert service references to API references
    let references: Vec<CodeReference> = service_references
        .into_iter()
        .map(|r| CodeReference {
            id: r.id,
            file_path: r.file_path,
            line: r.line,
            column: r.column,
            reference_type: r.reference_type,
            context: r.context,
            referencing_unit: None, // Not included in service type
        })
        .collect();

    let total_references = references.len();

    tracing::debug!(
        unit_id = %unit_id,
        references = total_references,
        "Found references to code unit"
    );

    let response = ReferencesResponse {
        unit_id: unit_id.clone(),
        unit_name: "Unit".to_string(), // We'd need to query for this separately
        total_references,
        references,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// Helper function to extract a snippet around a matched pattern
fn extract_snippet(text: &str, pattern: &str, context_chars: usize) -> String {
    if let Some(pos) = text.find(pattern) {
        let start = pos.saturating_sub(context_chars);
        let end = (pos + pattern.len() + context_chars).min(text.len());
        let snippet = &text[start..end];

        // Add ellipsis if we're not at the start/end
        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text.len() { "..." } else { "" };

        format!("{}{}{}", prefix, snippet, suffix)
    } else {
        // If pattern not found, return first part of text
        let end = context_chars.min(text.len());
        let suffix = if end < text.len() { "..." } else { "" };
        format!("{}{}", &text[..end], suffix)
    }
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

    // For now, use the search service to find pattern matches
    let service_request = crate::services::search::TextSearchRequest {
        query: payload.pattern.clone(),
        search_type: "patterns".to_string(),
        limit,
    };

    let service_results = ctx.search_service
        .search_text(service_request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let matches: Vec<PatternMatch> = service_results
        .into_iter()
        .map(|r| {
            // Extract snippet from content
            let matched_text = extract_snippet(&r.content, &payload.pattern, 50);

            PatternMatch {
                file_path: r.file_path.unwrap_or_default(),
                line: 0, // Would need line info from service
                column: 0,
                matched_text: matched_text.clone(),
                context: r.content,
                unit_id: Some(r.id),
            }
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
