//! API middleware

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    http::{request::Parts, StatusCode, HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Request logging middleware
pub async fn logging(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let method = req.method().clone();
    let uri = req.uri().clone();

    info!("Request: {} {}", method, uri);

    let response = next.run(req).await;

    info!("Response: {} {} -> {}", method, uri, response.status());

    Ok(response)
}

/// API key for authentication
#[derive(Debug, Clone)]
pub struct ApiKey(pub String);

/// Authenticated user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub api_key: String,
}

/// Authentication error response
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub message: String,
}

/// API key validator
#[derive(Clone)]
pub struct ApiKeyValidator {
    valid_keys: Arc<RwLock<HashMap<String, String>>>,
}

impl ApiKeyValidator {
    pub fn new() -> Self {
        let mut valid_keys = HashMap::new();

        // Add default API key for development
        // In production, load from secure storage
        if let Ok(api_key) = std::env::var("AXON_API_KEY") {
            valid_keys.insert(api_key, "default-user".to_string());
        } else {
            valid_keys.insert(
                "axon-dev-key-change-in-production".to_string(),
                "default-user".to_string(),
            );
        }

        Self {
            valid_keys: Arc::new(RwLock::new(valid_keys)),
        }
    }

    pub async fn validate(&self, key: &str) -> Option<String> {
        let keys = self.valid_keys.read().await;
        keys.get(key).cloned()
    }

    pub async fn add_key(&self, key: String, user_id: String) {
        let mut keys = self.valid_keys.write().await;
        keys.insert(key, user_id);
    }

    pub async fn revoke_key(&self, key: &str) -> bool {
        let mut keys = self.valid_keys.write().await;
        keys.remove(key).is_some()
    }
}

/// Authentication middleware
pub async fn auth(
    validator: ApiKeyValidator,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        // Support both "Bearer" and "ApiKey" formats
        let api_key = if auth_header.starts_with("Bearer ") {
            auth_header.trim_start_matches("Bearer ")
        } else if auth_header.starts_with("ApiKey ") {
            auth_header.trim_start_matches("ApiKey ")
        } else {
            auth_header
        };

        if let Some(user_id) = validator.validate(api_key).await {
            let auth_user = AuthUser {
                user_id: user_id.clone(),
                api_key: api_key.to_string(),
            };

            debug!(user_id = %user_id, "User authenticated");

            req.extensions_mut().insert(ApiKey(api_key.to_string()));
            req.extensions_mut().insert(auth_user);

            return Ok(next.run(req).await);
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        [(
            axum::http::header::WWW_AUTHENTICATE,
            HeaderValue::from_static("Bearer realm=\"Axon API\""),
        )],
        Json(AuthErrorResponse {
            error: "UNAUTHORIZED".to_string(),
            message: "Invalid or missing API key".to_string(),
        }),
    ))
}

/// Optional authentication - doesn't fail if no key provided
pub async fn optional_auth(
    validator: ApiKeyValidator,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        let api_key = if auth_header.starts_with("Bearer ") {
            auth_header.trim_start_matches("Bearer ")
        } else if auth_header.starts_with("ApiKey ") {
            auth_header.trim_start_matches("ApiKey ")
        } else {
            auth_header
        };

        if let Some(user_id) = validator.validate(api_key).await {
            let auth_user = AuthUser {
                user_id,
                api_key: api_key.to_string(),
            };
            req.extensions_mut().insert(ApiKey(api_key.to_string()));
            req.extensions_mut().insert(auth_user);
        }
    }

    next.run(req).await
}

/// Extractor for authenticated requests
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<AuthErrorResponse>);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts.extensions.get::<AuthUser>().cloned().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: "UNAUTHORIZED".to_string(),
                    message: "Authentication required".to_string(),
                }),
            )
        });

        async move { result }
    }
}

/// Rate limit tiers
#[derive(Debug, Clone, Copy)]
pub enum RateLimitTier {
    Auth,      // 10 req/minute
    Read,      // 1000 req/minute
    Write,     // 100 req/minute
    Execute,   // 50 req/minute
    Admin,     // 200 req/minute
}

impl RateLimitTier {
    fn max_requests(&self) -> usize {
        match self {
            RateLimitTier::Auth => 10,
            RateLimitTier::Read => 1000,
            RateLimitTier::Write => 100,
            RateLimitTier::Execute => 50,
            RateLimitTier::Admin => 200,
        }
    }

    fn window_duration(&self) -> Duration {
        Duration::from_secs(60) // 1 minute window for all tiers
    }
}

/// Rate limiter state
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

    pub async fn check_rate_limit(
        &self,
        key: &str,
        tier: RateLimitTier,
    ) -> Result<(), RateLimitError> {
        let mut states = self.states.write().await;

        let state = states
            .entry(key.to_string())
            .or_insert_with(RateLimitState::new);

        let now = Instant::now();
        let window_duration = tier.window_duration();

        if now.duration_since(state.window_start) >= window_duration {
            state.reset();
        }

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

    pub async fn cleanup_expired(&self) {
        let mut states = self.states.write().await;
        let now = Instant::now();

        states.retain(|_, state| now.duration_since(state.window_start) < Duration::from_secs(120));
    }
}

/// Rate limit error
#[derive(Debug)]
pub struct RateLimitError {
    pub tier: RateLimitTier,
    pub retry_after: u64,
}

#[derive(Debug, Serialize)]
struct RateLimitErrorResponse {
    error: String,
    message: String,
    retry_after: u64,
}

impl IntoResponse for RateLimitError {
    fn into_response(self) -> Response {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", self.retry_after.to_string())],
            Json(RateLimitErrorResponse {
                error: "RATE_LIMIT_EXCEEDED".to_string(),
                message: format!(
                    "Rate limit exceeded. Try again in {} seconds.",
                    self.retry_after
                ),
                retry_after: self.retry_after,
            }),
        )
            .into_response()
    }
}

/// Rate limit middleware
pub async fn rate_limit(
    limiter: RateLimiter,
    tier: RateLimitTier,
    req: Request,
    next: Next,
) -> Result<Response, RateLimitError> {
    let client_id = extract_client_id(&req);
    limiter.check_rate_limit(&client_id, tier).await?;
    Ok(next.run(req).await)
}

/// Extract client identifier from request
fn extract_client_id(req: &Request) -> String {
    if let Some(auth_user) = req.extensions().get::<AuthUser>() {
        return format!("user:{}", auth_user.user_id);
    }

    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip) = forwarded_str.split(',').next() {
                return format!("ip:{}", ip.trim());
            }
        }
    }

    "ip:unknown".to_string()
}
