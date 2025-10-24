//! VFS navigation endpoints (PRIORITY)

use crate::api::{
    error::{ApiError, ApiResult},
    types::{
        ApiResponse, CreateFileRequest, DirectoryTreeResponse, FileListRequest, FileResponse,
        TreeNode, UpdateFileRequest,
    },
    pagination::{LinkBuilder, build_pagination_info, decode_cursor, generate_next_cursor},
};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use std::sync::Arc;
use std::time::Instant;

/// VFS context shared across handlers
#[derive(Clone)]
pub struct VfsContext {
    pub vfs: Arc<VirtualFileSystem>,
    pub storage: Arc<ConnectionManager>,
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

    // List files from VFS root or recursively
    let root_path = cortex_vfs::VirtualPath::root();
    let vnodes = ctx.vfs.list_directory(&workspace_uuid, &root_path, params.recursive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Convert vnodes to FileResponse and apply filters
    let mut files: Vec<FileResponse> = vnodes
        .into_iter()
        .filter(|vnode| {
            // Filter by type if specified
            if let Some(ref file_type) = params.file_type {
                let node_type_str = match vnode.node_type {
                    cortex_vfs::NodeType::File => "file",
                    cortex_vfs::NodeType::Directory => "directory",
                    cortex_vfs::NodeType::SymLink => "symlink",
                    cortex_vfs::NodeType::Document => "document",
                };
                if node_type_str != file_type {
                    return false;
                }
            }

            // Filter by language if specified
            if let Some(ref language) = params.language {
                if let Some(lang) = &vnode.language {
                    let lang_str = format!("{:?}", lang).to_lowercase();
                    if !lang_str.contains(&language.to_lowercase()) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by cursor if present
            if let Some(ref cursor) = cursor_data {
                if vnode.created_at > cursor.last_timestamp ||
                   (vnode.created_at == cursor.last_timestamp && vnode.id.to_string() >= cursor.last_id) {
                    return false;
                }
            }

            true
        })
        .map(|vnode| FileResponse {
            id: vnode.id.to_string(),
            name: vnode.path.file_name().unwrap_or("").to_string(),
            path: vnode.path.to_string(),
            file_type: match vnode.node_type {
                cortex_vfs::NodeType::File => "file".to_string(),
                cortex_vfs::NodeType::Directory => "directory".to_string(),
                cortex_vfs::NodeType::SymLink => "symlink".to_string(),
                cortex_vfs::NodeType::Document => "document".to_string(),
            },
            size: vnode.size_bytes as u64,
            language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
            content: None, // Don't include content in list view
            created_at: vnode.created_at,
            updated_at: vnode.updated_at,
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

    // Query the database for the vnode
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = "SELECT * FROM vnode WHERE id = $id AND status != 'deleted' LIMIT 1";
    let mut response = conn.connection()
        .query(query)
        .bind(("id", file_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode: Option<cortex_vfs::VNode> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode = vnode.ok_or_else(|| ApiError::NotFound(format!("File {} not found", file_id)))?;

    // Read content if it's a file
    let content = if vnode.is_file() {
        let content_bytes = ctx.vfs.read_file(&vnode.workspace_id, &vnode.path)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // Try to convert to UTF-8 string
        String::from_utf8(content_bytes).ok()
    } else {
        None
    };

    let file_response = FileResponse {
        id: vnode.id.to_string(),
        name: vnode.path.file_name().unwrap_or("").to_string(),
        path: vnode.path.to_string(),
        file_type: match vnode.node_type {
            cortex_vfs::NodeType::File => "file".to_string(),
            cortex_vfs::NodeType::Directory => "directory".to_string(),
            cortex_vfs::NodeType::SymLink => "symlink".to_string(),
            cortex_vfs::NodeType::Document => "document".to_string(),
        },
        size: vnode.size_bytes as u64,
        language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
        content,
        created_at: vnode.created_at,
        updated_at: vnode.updated_at,
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

    tracing::debug!(file_id = %file_id, path = %vnode.path, "Retrieved file details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
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

    // Parse and validate path
    let path = cortex_vfs::VirtualPath::new(&payload.path)
        .map_err(|e| ApiError::BadRequest(format!("Invalid path: {}", e)))?;

    // Check if file already exists
    if ctx.vfs.exists(&workspace_uuid, &path).await
        .map_err(|e| ApiError::Internal(e.to_string()))? {
        return Err(ApiError::BadRequest("File already exists".to_string()));
    }

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        ctx.vfs.create_directory(&workspace_uuid, &parent, true)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Write file to VFS
    ctx.vfs.write_file(&workspace_uuid, &path, payload.content.as_bytes())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Retrieve the created vnode
    let vnode = ctx.vfs.metadata(&workspace_uuid, &path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let file_response = FileResponse {
        id: vnode.id.to_string(),
        name: vnode.path.file_name().unwrap_or("").to_string(),
        path: vnode.path.to_string(),
        file_type: "file".to_string(),
        size: vnode.size_bytes as u64,
        language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
        content: Some(payload.content),
        created_at: vnode.created_at,
        updated_at: vnode.updated_at,
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
        path = %path,
        size = vnode.size_bytes,
        "Created file in VFS"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
}

/// PUT /api/v1/files/{file_id} - Update file
async fn update_file(
    State(ctx): State<VfsContext>,
    Path(file_id): Path<String>,
    Json(payload): Json<UpdateFileRequest>,
) -> ApiResult<Json<ApiResponse<FileResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse file ID
    let file_uuid = uuid::Uuid::parse_str(&file_id)
        .map_err(|_| ApiError::BadRequest("Invalid file ID".to_string()))?;

    // Query the database for the vnode
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = "SELECT * FROM vnode WHERE id = $id AND status != 'deleted' LIMIT 1";
    let mut response = conn.connection()
        .query(query)
        .bind(("id", file_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode: Option<cortex_vfs::VNode> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode = vnode.ok_or_else(|| ApiError::NotFound(format!("File {} not found", file_id)))?;

    // Check if it's a file
    if !vnode.is_file() {
        return Err(ApiError::BadRequest("Not a file".to_string()));
    }

    // Update file content
    ctx.vfs.write_file(&vnode.workspace_id, &vnode.path, payload.content.as_bytes())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Retrieve updated vnode
    let updated_vnode = ctx.vfs.metadata(&vnode.workspace_id, &vnode.path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let file_response = FileResponse {
        id: updated_vnode.id.to_string(),
        name: updated_vnode.path.file_name().unwrap_or("").to_string(),
        path: updated_vnode.path.to_string(),
        file_type: "file".to_string(),
        size: updated_vnode.size_bytes as u64,
        language: updated_vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
        content: Some(payload.content),
        created_at: updated_vnode.created_at,
        updated_at: updated_vnode.updated_at,
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
        file_id = %file_id,
        path = %vnode.path,
        size = updated_vnode.size_bytes,
        "Updated file in VFS"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
}

/// DELETE /api/v1/files/{file_id} - Delete file
async fn delete_file(
    State(ctx): State<VfsContext>,
    Path(file_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse file ID
    let file_uuid = uuid::Uuid::parse_str(&file_id)
        .map_err(|_| ApiError::BadRequest("Invalid file ID".to_string()))?;

    // Query the database for the vnode
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = "SELECT * FROM vnode WHERE id = $id AND status != 'deleted' LIMIT 1";
    let mut response = conn.connection()
        .query(query)
        .bind(("id", file_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode: Option<cortex_vfs::VNode> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let vnode = vnode.ok_or_else(|| ApiError::NotFound(format!("File {} not found", file_id)))?;

    // Delete from VFS
    let recursive = vnode.is_directory();
    ctx.vfs.delete(&vnode.workspace_id, &vnode.path, recursive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        file_id = %file_id,
        path = %vnode.path,
        is_directory = vnode.is_directory(),
        "Deleted from VFS"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success((), request_id, duration)))
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

    // List all files recursively
    let root_path = cortex_vfs::VirtualPath::root();
    let vnodes = ctx.vfs.list_directory(&workspace_uuid, &root_path, true)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build tree structure
    let tree = build_tree_from_vnodes(vnodes);

    tracing::debug!(workspace_id = %workspace_id, "Generated directory tree");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(tree, request_id, duration)))
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
