//! Export and import endpoints for workspace data

use crate::api::{
    error::{ApiError, ApiResult},
    types::ApiResponse,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Export context
#[derive(Clone)]
pub struct ExportContext {
    pub storage: Arc<ConnectionManager>,
}

/// Export format enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[serde(rename = "tar.gz")]
    TarGz,
    Zip,
    Git,
}

/// Export status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportStatus {
    Processing,
    Completed,
    Failed,
}

/// Import source type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImportSourceType {
    File,
    Git,
    Url,
}

/// Export job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub format: ExportFormat,
    pub status: ExportStatus,
    pub include_history: bool,
    pub include_metadata: bool,
    pub estimated_size_mb: Option<i64>,
    pub actual_size_bytes: Option<i64>,
    pub download_url: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Import job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub source_type: ImportSourceType,
    pub source: String,
    pub status: ExportStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Export job response
#[derive(Debug, Serialize)]
pub struct ExportJobResponse {
    pub export_id: String,
    pub status: String,
    pub estimated_size_mb: Option<i64>,
}

/// Import job response
#[derive(Debug, Serialize)]
pub struct ImportJobResponse {
    pub import_id: String,
    pub status: String,
    pub workspace_id: String,
}

/// Export download response
#[derive(Debug, Serialize)]
pub struct ExportDownloadResponse {
    pub download_url: String,
    pub size_bytes: i64,
    pub expires_at: DateTime<Utc>,
}

