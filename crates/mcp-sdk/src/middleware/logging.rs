//! Logging middleware implementation.
//!
//! This module provides `LoggingMiddleware` which logs all requests and responses
//! using the `tracing` crate. It logs:
//!
//! - Incoming requests (method, id)
//! - Outgoing responses (success/error, duration)
//! - Request timing information
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::middleware::LoggingMiddleware;
//! use mcp_server::McpServer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .middleware(LoggingMiddleware::new())
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use crate::error::MiddlewareError;
use crate::middleware::{Middleware, RequestContext};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;

/// Logging middleware using the `tracing` crate.
///
/// This middleware logs all requests and responses, including timing information.
/// It uses the `tracing` crate for structured logging, which can be configured
/// with different subscribers for different output formats.
///
/// # Log Levels
///
/// - **INFO**: Successful requests
/// - **WARN**: Failed requests (errors)
/// - **DEBUG**: Detailed request/response information
///
/// # Examples
///
/// ```rust
/// use mcp_server::middleware::{LoggingMiddleware, Middleware, RequestContext};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Initialize tracing subscriber (usually done once at app start)
/// tracing_subscriber::fmt::init();
///
/// let middleware = LoggingMiddleware::new();
/// let mut context = RequestContext::new("initialize".to_string());
/// let request = JsonRpcRequest::new(
///     Some(json!(1)),
///     "initialize".to_string(),
///     None
/// );
///
/// middleware.on_request(&request, &mut context).await?;
/// // ... request processing ...
/// let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
/// middleware.on_response(&response, &context).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    /// Create a new logging middleware.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::LoggingMiddleware;
    ///
    /// let middleware = LoggingMiddleware::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        let id_str = request
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "notification".to_string());

        tracing::info!(
            method = %request.method,
            id = %id_str,
            "Incoming request"
        );

        tracing::debug!(
            method = %request.method,
            id = %id_str,
            params = ?request.params,
            "Request details"
        );

        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        let id_str = response
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_else(|| "notification".to_string());

        let elapsed = context
            .elapsed()
            .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "unknown".to_string());

        if response.is_error() {
            let error = response.error.as_ref().unwrap();
            tracing::warn!(
                method = %context.method(),
                id = %id_str,
                error_code = error.code,
                error_message = %error.message,
                duration = %elapsed,
                "Request failed"
            );
        } else {
            tracing::info!(
                method = %context.method(),
                id = %id_str,
                duration = %elapsed,
                "Request succeeded"
            );
        }

        tracing::debug!(
            method = %context.method(),
            id = %id_str,
            duration = %elapsed,
            result = ?response.result,
            error = ?response.error,
            "Response details"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tracing_subscriber;

    #[tokio::test]
    async fn test_logging_middleware_request() {
        // Initialize test subscriber
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let mut context = RequestContext::new("initialize".to_string());
        let request = JsonRpcRequest::new(
            Some(json!(1)),
            "initialize".to_string(),
            Some(json!({"protocolVersion": "2025-03-26"})),
        );

        let result = middleware.on_request(&request, &mut context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware_response_success() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let context = RequestContext::new("initialize".to_string());
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        let result = middleware.on_response(&response, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware_response_error() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let context = RequestContext::new("tools/call".to_string());
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));

        let result = middleware.on_response(&response, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware_notification() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let mut context = RequestContext::new("notifications/message".to_string());
        let request = JsonRpcRequest::notification(
            "notifications/message".to_string(),
            Some(json!({"message": "test"})),
        );

        let result = middleware.on_request(&request, &mut context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware_new() {
        let middleware = LoggingMiddleware::new();
        assert!(std::mem::size_of_val(&middleware) == 0); // ZST
    }

    #[tokio::test]
    async fn test_logging_middleware_default() {
        let middleware = LoggingMiddleware::default();
        assert!(std::mem::size_of_val(&middleware) == 0); // ZST
    }

    #[tokio::test]
    async fn test_logging_middleware_clone() {
        let middleware = LoggingMiddleware::new();
        let _cloned = middleware.clone();
    }

    #[tokio::test]
    async fn test_logging_middleware_debug() {
        let middleware = LoggingMiddleware::new();
        let debug_str = format!("{:?}", middleware);
        assert!(debug_str.contains("LoggingMiddleware"));
    }

    #[tokio::test]
    async fn test_logging_middleware_with_timing() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let context = RequestContext::new("test".to_string());

        // Wait a bit to ensure elapsed time is non-zero
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        let result = middleware.on_response(&response, &context).await;
        assert!(result.is_ok());
        assert!(context.elapsed().unwrap().as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_logging_middleware_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LoggingMiddleware>();
    }

    #[tokio::test]
    async fn test_logging_middleware_full_lifecycle() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let mut context = RequestContext::new("tools/call".to_string());

        // Request phase
        let request = JsonRpcRequest::new(
            Some(json!(42)),
            "tools/call".to_string(),
            Some(json!({"name": "echo", "arguments": {"message": "hello"}})),
        );
        middleware.on_request(&request, &mut context).await.unwrap();

        // Response phase
        let response = JsonRpcResponse::success(
            Some(json!(42)),
            json!({"content": [{"type": "text", "text": "hello"}]}),
        );
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_logging_middleware_with_various_id_types() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();

        // Number ID
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(123)), "test".to_string(), None);
        middleware.on_request(&request, &mut context).await.unwrap();

        // String ID
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!("req-456")), "test".to_string(), None);
        middleware.on_request(&request, &mut context).await.unwrap();

        // No ID (notification)
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::notification("test".to_string(), None);
        middleware.on_request(&request, &mut context).await.unwrap();
    }

    #[tokio::test]
    async fn test_logging_middleware_with_complex_error() {
        let _ = tracing_subscriber::fmt().try_init();

        let middleware = LoggingMiddleware::new();
        let context = RequestContext::new("tools/call".to_string());

        let response = JsonRpcResponse::invalid_params(Some(json!(1)), "Missing required field");

        middleware
            .on_response(&response, &context)
            .await
            .unwrap();
    }
}
