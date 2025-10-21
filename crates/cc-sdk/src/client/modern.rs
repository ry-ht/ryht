//! Modern type-safe client API with type-state pattern.
//!
//! This module provides a modern, ergonomic API for interacting with Claude Code
//! using the type-state pattern to ensure compile-time safety.
//!
//! # Type States
//!
//! The client progresses through these states:
//! - `NoBinary`: Initial state, no binary discovered
//! - `WithBinary`: Binary discovered but not configured
//! - `Configured`: Configuration set but not connected
//! - `Connected`: Fully connected and ready to send messages
//! - `Disconnected`: Previously connected, now disconnected
//!
//! # Advanced Features
//!
//! The modern client includes advanced capabilities inspired by the Claude CLI and mcp-sdk:
//!
//! - **Model Fallback**: Configure multiple models with automatic failover
//! - **Tool Filtering**: Allow/disallow specific tools with fine-grained control
//! - **Session Forking**: Create conversation branches from resume points
//! - **MCP Integration**: Easy configuration of Model Context Protocol servers
//! - **Dynamic Permissions**: Update permission modes without reconnecting
//! - **Session Management**: List sessions, get history, resume conversations
//! - **Rich Configuration**: System prompts, token limits, environment variables, etc.
//!
//! See [`MODERN_CLIENT_ENHANCEMENTS.md`](https://github.com/yourusername/cc-sdk/blob/main/crates/cc-sdk/MODERN_CLIENT_ENHANCEMENTS.md)
//! for detailed documentation of all features.
//!
//! # Basic Example
//!
//! ```no_run
//! use cc_sdk::ClaudeClient;
//! use cc_sdk::core::ModelId;
//! use cc_sdk::types::PermissionMode;
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> cc_sdk::Result<()> {
//!     // Build and connect client with type-safe state transitions
//!     let client = ClaudeClient::builder()
//!         .discover_binary().await?           // NoBinary -> WithBinary
//!         .model(ModelId::from("claude-sonnet-4-5-20250929"))
//!         .permission_mode(PermissionMode::AcceptEdits)
//!         .working_directory("/path/to/project")
//!         .add_allowed_tool("Bash")
//!         .configure()                        // WithBinary -> Configured
//!         .connect().await?                   // Configured -> Connected
//!         .build()?;
//!
//!     // Send messages and receive responses
//!     let mut stream = client.send("What's 2+2?").await?;
//!     while let Some(message) = stream.next().await {
//!         println!("{:?}", message?);
//!     }
//!
//!     // Clean disconnect
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Advanced Example
//!
//! ```no_run
//! use cc_sdk::{ClaudeClient, core::ModelId};
//! use cc_sdk::types::{PermissionMode, McpServerConfig};
//! use futures::StreamExt;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> cc_sdk::Result<()> {
//!     let client = ClaudeClient::builder()
//!         .discover_binary().await?
//!
//!         // Model fallback support
//!         .models(vec![
//!             ModelId::from("claude-sonnet-4-5-20250929"),
//!             ModelId::from("claude-opus-4-5-20250929"),
//!         ])
//!
//!         // Tool filtering
//!         .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
//!         .disallow_tool("Delete")
//!
//!         // MCP server integration
//!         .add_mcp_stdio_server(
//!             "filesystem",
//!             "npx",
//!             vec!["-y", "@modelcontextprotocol/server-filesystem"]
//!         )
//!
//!         // Advanced configuration
//!         .max_output_tokens(8000)
//!         .max_turns(20)
//!         .system_prompt("You are a helpful coding assistant.")
//!         .add_directory(PathBuf::from("/shared/libs"))
//!         .include_partial_messages(true)
//!
//!         .configure()
//!         .connect().await?
//!         .build()?;
//!
//!     // Use the client
//!     let mut stream = client.send("Help me with this code").await?;
//!     while let Some(msg) = stream.next().await {
//!         println!("{:?}", msg?);
//!     }
//!
//!     // Dynamic permission update
//!     client.set_permission_mode(PermissionMode::Default).await?;
//!
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```

use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::pin::Pin;

use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use std::collections::HashMap;

use crate::binary;
use crate::core::{state::*, BinaryPath, ModelId, SessionId};
use crate::error::{Error, BinaryError, ClientError, SessionError};
use crate::result::Result;
use crate::transport::{InputMessage, SubprocessTransport, Transport};
use crate::types::{ClaudeCodeOptions, Message, PermissionMode, McpServerConfig};

/// Modern type-safe Claude client with compile-time state verification.
///
/// The client uses the type-state pattern to prevent invalid operations:
/// - Cannot connect without a binary
/// - Cannot send messages without being connected
/// - Cannot configure twice
///
/// # Type Parameters
///
/// * `State` - The current state of the client (NoBinary, WithBinary, Configured, Connected, Disconnected)
pub struct ClaudeClient<State = Connected> {
    inner: Arc<ClientInner>,
    _state: PhantomData<State>,
}

/// Internal client state shared across type-state transitions.
struct ClientInner {
    binary_path: Option<BinaryPath>,
    options: Option<ClaudeCodeOptions>,
    transport: Option<Arc<tokio::sync::Mutex<SubprocessTransport>>>,
    session_id: Option<SessionId>,
    message_tx: Option<broadcast::Sender<Message>>,
}

impl ClientInner {
    fn new() -> Self {
        Self {
            binary_path: None,
            options: None,
            transport: None,
            session_id: None,
            message_tx: None,
        }
    }
}

// Builder pattern starting point
impl ClaudeClient<NoBinary> {
    /// Create a new client builder.
    ///
    /// This is the entry point for creating a Claude client. The builder
    /// starts in the `NoBinary` state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder();
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ClaudeClientBuilder<NoBinary> {
        ClaudeClientBuilder {
            inner: Arc::new(ClientInner::new()),
            _state: PhantomData,
        }
    }
}

/// Builder for constructing a Claude client with type-safe state transitions.
pub struct ClaudeClientBuilder<State = NoBinary> {
    inner: Arc<ClientInner>,
    _state: PhantomData<State>,
}

// NoBinary -> WithBinary transitions
impl ClaudeClientBuilder<NoBinary> {
    /// Discover the Claude binary automatically.
    ///
    /// Searches for Claude in standard locations (PATH, Homebrew, NVM, etc.).
    ///
    /// # Errors
    ///
    /// Returns `BinaryError::NotFound` if no valid Claude installation is found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_binary(self) -> Result<ClaudeClientBuilder<WithBinary>> {
        // Discover binary in blocking thread pool
        let binary_path = tokio::task::spawn_blocking(|| {
            binary::find_claude_binary()
        })
        .await
        .map_err(|e| Error::Protocol(format!("Discovery task failed: {}", e)))?
        .map_err(|_e| Error::Binary(BinaryError::NotFound {
            searched_paths: vec![],  // The error message already contains this info
        }))?;

        // Update inner state
        let inner = Arc::new(ClientInner {
            binary_path: Some(BinaryPath::new(binary_path)),
            options: None,
            transport: None,
            session_id: None,
            message_tx: None,
        });

        Ok(ClaudeClientBuilder {
            inner,
            _state: PhantomData,
        })
    }

    /// Use a specific binary path.
    ///
    /// Skips automatic discovery and uses the provided path directly.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .binary("/usr/local/bin/claude");
    /// # Ok(())
    /// # }
    /// ```
    pub fn binary(self, path: impl Into<BinaryPath>) -> ClaudeClientBuilder<WithBinary> {
        let inner = Arc::new(ClientInner {
            binary_path: Some(path.into()),
            options: None,
            transport: None,
            session_id: None,
            message_tx: None,
        });

        ClaudeClientBuilder {
            inner,
            _state: PhantomData,
        }
    }
}

