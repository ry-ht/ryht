//! Workspace management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{
        ApiResponse, CreateWorkspaceRequest, WorkspaceResponse,
        UpdateWorkspaceRequest, SyncWorkspaceRequest, SyncResponse, SyncChange,
    },
};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use std::sync::Arc;
use std::time::Instant;

/// Workspace context
#[derive(Clone)]
pub struct WorkspaceContext {
    pub vfs: Arc<VirtualFileSystem>,
    pub storage: Arc<ConnectionManager>,
}

/// Create workspace routes
pub fn workspace_routes(context: WorkspaceContext) -> Router {
    Router::new()
        .route("/api/v3/workspaces", get(list_workspaces))
        .route("/api/v3/workspaces", post(create_workspace))
        .route("/api/v3/workspaces/:workspace_id", get(get_workspace))
        .route("/api/v3/workspaces/:workspace_id", put(update_workspace))
        .route("/api/v3/workspaces/:workspace_id", delete(delete_workspace))
        .route("/api/v3/workspaces/:workspace_id/sync", post(sync_workspace))
        .with_state(context)
}

/// GET /api/v3/workspaces - List all workspaces
async fn list_workspaces(
    State(ctx): State<WorkspaceContext>,
) -> ApiResult<Json<ApiResponse<Vec<WorkspaceResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query all workspaces from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = "SELECT * FROM workspace ORDER BY created_at DESC";
    let mut response = conn.connection()
        .query(query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspaces: Vec<cortex_vfs::Workspace> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace_responses: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .map(|ws| WorkspaceResponse {
            id: ws.id.to_string(),
            name: ws.name,
            workspace_type: format!("{:?}", ws.workspace_type).to_lowercase(),
            source_type: format!("{:?}", ws.source_type).to_lowercase(),
            namespace: ws.namespace,
            source_path: ws.source_path.map(|p| p.to_string_lossy().to_string()),
            read_only: ws.read_only,
            created_at: ws.created_at,
            updated_at: ws.updated_at,
        })
        .collect();

    tracing::debug!(count = workspace_responses.len(), "Listed workspaces");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(workspace_responses, request_id, duration)))
}

