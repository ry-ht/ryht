//! Code Units API routes

use crate::api::types::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use cortex_core::types::CodeUnit;
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// Context for code unit routes
#[derive(Clone)]
pub struct CodeUnitContext {
    pub storage: Arc<ConnectionManager>,
    pub vfs: Arc<VirtualFileSystem>,
}

/// Create code unit routes
pub fn code_unit_routes(context: CodeUnitContext) -> Router {
    Router::new()
        .route("/api/v1/workspaces/:id/units", get(list_code_units))
        .route("/api/v1/units/:id", get(get_code_unit))
        .route("/api/v1/units/:id", put(update_code_unit))
        .with_state(context)
}

/// GET /api/v1/workspaces/{id}/units - List code units in workspace
async fn list_code_units(
    State(context): State<CodeUnitContext>,
    Path(workspace_id): Path<String>,
    Query(params): Query<CodeUnitListRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        workspace_id = %workspace_id,
        "Listing code units"
    );

    match list_code_units_impl(&context, &workspace_id, params).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to list code units");
            let api_response = ApiResponse::<CodeUnitListResponse>::error(
                e.to_string(),
                request_id,
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn list_code_units_impl(
    context: &CodeUnitContext,
    workspace_id: &str,
    params: CodeUnitListRequest,
) -> anyhow::Result<CodeUnitListResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    // Build query filters
    let limit = params.limit.min(1000);
    let offset = params.offset.unwrap_or(0);

    let mut query = format!(
        "SELECT * FROM code_unit WHERE file_path CONTAINS '{}'",
        workspace_id
    );

    // Apply filters
    if let Some(unit_type) = &params.unit_type {
        query.push_str(&format!(" AND unit_type = '{}'", unit_type));
    }
    if let Some(visibility) = &params.visibility {
        query.push_str(&format!(" AND visibility = '{}'", visibility));
    }
    if let Some(language) = &params.language {
        query.push_str(&format!(" AND language = '{}'", language));
    }
    if let Some(min_complexity) = params.min_complexity {
        query.push_str(&format!(
            " AND complexity.cyclomatic >= {}",
            min_complexity
        ));
    }
    if let Some(max_complexity) = params.max_complexity {
        query.push_str(&format!(
            " AND complexity.cyclomatic <= {}",
            max_complexity
        ));
    }
    if let Some(has_tests) = params.has_tests {
        query.push_str(&format!(" AND has_tests = {}", has_tests));
    }
    if let Some(has_docs) = params.has_docs {
        query.push_str(&format!(" AND has_documentation = {}", has_docs));
    }

    query.push_str(&format!(" LIMIT {} START {}", limit, offset));

    // Execute query
    let mut result = conn.query(&query).await?;
    let units: Vec<CodeUnit> = result.take(0)?;

    // Get total count
    let count_query = format!(
        "SELECT count() FROM code_unit WHERE file_path CONTAINS '{}' GROUP ALL",
        workspace_id
    );
    let mut count_result = conn.query(&count_query).await?;
    let total: usize = count_result
        .take::<Option<usize>>(0)?
        .unwrap_or(units.len());

    // Convert to response format
    let unit_responses: Vec<CodeUnitResponse> = units
        .into_iter()
        .map(|unit| {
            let complexity_score = unit.complexity_score();
            CodeUnitResponse {
                id: unit.id.to_string(),
                unit_type: format!("{:?}", unit.unit_type).to_lowercase(),
                name: unit.name,
                qualified_name: unit.qualified_name,
                display_name: unit.display_name,
                file_path: unit.file_path,
                language: format!("{:?}", unit.language).to_lowercase(),
                start_line: unit.start_line,
                end_line: unit.end_line,
                start_column: unit.start_column,
                end_column: unit.end_column,
                signature: unit.signature,
                body: unit.body,
                docstring: unit.docstring,
                visibility: format!("{:?}", unit.visibility).to_lowercase(),
                is_async: unit.is_async,
                is_exported: unit.is_exported,
                complexity: ComplexityResponse {
                    cyclomatic: unit.complexity.cyclomatic,
                    cognitive: unit.complexity.cognitive,
                    nesting: unit.complexity.nesting,
                    lines: unit.complexity.lines,
                    score: complexity_score,
                },
                has_tests: unit.has_tests,
                has_documentation: unit.has_documentation,
                created_at: unit.created_at,
                updated_at: unit.updated_at,
            }
        })
        .collect();

    Ok(CodeUnitListResponse {
        units: unit_responses,
        total,
        limit,
        offset,
    })
}

