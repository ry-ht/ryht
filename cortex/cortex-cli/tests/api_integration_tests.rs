//! Comprehensive Integration Tests for Cortex REST API Server
//!
//! These tests verify the REST API endpoints work correctly end-to-end.
//! They test:
//! - Server startup and shutdown
//! - Authentication flow (login, refresh, logout)
//! - Workspace CRUD operations
//! - VFS file operations (create, read, update, delete, list, tree)
//! - Session management including session-scoped VFS operations (critical!)
//! - Search operations (semantic, pattern, references)
//! - Memory operations (episodes, consolidation)
//! - Health and metrics endpoints
//! - Both success and error scenarios

use axum::http::{header, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

// ============================================================================
// Test Utilities and Setup
// ============================================================================

/// Test client for making API requests
struct ApiTestClient {
    base_url: String,
    client: reqwest::Client,
    access_token: Option<String>,
}

impl ApiTestClient {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            access_token: None,
        }
    }

    async fn get(&self, path: &str) -> reqwest::Response {
        let mut request = self.client.get(format!("{}{}", self.base_url, path));

        if let Some(token) = &self.access_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        request.send().await.expect("Failed to send request")
    }

    async fn post(&self, path: &str, body: Value) -> reqwest::Response {
        let mut request = self.client
            .post(format!("{}{}", self.base_url, path))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body);

        if let Some(token) = &self.access_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        request.send().await.expect("Failed to send request")
    }

    async fn put(&self, path: &str, body: Value) -> reqwest::Response {
        let mut request = self.client
            .put(format!("{}{}", self.base_url, path))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body);

        if let Some(token) = &self.access_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        request.send().await.expect("Failed to send request")
    }

    async fn delete(&self, path: &str) -> reqwest::Response {
        let mut request = self.client.delete(format!("{}{}", self.base_url, path));

        if let Some(token) = &self.access_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        request.send().await.expect("Failed to send request")
    }

    fn set_token(&mut self, token: String) {
        self.access_token = Some(token);
    }
}

/// Helper to setup test database in temp directory
async fn setup_test_database() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    unsafe {
        std::env::set_var("CORTEX_DATA_DIR", temp_dir.path());
    }
    temp_dir
}

/// Helper to start test server on a random port
async fn start_test_server() -> (tokio::task::JoinHandle<()>, String, TempDir) {
    use cortex_cli::api::server::{RestApiServer, ServerConfig};

    let temp_dir = setup_test_database().await;

    // Use a random available port
    let port = portpicker::pick_unused_port().expect("No ports available");

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port,
        workers: None,
    };

    let base_url = format!("http://127.0.0.1:{}", port);

    let server = RestApiServer::with_config(config)
        .await
        .expect("Failed to create server");

    let handle = tokio::spawn(async move {
        server.serve().await.expect("Server failed");
    });

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;

    (handle, base_url, temp_dir)
}

// ============================================================================
// Server Lifecycle Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_server_startup_and_shutdown() {
    let (handle, base_url, _temp_dir) = start_test_server().await;

    // Verify server is running
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/v1/health", base_url))
        .send()
        .await;

    assert!(response.is_ok());
    assert_eq!(response.unwrap().status(), StatusCode::OK);

    // Shutdown server
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_health_endpoint() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let client = ApiTestClient::new(base_url);

    let response = client.get("/api/v1/health").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["status"].is_string());
    assert!(body["data"]["version"].is_string());
    assert!(body["data"]["uptime_seconds"].is_number());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_metrics_endpoint() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let client = ApiTestClient::new(base_url);

    let response = client.get("/api/v1/metrics").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["workspaces"].is_number());
    assert!(body["data"]["files"].is_number());
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_login_success() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let client = ApiTestClient::new(base_url);

    let response = client.post("/api/v1/auth/login", json!({
        "email": "admin@cortex.local",
        "password": "admin123"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());
    assert_eq!(body["data"]["token_type"], "Bearer");
    assert!(body["data"]["expires_in"].is_number());
    assert!(body["data"]["user"]["id"].is_string());
    assert!(body["data"]["user"]["email"].is_string());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_login_invalid_credentials() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let client = ApiTestClient::new(base_url);

    let response = client.post("/api/v1/auth/login", json!({
        "email": "wrong@example.com",
        "password": "wrongpassword"
    })).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], false);
    assert!(body["error"].is_string());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_refresh_token() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    // First, login to get tokens
    let login_response = client.post("/api/v1/auth/login", json!({
        "email": "admin@cortex.local",
        "password": "admin123"
    })).await;

    let login_body: Value = login_response.json().await.expect("Failed to parse JSON");
    let refresh_token = login_body["data"]["refresh_token"].as_str().unwrap();

    // Now use refresh token
    let response = client.post("/api/v1/auth/refresh", json!({
        "refresh_token": refresh_token
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["access_token"].is_string());
    assert_eq!(body["data"]["token_type"], "Bearer");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_logout() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    // Login first
    let login_response = client.post("/api/v1/auth/login", json!({
        "email": "admin@cortex.local",
        "password": "admin123"
    })).await;

    let login_body: Value = login_response.json().await.expect("Failed to parse JSON");
    let access_token = login_body["data"]["access_token"].as_str().unwrap().to_string();
    client.set_token(access_token);

    // Logout
    let response = client.post("/api/v1/auth/logout", json!({})).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_get_current_user() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    // Login first
    let login_response = client.post("/api/v1/auth/login", json!({
        "email": "admin@cortex.local",
        "password": "admin123"
    })).await;

    let login_body: Value = login_response.json().await.expect("Failed to parse JSON");
    let access_token = login_body["data"]["access_token"].as_str().unwrap().to_string();
    client.set_token(access_token);

    // Get current user
    let response = client.get("/api/v1/auth/me").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["id"].is_string());
    assert!(body["data"]["email"].is_string());
    assert!(body["data"]["roles"].is_array());
}

