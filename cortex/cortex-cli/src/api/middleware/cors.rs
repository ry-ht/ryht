//! CORS middleware

use tower_http::cors::{Any, CorsLayer};

/// Create CORS layer with permissive settings
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}
