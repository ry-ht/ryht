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
    /// SDK control request format: {"type":"sdk_control_request","request":{...}}
    SdkControlRequest,
    /// Control format: {"type":"control","control":{...}}
    Control,
    /// Auto-detect based on CLI capabilities
    Auto,
}

impl Default for ControlProtocolFormat {
    fn default() -> Self {
        Self::SdkControlRequest
    }
}

/// Output format for CLI responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Plain text output
    Text,
    /// JSON output
    Json,
    /// Streaming JSON output (one JSON object per line)
    StreamJson,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::StreamJson
    }
}

/// Input format for CLI messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    /// Plain text input
    Text,
    /// Streaming JSON input (one JSON object per line)
    StreamJson,
}

impl Default for InputFormat {
    fn default() -> Self {
        Self::StreamJson
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
    /// System prompt configuration
    /// Can be either a string or a preset configuration
    pub system_prompt: Option<SystemPrompt>,
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
    /// Control protocol format
    pub control_protocol_format: ControlProtocolFormat,

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

    /// Debug mode with optional category filtering
    /// When set, enables debug output. The value can be empty for all categories
    /// or a comma-separated list of debug categories (e.g., "api,mcp")
    pub debug_mode: Option<String>,

    /// Print mode - non-interactive output
    /// When true, CLI runs in non-interactive mode and exits after response
    pub print_mode: bool,

    /// Output format for CLI responses
    pub output_format: OutputFormat,

    /// Input format for CLI messages
    pub input_format: InputFormat,

    /// Fallback model to use when primary model is unavailable
    pub fallback_model: Option<String>,

    /// Enable IDE auto-connect
    pub ide_autoconnect: bool,

    /// Enable strict MCP configuration validation
    pub strict_mcp_config: bool,

    /// Custom session ID (UUID format)
    pub custom_session_id: Option<uuid::Uuid>,

    /// Replay user messages from conversation history
    pub replay_user_messages: bool,
}

impl std::fmt::Debug for ClaudeCodeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCodeOptions")
            .field("system_prompt", &self.system_prompt)
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
            .field("fallback_model", &self.fallback_model)
            .field("ide_autoconnect", &self.ide_autoconnect)
            .field("strict_mcp_config", &self.strict_mcp_config)
            .field("custom_session_id", &self.custom_session_id)
            .field("replay_user_messages", &self.replay_user_messages)
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
    /// Set system prompt (modern API)
    pub fn system_prompt(mut self, prompt: SystemPrompt) -> Self {
        self.options.system_prompt = Some(prompt);
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

    /// Enable debug mode with optional category filtering
    ///
    /// When enabled, Claude CLI will output debug information. You can optionally
    /// specify categories to filter debug output (e.g., "api,mcp").
    ///
    /// # Arguments
    ///
    /// * `filter` - Optional debug category filter. Use empty string for all categories.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// // Enable debug for all categories
    /// let options = ClaudeCodeOptions::builder()
    ///     .debug_mode("")
    ///     .build();
    ///
    /// // Enable debug for specific categories
    /// let options = ClaudeCodeOptions::builder()
    ///     .debug_mode("api,mcp")
    ///     .build();
    /// ```
    pub fn debug_mode(mut self, filter: impl Into<String>) -> Self {
        self.options.debug_mode = Some(filter.into());
        self
    }

    /// Enable print mode (non-interactive)
    ///
    /// When enabled, the CLI runs in non-interactive mode and exits after
    /// receiving a response. Useful for one-shot queries.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable print mode
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .print_mode(true)
    ///     .build();
    /// ```
    pub fn print_mode(mut self, enabled: bool) -> Self {
        self.options.print_mode = enabled;
        self
    }

    /// Set output format
    ///
    /// Controls the format of responses from the Claude CLI.
    ///
    /// # Arguments
    ///
    /// * `format` - The output format to use
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::{ClaudeCodeOptions, OutputFormat};
    /// let options = ClaudeCodeOptions::builder()
    ///     .output_format(OutputFormat::Json)
    ///     .build();
    /// ```
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.options.output_format = format;
        self
    }

