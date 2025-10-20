//! Hook event types.
//!
//! This module defines all the events that can be emitted by the MCP server
//! and captured by hooks. Events provide insight into server lifecycle,
//! request processing, and errors.
//!
//! # Event Types
//!
//! - **Lifecycle Events**: ServerStarted, ServerStopped
//! - **Client Events**: ClientConnected, ClientDisconnected
//! - **Tool Events**: ToolCalled, ToolCompleted
//! - **Resource Events**: ResourceRead
//! - **Error Events**: Error
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::hooks::HookEvent;
//! use serde_json::json;
//!
//! // Tool call event
//! let event = HookEvent::ToolCalled {
//!     name: "echo".to_string(),
//!     args: json!({"message": "hello"}),
//! };
//!
//! // Error event
//! let event = HookEvent::Error {
//!     error: "Connection failed".to_string(),
//! };
//! ```

use serde_json::Value;

/// Events that can be emitted by the MCP server.
///
/// These events allow hooks to respond to various server activities,
/// enabling monitoring, logging, auditing, and other cross-cutting concerns.
///
/// # Thread Safety
///
/// `HookEvent` is `Clone` to allow distribution to multiple hooks.
/// It's also `Send` to allow passing between threads.
///
/// # Examples
///
/// ```rust
/// use mcp_server::hooks::HookEvent;
/// use serde_json::json;
///
/// match HookEvent::ServerStarted {
///     HookEvent::ServerStarted => println!("Server started!"),
///     _ => {}
/// }
///
/// let tool_event = HookEvent::ToolCalled {
///     name: "calculate".to_string(),
///     args: json!({"a": 1, "b": 2}),
/// };
/// ```
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// Server has started and is ready to accept requests.
    ///
    /// Emitted once when the server begins serving requests.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::ServerStarted;
    /// ```
    ServerStarted,

    /// Server is shutting down.
    ///
    /// Emitted once when the server begins graceful shutdown.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::ServerStopped;
    /// ```
    ServerStopped,

    /// A client has connected to the server.
    ///
    /// Emitted after a client successfully connects and completes initialization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::ClientConnected;
    /// ```
    ClientConnected,

    /// A client has disconnected from the server.
    ///
    /// Emitted when a client closes the connection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::ClientDisconnected;
    /// ```
    ClientDisconnected,

    /// A tool is about to be called.
    ///
    /// Emitted before a tool handler is invoked, providing the tool name
    /// and arguments.
    ///
    /// # Fields
    ///
    /// * `name` - The name of the tool being called
    /// * `args` - The arguments passed to the tool (JSON value)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    /// use serde_json::json;
    ///
    /// let event = HookEvent::ToolCalled {
    ///     name: "echo".to_string(),
    ///     args: json!({"message": "hello world"}),
    /// };
    /// ```
    ToolCalled {
        /// Tool name
        name: String,
        /// Tool arguments
        args: Value,
    },

    /// A tool call has completed.
    ///
    /// Emitted after a tool handler finishes, providing the result or error.
    ///
    /// # Fields
    ///
    /// * `name` - The name of the tool that was called
    /// * `result` - The result of the tool call (Ok for success, Err for failure)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    /// use serde_json::json;
    ///
    /// // Successful completion
    /// let event = HookEvent::ToolCompleted {
    ///     name: "echo".to_string(),
    ///     result: Ok(json!({"output": "hello world"})),
    /// };
    ///
    /// // Failed completion
    /// let event = HookEvent::ToolCompleted {
    ///     name: "echo".to_string(),
    ///     result: Err("Tool execution failed".to_string()),
    /// };
    /// ```
    ToolCompleted {
        /// Tool name
        name: String,
        /// Tool result (Ok with JSON value, or Err with error message)
        result: Result<Value, String>,
    },

    /// A resource has been read.
    ///
    /// Emitted after a resource is successfully read.
    ///
    /// # Fields
    ///
    /// * `uri` - The URI of the resource that was read
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::ResourceRead {
    ///     uri: "file:///config.json".to_string(),
    /// };
    /// ```
    ResourceRead {
        /// Resource URI
        uri: String,
    },

    /// An error occurred.
    ///
    /// Emitted when an error occurs during server operation. This includes
    /// tool errors, resource errors, transport errors, etc.
    ///
    /// # Fields
    ///
    /// * `error` - Error message
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let event = HookEvent::Error {
    ///     error: "Failed to connect to database".to_string(),
    /// };
    /// ```
    Error {
        /// Error message
        error: String,
    },
}

impl HookEvent {
    /// Get a human-readable event type name.
    ///
    /// Returns a string identifying the type of event, useful for logging and filtering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    /// use serde_json::json;
    ///
    /// assert_eq!(HookEvent::ServerStarted.event_type(), "server_started");
    ///
    /// let tool_event = HookEvent::ToolCalled {
    ///     name: "echo".to_string(),
    ///     args: json!({}),
    /// };
    /// assert_eq!(tool_event.event_type(), "tool_called");
    /// ```
    pub fn event_type(&self) -> &'static str {
        match self {
            HookEvent::ServerStarted => "server_started",
            HookEvent::ServerStopped => "server_stopped",
            HookEvent::ClientConnected => "client_connected",
            HookEvent::ClientDisconnected => "client_disconnected",
            HookEvent::ToolCalled { .. } => "tool_called",
            HookEvent::ToolCompleted { .. } => "tool_completed",
            HookEvent::ResourceRead { .. } => "resource_read",
            HookEvent::Error { .. } => "error",
        }
    }

    /// Check if this is a lifecycle event.
    ///
    /// Returns `true` for ServerStarted and ServerStopped events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// assert!(HookEvent::ServerStarted.is_lifecycle_event());
    /// assert!(HookEvent::ServerStopped.is_lifecycle_event());
    /// assert!(!HookEvent::ClientConnected.is_lifecycle_event());
    /// ```
    pub fn is_lifecycle_event(&self) -> bool {
        matches!(self, HookEvent::ServerStarted | HookEvent::ServerStopped)
    }

    /// Check if this is a tool event.
    ///
    /// Returns `true` for ToolCalled and ToolCompleted events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    /// use serde_json::json;
    ///
    /// let tool_called = HookEvent::ToolCalled {
    ///     name: "test".to_string(),
    ///     args: json!({}),
    /// };
    /// assert!(tool_called.is_tool_event());
    ///
    /// assert!(!HookEvent::ServerStarted.is_tool_event());
    /// ```
    pub fn is_tool_event(&self) -> bool {
        matches!(
            self,
            HookEvent::ToolCalled { .. } | HookEvent::ToolCompleted { .. }
        )
    }

    /// Check if this is an error event.
    ///
    /// Returns `true` for Error events.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookEvent;
    ///
    /// let error = HookEvent::Error {
    ///     error: "test error".to_string(),
    /// };
    /// assert!(error.is_error_event());
    ///
    /// assert!(!HookEvent::ServerStarted.is_error_event());
    /// ```
    pub fn is_error_event(&self) -> bool {
        matches!(self, HookEvent::Error { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_server_started_event() {
        let event = HookEvent::ServerStarted;
        assert_eq!(event.event_type(), "server_started");
        assert!(event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_server_stopped_event() {
        let event = HookEvent::ServerStopped;
        assert_eq!(event.event_type(), "server_stopped");
        assert!(event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_client_connected_event() {
        let event = HookEvent::ClientConnected;
        assert_eq!(event.event_type(), "client_connected");
        assert!(!event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_client_disconnected_event() {
        let event = HookEvent::ClientDisconnected;
        assert_eq!(event.event_type(), "client_disconnected");
        assert!(!event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_tool_called_event() {
        let event = HookEvent::ToolCalled {
            name: "echo".to_string(),
            args: json!({"message": "hello"}),
        };
        assert_eq!(event.event_type(), "tool_called");
        assert!(!event.is_lifecycle_event());
        assert!(event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_tool_completed_success_event() {
        let event = HookEvent::ToolCompleted {
            name: "echo".to_string(),
            result: Ok(json!({"output": "hello"})),
        };
        assert_eq!(event.event_type(), "tool_completed");
        assert!(!event.is_lifecycle_event());
        assert!(event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_tool_completed_error_event() {
        let event = HookEvent::ToolCompleted {
            name: "echo".to_string(),
            result: Err("execution failed".to_string()),
        };
        assert_eq!(event.event_type(), "tool_completed");
        assert!(!event.is_lifecycle_event());
        assert!(event.is_tool_event());
        assert!(!event.is_error_event()); // Note: This is NOT an Error event, it's a tool completion with error result
    }

    #[test]
    fn test_resource_read_event() {
        let event = HookEvent::ResourceRead {
            uri: "file:///config.json".to_string(),
        };
        assert_eq!(event.event_type(), "resource_read");
        assert!(!event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(!event.is_error_event());
    }

    #[test]
    fn test_error_event() {
        let event = HookEvent::Error {
            error: "test error".to_string(),
        };
        assert_eq!(event.event_type(), "error");
        assert!(!event.is_lifecycle_event());
        assert!(!event.is_tool_event());
        assert!(event.is_error_event());
    }

    #[test]
    fn test_event_clone() {
        let event = HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({"key": "value"}),
        };
        let cloned = event.clone();

        assert_eq!(event.event_type(), cloned.event_type());
    }

    #[test]
    fn test_event_debug() {
        let event = HookEvent::ServerStarted;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("ServerStarted"));
    }

    #[test]
    fn test_all_event_types() {
        let events = vec![
            HookEvent::ServerStarted,
            HookEvent::ServerStopped,
            HookEvent::ClientConnected,
            HookEvent::ClientDisconnected,
            HookEvent::ToolCalled {
                name: "test".to_string(),
                args: json!({}),
            },
            HookEvent::ToolCompleted {
                name: "test".to_string(),
                result: Ok(json!({})),
            },
            HookEvent::ResourceRead {
                uri: "test://uri".to_string(),
            },
            HookEvent::Error {
                error: "test".to_string(),
            },
        ];

        // All events should have unique type strings
        let type_strings: Vec<&str> = events.iter().map(|e| e.event_type()).collect();
        assert_eq!(type_strings.len(), 8);
    }

    #[test]
    fn test_event_type_strings() {
        assert_eq!(HookEvent::ServerStarted.event_type(), "server_started");
        assert_eq!(HookEvent::ServerStopped.event_type(), "server_stopped");
        assert_eq!(
            HookEvent::ClientConnected.event_type(),
            "client_connected"
        );
        assert_eq!(
            HookEvent::ClientDisconnected.event_type(),
            "client_disconnected"
        );

        let tool_called = HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({}),
        };
        assert_eq!(tool_called.event_type(), "tool_called");

        let tool_completed = HookEvent::ToolCompleted {
            name: "test".to_string(),
            result: Ok(json!({})),
        };
        assert_eq!(tool_completed.event_type(), "tool_completed");

        let resource_read = HookEvent::ResourceRead {
            uri: "test://uri".to_string(),
        };
        assert_eq!(resource_read.event_type(), "resource_read");

        let error = HookEvent::Error {
            error: "test".to_string(),
        };
        assert_eq!(error.event_type(), "error");
    }

    #[test]
    fn test_lifecycle_events() {
        assert!(HookEvent::ServerStarted.is_lifecycle_event());
        assert!(HookEvent::ServerStopped.is_lifecycle_event());
        assert!(!HookEvent::ClientConnected.is_lifecycle_event());
        assert!(!HookEvent::ClientDisconnected.is_lifecycle_event());

        let tool_called = HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({}),
        };
        assert!(!tool_called.is_lifecycle_event());

        let error = HookEvent::Error {
            error: "test".to_string(),
        };
        assert!(!error.is_lifecycle_event());
    }

    #[test]
    fn test_tool_events() {
        let tool_called = HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({}),
        };
        assert!(tool_called.is_tool_event());

        let tool_completed = HookEvent::ToolCompleted {
            name: "test".to_string(),
            result: Ok(json!({})),
        };
        assert!(tool_completed.is_tool_event());

        assert!(!HookEvent::ServerStarted.is_tool_event());
        assert!(!HookEvent::ClientConnected.is_tool_event());

        let error = HookEvent::Error {
            error: "test".to_string(),
        };
        assert!(!error.is_tool_event());
    }

    #[test]
    fn test_error_events() {
        let error = HookEvent::Error {
            error: "test error".to_string(),
        };
        assert!(error.is_error_event());

        assert!(!HookEvent::ServerStarted.is_error_event());
        assert!(!HookEvent::ClientConnected.is_error_event());

        let tool_called = HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({}),
        };
        assert!(!tool_called.is_error_event());

        // Tool completion with error result is NOT an error event
        let tool_completed_with_error = HookEvent::ToolCompleted {
            name: "test".to_string(),
            result: Err("execution failed".to_string()),
        };
        assert!(!tool_completed_with_error.is_error_event());
    }
}
