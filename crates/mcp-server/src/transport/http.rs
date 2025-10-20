//! HTTP Transport Implementation
//!
//! This module provides an HTTP-based transport implementation using Axum.
//! It supports both regular HTTP POST requests and Server-Sent Events (SSE)
//! for streaming responses.
//!
//! # Endpoints
//!
//! - `POST /mcp` - Send JSON-RPC requests and receive responses
//! - `GET /mcp/sse` - Establish SSE connection for streaming notifications
//!
//! # Examples
//!
//! ```rust,no_run
//! use mcp_server::transport::HttpTransport;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() {
//!     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
//!     let transport = HttpTransport::new(addr);
//!
//!     println!("HTTP transport listening on {}", addr);
//!
//!     // Server will run until shutdown signal
//!     // transport.serve(server).await.unwrap();
//! }
//! ```

#[cfg(feature = "http")]
use async_trait::async_trait;
#[cfg(feature = "http")]
use axum::{
    extract::State,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
#[cfg(feature = "http")]
use futures::stream::{self, Stream};
#[cfg(feature = "http")]
use std::convert::Infallible;
#[cfg(feature = "http")]
use std::net::SocketAddr;
#[cfg(feature = "http")]
use std::sync::Arc;
#[cfg(feature = "http")]
use tokio::sync::{mpsc, RwLock};
#[cfg(feature = "http")]
use tower_http::cors::CorsLayer;

#[cfg(feature = "http")]
use crate::error::TransportError;
#[cfg(feature = "http")]
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

#[cfg(feature = "http")]
use super::traits::Transport;

/// HTTP transport for JSON-RPC communication.
///
/// This transport provides HTTP endpoints for receiving requests and sending responses.
/// It supports both request/response via POST and streaming via SSE.
///
/// # Architecture
///
/// The HTTP transport uses channels to bridge between HTTP handlers and the transport trait:
/// - Incoming requests are queued in a channel for `recv()`
/// - Responses are sent back via a response channel
/// - SSE connections receive notifications via a broadcast channel
///
/// # Examples
///
/// ```rust,no_run
/// use mcp_server::transport::HttpTransport;
/// use std::net::SocketAddr;
///
/// #[tokio::main]
/// async fn main() {
///     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
///     let transport = HttpTransport::new(addr);
///
///     // Use transport with MCP server...
/// }
/// ```
#[cfg(feature = "http")]
pub struct HttpTransport {
    addr: SocketAddr,
    request_rx: Arc<RwLock<mpsc::UnboundedReceiver<JsonRpcRequest>>>,
    request_tx: mpsc::UnboundedSender<JsonRpcRequest>,
    response_tx: Arc<RwLock<Option<mpsc::UnboundedSender<JsonRpcResponse>>>>,
    notification_tx: Arc<tokio::sync::broadcast::Sender<JsonRpcResponse>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(feature = "http")]
struct AppState {
    request_tx: mpsc::UnboundedSender<JsonRpcRequest>,
    notification_tx: Arc<tokio::sync::broadcast::Sender<JsonRpcResponse>>,
}

#[cfg(feature = "http")]
impl HttpTransport {
    /// Create a new HTTP transport bound to the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address to bind to (e.g., "127.0.0.1:3000")
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::HttpTransport;
    /// use std::net::SocketAddr;
    ///
    /// let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    /// let transport = HttpTransport::new(addr);
    /// ```
    pub fn new(addr: SocketAddr) -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let (notification_tx, _) = tokio::sync::broadcast::channel(100);

        Self {
            addr,
            request_rx: Arc::new(RwLock::new(request_rx)),
            request_tx,
            response_tx: Arc::new(RwLock::new(None)),
            notification_tx: Arc::new(notification_tx),
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get the socket address this transport is bound to.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::HttpTransport;
    /// use std::net::SocketAddr;
    ///
    /// let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    /// let transport = HttpTransport::new(addr);
    ///
    /// assert_eq!(transport.addr(), &addr);
    /// ```
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    /// Build the Axum router for this transport.
    ///
    /// This creates the HTTP application with all routes configured.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::transport::HttpTransport;
    /// use std::net::SocketAddr;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
    ///     let transport = HttpTransport::new(addr);
    ///     let router = transport.router();
    ///
    ///     // Serve with axum
    ///     let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    ///     axum::serve(listener, router).await.unwrap();
    /// }
    /// ```
    pub fn router(&self) -> Router {
        let state = AppState {
            request_tx: self.request_tx.clone(),
            notification_tx: Arc::clone(&self.notification_tx),
        };

        Router::new()
            .route("/mcp", post(handle_mcp_request))
            .route("/mcp/sse", get(handle_sse))
            .layer(CorsLayer::permissive())
            .with_state(state)
    }
}

#[cfg(feature = "http")]
async fn handle_mcp_request(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    // Send request to transport
    if state.request_tx.send(request).is_err() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(JsonRpcResponse::internal_error(
                None,
                Some("Server is shutting down".to_string()),
            )),
        )
            .into_response();
    }

    // For now, return a simple acknowledgment
    // In a real implementation, we would wait for the response
    // This would require a request-response mapping system
    (
        StatusCode::OK,
        Json(JsonRpcResponse::success(
            None,
            serde_json::json!({"status": "received"}),
        )),
    )
        .into_response()
}