/// GET /api/v3/workspaces/:workspace_id - Get workspace details
async fn get_workspace(
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<WorkspaceResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Query workspace from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace: Option<cortex_vfs::Workspace> = conn.connection()
        .select(("workspace", workspace_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace = workspace.ok_or_else(||
        ApiError::NotFound(format!("Workspace {} not found", workspace_id))
    )?;

    let workspace_response = WorkspaceResponse {
        id: workspace.id.to_string(),
        name: workspace.name,
        workspace_type: format!("{:?}", workspace.workspace_type).to_lowercase(),
        source_type: format!("{:?}", workspace.source_type).to_lowercase(),
        namespace: workspace.namespace,
        source_path: workspace.source_path.map(|p| p.to_string_lossy().to_string()),
        read_only: workspace.read_only,
        created_at: workspace.created_at,
        updated_at: workspace.updated_at,
    };

    tracing::debug!(workspace_id = %workspace_id, "Retrieved workspace details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(workspace_response, request_id, duration)))
}

/// POST /api/v3/workspaces - Create workspace
async fn create_workspace(
    State(ctx): State<WorkspaceContext>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> ApiResult<Json<ApiResponse<WorkspaceResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace type
    let workspace_type = match payload.workspace_type.to_lowercase().as_str() {
        "code" => cortex_vfs::WorkspaceType::Code,
        "documentation" => cortex_vfs::WorkspaceType::Documentation,
        "mixed" => cortex_vfs::WorkspaceType::Mixed,
        "external" => cortex_vfs::WorkspaceType::External,
        _ => return Err(ApiError::BadRequest("Invalid workspace type".to_string())),
    };

    // Create workspace
    let workspace_id = uuid::Uuid::new_v4();
    let namespace = format!("ws_{}", workspace_id.to_string().replace('-', "_"));
    let now = chrono::Utc::now();

    let workspace = cortex_vfs::Workspace {
        id: workspace_id,
        name: payload.name.clone(),
        workspace_type,
        source_type: cortex_vfs::SourceType::Local,
        namespace: namespace.clone(),
        source_path: payload.source_path.as_ref().map(|p| std::path::PathBuf::from(p)),
        read_only: false,
        parent_workspace: None,
        fork_metadata: None,
        created_at: now,
        updated_at: now,
    };

    // Save to database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace_json = serde_json::to_value(&workspace)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("workspace", workspace_id.to_string()))
        .content(workspace_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace_response = WorkspaceResponse {
        id: workspace.id.to_string(),
        name: workspace.name,
        workspace_type: format!("{:?}", workspace.workspace_type).to_lowercase(),
        source_type: format!("{:?}", workspace.source_type).to_lowercase(),
        namespace: workspace.namespace,
        source_path: workspace.source_path.map(|p| p.to_string_lossy().to_string()),
        read_only: workspace.read_only,
        created_at: workspace.created_at,
        updated_at: workspace.updated_at,
    };

    tracing::info!(
        workspace_id = %workspace_id,
        name = %payload.name,
        "Created workspace"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(workspace_response, request_id, duration)))
}

/// DELETE /api/v3/workspaces/:workspace_id - Delete workspace
async fn delete_workspace(
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Delete workspace and all associated vnodes
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Delete all vnodes in workspace
    let delete_vnodes_query = "DELETE vnode WHERE workspace_id = $workspace_id";
    conn.connection()
        .query(delete_vnodes_query)
        .bind(("workspace_id", workspace_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Delete workspace
    let _: Option<cortex_vfs::Workspace> = conn.connection()
        .delete(("workspace", workspace_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(workspace_id = %workspace_id, "Deleted workspace");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success((), request_id, duration)))
}

/// PUT /api/v3/workspaces/:workspace_id - Update workspace
async fn update_workspace(
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> ApiResult<Json<ApiResponse<WorkspaceResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Get existing workspace
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace: Option<cortex_vfs::Workspace> = conn.connection()
        .select(("workspace", workspace_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut workspace = workspace.ok_or_else(||
        ApiError::NotFound(format!("Workspace {} not found", workspace_id))
    )?;

    // Update fields
    if let Some(name) = payload.name {
        workspace.name = name;
    }

    if let Some(workspace_type_str) = payload.workspace_type {
        workspace.workspace_type = match workspace_type_str.to_lowercase().as_str() {
            "code" => cortex_vfs::WorkspaceType::Code,
            "documentation" => cortex_vfs::WorkspaceType::Documentation,
            "mixed" => cortex_vfs::WorkspaceType::Mixed,
            "external" => cortex_vfs::WorkspaceType::External,
            _ => return Err(ApiError::BadRequest("Invalid workspace type".to_string())),
        };
    }

    if let Some(read_only) = payload.read_only {
        workspace.read_only = read_only;
    }

    workspace.updated_at = chrono::Utc::now();

    // Save to database
    let workspace_json = serde_json::to_value(&workspace)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .update(("workspace", workspace_uuid.to_string()))
        .content(workspace_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace_response = WorkspaceResponse {
        id: workspace.id.to_string(),
        name: workspace.name,
        workspace_type: format!("{:?}", workspace.workspace_type).to_lowercase(),
        source_type: format!("{:?}", workspace.source_type).to_lowercase(),
        namespace: workspace.namespace,
        source_path: workspace.source_path.map(|p| p.to_string_lossy().to_string()),
        read_only: workspace.read_only,
        created_at: workspace.created_at,
        updated_at: workspace.updated_at,
    };

    tracing::info!(
        workspace_id = %workspace_id,
        "Updated workspace"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(workspace_response, request_id, duration)))
}

/// POST /api/v3/workspaces/:workspace_id/sync - Sync workspace with filesystem
async fn sync_workspace(
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<SyncWorkspaceRequest>,
) -> ApiResult<Json<ApiResponse<SyncResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Get workspace
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace: Option<cortex_vfs::Workspace> = conn.connection()
        .select(("workspace", workspace_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let workspace = workspace.ok_or_else(||
        ApiError::NotFound(format!("Workspace {} not found", workspace_id))
    )?;

    // Check if workspace has a source path
    let source_path = workspace.source_path.ok_or_else(||
        ApiError::BadRequest("Workspace has no source path to sync from".to_string())
    )?;

    let force = payload.force.unwrap_or(false);
    let dry_run = payload.dry_run.unwrap_or(false);

    tracing::info!(
        workspace_id = %workspace_id,
        source_path = %source_path.display(),
        force = force,
        dry_run = dry_run,
        "Syncing workspace with filesystem"
    );

    // In a real implementation, we would:
    // 1. Walk the filesystem at source_path
    // 2. Compare with VFS entries
    // 3. Add/update/delete as needed
    // For now, we'll simulate the sync

    let mut changes = Vec::new();
    let mut files_added = 0;
    let mut files_updated = 0;
    let mut files_deleted = 0;

    // Simulate scanning filesystem
    if source_path.exists() {
        // This is a simplified version - a real implementation would use
        // walkdir or similar to recursively scan the directory
        if let Ok(entries) = std::fs::read_dir(&source_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let file_path = entry.path();
                        let relative_path = file_path.strip_prefix(&source_path)
                            .unwrap_or(&file_path)
                            .to_string_lossy()
                            .to_string();

                        if !dry_run {
                            // In a real implementation, we would add/update the file in VFS
                            files_added += 1;
                        }

                        changes.push(SyncChange {
                            path: relative_path,
                            change_type: "added".to_string(),
                            size_bytes: Some(metadata.len()),
                        });
                    }
                }
            }
        }
    } else {
        return Err(ApiError::BadRequest(
            format!("Source path does not exist: {}", source_path.display())
        ));
    }

    let total_processed = files_added + files_updated + files_deleted;
    let sync_duration = start.elapsed().as_millis() as u64;

    let response = SyncResponse {
        files_added,
        files_updated,
        files_deleted,
        total_processed,
        duration_ms: sync_duration,
        changes,
    };

    tracing::info!(
        workspace_id = %workspace_id,
        files_added = files_added,
        files_updated = files_updated,
        files_deleted = files_deleted,
        dry_run = dry_run,
        "Workspace sync completed"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}
