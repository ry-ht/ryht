//! Simple query interface for one-shot and streaming interactions
//!
//! This module provides the `query` function for simple, stateless interactions
//! with Claude Code CLI. It supports both:
//! - **Text mode**: Simple one-shot queries with a single string prompt
//! - **Streaming mode**: Continuous message streaming for bidirectional communication
//!
//! # Streaming Mode
//!
//! The streaming mode accepts a `Stream<InputMessage>` and returns a stream of responses.
//! This enables:
//! - Sending multiple messages in sequence
//! - Handling tool use requests and responses
//! - Dynamic message generation based on async operations
//! - Batch processing with full conversation context
//!
//! # Example: Streaming Mode
//!
//! ```rust,no_run
//! use axon::cc::{query, QueryInput, transport::InputMessage};
//! use futures::{stream, StreamExt};
//!
//! #[tokio::main]
//! async fn main() -> axon::cc::Result<()> {
//!     let session_id = "my-session";
//!
//!     // Create a stream of messages
//!     let messages = vec![
//!         InputMessage::user("What is 2+2?".to_string(), session_id.to_string()),
//!         InputMessage::user("What about 3+3?".to_string(), session_id.to_string()),
//!     ];
//!
//!     let input_stream = Box::pin(stream::iter(messages));
//!     let input = QueryInput::Stream(input_stream);
//!
//!     // Execute the streaming query
//!     let mut response_stream = query(input, None).await?;
//!
//!     while let Some(result) = response_stream.next().await {
//!         println!("{:?}", result?);
//!     }
//!
//!     Ok(())
//! }
//! ```

use super::{
    Result,
    transport::InputMessage,
    messages::Message,
    options::ClaudeCodeOptions,
    permissions::PermissionMode,
};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};

/// Query input type
pub enum QueryInput {
    /// Simple string prompt
    Text(String),
    /// Stream of input messages for continuous interaction
    Stream(Pin<Box<dyn Stream<Item = InputMessage> + Send>>),
}

impl From<String> for QueryInput {
    fn from(s: String) -> Self {
        QueryInput::Text(s)
    }
}

impl From<&str> for QueryInput {
    fn from(s: &str) -> Self {
        QueryInput::Text(s.to_string())
    }
}

