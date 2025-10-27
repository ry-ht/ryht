//! CORS middleware

use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

/// Create CORS layer with permissive settings
/// Includes max_age to cache preflight responses for 1 hour
/// expose_headers is added to allow browsers to read all response headers
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any)
        .max_age(Duration::from_secs(3600)) // Cache preflight for 1 hour
}
