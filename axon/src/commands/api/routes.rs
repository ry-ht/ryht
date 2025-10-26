//! API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::error::ApiError;
use super::websocket::WsManager;
use crate::commands::runtime_manager::AgentRuntimeManager;

/// Application state shared across routes
#[derive(Clone)]
pub struct AppState {
    pub runtime: Arc<tokio::sync::RwLock<AgentRuntimeManager>>,
    pub ws_manager: WsManager,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub active_agents: usize,
    pub running_workflows: usize,
    pub websocket_connections: usize,
}

/// API info response
#[derive(Debug, Serialize)]
pub struct ApiInfoResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub endpoints: Vec<EndpointInfo>,
}

#[derive(Debug, Serialize)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub description: String,
}

/// Create all API routes
pub fn create_routes(state: AppState) -> Router {
    Router::new()
        // API info and health
        .route("/", get(api_info))
        .route("/health", get(health))
        .route("/status", get(system_status))

        // Agent management
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/{id}", get(get_agent).delete(delete_agent).put(update_agent))
        .route("/agents/{id}/pause", post(pause_agent))
        .route("/agents/{id}/resume", post(resume_agent))
        .route("/agents/{id}/restart", post(restart_agent))
        .route("/agents/{id}/logs", get(get_agent_logs))

        // Workflow management
        .route("/workflows", get(list_workflows).post(run_workflow))
        .route("/workflows/{id}", get(get_workflow).delete(cancel_workflow))
        .route("/workflows/{id}/cancel", post(cancel_workflow))
        .route("/workflows/{id}/pause", post(pause_workflow))
        .route("/workflows/{id}/resume", post(resume_workflow))

        // Monitoring and metrics
        .route("/metrics", get(get_metrics))
        .route("/metrics/export", post(export_metrics))
        .route("/telemetry", get(get_telemetry))
        .route("/telemetry/summary", get(telemetry_summary))

        // Configuration
        .route("/config", get(get_config).put(update_config))
        .route("/config/validate", post(validate_config))

        .with_state(state)
}

/// API info endpoint
async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        name: "Axon Multi-Agent API".to_string(),
        version: crate::VERSION.to_string(),
        description: "REST API for managing multi-agent systems".to_string(),
        endpoints: vec![
            EndpointInfo {
                path: "/health".to_string(),
                method: "GET".to_string(),
                description: "Health check".to_string(),
            },
            EndpointInfo {
                path: "/agents".to_string(),
                method: "GET".to_string(),
                description: "List all agents".to_string(),
            },
            EndpointInfo {
                path: "/agents".to_string(),
                method: "POST".to_string(),
                description: "Create a new agent".to_string(),
            },
            EndpointInfo {
                path: "/workflows".to_string(),
                method: "POST".to_string(),
                description: "Execute a workflow".to_string(),
            },
            EndpointInfo {
                path: "/metrics".to_string(),
                method: "GET".to_string(),
                description: "Get system metrics".to_string(),
            },
        ],
    })
}

/// Health check endpoint
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let runtime = state.runtime.read().await;
    let system_status = runtime.get_system_status().await.ok();
    let ws_count = state.ws_manager.connection_count().await;

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: crate::VERSION.to_string(),
        uptime_seconds: 0, // TODO: Track server start time
        active_agents: system_status.as_ref().map(|s| s.active_agents).unwrap_or(0),
        running_workflows: system_status.as_ref().map(|s| s.running_workflows).unwrap_or(0),
        websocket_connections: ws_count,
    })
}

/// List all agents
async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::agents::AgentInfo>>, ApiError> {
    let runtime = state.runtime.read().await;
    let agents = runtime.list_agents(None).await?;
    Ok(Json(agents))
}

/// Create a new agent
#[derive(Debug, Deserialize)]
struct CreateAgentRequest {
    name: String,
    agent_type: crate::agents::AgentType,
    capabilities: Vec<String>,
    max_concurrent_tasks: Option<usize>,
}

#[derive(Debug, Serialize)]
struct CreateAgentResponse {
    id: String,
    name: String,
}

