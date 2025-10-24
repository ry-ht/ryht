//! Authentication middleware

use crate::services::auth::{AuthService, Claims};
use axum::{
    extract::{FromRequestParts, Request},
    http::{header::{AUTHORIZATION, WWW_AUTHENTICATE}, request::Parts, StatusCode, HeaderValue},
    middleware::Next,
    response::{Response, IntoResponse},
    Json,
};
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Raw bearer token extracted from request
#[derive(Debug, Clone)]
pub struct BearerToken(pub String);

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
    pub auth_service: Arc<AuthService>,
}

impl AuthState {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            auth_service: Arc::new(AuthService::new(storage)),
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
        // Extract authorization header - clone to avoid borrowing issues
        let auth_header = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        if let Some(auth_header) = auth_header {
            // Try Bearer token first
            if auth_header.starts_with("Bearer ") {
                let token = auth_header.trim_start_matches("Bearer ").to_string();

                // Check if token is blacklisted
                match state.auth_service.is_token_blacklisted(&token).await {
                    Ok(true) => {
                        tracing::warn!("Token is blacklisted (revoked)");
                        return unauthorized_response("Token has been revoked").into_response();
                    }
                    Err(e) => {
                        tracing::error!("Error checking token blacklist: {}", e);
                        // Continue with validation - fail open to prevent blacklist issues from blocking all auth
                    }
                    Ok(false) => {
                        // Token not blacklisted, continue with validation
                    }
                }

                // Use AuthService to validate token
                match state.auth_service.validate_token(&token).await {
                    Ok(Some(session)) => {
                        // Create claims from validated session
                        let claims = Claims {
                            sub: session.user_id.clone(),
                            email: session.email.clone(),
                            roles: session.roles.clone(),
                            exp: session.expires_at.timestamp(),
                            iat: chrono::Utc::now().timestamp(),
                            token_type: "access".to_string(),
                        };

                        let auth_user = AuthUser::from(&claims);

                        tracing::debug!(
                            user_id = %auth_user.user_id,
                            email = %auth_user.email,
                            roles = ?auth_user.roles,
                            "User authenticated via JWT"
                        );

                        // Store the raw token for logout functionality
                        req.extensions_mut().insert(BearerToken(token));
                        req.extensions_mut().insert(claims);
                        req.extensions_mut().insert(auth_user);
                        return next.run(req).await;
                    }
                    Ok(None) | Err(_) => {
                        tracing::warn!("JWT validation failed");
                        return unauthorized_response("Invalid or expired token").into_response();
                    }
                }
            }
            // Try API key
            else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ");

                // Use AuthService to validate API key
                match state.auth_service.validate_api_key(api_key).await {
                    Ok(Some(key_info)) => {
                        // Create claims from API key info
                        let claims = Claims {
                            sub: key_info.user_id.clone(),
                            email: String::new(), // API keys don't have email in info
                            roles: vec![], // Would need to fetch user to get roles
                            exp: key_info.expires_at.map(|dt| dt.timestamp()).unwrap_or(0),
                            iat: chrono::Utc::now().timestamp(),
                            token_type: "api_key".to_string(),
                        };

                        let auth_user = AuthUser::from(&claims);

                        tracing::debug!(
                            user_id = %auth_user.user_id,
                            "User authenticated via API key"
                        );

                        req.extensions_mut().insert(claims);
                        req.extensions_mut().insert(auth_user);
                        return next.run(req).await;
                    }
                    Ok(None) | Err(_) => {
                        tracing::warn!("API key validation failed");
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
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        if let Some(auth_header) = auth_header {
            if auth_header.starts_with("Bearer ") {
                let token = auth_header.trim_start_matches("Bearer ").to_string();

                // Check if token is blacklisted
                let is_blacklisted = state.auth_service.is_token_blacklisted(&token).await.unwrap_or(false);

                if !is_blacklisted {
                    if let Ok(Some(session)) = state.auth_service.validate_token(&token).await {
                        let claims = Claims {
                            sub: session.user_id.clone(),
                            email: session.email.clone(),
                            roles: session.roles.clone(),
                            exp: session.expires_at.timestamp(),
                            iat: chrono::Utc::now().timestamp(),
                            token_type: "access".to_string(),
                        };
                        let auth_user = AuthUser::from(&claims);
                        req.extensions_mut().insert(BearerToken(token));
                        req.extensions_mut().insert(claims);
                        req.extensions_mut().insert(auth_user);
                    }
                }
            } else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ").to_string();

                if let Ok(Some(key_info)) = state.auth_service.validate_api_key(&api_key).await {
                    let claims = Claims {
                        sub: key_info.user_id.clone(),
                        email: String::new(),
                        roles: vec![],
                        exp: key_info.expires_at.map(|dt| dt.timestamp()).unwrap_or(0),
                        iat: chrono::Utc::now().timestamp(),
                        token_type: "api_key".to_string(),
                    };
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

// Note: validate_jwt and validate_api_key functions are now in AuthService
// We don't need duplicate validation logic here

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

/// Extractor for bearer token - extracts raw token string
impl<S> FromRequestParts<S> for BearerToken
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
            .get::<BearerToken>()
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