    /// Set input format
    ///
    /// Controls the format of messages sent to the Claude CLI.
    ///
    /// # Arguments
    ///
    /// * `format` - The input format to use
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::{ClaudeCodeOptions, InputFormat};
    /// let options = ClaudeCodeOptions::builder()
    ///     .input_format(InputFormat::Text)
    ///     .build();
    /// ```
    pub fn input_format(mut self, format: InputFormat) -> Self {
        self.options.input_format = format;
        self
    }

    /// Set fallback model
    ///
    /// Specifies a fallback model to use when the primary model is unavailable.
    /// This is promoted from extra_args to a first-class field.
    ///
    /// # Arguments
    ///
    /// * `model` - The fallback model identifier
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .model("claude-sonnet-4")
    ///     .fallback_model("claude-opus-4")
    ///     .build();
    /// ```
    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.options.fallback_model = Some(model.into());
        self
    }

    /// Enable IDE auto-connect
    ///
    /// When enabled, the CLI will automatically connect to supported IDEs.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable IDE auto-connect
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .ide_autoconnect(true)
    ///     .build();
    /// ```
    pub fn ide_autoconnect(mut self, enable: bool) -> Self {
        self.options.ide_autoconnect = enable;
        self
    }

    /// Enable strict MCP configuration validation
    ///
    /// When enabled, the CLI will strictly validate MCP server configurations.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable strict MCP config validation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .strict_mcp_config(true)
    ///     .build();
    /// ```
    pub fn strict_mcp_config(mut self, enable: bool) -> Self {
        self.options.strict_mcp_config = enable;
        self
    }

    /// Set custom session ID
    ///
    /// Specifies a custom UUID to use as the session identifier.
    ///
    /// # Arguments
    ///
    /// * `id` - The custom session UUID
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// # use uuid::Uuid;
    /// let session_id = Uuid::new_v4();
    /// let options = ClaudeCodeOptions::builder()
    ///     .custom_session_id(session_id)
    ///     .build();
    /// ```
    pub fn custom_session_id(mut self, id: uuid::Uuid) -> Self {
        self.options.custom_session_id = Some(id);
        self
    }

    /// Enable replay user messages
    ///
    /// When enabled, user messages from conversation history will be replayed.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable replay user messages
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cc_sdk::options::ClaudeCodeOptions;
    /// let options = ClaudeCodeOptions::builder()
    ///     .replay_user_messages(true)
    ///     .build();
    /// ```
    pub fn replay_user_messages(mut self, enable: bool) -> Self {
        self.options.replay_user_messages = enable;
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
    fn test_options_builder() {
        let options = ClaudeCodeOptions::builder()
            .system_prompt(SystemPrompt::String("Test prompt".to_string()))
            .model("claude-3-opus")
            .permission_mode(PermissionMode::AcceptEdits)
            .allow_tool("read")
            .allow_tool("write")
            .max_turns(10)
            .build();

        assert!(matches!(options.system_prompt, Some(SystemPrompt::String(_))));
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

    #[test]
    fn test_debug_mode() {
        // Test debug mode with no filter
        let options = ClaudeCodeOptions::builder()
            .debug_mode("")
            .build();
        assert_eq!(options.debug_mode, Some("".to_string()));

        // Test debug mode with specific categories
        let options = ClaudeCodeOptions::builder()
            .debug_mode("api,mcp")
            .build();
        assert_eq!(options.debug_mode, Some("api,mcp".to_string()));

        // Test no debug mode by default
        let options = ClaudeCodeOptions::builder().build();
        assert_eq!(options.debug_mode, None);
    }

    #[test]
    fn test_print_mode() {
        // Test print mode enabled
        let options = ClaudeCodeOptions::builder()
            .print_mode(true)
            .build();
        assert!(options.print_mode);

        // Test print mode disabled
        let options = ClaudeCodeOptions::builder()
            .print_mode(false)
            .build();
        assert!(!options.print_mode);

        // Test print mode default (false)
        let options = ClaudeCodeOptions::builder().build();
        assert!(!options.print_mode);
    }

    #[test]
    fn test_output_format() {
        // Test all output formats
        let options = ClaudeCodeOptions::builder()
            .output_format(OutputFormat::Text)
            .build();
        assert_eq!(options.output_format, OutputFormat::Text);

        let options = ClaudeCodeOptions::builder()
            .output_format(OutputFormat::Json)
            .build();
        assert_eq!(options.output_format, OutputFormat::Json);

        let options = ClaudeCodeOptions::builder()
            .output_format(OutputFormat::StreamJson)
            .build();
        assert_eq!(options.output_format, OutputFormat::StreamJson);

        // Test default is StreamJson
        let options = ClaudeCodeOptions::builder().build();
        assert_eq!(options.output_format, OutputFormat::StreamJson);
    }

    #[test]
    fn test_input_format() {
        // Test all input formats
        let options = ClaudeCodeOptions::builder()
            .input_format(InputFormat::Text)
            .build();
        assert_eq!(options.input_format, InputFormat::Text);

        let options = ClaudeCodeOptions::builder()
            .input_format(InputFormat::StreamJson)
            .build();
        assert_eq!(options.input_format, InputFormat::StreamJson);

        // Test default is StreamJson
        let options = ClaudeCodeOptions::builder().build();
        assert_eq!(options.input_format, InputFormat::StreamJson);
    }

    #[test]
    fn test_combined_cli_features() {
        // Test combining debug mode, print mode, and format options
        let options = ClaudeCodeOptions::builder()
            .debug_mode("api,mcp")
            .print_mode(true)
            .output_format(OutputFormat::Json)
            .input_format(InputFormat::Text)
            .model("claude-sonnet-4")
            .build();

        assert_eq!(options.debug_mode, Some("api,mcp".to_string()));
        assert!(options.print_mode);
        assert_eq!(options.output_format, OutputFormat::Json);
        assert_eq!(options.input_format, InputFormat::Text);
        assert_eq!(options.model, Some("claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_fallback_model() {
        let options = ClaudeCodeOptions::builder()
            .model("claude-sonnet-4")
            .fallback_model("claude-opus-4")
            .build();

        assert_eq!(options.model, Some("claude-sonnet-4".to_string()));
        assert_eq!(options.fallback_model, Some("claude-opus-4".to_string()));

        // Test without fallback model
        let options = ClaudeCodeOptions::builder()
            .model("claude-sonnet-4")
            .build();
        assert_eq!(options.fallback_model, None);
    }

    #[test]
    fn test_ide_autoconnect() {
        let options = ClaudeCodeOptions::builder()
            .ide_autoconnect(true)
            .build();
        assert!(options.ide_autoconnect);

        // Test default is false
        let options = ClaudeCodeOptions::builder().build();
        assert!(!options.ide_autoconnect);
    }

    #[test]
    fn test_strict_mcp_config() {
        let options = ClaudeCodeOptions::builder()
            .strict_mcp_config(true)
            .build();
        assert!(options.strict_mcp_config);

        // Test default is false
        let options = ClaudeCodeOptions::builder().build();
        assert!(!options.strict_mcp_config);
    }

    #[test]
    fn test_custom_session_id() {
        use uuid::Uuid;

        let session_id = Uuid::new_v4();
        let options = ClaudeCodeOptions::builder()
            .custom_session_id(session_id)
            .build();
        assert_eq!(options.custom_session_id, Some(session_id));

        // Test without custom session ID
        let options = ClaudeCodeOptions::builder().build();
        assert_eq!(options.custom_session_id, None);
    }

    #[test]
    fn test_replay_user_messages() {
        let options = ClaudeCodeOptions::builder()
            .replay_user_messages(true)
            .build();
        assert!(options.replay_user_messages);

        // Test default is false
        let options = ClaudeCodeOptions::builder().build();
        assert!(!options.replay_user_messages);
    }

    #[test]
    fn test_all_new_features_combined() {
        use uuid::Uuid;

        let session_id = Uuid::new_v4();
        let options = ClaudeCodeOptions::builder()
            .model("claude-sonnet-4")
            .fallback_model("claude-opus-4")
            .ide_autoconnect(true)
            .strict_mcp_config(true)
            .custom_session_id(session_id)
            .replay_user_messages(true)
            .build();

        assert_eq!(options.model, Some("claude-sonnet-4".to_string()));
        assert_eq!(options.fallback_model, Some("claude-opus-4".to_string()));
        assert!(options.ide_autoconnect);
        assert!(options.strict_mcp_config);
        assert_eq!(options.custom_session_id, Some(session_id));
        assert!(options.replay_user_messages);
    }
}
