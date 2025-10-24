//! VFS navigation endpoints (PRIORITY)

use crate::api::{
    error::{ApiError, ApiResult},
    types::{
        ApiResponse, CreateFileRequest, DirectoryTreeResponse, FileListRequest, FileResponse,
        TreeNode, UpdateFileRequest,
    },
    pagination::{LinkBuilder, build_pagination_info, decode_cursor, generate_next_cursor},
};
use crate::services::VfsService;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;

/// VFS context shared across handlers
#[derive(Clone)]
pub struct VfsContext {
    pub vfs_service: Arc<VfsService>,
}

/// Create VFS routes
pub fn vfs_routes(context: VfsContext) -> Router {
    Router::new()
        .route("/api/v1/workspaces/{workspace_id}/files", get(list_files))
        .route("/api/v1/workspaces/{workspace_id}/files", post(create_file))
        .route("/api/v1/workspaces/{workspace_id}/tree", get(get_tree))
        .route("/api/v1/files/{file_id}", get(get_file))
        .route("/api/v1/files/{file_id}", put(update_file))
        .route("/api/v1/files/{file_id}", delete(delete_file))
        .with_state(context)
}

/// GET /api/v1/workspaces/{workspace_id}/files - Browse VFS
async fn list_files(
    State(ctx): State<VfsContext>,
    Path(workspace_id): Path<String>,
    Query(mut params): Query<FileListRequest>,
) -> ApiResult<Json<ApiResponse<Vec<FileResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Validate limit
    if params.limit < 10 {
        params.limit = 10;
    } else if params.limit > 100 {
        params.limit = 100;
    }

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Decode cursor if present
    let cursor_data = if let Some(ref cursor) = params.cursor {
        Some(decode_cursor(cursor).map_err(|e| ApiError::BadRequest(e))?)
    } else {
        None
    };

    // Use VFS service to list files
    let file_details = ctx.vfs_service
        .list_directory(&workspace_uuid, "/", params.recursive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Convert to FileResponse and apply filters
    let mut files: Vec<FileResponse> = file_details
        .into_iter()
        .filter(|file| {
            // Filter by type if specified
            if let Some(ref file_type) = params.file_type {
                if file.node_type != *file_type {
                    return false;
                }
            }

            // Filter by language if specified
            if let Some(ref language) = params.language {
                if let Some(ref lang) = file.language {
                    if !lang.to_lowercase().contains(&language.to_lowercase()) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by cursor if present
            if let Some(ref cursor) = cursor_data {
                if file.created_at > cursor.last_timestamp ||
                   (file.created_at == cursor.last_timestamp && file.id >= cursor.last_id) {
                    return false;
                }
            }

            true
        })
        .map(|file| FileResponse {
            id: file.id,
            name: file.name,
            path: file.path,
            file_type: file.node_type,
            size: file.size_bytes,
            language: file.language,
            content: None, // Don't include content in list view
            created_at: file.created_at,
            updated_at: file.updated_at,
            // Session-specific fields (not applicable for VFS routes)
            modified_in_session: None,
            change_type: None,
            session_version: None,
            base_version: None,
            encoding: None,
            line_count: None,
            hash: None,
            metadata: None,
        })
        .collect();

    // Sort by created_at DESC, id DESC for consistent pagination
    files.sort_by(|a, b| {
        b.created_at.cmp(&a.created_at)
            .then_with(|| b.id.cmp(&a.id))
    });

    // Apply cursor-based pagination
    let total = files.len();
    let mut result = files.into_iter().take(params.limit + 1).collect::<Vec<_>>();
    let has_more = result.len() > params.limit;
    if has_more {
        result.pop();
    }
    let paginated_files = result;

    // Generate next cursor
    let next_cursor = if has_more && !paginated_files.is_empty() {
        let last = paginated_files.last().unwrap();
        generate_next_cursor(
            last.id.clone(),
            last.created_at,
            cursor_data.map(|c| c.offset + params.limit).unwrap_or(params.limit),
        )
    } else {
        None
    };

    tracing::debug!(
        workspace_id = %workspace_id,
        total_files = total,
        returned = paginated_files.len(),
        has_more = has_more,
        "Listed VFS files"
    );

    let duration = start.elapsed().as_millis() as u64;

    // Build response with cursor-based pagination
    let pagination = build_pagination_info(
        paginated_files.len(),
        params.limit,
        Some(total),
        next_cursor.clone(),
    );

    let link_builder = LinkBuilder::new(format!("/api/v1/workspaces/{}/files", workspace_id));
    let links = link_builder.build_list_links(
        params.cursor.as_deref(),
        next_cursor.as_deref(),
        params.limit,
    );

    Ok(Json(ApiResponse::success_with_pagination(
        paginated_files,
        request_id,
        duration,
        pagination,
        links,
    )))
}

/// GET /api/v1/files/{file_id} - Get file details
async fn get_file(
    State(ctx): State<VfsContext>,
    Path(file_id): Path<String>,
) -> ApiResult<Json<ApiResponse<FileResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse file ID
    let file_uuid = uuid::Uuid::parse_str(&file_id)
        .map_err(|_| ApiError::BadRequest("Invalid file ID".to_string()))?;

    // Note: VfsService works with paths, not IDs. We'd need to query the DB to get the path from ID.
    // For now, this is a limitation. In a full implementation, we'd either:
    // 1. Add a get_by_id method to VfsService
    // 2. Keep a small amount of DB logic here for ID->path resolution
    // Let's add a TODO and keep the minimal DB access for ID resolution

    // TODO: Consider adding get_file_by_id to VfsService
    return Err(ApiError::Internal(
        "File access by ID requires path resolution - use workspace file listing instead".to_string()
    ));
}

/// POST /api/v1/workspaces/{workspace_id}/files - Create file
async fn create_file(
    State(ctx): State<VfsContext>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateFileRequest>,
) -> ApiResult<Json<ApiResponse<FileResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Check if file already exists
    if ctx.vfs_service.exists(&workspace_uuid, &payload.path).await
        .map_err(|e| ApiError::Internal(e.to_string()))? {
        return Err(ApiError::BadRequest("File already exists".to_string()));
    }

    // Use VFS service to write file (handles parent directory creation)
    let file = ctx.vfs_service
        .write_file(&workspace_uuid, &payload.path, payload.content.as_bytes())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Convert to API response format
    let file_response = FileResponse {
        id: file.id,
        name: file.name,
        path: file.path.clone(),
        file_type: file.node_type,
        size: file.size_bytes,
        language: file.language,
        content: Some(payload.content),
        created_at: file.created_at,
        updated_at: file.updated_at,
        // Session-specific fields (not applicable for VFS routes)
        modified_in_session: None,
        change_type: None,
        session_version: None,
        base_version: None,
        encoding: None,
        line_count: None,
        hash: None,
        metadata: None,
    };

    tracing::info!(
        workspace_id = %workspace_id,
        path = %file.path,
        size = file.size_bytes,
        "Created file in VFS"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
}

/// PUT /api/v1/files/{file_id} - Update file
async fn update_file(
    State(_ctx): State<VfsContext>,
    Path(file_id): Path<String>,
    Json(_payload): Json<UpdateFileRequest>,
) -> ApiResult<Json<ApiResponse<FileResponse>>> {
    let _file_uuid = uuid::Uuid::parse_str(&file_id)
        .map_err(|_| ApiError::BadRequest("Invalid file ID".to_string()))?;

    // TODO: File update by ID requires path resolution
    // Consider adding workspace_id + path based update endpoint instead
    return Err(ApiError::Internal(
        "File update by ID requires path resolution - use workspace-based file operations instead".to_string()
    ));
}

/// DELETE /api/v1/files/{file_id} - Delete file
async fn delete_file(
    State(_ctx): State<VfsContext>,
    Path(file_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let _file_uuid = uuid::Uuid::parse_str(&file_id)
        .map_err(|_| ApiError::BadRequest("Invalid file ID".to_string()))?;

    // TODO: File deletion by ID requires path resolution
    // Consider adding workspace_id + path based delete endpoint instead
    return Err(ApiError::Internal(
        "File deletion by ID requires path resolution - use workspace-based file operations instead".to_string()
    ));
}

/// GET /api/v1/workspaces/{workspace_id}/tree - Get directory tree
async fn get_tree(
    State(ctx): State<VfsContext>,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ApiResponse<DirectoryTreeResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse workspace ID
    let workspace_uuid = uuid::Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    // Use VFS service to get tree (max_depth of 10 for reasonable tree size)
    let service_tree = ctx.vfs_service
        .get_tree(&workspace_uuid, "/", 10)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Convert service tree to API response format
    let tree = convert_service_tree_to_api(service_tree);

    tracing::debug!(workspace_id = %workspace_id, "Generated directory tree");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(tree, request_id, duration)))
}

