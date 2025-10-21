//! Hook trait definition.
//!
//! This module defines the core `Hook` trait that all hook implementations must implement.
//!
//! # Trait Overview
//!
//! The `Hook` trait provides a single method `on_event` that is called whenever
//! an event occurs in the MCP server. Hooks can:
//! - React to server lifecycle events
//! - Monitor tool calls and completions
//! - Track resource access
//! - Handle errors
//!
//! # Error Handling
//!
//! Hook errors are logged but don't stop server operation. This ensures that a
//! failing hook doesn't crash the server or prevent other hooks from executing.
//!
//! # Examples
//!
//! ## Simple Logging Hook
//!
//! ```rust
//! use mcp_server::hooks::{Hook, HookEvent};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl Hook for LoggingHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         println!("Event: {:?}", event.event_type());
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Audit Hook with Database
//!
//! ```rust,no_run
//! use mcp_server::hooks::{Hook, HookEvent};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! struct AuditHook {
//!     // database: Arc<Database>,
//! }
//!
//! #[async_trait]
//! impl Hook for AuditHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         match event {
//!             HookEvent::ToolCalled { name, args } => {
//!                 // self.database.log_tool_call(name, args).await?;
//!                 println!("Logging tool call: {}", name);
//!             }
//!             HookEvent::ResourceRead { uri } => {
//!                 // self.database.log_resource_access(uri).await?;
//!                 println!("Logging resource read: {}", uri);
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//! ```

use crate::error::MiddlewareError;
use crate::hooks::HookEvent;
use async_trait::async_trait;

/// Hook trait for responding to server events.
///
/// Hooks provide a way to react to various events that occur during MCP server
/// operation. They're useful for:
/// - Logging and monitoring
/// - Auditing tool and resource access
/// - Collecting analytics
/// - Triggering external systems
/// - Error reporting
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to work with the async runtime.
///
/// # Error Handling
///
/// If a hook returns an error, the error is logged but doesn't affect server
/// operation. Other hooks will continue to execute normally.
///
/// # Examples
///
/// ```rust
/// use mcp_server::hooks::{Hook, HookEvent};
/// use mcp_server::error::MiddlewareError;
/// use async_trait::async_trait;
///
/// struct EventCounterHook {
///     count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
/// }
///
/// #[async_trait]
/// impl Hook for EventCounterHook {
///     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
///         self.count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
///         println!("Event #{}: {}",
///             self.count.load(std::sync::atomic::Ordering::Relaxed),
///             event.event_type()
///         );
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Hook: Send + Sync {
    /// Called when an event occurs.
    ///
    /// This method is invoked for every event emitted by the MCP server.
    /// The hook can inspect the event and perform any necessary actions.
    ///
    /// # Arguments
    ///
    /// * `event` - Reference to the event that occurred
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Event processed successfully
    /// * `Err(MiddlewareError)` - Error occurred (logged but doesn't stop server)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::{Hook, HookEvent};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct SelectiveHook;
    ///
    /// #[async_trait]
    /// impl Hook for SelectiveHook {
    ///     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
    ///         // Only handle tool events
    ///         if event.is_tool_event() {
    ///             println!("Tool event: {}", event.event_type());
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError>;
}

// Implement Hook for Arc<T: Hook> to allow registering Arc-wrapped hooks
#[async_trait]
impl<T: Hook + ?Sized> Hook for std::sync::Arc<T> {
    async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
        (**self).on_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Test hook implementations
    struct NoOpHook;

