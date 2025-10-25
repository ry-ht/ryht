//! Subprocess-based transport implementation
//!
//! This module implements the Transport trait using a subprocess to run the Claude CLI.

use super::{InputMessage, Transport, TransportState};
use crate::cc::{
    result::Result,
    messages::Message,
    options::ClaudeCodeOptions,
    permissions::PermissionMode,
    requests::{ControlRequest, ControlResponse},
};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default buffer size for channels
const CHANNEL_BUFFER_SIZE: usize = 100;

/// Minimum required CLI version
const MIN_CLI_VERSION: (u32, u32, u32) = (2, 0, 0);

/// Simple semantic version struct
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SemVer {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SemVer {
    fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse semantic version from string (e.g., "2.0.0" or "v2.0.0")
    fn parse(version: &str) -> Option<Self> {
        let version = version.trim().trim_start_matches('v');

        // Handle versions like "@anthropic-ai/claude-code/2.0.0"
        let version = if let Some(v) = version.split('/').next_back() {
            v
        } else {
            version
        };

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts.get(1)?.parse().ok()?,
            patch: parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
        })
    }
}

/// Subprocess-based transport for Claude CLI
pub struct SubprocessTransport {
    /// Configuration options
    options: ClaudeCodeOptions,
    /// CLI binary path
    cli_path: PathBuf,
    /// Child process
    child: Option<Child>,
    /// Sender for stdin
    stdin_tx: Option<mpsc::Sender<String>>,
    /// Sender for broadcasting messages to multiple receivers
    message_broadcast_tx: Option<tokio::sync::broadcast::Sender<Message>>,
    /// Receiver for control responses
    control_rx: Option<mpsc::Receiver<ControlResponse>>,
    /// Receiver for SDK control requests
    sdk_control_rx: Option<mpsc::Receiver<serde_json::Value>>,
    /// Transport state
    state: TransportState,
    /// Request counter for control requests
    request_counter: u64,
    /// Whether to close stdin after initial prompt
    #[allow(dead_code)]
    close_stdin_after_prompt: bool,
}

impl SubprocessTransport {
    /// Create a new subprocess transport
    pub fn new(options: ClaudeCodeOptions) -> Result<Self> {
        let cli_path = find_claude_cli()?;
        Ok(Self {
            options,
            cli_path,
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        })
    }
    
