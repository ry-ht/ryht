//! API error handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// API error type
#[derive(Debug)]
pub enum ApiError {
    /// Resource not found
    NotFound(String),
    /// Bad request
    BadRequest(String),
    /// Internal server error
    Internal(String),
    /// Unauthorized
    Unauthorized(String),
    /// Forbidden
    Forbidden(String),
    /// Conflict
    Conflict(String),
    /// Unprocessable entity
    UnprocessableEntity(String),
    /// Version conflict (optimistic locking failure)
    VersionConflict { expected: u64, current: u64, path: String, details: Option<serde_json::Value> },
    /// Payload too large
    PayloadTooLarge { size: u64, max_size: u64, details: Option<String> },
    /// Insufficient storage
    InsufficientStorage { used: u64, quota: u64, requested: u64, details: Option<String> },
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::UnprocessableEntity(msg) => write!(f, "Unprocessable entity: {}", msg),
            ApiError::VersionConflict { expected, current, path, .. } =>
                write!(f, "Version conflict for {}: expected {}, current {}", path, expected, current),
            ApiError::PayloadTooLarge { size, max_size, .. } =>
                write!(f, "Payload too large: {} bytes exceeds maximum of {} bytes", size, max_size),
            ApiError::InsufficientStorage { used, quota, requested, .. } =>
                write!(f, "Insufficient storage: {}/{} bytes used, {} bytes requested", used, quota, requested),
        }
    }
}

impl std::error::Error for ApiError {}

/// Error response body
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
    pub metadata: crate::api::types::ApiMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::VersionConflict { .. } => StatusCode::CONFLICT,
            ApiError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::InsufficientStorage { .. } => StatusCode::INSUFFICIENT_STORAGE,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Internal(_) => "INTERNAL_ERROR",
            ApiError::Unauthorized(_) => "UNAUTHORIZED",
            ApiError::Forbidden(_) => "FORBIDDEN",
            ApiError::Conflict(_) => "CONFLICT",
            ApiError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            ApiError::VersionConflict { .. } => "VERSION_CONFLICT",
            ApiError::PayloadTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            ApiError::InsufficientStorage { .. } => "INSUFFICIENT_STORAGE",
        }
    }

    pub fn details(&self) -> Option<serde_json::Value> {
        match self {
            ApiError::VersionConflict { expected, current, path, details } => {
                let mut map = serde_json::Map::new();
                map.insert("expected_version".to_string(), serde_json::json!(expected));
                map.insert("current_version".to_string(), serde_json::json!(current));
                map.insert("path".to_string(), serde_json::json!(path));
                if let Some(d) = details {
                    map.insert("additional_details".to_string(), d.clone());
                }
                Some(serde_json::Value::Object(map))
            }
            ApiError::PayloadTooLarge { size, max_size, details } => {
                let mut map = serde_json::Map::new();
                map.insert("size".to_string(), serde_json::json!(size));
                map.insert("max_size".to_string(), serde_json::json!(max_size));
                if let Some(d) = details {
                    map.insert("message".to_string(), serde_json::json!(d));
                }
                Some(serde_json::Value::Object(map))
            }
            ApiError::InsufficientStorage { used, quota, requested, details } => {
                let mut map = serde_json::Map::new();
                map.insert("used".to_string(), serde_json::json!(used));
                map.insert("quota".to_string(), serde_json::json!(quota));
                map.insert("requested".to_string(), serde_json::json!(requested));
                if let Some(d) = details {
                    map.insert("message".to_string(), serde_json::json!(d));
                }
                Some(serde_json::Value::Object(map))
            }
            _ => None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_message = self.to_string();
        let error_code = self.error_code().to_string();
        let details = self.details();

        let metadata = crate::api::types::ApiMetadata {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            version: "v1".to_string(),
            duration_ms: 0,
        };

        let body = ErrorResponse {
            success: false,
            error: ErrorDetail {
                code: error_code,
                message: error_message,
                details,
            },
            metadata,
        };

        (status, Json(body)).into_response()
    }
}

// Conversion from anyhow::Error
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

// Conversion from cortex_core::error::CortexError
impl From<cortex_core::error::CortexError> for ApiError {
    fn from(err: cortex_core::error::CortexError) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;
