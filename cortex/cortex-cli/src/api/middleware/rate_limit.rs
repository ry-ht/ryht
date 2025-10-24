//! Rate limiting middleware

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

/// Rate limit configuration for different endpoint categories
#[derive(Debug, Clone, Copy)]
pub enum RateLimitTier {
    /// Authentication endpoints: 10 req/minute
    Auth,
    /// Read endpoints: 1000 req/minute
    Read,
    /// Write endpoints: 100 req/minute
    Write,
    /// Search endpoints: 100 req/minute
    Search,
    /// Analysis endpoints: 50 req/minute
    Analysis,
    /// Build/Test endpoints: 10 req/minute
    Build,
    /// Export/Import endpoints: 5 req/hour
    ExportImport,
}

impl RateLimitTier {
    /// Get the maximum requests per window
    fn max_requests(&self) -> usize {
        match self {
            RateLimitTier::Auth => 10,
            RateLimitTier::Read => 1000,
            RateLimitTier::Write => 100,
            RateLimitTier::Search => 100,
            RateLimitTier::Analysis => 50,
            RateLimitTier::Build => 10,
            RateLimitTier::ExportImport => 5,
        }
    }

    /// Get the window duration
    fn window_duration(&self) -> Duration {
        match self {
            RateLimitTier::ExportImport => Duration::from_secs(3600), // 1 hour
            _ => Duration::from_secs(60), // 1 minute for all other tiers
        }
    }
}

/// Rate limiter state for a single client
#[derive(Debug, Clone)]
struct RateLimitState {
    count: usize,
    window_start: Instant,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.window_start = Instant::now();
    }
}

/// Rate limiter
#[derive(Clone)]
pub struct RateLimiter {
    states: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if request should be allowed
    async fn check_rate_limit(&self, key: &str, tier: RateLimitTier) -> Result<(), RateLimitError> {
        let mut states = self.states.write().await;

        let state = states.entry(key.to_string()).or_insert_with(RateLimitState::new);

        let now = Instant::now();
        let window_duration = tier.window_duration();

        // Reset window if expired
        if now.duration_since(state.window_start) >= window_duration {
            state.reset();
        }

        // Check limit
        if state.count >= tier.max_requests() {
            return Err(RateLimitError {
                tier,
                retry_after: window_duration
                    .saturating_sub(now.duration_since(state.window_start))
                    .as_secs(),
            });
        }

        state.count += 1;
        Ok(())
    }

    /// Cleanup expired entries periodically
    pub async fn cleanup_expired(&self) {
        let mut states = self.states.write().await;
        let now = Instant::now();

        states.retain(|_, state| {
            now.duration_since(state.window_start) < Duration::from_secs(120)
        });
    }
}

/// Rate limit error
#[derive(Debug)]
struct RateLimitError {
    tier: RateLimitTier,
    retry_after: u64,
}

#[derive(Debug, Serialize)]
struct RateLimitErrorResponse {
    success: bool,
    error: RateLimitErrorDetail,
}

#[derive(Debug, Serialize)]
struct RateLimitErrorDetail {
    code: String,
    message: String,
    retry_after: u64,
    tier: String,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        let tier_name = format!("{:?}", self.tier);

        let response = RateLimitErrorResponse {
            success: false,
            error: RateLimitErrorDetail {
                code: "RATE_LIMIT_EXCEEDED".to_string(),
                message: format!(
                    "Rate limit exceeded for {} tier. Try again in {} seconds.",
                    tier_name, self.retry_after
                ),
                retry_after: self.retry_after,
                tier: tier_name,
            },
        };

        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", self.retry_after.to_string())],
            Json(response),
        )
            .into_response()
    }
}

/// Rate limit middleware
pub struct RateLimitMiddleware;

impl RateLimitMiddleware {
    /// Apply rate limiting
    pub async fn apply(
        limiter: RateLimiter,
        tier: RateLimitTier,
        req: Request,
        next: Next,
    ) -> Result<Response, RateLimitError> {
        // Extract client identifier (IP address or user ID from auth)
        let client_id = extract_client_id(&req);

        // Check rate limit
        limiter.check_rate_limit(&client_id, tier).await?;

        Ok(next.run(req).await)
    }
}

/// Extract client identifier from request
fn extract_client_id(req: &Request) -> String {
    // Try to get user ID from extensions (set by auth middleware)
    if let Some(claims) = req.extensions().get::<crate::services::auth::Claims>() {
        return format!("user:{}", claims.sub);
    }

    // Fallback to IP address
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip) = forwarded_str.split(',').next() {
                return format!("ip:{}", ip.trim());
            }
        }
    }

    // Default to unknown
    "ip:unknown".to_string()
}

/// Helper function to create rate limit middleware for a specific tier
pub fn rate_limit(limiter: RateLimiter, tier: RateLimitTier) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, RateLimitError>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            RateLimitMiddleware::apply(limiter, tier, req, next).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let tier = RateLimitTier::Auth;

        // Should allow up to max requests
        for _ in 0..tier.max_requests() {
            assert!(limiter.check_rate_limit("test_client", tier).await.is_ok());
        }

        // Should deny next request
        assert!(limiter.check_rate_limit("test_client", tier).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_clients() {
        let limiter = RateLimiter::new();
        let tier = RateLimitTier::Auth;

        // Client 1 exhausts limit
        for _ in 0..tier.max_requests() {
            assert!(limiter.check_rate_limit("client1", tier).await.is_ok());
        }

        // Client 2 should still have quota
        assert!(limiter.check_rate_limit("client2", tier).await.is_ok());
    }
}