/// Query Claude Code for one-shot or unidirectional streaming interactions.
///
/// This function is ideal for simple, stateless queries where you don't need
/// bidirectional communication or conversation management. For interactive,
/// stateful conversations, use [`ClaudeClient`](crate::ClaudeClient) instead.
///
/// # Key differences from ClaudeClient:
/// - **Unidirectional**: Send all messages upfront, receive all responses
/// - **Stateless**: Each query is independent, no conversation state
/// - **Simple**: Fire-and-forget style, no connection management
/// - **No interrupts**: Cannot interrupt or send follow-up messages
///
/// # When to use query():
/// - Simple one-off questions ("What is 2+2?")
/// - Batch processing of independent prompts
/// - Code generation or analysis tasks
/// - Automated scripts and CI/CD pipelines
/// - When you know all inputs upfront
///
/// # When to use ClaudeClient:
/// - Interactive conversations with follow-ups
/// - Chat applications or REPL-like interfaces
/// - When you need to send messages based on responses
/// - When you need interrupt capabilities
/// - Long-running sessions with state
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude. Can be a string for single-shot queries
///   or a Stream of InputMessage for streaming mode.
/// * `options` - Optional configuration. If None, defaults to `ClaudeCodeOptions::default()`.
///
/// # Returns
///
/// A stream of messages from the conversation.
///
/// # Examples
///
/// ## Simple query:
/// ```rust,no_run
/// use super::{query, Result};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // One-off question
///     let mut messages = query("What is the capital of France?", None).await?;
///
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     Ok(())
/// }
/// ```
///
/// ## With options:
/// ```rust,no_run
/// use super::{query, ClaudeCodeOptions, Result};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     // Code generation with specific settings
///     let options = ClaudeCodeOptions::builder()
///         .system_prompt("You are an expert Python developer")
///         .model("claude-3-opus-20240229")
///         .build();
///
///     let mut messages = query("Create a Python web server", Some(options)).await?;
///
///     while let Some(msg) = messages.next().await {
///         println!("{:?}", msg?);
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query(
    prompt: impl Into<QueryInput>,
    options: Option<ClaudeCodeOptions>,
) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>> {
    let options = options.unwrap_or_default();
    let prompt = prompt.into();

    // Set environment variable to indicate SDK usage
    unsafe {
        std::env::set_var("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
    }

    match prompt {
        QueryInput::Text(text) => {
            // For simple text queries, use --print mode like Python SDK
            let stream = query_print_mode(text, options).await?;
            Ok(Box::pin(stream))
        }
        QueryInput::Stream(stream) => {
            // Interactive streaming mode - forward messages from input stream to CLI
            let stream = query_stream_mode(stream, options).await?;
            Ok(Box::pin(stream))
        }
    }
}

/// Execute a simple query using --print mode
async fn query_print_mode(
    prompt: String,
    options: ClaudeCodeOptions,
) -> Result<impl Stream<Item = Result<Message>>> {
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;
    use tokio::sync::Mutex;

    let cli_path = crate::cc::transport::subprocess::find_claude_cli()?;
    let mut cmd = Command::new(&cli_path);

    // Build command with --print mode
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--verbose");

    // Add all options to match Python SDK exactly
    if let Some(ref prompt) = options.system_prompt {
        match prompt {
            crate::cc::options::SystemPrompt::String(s) => {
                cmd.arg("--system-prompt").arg(s);
            }
            crate::cc::options::SystemPrompt::Preset { preset, append, .. } => {
                cmd.arg("--system-prompt-preset").arg(preset);
                if let Some(append_text) = append {
                    cmd.arg("--append-system-prompt").arg(append_text);
                }
            }
        }
    }

    if !options.allowed_tools.is_empty() {
        cmd.arg("--allowedTools")
            .arg(options.allowed_tools.join(","));
    }

    if let Some(max_turns) = options.max_turns {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    if !options.disallowed_tools.is_empty() {
        cmd.arg("--disallowedTools")
            .arg(options.disallowed_tools.join(","));
    }

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    if let Some(ref tool_name) = options.permission_prompt_tool_name {
        cmd.arg("--permission-prompt-tool").arg(tool_name);
    }

    match options.permission_mode {
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

    if options.continue_conversation {
        cmd.arg("--continue");
    }

    if let Some(ref resume_id) = options.resume {
        cmd.arg("--resume").arg(resume_id);
    }

    if !options.mcp_servers.is_empty() {
        let mcp_config = serde_json::json!({
            "mcpServers": options.mcp_servers
        });
        cmd.arg("--mcp-config").arg(mcp_config.to_string());
    }

    // Extra arguments
    for (key, value) in &options.extra_args {
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

    // Add the prompt with --print
    cmd.arg("--print").arg(&prompt);

    // Set up process pipes
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    
    // Handle max_output_tokens (priority: option > env var)
    // Maximum safe value is 32000, values above this may cause issues
    if let Some(max_tokens) = options.max_output_tokens {
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
            } else {
                warn!("Invalid CLAUDE_CODE_MAX_OUTPUT_TOKENS value: {}, setting to 8192", current_value);
                cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
            }
        }
    }

    info!("Starting Claude CLI with --print mode");
    debug!("Command: {:?}", cmd);

    let mut child = cmd.spawn().map_err(|e| {
        crate::cc::error::Error::Binary(crate::cc::error::BinaryError::SpawnFailed {
            path: cli_path.clone(),
            reason: format!("Failed to spawn process: {}", e),
            source: e,
        })
    })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| crate::cc::error::Error::Transport(crate::cc::error::TransportError::ChannelError("Failed to get stdout".into())))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| crate::cc::error::Error::Transport(crate::cc::error::TransportError::ChannelError("Failed to get stderr".into())))?;

    // Wrap child process in Arc<Mutex> for shared ownership
    let child = Arc::new(Mutex::new(child));
    let child_clone = Arc::clone(&child);

    // Create a channel to collect messages
    let (tx, rx) = mpsc::channel(100);

    // Spawn stderr handler
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                debug!("Claude stderr: {}", line);
            }
        }
    });

    // Clone tx for cleanup task
    let tx_cleanup = tx.clone();
    
    // Spawn stdout handler
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            debug!("Claude output: {}", line);

            // Parse JSON line
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(json) => {
                    match crate::cc::message_parser::parse_message(json) {
                        Ok(Some(message)) => {
                            if tx.send(Ok(message)).await.is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            // Ignore non-message JSON
                        }
                        Err(e) => {
                            if tx.send(Err(e)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to parse JSON: {} - Line: {}", e, line);
                }
            }
        }

        // Wait for process to complete and ensure cleanup
        let mut child = child_clone.lock().await;
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    let _ = tx
                        .send(Err(crate::cc::error::Error::Transport(crate::cc::error::TransportError::ProcessExited {
                            code: status.code(),
                        })))
                        .await;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(crate::cc::error::Error::Transport(crate::cc::error::TransportError::Io(e)))).await;
            }
        }
    });

    // Spawn cleanup task that will ensure process is killed when stream is dropped
    tokio::spawn(async move {
        // Wait for the channel to be closed (all receivers dropped)
        tx_cleanup.closed().await;
        
        // Kill the process if it's still running
        let mut child = child.lock().await;
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process already exited
                debug!("Claude CLI process already exited");
            }
            Ok(None) => {
                // Process still running, kill it
                info!("Killing Claude CLI process on stream drop");
                if let Err(e) = child.kill().await {
                    warn!("Failed to kill Claude CLI process: {}", e);
                } else {
                    // Wait for the process to actually exit
                    let _ = child.wait().await;
                    debug!("Claude CLI process killed and cleaned up");
                }
            }
            Err(e) => {
                warn!("Failed to check process status: {}", e);
            }
        }
    });

    // Return receiver as stream
    Ok(ReceiverStream::new(rx))
}