    #[async_trait]
    impl Hook for NoOpHook {
        async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
            Ok(())
        }
    }

    struct CountingHook {
        count: Arc<AtomicUsize>,
    }

    impl CountingHook {
        fn new() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn count(&self) -> usize {
            self.count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl Hook for CountingHook {
        async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    struct FailingHook {
        should_fail: bool,
    }

    #[async_trait]
    impl Hook for FailingHook {
        async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
            if self.should_fail {
                Err(MiddlewareError::Internal(anyhow::anyhow!(
                    "Hook failed"
                )))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_no_op_hook() {
        let hook = NoOpHook;
        let event = HookEvent::ServerStarted;

        let result = hook.on_event(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_counting_hook() {
        let hook = CountingHook::new();
        assert_eq!(hook.count(), 0);

        hook.on_event(&HookEvent::ServerStarted).await.unwrap();
        assert_eq!(hook.count(), 1);

        hook.on_event(&HookEvent::ClientConnected)
            .await
            .unwrap();
        assert_eq!(hook.count(), 2);

        hook.on_event(&HookEvent::ServerStopped).await.unwrap();
        assert_eq!(hook.count(), 3);
    }

    #[tokio::test]
    async fn test_failing_hook_success() {
        let hook = FailingHook { should_fail: false };
        let event = HookEvent::ServerStarted;

        let result = hook.on_event(&event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_failing_hook_failure() {
        let hook = FailingHook { should_fail: true };
        let event = HookEvent::ServerStarted;

        let result = hook.on_event(&event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hook_with_different_events() {
        let hook = CountingHook::new();

        let events = vec![
            HookEvent::ServerStarted,
            HookEvent::ClientConnected,
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
                error: "test error".to_string(),
            },
        ];

        for event in &events {
            hook.on_event(event).await.unwrap();
        }

        assert_eq!(hook.count(), events.len());
    }

    #[tokio::test]
    async fn test_hook_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoOpHook>();
        assert_send_sync::<CountingHook>();
        assert_send_sync::<FailingHook>();
    }

    #[tokio::test]
    async fn test_hook_can_be_boxed() {
        let hook: Box<dyn Hook> = Box::new(NoOpHook);
        let event = HookEvent::ServerStarted;

        hook.on_event(&event).await.unwrap();
    }

    #[tokio::test]
    async fn test_hook_can_be_arced() {
        let hook: Arc<dyn Hook> = Arc::new(NoOpHook);
        let event = HookEvent::ServerStarted;

        hook.on_event(&event).await.unwrap();

        // Clone and use
        let hook_clone = hook.clone();
        hook_clone.on_event(&event).await.unwrap();
    }

    // Selective hook that only processes certain events
    struct SelectiveHook {
        tool_event_count: Arc<AtomicUsize>,
        error_event_count: Arc<AtomicUsize>,
    }

    impl SelectiveHook {
        fn new() -> Self {
            Self {
                tool_event_count: Arc::new(AtomicUsize::new(0)),
                error_event_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn tool_events(&self) -> usize {
            self.tool_event_count.load(Ordering::Relaxed)
        }

        fn error_events(&self) -> usize {
            self.error_event_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl Hook for SelectiveHook {
        async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
            if event.is_tool_event() {
                self.tool_event_count.fetch_add(1, Ordering::Relaxed);
            }
            if event.is_error_event() {
                self.error_event_count.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_selective_hook() {
        let hook = SelectiveHook::new();

        hook.on_event(&HookEvent::ServerStarted).await.unwrap();
        assert_eq!(hook.tool_events(), 0);
        assert_eq!(hook.error_events(), 0);

        hook.on_event(&HookEvent::ToolCalled {
            name: "test".to_string(),
            args: json!({}),
        })
        .await
        .unwrap();
        assert_eq!(hook.tool_events(), 1);
        assert_eq!(hook.error_events(), 0);

        hook.on_event(&HookEvent::Error {
            error: "test".to_string(),
        })
        .await
        .unwrap();
        assert_eq!(hook.tool_events(), 1);
        assert_eq!(hook.error_events(), 1);

        hook.on_event(&HookEvent::ToolCompleted {
            name: "test".to_string(),
            result: Ok(json!({})),
        })
        .await
        .unwrap();
        assert_eq!(hook.tool_events(), 2);
        assert_eq!(hook.error_events(), 1);
    }

    #[tokio::test]
    async fn test_hook_concurrent_execution() {
        let hook = Arc::new(CountingHook::new());
        let mut handles = vec![];

        for i in 0..10 {
            let hook_clone = hook.clone();
            let handle = tokio::spawn(async move {
                let event = HookEvent::ToolCalled {
                    name: format!("tool_{}", i),
                    args: json!({}),
                };
                hook_clone.on_event(&event).await.unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(hook.count(), 10);
    }
}
