//! Hook system for Claude Code SDK
//!
//! This module contains all hook-related types and traits for intercepting
//! and modifying Claude's behavior at various points in the conversation.
//!
//! # Hook Lifecycle
//!
//! Hooks can be triggered at various points:
//! - **PreToolUse** - Before a tool is executed
//! - **PostToolUse** - After a tool is executed
//! - **UserPromptSubmit** - When user submits a prompt
//! - **Stop** - When conversation stops
//! - **SubagentStop** - When a subagent stops
//! - **PreCompact** - Before compacting conversation history
//!
//! # Main Types
//!
//! - [`HookCallback`] - Main trait for implementing hooks
//! - [`HookInput`] - Strongly-typed hook input (discriminated union)
//! - [`HookJSONOutput`] - Hook output controlling Claude's behavior
//! - [`HookMatcher`] - Configuration for matching hook events
//!
//! # Example
//!
//! ```rust
//! use cc_sdk::hooks::{HookCallback, HookInput, HookJSONOutput, HookContext};
//! use cc_sdk::Error;
//! use async_trait::async_trait;
//!
//! struct MyHook;
//!
//! #[async_trait]
//! impl HookCallback for MyHook {
//!     async fn execute(
//!         &self,
//!         input: &HookInput,
//!         tool_use_id: Option<&str>,
//!         context: &HookContext,
//!     ) -> Result<HookJSONOutput, Error> {
//!         // Hook implementation
//!         Ok(HookJSONOutput::Sync(Default::default()))
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Re-export CanUseTool from permissions (it's hook-related)
pub use crate::permissions::CanUseTool;

/// Hook context
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Abort signal (future support)
    pub signal: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

// ============================================================================
// Hook Input Types (Strongly-typed hook inputs for type safety)
// ============================================================================

/// Base hook input fields present across many hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input data for PreToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Name of the tool being used
    pub tool_name: String,
    /// Input parameters for the tool
    pub tool_input: serde_json::Value,
}

/// Input data for PostToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Name of the tool that was used
    pub tool_name: String,
    /// Input parameters that were passed to the tool
    pub tool_input: serde_json::Value,
    /// Response from the tool execution
    pub tool_response: serde_json::Value,
}

/// Input data for UserPromptSubmit hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// The prompt submitted by the user
    pub prompt: String,
}

/// Input data for Stop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for SubagentStop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for PreCompact hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Session ID for this conversation
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    /// Trigger type: "manual" or "auto"
    pub trigger: String,
    /// Custom instructions for compaction (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Union type for all hook inputs (discriminated by hook_event_name)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// PreToolUse hook input
    #[serde(rename = "PreToolUse")]
    PreToolUse(PreToolUseHookInput),
    /// PostToolUse hook input
    #[serde(rename = "PostToolUse")]
    PostToolUse(PostToolUseHookInput),
    /// UserPromptSubmit hook input
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit(UserPromptSubmitHookInput),
    /// Stop hook input
    #[serde(rename = "Stop")]
    Stop(StopHookInput),
    /// SubagentStop hook input
    #[serde(rename = "SubagentStop")]
    SubagentStop(SubagentStopHookInput),
    /// PreCompact hook input
    #[serde(rename = "PreCompact")]
    PreCompact(PreCompactHookInput),
}

// ============================================================================
// Hook Output Types (Strongly-typed hook outputs for type safety)
// ============================================================================

/// Async hook output for deferred execution
///
/// When a hook returns this output, the hook execution is deferred and
/// Claude continues without waiting for the hook to complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncHookJSONOutput {
    /// Must be true to indicate async execution
    #[serde(rename = "async")]
    pub async_: bool,
    /// Optional timeout in milliseconds for async operation
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "asyncTimeout")]
    pub async_timeout: Option<u32>,
}