/// Execute a streaming query using interactive mode
///
/// This function enables bidirectional streaming communication with Claude CLI:
/// - Accepts a stream of InputMessage to forward to Claude
/// - Returns a stream of Message responses
/// - Handles concurrent input/output streams
/// - Automatically manages process lifecycle
async fn query_stream_mode(
    mut input_stream: Pin<Box<dyn Stream<Item = InputMessage> + Send>>,
    options: ClaudeCodeOptions,
) -> Result<impl Stream<Item = Result<Message>>> {
    use std::sync::Arc;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;
    use tokio::sync::Mutex;

    let cli_path = crate::cc::transport::subprocess::find_claude_cli()?;
    let mut cmd = Command::new(&cli_path);

    // Build command for interactive mode (stream-json input/output)
    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--input-format").arg("stream-json");
    cmd.arg("--verbose");

    // Add all options - similar to query_print_mode but without --print flag
    if let Some(ref prompt) = options.system_prompt {
        match prompt {
            crate::cc::options::SystemPrompt::String(s) => {
                cmd.arg("--system-prompt").arg(s);
            }
            crate::cc::options::SystemPrompt::Preset { preset, append, .. } => {
                cmd.arg("--system-prompt-preset").arg(preset);
                if let Some(append_text) = append {
                    cmd.arg("--append-system-prompt").arg(append_text);
                }
            }
        }
    }

    if !options.allowed_tools.is_empty() {
        cmd.arg("--allowedTools")
            .arg(options.allowed_tools.join(","));
    }

    if let Some(max_turns) = options.max_turns {
        cmd.arg("--max-turns").arg(max_turns.to_string());
    }

    if !options.disallowed_tools.is_empty() {
        cmd.arg("--disallowedTools")
            .arg(options.disallowed_tools.join(","));
    }

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    if let Some(ref tool_name) = options.permission_prompt_tool_name {
        cmd.arg("--permission-prompt-tool").arg(tool_name);
    }

    match options.permission_mode {
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

    if options.continue_conversation {
        cmd.arg("--continue");
    }

    if let Some(ref resume_id) = options.resume {
        cmd.arg("--resume").arg(resume_id);
    }

    if !options.mcp_servers.is_empty() {
        let mcp_config = serde_json::json!({
            "mcpServers": options.mcp_servers
        });
        cmd.arg("--mcp-config").arg(mcp_config.to_string());
    }

    // Extra arguments
    for (key, value) in &options.extra_args {
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
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // Handle max_output_tokens (priority: option > env var)
    if let Some(max_tokens) = options.max_output_tokens {
        let capped = max_tokens.clamp(1, 32000);
        cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", capped.to_string());
        debug!("Setting max_output_tokens from option: {}", capped);
    } else if let Ok(current_value) = std::env::var("CLAUDE_CODE_MAX_OUTPUT_TOKENS") {
        if let Ok(tokens) = current_value.parse::<u32>() {
            if tokens > 32000 {
                warn!("CLAUDE_CODE_MAX_OUTPUT_TOKENS={} exceeds maximum safe value of 32000, overriding to 32000", tokens);
                cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "32000");
            }
        } else {
            warn!("Invalid CLAUDE_CODE_MAX_OUTPUT_TOKENS value: {}, setting to 8192", current_value);
            cmd.env("CLAUDE_CODE_MAX_OUTPUT_TOKENS", "8192");
        }
    }

    info!("Starting Claude CLI in interactive streaming mode");
    debug!("Command: {:?}", cmd);

    let mut child = cmd.spawn().map_err(|e| {
        crate::cc::error::Error::Binary(crate::cc::error::BinaryError::SpawnFailed {
            path: cli_path.clone(),
            reason: format!("Failed to spawn process: {}", e),
            source: e,
        })
    })?;

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

    // Wrap child process in Arc<Mutex> for shared ownership
    let child = Arc::new(Mutex::new(child));
    let child_clone_stdin = Arc::clone(&child);
    let child_clone_stdout = Arc::clone(&child);

    // Create channels for message output
    let (tx, rx) = mpsc::channel(100);

    // Spawn stdin handler - consumes input stream and forwards to CLI
    tokio::spawn(async move {
        let mut stdin = stdin;
        debug!("Stdin handler started for streaming mode");

        while let Some(input_msg) = input_stream.next().await {
            // Serialize InputMessage to JSON line
            match serde_json::to_string(&input_msg) {
                Ok(json_line) => {
                    debug!("Forwarding input message: {}", json_line);
                    if let Err(e) = stdin.write_all(json_line.as_bytes()).await {
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
                    debug!("Successfully forwarded message to Claude CLI");
                }
                Err(e) => {
                    error!("Failed to serialize input message: {}", e);
                    break;
                }
            }
        }

        // Input stream ended - close stdin to signal end of input
        debug!("Input stream ended, closing stdin");
        drop(stdin);

        // Wait for process to complete after input ends
        let mut child = child_clone_stdin.lock().await;
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    debug!("Claude CLI process exited with status: {:?}", status);
                }
            }
            Err(e) => {
                warn!("Failed to wait for Claude CLI process: {}", e);
            }
        }
    });

    // Clone tx for cleanup task
    let tx_cleanup = tx.clone();

    // Spawn stdout handler - reads responses from CLI and sends to output channel
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            debug!("Claude output: {}", line);

            // Parse JSON line
            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(json) => {
                    match crate::cc::message_parser::parse_message(json) {
                        Ok(Some(message)) => {
                            if tx.send(Ok(message)).await.is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            // Ignore non-message JSON (e.g., control messages)
                        }
                        Err(e) => {
                            if tx.send(Err(e)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to parse JSON: {} - Line: {}", e, line);
                }
            }
        }

        // Wait for process to complete
        let mut child = child_clone_stdout.lock().await;
        match child.wait().await {
            Ok(status) => {
                if !status.success() {
                    let _ = tx
                        .send(Err(crate::cc::error::Error::Transport(crate::cc::error::TransportError::ProcessExited {
                            code: status.code(),
                        })))
                        .await;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(crate::cc::error::Error::Transport(crate::cc::error::TransportError::Io(e)))).await;
            }
        }
    });

    // Spawn stderr handler
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                debug!("Claude stderr: {}", line);
            }
        }
    });

    // Spawn cleanup task that will ensure process is killed when stream is dropped
    tokio::spawn(async move {
        // Wait for the channel to be closed (all receivers dropped)
        tx_cleanup.closed().await;

        // Kill the process if it's still running
        let mut child = child.lock().await;
        match child.try_wait() {
            Ok(Some(_)) => {
                // Process already exited
                debug!("Claude CLI process already exited");
            }
            Ok(None) => {
                // Process still running, kill it
                info!("Killing Claude CLI process on stream drop");
                if let Err(e) = child.kill().await {
                    warn!("Failed to kill Claude CLI process: {}", e);
                } else {
                    // Wait for the process to actually exit
                    let _ = child.wait().await;
                    debug!("Claude CLI process killed and cleaned up");
                }
            }
            Err(e) => {
                warn!("Failed to check process status: {}", e);
            }
        }
    });

    // Return receiver as stream
    Ok(ReceiverStream::new(rx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_input_from_string() {
        let input: QueryInput = "Hello".into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_query_input_from_str() {
        let input: QueryInput = "World".into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "World"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_extra_args_formatting() {
        use std::collections::HashMap;

        // Test that extra_args are properly formatted as CLI flags
        let mut extra_args = HashMap::new();
        extra_args.insert("custom-flag".to_string(), Some("value".to_string()));
        extra_args.insert("--already-dashed".to_string(), None);
        extra_args.insert("-s".to_string(), Some("short".to_string()));

        let options = ClaudeCodeOptions {
            extra_args,
            ..Default::default()
        };

        // Verify the args are properly stored
        assert_eq!(options.extra_args.len(), 3);
        assert!(options.extra_args.contains_key("custom-flag"));
        assert!(options.extra_args.contains_key("--already-dashed"));
        assert!(options.extra_args.contains_key("-s"));
    }

    // Streaming mode tests
    #[tokio::test]
    async fn test_query_input_stream_construction() {
        use futures::stream;

        // Test that we can construct a QueryInput::Stream
        let messages = vec![
            InputMessage::user("Hello".to_string(), "session-1".to_string()),
            InputMessage::user("World".to_string(), "session-1".to_string()),
        ];

        let stream = Box::pin(stream::iter(messages));
        let input = QueryInput::Stream(stream);

        match input {
            QueryInput::Stream(_) => {
                // Success - we can construct a streaming input
            }
            _ => panic!("Expected Stream variant"),
        }
    }

    #[tokio::test]
    async fn test_input_message_serialization() {
        // Test that InputMessage serializes correctly for CLI
        let msg = InputMessage::user("Test message".to_string(), "session-123".to_string());

        let json = serde_json::to_string(&msg).expect("Failed to serialize");

        // Verify it contains expected fields
        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""session_id":"session-123""#));
        assert!(json.contains("Test message"));
    }

    #[tokio::test]
    async fn test_input_message_with_blocks() {
        // Test user message with content blocks (for file attachments)
        let blocks = vec![
            serde_json::json!({
                "type": "text",
                "text": "Check this file"
            }),
            serde_json::json!({
                "type": "document",
                "source": {
                    "type": "text",
                    "media_type": "text/plain",
                    "data": "file contents"
                }
            }),
        ];

        let msg = InputMessage::user_with_blocks(blocks, "session-456".to_string());
        let json = serde_json::to_string(&msg).expect("Failed to serialize");

        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains("Check this file"));
    }

    #[tokio::test]
    async fn test_tool_result_message() {
        // Test tool result message format
        let msg = InputMessage::tool_result(
            "tool-use-123".to_string(),
            "Command output".to_string(),
            "session-789".to_string(),
            false,
        );

        let json = serde_json::to_string(&msg).expect("Failed to serialize");

        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""tool_use_id":"tool-use-123""#));
        assert!(json.contains(r#""is_error":false"#));
        assert!(json.contains("Command output"));
    }

    // Mock test for streaming mode (doesn't actually spawn Claude CLI)
    #[tokio::test]
    async fn test_stream_empty_input() {
        use futures::stream;

        // Create an empty stream
        let empty_stream: Vec<InputMessage> = vec![];
        let stream = Box::pin(stream::iter(empty_stream));
        let input = QueryInput::Stream(stream);

        // Verify we can construct it
        match input {
            QueryInput::Stream(_) => {
                // Empty stream is valid input
            }
            _ => panic!("Expected Stream variant"),
        }
    }

    #[tokio::test]
    async fn test_stream_multiple_messages() {
        use futures::stream;

        // Create a stream with multiple messages
        let messages = vec![
            InputMessage::user("First message".to_string(), "session-1".to_string()),
            InputMessage::user("Second message".to_string(), "session-1".to_string()),
            InputMessage::user("Third message".to_string(), "session-1".to_string()),
        ];

        let stream = Box::pin(stream::iter(messages.clone()));
        let _input = QueryInput::Stream(stream);

        // Verify all messages serialize correctly
        for msg in messages {
            let json = serde_json::to_string(&msg).expect("Failed to serialize");
            assert!(json.contains(r#""type":"user""#));
            assert!(json.contains(r#""session_id":"session-1""#));
        }
    }

    #[tokio::test]
    async fn test_stream_with_tool_results() {
        use futures::stream;

        // Create a stream that includes tool results
        let messages = vec![
            InputMessage::user("Run a command".to_string(), "session-1".to_string()),
            InputMessage::tool_result(
                "tool-1".to_string(),
                "Success".to_string(),
                "session-1".to_string(),
                false,
            ),
        ];

        let stream = Box::pin(stream::iter(messages.clone()));
        let _input = QueryInput::Stream(stream);

        // Verify serialization of both types
        for msg in messages {
            let json = serde_json::to_string(&msg).expect("Failed to serialize");
            assert!(json.contains(r#""type":"user""#));
        }
    }

    #[test]
    fn test_query_input_conversions() {
        // Test From<String>
        let input: QueryInput = "test".to_string().into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "test"),
            _ => panic!("Expected Text variant"),
        }

        // Test From<&str>
        let input: QueryInput = "test2".into();
        match input {
            QueryInput::Text(s) => assert_eq!(s, "test2"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[tokio::test]
    async fn test_streaming_session_consistency() {
        use futures::stream;

        let session_id = "consistent-session-123";

        // All messages in a stream should use the same session ID
        let messages = vec![
            InputMessage::user("Message 1".to_string(), session_id.to_string()),
            InputMessage::user("Message 2".to_string(), session_id.to_string()),
            InputMessage::user("Message 3".to_string(), session_id.to_string()),
        ];

        for msg in &messages {
            assert_eq!(msg.session_id, session_id);
        }

        let stream = Box::pin(stream::iter(messages));
        let _input = QueryInput::Stream(stream);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use futures::stream;

    /// Example: Streaming query with simple text messages
    ///
    /// This demonstrates how to use the streaming mode for a conversation
    /// where you know all the messages upfront but want to send them one at a time.
    #[tokio::test]
    #[ignore] // Requires Claude CLI to be installed
    async fn test_streaming_conversation_example() {
        let session_id = "test-session-1";

        // Create a sequence of messages to send
        let messages = vec![
            InputMessage::user("What is 2+2?".to_string(), session_id.to_string()),
            InputMessage::user("What about 3+3?".to_string(), session_id.to_string()),
        ];

        let input_stream = Box::pin(stream::iter(messages));
        let input = QueryInput::Stream(input_stream);

        let options = ClaudeCodeOptions::default();

        // Execute the streaming query
        let mut response_stream = query(input, Some(options))
            .await
            .expect("Failed to execute streaming query");

        let mut response_count = 0;
        while let Some(result) = response_stream.next().await {
            match result {
                Ok(message) => {
                    println!("Received message: {:?}", message);
                    response_count += 1;
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }

        assert!(response_count > 0, "Should have received at least one response");
    }

    /// Example: Streaming with tool results
    ///
    /// This demonstrates handling a conversation that includes tool use.
    #[tokio::test]
    #[ignore] // Requires Claude CLI to be installed
    async fn test_streaming_with_tool_results_example() {
        let session_id = "test-session-2";

        // Simulate a conversation with tool use
        let messages = vec![
            InputMessage::user("List files in current directory".to_string(), session_id.to_string()),
            // In a real scenario, you would receive a tool use request, execute it,
            // and then send the result back:
            InputMessage::tool_result(
                "tool-use-123".to_string(),
                "file1.txt\nfile2.txt\nfile3.txt".to_string(),
                session_id.to_string(),
                false,
            ),
        ];

        let input_stream = Box::pin(stream::iter(messages));
        let input = QueryInput::Stream(input_stream);

        let mut options = ClaudeCodeOptions::default();
        options.permission_mode = PermissionMode::BypassPermissions;

        let mut response_stream = query(input, Some(options))
            .await
            .expect("Failed to execute streaming query");

        while let Some(result) = response_stream.next().await {
            match result {
                Ok(message) => {
                    println!("Received message: {:?}", message);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }
    }

    /// Example: Dynamic stream generation
    ///
    /// This shows how to create a stream from an async generator,
    /// allowing for dynamic message creation based on responses.
    #[tokio::test]
    #[ignore] // Requires Claude CLI to be installed
    async fn test_dynamic_stream_example() {
        let session_id = "test-session-3";

        // Create a channel to dynamically add messages
        let (tx, rx) = mpsc::channel::<InputMessage>(10);

        // Spawn a task that sends messages over time
        let session_id_clone = session_id.to_string();
        tokio::spawn(async move {
            // Send first message
            let _ = tx.send(InputMessage::user(
                "Start of conversation".to_string(),
                session_id_clone.clone(),
            )).await;

            // Simulate some async work
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Send second message
            let _ = tx.send(InputMessage::user(
                "Continue conversation".to_string(),
                session_id_clone,
            )).await;

            // Close the channel when done
            drop(tx);
        });

        // Convert receiver to stream
        let input_stream = Box::pin(ReceiverStream::new(rx));
        let input = QueryInput::Stream(input_stream);

        let options = ClaudeCodeOptions::default();

        let mut response_stream = query(input, Some(options))
            .await
            .expect("Failed to execute streaming query");

        let mut response_count = 0;
        while let Some(result) = response_stream.next().await {
            match result {
                Ok(_message) => {
                    response_count += 1;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        println!("Received {} responses", response_count);
    }

    /// Example: Stream with options
    ///
    /// Demonstrates using streaming mode with custom options.
    #[tokio::test]
    #[ignore] // Requires Claude CLI to be installed
    async fn test_streaming_with_options_example() {
        use crate::cc::options::SystemPrompt;

        let session_id = "test-session-4";

        let messages = vec![
            InputMessage::user("Explain Rust ownership".to_string(), session_id.to_string()),
        ];

        let input_stream = Box::pin(stream::iter(messages));
        let input = QueryInput::Stream(input_stream);

        let options = ClaudeCodeOptions::builder()
            .system_prompt(SystemPrompt::String("You are a Rust expert. Be concise.".to_string()))
            .model("claude-sonnet-4-5-20250929")
            .max_output_tokens(1000)
            .permission_mode(PermissionMode::AcceptEdits)
            .build();

        let mut response_stream = query(input, Some(options))
            .await
            .expect("Failed to execute streaming query");

        while let Some(result) = response_stream.next().await {
            if let Ok(message) = result {
                match message {
                    Message::Assistant { message, .. } => {
                        println!("Assistant: {:?}", message.content);
                    }
                    Message::System { subtype, .. } => {
                        println!("System ({})", subtype);
                    }
                    _ => {
                        println!("Other message: {:?}", message);
                    }
                }
            }
        }
    }
}
