//! Session management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{ApiResponse, CreateSessionRequest, FileDiff, FileListResponse, FileResponse, FileWriteResponse, SessionResponse, UpdateFileRequest},
};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Session context - includes VFS for session-aware operations
#[derive(Clone)]
pub struct SessionContext {
    pub storage: Arc<ConnectionManager>,
    pub vfs: Arc<VirtualFileSystem>,
}

/// Session database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub agent_type: String,
    pub workspace_id: Option<Uuid>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

/// Create session routes - includes session-aware VFS operations
pub fn session_routes(context: SessionContext) -> Router {
    Router::new()
        .route("/api/v1/sessions", get(list_sessions))
        .route("/api/v1/sessions", post(create_session))
        .route("/api/v1/sessions/{session_id}", get(get_session))
        .route("/api/v1/sessions/{session_id}", delete(delete_session))
        .route("/api/v1/sessions/{session_id}/merge", post(merge_session))
        .route("/api/v1/locks", get(list_locks))
        // Session-aware VFS operations (critical for multi-agent coordination)
        .route("/api/v1/sessions/{session_id}/files", get(list_session_files))
        .route("/api/v1/sessions/{session_id}/files/{path}", get(read_session_file))
        .route("/api/v1/sessions/{session_id}/files/{path}", put(write_session_file))
        .with_state(context)
}

/// GET /api/v1/sessions - List all sessions
async fn list_sessions(
    State(ctx): State<SessionContext>,
) -> ApiResult<Json<ApiResponse<Vec<SessionResponse>>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query all sessions from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = "SELECT * FROM session ORDER BY created_at DESC";
    let mut response = conn.connection()
        .query(query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let sessions: Vec<Session> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session_responses: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id.to_string(),
            name: s.name,
            agent_type: s.agent_type,
            status: format!("{:?}", s.status).to_lowercase(),
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect();

    tracing::debug!(count = session_responses.len(), "Listed sessions");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(session_responses, request_id, duration)))
}

/// GET /api/v1/sessions/{session_id} - Get session details
async fn get_session(
    State(ctx): State<SessionContext>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<ApiResponse<SessionResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse session ID
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    // Query session from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session: Option<Session> = conn.connection()
        .select(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session = session.ok_or_else(||
        ApiError::NotFound(format!("Session {} not found", session_id))
    )?;

    let session_response = SessionResponse {
        id: session.id.to_string(),
        name: session.name,
        agent_type: session.agent_type,
        status: format!("{:?}", session.status).to_lowercase(),
        created_at: session.created_at,
        updated_at: session.updated_at,
    };

    tracing::debug!(session_id = %session_id, "Retrieved session details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(session_response, request_id, duration)))
}

/// POST /api/v1/sessions - Create session
async fn create_session(
    State(ctx): State<SessionContext>,
    Json(payload): Json<CreateSessionRequest>,
) -> ApiResult<Json<ApiResponse<SessionResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Create session
    let session_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let workspace_id = payload.workspace_id.as_ref()
        .map(|id| Uuid::parse_str(id))
        .transpose()
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    let session = Session {
        id: session_id,
        name: payload.name.clone(),
        agent_type: payload.agent_type.clone(),
        workspace_id,
        status: SessionStatus::Active,
        created_at: now,
        updated_at: now,
    };

    // Save to database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session_json = serde_json::to_value(&session)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("session", session_id.to_string()))
        .content(session_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session_response = SessionResponse {
        id: session.id.to_string(),
        name: session.name,
        agent_type: session.agent_type,
        status: format!("{:?}", session.status).to_lowercase(),
        created_at: session.created_at,
        updated_at: session.updated_at,
    };

    tracing::info!(
        session_id = %session_id,
        name = %payload.name,
        agent_type = %payload.agent_type,
        "Created session"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(session_response, request_id, duration)))
}

/// DELETE /api/v1/sessions/{session_id} - Delete session
async fn delete_session(
    State(ctx): State<SessionContext>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse session ID
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    // Delete session from database
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<Session> = conn.connection()
        .delete(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(session_id = %session_id, "Deleted session");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success((), request_id, duration)))
}