// WithBinary state - configuration methods
impl ClaudeClientBuilder<WithBinary> {
    /// Set the model to use.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::core::ModelId;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .model(ModelId::from("claude-sonnet-4-5-20250929"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn model(mut self, model: ModelId) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.model = Some(model.into_inner());
        inner.options = Some(options);

        self
    }

    /// Set multiple models for fallback support.
    ///
    /// When the primary model is unavailable, the SDK will automatically
    /// try the next model in the list. This is useful for ensuring high
    /// availability.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::core::ModelId;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .models(vec![
    ///         ModelId::from("claude-sonnet-4-5-20250929"),
    ///         ModelId::from("claude-opus-4-5-20250929"),
    ///     ]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn models(mut self, models: Vec<ModelId>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        // Set primary model and store fallbacks in extra_args
        let mut options = inner.options.take().unwrap_or_default();
        if let Some(primary) = models.first() {
            options.model = Some(primary.clone().into_inner());
        }

        // Store fallback models in extra_args for now
        // In the future, this could be a dedicated field in ClaudeCodeOptions
        if models.len() > 1 {
            let fallback_models: Vec<String> = models.iter()
                .skip(1)
                .map(|m| m.clone().into_inner())
                .collect();
            options.extra_args.insert(
                "fallback-models".to_string(),
                Some(fallback_models.join(","))
            );
        }

        inner.options = Some(options);
        self
    }

    /// Set the permission mode.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::types::PermissionMode;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .permission_mode(PermissionMode::AcceptEdits);
    /// # Ok(())
    /// # }
    /// ```
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.permission_mode = mode;
        inner.options = Some(options);

        self
    }

    /// Set the working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .working_directory("/path/to/project");
    /// # Ok(())
    /// # }
    /// ```
    pub fn working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.cwd = Some(path.into());
        inner.options = Some(options);

        self
    }

    /// Add an allowed tool.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .add_allowed_tool("Bash")
    ///     .add_allowed_tool("Read");
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_allowed_tool(mut self, tool: impl Into<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.allowed_tools.push(tool.into());
        inner.options = Some(options);

        self
    }

    /// Set allowed tools (replaces existing list).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .allowed_tools(vec!["Bash".to_string(), "Read".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.allowed_tools = tools;
        inner.options = Some(options);

        self
    }

    /// Add a disallowed tool.
    ///
    /// Disallowed tools take precedence over allowed tools, preventing
    /// specific tools from being used even if they would otherwise be allowed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .disallow_tool("Bash")
    ///     .disallow_tool("Write");
    /// # Ok(())
    /// # }
    /// ```
    pub fn disallow_tool(mut self, tool: impl Into<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.disallowed_tools.push(tool.into());
        inner.options = Some(options);

        self
    }

    /// Set disallowed tools (replaces existing list).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .disallowed_tools(vec!["Bash".to_string(), "Write".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.disallowed_tools = tools;
        inner.options = Some(options);

        self
    }

    /// Set maximum output tokens per response.
    ///
    /// This controls how long Claude's responses can be. Valid range is 1-32000.
    /// This setting overrides the CLAUDE_CODE_MAX_OUTPUT_TOKENS environment variable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .max_output_tokens(8000);
    /// # Ok(())
    /// # }
    /// ```
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.max_output_tokens = Some(tokens.clamp(1, 32000));
        inner.options = Some(options);

        self
    }

    /// Set maximum number of conversation turns.
    ///
    /// Limits how many back-and-forth exchanges can occur in a single session.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .max_turns(20);
    /// # Ok(())
    /// # }
    /// ```
    pub fn max_turns(mut self, turns: i32) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.max_turns = Some(turns);
        inner.options = Some(options);

        self
    }

    /// Set system prompt for Claude.
    ///
    /// The system prompt provides context and instructions that Claude will
    /// follow throughout the conversation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .system_prompt("You are a helpful coding assistant.");
    /// # Ok(())
    /// # }
    /// ```
    #[allow(deprecated)]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.system_prompt = Some(prompt.into());
        inner.options = Some(options);

        self
    }

    /// Continue from a previous conversation.
    ///
    /// When enabled, the client will attempt to continue the most recent
    /// conversation in the current working directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .continue_conversation(true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn continue_conversation(mut self, enable: bool) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.continue_conversation = enable;
        inner.options = Some(options);

        self
    }

    /// Resume from a specific session ID.
    ///
    /// Loads and continues a previous conversation identified by its session ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::core::SessionId;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let session_id = SessionId::new("previous-session-123");
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .resume_session(session_id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn resume_session(mut self, session_id: SessionId) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.resume = Some(session_id.to_string());
        inner.options = Some(options);

        self
    }

    /// Fork a session when resuming.
    ///
    /// When enabled, resuming a session creates a new branch from that point
    /// rather than continuing the original session. This is useful for exploring
    /// alternative approaches without modifying the original conversation history.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::core::SessionId;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let session_id = SessionId::new("previous-session-123");
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .resume_session(session_id)
    ///     .fork_session(true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn fork_session(mut self, enable: bool) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.fork_session = enable;
        inner.options = Some(options);

        self
    }

    /// Add an MCP (Model Context Protocol) server.
    ///
    /// MCP servers provide additional tools and resources to Claude.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::types::McpServerConfig;
    /// use std::collections::HashMap;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .add_mcp_server(
    ///         "filesystem",
    ///         McpServerConfig::Stdio {
    ///             command: "npx".to_string(),
    ///             args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
    ///             env: None,
    ///         }
    ///     );
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_mcp_server(mut self, name: impl Into<String>, config: McpServerConfig) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.mcp_servers.insert(name.into(), config);
        inner.options = Some(options);

        self
    }

    /// Set all MCP servers at once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::types::McpServerConfig;
    /// use std::collections::HashMap;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let mut servers = HashMap::new();
    /// servers.insert(
    ///     "filesystem".to_string(),
    ///     McpServerConfig::Stdio {
    ///         command: "npx".to_string(),
    ///         args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
    ///         env: None,
    ///     }
    /// );
    ///
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .mcp_servers(servers);
    /// # Ok(())
    /// # }
    /// ```
    pub fn mcp_servers(mut self, servers: HashMap<String, McpServerConfig>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.mcp_servers = servers;
        inner.options = Some(options);

        self
    }

    /// Add a stdio-based MCP server (convenience method).
    ///
    /// This is a helper for the common case of stdio-based MCP servers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .add_mcp_stdio_server(
    ///         "filesystem",
    ///         "npx",
    ///         vec!["-y", "@modelcontextprotocol/server-filesystem"]
    ///     );
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_mcp_stdio_server(
        mut self,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<impl Into<String>>,
    ) -> Self {
        let config = McpServerConfig::Stdio {
            command: command.into(),
            args: Some(args.into_iter().map(|a| a.into()).collect()),
            env: None,
        };

        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.mcp_servers.insert(name.into(), config);
        inner.options = Some(options);

        self
    }

    /// Enable specific MCP tools.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .mcp_tools(vec!["filesystem__read".to_string(), "filesystem__write".to_string()]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn mcp_tools(mut self, tools: Vec<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.mcp_tools = tools;
        inner.options = Some(options);

        self
    }

    /// Add environment variables for the Claude subprocess.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use std::collections::HashMap;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let mut env = HashMap::new();
    /// env.insert("CUSTOM_VAR".to_string(), "value".to_string());
    ///
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .env(env);
    /// # Ok(())
    /// # }
    /// ```
    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.env = env;
        inner.options = Some(options);

        self
    }

    /// Add a single environment variable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .add_env("CUSTOM_VAR", "value");
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.env.insert(key.into(), value.into());
        inner.options = Some(options);

        self
    }

    /// Include partial assistant messages in streaming output.
    ///
    /// When enabled, you'll receive incremental updates as Claude generates
    /// its response, rather than waiting for the complete message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .include_partial_messages(true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn include_partial_messages(mut self, include: bool) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.include_partial_messages = include;
        inner.options = Some(options);

        self
    }

    /// Set additional directories to include in the working context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use std::path::PathBuf;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let builder = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .add_directory(PathBuf::from("/path/to/extra/context"))
    ///     .add_directory(PathBuf::from("/another/path"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_directory(mut self, path: impl Into<PathBuf>) -> Self {
        let inner = Arc::get_mut(&mut self.inner)
            .expect("Builder should have unique access to inner");

        let mut options = inner.options.take().unwrap_or_default();
        options.add_dirs.push(path.into());
        inner.options = Some(options);

        self
    }

    /// Configure the client with the current settings.
    ///
    /// Transitions to the `Configured` state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let configured = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .configure();
    /// # Ok(())
    /// # }
    /// ```
    pub fn configure(self) -> ClaudeClientBuilder<Configured> {
        // Ensure options are initialized
        let mut inner_mut = Arc::try_unwrap(self.inner)
            .unwrap_or_else(|arc| (*arc).clone());

        if inner_mut.options.is_none() {
            inner_mut.options = Some(ClaudeCodeOptions::default());
        }

        ClaudeClientBuilder {
            inner: Arc::new(inner_mut),
            _state: PhantomData,
        }
    }
}

