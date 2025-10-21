//! Middleware trait definition.
//!
//! This module defines the core `Middleware` trait that all middleware implementations must implement.
//!
//! # Trait Overview
//!
//! The `Middleware` trait provides two hooks:
//! - `on_request`: Called before a request is processed
//! - `on_response`: Called after a response is generated
//!
//! Both hooks are async and can:
//! - Inspect the request/response
//! - Modify the request context
//! - Return errors to halt processing
//!
//! # Examples
//!
//! ## Simple Logging Middleware
//!
//! ```rust
//! use mcp_server::middleware::{Middleware, RequestContext};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct SimpleLogger;
//!
//! #[async_trait]
//! impl Middleware for SimpleLogger {
//!     async fn on_request(
//!         &self,
//!         request: &JsonRpcRequest,
//!         _context: &mut RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         println!("Request: {}", request.method);
//!         Ok(())
//!     }
//!
//!     async fn on_response(
//!         &self,
//!         response: &JsonRpcResponse,
//!         _context: &RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         println!("Response: success={}", response.is_success());
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Authentication Middleware
//!
//! ```rust
//! use mcp_server::middleware::{Middleware, RequestContext};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct AuthMiddleware {
//!     api_key: String,
//! }
//!
//! #[async_trait]
//! impl Middleware for AuthMiddleware {
//!     async fn on_request(
//!         &self,
//!         request: &JsonRpcRequest,
//!         context: &mut RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         // Check for API key in metadata
//!         if let Some(key) = context.get_metadata("api_key") {
//!             if key.as_str() == Some(&self.api_key) {
//!                 return Ok(());
//!             }
//!         }
//!         Err(MiddlewareError::Blocked("Invalid API key".to_string()))
//!     }
//!
//!     async fn on_response(
//!         &self,
//!         _response: &JsonRpcResponse,
//!         _context: &RequestContext,
//!     ) -> Result<(), MiddlewareError> {
//!         Ok(())
//!     }
//! }
//! ```

use crate::error::MiddlewareError;
use crate::middleware::RequestContext;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;

/// Middleware trait for intercepting requests and responses.
///
/// Middleware is executed in a chain, with each middleware having the opportunity to:
/// - Inspect and modify the request context
/// - Block request processing by returning an error
/// - Inspect responses before they're sent
/// - Add timing, logging, metrics, etc.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to work with the async runtime.
///
/// # Error Handling
///
/// If a middleware returns an error from `on_request`, the request processing is halted
/// and an error response is sent to the client. The `on_response` phase will not be executed
/// for the failed request.
///
/// Errors from `on_response` are logged but don't affect the response sent to the client.
///
/// # Examples
///
/// ```rust
/// use mcp_server::middleware::{Middleware, RequestContext};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use mcp_server::error::MiddlewareError;
/// use async_trait::async_trait;
///
/// struct RateLimitMiddleware {
///     max_requests_per_second: u32,
/// }
///
/// #[async_trait]
/// impl Middleware for RateLimitMiddleware {
///     async fn on_request(
///         &self,
///         request: &JsonRpcRequest,
///         context: &mut RequestContext,
///     ) -> Result<(), MiddlewareError> {
///         // Check rate limit logic here
///         // For this example, always allow
///         Ok(())
///     }
///
///     async fn on_response(
///         &self,
///         response: &JsonRpcResponse,
///         context: &RequestContext,
///     ) -> Result<(), MiddlewareError> {
///         // Could update rate limit counters here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Called before a request is processed.
    ///
    /// This method is invoked before the request is routed to a handler. Middleware can:
    /// - Inspect the request
    /// - Modify the context (add metadata, start timing, etc.)
    /// - Block the request by returning an error
    ///
    /// # Arguments
    ///
    /// * `request` - The incoming JSON-RPC request
    /// * `context` - Mutable request context for sharing data
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Allow the request to proceed
    /// * `Err(MiddlewareError)` - Block the request and return an error
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::{Middleware, RequestContext};
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    /// use serde_json::json;
    ///
    /// struct MetadataMiddleware;
    ///
    /// #[async_trait]
    /// impl Middleware for MetadataMiddleware {
    ///     async fn on_request(
    ///         &self,
    ///         request: &JsonRpcRequest,
    ///         context: &mut RequestContext,
    ///     ) -> Result<(), MiddlewareError> {
    ///         // Store request method in context
    ///         context.set_metadata("method".to_string(), json!(request.method));
    ///         Ok(())
    ///     }
    ///
    ///     async fn on_response(
    ///         &self,
    ///         _response: &mcp_server::protocol::JsonRpcResponse,
    ///         _context: &RequestContext,
    ///     ) -> Result<(), MiddlewareError> {
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError>;

    /// Called after a response is generated.
    ///
    /// This method is invoked after the handler has processed the request and generated a response.
    /// Middleware can:
    /// - Inspect the response
    /// - Read context data (timing, metadata, etc.)
    /// - Log, record metrics, etc.
    ///
    /// Note: Errors from this method are logged but don't affect the response sent to the client.
    ///
    /// # Arguments
    ///
    /// * `response` - The generated JSON-RPC response
    /// * `context` - Immutable request context with accumulated data
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Response processing succeeded
    /// * `Err(MiddlewareError)` - Error occurred (logged but doesn't block response)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::{Middleware, RequestContext};
    /// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct TimingMiddleware;
    ///
    /// #[async_trait]
    /// impl Middleware for TimingMiddleware {
    ///     async fn on_request(
    ///         &self,
    ///         _request: &JsonRpcRequest,
    ///         context: &mut RequestContext,
    ///     ) -> Result<(), MiddlewareError> {
    ///         // Start timing is automatic in RequestContext
    ///         Ok(())
    ///     }
    ///
    ///     async fn on_response(
    ///         &self,
    ///         response: &JsonRpcResponse,
    ///         context: &RequestContext,
    ///     ) -> Result<(), MiddlewareError> {
    ///         if let Some(duration) = context.elapsed() {
    ///             println!("Request took: {:?}", duration);
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test middleware implementations
    struct PassthroughMiddleware;

    #[async_trait]
    impl Middleware for PassthroughMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            Ok(())
        }

        async fn on_response(
            &self,
            _response: &JsonRpcResponse,
            _context: &RequestContext,
        ) -> Result<(), MiddlewareError> {
            Ok(())
        }
    }

    struct BlockingMiddleware {
        should_block: bool,
    }

    #[async_trait]
    impl Middleware for BlockingMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            if self.should_block {
                Err(MiddlewareError::Blocked("Access denied".to_string()))
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
    async fn test_passthrough_middleware() {
        let middleware = PassthroughMiddleware;
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));

        // Should not error
        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_blocking_middleware_allows() {
        let middleware = BlockingMiddleware {
            should_block: false,
        };
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        let result = middleware.on_request(&request, &mut context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_blocking_middleware_blocks() {
        let middleware = BlockingMiddleware { should_block: true };
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        let result = middleware.on_request(&request, &mut context).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MiddlewareError::Blocked(msg) => assert_eq!(msg, "Access denied"),
            _ => panic!("Expected Blocked error"),
        }
    }

    #[tokio::test]
    async fn test_middleware_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PassthroughMiddleware>();
        assert_send_sync::<BlockingMiddleware>();
    }

    #[tokio::test]
    async fn test_middleware_can_be_boxed() {
        let middleware: Box<dyn Middleware> = Box::new(PassthroughMiddleware);
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        middleware.on_request(&request, &mut context).await.unwrap();
    }
}
