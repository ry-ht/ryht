//! Client module for Claude Code SDK.
//!
//! This module provides the type-safe client API using the type-state pattern.
//!
//! # Client API
//!
//! The client API (`ClaudeClient`) is the recommended way to interact with
//! Claude Code. It provides compile-time safety through type-states and an ergonomic
//! builder pattern.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::ClaudeClient;
//! use cc_sdk::core::ModelId;
//! use cc_sdk::types::PermissionMode;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> cc_sdk::Result<()> {
//!     // Create and connect client
//!     let client = ClaudeClient::builder()
//!         .discover_binary().await?
//!         .model(ModelId::from("claude-sonnet-4-5-20250929"))
//!         .permission_mode(PermissionMode::AcceptEdits)
//!         .working_directory("/path/to/project")
//!         .configure()
//!         .connect().await?
//!         .build()?;
//!
//!     // Send messages
//!     let mut stream = client.send("Hello!").await?;
//!     while let Some(message) = stream.next().await {
//!         println!("{:?}", message?);
//!     }
//!
//!     // Clean disconnect
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```

mod client;

pub use client::{ClaudeClient, ClaudeClientBuilder, MessageStream};
