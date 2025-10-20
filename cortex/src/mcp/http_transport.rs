use super::handlers::ToolHandlers;
use super::project_handlers::ProjectToolHandlers;
use super::transport::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::config::HttpConfig;
use crate::project::ProjectManager;
use anyhow::{Context as AnyhowContext, Result};
use axum::{
    extract::{Path, State},
    http::{header, Method},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use dashmap::DashMap;
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};

/// Request with project context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpRequest {
    /// Project path for multi-project support
    pub project_path: Option<PathBuf>,

    /// JSON-RPC request
    #[serde(flatten)]
    pub request: JsonRpcRequest,
}

/// Transport mode
enum TransportMode {
    /// Single-project mode with dedicated handlers
    SingleProject {
        handlers: Arc<ToolHandlers>,
    },
    /// Multi-project mode with project manager
    MultiProject {
        project_handlers: Arc<ProjectToolHandlers>,
    },
}

/// HTTP Transport state shared across handlers
#[derive(Clone)]
pub struct HttpTransportState {
    /// Transport mode
    mode: Arc<TransportMode>,

    /// Broadcast channel for SSE notifications
    notification_tx: broadcast::Sender<SseNotification>,

    /// Project-specific event channels
    project_channels: Arc<DashMap<String, broadcast::Sender<SseNotification>>>,

    /// Active sessions per project
    sessions: Arc<DashMap<String, Vec<String>>>,

    /// Configuration
    config: HttpConfig,
}

/// Server-sent event notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseNotification {
    /// Event type
    pub event: String,

    /// Event data
    pub data: Value,

    /// Optional project path
    pub project_path: Option<String>,

    /// Timestamp
    pub timestamp: i64,
}

