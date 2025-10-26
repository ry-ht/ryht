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
//! use crate::cc::hooks::{HookCallback, HookInput, HookJSONOutput, HookContext};
//! use super::Error;
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
pub use crate::cc::permissions::CanUseTool;

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
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Name of the tool being used
    pub tool_name: String,
    /// Input parameters for the tool
    pub tool_input: serde_json::Value,
}

/// Input data for PostToolUse hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
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
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// The prompt submitted by the user
    pub prompt: String,
}

/// Input data for Stop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for SubagentStop hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input data for PreCompact hook events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Base hook input fields (session_id, transcript_path, cwd, permission_mode)
    #[serde(flatten)]
    pub base: BaseHookInput,
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

    // Decision fields - use hook_specific_output instead
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
    ) -> Result<HookJSONOutput, crate::cc::error::Error>;
}

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher criteria
    pub matcher: Option<serde_json::Value>,
    /// Callbacks to invoke
    pub hooks: Vec<Arc<dyn HookCallback>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    fn create_base_hook_input() -> BaseHookInput {
        BaseHookInput {
            session_id: "test-session-123".to_string(),
            transcript_path: "/path/to/transcript.json".to_string(),
            cwd: "/current/working/dir".to_string(),
            permission_mode: Some("ask".to_string()),
        }
    }

    fn create_base_hook_input_no_permission() -> BaseHookInput {
        BaseHookInput {
            session_id: "test-session-456".to_string(),
            transcript_path: "/another/transcript.json".to_string(),
            cwd: "/another/dir".to_string(),
            permission_mode: None,
        }
    }

    // ========================================================================
    // Hook Input Type Tests
    // ========================================================================

    #[test]
    fn test_create_pre_tool_use_hook_input() {
        let input = PreToolUseHookInput {
            base: create_base_hook_input(),
            tool_name: "Read".to_string(),
            tool_input: json!({"file_path": "/test/file.txt"}),
        };

        assert_eq!(input.base.session_id, "test-session-123");
        assert_eq!(input.tool_name, "Read");
        assert_eq!(input.tool_input["file_path"], "/test/file.txt");
    }

    #[test]
    fn test_create_post_tool_use_hook_input() {
        let input = PostToolUseHookInput {
            base: create_base_hook_input(),
            tool_name: "Bash".to_string(),
            tool_input: json!({"command": "ls -la"}),
            tool_response: json!({"output": "file1\nfile2"}),
        };

        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.tool_input["command"], "ls -la");
        assert_eq!(input.tool_response["output"], "file1\nfile2");
    }

    #[test]
    fn test_create_user_prompt_submit_hook_input() {
        let input = UserPromptSubmitHookInput {
            base: create_base_hook_input(),
            prompt: "Help me write a function".to_string(),
        };

        assert_eq!(input.prompt, "Help me write a function");
        assert_eq!(input.base.cwd, "/current/working/dir");
    }

    #[test]
    fn test_create_stop_hook_input() {
        let input = StopHookInput {
            base: create_base_hook_input(),
            stop_hook_active: true,
        };

        assert!(input.stop_hook_active);
        assert_eq!(input.base.session_id, "test-session-123");
    }

    #[test]
    fn test_create_subagent_stop_hook_input() {
        let input = SubagentStopHookInput {
            base: create_base_hook_input(),
            stop_hook_active: false,
        };

        assert!(!input.stop_hook_active);
        assert_eq!(input.base.transcript_path, "/path/to/transcript.json");
    }

    #[test]
    fn test_create_pre_compact_hook_input_manual() {
        let input = PreCompactHookInput {
            base: create_base_hook_input(),
            trigger: "manual".to_string(),
            custom_instructions: Some("Focus on important messages".to_string()),
        };

        assert_eq!(input.trigger, "manual");
        assert_eq!(
            input.custom_instructions.as_ref().unwrap(),
            "Focus on important messages"
        );
    }

    #[test]
    fn test_create_pre_compact_hook_input_auto() {
        let input = PreCompactHookInput {
            base: create_base_hook_input(),
            trigger: "auto".to_string(),
            custom_instructions: None,
        };

        assert_eq!(input.trigger, "auto");
        assert!(input.custom_instructions.is_none());
    }

    // ========================================================================
    // Hook Input Serialization Tests
    // ========================================================================

    #[test]
    fn test_pre_tool_use_hook_input_serialization() {
        let input = HookInput::PreToolUse(PreToolUseHookInput {
            base: create_base_hook_input(),
            tool_name: "Write".to_string(),
            tool_input: json!({"file_path": "/test.txt", "content": "hello"}),
        });

        let json_str = serde_json::to_string(&input).unwrap();
        assert!(json_str.contains("PreToolUse"));
        assert!(json_str.contains("Write"));
        assert!(json_str.contains("test-session-123"));

        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            HookInput::PreToolUse(pre_tool) => {
                assert_eq!(pre_tool.tool_name, "Write");
                assert_eq!(pre_tool.base.session_id, "test-session-123");
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    #[test]
    fn test_post_tool_use_hook_input_serialization() {
        let input = HookInput::PostToolUse(PostToolUseHookInput {
            base: create_base_hook_input_no_permission(),
            tool_name: "Edit".to_string(),
            tool_input: json!({"file_path": "/test.rs"}),
            tool_response: json!({"success": true}),
        });

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookInput::PostToolUse(post_tool) => {
                assert_eq!(post_tool.tool_name, "Edit");
                assert_eq!(post_tool.base.session_id, "test-session-456");
                assert!(post_tool.base.permission_mode.is_none());
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    #[test]
    fn test_user_prompt_submit_hook_input_serialization() {
        let input = HookInput::UserPromptSubmit(UserPromptSubmitHookInput {
            base: create_base_hook_input(),
            prompt: "Test prompt".to_string(),
        });

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookInput::UserPromptSubmit(submit) => {
                assert_eq!(submit.prompt, "Test prompt");
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    #[test]
    fn test_stop_hook_input_serialization() {
        let input = HookInput::Stop(StopHookInput {
            base: create_base_hook_input(),
            stop_hook_active: true,
        });

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookInput::Stop(stop) => {
                assert!(stop.stop_hook_active);
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    #[test]
    fn test_subagent_stop_hook_input_serialization() {
        let input = HookInput::SubagentStop(SubagentStopHookInput {
            base: create_base_hook_input(),
            stop_hook_active: false,
        });

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookInput::SubagentStop(subagent_stop) => {
                assert!(!subagent_stop.stop_hook_active);
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    #[test]
    fn test_pre_compact_hook_input_serialization() {
        let input = HookInput::PreCompact(PreCompactHookInput {
            base: create_base_hook_input(),
            trigger: "manual".to_string(),
            custom_instructions: Some("Custom".to_string()),
        });

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookInput::PreCompact(compact) => {
                assert_eq!(compact.trigger, "manual");
                assert_eq!(compact.custom_instructions.unwrap(), "Custom");
            }
            _ => panic!("Wrong hook type deserialized"),
        }
    }

    // ========================================================================
    // Hook Output Tests
    // ========================================================================

    #[test]
    fn test_create_async_hook_output() {
        let output = AsyncHookJSONOutput {
            async_: true,
            async_timeout: Some(5000),
        };

        assert!(output.async_);
        assert_eq!(output.async_timeout.unwrap(), 5000);
    }

    #[test]
    fn test_create_async_hook_output_no_timeout() {
        let output = AsyncHookJSONOutput {
            async_: true,
            async_timeout: None,
        };

        assert!(output.async_);
        assert!(output.async_timeout.is_none());
    }

    #[test]
    fn test_create_sync_hook_output_default() {
        let output = SyncHookJSONOutput::default();

        assert!(output.continue_.is_none());
        assert!(output.suppress_output.is_none());
        assert!(output.stop_reason.is_none());
        assert!(output.system_message.is_none());
        assert!(output.reason.is_none());
        assert!(output.hook_specific_output.is_none());
    }

    #[test]
    fn test_create_sync_hook_output_with_continue() {
        let output = SyncHookJSONOutput {
            continue_: Some(true),
            suppress_output: Some(false),
            ..Default::default()
        };

        assert_eq!(output.continue_.unwrap(), true);
        assert_eq!(output.suppress_output.unwrap(), false);
    }

    #[test]
    fn test_create_sync_hook_output_with_block() {
        let output = SyncHookJSONOutput {
            continue_: Some(false),
            system_message: Some("Tool blocked".to_string()),
            reason: Some("Security policy".to_string()),
            hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                PreToolUseHookSpecificOutput {
                    permission_decision: Some("deny".to_string()),
                    permission_decision_reason: None,
                    updated_input: None,
                }
            )),
            ..Default::default()
        };

        assert_eq!(output.continue_.unwrap(), false);
        assert_eq!(output.system_message.as_ref().unwrap(), "Tool blocked");
        assert_eq!(output.reason.as_ref().unwrap(), "Security policy");
        assert!(matches!(
            output.hook_specific_output,
            Some(HookSpecificOutput::PreToolUse(PreToolUseHookSpecificOutput {
                permission_decision: Some(ref s),
                ..
            })) if s == "deny"
        ));
    }

    #[test]
    fn test_create_sync_hook_output_with_stop_reason() {
        let output = SyncHookJSONOutput {
            continue_: Some(false),
            stop_reason: Some("User cancelled".to_string()),
            ..Default::default()
        };

        assert_eq!(output.continue_.unwrap(), false);
        assert_eq!(output.stop_reason.as_ref().unwrap(), "User cancelled");
    }

    // ========================================================================
    // Hook Specific Output Tests
    // ========================================================================

    #[test]
    fn test_pre_tool_use_specific_output_allow() {
        let output = PreToolUseHookSpecificOutput {
            permission_decision: Some("allow".to_string()),
            permission_decision_reason: Some("Safe operation".to_string()),
            updated_input: None,
        };

        assert_eq!(output.permission_decision.as_ref().unwrap(), "allow");
        assert_eq!(
            output.permission_decision_reason.as_ref().unwrap(),
            "Safe operation"
        );
        assert!(output.updated_input.is_none());
    }

    #[test]
    fn test_pre_tool_use_specific_output_deny() {
        let output = PreToolUseHookSpecificOutput {
            permission_decision: Some("deny".to_string()),
            permission_decision_reason: Some("Dangerous path".to_string()),
            updated_input: None,
        };

        assert_eq!(output.permission_decision.as_ref().unwrap(), "deny");
    }

    #[test]
    fn test_pre_tool_use_specific_output_with_updated_input() {
        let updated = json!({"file_path": "/safe/path.txt"});
        let output = PreToolUseHookSpecificOutput {
            permission_decision: Some("allow".to_string()),
            permission_decision_reason: None,
            updated_input: Some(updated.clone()),
        };

        assert_eq!(output.updated_input.unwrap(), updated);
    }

    #[test]
    fn test_post_tool_use_specific_output() {
        let output = PostToolUseHookSpecificOutput {
            additional_context: Some("Operation completed successfully".to_string()),
        };

        assert_eq!(
            output.additional_context.as_ref().unwrap(),
            "Operation completed successfully"
        );
    }

    #[test]
    fn test_user_prompt_submit_specific_output() {
        let output = UserPromptSubmitHookSpecificOutput {
            additional_context: Some("Context from hook".to_string()),
        };

        assert_eq!(
            output.additional_context.as_ref().unwrap(),
            "Context from hook"
        );
    }

    #[test]
    fn test_session_start_specific_output() {
        let output = SessionStartHookSpecificOutput {
            additional_context: Some("Session initialized".to_string()),
        };

        assert_eq!(
            output.additional_context.as_ref().unwrap(),
            "Session initialized"
        );
    }

    // ========================================================================
    // Hook Specific Output Union Tests
    // ========================================================================

    #[test]
    fn test_hook_specific_output_pre_tool_use_variant() {
        let output = HookSpecificOutput::PreToolUse(PreToolUseHookSpecificOutput {
            permission_decision: Some("ask".to_string()),
            permission_decision_reason: None,
            updated_input: None,
        });

        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("PreToolUse"));
        assert!(json_str.contains("ask"));
    }

    #[test]
    fn test_hook_specific_output_post_tool_use_variant() {
        let output = HookSpecificOutput::PostToolUse(PostToolUseHookSpecificOutput {
            additional_context: Some("Success".to_string()),
        });

        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("PostToolUse"));
        assert!(json_str.contains("Success"));
    }

    // ========================================================================
    // Hook JSON Output Tests
    // ========================================================================

    #[test]
    fn test_hook_json_output_async_variant() {
        let output = HookJSONOutput::Async(AsyncHookJSONOutput {
            async_: true,
            async_timeout: Some(3000),
        });

        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("\"async\":true"));
        assert!(json_str.contains("3000"));

        let deserialized: HookJSONOutput = serde_json::from_str(&json_str).unwrap();
        match deserialized {
            HookJSONOutput::Async(async_out) => {
                assert!(async_out.async_);
                assert_eq!(async_out.async_timeout.unwrap(), 3000);
            }
            _ => panic!("Wrong output type"),
        }
    }

    #[test]
    fn test_hook_json_output_sync_variant() {
        let output = HookJSONOutput::Sync(SyncHookJSONOutput {
            continue_: Some(true),
            suppress_output: Some(false),
            ..Default::default()
        });

        let json_str = serde_json::to_string(&output).unwrap();
        let deserialized: HookJSONOutput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookJSONOutput::Sync(sync_out) => {
                assert_eq!(sync_out.continue_.unwrap(), true);
                assert_eq!(sync_out.suppress_output.unwrap(), false);
            }
            _ => panic!("Wrong output type"),
        }
    }

    #[test]
    fn test_sync_hook_output_with_hook_specific() {
        let output = SyncHookJSONOutput {
            continue_: Some(true),
            hook_specific_output: Some(HookSpecificOutput::PreToolUse(
                PreToolUseHookSpecificOutput {
                    permission_decision: Some("allow".to_string()),
                    permission_decision_reason: None,
                    updated_input: None,
                },
            )),
            ..Default::default()
        };

        let json_str = serde_json::to_string(&output).unwrap();
        assert!(json_str.contains("hookSpecificOutput"));
        assert!(json_str.contains("allow"));
    }

    // ========================================================================
    // Hook Context Tests
    // ========================================================================

    #[test]
    fn test_hook_context_with_signal() {
        let signal: Arc<dyn std::any::Any + Send + Sync> = Arc::new("test_signal".to_string());
        let context = HookContext {
            signal: Some(signal.clone()),
        };

        assert!(context.signal.is_some());
    }

    #[test]
    fn test_hook_context_without_signal() {
        let context = HookContext { signal: None };

        assert!(context.signal.is_none());
    }

    #[test]
    fn test_hook_context_clone() {
        let signal: Arc<dyn std::any::Any + Send + Sync> = Arc::new(42u32);
        let context = HookContext {
            signal: Some(signal),
        };

        let cloned = context.clone();
        assert!(cloned.signal.is_some());
    }

    // ========================================================================
    // Hook Matcher Tests
    // ========================================================================

    #[test]
    fn test_hook_matcher_with_matcher() {
        let matcher = HookMatcher {
            matcher: Some(json!({"tool_name": "Read"})),
            hooks: vec![],
        };

        assert!(matcher.matcher.is_some());
        assert_eq!(matcher.matcher.unwrap()["tool_name"], "Read");
        assert_eq!(matcher.hooks.len(), 0);
    }

    #[test]
    fn test_hook_matcher_without_matcher() {
        let matcher = HookMatcher {
            matcher: None,
            hooks: vec![],
        };

        assert!(matcher.matcher.is_none());
    }

    #[test]
    fn test_hook_matcher_clone() {
        let matcher = HookMatcher {
            matcher: Some(json!({"event": "PreToolUse"})),
            hooks: vec![],
        };

        let cloned = matcher.clone();
        assert_eq!(
            cloned.matcher.as_ref().unwrap()["event"],
            "PreToolUse"
        );
    }

    // ========================================================================
    // Mock Hook Callback Tests
    // ========================================================================

    struct TestHook {
        should_continue: bool,
    }

    #[async_trait]
    impl HookCallback for TestHook {
        async fn execute(
            &self,
            _input: &HookInput,
            _tool_use_id: Option<&str>,
            _context: &HookContext,
        ) -> Result<HookJSONOutput, crate::cc::error::Error> {
            Ok(HookJSONOutput::Sync(SyncHookJSONOutput {
                continue_: Some(self.should_continue),
                ..Default::default()
            }))
        }
    }

    #[tokio::test]
    async fn test_mock_hook_callback_continue() {
        let hook = TestHook {
            should_continue: true,
        };
        let input = HookInput::PreToolUse(PreToolUseHookInput {
            base: create_base_hook_input(),
            tool_name: "Test".to_string(),
            tool_input: json!({}),
        });
        let context = HookContext { signal: None };

        let result = hook.execute(&input, None, &context).await.unwrap();
        match result {
            HookJSONOutput::Sync(sync_out) => {
                assert_eq!(sync_out.continue_.unwrap(), true);
            }
            _ => panic!("Expected sync output"),
        }
    }

    #[tokio::test]
    async fn test_mock_hook_callback_stop() {
        let hook = TestHook {
            should_continue: false,
        };
        let input = HookInput::Stop(StopHookInput {
            base: create_base_hook_input(),
            stop_hook_active: true,
        });
        let context = HookContext { signal: None };

        let result = hook.execute(&input, Some("tool-123"), &context)
            .await
            .unwrap();
        match result {
            HookJSONOutput::Sync(sync_out) => {
                assert_eq!(sync_out.continue_.unwrap(), false);
            }
            _ => panic!("Expected sync output"),
        }
    }

    struct ErrorHook;

    #[async_trait]
    impl HookCallback for ErrorHook {
        async fn execute(
            &self,
            _input: &HookInput,
            _tool_use_id: Option<&str>,
            _context: &HookContext,
        ) -> Result<HookJSONOutput, crate::cc::error::Error> {
            Err(crate::cc::error::ClientError::HookFailed {
                hook_name: "ErrorHook".to_string(),
                reason: "Test error".to_string(),
                source: None,
            }
            .into())
        }
    }

    #[tokio::test]
    async fn test_mock_hook_callback_error() {
        let hook = ErrorHook;
        let input = HookInput::PreToolUse(PreToolUseHookInput {
            base: create_base_hook_input(),
            tool_name: "Test".to_string(),
            tool_input: json!({}),
        });
        let context = HookContext { signal: None };

        let result = hook.execute(&input, None, &context).await;
        assert!(result.is_err());
        match result {
            Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::HookFailed {
                hook_name,
                reason,
                ..
            })) => {
                assert_eq!(hook_name, "ErrorHook");
                assert_eq!(reason, "Test error");
            }
            _ => panic!("Expected HookFailed error"),
        }
    }

    struct AsyncReturningHook;

    #[async_trait]
    impl HookCallback for AsyncReturningHook {
        async fn execute(
            &self,
            _input: &HookInput,
            _tool_use_id: Option<&str>,
            _context: &HookContext,
        ) -> Result<HookJSONOutput, crate::cc::error::Error> {
            Ok(HookJSONOutput::Async(AsyncHookJSONOutput {
                async_: true,
                async_timeout: Some(10000),
            }))
        }
    }

    #[tokio::test]
    async fn test_mock_hook_callback_async_output() {
        let hook = AsyncReturningHook;
        let input = HookInput::UserPromptSubmit(UserPromptSubmitHookInput {
            base: create_base_hook_input(),
            prompt: "test".to_string(),
        });
        let context = HookContext { signal: None };

        let result = hook.execute(&input, None, &context).await.unwrap();
        match result {
            HookJSONOutput::Async(async_out) => {
                assert!(async_out.async_);
                assert_eq!(async_out.async_timeout.unwrap(), 10000);
            }
            _ => panic!("Expected async output"),
        }
    }

    // ========================================================================
    // Complex Scenario Tests
    // ========================================================================

    #[test]
    fn test_pre_tool_use_with_complete_hook_specific() {
        let hook_specific = HookSpecificOutput::PreToolUse(PreToolUseHookSpecificOutput {
            permission_decision: Some("allow".to_string()),
            permission_decision_reason: Some("Verified safe".to_string()),
            updated_input: Some(json!({"file_path": "/sanitized/path.txt"})),
        });

        let output = SyncHookJSONOutput {
            continue_: Some(true),
            suppress_output: Some(false),
            hook_specific_output: Some(hook_specific),
            ..Default::default()
        };

        let json_str = serde_json::to_string(&output).unwrap();
        let deserialized: SyncHookJSONOutput = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.continue_.unwrap(), true);
        assert!(deserialized.hook_specific_output.is_some());

        match deserialized.hook_specific_output.unwrap() {
            HookSpecificOutput::PreToolUse(pre_tool) => {
                assert_eq!(pre_tool.permission_decision.unwrap(), "allow");
                assert_eq!(
                    pre_tool.permission_decision_reason.unwrap(),
                    "Verified safe"
                );
                assert_eq!(
                    pre_tool.updated_input.unwrap()["file_path"],
                    "/sanitized/path.txt"
                );
            }
            _ => panic!("Wrong hook specific output type"),
        }
    }

    #[test]
    fn test_post_tool_use_with_additional_context() {
        let hook_specific = HookSpecificOutput::PostToolUse(PostToolUseHookSpecificOutput {
            additional_context: Some("File was modified successfully".to_string()),
        });

        let output = HookJSONOutput::Sync(SyncHookJSONOutput {
            continue_: Some(true),
            hook_specific_output: Some(hook_specific),
            ..Default::default()
        });

        let json_str = serde_json::to_string(&output).unwrap();
        let deserialized: HookJSONOutput = serde_json::from_str(&json_str).unwrap();

        match deserialized {
            HookJSONOutput::Sync(sync_out) => {
                match sync_out.hook_specific_output.unwrap() {
                    HookSpecificOutput::PostToolUse(post_tool) => {
                        assert_eq!(
                            post_tool.additional_context.unwrap(),
                            "File was modified successfully"
                        );
                    }
                    _ => panic!("Wrong hook specific output type"),
                }
            }
            _ => panic!("Wrong hook output type"),
        }
    }

    #[test]
    fn test_hook_matcher_with_multiple_hooks() {
        let hook1: Arc<dyn HookCallback> = Arc::new(TestHook {
            should_continue: true,
        });
        let hook2: Arc<dyn HookCallback> = Arc::new(TestHook {
            should_continue: false,
        });

        let matcher = HookMatcher {
            matcher: Some(json!({"tool_name": ".*"})),
            hooks: vec![hook1, hook2],
        };

        assert_eq!(matcher.hooks.len(), 2);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property Test Strategies
    // ========================================================================

    fn base_hook_input_strategy() -> impl Strategy<Value = BaseHookInput> {
        (
            "[a-z0-9-]{8,32}",
            "/[a-z/]{5,30}\\.json",
            "/[a-z/]{5,30}",
            proptest::option::of("[a-z]{3,10}"),
        )
            .prop_map(|(session_id, transcript_path, cwd, permission_mode)| BaseHookInput {
                session_id,
                transcript_path,
                cwd,
                permission_mode,
            })
    }

    fn pre_tool_use_input_strategy() -> impl Strategy<Value = PreToolUseHookInput> {
        (
            base_hook_input_strategy(),
            "[A-Z][a-z]{2,10}",
            prop::collection::vec(("[a-z_]{3,10}", "[a-z0-9]{1,20}"), 0..5),
        )
            .prop_map(|(base, tool_name, params)| {
                let tool_input = params
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect::<serde_json::Map<_, _>>();
                PreToolUseHookInput {
                    base,
                    tool_name,
                    tool_input: serde_json::Value::Object(tool_input),
                }
            })
    }

    fn post_tool_use_input_strategy() -> impl Strategy<Value = PostToolUseHookInput> {
        (
            base_hook_input_strategy(),
            "[A-Z][a-z]{2,10}",
            prop::collection::vec(("[a-z_]{3,10}", "[a-z0-9]{1,20}"), 0..5),
            prop::collection::vec(("[a-z_]{3,10}", "[a-z0-9]{1,20}"), 0..5),
        )
            .prop_map(|(base, tool_name, input_params, response_params)| {
                let tool_input = input_params
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect::<serde_json::Map<_, _>>();
                let tool_response = response_params
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect::<serde_json::Map<_, _>>();
                PostToolUseHookInput {
                    base,
                    tool_name,
                    tool_input: serde_json::Value::Object(tool_input),
                    tool_response: serde_json::Value::Object(tool_response),
                }
            })
    }

    fn user_prompt_submit_input_strategy() -> impl Strategy<Value = UserPromptSubmitHookInput> {
        (base_hook_input_strategy(), "[a-zA-Z0-9 ]{5,50}").prop_map(|(base, prompt)| {
            UserPromptSubmitHookInput { base, prompt }
        })
    }

    fn stop_hook_input_strategy() -> impl Strategy<Value = StopHookInput> {
        (base_hook_input_strategy(), any::<bool>())
            .prop_map(|(base, stop_hook_active)| StopHookInput {
                base,
                stop_hook_active,
            })
    }

    fn pre_compact_input_strategy() -> impl Strategy<Value = PreCompactHookInput> {
        (
            base_hook_input_strategy(),
            prop::sample::select(vec!["manual", "auto"]),
            proptest::option::of("[a-zA-Z0-9 ]{5,50}"),
        )
            .prop_map(|(base, trigger, custom_instructions)| PreCompactHookInput {
                base,
                trigger: trigger.to_string(),
                custom_instructions,
            })
    }

    // ========================================================================
    // Property Tests for Serialization Roundtrips
    // ========================================================================

    proptest! {
        #[test]
        fn test_pre_tool_use_roundtrip(input in pre_tool_use_input_strategy()) {
            let hook_input = HookInput::PreToolUse(input);
            let json_str = serde_json::to_string(&hook_input).unwrap();
            let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

            match (&hook_input, &deserialized) {
                (HookInput::PreToolUse(orig), HookInput::PreToolUse(deser)) => {
                    prop_assert_eq!(&orig.base.session_id, &deser.base.session_id);
                    prop_assert_eq!(&orig.tool_name, &deser.tool_name);
                }
                _ => panic!("Deserialization changed variant"),
            }
        }

        #[test]
        fn test_post_tool_use_roundtrip(input in post_tool_use_input_strategy()) {
            let hook_input = HookInput::PostToolUse(input);
            let json_str = serde_json::to_string(&hook_input).unwrap();
            let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

            match (&hook_input, &deserialized) {
                (HookInput::PostToolUse(orig), HookInput::PostToolUse(deser)) => {
                    prop_assert_eq!(&orig.base.session_id, &deser.base.session_id);
                    prop_assert_eq!(&orig.tool_name, &deser.tool_name);
                }
                _ => panic!("Deserialization changed variant"),
            }
        }

        #[test]
        fn test_user_prompt_submit_roundtrip(input in user_prompt_submit_input_strategy()) {
            let hook_input = HookInput::UserPromptSubmit(input);
            let json_str = serde_json::to_string(&hook_input).unwrap();
            let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

            match (&hook_input, &deserialized) {
                (HookInput::UserPromptSubmit(orig), HookInput::UserPromptSubmit(deser)) => {
                    prop_assert_eq!(&orig.prompt, &deser.prompt);
                }
                _ => panic!("Deserialization changed variant"),
            }
        }

        #[test]
        fn test_stop_hook_roundtrip(input in stop_hook_input_strategy()) {
            let hook_input = HookInput::Stop(input);
            let json_str = serde_json::to_string(&hook_input).unwrap();
            let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

            match (&hook_input, &deserialized) {
                (HookInput::Stop(orig), HookInput::Stop(deser)) => {
                    prop_assert_eq!(orig.stop_hook_active, deser.stop_hook_active);
                }
                _ => panic!("Deserialization changed variant"),
            }
        }

        #[test]
        fn test_pre_compact_roundtrip(input in pre_compact_input_strategy()) {
            let hook_input = HookInput::PreCompact(input);
            let json_str = serde_json::to_string(&hook_input).unwrap();
            let deserialized: HookInput = serde_json::from_str(&json_str).unwrap();

            match (&hook_input, &deserialized) {
                (HookInput::PreCompact(orig), HookInput::PreCompact(deser)) => {
                    prop_assert_eq!(&orig.trigger, &deser.trigger);
                }
                _ => panic!("Deserialization changed variant"),
            }
        }

        #[test]
        fn test_async_output_roundtrip(async_ in any::<bool>(), timeout in proptest::option::of(0u32..60000u32)) {
            let output = HookJSONOutput::Async(AsyncHookJSONOutput {
                async_,
                async_timeout: timeout,
            });

            let json_str = serde_json::to_string(&output).unwrap();
            let deserialized: HookJSONOutput = serde_json::from_str(&json_str).unwrap();

            match deserialized {
                HookJSONOutput::Async(async_out) => {
                    prop_assert_eq!(async_out.async_, async_);
                    prop_assert_eq!(async_out.async_timeout, timeout);
                }
                _ => panic!("Wrong output type"),
            }
        }

        #[test]
        fn test_sync_output_roundtrip(
            continue_ in proptest::option::of(any::<bool>()),
            suppress in proptest::option::of(any::<bool>()),
            decision in proptest::option::of(prop::sample::select(vec!["block", "approve"]))
        ) {
            let output = HookJSONOutput::Sync(SyncHookJSONOutput {
                continue_,
                suppress_output: suppress,
                decision: decision.map(|s| s.to_string()),
                ..Default::default()
            });

            let json_str = serde_json::to_string(&output).unwrap();
            let deserialized: HookJSONOutput = serde_json::from_str(&json_str).unwrap();

            match deserialized {
                HookJSONOutput::Sync(sync_out) => {
                    prop_assert_eq!(sync_out.continue_, continue_);
                    prop_assert_eq!(sync_out.suppress_output, suppress);
                }
                _ => panic!("Wrong output type"),
            }
        }
    }

    // ========================================================================
    // Property Tests for Valid Hook Type Strings
    // ========================================================================

    proptest! {
        #[test]
        fn test_hook_type_string_parsing(
            hook_type in prop::sample::select(vec![
                "PreToolUse",
                "PostToolUse",
                "UserPromptSubmit",
                "Stop",
                "SubagentStop",
                "PreCompact",
            ])
        ) {
            // Add required fields based on hook type
            let complete_json = match hook_type {
                "PreToolUse" => format!(
                    r#"{{"hook_event_name": "{}", "session_id": "test", "transcript_path": "/test", "cwd": "/test", "tool_name": "Test", "tool_input": {{}}}}"#,
                    hook_type
                ),
                "PostToolUse" => format!(
                    r#"{{"hook_event_name": "{}", "session_id": "test", "transcript_path": "/test", "cwd": "/test", "tool_name": "Test", "tool_input": {{}}, "tool_response": {{}}}}"#,
                    hook_type
                ),
                "UserPromptSubmit" => format!(
                    r#"{{"hook_event_name": "{}", "session_id": "test", "transcript_path": "/test", "cwd": "/test", "prompt": "test"}}"#,
                    hook_type
                ),
                "Stop" | "SubagentStop" => format!(
                    r#"{{"hook_event_name": "{}", "session_id": "test", "transcript_path": "/test", "cwd": "/test", "stop_hook_active": true}}"#,
                    hook_type
                ),
                "PreCompact" => format!(
                    r#"{{"hook_event_name": "{}", "session_id": "test", "transcript_path": "/test", "cwd": "/test", "trigger": "manual"}}"#,
                    hook_type
                ),
                _ => unreachable!(),
            };

            let result: Result<HookInput, _> = serde_json::from_str(&complete_json);
            prop_assert!(result.is_ok(), "Failed to parse hook type: {}", hook_type);
        }
    }
}
