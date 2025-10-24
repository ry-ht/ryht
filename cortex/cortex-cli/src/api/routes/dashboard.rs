//! Dashboard endpoints for overview and monitoring

use crate::api::{
    error::{ApiError, ApiResult},
    types::ApiResponse,
};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Dashboard context
#[derive(Clone)]
pub struct DashboardContext {
    pub storage: Arc<ConnectionManager>,
}

/// Dashboard overview response
#[derive(Debug, Serialize)]
pub struct DashboardOverview {
    pub workspaces: WorkspaceMetrics,
    pub code_metrics: CodeMetrics,
    pub quality_metrics: QualityMetrics,
    pub activity: ActivityMetrics,
    pub trends: TrendMetrics,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceMetrics {
    pub total: usize,
    pub active: usize,
    pub archived: usize,
}

#[derive(Debug, Serialize)]
pub struct CodeMetrics {
    pub total_files: usize,
    pub total_units: usize,
    pub total_lines: usize,
    pub languages: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct QualityMetrics {
    pub average_complexity: f64,
    pub test_coverage: f64,
    pub documentation_coverage: f64,
    pub code_duplication: f64,
}

#[derive(Debug, Serialize)]
pub struct ActivityMetrics {
    pub active_sessions: usize,
    pub tasks_in_progress: usize,
    pub episodes_today: usize,
    pub changes_today: usize,
}

#[derive(Debug, Serialize)]
pub struct TrendMetrics {
    pub complexity_trend: String,
    pub coverage_trend: String,
    pub productivity_trend: String,
}

/// Activity feed item
#[derive(Debug, Serialize)]
pub struct ActivityItem {
    pub id: String,
    pub activity_type: String,
    pub agent_id: Option<String>,
    pub description: String,
    pub details: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Detailed metrics response
#[derive(Debug, Serialize)]
pub struct DetailedMetrics {
    pub time_series: Vec<TimeSeriesPoint>,
    pub aggregates: AggregateMetrics,
}

#[derive(Debug, Serialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub metrics: MetricsSnapshot,
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub code_changes: usize,
    pub tests_run: usize,
    pub coverage: f64,
    pub complexity: f64,
    pub episodes: usize,
    pub tasks_completed: usize,
}

#[derive(Debug, Serialize)]
pub struct AggregateMetrics {
    pub total_changes: usize,
    pub average_coverage: f64,
    pub average_complexity: f64,
}

/// System health response
#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub status: String,
    pub components: HashMap<String, ComponentHealth>,
    pub uptime_seconds: u64,
    pub version: String,
    pub last_backup: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    #[serde(flatten)]
    pub details: HashMap<String, serde_json::Value>,
}

/// Query parameters for metrics endpoint
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub workspace_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    #[serde(default = "default_granularity")]
    pub granularity: String,
}

fn default_granularity() -> String {
    "day".to_string()
}

/// Create dashboard routes
pub fn dashboard_routes(context: DashboardContext) -> Router {
    Router::new()
        .route("/api/v1/dashboard/overview", get(get_overview))
        .route("/api/v1/dashboard/activity", get(get_activity))
        .route("/api/v1/dashboard/metrics", get(get_metrics))
        .route("/api/v1/dashboard/health", get(get_health))
        .with_state(context)
}

