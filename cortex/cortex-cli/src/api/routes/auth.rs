//! Authentication routes

use crate::api::{error::ApiError, types::ApiResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use cortex_storage::ConnectionManager;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Authentication context shared across routes
#[derive(Clone)]
pub struct AuthContext {
    pub storage: Arc<ConnectionManager>,
    pub jwt_secret: String,
    pub access_token_expiry: i64,  // minutes
    pub refresh_token_expiry: i64, // days
}

impl AuthContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "cortex-dev-secret-change-in-production".to_string()),
            access_token_expiry: 15,  // 15 minutes
            refresh_token_expiry: 7,  // 7 days
        }
    }
}

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // User ID
    pub email: String,    // User email
    pub roles: Vec<String>, // User roles
    pub exp: i64,         // Expiration time
    pub iat: i64,         // Issued at
    pub token_type: String, // "access" or "refresh"
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

/// User stored in database
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    email: String,
    password_hash: String,
    roles: Vec<String>,
    created_at: chrono::DateTime<Utc>,
}

/// Session stored in database
#[derive(Debug, Serialize, Deserialize)]
struct Session {
    id: String,
    user_id: String,
    refresh_token: String,
    expires_at: chrono::DateTime<Utc>,
    created_at: chrono::DateTime<Utc>,
}

/// API key stored in database
#[derive(Debug, Serialize, Deserialize)]
struct ApiKey {
    id: String,
    user_id: String,
    name: String,
    key_hash: String,
    scopes: Vec<String>,
    expires_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

/// POST /api/v1/auth/login - User login
async fn login(
    State(ctx): State<AuthContext>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Query user from database
    let query = format!(
        "SELECT * FROM users WHERE email = '{}' LIMIT 1",
        req.email
    );

    let conn = ctx.storage.acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Database connection failed: {}", e)))?;

    let mut result = conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

    let users: Vec<User> = result.take(0)
        .map_err(|e| ApiError::Internal(format!("Failed to parse users: {}", e)))?;

    let user = users.first()
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    let valid = verify(&req.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(format!("Password verification failed: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid email or password".to_string()));
    }

    // Generate tokens
    let access_token = generate_access_token(&ctx, &user)?;
    let refresh_token = generate_refresh_token(&ctx, &user)?;

    // Store refresh token in sessions table
    let session_id = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(ctx.refresh_token_expiry);

    let session = Session {
        id: session_id.clone(),
        user_id: user.id.clone(),
        refresh_token: refresh_token.clone(),
        expires_at,
        created_at: Utc::now(),
    };

    let insert_query = format!(
        "CREATE sessions:{} CONTENT {}",
        session_id,
        serde_json::to_string(&session)
            .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?
    );

    conn.connection().query(&insert_query)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store session: {}", e)))?;

    let response = LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: ctx.access_token_expiry * 60, // Convert to seconds
        user: UserInfo {
            id: user.id.clone(),
            email: user.email.clone(),
            roles: user.roles.clone(),
            created_at: user.created_at,
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

    // Decode and validate refresh token
    let token_data = decode::<Claims>(
        &req.refresh_token,
        &DecodingKey::from_secret(ctx.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| ApiError::Unauthorized(format!("Invalid refresh token: {}", e)))?;

    if token_data.claims.token_type != "refresh" {
        return Err(ApiError::Unauthorized("Invalid token type".to_string()));
    }

    // Verify session exists in database
    let query = format!(
        "SELECT * FROM sessions WHERE user_id = '{}' AND refresh_token = '{}' AND expires_at > {} LIMIT 1",
        token_data.claims.sub,
        req.refresh_token,
        Utc::now().timestamp()
    );

    let conn = ctx.storage.acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Database connection failed: {}", e)))?;

    let mut result = conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

    let sessions: Vec<Session> = result.take(0)
        .map_err(|e| ApiError::Internal(format!("Failed to parse sessions: {}", e)))?;

    if sessions.is_empty() {
        return Err(ApiError::Unauthorized("Session not found or expired".to_string()));
    }

    // Get user info
    let user_query = format!(
        "SELECT * FROM users WHERE id = '{}' LIMIT 1",
        token_data.claims.sub
    );

    let mut user_result = conn.connection().query(&user_query)
        .await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

    let users: Vec<User> = user_result.take(0)
        .map_err(|e| ApiError::Internal(format!("Failed to parse users: {}", e)))?;

    let user = users.first()
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    // Generate new access token
    let access_token = generate_access_token(&ctx, user)?;

    let response = serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ctx.access_token_expiry * 60,
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

    // Generate API key
    let key_id = Uuid::new_v4().to_string();
    let api_key = format!("cortex_{}", Uuid::new_v4().simple());

    // Hash the API key for storage
    let key_hash = hash(&api_key, DEFAULT_COST)
        .map_err(|e| ApiError::Internal(format!("Failed to hash API key: {}", e)))?;

    let expires_at = req.expires_in_days.map(|days| Utc::now() + Duration::days(days));

    let api_key_record = ApiKey {
        id: key_id.clone(),
        user_id: claims.sub.clone(),
        name: req.name.clone(),
        key_hash,
        scopes: req.scopes.clone(),
        expires_at,
        created_at: Utc::now(),
    };

    let conn = ctx.storage.acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Database connection failed: {}", e)))?;

    let insert_query = format!(
        "CREATE api_keys:{} CONTENT {}",
        key_id,
        serde_json::to_string(&api_key_record)
            .map_err(|e| ApiError::Internal(format!("Serialization failed: {}", e)))?
    );

    conn.connection().query(&insert_query)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store API key: {}", e)))?;

    let response = ApiKeyResponse {
        key_id: key_id.clone(),
        api_key, // Only returned this once
        name: req.name,
        scopes: req.scopes,
        expires_at,
        created_at: api_key_record.created_at,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(response, request_id, duration))))
}

/// POST /api/v1/auth/logout - Logout and invalidate session
async fn logout(
    State(ctx): State<AuthContext>,
    claims: Claims, // Extracted by auth middleware
) -> Result<impl IntoResponse, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let start = Instant::now();