#[cfg(feature = "http")]
async fn handle_sse(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.notification_tx.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(response) => {
                let json = serde_json::to_string(&response).ok()?;
                let event = Event::default().data(json);
                Some((Ok(event), rx))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream)
}

#[cfg(feature = "http")]
#[async_trait]
impl Transport for HttpTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        if self
            .closed
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return None;
        }

        let mut rx = self.request_rx.write().await;
        rx.recv().await
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
        if self
            .closed
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(TransportError::Closed);
        }

        // Send to direct response channel if available
        if let Some(tx) = self.response_tx.read().await.as_ref() {
            tx.send(response.clone())
                .map_err(|_| TransportError::Closed)?;
        }

        // Also broadcast for SSE connections
        let _ = self.notification_tx.send(response);

        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
#[cfg(feature = "http")]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_http_transport_creation() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let transport = HttpTransport::new(addr);

        assert_eq!(transport.addr(), &addr);
        assert!(!transport.is_closed());
    }

    #[tokio::test]
    async fn test_http_transport_close() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_http_transport_send_after_close() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        transport.close().await.unwrap();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        let result = transport.send(response).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Closed));
    }

    #[tokio::test]
    async fn test_http_transport_recv_after_close() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        transport.close().await.unwrap();

        let result = transport.recv().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_http_transport_router() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let transport = HttpTransport::new(addr);

        let router = transport.router();
        // Router should be created without errors
        drop(router);
    }

    #[tokio::test]
    async fn test_http_transport_request_channel() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        // Send a request through the channel
        let request = JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None);
        transport.request_tx.send(request.clone()).unwrap();

        // Receive it through the transport
        let received = transport.recv().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().method, "test");
    }

    #[tokio::test]
    async fn test_http_transport_notification_broadcast() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        // Subscribe to notifications
        let mut rx = transport.notification_tx.subscribe();

        // Send a response
        let response = JsonRpcResponse::success(Some(json!(1)), json!({"test": true}));
        transport.send(response.clone()).await.unwrap();

        // Receive notification
        let received = rx.recv().await;
        assert!(received.is_ok());
    }

    #[test]
    fn test_http_transport_thread_safe() {
        // Verify that HttpTransport implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<HttpTransport>();
        assert_sync::<HttpTransport>();
    }

    #[tokio::test]
    async fn test_http_transport_multiple_recv() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let mut transport = HttpTransport::new(addr);

        // Send multiple requests
        let req1 = JsonRpcRequest::new(Some(json!(1)), "test1".to_string(), None);
        let req2 = JsonRpcRequest::new(Some(json!(2)), "test2".to_string(), None);

        transport.request_tx.send(req1).unwrap();
        transport.request_tx.send(req2).unwrap();

        // Receive them in order
        let recv1 = transport.recv().await;
        assert!(recv1.is_some());
        assert_eq!(recv1.unwrap().method, "test1");

        let recv2 = transport.recv().await;
        assert!(recv2.is_some());
        assert_eq!(recv2.unwrap().method, "test2");
    }

    #[tokio::test]
    async fn test_http_transport_addr_getter() {
        let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let transport = HttpTransport::new(addr);

        assert_eq!(*transport.addr(), addr);
    }
}

// Re-export when feature is disabled with documentation
#[cfg(not(feature = "http"))]
compile_error!("http feature is required for HttpTransport");
