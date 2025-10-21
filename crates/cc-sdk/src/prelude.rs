//! Prelude module for convenient imports.
//!
//! This module re-exports the most commonly used types and functions from the SDK.
//! It is designed to provide a minimal but complete set of imports for typical usage.
//!
//! # Usage
//!
//! ```rust
//! use cc_sdk::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // All essential types are now in scope
//!     let client = ClaudeClient::builder()
//!         .discover_binary().await?
//!         .model(ModelId::from("claude-sonnet-4-5-20250929"))
//!         .permission_mode(PermissionMode::AcceptEdits)
//!         .configure()
//!         .connect().await?
//!         .build()?;
//!
//!     // ... use client
//!     Ok(())
//! }
//! ```
//!
//! # What's Included
//!
//! The prelude includes:
//!
//! - **Client API**: `ClaudeClient`, `ClaudeClientBuilder`, `MessageStream`
//! - **Core Types**: `ModelId`, `SessionId`, `BinaryPath`, `Version`
//! - **Error Handling**: `Error`, `Result` and all error variants
//! - **Messages**: `Message`, `UserMessage`, `AssistantMessage`, content types
//! - **Configuration**: `ClaudeCodeOptions`, `PermissionMode`
//! - **Session Management**: Session discovery, loading, and basic types
//! - **Settings Management**: Settings loading and saving
//! - **Binary Discovery**: `find_claude_binary`, `discover_installations`
//! - **Simple Query**: `query` function for one-shot interactions
//!
//! # Not Included
//!
//! Advanced features are intentionally excluded to keep the prelude focused:
//!
//! - Hook system types (use `cc_sdk::hooks::*`)
//! - MCP integration (use `cc_sdk::mcp::*`)
//! - SDK Control Protocol types (use `cc_sdk::requests::*`)
//! - Performance utilities (use `cc_sdk::perf_utils::*`)
//! - Token tracking (use `cc_sdk::token_tracker::*`)
//! - Advanced session features (use `cc_sdk::session::*`)

// ============================================================================
// Client API (Most Important)
// ============================================================================

/// Modern type-safe Claude client.
pub use crate::client::ClaudeClient;

/// Builder for creating a Claude client with type-safe state transitions.
pub use crate::client::ClaudeClientBuilder;

/// Stream of messages from Claude.
pub use crate::client::MessageStream;

// ============================================================================
// Core Types
// ============================================================================

/// Strongly-typed binary path.
pub use crate::core::BinaryPath;

/// Strongly-typed model identifier.
pub use crate::core::ModelId;

/// Strongly-typed session identifier.
pub use crate::core::SessionId;

/// Semantic version type.
pub use crate::core::Version;

// ============================================================================
// Error Handling
// ============================================================================

/// Main error type for the SDK.
pub use crate::error::Error;

/// Binary discovery and execution errors.
pub use crate::error::BinaryError;

/// Client operation errors.
pub use crate::error::ClientError;

/// Session management errors.
pub use crate::error::SessionError;

/// Settings loading/saving errors.
pub use crate::error::SettingsError;

/// Transport and communication errors.
pub use crate::error::TransportError;

/// Result type alias for SDK operations.
pub use crate::result::Result;

// ============================================================================
// Message Types
// ============================================================================

/// Top-level message type.
pub use crate::messages::Message;

/// User message containing prompts and content.
pub use crate::messages::UserMessage;

/// Assistant message containing responses.
pub use crate::messages::AssistantMessage;

/// Content blocks (text, thinking, tool use, tool result).
pub use crate::messages::ContentBlock;

/// Content value enumeration.
pub use crate::messages::ContentValue;

/// Text content block.
pub use crate::messages::TextContent;

/// Thinking content block.
pub use crate::messages::ThinkingContent;

/// Tool use content block.
pub use crate::messages::ToolUseContent;

/// Tool result content block.
pub use crate::messages::ToolResultContent;

/// User-specific content wrapper.
pub use crate::messages::UserContent;

/// Assistant-specific content wrapper.
pub use crate::messages::AssistantContent;

// ============================================================================
// Configuration Types
// ============================================================================

/// Main configuration options for Claude client.
pub use crate::options::ClaudeCodeOptions;

/// Builder for ClaudeCodeOptions.
pub use crate::options::ClaudeCodeOptionsBuilder;

/// Permission mode for tool and file access.
pub use crate::permissions::PermissionMode;

/// Agent definition (name, version, etc.).
pub use crate::options::AgentDefinition;

/// System prompt configuration.
pub use crate::options::SystemPrompt;

// ============================================================================
// Binary Discovery
// ============================================================================

/// Find the Claude binary automatically.
pub use crate::binary::find_claude_binary;

/// Discover all Claude installations on the system.
pub use crate::binary::discover_installations;

/// Information about a Claude installation.
pub use crate::binary::ClaudeInstallation;

// ============================================================================
// Simple Query Interface
// ============================================================================

/// Perform a one-shot query to Claude.
///
/// This is the simplest way to interact with Claude for basic use cases.
///
/// # Example
///
/// ```no_run
/// use cc_sdk::prelude::*;
/// use futures::StreamExt;
///
/// # async fn example() -> Result<()> {
/// let mut stream = query("What is 2 + 2?", None).await?;
/// while let Some(msg) = stream.next().await {
///     println!("{:?}", msg?);
/// }
/// # Ok(())
/// # }
/// ```
pub use crate::query::query;

// ============================================================================
// Session Management (Basic)
// ============================================================================

/// Project information.
pub use crate::session::Project;

/// Session information.
pub use crate::session::Session;

/// List all projects.
pub use crate::session::list_projects;

/// List sessions for a project.
pub use crate::session::list_sessions;

/// Load a session's message history.
pub use crate::session::load_session_history;

/// Find a project by its path.
pub use crate::session::find_project_by_path;

// ============================================================================
// Settings Management (Basic)
// ============================================================================

/// Claude settings structure.
pub use crate::settings::ClaudeSettings;

/// Settings scope (User, Project, Local).
pub use crate::settings::SettingsScope;

/// Load settings from specified scopes.
pub use crate::settings::load_settings;

/// Save settings to a scope.
pub use crate::settings::save_settings;
