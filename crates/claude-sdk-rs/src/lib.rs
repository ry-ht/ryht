//! # claude-sdk-rs - Rust SDK for Claude Code
//!
//! A comprehensive, type-safe, async-first Rust SDK that provides programmatic access
//! to Claude AI. This crate transforms the Claude Code CLI tool into a powerful library.
//!
//! ## Features
//!
//! claude-sdk-rs is designed with modularity in mind, using feature flags to provide
//! only the functionality you need:
//!
//! ### Core Features (Always Available)
//! - **Async-first API**: Built on tokio for high-performance async operations
//! - **Type-safe responses**: Structured data types for all API responses
//! - **Multiple response formats**: Text, JSON, and streaming JSON support
//! - **Session management**: Persistent conversation contexts
//! - **Error handling**: Comprehensive error types with detailed messages
//! - **Configuration**: Flexible configuration with builder pattern
//!
//! ### Optional Features
//!
//! #### `mcp` - Model Context Protocol
//! Enables MCP server and client functionality:
//! ```toml
//! [dependencies]
//! claude-sdk-rs = { version = "1.0", features = ["mcp"] }
//! ```
//! - MCP server implementation
//! - Tool integration capabilities
//! - External service connections
//! - Protocol message handling
//!
//! #### `sqlite` - SQLite Storage Backend
//! Adds SQLite support for persistent storage:
//! ```toml
//! [dependencies]
//! claude-sdk-rs = { version = "1.0", features = ["sqlite"] }
//! ```
//! - Persistent session storage
//! - Query history
//! - Configuration persistence
//!
//! #### `full` - All Features Enabled
//! Convenience feature that enables everything:
//! ```toml
//! [dependencies]
//! claude-sdk-rs = { version = "1.0", features = ["full"] }
//! ```
//!
//! ## Architecture Overview
//!
//! The SDK is organized into several core modules:
//!
//! ### Core Module (`core`)
//! Contains fundamental types and configuration:
//! - [`Config`] - SDK configuration with builder pattern
//! - [`Error`] - Comprehensive error handling
//! - [`Message`] - Request/response message types
//! - [`Session`] - Conversation session management
//! - [`StreamFormat`] - Response format configuration
//!
//! ### Runtime Module (`runtime`)
//! Handles Claude Code CLI interaction:
//! - [`Client`] - Main API client
//! - [`QueryBuilder`] - Fluent query construction
//! - [`MessageStream`] - Streaming response handling
//!
//! ### MCP Module (`mcp`) - *Feature Gated*
//! Model Context Protocol implementation:
//! - Server and client implementations
//! - Tool integration framework
//! - External service connections
//!
//! ## Quick Start Examples
//!
//! ### Basic SDK Usage
//! ```rust,no_run
//! use claude_sdk_rs::{Client, Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), claude_sdk_rs::Error> {
//!     // Create client with default configuration
//!     let client = Client::new(Config::default());
//!     
//!     // Send a simple query
//!     let response = client
//!         .query("Write a hello world program in Rust")
//!         .send()
//!         .await?;
//!     
//!     println!("Claude's response: {}", response);
//!     Ok(())
//! }
//! ```
//!
//! ### Advanced Configuration
//! ```rust,no_run
//! use claude_sdk_rs::{Client, Config, StreamFormat};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), claude_sdk_rs::Error> {
//!     // Build custom configuration
//!     let config = Config::builder()
//!         .model("claude-3-sonnet-20240229")
//!         .system_prompt("You are a helpful coding assistant.")
//!         .timeout_secs(60)
//!         .stream_format(StreamFormat::Json)
//!         .build()?;
//!
//!     let client = Client::new(config);
//!     
//!     // Get full response with metadata
//!     let response = client
//!         .query("Explain Rust ownership")
//!         .send_full()
//!         .await?;
//!     
//!     println!("Response: {}", response.content);
//!     if let Some(metadata) = response.metadata {
//!         println!("Cost: ${:.6}", metadata.cost_usd.unwrap_or(0.0));
//!         println!("Session: {}", metadata.session_id);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Streaming Responses
//! ```rust,no_run
//! use claude_sdk_rs::{Client, Config, StreamFormat};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), claude_sdk_rs::Error> {
//!     let client = Client::builder()
//!         .stream_format(StreamFormat::StreamJson)
//!         .build()?;
//!     
//!     let mut stream = client
//!         .query("Write a short story about a robot")
//!         .stream()
//!         .await?;
//!
//!     // Process streaming response
//!     while let Some(message) = stream.next().await {
//!         match message {
//!             Ok(msg) => println!("Message: {:?}", msg),
//!             Err(e) => eprintln!("Stream error: {}", e),
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Session Management
//! ```rust,no_run
//! use claude_sdk_rs::{Client, Config, StreamFormat};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), claude_sdk_rs::Error> {
//!     let client = Client::builder()
//!         .stream_format(StreamFormat::Json) // Needed for session metadata
//!         .build()?;
//!     
//!     // Start a conversation
//!     let response1 = client
//!         .query("Hello! My name is Alice and I'm learning Rust.")
//!         .send_full()
//!         .await?;
//!     
//!     println!("Response 1: {}", response1.content);
//!     
//!     // Continue in the same session - Claude remembers context
//!     let response2 = client
//!         .query("What's my name?")
//!         .send_full()
//!         .await?;
//!     
//!     println!("Response 2: {}", response2.content);
//!     // Should respond with "Alice"
//!     Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! The SDK provides comprehensive error handling with the [`Error`] enum:
//!
//! ```rust,no_run
//! use claude_sdk_rs::{Client, Config, Error};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::new(Config::default());
//!     
//!     match client.query("Hello").send().await {
//!         Ok(response) => println!("Success: {}", response),
//!         Err(Error::ProcessError(e)) => {
//!             eprintln!("Claude CLI process error: {}", e);
//!         }
//!         Err(Error::SerializationError(e)) => {
//!             eprintln!("JSON parsing error: {}", e);
//!         }
//!         Err(Error::BinaryNotFound) => {
//!             eprintln!("Claude CLI not found. Please install from https://claude.ai/code");
//!         }
//!         Err(Error::Timeout(_)) => {
//!             eprintln!("Request timed out. Consider increasing timeout_secs in Config");
//!         }
//!         Err(e) => eprintln!("Other error: {}", e),
//!     }
//! }
//! ```
//!
//! ## Performance and Best Practices
//!
//! ### Configuration Optimization
//! - Use appropriate timeout values for your use case
//! - Choose the right [`StreamFormat`] for your needs:
//!   - `Text`: Fastest, raw output
//!   - `Json`: Structured data with metadata
//!   - `StreamJson`: Real-time streaming with metadata
//!
//! ### Error Handling
//! - Always handle [`Error::BinaryNotFound`] to guide users to install Claude CLI
//! - Implement retry logic for transient failures
//! - Use appropriate timeout values
//!
//! ### Memory Management
//! - Reuse [`Client`] instances when possible
//! - Process streaming responses incrementally for large outputs
//! - Use [`Config::builder()`] to avoid unnecessary allocations
//!
//! ### Security Considerations
//! - Never log or store API responses containing sensitive data
//! - Use environment variables for configuration in production
//! - Validate user input before sending to Claude
//!
//! ## Compatibility and Requirements
//!
//! - **Rust Version**: 1.70 or later (MSRV)
//! - **Claude Code CLI**: Must be installed and authenticated
//! - **Runtime**: Requires tokio async runtime
//! - **Platforms**: Linux, macOS, Windows
//!
//! ## Examples
//!
//! For more comprehensive examples, see the `examples/` directory:
//! - `basic_usage.rs` - Simple SDK usage patterns
//! - `streaming.rs` - Real-time streaming responses
//! - `error_handling.rs` - Comprehensive error handling
//! - `configuration.rs` - Advanced configuration options
//! - `session_management.rs` - Multi-turn conversations

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

// Core modules (always available)
pub mod core;
pub mod runtime;

// Feature-gated modules
#[cfg(feature = "mcp")]
pub mod mcp;

// Re-export core types for convenience
pub use crate::core::{
    ClaudeResponse, Config, Cost, Error, Message, MessageMeta, MessageType, ResponseMetadata,
    Result, Session, SessionId, SessionManager, StreamFormat, TokenUsage, ToolPermission,
};

// Re-export runtime types
pub use crate::runtime::{Client, MessageStream, QueryBuilder};

// Re-export MCP types when feature is enabled
#[cfg(feature = "mcp")]
pub use crate::mcp::{McpConfig, McpServer};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{Client, Config, Error, Message, MessageType, Result, StreamFormat};
    pub use futures::StreamExt;
}
