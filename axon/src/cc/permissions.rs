//! Permission types for Claude Code SDK
//!
//! This module contains all permission-related types used for controlling
//! tool execution and managing permission policies.
//!
//! # Permission Modes
//!
//! - [`PermissionMode`] - Global permission mode (Default, AcceptEdits, Plan, BypassPermissions)
//!
//! # Permission Rules and Updates
//!
//! - [`PermissionUpdate`] - Updates to permission configuration
//! - [`PermissionRuleValue`] - Individual permission rule
//! - [`PermissionBehavior`] - Allow/Deny/Ask behavior
//!
//! # Permission Checking
//!
//! - [`CanUseTool`] - Trait for implementing permission checks
//! - [`ToolPermissionContext`] - Context for permission decisions
//! - [`PermissionResult`] - Result of permission check
//!
//! # Example
//!
//! ```rust
//! use crate::cc::permissions::PermissionMode;
//!
//! let mode = PermissionMode::AcceptEdits;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - CLI prompts for dangerous tools
    Default,
    /// Auto-accept file edits
    AcceptEdits,
    /// Plan mode - for planning tasks
    Plan,
    /// Allow all tools without prompting (use with caution)
    BypassPermissions,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
}

impl PermissionMode {
    /// Check if this mode automatically allows file edits.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::cc::permissions::PermissionMode;
    ///
    /// assert!(PermissionMode::AcceptEdits.allows_edits());
    /// assert!(!PermissionMode::Default.allows_edits());
    /// ```
    #[inline]
    pub fn allows_edits(self) -> bool {
        matches!(self, Self::AcceptEdits | Self::BypassPermissions)
    }

    /// Check if this mode allows all tools without prompting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::cc::permissions::PermissionMode;
    ///
    /// assert!(PermissionMode::BypassPermissions.bypasses_all());
    /// assert!(!PermissionMode::Default.bypasses_all());
    /// ```
    #[inline]
    pub fn bypasses_all(self) -> bool {
        matches!(self, Self::BypassPermissions)
    }

    /// Check if this mode requires user prompts.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::cc::permissions::PermissionMode;
    ///
    /// assert!(PermissionMode::Default.requires_prompts());
    /// assert!(!PermissionMode::BypassPermissions.requires_prompts());
    /// ```
    #[inline]
    pub fn requires_prompts(self) -> bool {
        matches!(self, Self::Default | Self::Plan)
    }
}

/// Permission update destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    /// User settings
    UserSettings,
    /// Project settings
    ProjectSettings,
    /// Local settings
    LocalSettings,
    /// Session
    Session,
}

/// Permission behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionBehavior {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
    /// Ask the user
    Ask,
}

/// Permission rule value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleValue {
    /// Tool name
    pub tool_name: String,
    /// Rule content
    pub rule_content: Option<String>,
}

/// Permission update type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateType {
    /// Add rules
    AddRules,
    /// Replace rules
    ReplaceRules,
    /// Remove rules
    RemoveRules,
    /// Set mode
    SetMode,
    /// Add directories
    AddDirectories,
    /// Remove directories
    RemoveDirectories,
}

/// Permission update
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    /// Update type
    #[serde(rename = "type")]
    pub update_type: PermissionUpdateType,
    /// Rules to update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRuleValue>>,
    /// Behavior to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    /// Mode to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionMode>,
    /// Directories to add/remove
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    /// Destination for the update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<PermissionUpdateDestination>,
}

/// Tool permission context
#[derive(Debug, Clone)]
pub struct ToolPermissionContext {
    /// Abort signal (future support)
    pub signal: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Permission suggestions from CLI
    pub suggestions: Vec<PermissionUpdate>,
}

/// Permission result - Allow
#[derive(Debug, Clone)]
pub struct PermissionResultAllow {
    /// Updated input parameters
    pub updated_input: Option<serde_json::Value>,
    /// Updated permissions
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

/// Permission result - Deny
#[derive(Debug, Clone)]
pub struct PermissionResultDeny {
    /// Denial message
    pub message: String,
    /// Whether to interrupt the conversation
    pub interrupt: bool,
}

/// Permission result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Allow the tool use
    Allow(PermissionResultAllow),
    /// Deny the tool use
    Deny(PermissionResultDeny),
}