async fn create_agent(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<CreateAgentResponse>, ApiError> {
    // Convert string capabilities to Capability enum
    let capabilities: std::collections::HashSet<_> = req.capabilities
        .into_iter()
        .filter_map(|s| {
            // Try to parse the string as a Capability
            // For now, just use a simple mapping - in production you'd want proper parsing
            match s.to_lowercase().as_str() {
                "code_generation" | "coding" => Some(crate::agents::Capability::CodeGeneration),
                "code_review" | "review" => Some(crate::agents::Capability::CodeReview),
                "testing" | "test" => Some(crate::agents::Capability::Testing),
                "documentation" | "docs" => Some(crate::agents::Capability::Documentation),
                "debugging" => Some(crate::agents::Capability::Debugging),
                "analysis" => Some(crate::agents::Capability::CodeAnalysis),
                _ => None,
            }
        })
        .collect();

    let config = crate::agents::AgentConfig {
        name: req.name.clone(),
        agent_type: req.agent_type,
        capabilities,
        max_concurrent_tasks: req.max_concurrent_tasks.unwrap_or(1),
        task_timeout_seconds: 300,
        custom_config: std::collections::HashMap::new(),
    };

    let mut runtime = state.runtime.write().await;
    let agent_id = runtime.start_agent(config).await?;

    Ok(Json(CreateAgentResponse {
        id: agent_id.to_string(),
        name: req.name,
    }))
}

/// Get agent by ID
async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::agents::AgentInfo>, ApiError> {
    let runtime = state.runtime.read().await;
    let agent = runtime.get_agent_info(&id).await?;
    Ok(Json(agent))
}

/// Delete (stop) an agent
async fn delete_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.stop_agent(&id, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Pause an agent
async fn pause_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.pause_agent(&id).await?;
    Ok(StatusCode::OK)
}

/// Resume an agent
async fn resume_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.resume_agent(&id).await?;
    Ok(StatusCode::OK)
}

/// List workflows
async fn list_workflows(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::commands::output::WorkflowInfo>>, ApiError> {
    let runtime = state.runtime.read().await;
    let workflows = runtime.list_workflows(None).await?;
    Ok(Json(workflows))
}

/// Run a workflow
#[derive(Debug, Deserialize)]
struct RunWorkflowRequest {
    workflow_def: String,
    input_params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RunWorkflowResponse {
    workflow_id: String,
}

async fn run_workflow(
    State(state): State<AppState>,
    Json(req): Json<RunWorkflowRequest>,
) -> Result<Json<RunWorkflowResponse>, ApiError> {
    let runtime = state.runtime.read().await;
    let workflow_id = runtime.execute_workflow(&req.workflow_def, req.input_params).await?;
    Ok(Json(RunWorkflowResponse { workflow_id }))
}

/// Get workflow status
async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::commands::output::WorkflowStatus>, ApiError> {
    let runtime = state.runtime.read().await;
    let status = runtime.get_workflow_status(&id).await?;
    Ok(Json(status))
}

/// Cancel a workflow
async fn cancel_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.cancel_workflow(&id).await?;
    Ok(StatusCode::OK)
}

/// Get metrics
async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<std::collections::HashMap<String, crate::commands::output::MetricsData>>, ApiError> {
    let runtime = state.runtime.read().await;
    let metrics = runtime.get_metrics(None).await?;
    Ok(Json(metrics))
}

/// Get telemetry
#[derive(Debug, Deserialize)]
struct TelemetryQuery {
    range: Option<u64>,
}

async fn get_telemetry(
    State(state): State<AppState>,
    Query(params): Query<TelemetryQuery>,
) -> Result<Json<crate::commands::output::TelemetryData>, ApiError> {
    let runtime = state.runtime.read().await;
    let range = params.range.unwrap_or(60);
    let telemetry = runtime.get_telemetry(range).await?;
    Ok(Json(telemetry))
}

/// System status
async fn system_status(
    State(state): State<AppState>,
) -> Result<Json<crate::commands::output::SystemStatus>, ApiError> {
    let runtime = state.runtime.read().await;
    let status = runtime.get_system_status().await?;
    Ok(Json(status))
}

/// Update agent configuration
#[derive(Debug, Deserialize)]
struct UpdateAgentRequest {
    max_concurrent_tasks: Option<usize>,
    task_timeout_seconds: Option<u64>,
}

async fn update_agent(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    Json(_req): Json<UpdateAgentRequest>,
) -> Result<StatusCode, ApiError> {
    // TODO: Implement agent configuration updates
    Ok(StatusCode::OK)
}

/// Restart an agent
async fn restart_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;

    // Get agent info before stopping
    let agent_info = runtime.get_agent_info(&id).await?;

    // Stop the agent
    runtime.stop_agent(&id, false).await?;

    // Recreate agent config
    let config = crate::agents::AgentConfig {
        name: agent_info.name,
        agent_type: agent_info.agent_type,
        capabilities: agent_info.capabilities.into_iter().collect(),
        max_concurrent_tasks: agent_info.metadata.max_concurrent_tasks,
        task_timeout_seconds: 300,
        custom_config: std::collections::HashMap::new(),
    };

    // Start new agent
    runtime.start_agent(config).await?;

    Ok(StatusCode::OK)
}

