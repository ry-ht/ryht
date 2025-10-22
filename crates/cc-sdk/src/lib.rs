//! # Claude Code SDK for Rust
//!
//! A comprehensive Rust SDK for interacting with the Claude Code CLI, providing modern
//! type-safe client interfaces, session management, and MCP integration.
//!
//! ## Features
//!
//! - **Modern Client API**: Type-safe client with compile-time state verification
//! - **Session Management**: Full CRUD operations on Claude Code sessions
//! - **Settings Management**: Load and save settings with scope precedence
//! - **MCP Integration**: Built-in support for Model Context Protocol servers
//! - **Simple Query Interface**: One-shot queries with the `query` function
//! - **Streaming Support**: Async streaming of responses
//! - **Type Safety**: Strongly typed messages, errors, and state transitions
//! - **Binary Discovery**: Automatic finding of Claude installations
//! - **Hook System**: Extensible hooks for customizing Claude behavior
//!
//! ## Quick Start (Modern API - Recommended)
//!
//! ```rust,no_run
//! use cc_sdk::{ClaudeClient, Result};
//! use cc_sdk::core::ModelId;
//! use cc_sdk::types::PermissionMode;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create client with type-safe builder
//!     let client = ClaudeClient::builder()
//!         .discover_binary().await?                     // Auto-discover Claude
//!         .model(ModelId::from("claude-sonnet-4-5-20250929"))
//!         .permission_mode(PermissionMode::AcceptEdits)
//!         .configure()
//!         .connect().await?
//!         .build()?;
//!
//!     // Send messages and receive responses
//!     let mut stream = client.send("What is 2 + 2?").await?;
//!     while let Some(msg) = stream.next().await {
//!         println!("{:?}", msg?);
//!     }
//!
//!     // Clean disconnect
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Quick Start (Simple Query)
//!
//! ```rust,no_run
//! use cc_sdk::{query, Result};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut messages = query("What is 2 + 2?", None).await?;
//!
//!     while let Some(msg) = messages.next().await {
//!         println!("{:?}", msg?);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Module Organization
//!
//! The SDK is organized into focused modules:
//!
//! - [`client`] - Modern type-safe client API
//! - [`session`] - Session discovery, caching, writing, filtering, and management
//! - [`settings`] - Settings loading and saving with scope precedence
//! - [`mcp`] - Model Context Protocol integration
//! - [`binary`] - Claude binary discovery and version management
//! - [`process`] - Process registry for concurrent session tracking
//! - [`messages`] - Message and content type definitions
//! - [`options`] - Configuration and builder types
//! - [`permissions`] - Permission-related types and traits
//! - [`hooks`] - Hook system for extending Claude behavior
//! - [`requests`] - SDK Control Protocol request/response types
//! - [`error`] - Modern error types with rich context
//! - [`core`] - Core types (ModelId, SessionId, BinaryPath, Version)
//!
//! ## Prelude
//!
//! For convenience, import commonly used types with:
//!
//! ```rust
//! use cc_sdk::prelude::*;
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

// Core modules
pub mod core;
pub mod error;
pub mod result;

// Client and transport
pub mod client;
pub mod transport;

// Type definitions (organized by domain)
pub mod messages;
pub mod options;
pub mod permissions;
pub mod hooks;
pub mod requests;

// Feature modules
pub mod binary;
pub mod cache;  // Generic caching infrastructure
pub mod mcp;
pub mod process;
pub mod session;
pub mod settings;
pub mod token_tracker;
pub mod model_recommendation;
pub mod streaming;
pub mod metrics;

// Internal modules
mod types;   // Re-exports from submodules
mod internal_query;
mod message_parser;
mod query;
mod perf_utils;

// ============================================================================
// Public API Re-exports
// ============================================================================

// ----------------------------------------------------------------------------
// Modern Client API (Recommended)
// ----------------------------------------------------------------------------

/// Modern type-safe Claude client with compile-time state verification.
pub use client::{ClaudeClient, ClaudeClientBuilder, MessageStream};

// ----------------------------------------------------------------------------
// Core Types
// ----------------------------------------------------------------------------

/// Core strongly-typed identifiers and values.
pub use core::{BinaryPath, ModelId, SessionId, Version};

// ----------------------------------------------------------------------------
// Error Handling (Modern)
// ----------------------------------------------------------------------------

/// Modern error type with rich context and error source tracking.
pub use error::{
    Error,
    BinaryError,
    ClientError,
    SessionError,
    SettingsError,
    TransportError,
};