/// Tool permission callback trait
#[async_trait]
pub trait CanUseTool: Send + Sync {
    /// Check if a tool can be used
    async fn can_use_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        context: &ToolPermissionContext,
    ) -> PermissionResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode_serialization() {
        let mode = PermissionMode::AcceptEdits;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, r#""acceptEdits""#);

        let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, mode);

        // Test Plan mode
        let plan_mode = PermissionMode::Plan;
        let plan_json = serde_json::to_string(&plan_mode).unwrap();
        assert_eq!(plan_json, r#""plan""#);

        let plan_deserialized: PermissionMode = serde_json::from_str(&plan_json).unwrap();
        assert_eq!(plan_deserialized, plan_mode);
    }

    // Property-based tests
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // Strategy for generating PermissionMode
        fn permission_mode_strategy() -> impl Strategy<Value = PermissionMode> {
            prop_oneof![
                Just(PermissionMode::Default),
                Just(PermissionMode::AcceptEdits),
                Just(PermissionMode::Plan),
                Just(PermissionMode::BypassPermissions),
            ]
        }

        // Strategy for generating PermissionBehavior
        fn permission_behavior_strategy() -> impl Strategy<Value = PermissionBehavior> {
            prop_oneof![
                Just(PermissionBehavior::Allow),
                Just(PermissionBehavior::Deny),
                Just(PermissionBehavior::Ask),
            ]
        }

        // Strategy for generating PermissionUpdateDestination
        fn permission_destination_strategy() -> impl Strategy<Value = PermissionUpdateDestination> {
            prop_oneof![
                Just(PermissionUpdateDestination::UserSettings),
                Just(PermissionUpdateDestination::ProjectSettings),
                Just(PermissionUpdateDestination::LocalSettings),
                Just(PermissionUpdateDestination::Session),
            ]
        }

        proptest! {
            // PermissionMode property tests
            #[test]
            fn permission_mode_serialization_roundtrip(mode in permission_mode_strategy()) {
                let json = serde_json::to_string(&mode).unwrap();
                let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(mode, deserialized);
            }

            #[test]
            fn permission_mode_allows_edits_consistency(mode in permission_mode_strategy()) {
                let allows = mode.allows_edits();
                let expected = matches!(mode, PermissionMode::AcceptEdits | PermissionMode::BypassPermissions);
                prop_assert_eq!(allows, expected);
            }

            #[test]
            fn permission_mode_bypasses_all_exclusive(mode in permission_mode_strategy()) {
                let bypasses = mode.bypasses_all();
                let expected = matches!(mode, PermissionMode::BypassPermissions);
                prop_assert_eq!(bypasses, expected);

                // If it bypasses all, it should also allow edits
                if bypasses {
                    prop_assert!(mode.allows_edits());
                }
            }

            #[test]
            fn permission_mode_requires_prompts_consistency(mode in permission_mode_strategy()) {
                let requires = mode.requires_prompts();
                let expected = matches!(mode, PermissionMode::Default | PermissionMode::Plan);
                prop_assert_eq!(requires, expected);

                // If it bypasses all, it shouldn't require prompts
                if mode.bypasses_all() {
                    prop_assert!(!requires);
                }
            }

            #[test]
            fn permission_mode_mutual_exclusivity(mode in permission_mode_strategy()) {
                // A mode cannot both bypass all and require prompts
                let bypasses = mode.bypasses_all();
                let requires = mode.requires_prompts();
                prop_assert!(!(bypasses && requires));
            }

            // PermissionBehavior property tests
            #[test]
            fn permission_behavior_serialization_roundtrip(behavior in permission_behavior_strategy()) {
                let json = serde_json::to_string(&behavior).unwrap();
                let deserialized: PermissionBehavior = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(behavior, deserialized);
            }

            // PermissionUpdateDestination property tests
            #[test]
            fn permission_destination_serialization_roundtrip(dest in permission_destination_strategy()) {
                let json = serde_json::to_string(&dest).unwrap();
                let deserialized: PermissionUpdateDestination = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(dest, deserialized);
            }

            // PermissionRuleValue property tests
            #[test]
            fn permission_rule_value_serialization_roundtrip(
                tool_name in "[a-zA-Z]{1,30}",
                rule_content in prop::option::of("\\PC{1,100}")
            ) {
                let rule = PermissionRuleValue {
                    tool_name,
                    rule_content,
                };
                let json = serde_json::to_string(&rule).unwrap();
                let deserialized: PermissionRuleValue = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(rule.tool_name, deserialized.tool_name);
                prop_assert_eq!(rule.rule_content, deserialized.rule_content);
            }

            // PermissionUpdate property tests
            #[test]
            fn permission_update_set_mode_serialization(mode in permission_mode_strategy()) {
                let update = PermissionUpdate {
                    update_type: PermissionUpdateType::SetMode,
                    rules: None,
                    behavior: None,
                    mode: Some(mode),
                    directories: None,
                    destination: None,
                };

                let json = serde_json::to_string(&update).unwrap();
                let deserialized: PermissionUpdate = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(update.update_type, deserialized.update_type);
                prop_assert_eq!(update.mode, deserialized.mode);
            }

            #[test]
            fn permission_update_add_directories_serialization(
                dirs in prop::collection::vec("[a-zA-Z0-9/_-]{1,50}", 1..5)
            ) {
                let update = PermissionUpdate {
                    update_type: PermissionUpdateType::AddDirectories,
                    rules: None,
                    behavior: None,
                    mode: None,
                    directories: Some(dirs.clone()),
                    destination: None,
                };

                let json = serde_json::to_string(&update).unwrap();
                let deserialized: PermissionUpdate = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(update.update_type, deserialized.update_type);
                prop_assert_eq!(update.directories, deserialized.directories);
            }

            // PermissionResultAllow property tests
            #[test]
            fn permission_result_allow_with_updates(
                updated in prop::option::of(prop::collection::vec(0i32..100, 0..5))
            ) {
                let input = updated.map(|v| serde_json::json!({"values": v}));
                let result = PermissionResultAllow {
                    updated_input: input,
                    updated_permissions: None,
                };

                // Ensure structure is as expected
                prop_assert!(result.updated_permissions.is_none());
            }

            // PermissionResultDeny property tests
            #[test]
            fn permission_result_deny_with_interrupt(
                message in "\\PC{1,100}",
                interrupt in prop::bool::ANY
            ) {
                let result = PermissionResultDeny {
                    message: message.clone(),
                    interrupt,
                };

                prop_assert_eq!(result.message, message);
                prop_assert_eq!(result.interrupt, interrupt);
            }

            // Combination tests for permission logic
            #[test]
            fn permission_mode_combinations_valid(
                mode1 in permission_mode_strategy(),
                mode2 in permission_mode_strategy()
            ) {
                // Test that different modes can coexist in a system
                // (e.g., in different contexts or transitions)
                let modes = vec![mode1, mode2];

                // At least one should be valid (trivially true, but tests enum validity)
                prop_assert!(!modes.is_empty());

                // All modes should serialize/deserialize consistently
                for mode in modes {
                    let json = serde_json::to_string(&mode).unwrap();
                    let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
                    prop_assert_eq!(mode, deserialized);
                }
            }

            #[test]
            fn permission_update_type_completeness(
                update_type in prop_oneof![
                    Just(PermissionUpdateType::AddRules),
                    Just(PermissionUpdateType::ReplaceRules),
                    Just(PermissionUpdateType::RemoveRules),
                    Just(PermissionUpdateType::SetMode),
                    Just(PermissionUpdateType::AddDirectories),
                    Just(PermissionUpdateType::RemoveDirectories),
                ]
            ) {
                // Test that all update types serialize correctly
                let json = serde_json::to_string(&update_type).unwrap();
                let deserialized: PermissionUpdateType = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(update_type, deserialized);
            }
        }
    }
}
