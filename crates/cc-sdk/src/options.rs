//! Configuration options for Claude Code SDK
//!
//! This module contains configuration types and builders for customizing
//! Claude Code SDK behavior.
//!
//! # Main Types
//!
//! - [`ClaudeCodeOptions`] - Main configuration struct
//! - [`ClaudeCodeOptionsBuilder`] - Builder for creating options
//! - [`McpServerConfig`] - MCP server configuration
//! - [`ControlProtocolFormat`] - Control protocol format selection
//!
//! # Configuration Categories
//!
//! - **System Prompts**: Configure system prompts for Claude
//! - **Tools**: Control which tools are allowed/disallowed
//! - **Permissions**: Set permission modes and behavior
//! - **MCP Servers**: Configure Model Context Protocol servers
//! - **Performance**: Set token limits and turn counts
//! - **Session**: Control conversation continuation and resumption
//!
//! # Example
//!
//! ```rust
//! use cc_sdk::options::{ClaudeCodeOptions, PermissionMode};
//!
//! let options = ClaudeCodeOptions::builder()
//!     .model("claude-sonnet-4")
//!     .permission_mode(PermissionMode::AcceptEdits)
//!     .max_turns(10)
//!     .build();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hooks::{CanUseTool, HookMatcher};
use crate::permissions::PermissionMode;

/// Control protocol format for sending messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlProtocolFormat {
    /// Legacy format: {"type":"sdk_control_request","request":{...}}
    Legacy,
    /// New format: {"type":"control","control":{...}}
    Control,
    /// Auto-detect based on CLI capabilities (default to Legacy for compatibility)
    Auto,
}

impl Default for ControlProtocolFormat {
    fn default() -> Self {
        // Default to Legacy for maximum compatibility
        Self::Legacy
    }
}

/// MCP (Model Context Protocol) server configuration
#[derive(Clone)]
pub enum McpServerConfig {
    /// Standard I/O based MCP server
    Stdio {
        /// Command to execute
        command: String,
        /// Command arguments
        args: Option<Vec<String>>,
        /// Environment variables
        env: Option<HashMap<String, String>>,
    },
    /// Server-Sent Events based MCP server
    Sse {
        /// Server URL
        url: String,
        /// HTTP headers
        headers: Option<HashMap<String, String>>,
    },
    /// HTTP-based MCP server
    Http {
        /// Server URL
        url: String,
        /// HTTP headers
        headers: Option<HashMap<String, String>>,
    },
    /// SDK MCP server (in-process)
    Sdk {
        /// Server name
        name: String,
        /// Server instance
        instance: Arc<dyn std::any::Any + Send + Sync>,
    },
}

impl std::fmt::Debug for McpServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio { command, args, env } => f
                .debug_struct("Stdio")
                .field("command", command)
                .field("args", args)
                .field("env", env)
                .finish(),
            Self::Sse { url, headers } => f
                .debug_struct("Sse")
                .field("url", url)
                .field("headers", headers)
                .finish(),
            Self::Http { url, headers } => f
                .debug_struct("Http")
                .field("url", url)
                .field("headers", headers)
                .finish(),
            Self::Sdk { name, .. } => f
                .debug_struct("Sdk")
                .field("name", name)
                .field("instance", &"<Arc<dyn Any>>")
                .finish(),
        }
    }
}