    /// Subscribe to messages without borrowing self (for lock-free consumption)
    pub fn subscribe_messages(&self) -> Option<Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>>> {
        self.message_broadcast_tx.as_ref().map(|tx| {
            let rx = tx.subscribe();
            Box::pin(tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                |result| async move {
                    match result {
                        Ok(msg) => Some(Ok(msg)),
                        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                            warn!("Receiver lagged by {} messages", n);
                            None
                        }
                    }
                },
            )) as Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>>
        })
    }

    /// Receive SDK control requests
    #[allow(dead_code)]
    pub async fn receive_sdk_control_request(&mut self) -> Option<serde_json::Value> {
        if let Some(ref mut rx) = self.sdk_control_rx {
            rx.recv().await
        } else {
            None
        }
    }
    
    /// Take the SDK control receiver (can only be called once)
    pub fn take_sdk_control_receiver(&mut self) -> Option<mpsc::Receiver<serde_json::Value>> {
        self.sdk_control_rx.take()
    }

    /// Create with a specific CLI path
    pub fn with_cli_path(options: ClaudeCodeOptions, cli_path: impl Into<PathBuf>) -> Self {
        Self {
            options,
            cli_path: cli_path.into(),
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: false,
        }
    }

    /// Set whether to close stdin after sending the initial prompt
    #[allow(dead_code)]
    pub fn set_close_stdin_after_prompt(&mut self, close: bool) {
        self.close_stdin_after_prompt = close;
    }

    /// Create transport for simple print mode (one-shot query)
    #[allow(dead_code)]
    pub fn for_print_mode(options: ClaudeCodeOptions, _prompt: String) -> Result<Self> {
        let cli_path = find_claude_cli()?;
        Ok(Self {
            options,
            cli_path,
            child: None,
            stdin_tx: None,
            message_broadcast_tx: None,
            control_rx: None,
            sdk_control_rx: None,
            state: TransportState::Disconnected,
            request_counter: 0,
            close_stdin_after_prompt: true,
        })
    }

    /// Build the command with all necessary arguments
    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.cli_path);

        // Output format
        let output_format_str = match self.options.output_format {
            crate::cc::options::OutputFormat::Text => "text",
            crate::cc::options::OutputFormat::Json => "json",
            crate::cc::options::OutputFormat::StreamJson => "stream-json",
        };
        cmd.arg("--output-format").arg(output_format_str);

        // Always add verbose for better logging
        cmd.arg("--verbose");

        // Input format (only for non-print mode)
        if !self.options.print_mode {
            let input_format_str = match self.options.input_format {
                crate::cc::options::InputFormat::Text => "text",
                crate::cc::options::InputFormat::StreamJson => "stream-json",
            };
            cmd.arg("--input-format").arg(input_format_str);
        }

        // Print mode (non-interactive)
        if self.options.print_mode {
            cmd.arg("--print");
        }

        // Debug mode with optional filtering
        if let Some(ref debug_filter) = self.options.debug_mode {
            if debug_filter.is_empty() {
                // Enable debug for all categories
                cmd.arg("--debug");
            } else {
                // Enable debug with specific categories
                cmd.arg("--debug").arg(debug_filter);
            }
        }
        
        // Include partial messages if requested
        if self.options.include_partial_messages {
            cmd.arg("--include-partial-messages");
        }
        
        // Add debug-to-stderr flag if debug_stderr is set
        if self.options.debug_stderr.is_some() {
            cmd.arg("--debug-to-stderr");
        }
        
        // Handle max_output_tokens (priority: option > env var)
        // Maximum safe value is 32000, values above this may cause issues
        if let Some(max_tokens) = self.options.max_output_tokens {
            // Option takes priority - validate and cap at 32000
            let capped = max_tokens.clamp(1, 32000);
            cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", capped.to_string());
            debug!("Setting max_output_tokens from option: {}", capped);
        } else {
            // Fall back to environment variable handling
            if let Ok(current_value) = std::env::var("CLAUDE_CODE_MAX_OUTPUT_TOKENS") {
                if let Ok(tokens) = current_value.parse::<u32>() {
                    if tokens > 32000 {
                        warn!("CLAUDE_CODE_MAX_OUTPUT_TOKENS={} exceeds maximum safe value of 32000, overriding to 32000", tokens);
                        cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "32000");
                    }
                    // If it's <= 32000, leave it as is
                } else {
                    // Invalid value, set to safe default
                    warn!("Invalid CLAUDE_CODE_MAX_OUTPUT_TOKENS value: {}, setting to 8192", current_value);
                    cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
                }
            }
        }

        // System prompts
        if let Some(ref prompt) = self.options.system_prompt {
            match prompt {
                crate::cc::options::SystemPrompt::String(s) => {
                    cmd.arg("--system-prompt").arg(s);
                }
                crate::cc::options::SystemPrompt::Preset { preset, append, .. } => {
                    // Use preset-based prompt
                    cmd.arg("--system-prompt-preset").arg(preset);

                    // Append if specified
                    if let Some(append_text) = append {
                        cmd.arg("--append-system-prompt").arg(append_text);
                    }
                }
            }
        }

        // Tool configuration
        if !self.options.allowed_tools.is_empty() {
            cmd.arg("--allowedTools")
                .arg(self.options.allowed_tools.join(","));
        }
        if !self.options.disallowed_tools.is_empty() {
            cmd.arg("--disallowedTools")
                .arg(self.options.disallowed_tools.join(","));
        }

        // Permission mode
        match self.options.permission_mode {
            PermissionMode::Default => {
                cmd.arg("--permission-mode").arg("default");
            }
            PermissionMode::AcceptEdits => {
                cmd.arg("--permission-mode").arg("acceptEdits");
            }
            PermissionMode::Plan => {
                cmd.arg("--permission-mode").arg("plan");
            }
            PermissionMode::BypassPermissions => {
                cmd.arg("--permission-mode").arg("bypassPermissions");
            }
        }

        // Model
        if let Some(ref model) = self.options.model {
            cmd.arg("--model").arg(model);
        }

        // Permission prompt tool
        if let Some(ref tool_name) = self.options.permission_prompt_tool_name {
            cmd.arg("--permission-prompt-tool").arg(tool_name);
        }

        // Max turns
        if let Some(max_turns) = self.options.max_turns {
            cmd.arg("--max-turns").arg(max_turns.to_string());
        }

        // Note: max_thinking_tokens is not currently supported by Claude CLI

        // Working directory
        if let Some(ref cwd) = self.options.cwd {
            cmd.current_dir(cwd);
        }
        
        // Add environment variables
        for (key, value) in &self.options.env {
            cmd.env(key, value);
        }

        // MCP servers - use --mcp-config with JSON format like Python SDK
        if !self.options.mcp_servers.is_empty() {
            let mcp_config = serde_json::json!({
                "mcpServers": self.options.mcp_servers
            });
            cmd.arg("--mcp-config").arg(mcp_config.to_string());
        }

        // Continue/resume
        if self.options.continue_conversation {
            cmd.arg("--continue");
        }
        if let Some(ref resume_id) = self.options.resume {
            cmd.arg("--resume").arg(resume_id);
        }

        // Settings file
        if let Some(ref settings) = self.options.settings {
            cmd.arg("--settings").arg(settings);
        }

        // Additional directories
        for dir in &self.options.add_dirs {
            cmd.arg("--add-dir").arg(dir);
        }

        // Fork session if requested
        if self.options.fork_session {
            cmd.arg("--fork-session");
        }

        // Programmatic agents
        if let Some(ref agents) = self.options.agents
            && !agents.is_empty()
                && let Ok(json_str) = serde_json::to_string(agents) {
                    cmd.arg("--agents").arg(json_str);
                }

        // Setting sources (comma-separated)
        if let Some(ref sources) = self.options.setting_sources
            && !sources.is_empty() {
                let value = sources.iter().map(|s| (match s { crate::cc::options::SettingSource::User => "user", crate::cc::options::SettingSource::Project => "project", crate::cc::options::SettingSource::Local => "local" }).to_string()).collect::<Vec<_>>().join(",");
                cmd.arg("--setting-sources").arg(value);
            }

        // Fallback model
        if let Some(ref fallback_model) = self.options.fallback_model {
            cmd.arg("--fallback-model").arg(fallback_model);
        }

        // IDE auto-connect
        if self.options.ide_autoconnect {
            cmd.arg("--ide");
        }

        // Strict MCP config
        if self.options.strict_mcp_config {
            cmd.arg("--strict-mcp-config");
        }

        // Custom session ID
        if let Some(ref session_id) = self.options.custom_session_id {
            cmd.arg("--session-id").arg(session_id.to_string());
        }

        // Replay user messages
        if self.options.replay_user_messages {
            cmd.arg("--replay-user-messages");
        }

        // Extra arguments
        for (key, value) in &self.options.extra_args {
            let flag = if key.starts_with("--") || key.starts_with("-") {
                key.clone()
            } else {
                format!("--{key}")
            };
            cmd.arg(&flag);
            if let Some(val) = value {
                cmd.arg(val);
            }
        }

        // Set up process pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables to indicate SDK usage and version
        cmd.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        cmd.env("CLAUDE_AGENT_SDK_VERSION", env!("CARGO_PKG_VERSION"));

        cmd
    }

    /// Check CLI version and warn if below minimum required version
    async fn check_cli_version(&self) -> Result<()> {
        // Run the CLI with --version flag (with a timeout to avoid hanging)
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::process::Command::new(&self.cli_path)
                .arg("--version")
                .output(),
        )
        .await;

        let output = match output {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                warn!("Failed to check CLI version: {}", e);
                return Ok(()); // Don't fail connection, just warn
            }
            Err(_) => {
                warn!("CLI version check timed out after 5 seconds");
                return Ok(());
            }
        };

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim();

        if let Some(semver) = SemVer::parse(version_str) {
            let min_version = SemVer::new(MIN_CLI_VERSION.0, MIN_CLI_VERSION.1, MIN_CLI_VERSION.2);

            if semver < min_version {
                warn!(
                    "⚠️  Claude CLI version {}.{}.{} is below minimum required version {}.{}.{}",
                    semver.major,
                    semver.minor,
                    semver.patch,
                    MIN_CLI_VERSION.0,
                    MIN_CLI_VERSION.1,
                    MIN_CLI_VERSION.2
                );
                warn!(
                    "   Some features may not work correctly. Please upgrade with: npm install -g @anthropic-ai/claude-code@latest"
                );
            } else {
                info!("Claude CLI version: {}.{}.{}", semver.major, semver.minor, semver.patch);
            }
        } else {
            debug!("Could not parse CLI version: {}", version_str);
        }

        Ok(())
    }

    /// Spawn the process and set up communication channels
    async fn spawn_process(&mut self) -> Result<()> {
        self.state = TransportState::Connecting;

        let mut cmd = self.build_command();
        info!("Starting Claude CLI with command: {:?}", cmd);

        let mut child = cmd.spawn().map_err(|e| {
            error!("Failed to spawn Claude CLI: {}", e);
            crate::cc::error::Error::Binary(crate::cc::error::BinaryError::SpawnFailed {
                path: self.cli_path.clone(),
                reason: format!("Failed to spawn process: {}", e),
                source: e,
            })
        })?;

        // Get stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| crate::cc::error::Error::Transport(crate::cc::error::TransportError::ChannelError("Failed to get stdin".into())))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| crate::cc::error::Error::Transport(crate::cc::error::TransportError::ChannelError("Failed to get stdout".into())))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| crate::cc::error::Error::Transport(crate::cc::error::TransportError::ChannelError("Failed to get stderr".into())))?;

        // Determine buffer size from options or use default
        let buffer_size = self.options.cli_channel_buffer_size.unwrap_or(CHANNEL_BUFFER_SIZE);

        // Create channels
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(buffer_size);
        // Use broadcast channel for messages to support multiple receivers
        let (message_broadcast_tx, _) =
            tokio::sync::broadcast::channel::<Message>(buffer_size);
        let (control_tx, control_rx) = mpsc::channel::<ControlResponse>(buffer_size);

        // Spawn stdin handler
        tokio::spawn(async move {
            let mut stdin = stdin;
            debug!("Stdin handler started");
            while let Some(line) = stdin_rx.recv().await {
                debug!("Received line from channel: {}", line);
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    error!("Failed to write to stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.write_all(b"\n").await {
                    error!("Failed to write newline: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush stdin: {}", e);
                    break;
                }
                debug!("Successfully sent to Claude process: {}", line);
            }
            debug!("Stdin handler ended");
        });

        // Create channel for SDK control requests
        let (sdk_control_tx, sdk_control_rx) = mpsc::channel::<serde_json::Value>(buffer_size);
        
        // Spawn stdout handler
        let message_broadcast_tx_clone = message_broadcast_tx.clone();
        let control_tx_clone = control_tx.clone();
        let sdk_control_tx_clone = sdk_control_tx.clone();
        tokio::spawn(async move {
            debug!("Stdout handler started");
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                debug!("Claude output: {}", line);

                // Try to parse as JSON
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(json) => {
                        // Check message type
                        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                            // Handle control responses - these are responses to OUR control requests
                            if msg_type == "control_response" {
                                debug!("Received control response: {:?}", json);

                                // Send to sdk_control channel for control protocol mode
                                let _ = sdk_control_tx_clone.send(json.clone()).await;

                                // Also parse and send to control_tx for non-control-protocol mode
                                // (needed for interrupt functionality when query_handler is None)
                                // CLI returns: {"type":"control_response","response":{"subtype":"success","request_id":"..."}}
                                // or: {"type":"control_response","response":{"subtype":"error","request_id":"...","error":"..."}}
                                if let Some(response_obj) = json.get("response")
                                    && let Some(request_id) = response_obj.get("request_id")
                                        .or_else(|| response_obj.get("requestId"))
                                        .and_then(|v| v.as_str())
                                    {
                                        // Determine success from subtype
                                        let subtype = response_obj.get("subtype").and_then(|v| v.as_str());
                                        let success = subtype == Some("success");

                                        let control_resp = ControlResponse::InterruptAck {
                                            request_id: request_id.to_string(),
                                            success,
                                        };
                                        let _ = control_tx_clone.send(control_resp).await;
                                    }
                                continue;
                            }

                            // Handle control requests FROM CLI (standard format)
                            if msg_type == "control_request" {
                                debug!("Received control request from CLI: {:?}", json);
                                // Send the FULL message including requestId and request
                                let _ = sdk_control_tx_clone.send(json.clone()).await;
                                continue;
                            }

                            // Handle control messages (new format)
                            if msg_type == "control"
                                && let Some(control) = json.get("control") {
                                    debug!("Received control message: {:?}", control);
                                    let _ = sdk_control_tx_clone.send(control.clone()).await;
                                    continue;
                                }

                            // Handle SDK control requests FROM CLI
                            if msg_type == "sdk_control_request" {
                                // Send the FULL message including requestId
                                debug!("Received SDK control request: {:?}", json);
                                let _ = sdk_control_tx_clone.send(json.clone()).await;
                                continue;
                            }
                            
                            // Check for system messages with SDK control subtypes
                            if msg_type == "system"
                                && let Some(subtype) = json.get("subtype").and_then(|v| v.as_str())
                                    && subtype.starts_with("sdk_control:") {
                                        // This is an SDK control message
                                        debug!("Received SDK control message: {}", subtype);
                                        let _ = sdk_control_tx_clone.send(json.clone()).await;
                                        // Still parse as regular message for now
                                    }
                        }

                        // Try to parse as a regular message
                        match crate::cc::message_parser::parse_message(json) {
                            Ok(Some(message)) => {
                                // Use broadcast send which doesn't fail if no receivers
                                let _ = message_broadcast_tx_clone.send(message);
                            }
                            Ok(None) => {
                                // Ignore non-message JSON
                            }
                            Err(e) => {
                                warn!("Failed to parse message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse JSON: {} - Line: {}", e, line);
                    }
                }
            }
            info!("Stdout reader ended");
        });

        // Spawn stderr handler - capture error messages for better diagnostics
        let message_broadcast_tx_for_error = message_broadcast_tx.clone();
        let debug_stderr = self.options.debug_stderr.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut error_buffer = Vec::new();
            
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    // If debug_stderr is set, write to it
                    if let Some(ref debug_output) = debug_stderr {
                        let mut output = debug_output.lock().await;
                        let _ = writeln!(output, "{line}");
                        let _ = output.flush();
                    }
                    
                    error!("Claude CLI stderr: {}", line);
                    error_buffer.push(line.clone());
                    
                    // Check for common error patterns
                    if line.contains("command not found") || line.contains("No such file") {
                        error!("Claude CLI binary not found or not executable");
                    } else if line.contains("ENOENT") || line.contains("spawn") {
                        error!("Failed to spawn Claude CLI process - binary may not be installed");
                    } else if line.contains("authentication") || line.contains("API key") || line.contains("Unauthorized") {
                        error!("Claude CLI authentication error - please run 'claude-code api login'");
                    } else if line.contains("model") && (line.contains("not available") || line.contains("not found")) {
                        error!("Model not available for your account: {}", line);
                    } else if line.contains("Error:") || line.contains("error:") {
                        error!("Claude CLI error detected: {}", line);
                    }
                }
            }
            
            // If we collected any errors, log them
            if !error_buffer.is_empty() {
                let error_msg = error_buffer.join("\n");
                error!("Claude CLI stderr output collected:\n{}", error_msg);
                
                // Try to send an error message through the broadcast channel
                let _ = message_broadcast_tx_for_error.send(Message::System {
                    subtype: "error".to_string(),
                    data: serde_json::json!({
                        "source": "stderr",
                        "error": "Claude CLI error output",
                        "details": error_msg
                    }),
                });
            }
        });

        // Store handles
        self.child = Some(child);
        self.stdin_tx = Some(stdin_tx);
        self.message_broadcast_tx = Some(message_broadcast_tx);
        self.control_rx = Some(control_rx);
        self.sdk_control_rx = Some(sdk_control_rx);
        self.state = TransportState::Connected;

        Ok(())
    }
}