/// Convert service DirectoryTree to API DirectoryTreeResponse
fn convert_service_tree_to_api(service_tree: crate::services::vfs::DirectoryTree) -> DirectoryTreeResponse {
    DirectoryTreeResponse {
        name: service_tree.name,
        path: service_tree.path,
        children: service_tree.children.map(|children| {
            children.into_iter().map(convert_service_tree_node_to_api).collect()
        }).unwrap_or_default(),
    }
}

/// Convert service DirectoryTree node to API TreeNode
fn convert_service_tree_node_to_api(service_node: crate::services::vfs::DirectoryTree) -> TreeNode {
    TreeNode {
        name: service_node.name,
        path: service_node.path,
        node_type: service_node.node_type,
        children: service_node.children.map(|children| {
            children.into_iter().map(convert_service_tree_node_to_api).collect()
        }),
    }
}

/// Helper function to build tree from flat list of vnodes
fn build_tree_from_vnodes(vnodes: Vec<cortex_vfs::VNode>) -> DirectoryTreeResponse {
    use std::collections::HashMap;

    // Create a map of path -> vnode
    let mut path_map: HashMap<String, cortex_vfs::VNode> = HashMap::new();
    for vnode in vnodes {
        path_map.insert(vnode.path.to_string(), vnode);
    }

    // Build tree recursively
    fn build_node(
        path: &str,
        path_map: &HashMap<String, cortex_vfs::VNode>,
    ) -> Option<TreeNode> {
        let vnode = path_map.get(path)?;

        let children = if vnode.is_directory() {
            // Find all direct children
            let mut child_nodes = Vec::new();
            let path_prefix = if path.is_empty() || path == "/" {
                String::new()
            } else {
                format!("{}/", path)
            };

            for (child_path, _) in path_map.iter() {
                if child_path.starts_with(&path_prefix) && child_path != path {
                    let relative = &child_path[path_prefix.len()..];
                    // Only direct children (no slashes in relative path)
                    if !relative.contains('/') {
                        if let Some(child_node) = build_node(child_path, path_map) {
                            child_nodes.push(child_node);
                        }
                    }
                }
            }

            // Sort children by name
            child_nodes.sort_by(|a, b| a.name.cmp(&b.name));

            Some(child_nodes)
        } else {
            None
        };

        Some(TreeNode {
            name: vnode.path.file_name().unwrap_or(path).to_string(),
            path: vnode.path.to_string(),
            node_type: match vnode.node_type {
                cortex_vfs::NodeType::File => "file".to_string(),
                cortex_vfs::NodeType::Directory => "directory".to_string(),
                cortex_vfs::NodeType::SymLink => "symlink".to_string(),
                cortex_vfs::NodeType::Document => "document".to_string(),
            },
            children,
        })
    }

    // Build children of root
    let mut root_children = Vec::new();
    for (path, _vnode) in path_map.iter() {
        // Only include top-level items (no slashes)
        if !path.contains('/') && !path.is_empty() {
            if let Some(node) = build_node(path, &path_map) {
                root_children.push(node);
            }
        } else if path == "/" {
            // Handle root directory
            if let Some(node) = build_node(path, &path_map) {
                if let Some(children) = node.children {
                    root_children.extend(children);
                }
            }
        }
    }

    // Sort root children
    root_children.sort_by(|a, b| a.name.cmp(&b.name));

    DirectoryTreeResponse {
        name: "root".to_string(),
        path: "/".to_string(),
        children: root_children,
    }
}