impl Serialize for McpServerConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;

        match self {
            Self::Stdio { command, args, env } => {
                map.serialize_entry("type", "stdio")?;
                map.serialize_entry("command", command)?;
                if let Some(args) = args {
                    map.serialize_entry("args", args)?;
                }
                if let Some(env) = env {
                    map.serialize_entry("env", env)?;
                }
            }
            Self::Sse { url, headers } => {
                map.serialize_entry("type", "sse")?;
                map.serialize_entry("url", url)?;
                if let Some(headers) = headers {
                    map.serialize_entry("headers", headers)?;
                }
            }
            Self::Http { url, headers } => {
                map.serialize_entry("type", "http")?;
                map.serialize_entry("url", url)?;
                if let Some(headers) = headers {
                    map.serialize_entry("headers", headers)?;
                }
            }
            Self::Sdk { name, .. } => {
                map.serialize_entry("type", "sdk")?;
                map.serialize_entry("name", name)?;
            }
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "lowercase")]
        enum McpServerConfigHelper {
            Stdio {
                command: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                args: Option<Vec<String>>,
                #[serde(skip_serializing_if = "Option::is_none")]
                env: Option<HashMap<String, String>>,
            },
            Sse {
                url: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                headers: Option<HashMap<String, String>>,
            },
            Http {
                url: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                headers: Option<HashMap<String, String>>,
            },
        }

        let helper = McpServerConfigHelper::deserialize(deserializer)?;
        Ok(match helper {
            McpServerConfigHelper::Stdio { command, args, env } => {
                McpServerConfig::Stdio { command, args, env }
            }
            McpServerConfigHelper::Sse { url, headers } => {
                McpServerConfig::Sse { url, headers }
            }
            McpServerConfigHelper::Http { url, headers } => {
                McpServerConfig::Http { url, headers }
            }
        })
    }
}

/// Setting source for configuration loading
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    /// User-level settings
    User,
    /// Project-level settings
    Project,
    /// Local settings
    Local,
}

/// Agent definition for programmatic agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent description
    pub description: String,
    /// Agent prompt
    pub prompt: String,
    /// Allowed tools for this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// System prompt configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Simple string prompt
    String(String),
    /// Preset-based prompt with optional append
    Preset {
        /// Preset type identifier (always "preset")
        #[serde(rename = "type")]
        preset_type: String,
        /// Preset name (e.g., "claude_code")
        preset: String,
        /// Optional text to append to the preset
        #[serde(skip_serializing_if = "Option::is_none")]
        append: Option<String>,
    },
}

/// Configuration options for Claude Code SDK
#[derive(Clone, Default)]
pub struct ClaudeCodeOptions {
    /// System prompt configuration (simplified in v0.1.12+)
    /// Can be either a string or a preset configuration
    /// Replaces the old system_prompt and append_system_prompt fields
    pub system_prompt_v2: Option<SystemPrompt>,
    /// [DEPRECATED] System prompt to prepend to all messages
    /// Use system_prompt_v2 instead
    #[deprecated(since = "0.1.12", note = "Use system_prompt_v2 instead")]
    pub system_prompt: Option<String>,
    /// [DEPRECATED] Additional system prompt to append
    /// Use system_prompt_v2 instead
    #[deprecated(since = "0.1.12", note = "Use system_prompt_v2 instead")]
    pub append_system_prompt: Option<String>,
    /// List of allowed tools
    pub allowed_tools: Vec<String>,
    /// List of disallowed tools
    pub disallowed_tools: Vec<String>,
    /// Permission mode for tool execution
    pub permission_mode: PermissionMode,
    /// MCP server configurations
    pub mcp_servers: HashMap<String, McpServerConfig>,
    /// MCP tools to enable
    pub mcp_tools: Vec<String>,
    /// Maximum number of conversation turns
    pub max_turns: Option<i32>,
    /// Maximum thinking tokens
    pub max_thinking_tokens: i32,
    /// Maximum output tokens per response (1-32000, overrides CLAUDE_CODE_MAX_OUTPUT_TOKENS env var)
    pub max_output_tokens: Option<u32>,
    /// Model to use
    pub model: Option<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Continue from previous conversation
    pub continue_conversation: bool,
    /// Resume from a specific conversation ID
    pub resume: Option<String>,
    /// Custom permission prompt tool name
    pub permission_prompt_tool_name: Option<String>,
    /// Settings file path for Claude Code CLI
    pub settings: Option<String>,
    /// Additional directories to add as working directories
    pub add_dirs: Vec<PathBuf>,
    /// Extra arbitrary CLI flags
    pub extra_args: HashMap<String, Option<String>>,
    /// Environment variables to pass to the process
    pub env: HashMap<String, String>,
    /// Debug output stream (e.g., stderr)
    pub debug_stderr: Option<Arc<Mutex<dyn Write + Send + Sync>>>,
    /// Include partial assistant messages in streaming output
    pub include_partial_messages: bool,
    /// Tool permission callback
    pub can_use_tool: Option<Arc<dyn CanUseTool>>,
    /// Hook configurations
    pub hooks: Option<HashMap<String, Vec<HookMatcher>>>,
    /// Control protocol format (defaults to Legacy for compatibility)
    pub control_protocol_format: ControlProtocolFormat,