#[async_trait]
impl Transport for SubprocessTransport {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    async fn connect(&mut self) -> Result<()> {
        if self.state == TransportState::Connected {
            return Ok(());
        }

        // Check CLI version before connecting
        if let Err(e) = self.check_cli_version().await {
            warn!("CLI version check failed: {}", e);
        }

        self.spawn_process().await?;
        info!("Connected to Claude CLI");
        Ok(())
    }

    async fn send_message(&mut self, message: InputMessage) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected));
        }

        let json = serde_json::to_string(&message)
            .map_err(|e| crate::cc::error::Error::Transport(crate::cc::error::TransportError::Json(e)))?;
        debug!("Serialized message: {}", json);

        if let Some(ref tx) = self.stdin_tx {
            debug!("Sending message to stdin channel");
            tx.send(json).await?;
            debug!("Message sent to channel");
            Ok(())
        } else {
            Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected))
        }
    }

    fn receive_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Message>> + Send + 'static>> {
        if let Some(ref tx) = self.message_broadcast_tx {
            // Create a new receiver from the broadcast sender
            let rx = tx.subscribe();
            // Convert broadcast receiver to stream
            Box::pin(tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                |result| async move {
                    match result {
                        Ok(msg) => Some(Ok(msg)),
                        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(
                            n,
                        )) => {
                            warn!("Receiver lagged by {} messages", n);
                            None
                        }
                    }
                },
            ))
        } else {
            Box::pin(futures::stream::empty())
        }
    }

    async fn send_control_request(&mut self, request: ControlRequest) -> Result<()> {
        if self.state != TransportState::Connected {
            return Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected));
        }

        self.request_counter += 1;
        let control_msg = match request {
            ControlRequest::Interrupt { request_id } => {
                serde_json::json!({
                    "type": "control_request",
                    "request": {
                        "type": "interrupt",
                        "request_id": request_id
                    }
                })
            }
        };

        let json = serde_json::to_string(&control_msg)
            .map_err(|e| crate::cc::error::Error::Transport(crate::cc::error::TransportError::Json(e)))?;

        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected))
        }
    }

    async fn receive_control_response(&mut self) -> Result<Option<ControlResponse>> {
        if let Some(ref mut rx) = self.control_rx {
            Ok(rx.recv().await)
        } else {
            Ok(None)
        }
    }
    
    async fn send_sdk_control_request(&mut self, request: serde_json::Value) -> Result<()> {
        // The request is already properly formatted as {"type": "control_request", ...}
        // Just send it directly without wrapping
        let json = serde_json::to_string(&request)
            .map_err(|e| crate::cc::error::Error::Transport(crate::cc::error::TransportError::Json(e)))?;

        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected))
        }
    }

    async fn send_sdk_control_response(&mut self, response: serde_json::Value) -> Result<()> {
        // Wrap the response in control_response format expected by CLI
        // The response should have: {"type": "control_response", "response": {...}}
        let control_response = serde_json::json!({
            "type": "control_response",
            "response": response
        });

        let json = serde_json::to_string(&control_response)
            .map_err(|e| crate::cc::error::Error::Transport(crate::cc::error::TransportError::Json(e)))?;

        if let Some(ref tx) = self.stdin_tx {
            tx.send(json).await?;
            Ok(())
        } else {
            Err(crate::cc::error::Error::Client(crate::cc::error::ClientError::NotConnected))
        }
    }

    fn is_connected(&self) -> bool {
        self.state == TransportState::Connected
    }

    async fn disconnect(&mut self) -> Result<()> {
        if self.state != TransportState::Connected {
            return Ok(());
        }

        self.state = TransportState::Disconnecting;

        // Close stdin channel
        self.stdin_tx.take();

        // Kill the child process
        if let Some(mut child) = self.child.take() {
            match child.kill().await {
                Ok(()) => info!("Claude CLI process terminated"),
                Err(e) => warn!("Failed to kill Claude CLI process: {}", e),
            }
        }

        self.state = TransportState::Disconnected;
        Ok(())
    }

    fn take_sdk_control_receiver(&mut self) -> Option<tokio::sync::mpsc::Receiver<serde_json::Value>> {
        self.sdk_control_rx.take()
    }

    async fn end_input(&mut self) -> Result<()> {
        // Close stdin channel to signal end of input
        self.stdin_tx.take();
        Ok(())
    }
}

