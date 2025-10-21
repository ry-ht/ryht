//! Mock Transport for Testing
//!
//! This module provides a mock transport implementation that can be used for testing
//! MCP servers without requiring actual I/O operations.
//!
//! # Features
//!
//! - Queue requests to be received by the transport
//! - Capture responses sent through the transport
//! - Thread-safe for concurrent testing
//! - Inspect all sent responses after test execution
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::transport::{Transport, MockTransport};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut transport = MockTransport::new();
//!
//!     // Queue a request
//!     let request = JsonRpcRequest::new(
//!         Some(json!(1)),
//!         "test_method".to_string(),
//!         None
//!     );
//!     transport.push_request(request);
//!
//!     // Receive the request
//!     let received = transport.recv().await;
//!     assert!(received.is_some());
//!
//!     // Send a response
//!     let response = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
//!     transport.send(response).await.unwrap();
//!
//!     // Inspect sent responses
//!     let responses = transport.responses();
//!     assert_eq!(responses.len(), 1);
//! }
//! ```

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::error::TransportError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

use super::traits::Transport;

/// Mock transport for testing.
///
/// This transport maintains a queue of requests and a vector of responses,
/// allowing tests to inspect all communication that occurred through the transport.
///
/// # Thread Safety
///
/// All internal state is protected by a `Mutex`, making this transport safe to
/// use across multiple async tasks.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use mcp_server::transport::{Transport, MockTransport};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use serde_json::json;
///
/// #[tokio::main]
/// async fn main() {
///     let mut transport = MockTransport::new();
///
///     // Setup: queue requests
///     transport.push_request(JsonRpcRequest::new(
///         Some(json!(1)),
///         "test".to_string(),
///         None
///     ));
///
///     // Test: receive and respond
///     let request = transport.recv().await.unwrap();
///     let response = JsonRpcResponse::success(request.id, json!({}));
///     transport.send(response).await.unwrap();
///
///     // Verify: check responses
///     assert_eq!(transport.responses().len(), 1);
/// }
/// ```
///
/// ## Testing Server Logic
///
/// ```rust
/// use mcp_server::transport::{Transport, MockTransport};
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use serde_json::json;
///
/// async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
///     JsonRpcResponse::success(request.id, json!({"handled": true}))
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let mut transport = MockTransport::new();
///
///     // Queue test requests
///     transport.push_request(JsonRpcRequest::new(
///         Some(json!(1)),
///         "initialize".to_string(),
///         None
///     ));
///
///     // Process requests
///     while let Some(request) = transport.recv().await {
///         let response = handle_request(request).await;
///         transport.send(response).await.unwrap();
///     }
///
///     // Verify all responses
///     let responses = transport.responses();
///     assert_eq!(responses.len(), 1);
///     assert!(responses[0].is_success());
/// }
/// ```
#[derive(Clone)]
pub struct MockTransport {
    state: Arc<Mutex<MockTransportState>>,
}

struct MockTransportState {
    requests: VecDeque<JsonRpcRequest>,
    responses: Vec<JsonRpcResponse>,
    closed: bool,
}

