//! Metrics middleware implementation.
//!
//! This module provides `MetricsMiddleware` which collects request metrics including:
//!
//! - Request counts (total, by method, success/error)
//! - Request timing (min, max, average)
//! - Active requests
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::middleware::{MetricsMiddleware, Middleware, RequestContext};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let middleware = MetricsMiddleware::new();
//!
//! // Process some requests
//! let mut context = RequestContext::new("initialize".to_string());
//! let request = JsonRpcRequest::new(Some(json!(1)), "initialize".to_string(), None);
//! middleware.on_request(&request, &mut context).await?;
//!
//! let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
//! middleware.on_response(&response, &context).await?;
//!
//! // Get metrics
//! let metrics = middleware.get_metrics();
//! assert_eq!(metrics.total_requests, 1);
//! assert_eq!(metrics.successful_requests, 1);
//! # Ok(())
//! # }
//! ```

use crate::error::MiddlewareError;
use crate::middleware::{Middleware, RequestContext};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Metrics collected by the middleware.
///
/// This struct provides a snapshot of the current metrics.
///
/// # Examples
///
/// ```rust
/// use mcp_server::middleware::{MetricsMiddleware, Middleware, RequestContext};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let middleware = MetricsMiddleware::new();
///
/// // Process request
/// let mut context = RequestContext::new("test".to_string());
/// let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
/// middleware.on_request(&request, &mut context).await?;
/// let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
/// middleware.on_response(&response, &context).await?;
///
/// let metrics = middleware.get_metrics();
/// assert_eq!(metrics.total_requests, 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Metrics {
    /// Total number of requests processed
    pub total_requests: u64,

    /// Number of successful requests
    pub successful_requests: u64,

    /// Number of failed requests
    pub failed_requests: u64,

    /// Number of currently active requests
    pub active_requests: usize,

    /// Total duration of all requests (milliseconds)
    pub total_duration_ms: u64,

    /// Minimum request duration (milliseconds)
    pub min_duration_ms: Option<u64>,

    /// Maximum request duration (milliseconds)
    pub max_duration_ms: Option<u64>,

    /// Average request duration (milliseconds)
    pub avg_duration_ms: Option<f64>,

    /// Request counts by method
    pub requests_by_method: HashMap<String, u64>,
}

impl Metrics {
    /// Create empty metrics.
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            active_requests: 0,
            total_duration_ms: 0,
            min_duration_ms: None,
            max_duration_ms: None,
            avg_duration_ms: None,
            requests_by_method: HashMap::new(),
        }
    }
}

/// Internal metrics storage using atomic operations for thread safety.
#[derive(Debug)]
struct MetricsStorage {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    active_requests: AtomicUsize,
    total_duration_ms: AtomicU64,
    min_duration_ms: RwLock<Option<u64>>,
    max_duration_ms: RwLock<Option<u64>>,
    requests_by_method: RwLock<HashMap<String, u64>>,
}

impl MetricsStorage {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            total_duration_ms: AtomicU64::new(0),
            min_duration_ms: RwLock::new(None),
            max_duration_ms: RwLock::new(None),
            requests_by_method: RwLock::new(HashMap::new()),
        }
    }

    fn increment_active(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_active(&self) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
    }

    fn record_request(&self, method: &str, duration: Duration, success: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        let duration_ms = duration.as_millis() as u64;
        self.total_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        // Update min/max durations
        {
            let mut min = self.min_duration_ms.write().unwrap();
            if min.is_none() || duration_ms < min.unwrap() {
                *min = Some(duration_ms);
            }
        }

        {
            let mut max = self.max_duration_ms.write().unwrap();
            if max.is_none() || duration_ms > max.unwrap() {
                *max = Some(duration_ms);
            }
        }

        // Update method counts
        {
            let mut by_method = self.requests_by_method.write().unwrap();
            *by_method.entry(method.to_string()).or_insert(0) += 1;
        }
    }

    fn snapshot(&self) -> Metrics {
        let total = self.total_requests.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);

        let avg_duration_ms = if total > 0 {
            Some(total_duration as f64 / total as f64)
        } else {
            None
        };

        Metrics {
            total_requests: total,
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            active_requests: self.active_requests.load(Ordering::Relaxed),
            total_duration_ms: total_duration,
            min_duration_ms: *self.min_duration_ms.read().unwrap(),
            max_duration_ms: *self.max_duration_ms.read().unwrap(),
            avg_duration_ms,
            requests_by_method: self.requests_by_method.read().unwrap().clone(),
        }
    }
}