impl Drop for SubprocessTransport {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Try to kill the process
            let _ = child.start_kill();
        }
    }
}

/// Find the Claude CLI binary
pub(crate) fn find_claude_cli() -> Result<PathBuf> {
    // First check if it's in PATH - try both 'claude' and 'claude-code'
    for cmd_name in &["claude", "claude-code"] {
        if let Ok(path) = which::which(cmd_name) {
            debug!("Found Claude CLI at: {}", path.display());
            return Ok(path);
        }
    }

    // Check common installation locations
    let home = dirs::home_dir().ok_or_else(|| crate::cc::error::Error::Binary(crate::cc::error::BinaryError::NotFound {
        searched_paths: vec![],
    }))?;

    let locations = vec![
        // npm global installations
        home.join(".npm-global/bin/claude"),
        home.join(".npm-global/bin/claude-code"),
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from("/usr/local/bin/claude-code"),
        // Local installations
        home.join(".local/bin/claude"),
        home.join(".local/bin/claude-code"),
        home.join("node_modules/.bin/claude"),
        home.join("node_modules/.bin/claude-code"),
        // Yarn installations
        home.join(".yarn/bin/claude"),
        home.join(".yarn/bin/claude-code"),
        // macOS specific npm location
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/opt/homebrew/bin/claude-code"),
    ];

    let mut searched = Vec::new();
    for path in &locations {
        searched.push(path.display().to_string());
        if path.exists() && path.is_file() {
            debug!("Found Claude CLI at: {}", path.display());
            return Ok(path.clone());
        }
    }

    // Log detailed error information
    warn!("Claude CLI not found in any standard location");
    warn!("Searched paths: {:?}", searched);

    // Check if Node.js is installed
    if which::which("node").is_err() && which::which("npm").is_err() {
        error!("Node.js/npm not found - Claude CLI requires Node.js");
        return Err(crate::cc::error::Error::Binary(crate::cc::error::BinaryError::NotFound {
            searched_paths: searched.into_iter().map(PathBuf::from).collect(),
        }));
    }

    Err(crate::cc::error::Error::Binary(crate::cc::error::BinaryError::NotFound {
        searched_paths: searched.into_iter().map(PathBuf::from).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_claude_cli_error_message() {
        // Test error message format without relying on CLI not being found
        let error = crate::cc::error::Error::Binary(crate::cc::error::BinaryError::NotFound {
            searched_paths: vec![PathBuf::from("/test/path1"), PathBuf::from("/test/path2")],
        });
        let error_msg = error.to_string();
        assert!(error_msg.contains("npm install -g @anthropic-ai/claude-code"));
    }

    #[tokio::test]
    async fn test_transport_lifecycle() {
        let options = ClaudeCodeOptions::default();
        let transport = SubprocessTransport::new(options).unwrap_or_else(|_| {
            // Use a dummy path for testing
            SubprocessTransport::with_cli_path(ClaudeCodeOptions::default(), "/usr/bin/true")
        });

        assert!(!transport.is_connected());
        assert_eq!(transport.state, TransportState::Disconnected);
    }

    #[test]
    fn test_semver_parse() {
        // Test basic version parsing
        let v = SemVer::parse("2.0.0").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);

        // Test with 'v' prefix
        let v = SemVer::parse("v2.1.3").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 3);

        // Test npm-style version
        let v = SemVer::parse("@anthropic-ai/claude-code/2.5.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, 1);

        // Test version without patch
        let v = SemVer::parse("2.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_semver_compare() {
        let v1 = SemVer::new(2, 0, 0);
        let v2 = SemVer::new(2, 0, 1);
        let v3 = SemVer::new(2, 1, 0);
        let v4 = SemVer::new(3, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);

        let min_version = SemVer::new(2, 0, 0);
        assert!(SemVer::new(1, 9, 9) < min_version);
        assert!(SemVer::new(2, 0, 0) >= min_version);
        assert!(SemVer::new(2, 1, 0) >= min_version);
    }

    #[test]
    fn test_build_command_debug_mode() {
        // Test debug mode with no filter
        let mut options = ClaudeCodeOptions::default();
        options.debug_mode = Some("".to_string());
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--debug"));
    }

    #[test]
    fn test_build_command_debug_mode_with_filter() {
        // Test debug mode with specific filter
        let mut options = ClaudeCodeOptions::default();
        options.debug_mode = Some("api,mcp".to_string());
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--debug"));
        assert!(cmd_str.contains("api,mcp"));
    }

    #[test]
    fn test_build_command_print_mode() {
        // Test print mode
        let mut options = ClaudeCodeOptions::default();
        options.print_mode = true;
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--print"));
        // Print mode should not include input-format
        assert!(!cmd_str.contains("--input-format"));
    }

    #[test]
    fn test_build_command_output_formats() {
        // Test text output format
        let mut options = ClaudeCodeOptions::default();
        options.output_format = crate::cc::options::OutputFormat::Text;
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--output-format"));
        assert!(cmd_str.contains("text"));

        // Test JSON output format
        let mut options = ClaudeCodeOptions::default();
        options.output_format = crate::cc::options::OutputFormat::Json;
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--output-format"));
        assert!(cmd_str.contains("json"));

        // Test stream-json output format (default)
        let options = ClaudeCodeOptions::default();
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--output-format"));
        assert!(cmd_str.contains("stream-json"));
    }

    #[test]
    fn test_build_command_input_formats() {
        // Test text input format
        let mut options = ClaudeCodeOptions::default();
        options.input_format = crate::cc::options::InputFormat::Text;
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--input-format"));
        assert!(cmd_str.contains("text"));

        // Test stream-json input format (default)
        let options = ClaudeCodeOptions::default();
        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);
        assert!(cmd_str.contains("--input-format"));
        assert!(cmd_str.contains("stream-json"));
    }

    #[test]
    fn test_build_command_combined_cli_features() {
        // Test combining multiple CLI features
        let mut options = ClaudeCodeOptions::default();
        options.debug_mode = Some("api".to_string());
        options.output_format = crate::cc::options::OutputFormat::Json;
        options.input_format = crate::cc::options::InputFormat::Text;
        options.model = Some("claude-sonnet-4".to_string());

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--debug"));
        assert!(cmd_str.contains("api"));
        assert!(cmd_str.contains("--output-format"));
        assert!(cmd_str.contains("json"));
        assert!(cmd_str.contains("--input-format"));
        assert!(cmd_str.contains("text"));
        assert!(cmd_str.contains("--model"));
        assert!(cmd_str.contains("claude-sonnet-4"));
    }

    #[test]
    fn test_build_command_fallback_model() {
        let mut options = ClaudeCodeOptions::default();
        options.model = Some("claude-sonnet-4".to_string());
        options.fallback_model = Some("claude-opus-4".to_string());

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--model"));
        assert!(cmd_str.contains("claude-sonnet-4"));
        assert!(cmd_str.contains("--fallback-model"));
        assert!(cmd_str.contains("claude-opus-4"));
    }

    #[test]
    fn test_build_command_ide_autoconnect() {
        let mut options = ClaudeCodeOptions::default();
        options.ide_autoconnect = true;

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--ide"));
    }

    #[test]
    fn test_build_command_strict_mcp_config() {
        let mut options = ClaudeCodeOptions::default();
        options.strict_mcp_config = true;

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--strict-mcp-config"));
    }

    #[test]
    fn test_build_command_custom_session_id() {
        use uuid::Uuid;

        let session_id = Uuid::new_v4();
        let mut options = ClaudeCodeOptions::default();
        options.custom_session_id = Some(session_id);

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--session-id"));
        assert!(cmd_str.contains(&session_id.to_string()));
    }

    #[test]
    fn test_build_command_replay_user_messages() {
        let mut options = ClaudeCodeOptions::default();
        options.replay_user_messages = true;

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--replay-user-messages"));
    }

    #[test]
    fn test_build_command_all_new_features() {
        use uuid::Uuid;

        let session_id = Uuid::new_v4();
        let mut options = ClaudeCodeOptions::default();
        options.fallback_model = Some("claude-opus-4".to_string());
        options.ide_autoconnect = true;
        options.strict_mcp_config = true;
        options.custom_session_id = Some(session_id);
        options.replay_user_messages = true;

        let transport = SubprocessTransport::with_cli_path(options, "/usr/bin/true");
        let cmd = transport.build_command();
        let cmd_str = format!("{:?}", cmd);

        assert!(cmd_str.contains("--fallback-model"));
        assert!(cmd_str.contains("claude-opus-4"));
        assert!(cmd_str.contains("--ide"));
        assert!(cmd_str.contains("--strict-mcp-config"));
        assert!(cmd_str.contains("--session-id"));
        assert!(cmd_str.contains(&session_id.to_string()));
        assert!(cmd_str.contains("--replay-user-messages"));
    }
}