/// Create export request
#[derive(Debug, Deserialize)]
pub struct CreateExportRequest {
    pub workspace_id: String,
    #[serde(default = "default_format")]
    pub format: ExportFormat,
    #[serde(default = "default_true")]
    pub include_history: bool,
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

fn default_format() -> ExportFormat {
    ExportFormat::TarGz
}

fn default_true() -> bool {
    true
}

/// Create import request
#[derive(Debug, Deserialize)]
pub struct CreateImportRequest {
    pub source_type: ImportSourceType,
    pub source: String,
    pub name: String,
    #[serde(rename = "type")]
    pub workspace_type: String,
}

/// Create export/import routes
pub fn export_routes(context: ExportContext) -> Router {
    Router::new()
        .route("/api/v1/export", post(create_export))
        .route("/api/v1/export/{id}", get(get_export_status))
        .route("/api/v1/export/{id}/download", get(download_export))
        .route("/api/v1/import", post(create_import))
        .route("/api/v1/import/{id}", get(get_import_status))
        .with_state(context)
}

/// POST /api/v1/export - Create export job
async fn create_export(
    State(ctx): State<ExportContext>,
    Json(payload): Json<CreateExportRequest>,
) -> ApiResult<Json<ApiResponse<ExportJobResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_id = Uuid::parse_str(&payload.workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Verify workspace exists
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace: Option<serde_json::Value> = conn.connection()
        .select(("workspace", workspace_id.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if workspace.is_none() {
        return Err(ApiError::NotFound(format!("Workspace {} not found", payload.workspace_id)));
    }

    // Create export job
    let export_id = Uuid::new_v4();
    let now = Utc::now();

    let export_job = ExportJob {
        id: export_id,
        workspace_id,
        format: payload.format.clone(),
        status: ExportStatus::Processing,
        include_history: payload.include_history,
        include_metadata: payload.include_metadata,
        estimated_size_mb: Some(100), // Mock estimation
        actual_size_bytes: None,
        download_url: None,
        error_message: None,
        created_at: now,
        completed_at: None,
    };

    // Save export job to database
    let export_json = serde_json::to_value(&export_job)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("export_job", export_id.to_string()))
        .content(export_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = ExportJobResponse {
        export_id: export_id.to_string(),
        status: "processing".to_string(),
        estimated_size_mb: export_job.estimated_size_mb,
    };

    tracing::info!(
        export_id = %export_id,
        workspace_id = %workspace_id,
        format = ?payload.format,
        "Created export job"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// GET /api/v1/export/{id} - Get export job status
async fn get_export_status(
    State(ctx): State<ExportContext>,
    Path(export_id): Path<String>,
) -> ApiResult<Json<ApiResponse<ExportJob>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let export_uuid = Uuid::parse_str(&export_id)
        .map_err(|_| ApiError::BadRequest("Invalid export ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let export_job: Option<ExportJob> = conn.connection()
        .select(("export_job", export_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let export_job = export_job.ok_or_else(||
        ApiError::NotFound(format!("Export job {} not found", export_id))
    )?;

    tracing::debug!(export_id = %export_id, status = ?export_job.status, "Retrieved export status");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(export_job, request_id, duration)))
}

/// GET /api/v1/export/{id}/download - Download export
async fn download_export(
    State(ctx): State<ExportContext>,
    Path(export_id): Path<String>,
) -> ApiResult<Json<ApiResponse<ExportDownloadResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let export_uuid = Uuid::parse_str(&export_id)
        .map_err(|_| ApiError::BadRequest("Invalid export ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let export_job: Option<ExportJob> = conn.connection()
        .select(("export_job", export_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let export_job = export_job.ok_or_else(||
        ApiError::NotFound(format!("Export job {} not found", export_id))
    )?;

    if export_job.status != ExportStatus::Completed {
        return Err(ApiError::BadRequest("Export is not completed yet".to_string()));
    }

    // Generate download URL (in production, this would be a signed S3 URL or similar)
    let download_url = format!("/api/v1/export/{}/download/file", export_id);
    let expires_at = Utc::now() + chrono::Duration::hours(24);

    let response = ExportDownloadResponse {
        download_url,
        size_bytes: export_job.actual_size_bytes.unwrap_or(0),
        expires_at,
    };

    tracing::info!(export_id = %export_id, "Generated download URL");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// POST /api/v1/import - Create import job
async fn create_import(
    State(ctx): State<ExportContext>,
    Json(payload): Json<CreateImportRequest>,
) -> ApiResult<Json<ApiResponse<ImportJobResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Create workspace for import
    let workspace_id = Uuid::new_v4();
    let now = Utc::now();

    let workspace_data = serde_json::json!({
        "id": workspace_id.to_string(),
        "name": payload.name,
        "workspace_type": "Code",
        "source_type": "Local",
        "namespace": format!("ws_{}", workspace_id.to_string().replace('-', "_")),
        "source_path": null,
        "read_only": false,
        "parent_workspace": null,
        "fork_metadata": null,
        "created_at": now,
        "updated_at": now,
    });

    let _: Option<serde_json::Value> = conn.connection()
        .create(("workspace", workspace_id.to_string()))
        .content(workspace_data)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Create import job
    let import_id = Uuid::new_v4();

    let import_job = ImportJob {
        id: import_id,
        workspace_id,
        source_type: payload.source_type.clone(),
        source: payload.source.clone(),
        status: ExportStatus::Processing,
        error_message: None,
        created_at: now,
        completed_at: None,
    };

    // Save import job to database
    let import_json = serde_json::to_value(&import_job)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("import_job", import_id.to_string()))
        .content(import_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = ImportJobResponse {
        import_id: import_id.to_string(),
        status: "processing".to_string(),
        workspace_id: workspace_id.to_string(),
    };

    tracing::info!(
        import_id = %import_id,
        workspace_id = %workspace_id,
        source = %payload.source,
        "Created import job"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// GET /api/v1/import/{id} - Get import job status
async fn get_import_status(
    State(ctx): State<ExportContext>,
    Path(import_id): Path<String>,
) -> ApiResult<Json<ApiResponse<ImportJob>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let import_uuid = Uuid::parse_str(&import_id)
        .map_err(|_| ApiError::BadRequest("Invalid import ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let import_job: Option<ImportJob> = conn.connection()
        .select(("import_job", import_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let import_job = import_job.ok_or_else(||
        ApiError::NotFound(format!("Import job {} not found", import_id))
    )?;

    tracing::debug!(import_id = %import_id, status = ?import_job.status, "Retrieved import status");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(import_job, request_id, duration)))
}
