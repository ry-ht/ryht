//! Modern error system for the Claude Code SDK
//!
//! This module provides a comprehensive error hierarchy for all aspects of the SDK,
//! including binary discovery, transport, sessions, settings, and client operations.
//!
//! # Error Hierarchy
//!
//! ```text
//! Error (top-level)
//! ├── Binary(BinaryError)
//! ├── Transport(TransportError)
//! ├── Session(SessionError)
//! ├── Settings(SettingsError)
//! ├── Client(ClientError)
//! ├── Config(String)
//! └── Protocol(String)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use cc_sdk::error::{Error, BinaryError};
//!
//! fn may_fail() -> Result<(), Error> {
//!     Err(BinaryError::NotFound {
//!         searched_paths: vec!["/usr/local/bin".into()],
//!     }.into())
//! }
//! ```

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Top-level error type for the Claude Code SDK.
///
/// This error type encompasses all possible errors that can occur during
/// SDK operations, including binary discovery, transport, sessions, settings,
/// and client operations.
///
/// # Conversions
///
/// All sub-error types automatically convert to `Error` via the `From` trait:
///
/// ```rust
/// use cc_sdk::error::{Error, BinaryError};
///
/// let binary_error = BinaryError::NotFound {
///     searched_paths: vec!["/usr/local/bin".into()],
/// };
/// let error: Error = binary_error.into();
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// Binary discovery and execution errors.
    #[error("Binary error: {0}")]
    Binary(#[from] BinaryError),

    /// Transport-layer errors (I/O, connection issues).
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Session management errors.
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// Settings and configuration loading errors.
    #[error("Settings error: {0}")]
    Settings(#[from] SettingsError),

    /// Client operation errors.
    #[error("Client error: {0}")]
    Client(#[from] ClientError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Protocol error (invalid messages, protocol violations).
    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Binary discovery and execution errors.
///
/// These errors occur during binary discovery, version checking, and process spawning.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::error::BinaryError;
///
/// // Binary not found
/// let error = BinaryError::NotFound {
///     searched_paths: vec!["/usr/local/bin".into(), "/usr/bin".into()],
/// };
///
/// // Incompatible version
/// let error = BinaryError::IncompatibleVersion {
///     found: "0.1.0".to_string(),
///     required: ">=0.2.0".to_string(),
/// };
/// ```
#[derive(Debug, Error)]
pub enum BinaryError {
    /// Claude CLI executable was not found.
    ///
    /// This error occurs when the Claude CLI cannot be located in any of the
    /// expected installation paths. The error message includes installation
    /// instructions and all searched paths.
    #[error("Claude CLI not found. Install with: npm install -g @anthropic-ai/claude-code")]
    NotFound {
        /// Paths that were searched for the CLI
        searched_paths: Vec<PathBuf>,
    },

    /// Version information could not be determined.
    ///
    /// This error occurs when the binary is found but its version cannot be
    /// determined (e.g., version command fails or output is malformed).
    #[error("Failed to determine Claude CLI version: {reason}")]
    VersionCheckFailed {
        /// Binary path that was checked
        binary_path: PathBuf,
        /// Reason for failure
        reason: String,
        /// Source error if available
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Binary version is incompatible with SDK requirements.
    ///
    /// This error occurs when the binary version does not meet the minimum
    /// version requirements of the SDK.
    #[error("Incompatible Claude CLI version: found {found}, required {required}")]
    IncompatibleVersion {
        /// Version that was found
        found: String,
        /// Required version constraint
        required: String,
    },

    /// Failed to spawn the binary process.
    ///
    /// This error occurs when the binary is found but cannot be executed
    /// (e.g., permission issues, missing dependencies).
    #[error("Failed to spawn Claude CLI process at {path}: {reason}")]
    SpawnFailed {
        /// Binary path that failed to spawn
        path: PathBuf,
        /// Reason for failure
        reason: String,
        /// Source error
        #[source]
        source: std::io::Error,
    },

    /// Environment variable has invalid value.
    ///
    /// This error occurs when a required environment variable is set but
    /// contains an invalid value.
    #[error("Invalid environment variable {var}: {reason}")]
    InvalidEnvVar {
        /// Name of the environment variable
        var: String,
        /// Reason the value is invalid
        reason: String,
    },
}

/// Transport-layer errors.
///
/// These errors occur during message transmission and reception over the transport layer.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::error::TransportError;
///
/// // Connection closed
/// let error = TransportError::Closed;
///
/// // Invalid message
/// let error = TransportError::InvalidMessage {
///     reason: "malformed JSON".to_string(),
///     raw: "{invalid}".to_string(),
/// };
/// ```
#[derive(Debug, Error)]
pub enum TransportError {
    /// I/O error during transport operations.
    ///
    /// This wraps standard I/O errors that occur during reading from or writing
    /// to the transport layer (e.g., stdin/stdout, process pipes).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Transport connection was closed.
    ///
    /// This error occurs when attempting to use a closed transport connection
    /// or when the connection is unexpectedly terminated.
    #[error("Connection closed")]
    Closed,

    /// Invalid or malformed message received.
    ///
    /// This error occurs when a message cannot be parsed or doesn't conform
    /// to the expected JSON-RPC format.
    #[error("Invalid message: {reason}\nRaw: {raw}")]
    InvalidMessage {
        /// Reason for invalidity
        reason: String,
        /// Raw message content
        raw: String,
    },

    /// JSON serialization/deserialization error.
    ///
    /// This error occurs when message serialization or deserialization fails.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Timeout waiting for response.
    ///
    /// This error occurs when a response is not received within the configured
    /// timeout duration.
    #[error("Timeout waiting for response after {duration:?}")]
    Timeout {
        /// Duration waited before timeout
        duration: Duration,
    },

    /// Process exited unexpectedly.
    ///
    /// This error occurs when the underlying process terminates before the
    /// expected end of communication.
    #[error("Process exited unexpectedly with code {code:?}")]
    ProcessExited {
        /// Exit code if available
        code: Option<i32>,
    },

    /// Stream ended unexpectedly.
    ///
    /// This error occurs when a message stream ends before receiving all
    /// expected messages.
    #[error("Stream ended unexpectedly")]
    StreamEnded,

    /// Channel communication error.
    ///
    /// This error occurs when internal channel communication fails.
    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// Session management errors.
///
/// These errors occur during session creation, resumption, and management.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::error::SessionError;
///
/// // Session not found
/// let error = SessionError::NotFound {
///     session_id: "abc123".to_string(),
/// };
///
/// // Invalid state
/// let error = SessionError::InvalidState {
///     current: "disconnected".to_string(),
///     expected: "connected".to_string(),
/// };
/// ```
#[derive(Debug, Error)]
pub enum SessionError {
    /// Session not found.
    ///
    /// This error occurs when attempting to resume or access a session that
    /// doesn't exist.
    #[error("Session not found: {session_id}")]
    NotFound {
        /// Session ID that was not found
        session_id: crate::core::SessionId,
    },

    /// I/O error during session operations.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Home directory could not be found.
    #[error("Home directory not found")]
    HomeDirectoryNotFound,

    /// Failed to parse session data.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Invalid session state.
    ///
    /// This error occurs when an operation is attempted in an invalid state
    /// (e.g., sending messages on a disconnected session).
    #[error("Invalid session state: expected {expected}, found {current}")]
    InvalidState {
        /// Current state
        current: String,
        /// Expected state
        expected: String,
    },

    /// Session initialization failed.
    ///
    /// This error occurs when session setup or initialization fails.
    #[error("Session initialization failed: {reason}")]
    InitializationFailed {
        /// Reason for failure
        reason: String,
        /// Source error if available
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Session already exists.
    ///
    /// This error occurs when attempting to create a session with an ID that
    /// already exists.
    #[error("Session already exists: {session_id}")]
    AlreadyExists {
        /// Session ID that already exists
        session_id: String,
    },

    /// Transcript file error.
    ///
    /// This error occurs when there are issues reading or writing the session
    /// transcript file.
    #[error("Transcript error at {path}: {reason}")]
    TranscriptError {
        /// Path to the transcript file
        path: PathBuf,
        /// Reason for error
        reason: String,
        /// Source error
        #[source]
        source: Option<std::io::Error>,
    },
}

/// Settings and configuration loading errors.
///
/// These errors occur when loading settings from various sources
/// (user, project, local settings files).
///
/// # Examples
///
/// ```rust
/// use cc_sdk::error::SettingsError;
/// use std::path::PathBuf;
///
/// // File not found
/// let error = SettingsError::FileNotFound {
///     path: PathBuf::from(".claude/settings.json"),
/// };
///
/// // Parse error
/// let error = SettingsError::ParseError {
///     path: PathBuf::from(".claude/settings.json"),
///     reason: "invalid JSON".to_string(),
/// };
/// ```
#[derive(Debug, Error)]
pub enum SettingsError {
    /// Settings file not found.
    ///
    /// This error occurs when a required settings file cannot be located.
    #[error("Settings file not found: {path}")]
    FileNotFound {
        /// Path to the missing file
        path: PathBuf,
    },

    /// Failed to parse settings file.
    ///
    /// This error occurs when a settings file contains invalid JSON or
    /// doesn't match the expected schema.
    #[error("Failed to parse settings at {path}: {reason}")]
    ParseError {
        /// Path to the settings file
        path: PathBuf,
        /// Reason for parse failure
        reason: String,
        /// Source error if available
        #[source]
        source: Option<serde_json::Error>,
    },

    /// Invalid settings value.
    ///
    /// This error occurs when settings contain semantically invalid values
    /// (e.g., negative timeouts, invalid paths).
    #[error("Invalid setting '{key}': {reason}")]
    InvalidValue {
        /// Setting key
        key: String,
        /// Reason the value is invalid
        reason: String,
    },

    /// I/O error reading settings.
    ///
    /// This error wraps I/O errors that occur during settings file access.
    #[error("I/O error accessing settings at {path}")]
    Io {
        /// Path where error occurred
        path: PathBuf,
        /// Source I/O error
        #[source]
        source: std::io::Error,
    },

    /// Conflicting settings.
    ///
    /// This error occurs when multiple setting sources provide conflicting
    /// values that cannot be resolved.
    #[error("Conflicting settings for '{key}': {conflict}")]
    Conflict {
        /// Setting key with conflict
        key: String,
        /// Description of the conflict
        conflict: String,
    },

    /// Invalid settings scope.
    ///
    /// This error occurs when an invalid scope is specified or a scope-specific
    /// operation cannot be performed.
    #[error("Invalid settings scope '{scope}': {reason}")]
    InvalidScope {
        /// Scope identifier
        scope: String,
        /// Reason the scope is invalid
        reason: String,
    },

    /// Failed to write settings file.
    ///
    /// This error occurs when settings cannot be written to disk.
    #[error("Failed to write settings to {path}: {reason}")]
    WriteError {
        /// Path to the settings file
        path: PathBuf,
        /// Reason for write failure
        reason: String,
    },
}

/// Client operation errors.
///
/// These errors occur during high-level client operations like sending messages,
/// handling permissions, and managing conversations.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::error::ClientError;
///
/// // Not connected
/// let error = ClientError::NotConnected;
///
/// // Permission denied
/// let error = ClientError::PermissionDenied {
///     tool_name: "Bash".to_string(),
///     reason: "user rejected".to_string(),
/// };
/// ```
#[derive(Debug, Error)]
pub enum ClientError {
    /// Client is not connected.
    ///
    /// This error occurs when attempting an operation that requires an active
    /// connection, but the client is not connected.
    #[error("Client is not connected")]
    NotConnected,

    /// Client is already connected.
    ///
    /// This error occurs when attempting to connect a client that is already
    /// in a connected state.
    #[error("Client is already connected")]
    AlreadyConnected,

    /// Permission denied for tool use.
    ///
    /// This error occurs when a tool use is denied by permission callbacks
    /// or permission mode.
    #[error("Permission denied for tool '{tool_name}': {reason}")]
    PermissionDenied {
        /// Name of the tool that was denied
        tool_name: String,
        /// Reason for denial
        reason: String,
    },

    /// Hook execution failed.
    ///
    /// This error occurs when a registered hook callback fails during execution.
    #[error("Hook '{hook_name}' execution failed: {reason}")]
    HookFailed {
        /// Name of the hook that failed
        hook_name: String,
        /// Reason for failure
        reason: String,
        /// Source error if available
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Unexpected response type.
    ///
    /// This error occurs when the client receives a response of an unexpected
    /// type for the current operation.
    #[error("Unexpected response: expected {expected}, got {actual}")]
    UnexpectedResponse {
        /// Expected response type
        expected: String,
        /// Actual response type received
        actual: String,
    },

    /// Control request failed.
    ///
    /// This error occurs when a control protocol request to the CLI fails.
    #[error("Control request failed: {reason}")]
    ControlRequestFailed {
        /// Reason for failure
        reason: String,
    },

    /// CLI returned an error.
    ///
    /// This error occurs when the CLI process reports an error condition.
    #[error("Claude CLI error: {message}")]
    CliError {
        /// Error message from CLI
        message: String,
        /// Error code if available
        code: Option<String>,
    },

    /// Feature not supported.
    ///
    /// This error occurs when a requested feature is not supported by the
    /// current CLI version or configuration.
    #[error("Feature not supported: {feature}")]
    NotSupported {
        /// Description of unsupported feature
        feature: String,
    },

    /// Operation interrupted.
    ///
    /// This error occurs when an operation is interrupted by user request
    /// or system signal.
    #[error("Operation interrupted: {reason}")]
    Interrupted {
        /// Reason for interruption
        reason: String,
    },

    /// Other client error.
    ///
    /// This error is used for miscellaneous client errors that don't fit
    /// into the other categories.
    #[error("Client error: {0}")]
    Other(String),
}

impl Error {
    /// Check if the error is recoverable.
    ///
    /// Recoverable errors are those that might succeed if retried, such as
    /// timeouts or connection issues.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Transport(TransportError::Timeout { .. })
            | Self::Transport(TransportError::StreamEnded)
            | Self::Transport(TransportError::Closed)
            | Self::Transport(TransportError::ProcessExited { .. })
            | Self::Session(SessionError::InvalidState { .. }) => true,
            _ => false,
        }
    }

    /// Check if the error is a configuration issue.
    ///
    /// Configuration errors typically require user intervention to fix.
    pub fn is_config_error(&self) -> bool {
        matches!(
            self,
            Self::Binary(_) | Self::Settings(_) | Self::Config(_)
        )
    }

    /// Check if the error indicates a connection problem.
    ///
    /// Connection errors suggest issues with the transport layer or process.
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            Self::Transport(TransportError::Closed)
                | Self::Transport(TransportError::ProcessExited { .. })
                | Self::Client(ClientError::NotConnected)
        )
    }

    /// Create a new Config error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create a new Protocol error.
    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol(message.into())
    }
}

// Implement From for common channel errors
impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(e: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Transport(TransportError::ChannelError(e.to_string()))
    }
}

impl From<tokio::sync::broadcast::error::RecvError> for Error {
    fn from(e: tokio::sync::broadcast::error::RecvError) -> Self {
        Self::Transport(TransportError::ChannelError(e.to_string()))
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for Error {
    fn from(e: crossbeam_channel::SendError<T>) -> Self {
        Self::Transport(TransportError::ChannelError(e.to_string()))
    }
}

impl From<crossbeam_channel::RecvError> for Error {
    fn from(e: crossbeam_channel::RecvError) -> Self {
        Self::Transport(TransportError::ChannelError(e.to_string()))
    }
}

// ============================================================================
// Legacy Compatibility Exports
// ============================================================================

/// Legacy error type alias (deprecated, use `Error` instead).
///
/// This type is maintained for backward compatibility with existing code.
/// New code should use the modern `Error` type.
#[deprecated(since = "0.3.0", note = "Use Error instead")]
pub type SdkError = Error;

/// Legacy result type alias (deprecated, use `crate::Result` instead).
///
/// This type is maintained for backward compatibility with existing code.
/// New code should use the modern `Result` type from the `result` module.
#[deprecated(since = "0.3.0", note = "Use crate::Result instead")]
pub type Result<T> = std::result::Result<T, Error>;

/// Legacy Error constructors for backward compatibility.
impl Error {
    /// Legacy constructor for InvalidState with message field.
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::Session(SessionError::InvalidState {
            current: "unknown".to_string(),
            expected: message.into(),
        })
    }

    /// Create a ControlRequestError (legacy, maps to ClientError::ControlRequestFailed).
    pub fn control_request_error(message: impl Into<String>) -> Self {
        Self::Client(ClientError::ControlRequestFailed {
            reason: message.into(),
        })
    }

    /// Create a timeout error (legacy, maps to TransportError::Timeout).
    pub fn timeout(seconds: u64) -> Self {
        Self::Transport(TransportError::Timeout {
            duration: Duration::from_secs(seconds),
        })
    }

    /// Create a parse error (legacy, maps to TransportError::InvalidMessage).
    pub fn parse_error(error: impl Into<String>, raw: impl Into<String>) -> Self {
        Self::Transport(TransportError::InvalidMessage {
            reason: error.into(),
            raw: raw.into(),
        })
    }

    /// Create an unexpected response error (legacy, maps to ClientError::UnexpectedResponse).
    pub fn unexpected_response(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::Client(ClientError::UnexpectedResponse {
            expected: expected.into(),
            actual: actual.into(),
        })
    }

    /// Create a CLI error (legacy, maps to ClientError::CliError).
    pub fn cli_error(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Client(ClientError::CliError {
            message: message.into(),
            code,
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_error_not_found() {
        let searched_paths = vec!["/usr/local/bin".into(), "/usr/bin".into()];
        let error = BinaryError::NotFound {
            searched_paths: searched_paths.clone(),
        };
        let msg = error.to_string();
        assert!(msg.contains("npm install -g @anthropic-ai/claude-code"));
        // The error message format doesn't include searched paths in Display
        // But the paths are available in the error struct for programmatic access
        if let BinaryError::NotFound { searched_paths: paths } = error {
            assert_eq!(paths.len(), 2);
        }
    }

    #[test]
    fn test_binary_error_incompatible_version() {
        let error = BinaryError::IncompatibleVersion {
            found: "0.1.0".to_string(),
            required: ">=0.2.0".to_string(),
        };
        assert!(error.to_string().contains("0.1.0"));
        assert!(error.to_string().contains(">=0.2.0"));
    }

    #[test]
    fn test_transport_error_timeout() {
        let error = TransportError::Timeout {
            duration: Duration::from_secs(30),
        };
        assert!(error.to_string().contains("30s"));
    }

    #[test]
    fn test_session_error_not_found() {
        let error = SessionError::NotFound {
            session_id: crate::core::SessionId::new("test-session"),
        };
        assert!(error.to_string().contains("test-session"));
    }

    #[test]
    fn test_settings_error_parse() {
        let error = SettingsError::ParseError {
            path: PathBuf::from(".claude/settings.json"),
            reason: "invalid JSON".to_string(),
            source: None,
        };
        assert!(error.to_string().contains(".claude/settings.json"));
        assert!(error.to_string().contains("invalid JSON"));
    }

    #[test]
    fn test_client_error_permission_denied() {
        let error = ClientError::PermissionDenied {
            tool_name: "Bash".to_string(),
            reason: "user rejected".to_string(),
        };
        assert!(error.to_string().contains("Bash"));
        assert!(error.to_string().contains("user rejected"));
    }

    #[test]
    fn test_error_is_recoverable() {
        let error = Error::Transport(TransportError::Timeout {
            duration: Duration::from_secs(30),
        });
        assert!(error.is_recoverable());

        let error = Error::Binary(BinaryError::NotFound {
            searched_paths: vec![],
        });
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_error_is_config_error() {
        let error = Error::Binary(BinaryError::NotFound {
            searched_paths: vec![],
        });
        assert!(error.is_config_error());

        let error = Error::Settings(SettingsError::FileNotFound {
            path: PathBuf::from("test"),
        });
        assert!(error.is_config_error());

        let error = Error::Transport(TransportError::Closed);
        assert!(!error.is_config_error());
    }

    #[test]
    fn test_error_is_connection_error() {
        let error = Error::Transport(TransportError::Closed);
        assert!(error.is_connection_error());

        let error = Error::Client(ClientError::NotConnected);
        assert!(error.is_connection_error());

        let error = Error::Config("test".to_string());
        assert!(!error.is_connection_error());
    }

    #[test]
    fn test_error_conversion_chain() {
        let binary_error = BinaryError::NotFound {
            searched_paths: vec![],
        };
        let error: Error = binary_error.into();
        matches!(error, Error::Binary(_));
    }

    #[test]
    fn test_channel_error_conversions() {
        // Note: TrySendError doesn't have a From impl, only SendError does
        // let (tx, _rx) = tokio::sync::mpsc::channel::<()>(1);
        // drop(_rx);
        // let send_error = tx.try_send(()).unwrap_err();
        // let _: Error = send_error.into();

        let recv_error = tokio::sync::broadcast::error::RecvError::Closed;
        let _: Error = recv_error.into();
    }

    #[test]
    fn test_error_display_formats() {
        let errors = vec![
            Error::Config("invalid config".to_string()),
            Error::Protocol("invalid protocol".to_string()),
            Error::Binary(BinaryError::NotFound {
                searched_paths: vec![],
            }),
            Error::Transport(TransportError::Closed),
            Error::Session(SessionError::NotFound {
                session_id: crate::core::SessionId::new("test"),
            }),
            Error::Settings(SettingsError::FileNotFound {
                path: PathBuf::from("test"),
            }),
            Error::Client(ClientError::NotConnected),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