/// Synchronous hook output with control and decision fields
///
/// This defines the structure for hook callbacks to control execution and provide
/// feedback to Claude.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncHookJSONOutput {
    // Common control fields
    /// Whether Claude should proceed after hook execution (default: true)
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_: Option<bool>,
    /// Hide stdout from transcript mode (default: false)
    #[serde(rename = "suppressOutput", skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    /// Message shown when continue is false
    #[serde(rename = "stopReason", skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    // Decision fields
    /// Set to "block" to indicate blocking behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>, // "block" or "approve" (deprecated)
    /// Warning message displayed to the user
    #[serde(rename = "systemMessage", skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
    /// Feedback message for Claude about the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    // Hook-specific outputs
    /// Event-specific controls (e.g., permissionDecision for PreToolUse)
    #[serde(rename = "hookSpecificOutput", skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

/// Union type for hook outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookJSONOutput {
    /// Async hook output (deferred execution)
    Async(AsyncHookJSONOutput),
    /// Sync hook output (immediate execution)
    Sync(SyncHookJSONOutput),
}

/// Hook-specific output for PreToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookSpecificOutput {
    /// Permission decision: "allow", "deny", or "ask"
    #[serde(rename = "permissionDecision", skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>,
    /// Reason for the permission decision
    #[serde(rename = "permissionDecisionReason", skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    /// Updated input parameters for the tool
    #[serde(rename = "updatedInput", skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
}

/// Hook-specific output for PostToolUse events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for UserPromptSubmit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Hook-specific output for SessionStart events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartHookSpecificOutput {
    /// Additional context to provide to Claude
    #[serde(rename = "additionalContext", skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

/// Union type for hook-specific outputs (discriminated by hookEventName)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName")]
pub enum HookSpecificOutput {
    /// PreToolUse-specific output
    #[serde(rename = "PreToolUse")]
    PreToolUse(PreToolUseHookSpecificOutput),
    /// PostToolUse-specific output
    #[serde(rename = "PostToolUse")]
    PostToolUse(PostToolUseHookSpecificOutput),
    /// UserPromptSubmit-specific output
    #[serde(rename = "UserPromptSubmit")]
    UserPromptSubmit(UserPromptSubmitHookSpecificOutput),
    /// SessionStart-specific output
    #[serde(rename = "SessionStart")]
    SessionStart(SessionStartHookSpecificOutput),
}

// ============================================================================
// Hook Callback Trait (Updated for strong typing)
// ============================================================================

/// Hook callback trait with strongly-typed inputs and outputs
///
/// This trait is used to implement custom hook callbacks that can intercept
/// and modify Claude's behavior at various points in the conversation.
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// Execute the hook with strongly-typed input and output
    ///
    /// # Arguments
    ///
    /// * `input` - Strongly-typed hook input (discriminated union)
    /// * `tool_use_id` - Optional tool use identifier
    /// * `context` - Hook context with abort signal support
    ///
    /// # Returns
    ///
    /// A `HookJSONOutput` that controls Claude's behavior
    async fn execute(
        &self,
        input: &HookInput,
        tool_use_id: Option<&str>,
        context: &HookContext,
    ) -> Result<HookJSONOutput, crate::error::Error>;
}

/// Legacy hook callback trait for backward compatibility
///
/// This trait is deprecated and will be removed in v0.4.0.
/// Please migrate to the new `HookCallback` trait with strong typing.
#[deprecated(
    since = "0.3.0",
    note = "Use the new HookCallback trait with HookInput/HookJSONOutput instead"
)]
#[allow(dead_code)]
#[async_trait]
pub trait HookCallbackLegacy: Send + Sync {
    /// Execute the hook with JSON values (legacy)
    async fn execute_legacy(
        &self,
        input: &serde_json::Value,
        tool_use_id: Option<&str>,
        context: &HookContext,
    ) -> serde_json::Value;
}

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher criteria
    pub matcher: Option<serde_json::Value>,
    /// Callbacks to invoke
    pub hooks: Vec<Arc<dyn HookCallback>>,
}
