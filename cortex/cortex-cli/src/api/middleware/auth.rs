//! Authentication middleware

use crate::api::routes::auth::Claims;
use axum::{
    extract::{FromRequestParts, Request},
    http::{header::{AUTHORIZATION, WWW_AUTHENTICATE}, request::Parts, StatusCode, HeaderValue},
    middleware::Next,
    response::{Response, IntoResponse},
    Json,
};
use bcrypt::verify;
use cortex_storage::ConnectionManager;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Authenticated user information stored in request extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub session_id: Option<String>,
}

impl AuthUser {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is an admin
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        self.roles.iter().any(|r| roles.contains(&r.as_str()))
    }
}

impl From<&Claims> for AuthUser {
    fn from(claims: &Claims) -> Self {
        Self {
            user_id: claims.sub.clone(),
            email: claims.email.clone(),
            roles: claims.roles.clone(),
            session_id: None,
        }
    }
}

/// Authentication middleware state
#[derive(Clone)]
pub struct AuthState {
    pub storage: Arc<ConnectionManager>,
    pub jwt_secret: String,
}

impl AuthState {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "cortex-dev-secret-change-in-production".to_string()),
        }
    }
}

/// Authentication error response
#[derive(Debug, Serialize)]
pub struct AuthErrorResponse {
    pub success: bool,
    pub error: AuthErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct AuthErrorDetail {
    pub code: String,
    pub message: String,
}

/// Authentication middleware
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Validate authentication token (middleware function)
    pub async fn validate(
        state: AuthState,
        mut req: Request,
        next: Next,
    ) -> Response {
        // Extract authorization header
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        if let Some(auth_header) = auth_header {
            // Try Bearer token first
            if auth_header.starts_with("Bearer ") {
                let token = auth_header.trim_start_matches("Bearer ");

                match validate_jwt(token, &state.jwt_secret) {
                    Ok(claims) => {
                        // Create AuthUser from claims
                        let auth_user = AuthUser::from(&claims);

                        // Log authentication
                        tracing::debug!(
                            user_id = %auth_user.user_id,
                            email = %auth_user.email,
                            roles = ?auth_user.roles,
                            "User authenticated via JWT"
                        );

                        // Insert both claims and AuthUser into request extensions
                        req.extensions_mut().insert(claims);
                        req.extensions_mut().insert(auth_user);
                        return next.run(req).await;
                    }
                    Err(e) => {
                        tracing::warn!("JWT validation failed: {}", e);
                        return unauthorized_response("Invalid or expired token").into_response();
                    }
                }
            }
            // Try API key
            else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ");

                match validate_api_key(api_key, &state).await {
                    Ok(claims) => {
                        let auth_user = AuthUser::from(&claims);

                        tracing::debug!(
                            user_id = %auth_user.user_id,
                            email = %auth_user.email,
                            roles = ?auth_user.roles,
                            "User authenticated via API key"
                        );

                        req.extensions_mut().insert(claims);
                        req.extensions_mut().insert(auth_user);
                        return next.run(req).await;
                    }
                    Err(e) => {
                        tracing::warn!("API key validation failed: {}", e);
                        return unauthorized_response("Invalid API key").into_response();
                    }
                }
            }
        }

        unauthorized_response("Authentication required").into_response()
    }

    /// Optional authentication - doesn't fail if no token provided
    pub async fn optional(
        state: AuthState,
        mut req: Request,
        next: Next,
    ) -> Response {
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        if let Some(auth_header) = auth_header {
            if auth_header.starts_with("Bearer ") {
                let token = auth_header.trim_start_matches("Bearer ");

                if let Ok(claims) = validate_jwt(token, &state.jwt_secret) {
                    let auth_user = AuthUser::from(&claims);
                    req.extensions_mut().insert(claims);
                    req.extensions_mut().insert(auth_user);
                }
            } else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ");

                if let Ok(claims) = validate_api_key(api_key, &state).await {
                    let auth_user = AuthUser::from(&claims);
                    req.extensions_mut().insert(claims);
                    req.extensions_mut().insert(auth_user);
                }
            }
        }

        next.run(req).await
    }

    /// Role-based access control middleware - requires specific role
    pub async fn require_role(
        required_role: String,
        req: Request,
        next: Next,
    ) -> Response {
        // Get AuthUser from request extensions
        let auth_user = match req.extensions().get::<AuthUser>() {
            Some(user) => user,
            None => return forbidden_response("Authentication required").into_response(),
        };

        // Check if user has required role or is admin
        if auth_user.has_role(&required_role) || auth_user.is_admin() {
            tracing::debug!(
                user_id = %auth_user.user_id,
                required_role = %required_role,
                "Role check passed"
            );
            next.run(req).await
        } else {
            tracing::warn!(
                user_id = %auth_user.user_id,
                required_role = %required_role,
                user_roles = ?auth_user.roles,
                "Insufficient permissions"
            );
            forbidden_response(&format!(
                "Insufficient permissions. Required role: {}",
                required_role
            )).into_response()
        }
    }

    /// Admin-only access control middleware
    pub async fn require_admin(
        req: Request,
        next: Next,
    ) -> Response {
        let auth_user = match req.extensions().get::<AuthUser>() {
            Some(user) => user,
            None => return forbidden_response("Authentication required").into_response(),
        };

        if auth_user.is_admin() {
            tracing::debug!(
                user_id = %auth_user.user_id,
                "Admin check passed"
            );
            next.run(req).await
        } else {
            tracing::warn!(
                user_id = %auth_user.user_id,
                user_roles = ?auth_user.roles,
                "Admin access denied"
            );
            forbidden_response("Admin access required").into_response()
        }
    }

    /// Check if user has any of the specified roles
    pub async fn require_any_role(
        required_roles: Vec<String>,
        req: Request,
        next: Next,
    ) -> Response {
        let auth_user = match req.extensions().get::<AuthUser>() {
            Some(user) => user,
            None => return forbidden_response("Authentication required").into_response(),
        };

        let role_refs: Vec<&str> = required_roles.iter().map(|s| s.as_str()).collect();

        if auth_user.has_any_role(&role_refs) || auth_user.is_admin() {
            tracing::debug!(
                user_id = %auth_user.user_id,
                required_roles = ?required_roles,
                "Role check passed"
            );
            next.run(req).await
        } else {
            tracing::warn!(
                user_id = %auth_user.user_id,
                required_roles = ?required_roles,
                user_roles = ?auth_user.roles,
                "Insufficient permissions"
            );
            forbidden_response(&format!(
                "Insufficient permissions. Required one of: {}",
                required_roles.join(", ")
            )).into_response()
        }
    }
}

