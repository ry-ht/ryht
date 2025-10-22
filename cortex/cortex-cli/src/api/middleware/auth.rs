//! Authentication middleware

use crate::api::routes::auth::Claims;
use axum::{
    extract::{FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use bcrypt::verify;
use cortex_storage::ConnectionManager;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    ) -> Result<Response, StatusCode> {
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
                        // Insert claims into request extensions
                        req.extensions_mut().insert(claims);
                        return Ok(next.run(req).await);
                    }
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                }
            }
            // Try API key
            else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ");

                match validate_api_key(api_key, &state).await {
                    Ok(claims) => {
                        req.extensions_mut().insert(claims);
                        return Ok(next.run(req).await);
                    }
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                }
            }
        }

        Err(StatusCode::UNAUTHORIZED)
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
                    req.extensions_mut().insert(claims);
                }
            } else if auth_header.starts_with("ApiKey ") {
                let api_key = auth_header.trim_start_matches("ApiKey ");

                if let Ok(claims) = validate_api_key(api_key, &state).await {
                    req.extensions_mut().insert(claims);
                }
            }
        }

        next.run(req).await
    }

    /// Role-based access control middleware
    pub fn require_role(required_role: String) -> impl Fn(Claims, Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
        move |claims: Claims, req: Request, next: Next| {
            let required_role = required_role.clone();
            Box::pin(async move {
                if claims.roles.contains(&required_role) || claims.roles.contains(&"admin".to_string()) {
                    Ok(next.run(req).await)
                } else {
                    Err(StatusCode::FORBIDDEN)
                }
            })
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
async fn validate_api_key(api_key: &str, state: &AuthState) -> Result<Claims, Box<dyn std::error::Error>> {
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

/// Extractor for authenticated requests
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<AuthErrorResponse>);

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
