//! Hook system for event-driven server monitoring.
//!
//! This module provides a flexible hook system that allows reacting to various
//! events that occur during MCP server operation. Hooks enable:
//!
//! - Logging and monitoring
//! - Auditing and compliance
//! - Analytics collection
//! - External system integration
//! - Error tracking
//!
//! # Architecture
//!
//! The hook system consists of three main components:
//!
//! 1. **HookEvent**: Enum of all events that can occur
//! 2. **Hook**: Trait for implementing event handlers
//! 3. **HookRegistry**: Manager for registering hooks and emitting events
//!
//! ```text
//! ┌─────────────┐
//! │   Server    │
//! └──────┬──────┘
//!        │
//!        ├─ emit(Event) ──▶ ┌──────────────┐
//!        │                  │ HookRegistry │
//!        │                  └──────┬───────┘
//!        │                         │
//!        │                    ┌────┴────┐
//!        │                    │         │
//!        │                  Hook1     Hook2
//!        │                    │         │
//!        └─────────────────on_event  on_event
//! ```
//!
//! # Features
//!
//! - **Type-Safe Events**: Strongly-typed event enum
//! - **Async Hooks**: All hooks are async for I/O operations
//! - **Error Isolation**: Hook errors don't affect server or other hooks
//! - **Thread-Safe**: Concurrent hook registration and event emission
//! - **Zero Overhead**: Hooks are only called when events occur
//!
//! # Examples
//!
//! ## Basic Hook Implementation
//!
//! ```rust
//! use mcp_server::hooks::{Hook, HookEvent, HookRegistry};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl Hook for LoggingHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         match event {
//!             HookEvent::ServerStarted => {
//!                 println!("Server started!");
//!             }
//!             HookEvent::ToolCalled { name, .. } => {
//!                 println!("Tool called: {}", name);
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = HookRegistry::new();
//! registry.register(LoggingHook).await;
//!
//! // Emit events
//! registry.emit(&HookEvent::ServerStarted).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## Audit Hook with State
//!
//! ```rust
//! use mcp_server::hooks::{Hook, HookEvent};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! struct AuditHook {
//!     events: Arc<Mutex<Vec<String>>>,
//! }
//!
//! impl AuditHook {
//!     fn new() -> Self {
//!         Self {
//!             events: Arc::new(Mutex::new(Vec::new())),
//!         }
//!     }
//! }
//!
//! #[async_trait]
//! impl Hook for AuditHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         let event_type = event.event_type().to_string();
//!         let mut events = self.events.lock().await;
//!         events.push(event_type);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Selective Hook
//!
//! ```rust
//! use mcp_server::hooks::{Hook, HookEvent};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct ToolMonitorHook;
//!
//! #[async_trait]
//! impl Hook for ToolMonitorHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         // Only process tool events
//!         if !event.is_tool_event() {
//!             return Ok(());
//!         }
//!
//!         match event {
//!             HookEvent::ToolCalled { name, args } => {
//!                 println!("Tool '{}' called with args: {:?}", name, args);
//!             }
//!             HookEvent::ToolCompleted { name, result } => {
//!                 println!("Tool '{}' completed: {:?}", name, result.is_ok());
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Using with Server
//!
//! ```rust,no_run
//! use mcp_server::hooks::{Hook, HookEvent, HookRegistry};
//! use mcp_server::error::MiddlewareError;
//! use mcp_server::McpServer;
//! use async_trait::async_trait;
//!
//! struct MyHook;
//!
//! #[async_trait]
//! impl Hook for MyHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         // Handle events
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let hooks = HookRegistry::new();
//! hooks.register(MyHook).await;
//!
//! let server = McpServer::builder()
//!     .name("my-server")
//!     .hooks(hooks)
//!     .build()?;
//! # Ok(())
//! # }
//! ```

pub mod events;
pub mod registry;
pub mod traits;

