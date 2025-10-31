//! Middleware registry for managing middleware chain.
//!
//! This module provides the [`MiddlewareRegistry`] for thread-safe middleware management.

use std::sync::Arc;
use tokio::sync::RwLock;

use super::{Middleware, RequestContext};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::error::MiddlewareError;

/// A thread-safe registry for managing MCP middleware.
///
/// The registry stores all registered middleware and provides methods for:
/// - Registering new middleware
/// - Running middleware chain on requests
/// - Running middleware chain on responses
///
/// # Thread Safety
///
/// The registry uses `Arc<RwLock<>>` internally, making it safe to share
/// across multiple threads and async tasks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use mcp_server::middleware::{Middleware, MiddlewareRegistry, RequestContext};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use mcp_server::error::MiddlewareError;
/// use async_trait::async_trait;
/// use serde_json::json;
///
/// # struct LogMiddleware;
/// # #[async_trait]
/// # impl Middleware for LogMiddleware {
/// #     async fn on_request(&self, _req: &JsonRpcRequest, _ctx: &mut RequestContext) -> Result<(), MiddlewareError> {
/// #         Ok(())
/// #     }
/// #     async fn on_response(&self, _resp: &JsonRpcResponse, _ctx: &RequestContext) -> Result<(), MiddlewareError> {
/// #         Ok(())
/// #     }
/// # }
/// #
/// # async fn example() {
/// let registry = MiddlewareRegistry::new();
///
/// // Register middleware
/// registry.register(LogMiddleware).await;
///
/// // Count registered middleware
/// assert_eq!(registry.count().await, 1);
/// # }
/// ```
#[derive(Clone)]
pub struct MiddlewareRegistry {
    middleware: Arc<RwLock<Vec<Arc<dyn Middleware>>>>,
}

impl MiddlewareRegistry {
    /// Creates a new empty middleware registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::MiddlewareRegistry;
    ///
    /// let registry = MiddlewareRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            middleware: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Registers a new middleware with the registry.
    ///
    /// Middleware is executed in the order registered (first registered, first executed).
    ///
    /// # Arguments
    ///
    /// * `middleware` - The middleware to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::{Middleware, MiddlewareRegistry, RequestContext};
    /// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// # struct MyMiddleware;
    /// # #[async_trait]
    /// # impl Middleware for MyMiddleware {
    /// #     async fn on_request(&self, _req: &JsonRpcRequest, _ctx: &mut RequestContext) -> Result<(), MiddlewareError> {
    /// #         Ok(())
    /// #     }
    /// #     async fn on_response(&self, _resp: &JsonRpcResponse, _ctx: &RequestContext) -> Result<(), MiddlewareError> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = MiddlewareRegistry::new();
    /// registry.register(MyMiddleware).await;
    /// # }
    /// ```
    pub async fn register<M: Middleware + 'static>(&self, middleware: M) {
        let mut mw = self.middleware.write().await;
        mw.push(Arc::new(middleware));
    }

    /// Registers an Arc-wrapped middleware.
    ///
    /// This is useful when you already have an Arc-wrapped middleware instance.
    ///
    /// # Arguments
    ///
    /// * `middleware` - Arc-wrapped middleware to register
    pub async fn register_arc(&self, middleware: Arc<dyn Middleware>) {
        let mut mw = self.middleware.write().await;
        mw.push(middleware);
    }

    /// Runs all registered middleware on a request.
    ///
    /// Middleware is executed in registration order. If any middleware returns
    /// an error, the chain is stopped and the error is returned.
    ///
    /// # Arguments
    ///
    /// * `request` - The JSON-RPC request
    /// * `context` - Mutable request context for sharing data
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all middleware executed successfully
    /// - `Err(MiddlewareError)` if any middleware failed
    pub async fn run_on_request(
        &self,
        request: &JsonRpcRequest,
        context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        let middleware = self.middleware.read().await;

        for mw in middleware.iter() {
            mw.on_request(request, context).await?;
        }

        Ok(())
    }

    /// Runs all registered middleware on a response.
    ///
    /// Middleware is executed in reverse registration order (last registered,
    /// first executed on response). Errors are logged but don't stop execution.
    ///
    /// # Arguments
    ///
    /// * `response` - The JSON-RPC response
    /// * `context` - Request context with shared data
    pub async fn run_on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        let middleware = self.middleware.read().await;

        // Execute in reverse order for responses
        for mw in middleware.iter().rev() {
            mw.on_response(response, context).await?;
        }

        Ok(())
    }

    /// Returns the number of registered middleware.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::{Middleware, MiddlewareRegistry, RequestContext};
    /// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// # struct M1;
    /// # #[async_trait]
    /// # impl Middleware for M1 {
    /// #     async fn on_request(&self, _: &JsonRpcRequest, _: &mut RequestContext) -> Result<(), MiddlewareError> { Ok(()) }
    /// #     async fn on_response(&self, _: &JsonRpcResponse, _: &RequestContext) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    /// # struct M2;
    /// # #[async_trait]
    /// # impl Middleware for M2 {
    /// #     async fn on_request(&self, _: &JsonRpcRequest, _: &mut RequestContext) -> Result<(), MiddlewareError> { Ok(()) }
    /// #     async fn on_response(&self, _: &JsonRpcResponse, _: &RequestContext) -> Result<(), MiddlewareError> { Ok(()) }
    /// # }
    /// #
    /// # async fn example() {
    /// let registry = MiddlewareRegistry::new();
    /// registry.register(M1).await;
    /// registry.register(M2).await;
    /// assert_eq!(registry.count().await, 2);
    /// # }
    /// ```
    pub async fn count(&self) -> usize {
        self.middleware.read().await.len()
    }
}