impl HttpTransportState {
    /// Create new transport state for single-project mode
    pub fn new(handlers: Arc<ToolHandlers>, config: HttpConfig) -> Self {
        let (notification_tx, _) = broadcast::channel(100);

        Self {
            mode: Arc::new(TransportMode::SingleProject { handlers }),
            notification_tx,
            project_channels: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Create new transport state for multi-project mode
    pub fn new_with_project_manager(
        project_manager: Arc<ProjectManager>,
        config: HttpConfig,
    ) -> Self {
        let (notification_tx, _) = broadcast::channel(100);
        let project_handlers = Arc::new(ProjectToolHandlers::new(project_manager));

        Self {
            mode: Arc::new(TransportMode::MultiProject { project_handlers }),
            notification_tx,
            project_channels: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Send notification to all connected clients
    pub fn send_notification(&self, notification: SseNotification) {
        // Send to global channel
        let _ = self.notification_tx.send(notification.clone());

        // Send to project-specific channel if applicable
        if let Some(ref project_path) = notification.project_path {
            if let Some(tx) = self.project_channels.get(project_path) {
                let _ = tx.send(notification);
            }
        }
    }

    /// Get or create project-specific channel
    fn get_project_channel(&self, project_path: &str) -> broadcast::Sender<SseNotification> {
        self.project_channels
            .entry(project_path.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(50);
                tx
            })
            .clone()
    }

    /// Handle tool call with project context
    async fn handle_tool_call_with_project(
        &self,
        tool_name: &str,
        arguments: Value,
        project_path: Option<&std::path::Path>,
    ) -> Result<Value> {
        match self.mode.as_ref() {
            TransportMode::SingleProject { handlers } => {
                handlers.handle_tool_call(tool_name, arguments).await
            }
            TransportMode::MultiProject { project_handlers } => {
                project_handlers.handle_tool_call_for_project(
                    tool_name,
                    arguments,
                    project_path
                ).await
            }
        }
    }
}

/// HTTP Transport for MCP server
pub struct HttpTransport {
    state: HttpTransportState,
    config: HttpConfig,
}

impl HttpTransport {
    /// Create a new HTTP transport for single-project mode
    pub fn new(handlers: Arc<ToolHandlers>, config: HttpConfig) -> Self {
        let state = HttpTransportState::new(handlers, config.clone());

        Self { state, config }
    }

    /// Create a new HTTP transport for multi-project mode
    pub fn new_with_project_manager(
        project_manager: Arc<ProjectManager>,
        config: HttpConfig,
    ) -> Self {
        let state = HttpTransportState::new_with_project_manager(project_manager, config.clone());

        Self { state, config }
    }

    /// Start the HTTP server
    pub async fn serve(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting HTTP/SSE transport on {}", addr);

        let app = self.create_router();

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?;

        info!("Meridian MCP HTTP server listening on {}", addr);

        axum::serve(listener, app)
            .await
            .context("Server error")?;

        Ok(())
    }

    /// Create the router with all endpoints
    fn create_router(self) -> Router {
        // Configure CORS
        use tower_http::cors::AllowOrigin;

        let cors_origins = if self.config.cors_origins.contains(&"*".to_string()) {
            AllowOrigin::any()
        } else {
            AllowOrigin::list(
                self.config
                    .cors_origins
                    .iter()
                    .map(|s| s.parse().unwrap())
                    .collect::<Vec<_>>()
            )
        };

        let cors = CorsLayer::new()
            .allow_origin(cors_origins)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
            .max_age(Duration::from_secs(3600));

        // Create router without state first
        let app = Router::new()
            // Health check endpoint
            .route("/health", get(health_check))

            // MCP JSON-RPC endpoint
            .route("/mcp/request", post(handle_mcp_request))

            // SSE endpoint for notifications (global)
            .route("/mcp/events", get(handle_sse_events))

            // SSE endpoint for project-specific notifications
            .route("/mcp/events/{project_id}", get(handle_project_sse_events))

            // Server info endpoint
            .route("/mcp/info", get(server_info));

        // Add state and CORS layer
        app.with_state(self.state).layer(cors)
    }
}

/// Health check handler
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "meridian-mcp",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Server info handler
async fn server_info(State(state): State<HttpTransportState>) -> Json<Value> {
    let project_count = state.project_channels.len();
    let active_sessions: usize = state.sessions.iter().map(|e| e.value().len()).sum();

    Json(json!({
        "server": "meridian-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": "2024-11-05",
        "transport": "http/sse",
        "projects": project_count,
        "active_sessions": active_sessions,
        "max_connections": state.config.max_connections
    }))
}

/// Handle MCP JSON-RPC request
fn handle_mcp_request(
    State(state): State<HttpTransportState>,
    Json(req): Json<McpHttpRequest>,
) -> Pin<Box<dyn Future<Output = Json<JsonRpcResponse>> + Send>> {
    Box::pin(async move {
    debug!(
        "Received MCP request: method={}, project={:?}",
        req.request.method, req.project_path
    );

    // Store project path in request context if provided
    let project_id = req
        .project_path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|s| s.to_string());

    let request_id = req.request.id.clone();

    // Handle the request based on method
    let response = match req.request.method.as_str() {
        "initialize" => handle_initialize(request_id, req.request.params),
        "tools/list" => handle_list_tools(request_id),
        "tools/call" => {
            handle_call_tool_with_state(request_id, req.request.params, req.project_path.as_deref(), &state).await
        }
        "resources/list" => handle_list_resources(request_id),
        "resources/read" => handle_read_resource(request_id, req.request.params).await,
        "ping" => JsonRpcResponse::success(request_id, json!({"status": "ok"})),
        _ => JsonRpcResponse::error(
            request_id,
            JsonRpcError::method_not_found(format!("Method not found: {}", req.request.method)),
        ),
    };

    // Send notification about request completion if we have a project
    if let Some(project_id) = project_id {
        let notification = SseNotification {
            event: "request_completed".to_string(),
            data: json!({
                "method": req.request.method,
                "success": response.error.is_none()
            }),
            project_path: Some(project_id),
            timestamp: chrono::Utc::now().timestamp(),
        };

        state.send_notification(notification);
    }

    Json(response)
    })
}

/// Handle SSE events (global stream)
async fn handle_sse_events(
    State(state): State<HttpTransportState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE connection established (global)");

    let rx = state.notification_tx.subscribe();

    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(notification) => {
                    let event = Event::default()
                        .event(notification.event)
                        .json_data(notification.data)
                        .ok()?;
                    Some(Ok(event))
                }
                Err(e) => {
                    warn!("SSE broadcast error: {}", e);
                    None
                }
            }
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Handle project-specific SSE events
async fn handle_project_sse_events(
    State(state): State<HttpTransportState>,
    Path(project_id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE connection established for project: {}", project_id);

    let tx = state.get_project_channel(&project_id);
    let rx = tx.subscribe();

    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(notification) => {
                    let event = Event::default()
                        .event(notification.event)
                        .json_data(notification.data)
                        .ok()?;
                    Some(Ok(event))
                }
                Err(e) => {
                    warn!("SSE broadcast error: {}", e);
                    None
                }
            }
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// === Request Handlers (mirrored from server.rs) ===

fn handle_initialize(id: Option<Value>, _params: Option<Value>) -> JsonRpcResponse {
    info!("Handling initialize request (HTTP)");

    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "meridian",
            "version": env!("CARGO_PKG_VERSION"),
            "transport": "http/sse"
        }
    });