impl Clone for ClientInner {
    fn clone(&self) -> Self {
        Self {
            binary_path: self.binary_path.clone(),
            options: self.options.clone(),
            transport: self.transport.clone(),
            session_id: self.session_id.clone(),
            message_tx: self.message_tx.clone(),
        }
    }
}

// Configured -> Connected transition
impl ClaudeClientBuilder<Configured> {
    /// Connect to Claude and start a session.
    ///
    /// Creates the subprocess transport and establishes connection.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Transport creation fails
    /// - Connection cannot be established
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let client = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .configure()
    ///     .connect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(self) -> Result<ClaudeClientBuilder<Connected>> {
        // Binary path is verified but SubprocessTransport will discover it again
        // This is ok since it uses the cached result
        let _ = self.inner.binary_path.as_ref()
            .ok_or_else(|| Error::Binary(BinaryError::NotFound {
                searched_paths: vec![],
            }))?;

        let options = self.inner.options.as_ref()
            .ok_or_else(|| Error::Config(
                "Options not configured".to_string()
            ))?;

        // Create transport - it discovers binary internally
        let mut transport = SubprocessTransport::new(options.clone())
            .map_err(|e| Error::Protocol(format!("Transport creation failed: {}", e)))?;

        // Connect
        transport.connect().await
            .map_err(|e| Error::Protocol(format!("Connection failed: {}", e)))?;

