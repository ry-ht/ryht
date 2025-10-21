//! Transport Layer for MCP Server
//!
//! This module provides the transport abstraction for sending and receiving JSON-RPC messages
//! in the MCP protocol. It defines a common `Transport` trait and provides multiple implementations
//! for different communication channels.
//!
//! # Overview
//!
//! The transport layer is responsible for:
//! - Receiving JSON-RPC requests from clients
//! - Sending JSON-RPC responses back to clients
//! - Managing connection lifecycle (open/close)
//! - Handling transport-specific error conditions
//!
//! # Available Transports
//!
//! ## Stdio Transport
//!
//! The stdio transport is the most common transport for MCP servers. It reads requests
//! line-by-line from stdin and writes responses line-by-line to stdout.
//!
//! ```rust,no_run
//! use mcp_server::transport::{Transport, StdioTransport};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut transport = StdioTransport::new();
//!
//!     while let Some(request) = transport.recv().await {
//!         // Handle request...
//!     }
//! }
//! ```
//!
//! ## HTTP Transport (feature: `http`)
//!
//! The HTTP transport provides REST endpoints for JSON-RPC communication.
//! It supports both POST requests for request/response and SSE for streaming.
//!
//! ```rust,no_run
//! #[cfg(feature = "http")]
//! use mcp_server::transport::HttpTransport;
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() {
//!     # #[cfg(feature = "http")]
//!     # {
//!     let addr: SocketAddr = "127.0.0.1:3000".parse().unwrap();
//!     let transport = HttpTransport::new(addr);
//!     // Use transport...
//!     # }
//! }
//! ```
//!
//! ## Mock Transport
//!
//! The mock transport is designed for testing. It allows you to queue requests
//! and inspect responses without any actual I/O.
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
//!     transport.push_request(JsonRpcRequest::new(
//!         Some(json!(1)),
//!         "test".to_string(),
//!         None
//!     ));
//!
//!     // Receive and respond
//!     let request = transport.recv().await.unwrap();
//!     let response = JsonRpcResponse::success(request.id, json!({}));
//!     transport.send(response).await.unwrap();
//!
//!     // Verify responses
//!     assert_eq!(transport.responses().len(), 1);
//! }
//! ```
//!
//! # Implementing Custom Transports
//!
//! You can implement custom transports by implementing the `Transport` trait:
//!
//! ```rust
//! use async_trait::async_trait;
//! use mcp_server::transport::Transport;
//! use mcp_server::protocol::{JsonRpcRequest, JsonRpcResponse};
//! use mcp_server::error::TransportError;
//!
//! struct CustomTransport {
//!     closed: bool,
//! }
//!
//! #[async_trait]
//! impl Transport for CustomTransport {
//!     async fn recv(&mut self) -> Option<JsonRpcRequest> {
//!         if self.closed {
//!             return None;
//!         }
//!         // Your implementation...
//!         None
//!     }
//!
//!     async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
//!         if self.closed {
//!             return Err(TransportError::Closed);
//!         }
//!         // Your implementation...
//!         Ok(())
//!     }
//!
//!     async fn close(&mut self) -> Result<(), TransportError> {
//!         self.closed = true;
//!         Ok(())
//!     }
//!
//!     fn is_closed(&self) -> bool {
//!         self.closed
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! Transport operations can fail with `TransportError`:
//!
//! - `TransportError::Io` - I/O errors (network, file system)
//! - `TransportError::Closed` - Transport is closed
//! - `TransportError::InvalidMessage` - Message serialization/deserialization failed
//!
//! # Thread Safety
//!
//! All transport implementations are thread-safe (`Send + Sync`) and can be used
//! across async tasks.

// Core trait definition
pub mod traits;

// Transport implementations
pub mod stdio;

#[cfg(feature = "http")]
pub mod http;

pub mod mock;

// Re-export commonly used types
pub use traits::Transport;

pub use stdio::StdioTransport;

#[cfg(feature = "http")]
pub use http::HttpTransport;

pub use mock::MockTransport;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
    use serde_json::json;

    #[tokio::test]
    async fn test_stdio_transport_implements_trait() {
        let _transport: Box<dyn Transport> = Box::new(StdioTransport::new());
    }

    #[tokio::test]
    async fn test_mock_transport_implements_trait() {
        let _transport: Box<dyn Transport> = Box::new(MockTransport::new());
    }

    #[cfg(feature = "http")]
    #[tokio::test]
    async fn test_http_transport_implements_trait() {
        let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
        let _transport: Box<dyn Transport> = Box::new(HttpTransport::new(addr));
    }

    #[tokio::test]
    async fn test_transport_trait_object() {
        // Verify we can use Transport as a trait object
        let mut transport: Box<dyn Transport> = Box::new(MockTransport::new());

        assert!(!transport.is_closed());

        // Test close
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_transport_trait_object_recv() {
        let mock = MockTransport::new();
        mock.push_request(JsonRpcRequest::new(
            Some(json!(1)),
            "test".to_string(),
            None,
        ));

        let mut transport: Box<dyn Transport> = Box::new(mock);

        let request = transport.recv().await;
        assert!(request.is_some());
        assert_eq!(request.unwrap().method, "test");
    }

    #[tokio::test]
    async fn test_transport_trait_object_send() {
        let mut transport: Box<dyn Transport> = Box::new(MockTransport::new());

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        assert!(transport.send(response).await.is_ok());
    }

    #[tokio::test]
    async fn test_all_transports_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<StdioTransport>();
        assert_send_sync::<MockTransport>();

        #[cfg(feature = "http")]
        assert_send_sync::<HttpTransport>();
    }

    #[tokio::test]
    async fn test_transport_error_on_closed() {
        let mut transport = MockTransport::new();

        transport.close().await.unwrap();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        let result = transport.send(response).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::TransportError::Closed => {}
            _ => panic!("Expected TransportError::Closed"),
        }
    }

    #[tokio::test]
    async fn test_transport_lifecycle() {
        let mut transport = MockTransport::new();

        // Open state
        assert!(!transport.is_closed());

        // Can send/recv
        transport
            .push_request(JsonRpcRequest::new(Some(json!(1)), "test".to_string(), None));
        assert!(transport.recv().await.is_some());

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        assert!(transport.send(response).await.is_ok());

        // Close
        transport.close().await.unwrap();
        assert!(transport.is_closed());

        // Cannot recv after close
        assert!(transport.recv().await.is_none());

        // Cannot send after close
        let response = JsonRpcResponse::success(Some(json!(2)), json!({}));
        assert!(transport.send(response).await.is_err());
    }

    #[tokio::test]
    async fn test_multiple_transports_concurrently() {
        let mut transport1 = MockTransport::new();
        let mut transport2 = MockTransport::new();

        transport1.push_request(JsonRpcRequest::new(
            Some(json!(1)),
            "request1".to_string(),
            None,
        ));
        transport2.push_request(JsonRpcRequest::new(
            Some(json!(2)),
            "request2".to_string(),
            None,
        ));

        let handle1 = tokio::spawn(async move {
            let request = transport1.recv().await.unwrap();
            assert_eq!(request.method, "request1");
            let response = JsonRpcResponse::success(request.id, json!({}));
            transport1.send(response).await.unwrap();
            transport1.response_count()
        });

        let handle2 = tokio::spawn(async move {
            let request = transport2.recv().await.unwrap();
            assert_eq!(request.method, "request2");
            let response = JsonRpcResponse::success(request.id, json!({}));
            transport2.send(response).await.unwrap();
            transport2.response_count()
        });

        let count1 = handle1.await.unwrap();
        let count2 = handle2.await.unwrap();

        assert_eq!(count1, 1);
        assert_eq!(count2, 1);
    }
}
