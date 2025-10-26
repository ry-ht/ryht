//! Workspace management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    types::{
        ApiResponse, CreateWorkspaceRequest, WorkspaceResponse,
        UpdateWorkspaceRequest, SyncWorkspaceRequest, SyncResponse, SyncChange,
        PaginationParams,
    },
    pagination::{LinkBuilder, build_pagination_info, generate_next_cursor},
};
use crate::services::{WorkspaceService, workspace::ListWorkspaceFilters};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;

/// Workspace context
#[derive(Clone)]
pub struct WorkspaceContext {
    pub workspace_service: Arc<WorkspaceService>,
}

/// Create workspace routes
pub fn workspace_routes(context: WorkspaceContext) -> Router {
    Router::new()
        .route("/api/v1/workspaces", get(list_workspaces))
        .route("/api/v1/workspaces", post(create_workspace))
        .route("/api/v1/workspaces/{workspace_id}", get(get_workspace))
        .route("/api/v1/workspaces/{workspace_id}", put(update_workspace))
        .route("/api/v1/workspaces/{workspace_id}", delete(delete_workspace))
        .route("/api/v1/workspaces/{workspace_id}/sync", post(sync_workspace))
        .with_state(context)
}

/// GET /api/v1/workspaces - List all workspaces
async fn list_workspaces(
    auth_user: AuthUser, // Extract authenticated user
    State(ctx): State<WorkspaceContext>,
    Query(mut params): Query<PaginationParams>,
) -> ApiResult<Json<ApiResponse<Vec<WorkspaceResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Log authenticated operation
    tracing::info!(
        user_id = %auth_user.user_id,
        email = %auth_user.email,
        roles = ?auth_user.roles,
        "User listing workspaces"
    );

    // Validate pagination params
    params.validate().map_err(|e| ApiError::BadRequest(e))?;

    // Use workspace service to list workspaces
    let filters = ListWorkspaceFilters {
        workspace_type: None,
        limit: Some(params.limit + 1), // Fetch one extra to check if there are more
    };

    let workspaces = ctx.workspace_service
        .list_workspaces(filters)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Check if there are more results
    let mut workspaces = workspaces;
    let has_more = workspaces.len() > params.limit;
    if has_more {
        workspaces.pop(); // Remove the extra item
    }

    // Generate next cursor if there are more results
    let next_cursor = if has_more && !workspaces.is_empty() {
        let last = workspaces.last().unwrap();
        generate_next_cursor(
            last.id.clone(),
            last.created_at,
            params.limit,
        )
    } else {
        None
    };

    // Convert to API response format
    let workspace_responses: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .map(|ws| WorkspaceResponse {
            id: ws.id,
            name: ws.name,
            workspace_type: ws.workspace_type(),
            source_type: ws.source_type(),
            namespace: ws.namespace,
            source_path: ws.source_path(),
            read_only: ws.read_only,
            created_at: ws.created_at,
            updated_at: ws.updated_at,
        })
        .collect();

    tracing::debug!(count = workspace_responses.len(), has_more = has_more, "Listed workspaces");

    let duration = start.elapsed().as_millis() as u64;

    // Build pagination info and HATEOAS links
    let pagination = build_pagination_info(
        workspace_responses.len(),
        params.limit,
        None, // Total count would require additional query
        next_cursor.clone(),
    );

    let link_builder = LinkBuilder::new("/api/v1/workspaces");
    let links = link_builder.build_list_links(
        params.cursor.as_deref(),
        next_cursor.as_deref(),
        params.limit,
    );

    Ok(Json(ApiResponse::success_with_pagination(
        workspace_responses,
        request_id,
        duration,
        pagination,
        links,
    )))
}

