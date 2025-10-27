//! CORS middleware

use axum::{http::StatusCode, response::IntoResponse};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

/// Create CORS layer with permissive settings
/// Includes max_age to cache preflight responses for 1 hour
/// Note: We use allow_origin(Any) without allow_credentials for maximum compatibility
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(Duration::from_secs(3600)) // Cache preflight for 1 hour
}

/// Handler for OPTIONS requests (CORS preflight)
/// Returns 204 No Content, CORS headers will be added by CorsLayer middleware
pub async fn options_handler() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