/// Metrics middleware for collecting request statistics.
///
/// This middleware collects various metrics about requests, including counts,
/// timing information, and success/failure rates. All metrics are thread-safe
/// and can be accessed via `get_metrics()`.
///
/// # Examples
///
/// ```rust
/// use mcp_server::middleware::MetricsMiddleware;
/// use mcp_server::McpServer;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let metrics = MetricsMiddleware::new();
///
/// let server = McpServer::builder()
///     .name("my-server")
///     .middleware(metrics.clone())
///     .build()?;
///
/// // Later, get metrics
/// let stats = metrics.get_metrics();
/// println!("Total requests: {}", stats.total_requests);
/// println!("Average duration: {:?}ms", stats.avg_duration_ms);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MetricsMiddleware {
    storage: Arc<MetricsStorage>,
}

impl MetricsMiddleware {
    /// Create a new metrics middleware.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::MetricsMiddleware;
    ///
    /// let middleware = MetricsMiddleware::new();
    /// ```
    pub fn new() -> Self {
        Self {
            storage: Arc::new(MetricsStorage::new()),
        }
    }

    /// Get a snapshot of current metrics.
    ///
    /// Returns a clone of the current metrics state. This is a relatively
    /// inexpensive operation as it only locks for reading.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::{MetricsMiddleware, Middleware, RequestContext};
    /// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let middleware = MetricsMiddleware::new();
    ///
    /// // Process some requests...
    /// let mut context = RequestContext::new("test".to_string());
    /// let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
    /// middleware.on_request(&request, &mut context).await?;
    /// let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
    /// middleware.on_response(&response, &context).await?;
    ///
    /// let metrics = middleware.get_metrics();
    /// println!("Total: {}, Success: {}, Failed: {}",
    ///     metrics.total_requests,
    ///     metrics.successful_requests,
    ///     metrics.failed_requests
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_metrics(&self) -> Metrics {
        self.storage.snapshot()
    }

    /// Reset all metrics to zero.
    ///
    /// This is useful for testing or for periodic metric resets.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::middleware::MetricsMiddleware;
    ///
    /// let middleware = MetricsMiddleware::new();
    /// middleware.reset_metrics();
    ///
    /// let metrics = middleware.get_metrics();
    /// assert_eq!(metrics.total_requests, 0);
    /// ```
    pub fn reset_metrics(&self) {
        self.storage.total_requests.store(0, Ordering::Relaxed);
        self.storage
            .successful_requests
            .store(0, Ordering::Relaxed);
        self.storage.failed_requests.store(0, Ordering::Relaxed);
        self.storage.active_requests.store(0, Ordering::Relaxed);
        self.storage
            .total_duration_ms
            .store(0, Ordering::Relaxed);
        *self.storage.min_duration_ms.write().unwrap() = None;
        *self.storage.max_duration_ms.write().unwrap() = None;
        self.storage.requests_by_method.write().unwrap().clear();
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn on_request(
        &self,
        _request: &JsonRpcRequest,
        _context: &mut RequestContext,
    ) -> Result<(), MiddlewareError> {
        self.storage.increment_active();
        Ok(())
    }

    async fn on_response(
        &self,
        response: &JsonRpcResponse,
        context: &RequestContext,
    ) -> Result<(), MiddlewareError> {
        self.storage.decrement_active();

        if let Some(duration) = context.elapsed() {
            let success = response.is_success();
            self.storage
                .record_request(context.method(), duration, success);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_metrics_middleware_new() {
        let middleware = MetricsMiddleware::new();
        let metrics = middleware.get_metrics();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_middleware_default() {
        let middleware = MetricsMiddleware::default();
        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_increment_active() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        middleware.on_request(&request, &mut context).await.unwrap();

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.active_requests, 1);
    }

    #[tokio::test]
    async fn test_metrics_decrement_active() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_successful_request() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_failed_request() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::method_not_found(Some(json!(1)));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_metrics_duration_tracking() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);

        middleware.on_request(&request, &mut context).await.unwrap();

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let metrics = middleware.get_metrics();
        assert!(metrics.min_duration_ms.is_some());
        assert!(metrics.max_duration_ms.is_some());
        assert!(metrics.avg_duration_ms.is_some());
        assert!(metrics.min_duration_ms.unwrap() >= 10);
    }

    #[tokio::test]
    async fn test_metrics_by_method() {
        let middleware = MetricsMiddleware::new();

        // Request 1: initialize
        let mut context1 = RequestContext::new("initialize".to_string());
        let request1 = JsonRpcRequest::new(Some(json!(1)), "initialize".to_string(), None);
        middleware
            .on_request(&request1, &mut context1)
            .await
            .unwrap();
        let response1 = JsonRpcResponse::success(Some(json!(1)), json!({}));
        middleware
            .on_response(&response1, &context1)
            .await
            .unwrap();

        // Request 2: tools/list
        let mut context2 = RequestContext::new("tools/list".to_string());
        let request2 = JsonRpcRequest::new(Some(json!(2)), "tools/list".to_string(), None);
        middleware
            .on_request(&request2, &mut context2)
            .await
            .unwrap();
        let response2 = JsonRpcResponse::success(Some(json!(2)), json!({}));
        middleware
            .on_response(&response2, &context2)
            .await
            .unwrap();

        // Request 3: initialize again
        let mut context3 = RequestContext::new("initialize".to_string());
        let request3 = JsonRpcRequest::new(Some(json!(3)), "initialize".to_string(), None);
        middleware
            .on_request(&request3, &mut context3)
            .await
            .unwrap();
        let response3 = JsonRpcResponse::success(Some(json!(3)), json!({}));
        middleware
            .on_response(&response3, &context3)
            .await
            .unwrap();

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 3);
        assert_eq!(*metrics.requests_by_method.get("initialize").unwrap(), 2);
        assert_eq!(*metrics.requests_by_method.get("tools/list").unwrap(), 1);
    }