    JsonRpcResponse::success(id, result)
}

fn handle_list_tools(id: Option<Value>) -> JsonRpcResponse {
    info!("Handling tools/list request (HTTP)");

    use super::tools::get_all_tools;
    let tools = get_all_tools();
    let result = json!({ "tools": tools });

    JsonRpcResponse::success(id, result)
}

async fn handle_call_tool_with_state(
    id: Option<Value>,
    params: Option<Value>,
    project_path: Option<&std::path::Path>,
    state: &HttpTransportState,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing parameters".to_string()),
            )
        }
    };

    // Extract tool name and arguments
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing tool name".to_string()),
            )
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    info!("Calling tool: {} (HTTP) for project: {:?}", tool_name, project_path);

    // Call the tool handler with project context
    match state.handle_tool_call_with_project(tool_name, arguments, project_path).await {
        Ok(result) => {
            let response = json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result)
                            .unwrap_or_else(|_| result.to_string())
                    }
                ]
            });
            JsonRpcResponse::success(id, response)
        }
        Err(e) => {
            error!("Tool call failed: {}", e);
            JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Tool execution failed: {}", e)),
            )
        }
    }
}

fn handle_list_resources(id: Option<Value>) -> JsonRpcResponse {
    info!("Handling resources/list request (HTTP)");

    use super::tools::get_all_resources;
    let resources = get_all_resources();
    let result = json!({ "resources": resources });

    JsonRpcResponse::success(id, result)
}

async fn handle_read_resource(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing parameters".to_string()),
            )
        }
    };

    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(uri) => uri,
        None => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params("Missing resource URI".to_string()),
            )
        }
    };

    info!("Reading resource: {} (HTTP)", uri);

    // Handle different resource URIs
    let content = match uri {
        "meridian://index/current" => {
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": json!({
                    "status": "active",
                    "total_symbols": 0,
                    "total_files": 0
                }).to_string()
            })
        }
        "meridian://memory/episodes" => {
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": json!({
                    "episodes": []
                }).to_string()
            })
        }
        "meridian://memory/working" => {
            json!({
                "uri": uri,
                "mimeType": "application/json",
                "text": json!({
                    "active_symbols": [],
                    "token_count": 0
                }).to_string()
            })
        }
        _ => {
            return JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params(format!("Unknown resource: {}", uri)),
            )
        }
    };

    JsonRpcResponse::success(id, json!({ "contents": [content] }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_notification_serialization() {
        let notification = SseNotification {
            event: "test_event".to_string(),
            data: json!({"key": "value"}),
            project_path: Some("/path/to/project".to_string()),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&notification).unwrap();
        let parsed: SseNotification = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event, "test_event");
        assert_eq!(parsed.timestamp, 1234567890);
    }

    #[test]
    fn test_mcp_http_request_serialization() {
        let request = McpHttpRequest {
            project_path: Some(PathBuf::from("/path/to/project")),
            request: JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "tools/list".to_string(),
                params: None,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: McpHttpRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.request.method, "tools/list");
        assert!(parsed.project_path.is_some());
    }
}