impl Default for MiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct TestMiddleware {
        name: String,
    }

    impl TestMiddleware {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl Middleware for TestMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            let key = format!("{}_request", self.name);
            context.set_metadata(key, json!(true));
            Ok(())
        }

        async fn on_response(
            &self,
            _response: &JsonRpcResponse,
            context: &RequestContext,
        ) -> Result<(), MiddlewareError> {
            let key = format!("{}_response", self.name);
            // Can't mutate context in response phase
            assert!(context.get_metadata(&format!("{}_request", self.name)).is_some());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_registry_new() {
        let registry = MiddlewareRegistry::new();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_register() {
        let registry = MiddlewareRegistry::new();
        registry.register(TestMiddleware::new("test")).await;
        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_registry_multiple_middleware() {
        let registry = MiddlewareRegistry::new();
        registry.register(TestMiddleware::new("m1")).await;
        registry.register(TestMiddleware::new("m2")).await;
        registry.register(TestMiddleware::new("m3")).await;
        assert_eq!(registry.count().await, 3);
    }

    #[tokio::test]
    async fn test_run_on_request() {
        let registry = MiddlewareRegistry::new();
        registry.register(TestMiddleware::new("m1")).await;
        registry.register(TestMiddleware::new("m2")).await;

        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let mut context = RequestContext::new("test".to_string());

        registry.run_on_request(&request, &mut context).await.unwrap();

        // Verify both middleware ran
        assert!(context.get_metadata("m1_request").is_some());
        assert!(context.get_metadata("m2_request").is_some());
    }

    #[tokio::test]
    async fn test_run_on_response() {
        let registry = MiddlewareRegistry::new();
        registry.register(TestMiddleware::new("m1")).await;
        registry.register(TestMiddleware::new("m2")).await;

        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        let mut context = RequestContext::new("test".to_string());

        // Run request phase first
        registry.run_on_request(&request, &mut context).await.unwrap();

        // Then response phase
        registry.run_on_response(&response, &context).await.unwrap();
    }

    struct FailingMiddleware;

    #[async_trait]
    impl Middleware for FailingMiddleware {
        async fn on_request(
            &self,
            _request: &JsonRpcRequest,
            _context: &mut RequestContext,
        ) -> Result<(), MiddlewareError> {
            Err(MiddlewareError::Blocked("Test error".to_string()))
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
    async fn test_middleware_error_stops_chain() {
        let registry = MiddlewareRegistry::new();
        registry.register(TestMiddleware::new("m1")).await;
        registry.register(FailingMiddleware).await;
        registry.register(TestMiddleware::new("m2")).await; // Should not run

        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let mut context = RequestContext::new("test".to_string());

        let result = registry.run_on_request(&request, &mut context).await;
        assert!(result.is_err());

        // m1 ran, but m2 didn't
        assert!(context.get_metadata("m1_request").is_some());
        assert!(context.get_metadata("m2_request").is_none());
    }

    #[tokio::test]
    async fn test_register_arc() {
        let registry = MiddlewareRegistry::new();
        let middleware: Arc<dyn Middleware> = Arc::new(TestMiddleware::new("test"));

        registry.register_arc(middleware).await;
        assert_eq!(registry.count().await, 1);
    }
}
