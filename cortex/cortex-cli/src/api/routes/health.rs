//! Health check endpoints

use crate::api::{
    error::ApiResult,
    types::{ApiResponse, HealthResponse, DatabaseHealth, MemoryHealth, MetricsResponse},
};
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;

/// Application state with database connection
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub storage: Arc<ConnectionManager>,
}

/// Create health check routes
pub fn health_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v3/health", get(health_check))
        .route("/api/v3/metrics", get(metrics))
        .with_state(state)
}

/// Health check endpoint
async fn health_check(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<HealthResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Check database connectivity
    let db_start = Instant::now();
    let db_connected = match state.storage.acquire().await {
        Ok(conn) => {
            // Try a simple query to verify connection
            match conn.connection().query("SELECT * FROM ONLY $tb LIMIT 1").bind(("tb", "workspace")).await {
                Ok(_) => true,
                Err(e) => {
                    tracing::warn!("Database query failed: {}", e);
                    false
                }
            }
        }
        Err(e) => {
            tracing::warn!("Database connection failed: {}", e);
            false
        }
    };
    let db_response_time = db_start.elapsed().as_millis() as u64;

    // Get memory usage (approximation using process memory)
    let memory_info = if let Ok(usage) = sys_info::mem_info() {
        MemoryHealth {
            total_bytes: usage.total * 1024, // Convert KB to bytes
            used_bytes: (usage.total - usage.avail) * 1024,
        }
    } else {
        MemoryHealth {
            total_bytes: 0,
            used_bytes: 0,
        }
    };

    // Determine overall status
    let status = if db_connected {
        "healthy".to_string()
    } else {
        "unhealthy".to_string()
    };

    let health = HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        database: DatabaseHealth {
            connected: db_connected,
            response_time_ms: db_response_time,
        },
        memory: memory_info,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(health, request_id, duration)))
}

/// Metrics endpoint
async fn metrics(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<MetricsResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    // Gather actual metrics from database
    let conn = state.storage.acquire().await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    // Count workspaces
    let workspace_query = "SELECT count() as count FROM workspace GROUP ALL";
    let mut ws_response = conn.connection()
        .query(workspace_query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let workspace_results: Vec<serde_json::Value> = ws_response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;
    let workspaces = workspace_results.first()
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0) as usize;

    // Count files (vnodes)
    let vnode_query = "SELECT count() as count FROM vnode WHERE status != 'deleted' GROUP ALL";
    let mut vnode_response = conn.connection()
        .query(vnode_query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let vnode_results: Vec<serde_json::Value> = vnode_response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;
    let files = vnode_results.first()
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0) as usize;

    // Calculate total size
    let size_query = "SELECT math::sum(size_bytes) as total_size FROM vnode WHERE status != 'deleted' GROUP ALL";
    let mut size_response = conn.connection()
        .query(size_query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let size_results: Vec<serde_json::Value> = size_response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;
    let total_size_bytes = size_results.first()
        .and_then(|v| v.get("total_size").and_then(|s| s.as_u64()))
        .unwrap_or(0);

    // Count episodes
    let episode_query = "SELECT count() as count FROM episode GROUP ALL";
    let mut episode_response = conn.connection()
        .query(episode_query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let episode_results: Vec<serde_json::Value> = episode_response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;
    let episodes = episode_results.first()
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0) as usize;

    // Count semantic nodes
    let semantic_query = "SELECT count() as count FROM semantic_node GROUP ALL";
    let mut semantic_response = conn.connection()
        .query(semantic_query)
        .await
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;

    let semantic_results: Vec<serde_json::Value> = semantic_response.take(0)
        .map_err(|e| crate::api::error::ApiError::Internal(e.to_string()))?;
    let semantic_nodes = semantic_results.first()
        .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
        .unwrap_or(0) as usize;

    let metrics = MetricsResponse {
        workspaces,
        files,
        total_size_bytes,
        episodes,
        semantic_nodes,
    };

    tracing::debug!(
        workspaces = metrics.workspaces,
        files = metrics.files,
        episodes = metrics.episodes,
        "Collected system metrics"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(metrics, request_id, duration)))
}
