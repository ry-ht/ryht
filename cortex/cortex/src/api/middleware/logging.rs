//! Request logging middleware

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

/// Request logging middleware
pub struct RequestLogger;

impl RequestLogger {
    /// Log incoming requests and responses
    pub async fn log(req: Request<Body>, next: Next) -> Response {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Log headers for debugging
        let content_length = req.headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let start = Instant::now();

        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            content_length = %content_length,
            "Incoming request"
        );

        let response = next.run(req).await;

        let duration = start.elapsed();
        let status = response.status();

        if status.is_success() {
            info!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request completed"
            );
        } else {
            warn!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status,
                duration_ms = %duration.as_millis(),
                "Request failed"
            );
        }

        response
    }
}