/// GET /api/v1/units/{id} - Get code unit details
async fn get_code_unit(
    State(context): State<CodeUnitContext>,
    Path(unit_id): Path<String>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        unit_id = %unit_id,
        "Getting code unit"
    );

    match get_code_unit_impl(&context, &unit_id).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to get code unit");
            let api_response =
                ApiResponse::<CodeUnitResponse>::error(e.to_string(), request_id);
            (StatusCode::NOT_FOUND, Json(api_response)).into_response()
        }
    }
}

async fn get_code_unit_impl(
    context: &CodeUnitContext,
    unit_id: &str,
) -> anyhow::Result<CodeUnitResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    // Query for the specific code unit
    let query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
    let mut result = conn.query(&query).await?;
    let units: Vec<CodeUnit> = result.take(0)?;

    let unit = units
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Code unit not found"))?;

    let complexity_score = unit.complexity_score();

    Ok(CodeUnitResponse {
        id: unit.id.to_string(),
        unit_type: format!("{:?}", unit.unit_type).to_lowercase(),
        name: unit.name,
        qualified_name: unit.qualified_name,
        display_name: unit.display_name,
        file_path: unit.file_path,
        language: format!("{:?}", unit.language).to_lowercase(),
        start_line: unit.start_line,
        end_line: unit.end_line,
        start_column: unit.start_column,
        end_column: unit.end_column,
        signature: unit.signature,
        body: unit.body,
        docstring: unit.docstring,
        visibility: format!("{:?}", unit.visibility).to_lowercase(),
        is_async: unit.is_async,
        is_exported: unit.is_exported,
        complexity: ComplexityResponse {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
            score: complexity_score,
        },
        has_tests: unit.has_tests,
        has_documentation: unit.has_documentation,
        created_at: unit.created_at,
        updated_at: unit.updated_at,
    })
}

/// PUT /api/v1/units/{id} - Update code unit
async fn update_code_unit(
    State(context): State<CodeUnitContext>,
    Path(unit_id): Path<String>,
    Json(update): Json<UpdateCodeUnitRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        unit_id = %unit_id,
        "Updating code unit"
    );

    match update_code_unit_impl(&context, &unit_id, update).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to update code unit");
            let api_response =
                ApiResponse::<CodeUnitResponse>::error(e.to_string(), request_id);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn update_code_unit_impl(
    context: &CodeUnitContext,
    unit_id: &str,
    update: UpdateCodeUnitRequest,
) -> anyhow::Result<CodeUnitResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    // First, get the existing unit
    let query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
    let mut result = conn.query(&query).await?;
    let units: Vec<CodeUnit> = result.take(0)?;

    let mut unit = units
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Code unit not found"))?;

    // Check version if provided
    if let Some(expected_version) = update.expected_version {
        if unit.version != expected_version {
            return Err(anyhow::anyhow!(
                "Version mismatch: expected {}, found {}",
                expected_version,
                unit.version
            ));
        }
    }

    // Update fields
    if let Some(body) = update.body {
        unit.body = Some(body);
    }
    if let Some(docstring) = update.docstring {
        unit.docstring = Some(docstring);
        unit.has_documentation = true;
    }

    // Increment version and update timestamp
    unit.version += 1;
    unit.updated_at = chrono::Utc::now();

    // Clone values we need for the response before moving unit
    let version = unit.version;
    let updated_at = unit.updated_at;
    let has_documentation = unit.has_documentation;
    let complexity_score = unit.complexity_score();

    // Save to database
    let update_query = format!(
        "UPDATE code_unit:{} SET body = $body, docstring = $docstring, version = $version, updated_at = $updated_at, has_documentation = $has_documentation",
        unit_id
    );

    conn.query(&update_query)
        .bind(("body", unit.body.clone()))
        .bind(("docstring", unit.docstring.clone()))
        .bind(("version", version))
        .bind(("updated_at", updated_at))
        .bind(("has_documentation", has_documentation))
        .await?;

    Ok(CodeUnitResponse {
        id: unit.id.to_string(),
        unit_type: format!("{:?}", unit.unit_type).to_lowercase(),
        name: unit.name,
        qualified_name: unit.qualified_name,
        display_name: unit.display_name,
        file_path: unit.file_path,
        language: format!("{:?}", unit.language).to_lowercase(),
        start_line: unit.start_line,
        end_line: unit.end_line,
        start_column: unit.start_column,
        end_column: unit.end_column,
        signature: unit.signature,
        body: unit.body,
        docstring: unit.docstring,
        visibility: format!("{:?}", unit.visibility).to_lowercase(),
        is_async: unit.is_async,
        is_exported: unit.is_exported,
        complexity: ComplexityResponse {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
            score: complexity_score,
        },
        has_tests: unit.has_tests,
        has_documentation: unit.has_documentation,
        created_at: unit.created_at,
        updated_at: unit.updated_at,
    })
}
