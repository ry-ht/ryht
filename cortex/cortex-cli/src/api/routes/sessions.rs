//! Session management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::{ApiResponse, CreateSessionRequest, FileResponse, SessionResponse, UpdateFileRequest},
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
        .route("/api/v3/sessions", get(list_sessions))
        .route("/api/v3/sessions", post(create_session))
        .route("/api/v3/sessions/:session_id", get(get_session))
        .route("/api/v3/sessions/:session_id", delete(delete_session))
        .route("/api/v3/sessions/:session_id/merge", post(merge_session))
        .route("/api/v3/locks", get(list_locks))
        // Session-aware VFS operations (critical for multi-agent coordination)
        .route("/api/v3/sessions/:session_id/files", get(list_session_files))
        .route("/api/v3/sessions/:session_id/files/:path", get(read_session_file))
        .route("/api/v3/sessions/:session_id/files/:path", put(write_session_file))
        .with_state(context)
}

/// GET /api/v3/sessions - List all sessions
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

/// GET /api/v3/sessions/:session_id - Get session details
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

/// POST /api/v3/sessions - Create session
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

/// DELETE /api/v3/sessions/:session_id - Delete session
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
    #[serde(default)]
    recursive: bool,
}

/// GET /api/v3/sessions/:session_id/files - List files in session scope
async fn list_session_files(
    State(ctx): State<SessionContext>,
    Path(session_id): Path<String>,
    Query(params): Query<FileListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<FileResponse>>>> {
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

    // List files from VFS
    let root_path = cortex_vfs::VirtualPath::root();
    let vnodes = ctx.vfs.list_directory(&workspace_id, &root_path, params.recursive)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let files: Vec<FileResponse> = vnodes
        .into_iter()
        .filter(|vnode| vnode.is_file()) // Only include files
        .map(|vnode| FileResponse {
            id: vnode.id.to_string(),
            name: vnode.path.file_name().unwrap_or("").to_string(),
            path: vnode.path.to_string(),
            file_type: "file".to_string(),
            size: vnode.size_bytes as u64,
            language: vnode.language.map(|l| format!("{:?}", l).to_lowercase()),
            content: None,
            created_at: vnode.created_at,
            updated_at: vnode.updated_at,
        })
        .collect();

    tracing::debug!(
        session_id = %session_id,
        workspace_id = %workspace_id,
        file_count = files.len(),
        "Listed session files"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(files, request_id, duration)))
}

/// GET /api/v3/sessions/:session_id/files/:path - Read file from session
async fn read_session_file(
    State(ctx): State<SessionContext>,
    Path((session_id, file_path)): Path<(String, String)>,
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

    let content = String::from_utf8(content_bytes)
        .map_err(|_| ApiError::Internal("File contains invalid UTF-8".to_string()))?;

    // Get metadata
    let vnode = ctx.vfs.metadata(&workspace_id, &path)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

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
    };

    tracing::debug!(
        session_id = %session_id,
        path = %file_path,
        "Read session file"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
}

/// PUT /api/v3/sessions/:session_id/files/:path - Write file in session
async fn write_session_file(
    State(ctx): State<SessionContext>,
    Path((session_id, file_path)): Path<(String, String)>,
    Json(payload): Json<UpdateFileRequest>,
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
    };

    tracing::info!(
        session_id = %session_id,
        path = %file_path,
        size = vnode.size_bytes,
        "Wrote session file"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(file_response, request_id, duration)))
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

/// POST /api/v3/sessions/:session_id/merge - Merge session changes
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

/// GET /api/v3/locks - List active locks
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