    // ========== Phase 2 Enhancements ==========
    /// Setting sources to load (user, project, local)
    /// When None, no filesystem settings are loaded (matches Python SDK v0.1.0 behavior)
    pub setting_sources: Option<Vec<SettingSource>>,
    /// Fork session when resuming instead of continuing
    /// When true, creates a new branch from the resumed session
    pub fork_session: bool,
    /// Programmatic agent definitions
    /// Define agents inline without filesystem dependencies
    pub agents: Option<HashMap<String, AgentDefinition>>,
    /// CLI channel buffer size for internal communication channels
    /// Controls the size of message, control, and stdin buffers (default: 100)
    /// Increase for high-throughput scenarios to prevent message lag
    pub cli_channel_buffer_size: Option<usize>,
}

impl std::fmt::Debug for ClaudeCodeOptions {
    #[allow(deprecated)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCodeOptions")
            .field("system_prompt", &self.system_prompt)
            .field("append_system_prompt", &self.append_system_prompt)
            .field("allowed_tools", &self.allowed_tools)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("permission_mode", &self.permission_mode)
            .field("mcp_servers", &self.mcp_servers)
            .field("mcp_tools", &self.mcp_tools)
            .field("max_turns", &self.max_turns)
            .field("max_thinking_tokens", &self.max_thinking_tokens)
            .field("max_output_tokens", &self.max_output_tokens)
            .field("model", &self.model)
            .field("cwd", &self.cwd)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("permission_prompt_tool_name", &self.permission_prompt_tool_name)
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("extra_args", &self.extra_args)
            .field("env", &self.env)
            .field("debug_stderr", &self.debug_stderr.is_some())
            .field("include_partial_messages", &self.include_partial_messages)
            .field("can_use_tool", &self.can_use_tool.is_some())
            .field("hooks", &self.hooks.is_some())
            .field("control_protocol_format", &self.control_protocol_format)
            .finish()
    }
}

impl ClaudeCodeOptions {
    /// Create a new options builder
    pub fn builder() -> ClaudeCodeOptionsBuilder {
        ClaudeCodeOptionsBuilder::default()
    }
}

/// Builder for ClaudeCodeOptions
#[derive(Debug, Default)]
pub struct ClaudeCodeOptionsBuilder {
    options: ClaudeCodeOptions,
}

