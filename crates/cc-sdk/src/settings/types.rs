//! Settings type definitions.
//!
//! This module defines the core types for settings management, including
//! settings scope, hook configurations, and Claude settings structure.

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::options::McpServerConfig;

/// Settings scope determines where settings are loaded from and stored.
///
/// Settings are loaded with precedence: Local > Project > User
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsScope {
    /// User-level settings (~/.claude/settings.json)
    User,

    /// Project-level settings (<project>/.claude/settings.json)
    Project,

    /// Local settings (./.claude/settings.json in current directory)
    Local,
}

impl SettingsScope {
    /// Get the file path for this settings scope.
    ///
    /// # Arguments
    ///
    /// * `project_path` - Optional project path, required for Project and Local scopes
    ///
    /// # Returns
    ///
    /// The settings file path if it can be determined, None otherwise.
    pub fn file_path(&self, project_path: Option<&PathBuf>) -> Option<PathBuf> {
        match self {
            SettingsScope::User => {
                dirs::home_dir().map(|home| home.join(".claude").join("settings.json"))
            }
            SettingsScope::Project => {
                project_path.map(|path| path.join(".claude").join("settings.json"))
            }
            SettingsScope::Local => Some(PathBuf::from(".claude/settings.json")),
        }
    }

    /// Get all scopes in order of precedence (highest to lowest).
    pub fn all_ordered() -> Vec<SettingsScope> {
        vec![
            SettingsScope::Local,
            SettingsScope::Project,
            SettingsScope::User,
        ]
    }
}

/// Hook configuration for a specific hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Hook name/event type (e.g., "pre_tool_use", "post_tool_use")
    pub hook_type: String,

    /// Command to execute for this hook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Arguments for the command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Whether this hook is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Additional configuration data
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl HookConfig {
    /// Create a new hook configuration.
    pub fn new(hook_type: impl Into<String>) -> Self {
        Self {
            hook_type: hook_type.into(),
            command: None,
            args: None,
            enabled: true,
            config: HashMap::new(),
        }
    }

    /// Set the command for this hook.
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Set the arguments for this hook.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Disable this hook.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Complete Claude settings structure.
///
/// This represents the merged settings from all scopes (user, project, local).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeSettings {
    /// Hook configurations organized by hook type
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub hooks: HashMap<String, Vec<HookConfig>>,

    /// MCP server configurations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    /// Default model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    /// Default permission mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,

    /// Custom prompts
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub prompts: HashMap<String, String>,

    /// Environment variables to set
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Additional settings
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl ClaudeSettings {
    /// Create empty settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another settings instance into this one.
    ///
    /// Settings from `other` take precedence over existing settings.
    pub fn merge(&mut self, other: ClaudeSettings) {
        // Merge hooks
        for (hook_type, configs) in other.hooks {
            self.hooks
                .entry(hook_type)
                .or_insert_with(Vec::new)
                .extend(configs);
        }

        // Merge MCP servers
        self.mcp_servers.extend(other.mcp_servers);

        // Override scalar values
        if other.default_model.is_some() {
            self.default_model = other.default_model;
        }
        if other.permission_mode.is_some() {
            self.permission_mode = other.permission_mode;
        }

        // Merge prompts and env
        self.prompts.extend(other.prompts);
        self.env.extend(other.env);

        // Merge additional settings
        self.additional.extend(other.additional);
    }

    /// Add a hook configuration.
    pub fn add_hook(&mut self, hook_type: impl Into<String>, config: HookConfig) {
        let hook_type = hook_type.into();
        self.hooks
            .entry(hook_type)
            .or_insert_with(Vec::new)
            .push(config);
    }

    /// Add an MCP server configuration.
    pub fn add_mcp_server(&mut self, name: impl Into<String>, config: McpServerConfig) {
        self.mcp_servers.insert(name.into(), config);
    }

    /// Get hooks for a specific hook type.
    pub fn get_hooks(&self, hook_type: &str) -> Vec<&HookConfig> {
        self.hooks
            .get(hook_type)
            .map(|configs| configs.iter().filter(|c| c.enabled).collect())
            .unwrap_or_default()
    }

    /// Get an MCP server configuration by name.
    pub fn get_mcp_server(&self, name: &str) -> Option<&McpServerConfig> {
        self.mcp_servers.get(name)
    }
}