pub use events::HookEvent;
pub use registry::HookRegistry;
pub use traits::Hook;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MiddlewareError;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    // Integration test: Full hook lifecycle
    struct TestHook {
        server_started: Arc<AtomicUsize>,
        tools_called: Arc<AtomicUsize>,
        errors: Arc<AtomicUsize>,
    }

    impl TestHook {
        fn new() -> Self {
            Self {
                server_started: Arc::new(AtomicUsize::new(0)),
                tools_called: Arc::new(AtomicUsize::new(0)),
                errors: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn server_started_count(&self) -> usize {
            self.server_started.load(Ordering::Relaxed)
        }

        fn tools_called_count(&self) -> usize {
            self.tools_called.load(Ordering::Relaxed)
        }

        fn errors_count(&self) -> usize {
            self.errors.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl Hook for TestHook {
        async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
            match event {
                HookEvent::ServerStarted => {
                    self.server_started.fetch_add(1, Ordering::Relaxed);
                }
                HookEvent::ToolCalled { .. } => {
                    self.tools_called.fetch_add(1, Ordering::Relaxed);
                }
                HookEvent::Error { .. } => {
                    self.errors.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_system_integration() {
        let hook = Arc::new(TestHook::new());
        let registry = HookRegistry::new();

        registry.register(hook.clone()).await;

        // Emit various events
        registry.emit(&HookEvent::ServerStarted).await;
        registry
            .emit(&HookEvent::ToolCalled {
                name: "test1".to_string(),
                args: json!({}),
            })
            .await;
        registry
            .emit(&HookEvent::ToolCalled {
                name: "test2".to_string(),
                args: json!({}),
            })
            .await;
        registry
            .emit(&HookEvent::Error {
                error: "test".to_string(),
            })
            .await;

        // Verify counts
        assert_eq!(hook.server_started_count(), 1);
        assert_eq!(hook.tools_called_count(), 2);
        assert_eq!(hook.errors_count(), 1);
    }

    #[tokio::test]
    async fn test_multiple_hooks_receive_same_event() {
        let registry = HookRegistry::new();

        let hook1 = Arc::new(TestHook::new());
        let hook2 = Arc::new(TestHook::new());

        registry.register(hook1.clone()).await;
        registry.register(hook2.clone()).await;

        registry.emit(&HookEvent::ServerStarted).await;

        assert_eq!(hook1.server_started_count(), 1);
        assert_eq!(hook2.server_started_count(), 1);
    }

    #[tokio::test]
    async fn test_hook_event_type_categorization() {
        // Lifecycle events
        assert!(HookEvent::ServerStarted.is_lifecycle_event());
        assert!(HookEvent::ServerStopped.is_lifecycle_event());

        // Tool events
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

        // Error events
        let error = HookEvent::Error {
            error: "test".to_string(),
        };
        assert!(error.is_error_event());

        // Non-categorized events
        assert!(!HookEvent::ClientConnected.is_lifecycle_event());
        assert!(!HookEvent::ClientConnected.is_tool_event());
        assert!(!HookEvent::ClientConnected.is_error_event());
    }

    #[tokio::test]
    async fn test_hook_registry_lifecycle() {
        let registry = HookRegistry::new();
        assert_eq!(registry.count().await, 0);

        struct NoOpHook;

        #[async_trait]
        impl Hook for NoOpHook {
            async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
                Ok(())
            }
        }

        registry.register(NoOpHook).await;
        assert_eq!(registry.count().await, 1);

        registry.register(NoOpHook).await;
        assert_eq!(registry.count().await, 2);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_hook_with_all_event_types() {
        let hook = Arc::new(TestHook::new());
        let registry = HookRegistry::new();

        registry.register(hook.clone()).await;

        let all_events = vec![
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

        for event in &all_events {
            registry.emit(event).await;
        }

        // TestHook only counts specific events
        assert_eq!(hook.server_started_count(), 1);
        assert_eq!(hook.tools_called_count(), 1);
        assert_eq!(hook.errors_count(), 1);
    }

    // Helper function to create a function-based hook for testing
    struct FnHook<F>
    where
        F: Fn(&HookEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), MiddlewareError>> + Send>>
            + Send
            + Sync,
    {
        f: F,
    }

    #[async_trait]
    impl<F> Hook for FnHook<F>
    where
        F: Fn(&HookEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), MiddlewareError>> + Send>>
            + Send
            + Sync,
    {
        async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
            (self.f)(event).await
        }
    }

    impl HookRegistry {
        async fn register_test_fn<F, Fut>(&self, f: F)
        where
            F: Fn(&HookEvent) -> Fut + Send + Sync + 'static,
            Fut: std::future::Future<Output = Result<(), MiddlewareError>> + Send + 'static,
        {
            let hook = FnHook {
                f: move |event: &HookEvent| Box::pin(f(event)),
            };
            self.register(hook).await;
        }
    }

    #[tokio::test]
    async fn test_hook_registry_clone_shares_state() {
        let registry = HookRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        registry
            .register_test_fn(move |_event: &HookEvent| {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .await;

        let cloned = registry.clone();
        cloned.emit(&HookEvent::ServerStarted).await;

        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