/// Get agent logs
#[derive(Debug, Deserialize)]
struct LogsQuery {
    lines: Option<usize>,
    follow: Option<bool>,
}

#[derive(Debug, Serialize)]
struct LogsResponse {
    logs: Vec<String>,
}

async fn get_agent_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, ApiError> {
    let runtime = state.runtime.read().await;
    let lines = params.lines.unwrap_or(100);
    let _follow = params.follow.unwrap_or(false);

    // Verify agent exists
    runtime.get_agent_info(&id).await?;

    // Get the log file path from the config
    let config = crate::commands::config::AxonConfig::load()
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let log_file = config.logs_dir().join(format!("{}.log", id));

    let logs = if log_file.exists() {
        let content = std::fs::read_to_string(&log_file)
            .map_err(|e| ApiError::Internal(format!("Failed to read log file: {}", e)))?;
        let log_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let start = log_lines.len().saturating_sub(lines);
        log_lines[start..].to_vec()
    } else {
        vec!["No logs available yet".to_string()]
    };

    Ok(Json(LogsResponse { logs }))
}

/// Pause a workflow
async fn pause_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.pause_workflow(&id).await?;
    Ok(StatusCode::OK)
}

/// Resume a workflow
async fn resume_workflow(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mut runtime = state.runtime.write().await;
    runtime.resume_workflow(&id).await?;
    Ok(StatusCode::OK)
}

/// Export metrics
#[derive(Debug, Deserialize)]
struct ExportMetricsRequest {
    format: Option<String>,
    output_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportMetricsResponse {
    success: bool,
    path: String,
    format: String,
}

async fn export_metrics(
    State(state): State<AppState>,
    Json(req): Json<ExportMetricsRequest>,
) -> Result<Json<ExportMetricsResponse>, ApiError> {
    let runtime = state.runtime.read().await;
    let format = req.format.unwrap_or_else(|| "json".to_string());
    let output_path = req.output_path.unwrap_or_else(|| {
        format!("/tmp/axon-metrics-{}.{}", chrono::Utc::now().timestamp(), format)
    });

    runtime
        .export_metrics(&std::path::PathBuf::from(&output_path), &format)
        .await?;

    Ok(Json(ExportMetricsResponse {
        success: true,
        path: output_path,
        format,
    }))
}

/// Get telemetry summary
#[derive(Debug, Serialize)]
struct TelemetrySummary {
    total_requests: u64,
    success_rate: f64,
    avg_response_time_ms: u64,
    error_count: u64,
}

async fn telemetry_summary(
    State(state): State<AppState>,
) -> Result<Json<TelemetrySummary>, ApiError> {
    let runtime = state.runtime.read().await;
    let telemetry = runtime.get_telemetry(60).await?;

    let success_rate = if telemetry.total_requests > 0 {
        (telemetry.successful_requests as f64 / telemetry.total_requests as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(TelemetrySummary {
        total_requests: telemetry.total_requests,
        success_rate,
        avg_response_time_ms: telemetry.avg_response_time_ms,
        error_count: telemetry.failed_requests,
    }))
}

/// Get configuration
#[derive(Debug, Serialize)]
struct ConfigResponse {
    workspace_name: String,
    workspace_path: String,
}

async fn get_config(
    State(_state): State<AppState>,
) -> Result<Json<ConfigResponse>, ApiError> {
    let config = crate::commands::config::AxonConfig::load()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ConfigResponse {
        workspace_name: config.workspace_name,
        workspace_path: config.workspace_path.to_string_lossy().to_string(),
    }))
}

/// Update configuration
#[derive(Debug, Deserialize)]
struct UpdateConfigRequest {
    workspace_name: Option<String>,
    workspace_path: Option<String>,
    server_host: Option<String>,
    server_port: Option<u16>,
    runtime_max_agents: Option<usize>,
    runtime_agent_timeout_seconds: Option<u64>,
    runtime_task_queue_size: Option<usize>,
    runtime_enable_auto_recovery: Option<bool>,
    cortex_enabled: Option<bool>,
    cortex_mcp_server_url: Option<String>,
    cortex_workspace: Option<String>,
}

