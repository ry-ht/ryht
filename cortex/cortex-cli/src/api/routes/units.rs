//! Code Units API routes

use crate::api::types::*;
use crate::services::CodeUnitService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// Context for code unit routes
#[derive(Clone)]
pub struct CodeUnitContext {
    pub service: Arc<CodeUnitService>,
}

/// Create code unit routes
pub fn code_unit_routes(context: CodeUnitContext) -> Router {
    Router::new()
        .route("/api/v1/workspaces/{id}/units", get(list_code_units))
        .route("/api/v1/units/{id}", get(get_code_unit))
        .route("/api/v1/units/{id}", put(update_code_unit))
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
    // Parse workspace UUID
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| anyhow::anyhow!("Invalid workspace ID: {}", e))?;

    let limit = params.limit.min(1000);

    // Call service to list code units
    let units = context
        .service
        .list_code_units(
            workspace_uuid,
            params.unit_type.clone(),
            params.language.clone(),
            params.visibility.clone(),
            params.min_complexity.map(|c| c as i32),
            limit,
        )
        .await?;

    // Get total count with filters
    let filters = crate::services::code_units::CodeUnitFilters {
        unit_type: params.unit_type,
        language: params.language,
        visibility: params.visibility,
        has_tests: params.has_tests.unwrap_or(false),
        has_documentation: params.has_docs.unwrap_or(false),
        limit: None,
    };

    let total = context.service.count_units(workspace_uuid, filters).await?;

    // Convert service types to API response types
    let unit_responses: Vec<CodeUnitResponse> = units
        .into_iter()
        .map(|unit| CodeUnitResponse {
            id: unit.id,
            unit_type: unit.unit_type,
            name: unit.name,
            qualified_name: unit.qualified_name,
            display_name: unit.display_name,
            file_path: unit.file_path,
            language: unit.language,
            start_line: unit.start_line,
            end_line: unit.end_line,
            start_column: unit.start_column,
            end_column: unit.end_column,
            signature: unit.signature,
            body: unit.body,
            docstring: unit.docstring,
            visibility: unit.visibility,
            is_async: unit.is_async,
            is_exported: unit.is_exported,
            complexity: ComplexityResponse {
                cyclomatic: unit.complexity.cyclomatic,
                cognitive: unit.complexity.cognitive,
                nesting: unit.complexity.nesting,
                lines: unit.complexity.lines,
                score: unit.complexity.score,
            },
            has_tests: unit.has_tests,
            has_documentation: unit.has_documentation,
            created_at: unit.created_at,
            updated_at: unit.updated_at,
        })
        .collect();

    Ok(CodeUnitListResponse {
        units: unit_responses,
        total,
        limit,
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
    // Call service to get code unit
    let unit = context.service.get_code_unit(unit_id).await?;

    // Convert service type to API response type
    Ok(CodeUnitResponse {
        id: unit.id,
        unit_type: unit.unit_type,
        name: unit.name,
        qualified_name: unit.qualified_name,
        display_name: unit.display_name,
        file_path: unit.file_path,
        language: unit.language,
        start_line: unit.start_line,
        end_line: unit.end_line,
        start_column: unit.start_column,
        end_column: unit.end_column,
        signature: unit.signature,
        body: unit.body,
        docstring: unit.docstring,
        visibility: unit.visibility,
        is_async: unit.is_async,
        is_exported: unit.is_exported,
        complexity: ComplexityResponse {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
            score: unit.complexity.score,
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
    // Call service to update code unit
    let unit = context
        .service
        .update_code_unit(
            unit_id,
            update.body,
            update.docstring,
            update.expected_version,
        )
        .await?;

    // Convert service type to API response type
    Ok(CodeUnitResponse {
        id: unit.id,
        unit_type: unit.unit_type,
        name: unit.name,
        qualified_name: unit.qualified_name,
        display_name: unit.display_name,
        file_path: unit.file_path,
        language: unit.language,
        start_line: unit.start_line,
        end_line: unit.end_line,
        start_column: unit.start_column,
        end_column: unit.end_column,
        signature: unit.signature,
        body: unit.body,
        docstring: unit.docstring,
        visibility: unit.visibility,
        is_async: unit.is_async,
        is_exported: unit.is_exported,
        complexity: ComplexityResponse {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
            score: unit.complexity.score,
        },
        has_tests: unit.has_tests,
        has_documentation: unit.has_documentation,
        created_at: unit.created_at,
        updated_at: unit.updated_at,
    })
}
