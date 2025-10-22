//! Task and workflow management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    types::ApiResponse,
};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Task context
#[derive(Clone)]
pub struct TaskContext {
    pub storage: Arc<ConnectionManager>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

/// Task priority enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Task database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assigned_to: Vec<String>,
    pub estimated_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub progress: f64,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub spec_reference: Option<SpecReference>,
    pub completion_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Spec reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecReference {
    pub spec_name: String,
    pub section: String,
}

/// Task response
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Vec<String>,
    pub estimated_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub progress: f64,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    pub spec_reference: Option<SpecReference>,
    pub completion_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create task request
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: TaskPriority,
    pub estimated_hours: Option<f64>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub spec_reference: Option<SpecReference>,
}

fn default_priority() -> TaskPriority {
    TaskPriority::Medium
}

/// Update task request
#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub assigned_to: Option<Vec<String>>,
    pub estimated_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub progress: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub completion_note: Option<String>,
}

/// Task list query parameters
#[derive(Debug, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub assigned_to: Option<String>,
    pub tags: Option<String>, // Comma-separated
}

/// Create task routes
pub fn task_routes(context: TaskContext) -> Router {
    Router::new()
        .route("/api/v1/tasks", get(list_tasks))
        .route("/api/v1/tasks", post(create_task))
        .route("/api/v1/tasks/:id", get(get_task))
        .route("/api/v1/tasks/:id", put(update_task))
        .route("/api/v1/tasks/:id", delete(delete_task))
        .with_state(context)
}

/// GET /api/v1/tasks - List tasks
async fn list_tasks(
    State(ctx): State<TaskContext>,
    Query(params): Query<TaskListQuery>,
) -> ApiResult<Json<ApiResponse<Vec<TaskResponse>>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Build query based on filters
    let mut query = "SELECT * FROM task".to_string();
    let mut conditions = Vec::new();

    if let Some(status) = &params.status {
        conditions.push(format!("status = '{:?}'", status).to_lowercase());
    }

    if let Some(priority) = &params.priority {
        conditions.push(format!("priority = '{:?}'", priority).to_lowercase());
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" ORDER BY created_at DESC");

    let mut response = conn.connection()
        .query(&query)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let tasks: Vec<Task> = response.take(0)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Filter by assigned_to if specified
    let filtered_tasks: Vec<Task> = if let Some(assigned_to) = &params.assigned_to {
        tasks.into_iter()
            .filter(|t| t.assigned_to.contains(assigned_to))
            .collect()
    } else {
        tasks
    };

    // Filter by tags if specified
    let final_tasks: Vec<Task> = if let Some(tags_str) = &params.tags {
        let required_tags: Vec<&str> = tags_str.split(',').collect();
        filtered_tasks.into_iter()
            .filter(|t| required_tags.iter().any(|tag| t.tags.contains(&tag.to_string())))
            .collect()
    } else {
        filtered_tasks
    };

    let task_responses: Vec<TaskResponse> = final_tasks
        .into_iter()
        .map(|t| TaskResponse {
            id: t.id.to_string(),
            title: t.title,
            description: t.description,
            status: format!("{:?}", t.status).to_lowercase(),
            priority: format!("{:?}", t.priority).to_lowercase(),
            assigned_to: t.assigned_to,
            estimated_hours: t.estimated_hours,
            actual_hours: t.actual_hours,
            progress: t.progress,
            tags: t.tags,
            dependencies: t.dependencies,
            spec_reference: t.spec_reference,
            completion_note: t.completion_note,
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    tracing::debug!(count = task_responses.len(), "Listed tasks");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(task_responses, request_id, duration)))
}