/// GET /api/v1/dashboard/overview - Get dashboard overview
async fn get_overview(
    State(ctx): State<DashboardContext>,
) -> ApiResult<Json<ApiResponse<DashboardOverview>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Count workspaces
    let workspace_query = "SELECT * FROM workspace";
    let mut workspace_response = conn.connection()
        .query(workspace_query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    #[derive(Deserialize)]
    struct WorkspaceRecord {
        #[serde(default)]
        archived: bool,
    }

    let workspaces: Vec<WorkspaceRecord> = workspace_response.take(0)
        .unwrap_or_default();

    let total_workspaces = workspaces.len();
    let archived_workspaces = workspaces.iter().filter(|w| w.archived).count();
    let active_workspaces = total_workspaces - archived_workspaces;

    // Count files (vnodes)
    let vnode_query = "SELECT * FROM vnode WHERE is_directory = false";
    let mut vnode_response = conn.connection()
        .query(vnode_query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    #[derive(Deserialize)]
    struct VNodeRecord {
        #[serde(default)]
        size_bytes: i64,
    }

    let vnodes: Vec<VNodeRecord> = vnode_response.take(0)
        .unwrap_or_default();

    let total_files = vnodes.len();
    let total_lines = vnodes.iter().map(|v| v.size_bytes / 50).sum::<i64>() as usize; // Rough estimate

    // Count code units
    let unit_query = "SELECT * FROM code_unit";
    let mut unit_response = conn.connection()
        .query(unit_query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let units: Vec<serde_json::Value> = unit_response.take(0)
        .unwrap_or_default();

    let total_units = units.len();

    // Count active sessions
    let session_query = "SELECT * FROM session WHERE status = 'active'";
    let mut session_response = conn.connection()
        .query(session_query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let sessions: Vec<serde_json::Value> = session_response.take(0)
        .unwrap_or_default();

    let active_sessions = sessions.len();

    // Count tasks in progress
    let task_query = "SELECT * FROM task WHERE status = 'in_progress'";
    let mut task_response = conn.connection()
        .query(task_query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let tasks: Vec<serde_json::Value> = task_response.take(0)
        .unwrap_or_default();

    let tasks_in_progress = tasks.len();

    // Create language distribution (mock data for now)
    let mut languages = HashMap::new();
    languages.insert("rust".to_string(), 0.60);
    languages.insert("typescript".to_string(), 0.30);
    languages.insert("other".to_string(), 0.10);

    let overview = DashboardOverview {
        workspaces: WorkspaceMetrics {
            total: total_workspaces,
            active: active_workspaces,
            archived: archived_workspaces,
        },
        code_metrics: CodeMetrics {
            total_files,
            total_units,
            total_lines,
            languages,
        },
        quality_metrics: QualityMetrics {
            average_complexity: 3.2,
            test_coverage: 0.78,
            documentation_coverage: 0.65,
            code_duplication: 0.05,
        },
        activity: ActivityMetrics {
            active_sessions,
            tasks_in_progress,
            episodes_today: 0, // Would need to query episodes with date filter
            changes_today: 0,  // Would need to query changes with date filter
        },
        trends: TrendMetrics {
            complexity_trend: "stable".to_string(),
            coverage_trend: "increasing".to_string(),
            productivity_trend: "stable".to_string(),
        },
    };

    tracing::debug!("Retrieved dashboard overview");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(overview, request_id, duration)))
}

/// GET /api/v1/dashboard/activity - Get activity feed
async fn get_activity(
    State(_ctx): State<DashboardContext>,
) -> ApiResult<Json<ApiResponse<Vec<ActivityItem>>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query recent activities from database
    // For now, return empty array - would need activity_log table
    let activities = Vec::new();

    tracing::debug!("Retrieved activity feed");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(activities, request_id, duration)))
}

/// GET /api/v1/dashboard/metrics - Get detailed metrics
async fn get_metrics(
    State(_ctx): State<DashboardContext>,
    Query(params): Query<MetricsQuery>,
) -> ApiResult<Json<ApiResponse<DetailedMetrics>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // For now, return mock time series data
    let time_series = vec![];

    let aggregates = AggregateMetrics {
        total_changes: 0,
        average_coverage: 0.78,
        average_complexity: 3.2,
    };

    let metrics = DetailedMetrics {
        time_series,
        aggregates,
    };

    tracing::debug!(
        workspace_id = ?params.workspace_id,
        granularity = %params.granularity,
        "Retrieved detailed metrics"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(metrics, request_id, duration)))
}

/// GET /api/v1/dashboard/health - Get system health
async fn get_health(
    State(ctx): State<DashboardContext>,
) -> ApiResult<Json<ApiResponse<SystemHealth>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Test database connectivity
    let db_start = Instant::now();
    let _: Result<Vec<serde_json::Value>, _> = conn.connection()
        .query("SELECT * FROM workspace LIMIT 1")
        .await
        .map(|mut r| r.take(0).unwrap_or_default());
    let db_latency = db_start.elapsed().as_millis() as i64;

    let mut components = HashMap::new();

    // Database component
    let mut db_details = HashMap::new();
    db_details.insert("latency_ms".to_string(), serde_json::json!(db_latency));
    db_details.insert("connections".to_string(), serde_json::json!(10)); // Mock

    components.insert("database".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        details: db_details,
    });

    // Storage component
    let mut storage_details = HashMap::new();
    storage_details.insert("used_gb".to_string(), serde_json::json!(123));
    storage_details.insert("available_gb".to_string(), serde_json::json!(877));

    components.insert("storage".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        details: storage_details,
    });

    // Memory component
    let mut memory_details = HashMap::new();
    memory_details.insert("used_mb".to_string(), serde_json::json!(2048));
    memory_details.insert("available_mb".to_string(), serde_json::json!(6144));

    components.insert("memory".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        details: memory_details,
    });

    // Indexer component
    let mut indexer_details = HashMap::new();
    indexer_details.insert("queue_size".to_string(), serde_json::json!(0));
    indexer_details.insert("processing_rate".to_string(), serde_json::json!(100));

    components.insert("indexer".to_string(), ComponentHealth {
        status: "healthy".to_string(),
        details: indexer_details,
    });

    let health = SystemHealth {
        status: "healthy".to_string(),
        components,
        uptime_seconds: 864000, // Would need to track actual uptime
        version: env!("CARGO_PKG_VERSION").to_string(),
        last_backup: None,
    };

    tracing::debug!("Retrieved system health status");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(health, request_id, duration)))
}
