//! Authentication proxy - forwards auth requests to Cortex

use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Auth proxy state
#[derive(Clone)]
pub struct AuthProxyState {
    /// Cortex URL
    pub cortex_url: String,
    /// HTTP client
    pub client: Arc<reqwest::Client>,
}

impl AuthProxyState {
    /// Create new auth proxy state
    pub fn new(cortex_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            cortex_url: cortex_url.into(),
            client: Arc::new(client),
        }
    }
}

/// Proxy authentication requests to Cortex
pub async fn proxy_auth(
    State(proxy_state): State<AuthProxyState>,
    req: Request<Body>,
) -> Response {
    // Extract request parts
    let method = req.method().clone();
    let uri = req.uri();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    // Build target URL
    let target_url = if query.is_empty() {
        format!("{}{}", proxy_state.cortex_url, path)
    } else {
        format!("{}{}?{}", proxy_state.cortex_url, path, query)
    };

    debug!("Proxying auth request: {} {} -> {}", method, path, target_url);

    // Extract headers
    let headers = req.headers().clone();

    // Extract body
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                "Failed to read request body"
            ).into_response();
        }
    };

    // Build proxied request
    let mut proxy_req = match method {
        Method::GET => proxy_state.client.get(&target_url),
        Method::POST => proxy_state.client.post(&target_url),
        Method::PUT => proxy_state.client.put(&target_url),
        Method::DELETE => proxy_state.client.delete(&target_url),
        Method::PATCH => proxy_state.client.patch(&target_url),
        _ => {
            warn!("Unsupported HTTP method for auth proxy: {}", method);
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed"
            ).into_response();
        }
    };

    // Copy relevant headers (skip host, connection, etc.)
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if !matches!(
            name_str.to_lowercase().as_str(),
            "host" | "connection" | "transfer-encoding" | "content-length"
        ) {
            if let Ok(value_str) = value.to_str() {
                proxy_req = proxy_req.header(name_str, value_str);
            }
        }
    }

    // Add body if present
    if !body_bytes.is_empty() {
        proxy_req = proxy_req.body(body_bytes.to_vec());
    }

    // Execute request
    let proxy_response = match proxy_req.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to proxy auth request to Cortex: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                format!("Failed to connect to Cortex auth service: {}", e)
            ).into_response();
        }
    };

    // Build response
    let status = proxy_response.status();
    let response_headers = proxy_response.headers().clone();

    let body_bytes = match proxy_response.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read Cortex response body: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                "Failed to read Cortex response"
            ).into_response();
        }
    };

    // Create response with headers
    let mut response = Response::builder().status(status);

    // Copy response headers (skip some problematic ones)
    for (name, value) in response_headers.iter() {
        let name_str = name.as_str();
        if !matches!(
            name_str.to_lowercase().as_str(),
            "connection" | "transfer-encoding" | "content-length"
        ) {
            response = response.header(name_str, value);
        }
    }

    // Set content-type if not present
    if !response_headers.contains_key("content-type") {
        response = response.header("content-type", "application/json");
    }

    match response.body(Body::from(body_bytes)) {
        Ok(resp) => {
            debug!("Successfully proxied auth request: status={}", status);
            resp
        }
        Err(e) => {
            error!("Failed to build response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response"
            ).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_proxy_state_creation() {
        let state = AuthProxyState::new("http://localhost:8080");
        assert_eq!(state.cortex_url, "http://localhost:8080");
    }
}