// ============================================================================
// Workspace CRUD Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_create_workspace() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    // Login first
    authenticate_client(&mut client).await;

    let response = client.post("/api/v1/workspaces", json!({
        "name": "Test Workspace",
        "workspace_type": "code",
        "source_path": "/tmp/test"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["id"].is_string());
    assert_eq!(body["data"]["name"], "Test Workspace");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_workspaces() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/workspaces").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_get_workspace() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // Create workspace first
    let create_response = client.post("/api/v1/workspaces", json!({
        "name": "Get Test Workspace",
        "workspace_type": "code"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let workspace_id = create_body["data"]["id"].as_str().unwrap();

    // Get workspace
    let response = client.get(&format!("/api/v1/workspaces/{}", workspace_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["id"], workspace_id);
    assert_eq!(body["data"]["name"], "Get Test Workspace");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_update_workspace() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // Create workspace first
    let create_response = client.post("/api/v1/workspaces", json!({
        "name": "Update Test",
        "workspace_type": "code"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let workspace_id = create_body["data"]["id"].as_str().unwrap();

    // Update workspace
    let response = client.put(&format!("/api/v1/workspaces/{}", workspace_id), json!({
        "name": "Updated Workspace Name"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_delete_workspace() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // Create workspace first
    let create_response = client.post("/api/v1/workspaces", json!({
        "name": "Delete Test",
        "workspace_type": "code"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let workspace_id = create_body["data"]["id"].as_str().unwrap();

    // Delete workspace
    let response = client.delete(&format!("/api/v1/workspaces/{}", workspace_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_sync_workspace() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // Create workspace first
    let create_response = client.post("/api/v1/workspaces", json!({
        "name": "Sync Test",
        "workspace_type": "code",
        "source_path": "/tmp/sync_test"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let workspace_id = create_body["data"]["id"].as_str().unwrap();

    // Sync workspace
    let response = client.post(&format!("/api/v1/workspaces/{}/sync", workspace_id), json!({
        "force": false,
        "dry_run": true
    })).await;

    assert!(response.status().is_success() || response.status().is_client_error());
}

// ============================================================================
// VFS File Operations Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_workspace_files() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    let response = client.get(&format!("/api/v1/workspaces/{}/files", workspace_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_create_file() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    let response = client.post(&format!("/api/v1/workspaces/{}/files", workspace_id), json!({
        "path": "/test.txt",
        "content": "Hello, World!",
        "language": "text"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["id"].is_string());
    assert_eq!(body["data"]["path"], "/test.txt");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_read_file() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create file first
    let create_response = client.post(&format!("/api/v1/workspaces/{}/files", workspace_id), json!({
        "path": "/read_test.txt",
        "content": "Test Content"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let file_id = create_body["data"]["id"].as_str().unwrap();

    // Read file
    let response = client.get(&format!("/api/v1/files/{}", file_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["content"], "Test Content");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_update_file() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create file first
    let create_response = client.post(&format!("/api/v1/workspaces/{}/files", workspace_id), json!({
        "path": "/update_test.txt",
        "content": "Original Content"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let file_id = create_body["data"]["id"].as_str().unwrap();

    // Update file
    let response = client.put(&format!("/api/v1/files/{}", file_id), json!({
        "content": "Updated Content"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_delete_file() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create file first
    let create_response = client.post(&format!("/api/v1/workspaces/{}/files", workspace_id), json!({
        "path": "/delete_test.txt",
        "content": "To be deleted"
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let file_id = create_body["data"]["id"].as_str().unwrap();

    // Delete file
    let response = client.delete(&format!("/api/v1/files/{}", file_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_get_directory_tree() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    let response = client.get(&format!("/api/v1/workspaces/{}/tree", workspace_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["name"].is_string());
    assert!(body["data"]["children"].is_array());
}

// ============================================================================
// Session Management Tests (Critical!)
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_create_session() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    let response = client.post("/api/v1/sessions", json!({
        "name": "Test Session",
        "agent_type": "code_editor",
        "workspace_id": workspace_id
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["id"].is_string());
    assert_eq!(body["data"]["name"], "Test Session");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_sessions() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/sessions").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_get_session() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create session first
    let create_response = client.post("/api/v1/sessions", json!({
        "name": "Get Test Session",
        "agent_type": "researcher",
        "workspace_id": workspace_id
    })).await;

    let create_body: Value = create_response.json().await.expect("Failed to parse JSON");
    let session_id = create_body["data"]["id"].as_str().unwrap();

    // Get session
    let response = client.get(&format!("/api/v1/sessions/{}", session_id)).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert_eq!(body["data"]["id"], session_id);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_session_scoped_file_operations() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create session
    let session_response = client.post("/api/v1/sessions", json!({
        "name": "File Ops Session",
        "agent_type": "code_editor",
        "workspace_id": workspace_id
    })).await;

    let session_body: Value = session_response.json().await.expect("Failed to parse JSON");
    let session_id = session_body["data"]["id"].as_str().unwrap();

    // List files in session scope
    let list_response = client.get(&format!("/api/v1/sessions/{}/files", session_id)).await;
    assert_eq!(list_response.status(), StatusCode::OK);

    // Write file in session scope
    let write_response = client.put(
        &format!("/api/v1/sessions/{}/files/session_test.txt", session_id),
        json!({
            "content": "Session scoped content"
        })
    ).await;
    assert_eq!(write_response.status(), StatusCode::OK);

    // Read file from session scope
    let read_response = client.get(&format!("/api/v1/sessions/{}/files/session_test.txt", session_id)).await;
    assert_eq!(read_response.status(), StatusCode::OK);

    let read_body: Value = read_response.json().await.expect("Failed to parse JSON");
    assert_eq!(read_body["data"]["content"], "Session scoped content");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_merge_session() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    // Create session
    let session_response = client.post("/api/v1/sessions", json!({
        "name": "Merge Test Session",
        "agent_type": "code_editor",
        "workspace_id": workspace_id
    })).await;

    let session_body: Value = session_response.json().await.expect("Failed to parse JSON");
    let session_id = session_body["data"]["id"].as_str().unwrap();

    // Merge session
    let response = client.post(&format!("/api/v1/sessions/{}/merge", session_id), json!({
        "strategy": "auto"
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"]["merge_id"].is_string());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_locks() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/locks").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

// ============================================================================
// Search Operations Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_semantic_search() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/search?query=test&limit=10").await;

    assert!(response.status().is_success());

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_pattern_search() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;
    let workspace_id = create_test_workspace(&mut client).await;

    let response = client.post("/api/v1/search/pattern", json!({
        "workspace_id": workspace_id,
        "pattern": "fn.*test",
        "language": "rust"
    })).await;

    assert!(response.status().is_success());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_find_references() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // This would require a valid unit ID from the system
    let unit_id = "test-unit-id";
    let response = client.get(&format!("/api/v1/search/references/{}", unit_id)).await;

    // May return 404 if unit doesn't exist, which is expected in tests
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

// ============================================================================
// Memory Operations Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_episodes() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/memory/episodes").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_get_episode() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // This would require a valid episode ID
    let episode_id = "test-episode-id";
    let response = client.get(&format!("/api/v1/memory/episodes/{}", episode_id)).await;

    // May return 404 if episode doesn't exist
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_search_episodes() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.post("/api/v1/memory/search", json!({
        "query": "test query",
        "limit": 10
    })).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_consolidate_memory() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.post("/api/v1/memory/consolidate", json!({})).await;

    assert!(response.status().is_success());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_list_learned_patterns() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/memory/patterns").await;

    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], true);
    assert!(body["data"].is_array());
}

// ============================================================================
// Error Scenario Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires database setup
async fn test_unauthorized_access() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let client = ApiTestClient::new(base_url);

    // Try to access protected endpoint without authentication
    let response = client.get("/api/v1/workspaces").await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_not_found_error() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    let response = client.get("/api/v1/workspaces/nonexistent-id").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["success"], false);
    assert!(body["error"].is_string());
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_bad_request_error() {
    let (_handle, base_url, _temp_dir) = start_test_server().await;
    let mut client = ApiTestClient::new(base_url.clone());

    authenticate_client(&mut client).await;

    // Send invalid JSON
    let response = client.post("/api/v1/workspaces", json!({
        "invalid_field": "value"
    })).await;

    assert!(response.status().is_client_error());
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to authenticate a test client
async fn authenticate_client(client: &mut ApiTestClient) {
    let login_response = client.post("/api/v1/auth/login", json!({
        "email": "admin@cortex.local",
        "password": "admin123"
    })).await;

    let login_body: Value = login_response.json().await.expect("Failed to parse JSON");
    let access_token = login_body["data"]["access_token"].as_str().unwrap().to_string();
    client.set_token(access_token);
}

/// Helper to create a test workspace
async fn create_test_workspace(client: &mut ApiTestClient) -> String {
    let response = client.post("/api/v1/workspaces", json!({
        "name": "Test Workspace",
        "workspace_type": "code"
    })).await;

    let body: Value = response.json().await.expect("Failed to parse JSON");
    body["data"]["id"].as_str().unwrap().to_string()
}