    // Delete all sessions for user
    let query = format!(
        "DELETE FROM sessions WHERE user_id = '{}'",
        claims.sub
    );

    let conn = ctx.storage.acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Database connection failed: {}", e)))?;

    conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete sessions: {}", e)))?;

    let response = serde_json::json!({
        "message": "Logged out successfully"
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

    let query = format!(
        "SELECT * FROM users WHERE id = '{}' LIMIT 1",
        claims.sub
    );

    let conn = ctx.storage.acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Database connection failed: {}", e)))?;

    let mut result = conn.connection().query(&query)
        .await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

    let users: Vec<User> = result.take(0)
        .map_err(|e| ApiError::Internal(format!("Failed to parse users: {}", e)))?;

    let user = users.first()
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let user_info = UserInfo {
        id: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        created_at: user.created_at,
    };

    let duration = start.elapsed().as_millis() as u64;

    Ok((StatusCode::OK, Json(ApiResponse::success(user_info, request_id, duration))))
}

/// Generate access token
fn generate_access_token(ctx: &AuthContext, user: &User) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::minutes(ctx.access_token_expiry);

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "access".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ctx.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to generate token: {}", e)))
}

/// Generate refresh token
fn generate_refresh_token(ctx: &AuthContext, user: &User) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::days(ctx.refresh_token_expiry);

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        roles: user.roles.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        token_type: "refresh".to_string(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ctx.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("Failed to generate token: {}", e)))
}

/// Create authentication routes
pub fn auth_routes(ctx: AuthContext) -> Router {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/api-key", post(create_api_key))
        .route("/api/v1/auth/me", get(me))
        .with_state(ctx)
}
