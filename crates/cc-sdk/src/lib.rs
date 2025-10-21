//! # Claude Code SDK for Rust
//!
//! A Rust SDK for interacting with the Claude Code CLI, providing both simple query
//! and modern type-safe client interfaces.
//!
//! ## Features
//!
//! - **Modern Client API**: Type-safe client with compile-time state verification (Phase 3)
//! - **Simple Query Interface**: One-shot queries with the `query` function
//! - **Streaming Support**: Async streaming of responses
//! - **Type Safety**: Strongly typed messages, errors, and state transitions
//! - **Binary Discovery**: Automatic finding of Claude installations
//! - **Flexible Configuration**: Extensive options for customization
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

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod binary;
pub mod client;  // Phase 3: Modern client module (now public)
mod client_legacy;  // Legacy ClaudeSDKClient
// mod client_v2;  // Has compilation errors
// mod client_final;  // Has compilation errors
mod client_working;
pub mod core;
pub mod error;
mod errors;
mod interactive;
mod internal_query;
pub mod mcp;  // Phase 4: MCP integration module
mod message_parser;
pub mod model_recommendation;
mod optimized_client;
mod perf_utils;
mod query;
pub mod result;
mod sdk_mcp;
pub mod session;  // Phase 4: Session management module
pub mod settings;  // Phase 4: Settings management module
pub mod token_tracker;
pub mod transport;
mod types;

// Re-export main types and functions
// Phase 3: Modern client API (recommended)
pub use client::{ClaudeClient, ClaudeClientBuilder, MessageStream};

// Legacy clients (backward compatibility)
pub use client_legacy::ClaudeSDKClient;
// pub use client_v2::ClaudeSDKClientV2;  // Has compilation errors
// pub use client_final::ClaudeSDKClientFinal;  // Has compilation errors
pub use client_working::ClaudeSDKClientWorking;

// Phase 1: Modern error types (preferred)
pub use error::{
    Error, BinaryError, ClientError, SessionError, SettingsError, TransportError,
};

// Legacy error types (backward compatibility)
pub use errors::{Result as LegacyResult, SdkError};

// Phase 1: Modern result type (preferred)
pub use result::Result;

// Core types
pub use core::{BinaryPath, ModelId, SessionId, Version};

pub use interactive::InteractiveClient;
pub use internal_query::Query;
pub use query::query;
// Keep the old name as an alias for backward compatibility
pub use interactive::InteractiveClient as SimpleInteractiveClient;
pub use model_recommendation::ModelRecommendation;
pub use optimized_client::{ClientMode, OptimizedClient};
pub use perf_utils::{MessageBatcher, PerformanceMetrics, RetryConfig};
pub use token_tracker::{BudgetLimit, BudgetManager, BudgetStatus, TokenUsageTracker};
/// Default interactive client - the recommended client for interactive use
pub type ClaudeSDKClientDefault = InteractiveClient;
pub use types::{
    AssistantContent, AssistantMessage, ClaudeCodeOptions, ContentBlock, ContentValue,
    ControlProtocolFormat, ControlRequest, ControlResponse, McpServerConfig, Message,
    PermissionMode, ResultMessage, SystemMessage, TextContent, ThinkingContent,
    ToolResultContent, ToolUseContent, UserContent, UserMessage,
    // Permission types
    PermissionBehavior, PermissionResult, PermissionResultAllow, PermissionResultDeny,
    PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination, PermissionUpdateType,
    ToolPermissionContext, CanUseTool,
    // Hook types (v0.3.0 - strongly-typed hooks)
    HookCallback, HookContext, HookMatcher,
    // Hook Input types (strongly-typed)
    BaseHookInput, HookInput, PreToolUseHookInput, PostToolUseHookInput,
    UserPromptSubmitHookInput, StopHookInput, SubagentStopHookInput, PreCompactHookInput,
    // Hook Output types (strongly-typed)
    HookJSONOutput, AsyncHookJSONOutput, SyncHookJSONOutput,
    HookSpecificOutput, PreToolUseHookSpecificOutput, PostToolUseHookSpecificOutput,
    UserPromptSubmitHookSpecificOutput, SessionStartHookSpecificOutput,
    // SDK Control Protocol types
    SDKControlInitializeRequest, SDKControlInterruptRequest, SDKControlMcpMessageRequest,
    SDKControlPermissionRequest, SDKControlRequest, SDKControlSetPermissionModeRequest,
    SDKHookCallbackRequest,
    // Phase 2 enhancements
    SettingSource, AgentDefinition, SystemPrompt,
};

// Phase 3: Type aliases for naming consistency
/// Alias for ClaudeCodeOptions (matches Python SDK naming)
pub type ClaudeAgentOptions = ClaudeCodeOptions;
/// Alias for ClaudeCodeOptionsBuilder (matches Python SDK naming)
pub type ClaudeAgentOptionsBuilder = ClaudeCodeOptionsBuilder;

// Re-export builder
pub use types::ClaudeCodeOptionsBuilder;

// Re-export transport types for convenience
pub use transport::SubprocessTransport;

// Re-export SDK MCP types
pub use sdk_mcp::{
    SdkMcpServer, SdkMcpServerBuilder, ToolDefinition, ToolHandler, ToolInputSchema,
    ToolResult, create_simple_tool,
    ToolResultContent as SdkToolResultContent,
};

/// Prelude module for convenient imports
pub mod prelude {
    // Phase 1: Modern types (preferred)
    pub use crate::{
        BinaryPath, Error, ModelId, Result, SessionId, Version,
    };

    // Phase 3: Modern client API (recommended)
    pub use crate::{ClaudeClient, ClaudeClientBuilder, MessageStream};

    // Phase 4: Session and settings management
    pub use crate::session::{Project, Session, list_projects, list_sessions, load_session_history};
    pub use crate::settings::{ClaudeSettings, SettingsScope, load_settings, save_settings};

    // Legacy types (backward compatibility)
    pub use crate::{
        ClaudeCodeOptions, ClaudeSDKClientWorking, Message, PermissionMode,
        LegacyResult, SdkError, query,
    };

    pub use crate::binary::{find_claude_binary, discover_installations, ClaudeInstallation};

    // Error types for pattern matching
    pub use crate::error::{
        BinaryError, ClientError, SessionError, SettingsError, TransportError,
    };
}