// ============================================================================
// Session-aware VFS Operations (Critical for Multi-Agent Coordination)
// ============================================================================

#[derive(Debug, Deserialize)]
struct FileListQuery {
    /// Filter by path prefix (e.g., /src)
    path: Option<String>,
    /// Include subdirectories
    #[serde(default)]
    recursive: bool,
    /// Only show files modified in this session
    #[serde(default)]
    modified_only: bool,
    /// Filter by type: file, directory, or all
    #[serde(rename = "type")]
    file_type: Option<String>,
    /// Include file content in response
    #[serde(default)]
    include_content: bool,
}

#[derive(Debug, Deserialize)]
struct FileReadQuery {
    /// Include file metadata
    #[serde(default)]
    include_metadata: bool,
    /// Include parsed AST if available
    #[serde(default)]
    include_ast: bool,
    /// Specific session version to read
    version: Option<u64>,
}

/// GET /api/v1/sessions/{session_id}/files - List files in session scope
///
/// Returns all files visible within the session scope, including both workspace files
/// and session-specific modifications.
///
/// Query Parameters:
/// - `path`: Filter by path prefix (e.g., `/src`)
/// - `recursive`: Include subdirectories (default: false)
/// - `modified_only`: Only show files modified in this session (default: false)
/// - `type`: Filter by type - file|directory|all (default: all)
/// - `include_content`: Include file content in response (default: false)
async fn list_session_files(
    State(ctx): State<SessionContext>,
    Path(session_id): Path<String>,
    Query(params): Query<FileListQuery>,
) -> ApiResult<Json<ApiResponse<FileListResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse session ID and get session
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session: Option<Session> = conn.connection()
        .select(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session = session.ok_or_else(||
        ApiError::NotFound(format!("Session {} not found", session_id))
    )?;

    // Get workspace ID from session
    let workspace_id = session.workspace_id.ok_or_else(||
        ApiError::BadRequest("Session has no associated workspace".to_string())
    )?;

    // Determine the base path to list
    let base_path = if let Some(ref path_filter) = params.path {
        cortex_vfs::VirtualPath::new(path_filter)
            .map_err(|e| ApiError::BadRequest(format!("Invalid path filter: {}", e)))?
    } else {
        cortex_vfs::VirtualPath::root()
    };

    // List files from VFS
    let vnodes = ctx.vfs.list_directory(&workspace_id, &base_path, params.recursive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Get all modifications for this session
    let session_modifications = get_session_modifications(&ctx, &session_id).await?;
    let mut modifications_map: HashMap<String, &SessionFileModification> = HashMap::new();

    for modification in &session_modifications {
        // Keep only the latest modification per file
        modifications_map.entry(modification.file_path.clone())
            .or_insert(modification);
    }

    // Apply filters and transform to response format
    let mut files: Vec<FileResponse> = vnodes
        .into_iter()
        .filter(|vnode| {
            // Apply type filter
            let type_match = match params.file_type.as_deref() {
                Some("file") => vnode.is_file(),
                Some("directory") => !vnode.is_file(),
                Some("all") | None => true,
                _ => true,
            };

            // Apply path prefix filter (already handled by base_path, but double-check)
            let path_match = if let Some(ref path_filter) = params.path {
                vnode.path.to_string().starts_with(path_filter)
            } else {
                true
            };

            type_match && path_match
        })
        .map(|vnode| {
            // Check if this file has been modified in this session
            let file_path_str = vnode.path.to_string();
            let modification = modifications_map.get(&file_path_str);

            let modified_in_session = modification.is_some();
            let session_version = modification.map(|m| m.version);
            let base_version = modification.and_then(|m| m.base_version);
            let change_type = modification.map(|m| m.change_type.clone());

            FileResponse {
                id: vnode.id.to_string(),
                name: vnode.path.file_name().unwrap_or("").to_string(),
                path: file_path_str,
                file_type: if vnode.is_file() { "file" } else { "directory" }.to_string(),
                size: vnode.size_bytes as u64,
                language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
                content: None, // Will be filled if include_content is true
                created_at: vnode.created_at,
                updated_at: vnode.updated_at,
                modified_in_session: Some(modified_in_session),
                change_type,
                session_version,
                base_version,
                encoding: None,
                line_count: None,
                hash: None,
                metadata: None,
            }
        })
        .collect();

    // Apply modified_only filter
    if params.modified_only {
        files.retain(|f| f.modified_in_session.unwrap_or(false));
    }

    // Load content if requested
    if params.include_content {
        for file in &mut files {
            if file.file_type == "file" {
                let path = cortex_vfs::VirtualPath::new(&file.path)
                    .map_err(|e| ApiError::Internal(format!("Invalid path: {}", e)))?;

                if let Ok(content_bytes) = ctx.vfs.read_file(&workspace_id, &path).await {
                    if let Ok(content) = String::from_utf8(content_bytes) {
                        file.content = Some(content);
                    }
                }
            }
        }
    }

    let total = files.len();

    let response = FileListResponse {
        files,
        total,
        session_id: Some(session_id.clone()),
    };

    tracing::debug!(
        session_id = %session_id,
        workspace_id = %workspace_id,
        file_count = total,
        recursive = params.recursive,
        modified_only = params.modified_only,
        include_content = params.include_content,
        "Listed session files"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

/// GET /api/v1/sessions/{session_id}/files/:path - Read file from session
///
/// Retrieves file content as it appears within the session scope, including any
/// uncommitted modifications.
///
/// Query Parameters:
/// - `include_metadata`: Include file metadata (default: false)
/// - `include_ast`: Include parsed AST if available (default: false)
/// - `version`: Specific session version to read (default: latest)
async fn read_session_file(
    State(ctx): State<SessionContext>,
    Path((session_id, file_path)): Path<(String, String)>,
    Query(params): Query<FileReadQuery>,
) -> ApiResult<Json<ApiResponse<FileResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse session ID and get session
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session: Option<Session> = conn.connection()
        .select(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session = session.ok_or_else(||
        ApiError::NotFound(format!("Session {} not found", session_id))
    )?;

    // Get workspace ID from session
    let workspace_id = session.workspace_id.ok_or_else(||
        ApiError::BadRequest("Session has no associated workspace".to_string())
    )?;

    // Parse file path
    let path = cortex_vfs::VirtualPath::new(&file_path)
        .map_err(|e| ApiError::BadRequest(format!("Invalid path: {}", e)))?;

    // Read file content
    let content_bytes = ctx.vfs.read_file(&workspace_id, &path)
        .await
        .map_err(|e| ApiError::NotFound(format!("File not found: {}", e)))?;

    let content = String::from_utf8(content_bytes.clone())
        .map_err(|_| ApiError::Internal("File contains invalid UTF-8".to_string()))?;

    // Get metadata
    let vnode = ctx.vfs.metadata(&workspace_id, &path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Calculate additional metadata
    let line_count = content.lines().count();
    let hash = format!("sha256:{:x}", md5::compute(&content_bytes));

    // Check if this file has been modified in this session
    let file_path_str = vnode.path.to_string();
    let modification = get_file_modification(&ctx, &session_id, &file_path_str).await?;

    let modified_in_session = modification.is_some();
    let session_version = modification.as_ref().map(|m| m.version);
    let base_version = modification.as_ref().and_then(|m| m.base_version);

    let metadata = if params.include_metadata {
        let mut meta = serde_json::Map::new();
        meta.insert("created_at".to_string(), serde_json::json!(vnode.created_at));
        meta.insert("modified_at".to_string(), serde_json::json!(vnode.updated_at));
        meta.insert("permissions".to_string(), serde_json::json!("644")); // Default
        Some(serde_json::Value::Object(meta))
    } else {
        None
    };

    let file_response = FileResponse {
        id: vnode.id.to_string(),
        name: vnode.path.file_name().unwrap_or("").to_string(),
        path: vnode.path.to_string(),
        file_type: "file".to_string(),
        size: vnode.size_bytes as u64,
        language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
        content: Some(content),
        created_at: vnode.created_at,
        updated_at: vnode.updated_at,
        modified_in_session: Some(modified_in_session),
        change_type: if modified_in_session { Some("modified".to_string()) } else { None },
        session_version,
        base_version,
        encoding: Some("utf-8".to_string()),
        line_count: Some(line_count),
        hash: Some(hash),
        metadata,
    };

    tracing::debug!(
        session_id = %session_id,
        path = %file_path,
        include_metadata = params.include_metadata,
        include_ast = params.include_ast,
        version = ?params.version,
        "Read session file"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
}

/// PUT /api/v1/sessions/{session_id}/files/:path - Write file in session
///
/// Creates or modifies a file within the session scope. Changes are isolated to
/// the session until merged.
///
/// Request Body Fields:
/// - `content` (required): File content as string
/// - `encoding` (optional): Character encoding (default: utf-8)
/// - `expected_version` (optional): Base version for optimistic locking
/// - `create_if_missing` (optional): Create new file if it doesn't exist (default: true)
/// - `metadata` (optional): Additional metadata for the file
///
/// Error Codes:
/// - `404 NOT_FOUND`: Session does not exist
/// - `409 VERSION_CONFLICT`: File version mismatch (optimistic locking failure)
/// - `413 PAYLOAD_TOO_LARGE`: File content exceeds maximum size limit
/// - `507 INSUFFICIENT_STORAGE`: Session storage quota exceeded
async fn write_session_file(
    State(ctx): State<SessionContext>,
    Path((session_id, file_path)): Path<(String, String)>,
    Json(payload): Json<UpdateFileRequest>,
) -> ApiResult<Json<ApiResponse<FileWriteResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Constants for validation
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
    const MAX_SESSION_QUOTA: u64 = 100 * 1024 * 1024; // 100 MB

    // Validate encoding
    if payload.encoding != "utf-8" {
        return Err(ApiError::BadRequest(format!(
            "Unsupported encoding: {}. Only utf-8 is currently supported.",
            payload.encoding
        )));
    }

    // Check payload size
    let content_size = payload.content.len() as u64;
    if content_size > MAX_FILE_SIZE {
        return Err(ApiError::PayloadTooLarge {
            size: content_size,
            max_size: MAX_FILE_SIZE,
            details: Some(format!("File size {} bytes exceeds maximum of {} bytes", content_size, MAX_FILE_SIZE)),
        });
    }

    // Parse session ID and get session
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session: Option<Session> = conn.connection()
        .select(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session = session.ok_or_else(||
        ApiError::NotFound(format!("Session {} not found", session_id))
    )?;

    // Get workspace ID from session
    let workspace_id = session.workspace_id.ok_or_else(||
        ApiError::BadRequest("Session has no associated workspace".to_string())
    )?;

    // Parse file path
    let path = cortex_vfs::VirtualPath::new(&file_path)
        .map_err(|e| ApiError::BadRequest(format!("Invalid path: {}", e)))?;

    // Check if file exists and handle optimistic locking
    let existing_file = ctx.vfs.metadata(&workspace_id, &path).await.ok();
    let is_new_file = existing_file.is_none();

    // Get current session modification if any
    let current_modification = get_file_modification(&ctx, &session_id, &file_path).await?;
    let current_version = current_modification.as_ref().map(|m| m.version);

    // If expected_version is specified, validate it
    if let Some(expected_version) = payload.expected_version {
        if let Some(curr_ver) = current_version {
            if expected_version != curr_ver {
                return Err(ApiError::VersionConflict {
                    expected: expected_version,
                    current: curr_ver,
                    path: file_path.clone(),
                    details: Some(serde_json::json!({
                        "session_id": session_id,
                        "message": "File has been modified in this session since expected version"
                    })),
                });
            }
        } else if expected_version != 0 {
            // If no session modification exists, expected version should be 0
            return Err(ApiError::VersionConflict {
                expected: expected_version,
                current: 0,
                path: file_path.clone(),
                details: Some(serde_json::json!({
                    "session_id": session_id,
                    "message": "File has not been modified in this session yet"
                })),
            });
        }
    }

    // If create_if_missing is false and file doesn't exist, return error
    if !payload.create_if_missing && is_new_file {
        return Err(ApiError::NotFound(format!(
            "File {} does not exist and create_if_missing is false",
            file_path
        )));
    }

    // Calculate old content for diff if file exists
    let old_content = if let Some(ref existing) = existing_file {
        ctx.vfs.read_file(&workspace_id, &path)
            .await
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    } else {
        None
    };

    // Check storage quota (simplified - in reality would check across all session files)
    let current_usage = content_size; // Simplified
    if current_usage > MAX_SESSION_QUOTA {
        return Err(ApiError::InsufficientStorage {
            used: current_usage,
            quota: MAX_SESSION_QUOTA,
            requested: content_size,
            details: Some("Session storage quota would be exceeded".to_string()),
        });
    }

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        ctx.vfs.create_directory(&workspace_id, &parent, true)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    // Write file content
    ctx.vfs.write_file(&workspace_id, &path, payload.content.as_bytes())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Get updated metadata
    let vnode = ctx.vfs.metadata(&workspace_id, &path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Calculate diff if this was a modification
    let diff = if let Some(old) = old_content {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = payload.content.lines().collect();

        // Simple diff calculation
        let lines_added = new_lines.len().saturating_sub(old_lines.len());
        let lines_removed = old_lines.len().saturating_sub(new_lines.len());
        let lines_changed = old_lines.iter().zip(new_lines.iter())
            .filter(|(a, b)| a != b)
            .count();

        Some(FileDiff {
            lines_added,
            lines_removed,
            lines_changed,
        })
    } else {
        None
    };

    // Calculate hash
    let hash = format!("sha256:{:x}", md5::compute(payload.content.as_bytes()));

    // Determine base version (version before this session started modifying)
    let base_version = if is_new_file {
        None
    } else {
        current_modification.as_ref().and_then(|m| m.base_version).or(Some(0))
    };

    // Record this modification in the session
    let change_type = if is_new_file { "created" } else { "modified" };
    let modification = record_file_modification(
        &ctx,
        &session_id,
        &file_path,
        &vnode.id.to_string(),
        change_type,
        &hash,
        vnode.size_bytes as u64,
        base_version,
    ).await?;

    let session_version = modification.version;
    let previous_version = current_version;

    let response = FileWriteResponse {
        id: vnode.id.to_string(),
        path: vnode.path.to_string(),
        change_type: if is_new_file { "created" } else { "modified" }.to_string(),
        session_version,
        base_version,
        previous_version,
        size_bytes: vnode.size_bytes as u64,
        hash,
        modified_at: vnode.updated_at,
        session_id: session_id.clone(),
        diff,
    };

    tracing::info!(
        session_id = %session_id,
        path = %file_path,
        size = vnode.size_bytes,
        change_type = %response.change_type,
        "Wrote session file"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(response, request_id, duration)))
}

// ============================================================================
// Session Merge and Locks
// ============================================================================

/// Lock database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lock {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub lock_type: LockType,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LockType {
    Exclusive,
    Shared,
}

/// Lock response
#[derive(Debug, Serialize)]
pub struct LockResponse {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub lock_type: String,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session merge request
#[derive(Debug, Deserialize)]
pub struct MergeSessionRequest {
    #[serde(default = "default_strategy")]
    pub strategy: MergeStrategy,
    #[serde(default)]
    pub conflict_resolution: std::collections::HashMap<String, String>,
}

fn default_strategy() -> MergeStrategy {
    MergeStrategy::Auto
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Auto,
    Manual,
    Theirs,
    Mine,
}

/// Merge result response
#[derive(Debug, Serialize)]
pub struct MergeResultResponse {
    pub merge_id: String,
    pub status: String,
    pub changes_merged: usize,
    pub conflicts_resolved: usize,
    pub new_version: u64,
}

/// POST /api/v1/sessions/{session_id}/merge - Merge session changes
async fn merge_session(
    State(ctx): State<SessionContext>,
    Path(session_id): Path<String>,
    Json(payload): Json<MergeSessionRequest>,
) -> ApiResult<Json<ApiResponse<MergeResultResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Parse session ID and get session
    let session_uuid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session: Option<Session> = conn.connection()
        .select(("session", session_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let session = session.ok_or_else(||
        ApiError::NotFound(format!("Session {} not found", session_id))
    )?;

    // In a real implementation, this would:
    // 1. Identify all changes made in the session
    // 2. Check for conflicts with the base workspace
    // 3. Apply the merge strategy to resolve conflicts
    // 4. Commit the changes to the base workspace
    // 5. Update the session status

    // For now, return a mock response
    let merge_id = Uuid::new_v4();
    let merge_result = MergeResultResponse {
        merge_id: merge_id.to_string(),
        status: "success".to_string(),
        changes_merged: 0, // Would count actual changes
        conflicts_resolved: 0,
        new_version: 1, // Would be the actual new version
    };

    tracing::info!(
        session_id = %session_id,
        merge_id = %merge_id,
        strategy = ?payload.strategy,
        "Merged session changes"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(merge_result, request_id, duration)))
}

/// GET /api/v1/locks - List active locks
async fn list_locks(
    State(ctx): State<SessionContext>,
) -> ApiResult<Json<ApiResponse<Vec<LockResponse>>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Query all active locks
    let query = "SELECT * FROM lock WHERE expires_at > $now ORDER BY acquired_at DESC";
    let mut response = conn.connection()
        .query(query)
        .bind(("now", Utc::now()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let locks: Vec<Lock> = response.take(0)
        .unwrap_or_default();

    let lock_responses: Vec<LockResponse> = locks
        .into_iter()
        .map(|l| LockResponse {
            id: l.id.to_string(),
            entity_type: l.entity_type,
            entity_id: l.entity_id,
            lock_type: format!("{:?}", l.lock_type).to_lowercase(),
            owner: l.owner,
            acquired_at: l.acquired_at,
            expires_at: l.expires_at,
        })
        .collect();

    tracing::debug!(count = lock_responses.len(), "Listed active locks");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(lock_responses, request_id, duration)))
}

// ============================================================================
// Session Modification Tracking Helper Functions
// ============================================================================

/// Session file modification record
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFileModification {
    id: String,
    session_id: String,
    file_path: String,
    file_id: String,
    change_type: String,
    version: u64,
    base_version: Option<u64>,
    content_hash: String,
    size_bytes: u64,
    created_at: DateTime<Utc>,
}

/// Get the latest modification record for a file in a session
async fn get_file_modification(
    ctx: &SessionContext,
    session_id: &str,
    file_path: &str,
) -> Result<Option<SessionFileModification>, ApiError> {
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = format!(
        "SELECT * FROM session_file_modifications WHERE session_id = '{}' AND file_path = '{}' ORDER BY version DESC LIMIT 1",
        session_id, file_path
    );

    let mut result = conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let modifications: Vec<SessionFileModification> = result.take(0)
        .unwrap_or_default();

    Ok(modifications.into_iter().next())
}

/// Record a file modification in a session
async fn record_file_modification(
    ctx: &SessionContext,
    session_id: &str,
    file_path: &str,
    file_id: &str,
    change_type: &str,
    content_hash: &str,
    size_bytes: u64,
    base_version: Option<u64>,
) -> Result<SessionFileModification, ApiError> {
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Get previous version number for this file in this session
    let previous_mod = get_file_modification(ctx, session_id, file_path).await?;
    let version = previous_mod.as_ref().map(|m| m.version + 1).unwrap_or(1);

    let modification_id = Uuid::new_v4().to_string();
    let modification = SessionFileModification {
        id: modification_id.clone(),
        session_id: session_id.to_string(),
        file_path: file_path.to_string(),
        file_id: file_id.to_string(),
        change_type: change_type.to_string(),
        version,
        base_version,
        content_hash: content_hash.to_string(),
        size_bytes,
        created_at: Utc::now(),
    };

    let modification_json = serde_json::to_value(&modification)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("session_file_modifications", modification_id))
        .content(modification_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(modification)
}

/// Get all modified files in a session
async fn get_session_modifications(
    ctx: &SessionContext,
    session_id: &str,
) -> Result<Vec<SessionFileModification>, ApiError> {
    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let query = format!(
        "SELECT * FROM session_file_modifications WHERE session_id = '{}' ORDER BY created_at DESC",
        session_id
    );

    let mut result = conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let modifications: Vec<SessionFileModification> = result.take(0)
        .unwrap_or_default();

    Ok(modifications)
}