/// Validate JWT token
fn validate_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    // Verify it's an access token
    if token_data.claims.token_type != "access" {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }

    Ok(token_data.claims)
}

/// Validate API key
async fn validate_api_key(api_key: &str, state: &AuthState) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
    // Get all API keys from database
    let query = "SELECT * FROM api_keys WHERE expires_at IS NULL OR expires_at > time::now()";

    let conn = state.storage.acquire().await?;
    let mut result = conn.connection().query(query).await?;

    #[derive(Deserialize)]
    struct ApiKeyRecord {
        id: String,
        user_id: String,
        name: String,
        key_hash: String,
        scopes: Vec<String>,
    }

    let api_keys: Vec<ApiKeyRecord> = result.take(0)?;

    // Try to find matching key by verifying hash
    for key_record in api_keys {
        if verify(api_key, &key_record.key_hash).unwrap_or(false) {
            // Get user info
            let user_query = format!("SELECT * FROM users WHERE id = '{}' LIMIT 1", key_record.user_id);
            let mut user_result = conn.connection().query(&user_query).await?;

            #[derive(Deserialize)]
            struct User {
                email: String,
                roles: Vec<String>,
            }

            let users: Vec<User> = user_result.take(0)?;
            if let Some(user) = users.first() {
                return Ok(Claims {
                    sub: key_record.user_id,
                    email: user.email.clone(),
                    roles: user.roles.clone(),
                    exp: 0, // API keys don't expire in token
                    iat: chrono::Utc::now().timestamp(),
                    token_type: "api_key".to_string(),
                });
            }
        }
    }

    Err("Invalid API key".into())
}

/// Create unauthorized response with WWW-Authenticate header
fn unauthorized_response(message: &str) -> (StatusCode, [(axum::http::HeaderName, HeaderValue); 1], Json<AuthErrorResponse>) {
    (
        StatusCode::UNAUTHORIZED,
        [(
            WWW_AUTHENTICATE,
            HeaderValue::from_static("Bearer realm=\"Cortex API\""),
        )],
        Json(AuthErrorResponse {
            success: false,
            error: AuthErrorDetail {
                code: "UNAUTHORIZED".to_string(),
                message: message.to_string(),
            },
        }),
    )
}

/// Create forbidden response
fn forbidden_response(message: &str) -> (StatusCode, Json<AuthErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(AuthErrorResponse {
            success: false,
            error: AuthErrorDetail {
                code: "FORBIDDEN".to_string(),
                message: message.to_string(),
            },
        }),
    )
}

/// Extractor for authenticated requests - extracts Claims
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, [(axum::http::HeaderName, HeaderValue); 1], Json<AuthErrorResponse>);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    [(
                        WWW_AUTHENTICATE,
                        HeaderValue::from_static("Bearer realm=\"Cortex API\""),
                    )],
                    Json(AuthErrorResponse {
                        success: false,
                        error: AuthErrorDetail {
                            code: "UNAUTHORIZED".to_string(),
                            message: "Missing or invalid authentication token".to_string(),
                        },
                    }),
                )
            });

        async move { result }
    }
}

/// Extractor for authenticated requests - extracts AuthUser
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, [(axum::http::HeaderName, HeaderValue); 1], Json<AuthErrorResponse>);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    [(
                        WWW_AUTHENTICATE,
                        HeaderValue::from_static("Bearer realm=\"Cortex API\""),
                    )],
                    Json(AuthErrorResponse {
                        success: false,
                        error: AuthErrorDetail {
                            code: "UNAUTHORIZED".to_string(),
                            message: "Missing or invalid authentication token".to_string(),
                        },
                    }),
                )
            });

        async move { result }
    }
}
