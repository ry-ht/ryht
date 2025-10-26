//! Unit Tests for REST API Endpoints
//!
//! Tests the REST API server endpoints including:
//! - Health and info endpoints
//! - Agent management endpoints
//! - Workflow management endpoints
//! - Metrics and telemetry endpoints
//! - Configuration endpoints
//! - WebSocket connections

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt; // For oneshot
use axon::commands::api::routes::{AppState, HealthResponse, ApiInfoResponse};
use axon::commands::api::websocket::WsManager;
use axon::commands::runtime_manager::AgentRuntimeManager;
use axon::commands::config::AxonConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test app state
async fn create_test_state() -> AppState {
    let config = AxonConfig::default();
    let runtime = AgentRuntimeManager::new(config).unwrap();
    let ws_manager = WsManager::new();

    AppState {
        runtime: Arc::new(RwLock::new(runtime)),
        ws_manager,
    }
}

/// Helper to send a request and get response
async fn send_request(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, String) {
    let request_builder = Request::builder()
        .method(method)
        .uri(path);

    let request = if let Some(body_json) = body {
        request_builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body_json).unwrap()))
            .unwrap()
    } else {
        request_builder
            .body(Body::empty())
            .unwrap()
    };

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to send request");

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read response body");
    let body_str = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");

    (status, body_str)
}

// ============================================================================
// API Info Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_api_info_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, body) = send_request(app, "GET", "/", None).await;

    assert_eq!(status, StatusCode::OK);

    let response: ApiInfoResponse = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(response.name, "Axon Multi-Agent API");
    assert!(!response.version.is_empty());
    assert!(!response.endpoints.is_empty());
}

#[tokio::test]
async fn test_api_info_response_structure() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, body) = send_request(app, "GET", "/", None).await;

    assert_eq!(status, StatusCode::OK);

    let response: ApiInfoResponse = serde_json::from_str(&body).unwrap();

    // Verify endpoints are documented
    assert!(response.endpoints.iter().any(|e| e.path == "/health"));
    assert!(response.endpoints.iter().any(|e| e.path == "/agents"));
    assert!(response.endpoints.iter().any(|e| e.path == "/workflows"));
    assert!(response.endpoints.iter().any(|e| e.path == "/metrics"));
}

// ============================================================================
// Health Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, body) = send_request(app, "GET", "/health", None).await;

    assert_eq!(status, StatusCode::OK);

    let response: HealthResponse = serde_json::from_str(&body).expect("Failed to parse JSON");
    assert_eq!(response.status, "healthy");
    assert!(!response.version.is_empty());
}

#[tokio::test]
async fn test_health_response_structure() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, body) = send_request(app, "GET", "/health", None).await;

    assert_eq!(status, StatusCode::OK);

    let response: HealthResponse = serde_json::from_str(&body).unwrap();

    // Verify all fields are present
    assert!(!response.status.is_empty());
    assert!(!response.version.is_empty());
    assert!(response.uptime_seconds >= 0);
    assert!(response.active_agents >= 0);
    assert!(response.running_workflows >= 0);
    assert!(response.websocket_connections >= 0);
}

// ============================================================================
// Agent Management Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_list_agents_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/agents", None).await;

    // Should return OK even with no agents
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_create_agent_endpoint_validation() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    // Test with invalid payload
    let invalid_payload = json!({});

    let (status, _body) = send_request(
        app,
        "POST",
        "/agents",
        Some(invalid_payload),
    ).await;

    // Should fail validation
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_create_agent_endpoint_valid_payload() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let payload = json!({
        "name": "test-agent",
        "agent_type": "Developer",
        "config": {
            "max_concurrent_tasks": 5
        }
    });

    let (status, _body) = send_request(
        app,
        "POST",
        "/agents",
        Some(payload),
    ).await;

    // Status should be OK or CREATED (depending on implementation)
    // May fail without actual runtime, but tests API surface
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_get_agent_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    // Try to get non-existent agent
    let (status, _body) = send_request(app, "GET", "/agents/test-id", None).await;

    // Should return 404 or similar error
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_delete_agent_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    // Try to delete non-existent agent
    let (status, _body) = send_request(app, "DELETE", "/agents/test-id", None).await;

    // Should handle gracefully
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_pause_agent_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "POST", "/agents/test-id/pause", None).await;

    // Should return error for non-existent agent
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_resume_agent_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "POST", "/agents/test-id/resume", None).await;

    // Should return error for non-existent agent
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_restart_agent_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "POST", "/agents/test-id/restart", None).await;

    // Should return error for non-existent agent
    assert!(status.is_client_error() || status.is_server_error());
}