impl ClaudeCodeOptionsBuilder {
    /// Set system prompt
    #[allow(deprecated)]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    /// Set append system prompt
    #[allow(deprecated)]
    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.options.append_system_prompt = Some(prompt.into());
        self
    }

    /// Add allowed tools
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.allowed_tools = tools;
        self
    }

    /// Add a single allowed tool
    pub fn allow_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.allowed_tools.push(tool.into());
        self
    }

    /// Add disallowed tools
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.disallowed_tools = tools;
        self
    }

    /// Add a single disallowed tool
    pub fn disallow_tool(mut self, tool: impl Into<String>) -> Self {
        self.options.disallowed_tools.push(tool.into());
        self
    }

    /// Set permission mode
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = mode;
        self
    }

    /// Add MCP server
    pub fn add_mcp_server(mut self, name: impl Into<String>, config: McpServerConfig) -> Self {
        self.options.mcp_servers.insert(name.into(), config);
        self
    }

    /// Set all MCP servers from a map
    pub fn mcp_servers(mut self, servers: HashMap<String, McpServerConfig>) -> Self {
        self.options.mcp_servers = servers;
        self
    }

    /// Set MCP tools
    pub fn mcp_tools(mut self, tools: Vec<String>) -> Self {
        self.options.mcp_tools = tools;
        self
    }

    /// Set max turns
    pub fn max_turns(mut self, turns: i32) -> Self {
        self.options.max_turns = Some(turns);
        self
    }

    /// Set max thinking tokens
    pub fn max_thinking_tokens(mut self, tokens: i32) -> Self {
        self.options.max_thinking_tokens = tokens;
        self
    }

    /// Set max output tokens (1-32000, overrides CLAUDE_CODE_MAX_OUTPUT_TOKENS env var)
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.options.max_output_tokens = Some(tokens.clamp(1, 32000));
        self
    }

    /// Set model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    /// Set working directory
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(path.into());
        self
    }

    /// Enable continue conversation
    pub fn continue_conversation(mut self, enable: bool) -> Self {
        self.options.continue_conversation = enable;
        self
    }

    /// Set resume conversation ID
    pub fn resume(mut self, id: impl Into<String>) -> Self {
        self.options.resume = Some(id.into());
        self
    }

    /// Set permission prompt tool name
    pub fn permission_prompt_tool_name(mut self, name: impl Into<String>) -> Self {
        self.options.permission_prompt_tool_name = Some(name.into());
        self
    }

    /// Set settings file path
    pub fn settings(mut self, settings: impl Into<String>) -> Self {
        self.options.settings = Some(settings.into());
        self
    }

    /// Add directories as working directories
    pub fn add_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.options.add_dirs = dirs;
        self
    }

    /// Add a single directory as working directory
    pub fn add_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.options.add_dirs.push(dir.into());
        self
    }

    /// Add extra CLI arguments
    pub fn extra_args(mut self, args: HashMap<String, Option<String>>) -> Self {
        self.options.extra_args = args;
        self
    }

    /// Add a single extra CLI argument
    pub fn add_extra_arg(mut self, key: impl Into<String>, value: Option<String>) -> Self {
        self.options.extra_args.insert(key.into(), value);
        self
    }

    /// Set control protocol format
    pub fn control_protocol_format(mut self, format: ControlProtocolFormat) -> Self {
        self.options.control_protocol_format = format;
        self
    }

    /// Include partial assistant messages in streaming output
    pub fn include_partial_messages(mut self, include: bool) -> Self {
        self.options.include_partial_messages = include;
        self
    }

    /// Enable fork_session behavior
    pub fn fork_session(mut self, fork: bool) -> Self {
        self.options.fork_session = fork;
        self
    }

    /// Set setting sources
    pub fn setting_sources(mut self, sources: Vec<SettingSource>) -> Self {
        self.options.setting_sources = Some(sources);
        self
    }

    /// Define programmatic agents
    pub fn agents(mut self, agents: HashMap<String, AgentDefinition>) -> Self {
        self.options.agents = Some(agents);
        self
    }

    /// Set CLI channel buffer size
    ///
    /// Controls the size of internal communication channels (message, control, stdin buffers).
    /// Default is 100. Increase for high-throughput scenarios to prevent message lag.
    ///
    /// # Arguments
    ///
    /// * `size` - Buffer size (number of messages that can be queued)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .cli_channel_buffer_size(500)
    ///     .build();
    /// ```
    pub fn cli_channel_buffer_size(mut self, size: usize) -> Self {
        self.options.cli_channel_buffer_size = Some(size);
        self
    }

    /// Build the options
    pub fn build(self) -> ClaudeCodeOptions {
        self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_options_builder() {
        let options = ClaudeCodeOptions::builder()
            .system_prompt("Test prompt")
            .model("claude-3-opus")
            .permission_mode(PermissionMode::AcceptEdits)
            .allow_tool("read")
            .allow_tool("write")
            .max_turns(10)
            .build();

        assert_eq!(options.system_prompt, Some("Test prompt".to_string()));
        assert_eq!(options.model, Some("claude-3-opus".to_string()));
        assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
        assert_eq!(options.allowed_tools, vec!["read", "write"]);
        assert_eq!(options.max_turns, Some(10));
    }

    #[test]
    fn test_extra_args() {
        let mut extra_args = HashMap::new();
        extra_args.insert("custom-flag".to_string(), Some("value".to_string()));
        extra_args.insert("boolean-flag".to_string(), None);

        let options = ClaudeCodeOptions::builder()
            .extra_args(extra_args.clone())
            .add_extra_arg("another-flag", Some("another-value".to_string()))
            .build();

        assert_eq!(options.extra_args.len(), 3);
        assert_eq!(options.extra_args.get("custom-flag"), Some(&Some("value".to_string())));
        assert_eq!(options.extra_args.get("boolean-flag"), Some(&None));
        assert_eq!(options.extra_args.get("another-flag"), Some(&Some("another-value".to_string())));
    }
}
