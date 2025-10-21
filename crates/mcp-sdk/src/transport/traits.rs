//! Transport trait definitions
//!
//! This module defines the core `Transport` trait that all transport implementations must implement.
//!
//! # Transport Abstraction
//!
//! The `Transport` trait provides a unified interface for sending and receiving JSON-RPC messages
//! over different transport protocols (stdio, HTTP, WebSocket, etc.).
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::transport::{Transport, StdioTransport};
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut transport = StdioTransport::new();
//!
//!     // Receive requests
//!     while let Some(request) = transport.recv().await {
//!         // Process request...
//!         let response = JsonRpcResponse::success(request.id, serde_json::json!({}));
//!
//!         // Send response
//!         transport.send(response).await.unwrap();
//!     }
//! }
//! ```

use async_trait::async_trait;

use crate::error::TransportError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Transport trait for sending and receiving JSON-RPC messages.
///
/// This trait defines the interface that all transport implementations must provide.
/// It supports bidirectional communication, graceful shutdown, and connection state tracking.
///
/// # Implementation Notes
///
/// - All implementations must be thread-safe (`Send + Sync`)
/// - `recv()` should block until a message is available or the transport is closed
/// - `send()` should be non-blocking or handle backpressure appropriately
/// - `close()` should be idempotent (calling multiple times is safe)
/// - After `close()` is called, `recv()` should return `None` and `send()` should return an error
///
/// # Examples
///
/// ```rust
/// use async_trait::async_trait;
/// use mcp_server::transport::Transport;
/// use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
/// use mcp_server::error::TransportError;
///
/// struct CustomTransport {
///     closed: bool,
/// }
///
/// #[async_trait]
/// impl Transport for CustomTransport {
///     async fn recv(&mut self) -> Option<JsonRpcRequest> {
///         if self.closed {
///             return None;
///         }
///         // Custom receive logic...
///         None
///     }
///
///     async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
///         if self.closed {
///             return Err(TransportError::Closed);
///         }
///         // Custom send logic...
///         Ok(())
///     }
///
///     async fn close(&mut self) -> Result<(), TransportError> {
///         self.closed = true;
///         Ok(())
///     }
///
///     fn is_closed(&self) -> bool {
///         self.closed
///     }
/// }
/// ```
#[async_trait]
pub trait Transport: Send + Sync {
    /// Receive the next JSON-RPC request from the transport.
    ///
    /// This method blocks until a request is available or the transport is closed.
    /// Returns `None` when the transport is closed or EOF is reached.
    ///
    /// # Returns
    ///
    /// - `Some(request)` - A valid JSON-RPC request was received
    /// - `None` - The transport is closed or EOF was reached
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::transport::{Transport, StdioTransport};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = StdioTransport::new();
    ///
    ///     while let Some(request) = transport.recv().await {
    ///         println!("Received request: {}", request.method);
    ///     }
    ///
    ///     println!("Transport closed");
    /// }
    /// ```
    async fn recv(&mut self) -> Option<JsonRpcRequest>;

    /// Send a JSON-RPC response through the transport.
    ///
    /// This method sends a response to the client. It may block if the transport
    /// has backpressure, but should generally be non-blocking.
    ///
    /// # Arguments
    ///
    /// * `response` - The JSON-RPC response to send
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transport is closed (`TransportError::Closed`)
    /// - An I/O error occurs (`TransportError::Io`)
    /// - The message cannot be serialized (`TransportError::InvalidMessage`)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::transport::{Transport, StdioTransport};
    /// use mcp_server::protocol::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = StdioTransport::new();
    ///
    ///     let response = JsonRpcResponse::success(
    ///         Some(json!(1)),
    ///         json!({"message": "Success"})
    ///     );
    ///
    ///     transport.send(response).await.unwrap();
    /// }
    /// ```
    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError>;

    /// Close the transport gracefully.
    ///
    /// This method performs a graceful shutdown of the transport, ensuring that
    /// any pending messages are flushed and resources are released.
    ///
    /// # Errors
    ///
    /// Returns an error if the transport cannot be closed cleanly. However, the
    /// transport should still be considered closed even if an error is returned.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::transport::{Transport, StdioTransport};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = StdioTransport::new();
    ///
    ///     // Use the transport...
    ///
    ///     transport.close().await.unwrap();
    ///     assert!(transport.is_closed());
    /// }
    /// ```
    async fn close(&mut self) -> Result<(), TransportError>;

    /// Check if the transport is closed.
    ///
    /// Returns `true` if the transport has been closed (either explicitly via `close()`
    /// or due to an error/EOF condition), `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mcp_server::transport::{Transport, StdioTransport};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut transport = StdioTransport::new();
    ///
    ///     assert!(!transport.is_closed());
    ///
    ///     transport.close().await.unwrap();
    ///     assert!(transport.is_closed());
    /// }
    /// ```
    fn is_closed(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock transport for testing
    struct MockTestTransport {
        closed: bool,
    }

    #[async_trait]
    impl Transport for MockTestTransport {
        async fn recv(&mut self) -> Option<JsonRpcRequest> {
            if self.closed {
                return None;
            }
            Some(JsonRpcRequest::new(
                Some(json!(1)),
                "test".to_string(),
                None,
            ))
        }

        async fn send(&mut self, _response: JsonRpcResponse) -> Result<(), TransportError> {
            if self.closed {
                return Err(TransportError::Closed);
            }
            Ok(())
        }

        async fn close(&mut self) -> Result<(), TransportError> {
            self.closed = true;
            Ok(())
        }

        fn is_closed(&self) -> bool {
            self.closed
        }
    }

    #[tokio::test]
    async fn test_mock_transport_recv() {
        let mut transport = MockTestTransport { closed: false };

        let request = transport.recv().await;
        assert!(request.is_some());
        assert_eq!(request.unwrap().method, "test");
    }

    #[tokio::test]
    async fn test_mock_transport_recv_after_close() {
        let mut transport = MockTestTransport { closed: false };

        transport.close().await.unwrap();
        let request = transport.recv().await;
        assert!(request.is_none());
    }

    #[tokio::test]
    async fn test_mock_transport_send() {
        let mut transport = MockTestTransport { closed: false };

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        assert!(transport.send(response).await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_transport_send_after_close() {
        let mut transport = MockTestTransport { closed: false };

        transport.close().await.unwrap();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        let result = transport.send(response).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Closed));
    }

    #[tokio::test]
    async fn test_mock_transport_close() {
        let mut transport = MockTestTransport { closed: false };

        assert!(!transport.is_closed());
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_mock_transport_close_idempotent() {
        let mut transport = MockTestTransport { closed: false };

        transport.close().await.unwrap();
        transport.close().await.unwrap(); // Should not panic
        assert!(transport.is_closed());
    }
}
