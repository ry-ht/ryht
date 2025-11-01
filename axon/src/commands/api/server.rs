//! REST API Server implementation

use anyhow::Result;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    Router,
};
use cortex_core::config::GlobalConfig;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{debug, info, warn, Level};

use super::{auth_proxy, middleware as api_middleware, routes, websocket};
use crate::commands::{config::AxonConfig, runtime_manager::AgentRuntimeManager};

/// SPA fallback handler - serves index.html for all non-API routes
async fn spa_fallback_handler(
    dashboard_path: PathBuf,
    req: Request<Body>,
) -> Response {
    let path = req.uri().path();
    debug!("SPA fallback handler called for path: {}", path);

    // If path starts with /api/, return 404
    if path.starts_with("/api/") {
        debug!("Path starts with /api/, returning 404");
        return (StatusCode::NOT_FOUND, "Not found").into_response();
    }

    // Build the file path from the request path
    let file_path = if path == "/" {
        dashboard_path.join("index.html")
    } else {
        // Remove leading slash and join with dashboard path
        let clean_path = path.trim_start_matches('/');
        dashboard_path.join(clean_path)
    };

    debug!("Trying to serve file: {:?}", file_path);

    // Check if the file exists
    if file_path.exists() && file_path.is_file() {
        debug!("File exists, serving: {:?}", file_path);
        // File exists, try to serve it
        match tokio::fs::read(&file_path).await {
            Ok(content) => {
                // Determine content type from file extension
                let content_type = match file_path.extension().and_then(|s| s.to_str()) {
                    Some("html") => "text/html; charset=utf-8",
                    Some("css") => "text/css; charset=utf-8",
                    Some("js") => "application/javascript; charset=utf-8",
                    Some("json") => "application/json",
                    Some("png") => "image/png",
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    Some("svg") => "image/svg+xml",
                    Some("woff") => "font/woff",
                    Some("woff2") => "font/woff2",
                    Some("webp") => "image/webp",
                    _ => "application/octet-stream",
                };

                return (
                    StatusCode::OK,
                    [("content-type", content_type)],
                    content,
                )
                    .into_response();
            }
            Err(_) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, "Error reading file").into_response();
            }
        }
    }

    // File not found, serve index.html for SPA routing
    debug!("File not found, falling back to index.html");
    let index_path = dashboard_path.join("index.html");
    debug!("Index path: {:?}", index_path);
    match tokio::fs::read_to_string(&index_path).await {
        Ok(content) => (
            StatusCode::OK,
            [("content-type", "text/html; charset=utf-8")],
            content,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Dashboard not found").into_response(),
    }
}

/// Run the REST API server (blocking)
pub async fn start_server(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    info!("Starting Axon REST API Server");
    info!("Host: {}", host);
    info!("Port: {}", port);
    if let Some(w) = workers {
        info!("Workers: {}", w);
    }

    // Load configurations
    let config = AxonConfig::load()?;
    let global_config = GlobalConfig::load_or_create_default().await?;

    // Create runtime manager
    let runtime = Arc::new(RwLock::new(AgentRuntimeManager::new(config.clone())?));

    // Create WebSocket manager
    let ws_manager = websocket::WsManager::new();

    // Create middleware instances
    let api_key_validator = api_middleware::ApiKeyValidator::new();
    let rate_limiter = api_middleware::RateLimiter::new();

    // Spawn background task for rate limiter cleanup
    let rate_limiter_clone = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            rate_limiter_clone.cleanup_expired().await;
        }
    });

    // Create application state
    let app_state = routes::AppState {
        runtime: runtime.clone(),
        ws_manager: ws_manager.clone(),
    };

    // Build CORS layer with specific configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(Duration::from_secs(3600));

    // Create API routes with middleware
    let api_routes = routes::create_routes(app_state)
        .layer(middleware::from_fn({
            let validator = api_key_validator.clone();
            move |req, next| {
                let validator = validator.clone();
                api_middleware::optional_auth(validator, req, next)
            }
        }))
        .layer(middleware::from_fn(api_middleware::logging))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
        )
        .layer(cors.clone());

    // Create WebSocket routes
    let ws_routes = websocket::websocket_routes(ws_manager.clone());

    // Create auth proxy routes (forward to Cortex)
    let cortex_api_url = config.cortex.api_url
        .clone()
        .unwrap_or_else(|| {
            // Use GlobalConfig if not specified in workspace config
            format!(
                "http://{}:{}",
                global_config.cortex().server.host,
                global_config.cortex().server.port
            )
        });

    let auth_proxy_state = auth_proxy::AuthProxyState::new(cortex_api_url.clone());
    let auth_routes = Router::new()
        .fallback(auth_proxy::proxy_auth)
        .with_state(auth_proxy_state)
        .layer(cors.clone());

    info!("Auth proxy configured: /api/v1/auth/* -> {}", cortex_api_url);

    // Check if dashboard directory exists
    let dashboard_path = std::path::PathBuf::from("./dashboard");
    let has_dashboard = dashboard_path.exists() && dashboard_path.is_dir();

    info!("Checking for dashboard at: {:?}", dashboard_path.canonicalize().unwrap_or(dashboard_path.clone()));
    info!("Dashboard exists: {}, is_dir: {}", dashboard_path.exists(), dashboard_path.is_dir());

    if !has_dashboard {
        warn!("Dashboard directory not found at ./dashboard");
        warn!("Run build script to copy dashboard files: ./build-and-copy.sh release");
    } else {
        info!("Dashboard directory found, will serve static files");
    }

    // Combine all routes
    let mut app = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/api/v1", ws_routes)
        .nest("/api/v1/auth", auth_routes);

    // Serve dashboard static files if directory exists with SPA fallback
    if has_dashboard {
        info!("Serving dashboard from ./dashboard (with SPA routing)");
        let dashboard_path_clone = dashboard_path.clone();
        app = app.fallback(move |req| {
            let path = dashboard_path_clone.clone();
            spa_fallback_handler(path, req)
        });
    } else {
        app = app.fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "Not found") });
    }

    // Parse socket address
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    info!("Axon API Server starting...");
    info!("  Listening: http://{}", addr);
    info!("  API:       http://{}/api/v1/", addr);
    info!("  WebSocket: ws://{}/api/v1/ws", addr);
    if has_dashboard {
        info!("  Dashboard: http://{}/", addr);
    } else {
        info!("  Dashboard: Not available (run ./build-and-copy.sh to deploy)");
    }

    // Print API key information
    if let Ok(_api_key) = std::env::var("AXON_API_KEY") {
        info!("Using custom API key from AXON_API_KEY environment variable");
    } else {
        info!("Using default API key: axon-dev-key-change-in-production");
        info!("Set AXON_API_KEY environment variable to use a custom API key");
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        // Test that server can be created
        // Actual server startup is tested in integration tests
        let config = AxonConfig::default();
        let runtime = AgentRuntimeManager::new(config);
        assert!(runtime.is_ok());
    }
}