/// GET /api/v1/workspaces/{workspace_id} - Get workspace details
async fn get_workspace(
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<WorkspaceResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Use workspace service to get workspace
    let workspace = ctx.workspace_service
        .get_workspace(&workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // Convert to API response format
    let workspace_response = WorkspaceResponse {
        id: workspace.id,
        name: workspace.name,
        workspace_type: workspace.workspace_type(),
        source_type: workspace.source_type(),
        namespace: workspace.namespace,
        source_path: workspace.source_path(),
        read_only: workspace.read_only,
        created_at: workspace.created_at,
        updated_at: workspace.updated_at,
    };

    tracing::debug!(workspace_id = %workspace_id, "Retrieved workspace details");

    let duration = start.elapsed().as_millis() as u64;

    // Add HATEOAS links for workspace
    let links = LinkBuilder::build_workspace_links(&workspace_id);
    let mut response = ApiResponse::success(workspace_response, request_id, duration);
    response.links = Some(links);

    Ok(Json(response))
}

/// POST /api/v1/workspaces - Create workspace
async fn create_workspace(
    auth_user: AuthUser, // Extract authenticated user
    State(ctx): State<WorkspaceContext>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> ApiResult<Json<ApiResponse<WorkspaceResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::info!(
        user_id = %auth_user.user_id,
        workspace_name = %payload.name,
        "User creating workspace"
    );

    // Convert API request to service request
    let service_request = crate::services::workspace::CreateWorkspaceRequest {
        name: payload.name.clone(),
        workspace_type: Some(payload.workspace_type.clone()),
        source_path: payload.source_path.clone(),
        sync_sources: None,
        read_only: Some(false),
        metadata: None,
    };

    // Use workspace service to create workspace
    let workspace = ctx.workspace_service
        .create_workspace(service_request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Clone workspace_id before moving workspace
    let workspace_id = workspace.id.clone();

    // Convert to API response format
    let workspace_response = WorkspaceResponse {
        id: workspace.id,
        name: workspace.name,
        workspace_type: workspace.workspace_type(),
        source_type: workspace.source_type(),
        namespace: workspace.namespace,
        source_path: workspace.source_path(),
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

/// DELETE /api/v1/workspaces/{workspace_id} - Delete workspace
async fn delete_workspace(
    auth_user: AuthUser, // Extract authenticated user
    State(ctx): State<WorkspaceContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Check if user is admin (deletion is sensitive operation)
    if !auth_user.is_admin() {
        tracing::warn!(
            user_id = %auth_user.user_id,
            workspace_id = %workspace_id,
            "Non-admin user attempted to delete workspace"
        );
        return Err(ApiError::Forbidden(
            "Only administrators can delete workspaces".to_string()
        ));
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        workspace_id = %workspace_id,
        "Admin deleting workspace"
    );

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Use workspace service to delete workspace
    ctx.workspace_service
        .delete_workspace(&workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(workspace_id = %workspace_id, "Deleted workspace");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success((), request_id, duration)))
}

/// PUT /api/v1/workspaces/{workspace_id} - Update workspace
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

    // Convert API request to service request
    let service_request = crate::services::workspace::UpdateWorkspaceRequest {
        name: payload.name.clone(),
        workspace_type: payload.workspace_type.clone(),
        read_only: payload.read_only,
    };

    // Use workspace service to update workspace
    let workspace = ctx.workspace_service
        .update_workspace(&workspace_uuid, service_request)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::NotFound(format!("Workspace {} not found", workspace_id))
            } else {
                ApiError::Internal(e.to_string())
            }
        })?;

    // Convert to API response format
    let workspace_response = WorkspaceResponse {
        id: workspace.id,
        name: workspace.name,
        workspace_type: workspace.workspace_type(),
        source_type: workspace.source_type(),
        namespace: workspace.namespace,
        source_path: workspace.source_path(),
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

/// POST /api/v1/workspaces/{workspace_id}/sync - Sync workspace with filesystem
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

    // Use workspace service to get workspace
    let workspace = ctx.workspace_service
        .get_workspace(&workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // Check if workspace has a source path
    let source_path_str = workspace.source_path().ok_or_else(||
        ApiError::BadRequest("Workspace has no source path to sync from".to_string())
    )?;

    let source_path = std::path::PathBuf::from(&source_path_str);

    let force = payload.force.unwrap_or(false);
    let dry_run = payload.dry_run.unwrap_or(false);

    tracing::info!(
        workspace_id = %workspace_id,
        source_path = %source_path.display(),
        force = force,
        dry_run = dry_run,
        "Syncing workspace with filesystem"
    );

    // Verify source path exists
    if !source_path.exists() {
        return Err(ApiError::BadRequest(
            format!("Source path does not exist: {}", source_path.display())
        ));
    }

    let mut changes = Vec::new();
    let mut files_added = 0;
    let mut files_updated = 0;
    let mut files_deleted = 0;

    // Get VFS handle from workspace service
    let vfs = &ctx.workspace_service.vfs;

    // Get all existing files in VFS for this workspace
    let root_path = cortex_vfs::VirtualPath::root();
    let existing_vnodes = vfs.list_directory(&workspace_uuid, &root_path, true)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build a set of existing file paths in VFS
    let mut vfs_files: std::collections::HashSet<String> = existing_vnodes
        .iter()
        .filter(|v| v.is_file())
        .map(|v| v.path.to_string())
        .collect();

    // Walk the filesystem and compare with VFS
    use walkdir::WalkDir;
    let walker = WalkDir::new(&source_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in walker {
        let file_path = entry.path();
        let metadata = entry.metadata()
            .map_err(|e| ApiError::Internal(format!("Failed to read metadata for {}: {}", file_path.display(), e)))?;

        // Calculate relative path
        let relative_path = file_path.strip_prefix(&source_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Convert to VFS path format (use forward slashes)
        let vfs_path_str = format!("/{}", relative_path.replace('\\', "/"));
        let vfs_path = cortex_vfs::VirtualPath::new(&vfs_path_str)
            .map_err(|e| ApiError::Internal(format!("Invalid path {}: {}", vfs_path_str, e)))?;

        // Check if file exists in VFS
        let vnode_exists = vfs.metadata(&workspace_uuid, &vfs_path).await.is_ok();

        if vnode_exists {
            // File exists - check if it needs updating
            if force {
                // Force update: read content and write to VFS
                if !dry_run {
                    let content = std::fs::read(file_path)
                        .map_err(|e| ApiError::Internal(format!("Failed to read file {}: {}", file_path.display(), e)))?;

                    vfs.write_file(&workspace_uuid, &vfs_path, &content)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to write file to VFS: {}", e)))?;
                }

                files_updated += 1;
                changes.push(SyncChange {
                    path: vfs_path_str.clone(),
                    change_type: "updated".to_string(),
                    size_bytes: Some(metadata.len()),
                });

                tracing::debug!(path = %vfs_path_str, "Updated file in VFS");
            }

            // Mark as seen
            vfs_files.remove(&vfs_path_str);
        } else {
            // File doesn't exist - add it
            if !dry_run {
                let content = std::fs::read(file_path)
                    .map_err(|e| ApiError::Internal(format!("Failed to read file {}: {}", file_path.display(), e)))?;

                // Create parent directories if needed
                if let Some(parent) = vfs_path.parent() {
                    vfs.create_directory(&workspace_uuid, &parent, true)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to create directory: {}", e)))?;
                }

                vfs.write_file(&workspace_uuid, &vfs_path, &content)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to write file to VFS: {}", e)))?;
            }

            files_added += 1;
            changes.push(SyncChange {
                path: vfs_path_str.clone(),
                change_type: "added".to_string(),
                size_bytes: Some(metadata.len()),
            });

            tracing::debug!(path = %vfs_path_str, "Added file to VFS");
        }
    }

    // Delete files that exist in VFS but not in filesystem
    for vfs_file_path in vfs_files {
        let vfs_path = cortex_vfs::VirtualPath::new(&vfs_file_path)
            .map_err(|e| ApiError::Internal(format!("Invalid VFS path {}: {}", vfs_file_path, e)))?;

        if !dry_run {
            vfs.delete(&workspace_uuid, &vfs_path, false)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to delete file from VFS: {}", e)))?;
        }

        files_deleted += 1;
        changes.push(SyncChange {
            path: vfs_file_path.clone(),
            change_type: "deleted".to_string(),
            size_bytes: None,
        });

        tracing::debug!(path = %vfs_file_path, "Deleted file from VFS");
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
