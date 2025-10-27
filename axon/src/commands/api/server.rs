//! REST API Server implementation

use anyhow::Result;
use axum::{middleware, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{info, warn, Level};

use super::{middleware as api_middleware, routes, websocket};
use crate::commands::{config::AxonConfig, runtime_manager::AgentRuntimeManager};

/// Run the REST API server (blocking)
pub async fn start_server(host: String, port: u16, workers: Option<usize>) -> Result<()> {
    info!("Starting Axon REST API Server");
    info!("Host: {}", host);
    info!("Port: {}", port);
    if let Some(w) = workers {
        info!("Workers: {}", w);
    }

    // Load configuration
    let config = AxonConfig::load()?;

    // Create runtime manager
    let runtime = Arc::new(RwLock::new(AgentRuntimeManager::new(config)?));

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

    // Check if dashboard directory exists
    let dashboard_path = std::path::PathBuf::from("./dashboard");
    let has_dashboard = dashboard_path.exists() && dashboard_path.is_dir();

    if !has_dashboard {
        warn!("Dashboard directory not found at ./dashboard");
        warn!("Run build script to copy dashboard files: ./build-and-copy.sh release");
    }

    // Combine all routes
    let mut app = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/api/v1", ws_routes);

    // Serve dashboard static files if directory exists
    if has_dashboard {
        info!("Serving dashboard from ./dashboard");
        app = app.fallback_service(ServeDir::new(&dashboard_path));
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