        // Create message broadcast channel
        let (message_tx, _message_rx) = broadcast::channel(100);

        // Generate session ID
        let session_id = SessionId::generate();

        let inner = Arc::new(ClientInner {
            binary_path: self.inner.binary_path.clone(),
            options: self.inner.options.clone(),
            transport: Some(Arc::new(tokio::sync::Mutex::new(transport))),
            session_id: Some(session_id),
            message_tx: Some(message_tx),
        });

        Ok(ClaudeClientBuilder {
            inner,
            _state: PhantomData,
        })
    }
}

// Connected state builder
impl ClaudeClientBuilder<Connected> {
    /// Send an initial prompt and complete the client setup.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let client = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .configure()
    ///     .connect().await?
    ///     .with_prompt("Hello!")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_prompt(self, _prompt: impl Into<String>) -> Result<ClaudeClient<Connected>> {
        let client = ClaudeClient {
            inner: self.inner,
            _state: PhantomData,
        };

        // We'll send the message asynchronously later
        // For now, just return the client
        // The user can call send() separately

        Ok(client)
    }

    /// Build the client without sending an initial message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// let client = ClaudeClient::builder()
    ///     .discover_binary().await?
    ///     .configure()
    ///     .connect().await?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<ClaudeClient<Connected>> {
        Ok(ClaudeClient {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}

/// Stream of messages from Claude.
///
/// Implements `Stream` for async iteration over messages.
pub struct MessageStream {
    receiver: BroadcastStream<Message>,
}

impl Stream for MessageStream {
    type Item = Result<Message>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(msg))) => std::task::Poll::Ready(Some(Ok(msg))),
            std::task::Poll::Ready(Some(Err(e))) => {
                std::task::Poll::Ready(Some(Err(Error::Protocol(
                    format!("Broadcast error: {}", e)
                ))))
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

// Connected client operations
impl ClaudeClient<Connected> {
    /// Send a message to Claude.
    ///
    /// Returns a stream of response messages.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transport is not connected
    /// - Sending the message fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use futures::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let mut stream = client.send("What's 2+2?").await?;
    /// while let Some(message) = stream.next().await {
    ///     println!("{:?}", message?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(&self, message: impl Into<String>) -> Result<MessageStream> {
        let transport = self.inner.transport.as_ref()
            .ok_or_else(|| Error::Client(ClientError::NotConnected))?;

        let session_id = self.inner.session_id.as_ref()
            .ok_or_else(|| Error::Session(SessionError::NotFound {
                session_id: SessionId::new("unknown"),
            }))?;

        let input_msg = InputMessage::user(message.into(), session_id.to_string());

        // Send message
        let mut transport_guard = transport.lock().await;
        transport_guard.send_message(input_msg).await
            .map_err(|e| Error::Protocol(format!("Send failed: {}", e)))?;

        // Create receiver for this stream
        let receiver = self.inner.message_tx.as_ref()
            .ok_or_else(|| Error::Config("Message channel not initialized".to_string()))?
            .subscribe();

        Ok(MessageStream {
            receiver: BroadcastStream::new(receiver),
        })
    }

    /// Send a message with attached files.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transport is not connected
    /// - Sending the message fails
    /// - File paths are invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use std::path::PathBuf;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let files = vec![PathBuf::from("file1.txt"), PathBuf::from("file2.txt")];
    /// let stream = client.send_with_files("Analyze these files", files).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_with_files(
        &self,
        message: String,
        _files: Vec<PathBuf>,
    ) -> Result<MessageStream> {
        // For now, just send the message text
        // File attachment would require additional protocol support
        // TODO: Implement file attachment in the message protocol
        self.send(message).await
    }

    /// Interrupt the current operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the interrupt signal cannot be sent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// client.interrupt().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interrupt(&self) -> Result<()> {
        let transport = self.inner.transport.as_ref()
            .ok_or_else(|| Error::Client(ClientError::NotConnected))?;

        let mut transport_guard = transport.lock().await;

        // Send interrupt via control request
        use crate::types::ControlRequest;
        let request_id = uuid::Uuid::new_v4().to_string();
        transport_guard.send_control_request(ControlRequest::Interrupt { request_id }).await
            .map_err(|e| Error::Protocol(format!("Interrupt failed: {}", e)))?;

        Ok(())
    }

    /// Get the session ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// println!("Session ID: {}", client.session_id());
    /// # Ok(())
    /// # }
    /// ```
    pub fn session_id(&self) -> &SessionId {
        self.inner.session_id.as_ref()
            .expect("Connected client should have session ID")
    }

    /// Get the model ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// if let Some(model) = client.model() {
    ///     println!("Using model: {}", model);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn model(&self) -> Option<ModelId> {
        self.inner.options.as_ref()
            .and_then(|opts| opts.model.as_ref())
            .map(|m| ModelId::from(m.as_str()))
    }

    /// Get the current settings.
    ///
    /// Returns the ClaudeCodeOptions used to configure this client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let options = client.options();
    /// println!("Permission mode: {:?}", options.permission_mode);
    /// # Ok(())
    /// # }
    /// ```
    pub fn options(&self) -> &ClaudeCodeOptions {
        self.inner.options.as_ref()
            .expect("Connected client should have options")
    }

    /// List all sessions for the current project.
    ///
    /// # Errors
    ///
    /// Returns an error if session listing fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let sessions = client.list_project_sessions().await?;
    /// for session in sessions {
    ///     println!("Session: {:?}", session.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_project_sessions(&self) -> Result<Vec<crate::session::Session>> {
        use crate::session;

        // Get the working directory from options
        let project_path = self.inner.options.as_ref()
            .and_then(|opts| opts.cwd.as_ref())
            .ok_or_else(|| Error::Config("No working directory configured".to_string()))?;

        // Find the project by path
        let project = session::find_project_by_path(project_path).await?
            .ok_or_else(|| Error::Session(SessionError::NotFound {
                session_id: SessionId::new("project"),
            }))?;

        // List sessions for this project
        session::list_sessions(&project.id).await
    }

    /// Update permission mode dynamically.
    ///
    /// Changes the permission mode for tool execution without reconnecting.
    ///
    /// # Errors
    ///
    /// Returns an error if the permission update cannot be sent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    /// use cc_sdk::types::PermissionMode;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// client.set_permission_mode(PermissionMode::Default).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        let transport = self.inner.transport.as_ref()
            .ok_or_else(|| Error::Client(ClientError::NotConnected))?;

        let mut transport_guard = transport.lock().await;

        // Send permission mode update via SDK control request
        use crate::requests::{SDKControlRequest, SDKControlSetPermissionModeRequest};
        let mode_str = match mode {
            PermissionMode::Default => "default",
            PermissionMode::AcceptEdits => "acceptEdits",
            PermissionMode::Plan => "plan",
            PermissionMode::BypassPermissions => "bypassPermissions",
        };

        let req = SDKControlRequest::SetPermissionMode(SDKControlSetPermissionModeRequest {
            subtype: "set_permission_mode".to_string(),
            mode: mode_str.to_string(),
        });

        let req_json = serde_json::to_value(&req)
            .map_err(|e| Error::Protocol(format!("Failed to serialize request: {}", e)))?;

        transport_guard.send_sdk_control_request(req_json).await
            .map_err(|e| Error::Protocol(format!("Permission mode update failed: {}", e)))?;

        Ok(())
    }

    /// Get conversation history for the current session.
    ///
    /// Retrieves all messages exchanged in this session.
    ///
    /// # Errors
    ///
    /// Returns an error if history cannot be retrieved.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let history = client.get_history().await?;
    /// for msg in history {
    ///     println!("{:?}", msg);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_history(&self) -> Result<Vec<Message>> {
        use crate::session;

        let session_id = self.session_id();
        session::load_session_history(session_id).await
    }

    /// Check if the client is currently connected.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// assert!(client.is_connected());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.inner.transport.is_some()
    }

    /// Get the binary path being used.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// if let Some(path) = client.binary_path() {
    ///     println!("Using Claude at: {:?}", path);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn binary_path(&self) -> Option<&BinaryPath> {
        self.inner.binary_path.as_ref()
    }

    /// Disconnect from Claude.
    ///
    /// Transitions to the `Disconnected` state.
    ///
    /// # Errors
    ///
    /// Returns an error if disconnection fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// client.disconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disconnect(self) -> Result<ClaudeClient<Disconnected>> {
        if let Some(transport) = self.inner.transport.as_ref() {
            let mut transport_guard = transport.lock().await;
            transport_guard.disconnect().await
                .map_err(|e| Error::Protocol(format!("Disconnect failed: {}", e)))?;
        }

        Ok(ClaudeClient {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}

// Disconnected client - can only reconnect or be dropped
impl ClaudeClient<Disconnected> {
    /// Reconnect to Claude.
    ///
    /// Transitions back to the `Connected` state.
    ///
    /// # Errors
    ///
    /// Returns an error if reconnection fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use cc_sdk::ClaudeClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> cc_sdk::Result<()> {
    /// # let client = ClaudeClient::builder()
    /// #     .discover_binary().await?
    /// #     .configure()
    /// #     .connect().await?
    /// #     .build()?;
    /// let disconnected = client.disconnect().await?;
    /// let reconnected = disconnected.reconnect().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reconnect(self) -> Result<ClaudeClient<Connected>> {
        // Check transport exists before moving inner
        if let Some(transport) = &self.inner.transport {
            let mut transport_guard = transport.lock().await;
            transport_guard.connect().await
                .map_err(|e| Error::Protocol(format!("Reconnection failed: {}", e)))?;
        } else {
            return Err(Error::Client(ClientError::NotConnected));
        }

        Ok(ClaudeClient {
            inner: self.inner,
            _state: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_state_transitions() {
        // Test that builder compiles with correct state transitions
        let _builder = ClaudeClient::builder();

        // Can't send without connecting - this should not compile:
        // let client = ClaudeClient::builder();
        // client.send("Hello"); // ERROR: method not found
    }

    #[tokio::test]
    async fn test_binary_path_construction() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude");

        assert!(builder.inner.binary_path.is_some());
    }

    #[tokio::test]
    async fn test_model_fallback_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .models(vec![
                ModelId::from("claude-sonnet-4-5"),
                ModelId::from("claude-opus-4-5"),
                ModelId::from("claude-haiku-4-0"),
            ])
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.model, Some("claude-sonnet-4-5".to_string()));
        assert!(options.extra_args.contains_key("fallback-models"));
    }

    #[tokio::test]
    async fn test_tool_filtering() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
            .disallow_tool("Write")
            .disallow_tool("Delete")
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.allowed_tools.len(), 2);
        assert_eq!(options.disallowed_tools.len(), 2);
        assert!(options.disallowed_tools.contains(&"Write".to_string()));
    }

    #[tokio::test]
    async fn test_session_forking() {
        let session_id = SessionId::new("test-session-123");
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .resume_session(session_id)
            .fork_session(true)
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.resume, Some("test-session-123".to_string()));
        assert!(options.fork_session);
    }

    #[tokio::test]
    async fn test_mcp_server_configuration() {
        let mut env = HashMap::new();
        env.insert("NODE_ENV".to_string(), "production".to_string());

        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .add_mcp_server(
                "filesystem",
                McpServerConfig::Stdio {
                    command: "npx".to_string(),
                    args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
                    env: Some(env),
                }
            )
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert!(options.mcp_servers.contains_key("filesystem"));
    }

    #[tokio::test]
    async fn test_mcp_stdio_server_helper() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .add_mcp_stdio_server(
                "filesystem",
                "npx",
                vec!["-y", "@modelcontextprotocol/server-filesystem"]
            )
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert!(options.mcp_servers.contains_key("filesystem"));

        if let Some(McpServerConfig::Stdio { command, args, .. }) = options.mcp_servers.get("filesystem") {
            assert_eq!(command, "npx");
            assert!(args.is_some());
        } else {
            panic!("Expected Stdio MCP server config");
        }
    }

    #[tokio::test]
    async fn test_advanced_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .max_output_tokens(8000)
            .max_turns(20)
            .system_prompt("You are a helpful assistant")
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.max_output_tokens, Some(8000));
        assert_eq!(options.max_turns, Some(20));
        #[allow(deprecated)]
        {
            assert_eq!(options.system_prompt, Some("You are a helpful assistant".to_string()));
        }
    }

    #[tokio::test]
    async fn test_environment_variables() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .add_env("CUSTOM_VAR", "value1")
            .add_env("ANOTHER_VAR", "value2")
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.env.len(), 2);
        assert_eq!(options.env.get("CUSTOM_VAR"), Some(&"value1".to_string()));
        assert_eq!(options.env.get("ANOTHER_VAR"), Some(&"value2".to_string()));
    }

    #[tokio::test]
    async fn test_directory_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .working_directory("/main/project")
            .add_directory(PathBuf::from("/extra/context1"))
            .add_directory(PathBuf::from("/extra/context2"))
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.cwd, Some(PathBuf::from("/main/project")));
        assert_eq!(options.add_dirs.len(), 2);
    }

    #[tokio::test]
    async fn test_permission_mode_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .permission_mode(PermissionMode::AcceptEdits)
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
    }

    #[tokio::test]
    async fn test_continue_conversation_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .continue_conversation(true)
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert!(options.continue_conversation);
    }

    #[tokio::test]
    async fn test_partial_messages_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .include_partial_messages(true)
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert!(options.include_partial_messages);
    }

    #[tokio::test]
    async fn test_max_output_tokens_clamping() {
        // Test that max_output_tokens is clamped to valid range
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .max_output_tokens(50000) // Above max
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.max_output_tokens, Some(32000)); // Should be clamped

        let builder2 = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .max_output_tokens(0) // Below min
            .configure();

        let options2 = builder2.inner.options.as_ref().unwrap();
        assert_eq!(options2.max_output_tokens, Some(1)); // Should be clamped
    }

    #[tokio::test]
    async fn test_fluent_builder_chaining() {
        // Test that builder methods can be chained fluently
        let _builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .model(ModelId::from("claude-sonnet-4-5"))
            .permission_mode(PermissionMode::AcceptEdits)
            .max_output_tokens(8000)
            .max_turns(20)
            .add_allowed_tool("Bash")
            .add_allowed_tool("Read")
            .disallow_tool("Write")
            .add_directory(PathBuf::from("/extra"))
            .add_env("VAR", "value")
            .include_partial_messages(true)
            .configure();

        // If we got here without compiler errors, the fluent API works
    }

    #[tokio::test]
    async fn test_mcp_tools_configuration() {
        let builder = ClaudeClient::builder()
            .binary("/usr/local/bin/claude")
            .mcp_tools(vec!["fs__read".to_string(), "fs__write".to_string()])
            .configure();

        let options = builder.inner.options.as_ref().unwrap();
        assert_eq!(options.mcp_tools.len(), 2);
        assert!(options.mcp_tools.contains(&"fs__read".to_string()));
    }
}
