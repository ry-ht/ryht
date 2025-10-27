//! Authentication routes

use crate::api::{error::ApiError, types::ApiResponse};
use crate::api::middleware::auth::BearerToken;
use crate::services::auth::{AuthService, Claims};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Authentication context shared across routes
#[derive(Clone)]
pub struct AuthContext {
    pub auth_service: Arc<AuthService>,
}

impl AuthContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            auth_service: Arc::new(AuthService::new(storage)),
        }
    }
}


/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub created_at: chrono::DateTime<Utc>,
}

/// Register request
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub roles: Option<Vec<String>>,
}

/// Refresh token request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// API key request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_in_days: Option<i64>,
}

/// API key response
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub key_id: String,
    pub api_key: String, // Only returned once
    pub name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

/// POST /api/v1/auth/register - User registration
async fn register(
    State(ctx): State<AuthContext>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Create user with default or provided roles
    let roles = req.roles.unwrap_or_else(|| vec!["user".to_string()]);

    // Save password before it's moved
    let password = req.password.clone();

    let user = ctx.auth_service
        .create_user(req.email.clone(), req.password, roles)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Generate tokens for immediate login
    let authenticated = ctx.auth_service
        .authenticate_user(&req.email, &password)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = LoginResponse {
        access_token: authenticated.access_token,
        refresh_token: authenticated.refresh_token,
        token_type: authenticated.token_type,
        expires_in: authenticated.expires_in,
        user: UserInfo {
            id: user.id,
            email: user.email,
            roles: user.roles,
            created_at: user.created_at,
        },
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response, request_id, duration))))
}

/// POST /api/v1/auth/login - User login
async fn login(
    State(ctx): State<AuthContext>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use AuthService to authenticate user
    let authenticated = ctx.auth_service
        .authenticate_user(&req.email, &req.password)
        .await
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    let response = LoginResponse {
        access_token: authenticated.access_token,
        refresh_token: authenticated.refresh_token,
        token_type: authenticated.token_type,
        expires_in: authenticated.expires_in,
        user: UserInfo {
            id: authenticated.user.id,
            email: authenticated.user.email,
            roles: authenticated.user.roles,
            created_at: authenticated.user.created_at,
        },
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::OK, Json(ApiResponse::success(response, request_id, duration))))
}

/// POST /api/v1/auth/refresh - Refresh access token
async fn refresh_token(
    State(ctx): State<AuthContext>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use AuthService to refresh token
    let tokens = ctx.auth_service
        .refresh_token(&req.refresh_token)
        .await
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    let response = serde_json::json!({
        "access_token": tokens.access_token,
        "token_type": tokens.token_type,
        "expires_in": tokens.expires_in,
    });

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::OK, Json(ApiResponse::success(response, request_id, duration))))
}

/// POST /api/v1/auth/api-key - Create API key
async fn create_api_key(
    State(ctx): State<AuthContext>,
    claims: Claims, // Extracted by auth middleware
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use AuthService to create API key
    let api_key = ctx.auth_service
        .create_api_key(&claims.sub, req.name.clone(), req.scopes.clone(), req.expires_in_days)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = ApiKeyResponse {
        key_id: api_key.id,
        api_key: api_key.key, // Only returned this once
        name: api_key.name,
        scopes: api_key.scopes,
        expires_at: api_key.expires_at,
        created_at: api_key.created_at,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response, request_id, duration))))
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    /// If true, revokes all tokens for the user (logout from all devices)
    /// If false or not provided, only revokes the current token
    pub logout_all_devices: Option<bool>,
}

/// POST /api/v1/auth/logout - Logout and invalidate session
async fn logout(
    State(ctx): State<AuthContext>,
    claims: Claims,         // Extracted by auth middleware
    token: BearerToken,     // Raw token extracted by auth middleware
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    if req.logout_all_devices.unwrap_or(false) {
        // Logout from all devices - revoke all user tokens and sessions
        ctx.auth_service
            .revoke_all_user_tokens(&claims.sub)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    } else {
        // Single device logout - only revoke the current token
        ctx.auth_service
            .revoke_token(&token.0)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    let response = serde_json::json!({
        "message": if req.logout_all_devices.unwrap_or(false) {
            "Logged out from all devices successfully"
        } else {
            "Logged out successfully"
        }
    });

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::OK, Json(ApiResponse::success(response, request_id, duration))))
}

/// GET /api/v1/auth/me - Get current user info
async fn me(
    State(ctx): State<AuthContext>,
    claims: Claims, // Extracted by auth middleware
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Use AuthService to get user info
    let user = ctx.auth_service
        .get_user(&claims.sub)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let user_info = UserInfo {
        id: user.id,
        email: user.email,
        roles: user.roles,
        created_at: user.created_at,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::OK, Json(ApiResponse::success(user_info, request_id, duration))))
}

/// Create public authentication routes (no auth required)
pub fn public_auth_routes(ctx: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh_token))
        .with_state(ctx)
}

/// Create protected authentication routes (auth required)
pub fn protected_auth_routes(ctx: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/api-key", post(create_api_key))
        .route("/api/v1/auth/me", get(me))
        .with_state(ctx)
}

/// Create authentication routes (deprecated - use public_auth_routes and protected_auth_routes)
#[deprecated(note = "Use public_auth_routes and protected_auth_routes instead")]
pub fn auth_routes(ctx: AuthContext) -> Router {
    public_auth_routes(ctx)
}
