//! Stdio Transport Implementation
//!
//! This module provides a transport implementation that uses standard input/output
//! for communication. This is the most common transport for MCP servers, as it allows
//! them to be launched as child processes by MCP clients.
//!
//! # Protocol
//!
//! The stdio transport uses line-delimited JSON:
//! - Each request is a single line of JSON followed by a newline
//! - Each response is a single line of JSON followed by a newline
//! - EOF on stdin signals that the transport should close
//!
//! # Examples
//!
//! ```rust,no_run
//! use mcp_server::transport::{Transport, StdioTransport};
//! use mcp_server::protocol::JsonRpcResponse;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut transport = StdioTransport::new();
//!
//!     while let Some(request) = transport.recv().await {
//!         println!("Received: {}", request.method);
//!
//!         let response = JsonRpcResponse::success(
//!             request.id,
//!             json!({"message": "OK"})
//!         );
//!
//!         transport.send(response).await.unwrap();
//!     }
//! }
//! ```

use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};

use crate::error::TransportError;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

use super::traits::Transport;

/// Stdio transport for JSON-RPC communication.
///
/// This transport reads JSON-RPC requests line-by-line from stdin and writes
/// JSON-RPC responses line-by-line to stdout.
///
/// # Thread Safety
///
/// This transport is thread-safe and can be used across async tasks. The closed
/// state is managed with an atomic boolean.
///
/// # Error Handling
///
/// - Malformed JSON lines are silently skipped and logged to stderr
/// - EOF on stdin causes `recv()` to return `None`
/// - I/O errors are propagated as `TransportError::Io`
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
///     // Read requests from stdin, write responses to stdout
///     while let Some(request) = transport.recv().await {
///         // Handle request...
///     }
/// }
/// ```
pub struct StdioTransport {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
    closed: Arc<AtomicBool>,
}

impl StdioTransport {
    /// Create a new stdio transport.
    ///
    /// This creates a transport that reads from stdin and writes to stdout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::transport::StdioTransport;
    ///
    /// let transport = StdioTransport::new();
    /// ```
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(stdin()),
            stdout: stdout(),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn recv(&mut self) -> Option<JsonRpcRequest> {
        if self.closed.load(Ordering::SeqCst) {
            return None;
        }

        let mut line = String::new();
        match self.stdin.read_line(&mut line).await {
            Ok(0) => {
                // EOF reached
                self.closed.store(true, Ordering::SeqCst);
                None
            }
            Ok(_) => {
                // Try to parse the line as JSON-RPC request
                match serde_json::from_str::<JsonRpcRequest>(&line) {
                    Ok(request) => Some(request),
                    Err(e) => {
                        // Log error to stderr and continue
                        eprintln!("Failed to parse JSON-RPC request: {}", e);
                        eprintln!("Line: {}", line.trim());
                        // Try to receive the next line
                        Box::pin(self.recv()).await
                    }
                }
            }
            Err(e) => {
                eprintln!("I/O error reading from stdin: {}", e);
                self.closed.store(true, Ordering::SeqCst);
                None
            }
        }
    }

    async fn send(&mut self, response: JsonRpcResponse) -> Result<(), TransportError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(TransportError::Closed);
        }

        // Serialize response to JSON
        let json = serde_json::to_string(&response)
            .map_err(|e| TransportError::InvalidMessage(e.to_string()))?;

        // Write JSON line to stdout
        self.stdout
            .write_all(json.as_bytes())
            .await
            .map_err(TransportError::Io)?;

        self.stdout
            .write_all(b"\n")
            .await
            .map_err(TransportError::Io)?;

        self.stdout.flush().await.map_err(TransportError::Io)?;

        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.closed.store(true, Ordering::SeqCst);
        self.stdout.flush().await.map_err(TransportError::Io)?;
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stdio_transport_creation() {
        let transport = StdioTransport::new();
        assert!(!transport.is_closed());
    }

    #[test]
    fn test_stdio_transport_default() {
        let transport = StdioTransport::default();
        assert!(!transport.is_closed());
    }

    #[tokio::test]
    async fn test_stdio_transport_close() {
        let mut transport = StdioTransport::new();
        assert!(!transport.is_closed());

        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_stdio_transport_close_idempotent() {
        let mut transport = StdioTransport::new();

        transport.close().await.unwrap();
        assert!(transport.is_closed());

        // Second close should also succeed
        transport.close().await.unwrap();
        assert!(transport.is_closed());
    }

    #[tokio::test]
    async fn test_stdio_transport_send_after_close() {
        let mut transport = StdioTransport::new();
        transport.close().await.unwrap();

        let response = JsonRpcResponse::success(Some(json!(1)), json!({}));
        let result = transport.send(response).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::Closed));
    }

    #[tokio::test]
    async fn test_stdio_transport_recv_after_close() {
        let mut transport = StdioTransport::new();
        transport.close().await.unwrap();

        let result = transport.recv().await;
        assert!(result.is_none());
    }

    // Note: Testing actual stdin/stdout interaction requires integration tests
    // with process spawning, which is beyond the scope of unit tests.
    // The send/recv logic is tested indirectly through the mock transport tests
    // in traits.rs and through integration tests.

    #[tokio::test]
    async fn test_stdio_transport_closed_state_atomic() {
        let mut transport = StdioTransport::new();
        let closed_clone = Arc::clone(&transport.closed);

        assert!(!closed_clone.load(Ordering::SeqCst));

        transport.close().await.unwrap();

        assert!(closed_clone.load(Ordering::SeqCst));
        assert!(transport.is_closed());
    }

    #[test]
    fn test_stdio_transport_thread_safe() {
        // This test verifies that StdioTransport implements Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<StdioTransport>();
        assert_sync::<StdioTransport>();
    }
}