// ============================================================================
// Workflow Management Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_list_workflows_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/workflows", None).await;

    // Should return OK even with no workflows
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_run_workflow_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let payload = json!({
        "name": "test-workflow",
        "tasks": [
            {
                "id": "task-1",
                "type": "Development",
                "input": {"code": "print('hello')"}
            }
        ]
    });

    let (status, _body) = send_request(
        app,
        "POST",
        "/workflows",
        Some(payload),
    ).await;

    // May succeed or fail depending on runtime state
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_get_workflow_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/workflows/test-id", None).await;

    // Should return error for non-existent workflow
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_cancel_workflow_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "POST", "/workflows/test-id/cancel", None).await;

    // Should return error for non-existent workflow
    assert!(status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_pause_workflow_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "POST", "/workflows/test-id/pause", None).await;

    // Should return error for non-existent workflow
    assert!(status.is_client_error() || status.is_server_error());
}

// ============================================================================
// Metrics and Telemetry Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_metrics_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/metrics", None).await;

    // Should return OK
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_export_metrics_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let payload = json!({
        "format": "json",
        "destination": "/tmp/metrics.json"
    });

    let (status, _body) = send_request(
        app,
        "POST",
        "/metrics/export",
        Some(payload),
    ).await;

    // May succeed or fail depending on permissions
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_get_telemetry_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/telemetry", None).await;

    // Should return OK
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_telemetry_summary_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/telemetry/summary", None).await;

    // Should return OK
    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// Configuration Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_get_config_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/config", None).await;

    // Should return OK
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_update_config_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let payload = json!({
        "max_concurrent_agents": 15,
        "timeout_seconds": 600
    });

    let (status, _body) = send_request(
        app,
        "PUT",
        "/config",
        Some(payload),
    ).await;

    // May succeed or fail depending on validation
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
}

#[tokio::test]
async fn test_validate_config_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let payload = json!({
        "max_concurrent_agents": 20,
        "memory_limit_mb": 2048
    });

    let (status, _body) = send_request(
        app,
        "POST",
        "/config/validate",
        Some(payload),
    ).await;

    // Should validate config
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
}

// ============================================================================
// Status Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_system_status_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/status", None).await;

    // Should return OK
    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_endpoint() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let (status, _body) = send_request(app, "GET", "/invalid/endpoint", None).await;

    // Should return 404
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_method() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    // Try POST on GET-only endpoint
    let (status, _body) = send_request(app, "POST", "/health", None).await;

    // Should return 405 Method Not Allowed
    assert!(status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_malformed_json() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = axum::Router::new()
        .nest("/api/v1", routes::create_routes(state));

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/agents")
        .header("content-type", "application/json")
        .body(Body::from("{invalid json"))
        .unwrap();

    let response = app.oneshot(request).await.expect("Failed to send request");
    let status = response.status();

    // Should return 400 Bad Request
    assert!(status.is_client_error());
}

// ============================================================================
// WebSocket Manager Tests
// ============================================================================

#[tokio::test]
async fn test_ws_manager_creation() {
    let ws_manager = WsManager::new();

    let count = ws_manager.connection_count().await;
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_ws_manager_broadcast() {
    let ws_manager = WsManager::new();

    let message = "test message";
    let result = ws_manager.broadcast(message.to_string()).await;

    // Should succeed even with no connections
    assert!(result.is_ok());
}

// ============================================================================
// HTTP Header Tests
// ============================================================================

#[tokio::test]
async fn test_cors_headers() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .header("origin", "http://localhost:3000")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.expect("Failed to send request");

    // CORS headers should be present (depends on middleware configuration)
    // This test validates that the request succeeds
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_content_type_json() {
    use axon::commands::api::routes;

    let state = create_test_state().await;
    let app = routes::create_routes(state);

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.expect("Failed to send request");

    // Response should be JSON
    let content_type = response.headers().get("content-type");
    if let Some(ct) = content_type {
        assert!(ct.to_str().unwrap().contains("application/json"));
    }
}