/// GET /api/v1/tasks/:id - Get task details
async fn get_task(
    State(ctx): State<TaskContext>,
    Path(task_id): Path<String>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let task_uuid = Uuid::parse_str(&task_id)
        .map_err(|_| ApiError::BadRequest("Invalid task ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let task: Option<Task> = conn.connection()
        .select(("task", task_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let task = task.ok_or_else(||
        ApiError::NotFound(format!("Task {} not found", task_id))
    )?;

    let task_response = TaskResponse {
        id: task.id.to_string(),
        title: task.title,
        description: task.description,
        status: format!("{:?}", task.status).to_lowercase(),
        priority: format!("{:?}", task.priority).to_lowercase(),
        assigned_to: task.assigned_to,
        estimated_hours: task.estimated_hours,
        actual_hours: task.actual_hours,
        progress: task.progress,
        tags: task.tags,
        dependencies: task.dependencies,
        spec_reference: task.spec_reference,
        completion_note: task.completion_note,
        created_at: task.created_at,
        updated_at: task.updated_at,
    };

    tracing::debug!(task_id = %task_id, "Retrieved task details");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(task_response, request_id, duration)))
}

/// POST /api/v1/tasks - Create task
async fn create_task(
    State(ctx): State<TaskContext>,
    Json(payload): Json<CreateTaskRequest>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let task_id = Uuid::new_v4();
    let now = Utc::now();

    let task = Task {
        id: task_id,
        title: payload.title.clone(),
        description: payload.description.clone(),
        status: TaskStatus::Pending,
        priority: payload.priority.clone(),
        assigned_to: Vec::new(),
        estimated_hours: payload.estimated_hours,
        actual_hours: None,
        progress: 0.0,
        tags: payload.tags.clone(),
        dependencies: Vec::new(),
        spec_reference: payload.spec_reference.clone(),
        completion_note: None,
        created_at: now,
        updated_at: now,
    };

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let task_json = serde_json::to_value(&task)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .create(("task", task_id.to_string()))
        .content(task_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let task_response = TaskResponse {
        id: task.id.to_string(),
        title: task.title,
        description: task.description,
        status: format!("{:?}", task.status).to_lowercase(),
        priority: format!("{:?}", task.priority).to_lowercase(),
        assigned_to: task.assigned_to,
        estimated_hours: task.estimated_hours,
        actual_hours: task.actual_hours,
        progress: task.progress,
        tags: task.tags,
        dependencies: task.dependencies,
        spec_reference: task.spec_reference,
        completion_note: task.completion_note,
        created_at: task.created_at,
        updated_at: task.updated_at,
    };

    tracing::info!(
        task_id = %task_id,
        title = %payload.title,
        "Created task"
    );

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(task_response, request_id, duration)))
}

/// PUT /api/v1/tasks/:id - Update task
async fn update_task(
    State(ctx): State<TaskContext>,
    Path(task_id): Path<String>,
    Json(payload): Json<UpdateTaskRequest>,
) -> ApiResult<Json<ApiResponse<TaskResponse>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let task_uuid = Uuid::parse_str(&task_id)
        .map_err(|_| ApiError::BadRequest("Invalid task ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Retrieve existing task
    let task: Option<Task> = conn.connection()
        .select(("task", task_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut task = task.ok_or_else(||
        ApiError::NotFound(format!("Task {} not found", task_id))
    )?;

    // Update fields
    if let Some(title) = payload.title {
        task.title = title;
    }
    if let Some(description) = payload.description {
        task.description = description;
    }
    if let Some(status) = payload.status {
        task.status = status;
    }
    if let Some(priority) = payload.priority {
        task.priority = priority;
    }
    if let Some(assigned_to) = payload.assigned_to {
        task.assigned_to = assigned_to;
    }
    if let Some(estimated_hours) = payload.estimated_hours {
        task.estimated_hours = Some(estimated_hours);
    }
    if let Some(actual_hours) = payload.actual_hours {
        task.actual_hours = Some(actual_hours);
    }
    if let Some(progress) = payload.progress {
        task.progress = progress;
    }
    if let Some(tags) = payload.tags {
        task.tags = tags;
    }
    if let Some(completion_note) = payload.completion_note {
        task.completion_note = Some(completion_note);
    }

    task.updated_at = Utc::now();

    // Save updated task
    let task_json = serde_json::to_value(&task)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<serde_json::Value> = conn.connection()
        .update(("task", task_uuid.to_string()))
        .content(task_json)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let task_response = TaskResponse {
        id: task.id.to_string(),
        title: task.title,
        description: task.description,
        status: format!("{:?}", task.status).to_lowercase(),
        priority: format!("{:?}", task.priority).to_lowercase(),
        assigned_to: task.assigned_to,
        estimated_hours: task.estimated_hours,
        actual_hours: task.actual_hours,
        progress: task.progress,
        tags: task.tags,
        dependencies: task.dependencies,
        spec_reference: task.spec_reference,
        completion_note: task.completion_note,
        created_at: task.created_at,
        updated_at: task.updated_at,
    };

    tracing::info!(task_id = %task_id, "Updated task");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success(task_response, request_id, duration)))
}

/// DELETE /api/v1/tasks/:id - Delete task
async fn delete_task(
    State(ctx): State<TaskContext>,
    Path(task_id): Path<String>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    let task_uuid = Uuid::parse_str(&task_id)
        .map_err(|_| ApiError::BadRequest("Invalid task ID".to_string()))?;

    let conn = ctx.storage.acquire().await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let _: Option<Task> = conn.connection()
        .delete(("task", task_uuid.to_string()))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(task_id = %task_id, "Deleted task");

    let duration = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::success((), request_id, duration)))
}