    #[tokio::test]
    async fn test_metrics_reset() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let metrics_before = middleware.get_metrics();
        assert_eq!(metrics_before.total_requests, 1);

        middleware.reset_metrics();

        let metrics_after = middleware.get_metrics();
        assert_eq!(metrics_after.total_requests, 0);
        assert_eq!(metrics_after.successful_requests, 0);
        assert_eq!(metrics_after.failed_requests, 0);
        assert_eq!(metrics_after.active_requests, 0);
        assert!(metrics_after.min_duration_ms.is_none());
        assert!(metrics_after.max_duration_ms.is_none());
        assert!(metrics_after.requests_by_method.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_multiple_requests() {
        let middleware = MetricsMiddleware::new();

        for i in 1..=5 {
            let mut context = RequestContext::new("test".to_string());
            let request = JsonRpcRequest::new(Some(json!(i)), "test".to_string(), None);
            middleware
                .on_request(&request, &mut context)
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_millis(5)).await;

            let response = if i % 2 == 0 {
                JsonRpcResponse::success(Some(json!(i)), json!({}))
            } else {
                JsonRpcResponse::method_not_found(Some(json!(i)))
            };

            middleware
                .on_response(&response, &context)
                .await
                .unwrap();
        }

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 5);
        assert_eq!(metrics.successful_requests, 2);
        assert_eq!(metrics.failed_requests, 3);
        assert_eq!(metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_concurrent_requests() {
        let middleware = MetricsMiddleware::new();
        let middleware_clone = middleware.clone();

        let mut handles = vec![];

        for i in 1..=10 {
            let mw = middleware_clone.clone();
            let handle = tokio::spawn(async move {
                let mut context = RequestContext::new("test".to_string());
                let request = JsonRpcRequest::new(Some(json!(i)), "test".to_string(), None);
                mw.on_request(&request, &mut context).await.unwrap();

                tokio::time::sleep(Duration::from_millis(5)).await;

                let response = JsonRpcResponse::success(Some(json!(i)), json!({}));
                mw.on_response(&response, &context).await.unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let metrics = middleware.get_metrics();
        assert_eq!(metrics.total_requests, 10);
        assert_eq!(metrics.successful_requests, 10);
        assert_eq!(metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn test_metrics_clone() {
        let middleware = MetricsMiddleware::new();
        let mut context = RequestContext::new("test".to_string());
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));

        middleware.on_request(&request, &mut context).await.unwrap();
        middleware
            .on_response(&response, &context)
            .await
            .unwrap();

        let cloned = middleware.clone();
        let metrics = cloned.get_metrics();
        assert_eq!(metrics.total_requests, 1);
    }

    #[tokio::test]
    async fn test_metrics_debug() {
        let middleware = MetricsMiddleware::new();
        let debug_str = format!("{:?}", middleware);
        assert!(debug_str.contains("MetricsMiddleware"));
    }

    #[tokio::test]
    async fn test_metrics_struct_debug() {
        let metrics = Metrics::new();
        let debug_str = format!("{:?}", metrics);
        assert!(debug_str.contains("Metrics"));
    }

    #[tokio::test]
    async fn test_metrics_struct_clone() {
        let metrics = Metrics::new();
        let _cloned = metrics.clone();
    }

    #[tokio::test]
    async fn test_metrics_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MetricsMiddleware>();
    }

    #[tokio::test]
    async fn test_metrics_avg_duration() {
        let middleware = MetricsMiddleware::new();

        for i in 1..=3 {
            let mut context = RequestContext::new("test".to_string());
            let request = JsonRpcRequest::new(Some(json!(i)), "test".to_string(), None);
            middleware
                .on_request(&request, &mut context)
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_millis(10)).await;

            let response = JsonRpcResponse::success(Some(json!(i)), json!({}));
            middleware
                .on_response(&response, &context)
                .await
                .unwrap();
        }

        let metrics = middleware.get_metrics();
        assert!(metrics.avg_duration_ms.is_some());
        assert!(metrics.avg_duration_ms.unwrap() >= 10.0);
    }
}