/// Modern result type (alias for `Result<T, Error>`).
pub use result::Result;

// ----------------------------------------------------------------------------
// Message Types
// ----------------------------------------------------------------------------

/// Message and content type definitions.
pub use messages::{
    Message,
    UserMessage,
    AssistantMessage,
    ContentBlock,
    ContentValue,
    TextContent,
    ThinkingContent,
    ToolUseContent,
    ToolResultContent,
    UserContent,
    AssistantContent,
};

/// Convenience aliases for message variants.
pub use messages::{
    Message::Result as ResultMessage,
    Message::System as SystemMessage,
};

// ----------------------------------------------------------------------------
// Configuration Types
// ----------------------------------------------------------------------------

/// Configuration and builder types for Claude client options.
pub use options::{
    ClaudeCodeOptions,
    ClaudeCodeOptionsBuilder,
    ControlProtocolFormat,
    McpServerConfig,
    SettingSource,
    AgentDefinition,
    SystemPrompt,
};

/// Alias for ClaudeCodeOptions (matches Python SDK naming).
pub type ClaudeAgentOptions = ClaudeCodeOptions;

/// Alias for ClaudeCodeOptionsBuilder (matches Python SDK naming).
pub type ClaudeAgentOptionsBuilder = ClaudeCodeOptionsBuilder;

// ----------------------------------------------------------------------------
// Permission Types
// ----------------------------------------------------------------------------

/// Permission-related types and traits.
pub use permissions::{
    PermissionMode,
    PermissionBehavior,
    PermissionResult,
    PermissionResultAllow,
    PermissionResultDeny,
    PermissionRuleValue,
    PermissionUpdate,
    PermissionUpdateDestination,
    PermissionUpdateType,
    ToolPermissionContext,
    CanUseTool,
};

// ----------------------------------------------------------------------------
// Hook System
// ----------------------------------------------------------------------------

/// Hook system for extending Claude behavior.
pub use hooks::{
    HookCallback,
    HookContext,
    HookMatcher,
    // Input types
    BaseHookInput,
    HookInput,
    PreToolUseHookInput,
    PostToolUseHookInput,
    UserPromptSubmitHookInput,
    StopHookInput,
    SubagentStopHookInput,
    PreCompactHookInput,
    // Output types
    HookJSONOutput,
    AsyncHookJSONOutput,
    SyncHookJSONOutput,
    HookSpecificOutput,
    PreToolUseHookSpecificOutput,
    PostToolUseHookSpecificOutput,
    UserPromptSubmitHookSpecificOutput,
    SessionStartHookSpecificOutput,
};

// ----------------------------------------------------------------------------
// SDK Control Protocol
// ----------------------------------------------------------------------------

/// SDK Control Protocol request and response types.
pub use requests::{
    SDKControlRequest,
    SDKControlInitializeRequest,
    SDKControlInterruptRequest,
    SDKControlMcpMessageRequest,
    SDKControlPermissionRequest,
    SDKControlSetPermissionModeRequest,
    SDKHookCallbackRequest,
    ControlRequest,
    ControlResponse,
};

// ----------------------------------------------------------------------------
// Utilities
// ----------------------------------------------------------------------------

/// Simple query interface for one-shot interactions.
pub use query::query;

/// Internal query builder (advanced usage).
pub use internal_query::Query;

/// Model recommendation system.
pub use model_recommendation::ModelRecommendation;

/// Performance utilities for batching and retry logic.
pub use perf_utils::{MessageBatcher, PerformanceMetrics, RetryConfig};

/// Token usage tracking and budget management.
pub use token_tracker::{BudgetLimit, BudgetManager, BudgetStatus, TokenUsageTracker};

/// Transport implementation.
pub use transport::SubprocessTransport;

/// Streaming utilities for JSONL parsing and output buffering.
pub use streaming::{JsonlReader, OutputBuffer, extract_session_id, extract_session_id_from_line, parse_jsonl_line};

/// Real-time metrics tracking for sessions.
pub use metrics::{SessionMetrics, DEFAULT_INPUT_TOKEN_COST, DEFAULT_OUTPUT_TOKEN_COST};

// ============================================================================
// Prelude Module
// ============================================================================

/// Prelude module for convenient imports.
///
/// This module re-exports the most commonly used types and functions.
/// Import everything with:
///
/// ```rust
/// use cc_sdk::prelude::*;
/// ```
pub mod prelude;
