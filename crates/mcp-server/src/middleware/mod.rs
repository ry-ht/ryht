//! Middleware system for request/response processing.
//!
//! This module provides a flexible middleware system that allows intercepting and processing
//! JSON-RPC requests and responses before they are handled by the server or sent to the client.
//!
//! # Architecture
//!
//! Middleware is executed in a chain:
//!
//! ```text
//! Request → Middleware 1 → Middleware 2 → ... → Handler → Response
//!           ↓                ↓                              ↑
//!           on_request       on_request                     on_response
//!                                                           ↓
//!                                                    Middleware 2
//!                                                           ↓
//!                                                    Middleware 1
//! ```
//!
//! # Features
//!
//! - **Request Interception**: Modify or inspect requests before processing
//! - **Response Interception**: Modify or inspect responses before sending
//! - **Context Sharing**: Pass data between request and response phases
//! - **Error Handling**: Graceful error handling with detailed error types
//! - **Timing Support**: Built-in request timing capabilities
//! - **Metadata Storage**: Store arbitrary metadata for request lifecycle
//!
//! # Examples
//!
//! ## Basic Middleware
//!
//! ```rust
//! use mcp_server::middleware::{Middleware, RequestContext};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct CustomMiddleware;
//!
//! #[async_trait]
//! impl Middleware for CustomMiddleware {
//!     async fn on_request(
//!         &self,
//!         request: &JsonRpcRequest,
//!         context: &mut RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         println!("Received request: {}", request.method);
//!         Ok(())
//!     }
//!
//!     async fn on_response(
//!         &self,
//!         response: &JsonRpcResponse,
//!         context: &RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         if let Some(duration) = context.elapsed() {
//!             println!("Request took: {:?}", duration);
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Using Built-in Middleware
//!
//! ```rust
//! use mcp_server::middleware::{LoggingMiddleware, MetricsMiddleware};
//! use mcp_server::McpServer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .middleware(LoggingMiddleware::new())
//!     .middleware(MetricsMiddleware::new())
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod context;
pub mod logging;
pub mod metrics;
pub mod traits;

pub use context::RequestContext;
pub use logging::LoggingMiddleware;
pub use metrics::MetricsMiddleware;
pub use traits::Middleware;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MiddlewareError;
    use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
    use async_trait::async_trait;
    use serde_json::json;

    // Test middleware that tracks execution
    struct TestMiddleware {
        request_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        response_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl TestMiddleware {
        fn new() -> Self {
            Self {
                request_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                response_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }
        }

        fn request_count(&self) -> usize {
            self.request_count
                .load(std::sync::atomic::Ordering::SeqCst)
        }

        fn response_count(&self) -> usize {
            self.response_count
                .load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Middleware for TestMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            self.request_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn on_response(
            &self,
            _response: &JsonRpcResponse,
            _context: &RequestContext,
        ) -> Result<(), MiddlewareError> {
            self.response_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_execution() {
        let middleware = TestMiddleware::new();
        let mut context = RequestContext::new("test_method".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        // Execute request phase
        middleware.on_request(&request, &mut context).await.unwrap();
        assert_eq!(middleware.request_count(), 1);

        // Execute response phase
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();
        assert_eq!(middleware.response_count(), 1);
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        let mw1 = TestMiddleware::new();
        let mw2 = TestMiddleware::new();

        let mut context = RequestContext::new("test_method".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        // Execute chain
        mw1.on_request(&request, &mut context).await.unwrap();
        mw2.on_request(&request, &mut context).await.unwrap();

        assert_eq!(mw1.request_count(), 1);
        assert_eq!(mw2.request_count(), 1);

        mw1.on_response(&response, &context).await.unwrap();
        mw2.on_response(&response, &context).await.unwrap();

        assert_eq!(mw1.response_count(), 1);
        assert_eq!(mw2.response_count(), 1);
    }

    // Test middleware that fails
    struct FailingMiddleware {
        fail_on_request: bool,
    }

    #[async_trait]
    impl Middleware for FailingMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            if self.fail_on_request {
                Err(MiddlewareError::Blocked("Request blocked".to_string()))
            } else {
                Ok(())
            }
        }

        async fn on_response(
            &self,
            _response: &JsonRpcResponse,
            _context: &RequestContext,
        ) -> Result<(), MiddlewareError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_middleware_error_handling() {
        let middleware = FailingMiddleware {
            fail_on_request: true,
        };
        let mut context = RequestContext::new("test_method".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        let result = middleware.on_request(&request, &mut context).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MiddlewareError::Blocked(msg) => assert_eq!(msg, "Request blocked"),
            _ => panic!("Expected Blocked error"),
        }
    }

    #[tokio::test]
    async fn test_middleware_context_sharing() {
        struct ContextMiddleware;

        #[async_trait]
        impl Middleware for ContextMiddleware {
            async fn on_request(
                &self,
                _request: &JsonRpcRequest,
                context: &mut RequestContext,
            ) -> Result<(), MiddlewareError> {
                context.set_metadata("key".to_string(), json!("value"));
                Ok(())
            }

            async fn on_response(
                &self,
                _response: &JsonRpcResponse,
                context: &RequestContext,
            ) -> Result<(), MiddlewareError> {
                let value = context.get_metadata("key");
                assert_eq!(value, Some(&json!("value")));
                Ok(())
            }
        }

        let middleware = ContextMiddleware;
        let mut context = RequestContext::new("test_method".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();
    }
}
