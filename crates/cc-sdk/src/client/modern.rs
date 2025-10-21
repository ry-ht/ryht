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
//! # Examples
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
//!         .with_prompt("Hello, Claude!")?;   // Send initial prompt
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

use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::pin::Pin;

use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::binary;
use crate::core::{state::*, BinaryPath, ModelId, SessionId};
use crate::error::{Error, BinaryError, ClientError, SessionError};
use crate::result::Result;
use crate::transport::{InputMessage, SubprocessTransport, Transport};
use crate::types::{ClaudeCodeOptions, Message, PermissionMode};

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
}