impl MockTransport {
    /// Create a new mock transport.
    ///
    /// The transport starts with empty request queue and response history.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::MockTransport;
    ///
    /// let transport = MockTransport::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockTransportState {
                requests: VecDeque::new(),
                responses: Vec::new(),
                closed: false,
            })),
        }
    }

    /// Queue a request to be received by `recv()`.
    ///
    /// Requests are processed in FIFO order.
    ///
    /// # Arguments
    ///
    /// * `request` - The JSON-RPC request to queue
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::{Transport, MockTransport};
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = MockTransport::new();
    ///
    ///     transport.push_request(JsonRpcRequest::new(
    ///         Some(json!(1)),
    ///         "test".to_string(),
    ///         None
    ///     ));
    ///
    ///     let received = transport.recv().await;
    ///     assert!(received.is_some());
    /// }
    /// ```
    pub fn push_request(&self, request: JsonRpcRequest) {
        let mut state = self.state.lock();
        state.requests.push_back(request);
    }

    /// Get all responses that have been sent through the transport.
    ///
    /// This returns a snapshot of all responses at the time of the call.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::{Transport, MockTransport};
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = MockTransport::new();
    ///
    ///     let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
    ///     transport.send(response).await.unwrap();
    ///
    ///     let responses = transport.responses();
    ///     assert_eq!(responses.len(), 1);
    /// }
    /// ```
    pub fn responses(&self) -> Vec<JsonRpcResponse> {
        let state = self.state.lock();
        state.responses.clone()
    }

    /// Clear all queued requests.
    ///
    /// This removes all pending requests from the queue.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::MockTransport;
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let transport = MockTransport::new();
    ///
    /// transport.push_request(JsonRpcRequest::new(
    ///     Some(json!(1)),
    ///     "test".to_string(),
    ///     None
    /// ));
    ///
    /// transport.clear_requests();
    /// assert_eq!(transport.request_count(), 0);
    /// ```
    pub fn clear_requests(&self) {
        let mut state = self.state.lock();
        state.requests.clear();
    }

    /// Clear all recorded responses.
    ///
    /// This removes all responses from the history.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::{Transport, MockTransport};
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = MockTransport::new();
    ///
    ///     let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
    ///     transport.send(response).await.unwrap();
    ///
    ///     transport.clear_responses();
    ///     assert_eq!(transport.response_count(), 0);
    /// }
    /// ```
    pub fn clear_responses(&self) {
        let mut state = self.state.lock();
        state.responses.clear();
    }

    /// Get the number of queued requests.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::MockTransport;
    /// use mcp_server::protocol::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let transport = MockTransport::new();
    /// assert_eq!(transport.request_count(), 0);
    ///
    /// transport.push_request(JsonRpcRequest::new(
    ///     Some(json!(1)),
    ///     "test".to_string(),
    ///     None
    /// ));
    ///
    /// assert_eq!(transport.request_count(), 1);
    /// ```
    pub fn request_count(&self) -> usize {
        let state = self.state.lock();
        state.requests.len()
    }

    /// Get the number of sent responses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::{Transport, MockTransport};
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = MockTransport::new();
    ///     assert_eq!(transport.response_count(), 0);
    ///
    ///     let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
    ///     transport.send(response).await.unwrap();
    ///
    ///     assert_eq!(transport.response_count(), 1);
    /// }
    /// ```
    pub fn response_count(&self) -> usize {
        let state = self.state.lock();
        state.responses.len()
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        let mut state = self.state.lock();

        if state.closed {
            return None;
        }

        state.requests.pop_front()
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
        let mut state = self.state.lock();

        if state.closed {
            return Err(TransportError::Closed);
        }

        state.responses.push(response);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        let mut state = self.state.lock();
        state.closed = true;
        Ok(())
    }

    fn is_closed(&self) -> bool {
        let state = self.state.lock();
        state.closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mock_transport_creation() {
        let transport = MockTransport::new();
        assert!(!transport.is_closed());
        assert_eq!(transport.request_count(), 0);
        assert_eq!(transport.response_count(), 0);
    }

    #[test]
    fn test_mock_transport_default() {
        let transport = MockTransport::default();
        assert!(!transport.is_closed());
    }

    #[tokio::test]
    async fn test_mock_transport_push_and_recv() {
        let mut transport = MockTransport::new();

        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        transport.push_request(request.clone());

        assert_eq!(transport.request_count(), 1);

        let received = transport.recv().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().method, "test");

        assert_eq!(transport.request_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_transport_multiple_requests() {
        let mut transport = MockTransport::new();

        transport.push_request(JsonRpcRequest::new(Some(json!(1)), "first".to_string(), None));
        transport.push_request(JsonRpcRequest::new(Some(json!(2)), "second".to_string(), None));
        transport.push_request(JsonRpcRequest::new(Some(json!(3)), "third".to_string(), None));

        assert_eq!(transport.request_count(), 3);

        let first = transport.recv().await.unwrap();
        assert_eq!(first.method, "first");

        let second = transport.recv().await.unwrap();
        assert_eq!(second.method, "second");

        let third = transport.recv().await.unwrap();
        assert_eq!(third.method, "third");

        assert_eq!(transport.request_count(), 0);
    }

    #[tokio::test]
    async fn test_mock_transport_send_and_responses() {
        let mut transport = MockTransport::new();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({"test": true}));
        transport.send(response.clone()).await.unwrap();

        assert_eq!(transport.response_count(), 1);

        let responses = transport.responses();
        assert_eq!(responses.len(), 1);
        assert!(responses[0].is_success());
    }

    #[tokio::test]
    async fn test_mock_transport_multiple_responses() {
        let mut transport = MockTransport::new();

        for i in 1..=5 {
            let response = JsonRpcResponse::success(Some(json!(i)), json!({}));
            transport.send(response).await.unwrap();
        }

        assert_eq!(transport.response_count(), 5);

        let responses = transport.responses();
        assert_eq!(responses.len(), 5);
    }

    #[tokio::test]
    async fn test_mock_transport_close() {
        let mut transport = MockTransport::new();

        assert!(!transport.is_closed());

        transport.close().await.unwrap();

        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_mock_transport_recv_after_close() {
        let mut transport = MockTransport::new();

        transport.push_request(JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None));
        transport.close().await.unwrap();

        let result = transport.recv().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_transport_send_after_close() {
        let mut transport = MockTransport::new();

        transport.close().await.unwrap();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        let result = transport.send(response).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Closed));
    }

    #[tokio::test]
    async fn test_mock_transport_clear_requests() {
        let mut transport = MockTransport::new();

        transport.push_request(JsonRpcRequest::new(Some(json!(1)), "test1".to_string(), None));
        transport.push_request(JsonRpcRequest::new(Some(json!(2)), "test2".to_string(), None));

        assert_eq!(transport.request_count(), 2);

        transport.clear_requests();

        assert_eq!(transport.request_count(), 0);

        let result = transport.recv().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_transport_clear_responses() {
        let mut transport = MockTransport::new();

        transport
            .send(JsonRpcResponse::success(Some(json!(1)), json!({})))
            .await
            .unwrap();
        transport
            .send(JsonRpcResponse::success(Some(json!(2)), json!({})))
            .await
            .unwrap();

        assert_eq!(transport.response_count(), 2);

        transport.clear_responses();

        assert_eq!(transport.response_count(), 0);
        assert_eq!(transport.responses().len(), 0);
    }

    #[tokio::test]
    async fn test_mock_transport_clone() {
        let transport = MockTransport::new();

        transport.push_request(JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None));

        let cloned = transport.clone();

        assert_eq!(cloned.request_count(), 1);
        assert_eq!(transport.request_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_transport_concurrent_access() {
        let transport = MockTransport::new();

        // Spawn multiple tasks that push requests
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let transport = transport.clone();
                tokio::spawn(async move {
                    transport.push_request(JsonRpcRequest::new(
                        Some(json!(i)),
                        format!("test{}", i),
                        None,
                    ));
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(transport.request_count(), 10);
    }

    #[tokio::test]
    async fn test_mock_transport_full_workflow() {
        let mut transport = MockTransport::new();

        // Setup: Queue requests
        transport.push_request(JsonRpcRequest::new(
            Some(json!(1)),
            "initialize".to_string(),
            Some(json!({"version": "1.0"})),
        ));
        transport.push_request(JsonRpcRequest::new(
            Some(json!(2)),
            "tools/list".to_string(),
            None,
        ));

        // Process requests
        while let Some(request) = transport.recv().await {
            let response = match request.method.as_str() {
                "initialize" => {
                    JsonRpcResponse::success(request.id, json!({"capabilities": {}}))
                }
                "tools/list" => JsonRpcResponse::success(request.id, json!({"tools": []})),
                _ => JsonRpcResponse::method_not_found(request.id),
            };

            transport.send(response).await.unwrap();
        }

        // Verify
        let responses = transport.responses();
        assert_eq!(responses.len(), 2);
        assert!(responses[0].is_success());
        assert!(responses[1].is_success());
    }

    #[test]
    fn test_mock_transport_thread_safe() {
        // Verify that MockTransport implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<MockTransport>();
        assert_sync::<MockTransport>();
    }

    #[tokio::test]
    async fn test_mock_transport_idempotent_close() {
        let mut transport = MockTransport::new();

        transport.close().await.unwrap();
        transport.close().await.unwrap(); // Should not panic
        transport.close().await.unwrap(); // Should not panic

        assert!(transport.is_closed());
    }
}