async fn update_config(
    State(_state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<StatusCode, ApiError> {
    let mut config = crate::commands::config::AxonConfig::load()
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Update fields if provided
    if let Some(workspace_name) = req.workspace_name {
        config.workspace_name = workspace_name;
    }
    if let Some(workspace_path) = req.workspace_path {
        config.workspace_path = std::path::PathBuf::from(workspace_path);
    }
    if let Some(host) = req.server_host {
        config.server.host = host;
    }
    if let Some(port) = req.server_port {
        config.server.port = port;
    }
    if let Some(max_agents) = req.runtime_max_agents {
        config.runtime.max_agents = max_agents;
    }
    if let Some(timeout) = req.runtime_agent_timeout_seconds {
        config.runtime.agent_timeout_seconds = timeout;
    }
    if let Some(queue_size) = req.runtime_task_queue_size {
        config.runtime.task_queue_size = queue_size;
    }
    if let Some(auto_recovery) = req.runtime_enable_auto_recovery {
        config.runtime.enable_auto_recovery = auto_recovery;
    }
    if let Some(enabled) = req.cortex_enabled {
        config.cortex.enabled = enabled;
    }
    if let Some(url) = req.cortex_mcp_server_url {
        config.cortex.mcp_server_url = Some(url);
    }
    if let Some(workspace) = req.cortex_workspace {
        config.cortex.workspace = Some(workspace);
    }

    // Save the updated configuration
    config.save()
        .map_err(|e| ApiError::Internal(format!("Failed to save configuration: {}", e)))?;

    Ok(StatusCode::OK)
}

/// Validate configuration
#[derive(Debug, Deserialize)]
struct ValidateConfigRequest {
    workspace_name: Option<String>,
    workspace_path: Option<String>,
    server_host: Option<String>,
    server_port: Option<u16>,
    runtime_max_agents: Option<usize>,
    runtime_agent_timeout_seconds: Option<u64>,
    runtime_task_queue_size: Option<usize>,
    runtime_enable_auto_recovery: Option<bool>,
    cortex_enabled: Option<bool>,
    cortex_mcp_server_url: Option<String>,
    cortex_workspace: Option<String>,
}

#[derive(Debug, Serialize)]
struct ValidateConfigResponse {
    valid: bool,
    errors: Vec<String>,
}

async fn validate_config(
    State(_state): State<AppState>,
    Json(req): Json<ValidateConfigRequest>,
) -> Result<Json<ValidateConfigResponse>, ApiError> {
    let mut errors = Vec::new();

    // Validate workspace_name
    if let Some(ref name) = req.workspace_name {
        if name.is_empty() {
            errors.push("workspace_name cannot be empty".to_string());
        }
        if name.contains('/') || name.contains('\\') {
            errors.push("workspace_name cannot contain path separators".to_string());
        }
    }

    // Validate workspace_path
    if let Some(ref path) = req.workspace_path {
        if path.is_empty() {
            errors.push("workspace_path cannot be empty".to_string());
        }
    }

    // Validate server_host
    if let Some(ref host) = req.server_host {
        if host.is_empty() {
            errors.push("server_host cannot be empty".to_string());
        }
        // Basic validation - could be IP or hostname
        if !host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':') {
            errors.push("server_host contains invalid characters".to_string());
        }
    }

    // Validate server_port
    if let Some(port) = req.server_port {
        if port == 0 {
            errors.push("server_port cannot be 0".to_string());
        }
        if port < 1024 {
            errors.push("server_port below 1024 may require elevated privileges".to_string());
        }
    }

    // Validate runtime_max_agents
    if let Some(max_agents) = req.runtime_max_agents {
        if max_agents == 0 {
            errors.push("runtime_max_agents must be at least 1".to_string());
        }
        if max_agents > 1000 {
            errors.push("runtime_max_agents seems unreasonably high (>1000)".to_string());
        }
    }

    // Validate runtime_agent_timeout_seconds
    if let Some(timeout) = req.runtime_agent_timeout_seconds {
        if timeout == 0 {
            errors.push("runtime_agent_timeout_seconds cannot be 0".to_string());
        }
        if timeout > 86400 {
            errors.push("runtime_agent_timeout_seconds exceeds 24 hours".to_string());
        }
    }

    // Validate runtime_task_queue_size
    if let Some(queue_size) = req.runtime_task_queue_size {
        if queue_size == 0 {
            errors.push("runtime_task_queue_size must be at least 1".to_string());
        }
        if queue_size > 100000 {
            errors.push("runtime_task_queue_size seems unreasonably high (>100000)".to_string());
        }
    }

    // Validate cortex_mcp_server_url if provided
    if let Some(ref url) = req.cortex_mcp_server_url {
        if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
            errors.push("cortex_mcp_server_url must be a valid HTTP(S) URL".to_string());
        }
    }

    Ok(Json(ValidateConfigResponse {
        valid: errors.is_empty(),
        errors,
    }))
}
