//! Hook registry for managing and emitting events.
//!
//! This module provides the `HookRegistry` which manages all registered hooks
//! and distributes events to them. The registry ensures thread-safe hook
//! management and graceful error handling.
//!
//! # Features
//!
//! - **Thread-Safe**: Uses `Arc<RwLock<>>` for concurrent access
//! - **Error Isolation**: Hook errors don't affect other hooks or server operation
//! - **Async Event Emission**: All hooks run asynchronously
//! - **Multiple Hooks**: Support for registering multiple hooks
//!
//! # Examples
//!
//! ```rust
//! use mcp_server::hooks::{HookRegistry, Hook, HookEvent};
//! use mcp_server::error::MiddlewareError;
//! use async_trait::async_trait;
//!
//! struct MyHook;
//!
//! #[async_trait]
//! impl Hook for MyHook {
//!     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
//!         println!("Event: {}", event.event_type());
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let registry = HookRegistry::new();
//!
//! // Register hook
//! registry.register(MyHook).await;
//!
//! // Emit events
//! registry.emit(&HookEvent::ServerStarted).await;
//! # Ok(())
//! # }
//! ```

use crate::hooks::{Hook, HookEvent};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing hooks and distributing events.
///
/// The `HookRegistry` maintains a collection of registered hooks and provides
/// methods to emit events to all hooks. It ensures thread-safe access and
/// graceful error handling.
///
/// # Thread Safety
///
/// The registry uses `Arc<RwLock<>>` internally, making it safe to share
/// across threads and clone cheaply.
///
/// # Error Handling
///
/// When an event is emitted, if a hook returns an error:
/// - The error is logged (via `tracing::error!`)
/// - Other hooks continue to execute
/// - The server continues operating normally
///
/// This ensures that a failing hook doesn't crash the server or prevent
/// other hooks from functioning.
///
/// # Examples
///
/// ```rust
/// use mcp_server::hooks::{HookRegistry, Hook, HookEvent};
/// use mcp_server::error::MiddlewareError;
/// use async_trait::async_trait;
///
/// struct LoggingHook;
///
/// #[async_trait]
/// impl Hook for LoggingHook {
///     async fn on_event(&self, event: &HookEvent) -> Result<(), MiddlewareError> {
///         println!("Event: {}", event.event_type());
///         Ok(())
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let registry = HookRegistry::new();
///
/// // Register multiple hooks
/// registry.register(LoggingHook).await;
/// registry.register(LoggingHook).await;
///
/// // Emit event to all hooks
/// registry.emit(&HookEvent::ServerStarted).await;
///
/// // Get hook count
/// assert_eq!(registry.count().await, 2);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HookRegistry {
    hooks: Arc<RwLock<Vec<Arc<dyn Hook>>>>,
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("hooks", &"<Vec<Arc<dyn Hook>>>")
            .finish()
    }
}

impl HookRegistry {
    /// Create a new empty hook registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::HookRegistry;
    ///
    /// let registry = HookRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new hook.
    ///
    /// The hook will receive all future events emitted to this registry.
    ///
    /// # Arguments
    ///
    /// * `hook` - The hook implementation to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::{HookRegistry, Hook, HookEvent};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct MyHook;
    ///
    /// #[async_trait]
    /// impl Hook for MyHook {
    ///     async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = HookRegistry::new();
    /// registry.register(MyHook).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register<H: Hook + 'static>(&self, hook: H) {
        let mut hooks = self.hooks.write().await;
        hooks.push(Arc::new(hook));
    }

    /// Register an Arc-wrapped hook.
    ///
    /// This is useful when you already have an Arc-wrapped hook instance.
    ///
    /// # Arguments
    ///
    /// * `hook` - Arc-wrapped hook to register
    pub async fn register_arc(&self, hook: Arc<dyn Hook>) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
    }

    /// Emit an event to all registered hooks.
    ///
    /// All hooks will receive the event. If a hook returns an error, it's logged
    /// but doesn't prevent other hooks from executing or affect server operation.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::{HookRegistry, HookEvent};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = HookRegistry::new();
    ///
    /// // Emit lifecycle event
    /// registry.emit(&HookEvent::ServerStarted).await;
    ///
    /// // Emit tool event
    /// registry.emit(&HookEvent::ToolCalled {
    ///     name: "echo".to_string(),
    ///     args: serde_json::json!({"message": "hello"}),
    /// }).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn emit(&self, event: &HookEvent) {
        let hooks = self.hooks.read().await;

        for hook in hooks.iter() {
            if let Err(e) = hook.on_event(event).await {
                tracing::error!(
                    event_type = %event.event_type(),
                    error = %e,
                    "Hook error"
                );
            }
        }
    }

    /// Get the number of registered hooks.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::{HookRegistry, Hook, HookEvent};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct MyHook;
    ///
    /// #[async_trait]
    /// impl Hook for MyHook {
    ///     async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = HookRegistry::new();
    /// assert_eq!(registry.count().await, 0);
    ///
    /// registry.register(MyHook).await;
    /// assert_eq!(registry.count().await, 1);
    ///
    /// registry.register(MyHook).await;
    /// assert_eq!(registry.count().await, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(&self) -> usize {
        let hooks = self.hooks.read().await;
        hooks.len()
    }

    /// Clear all registered hooks.
    ///
    /// This removes all hooks from the registry. Useful for testing or resetting state.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::hooks::{HookRegistry, Hook, HookEvent};
    /// use mcp_server::error::MiddlewareError;
    /// use async_trait::async_trait;
    ///
    /// struct MyHook;
    ///
    /// #[async_trait]
    /// impl Hook for MyHook {
    ///     async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = HookRegistry::new();
    /// registry.register(MyHook).await;
    /// assert_eq!(registry.count().await, 1);
    ///
    /// registry.clear().await;
    /// assert_eq!(registry.count().await, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn clear(&self) {
        let mut hooks = self.hooks.write().await;
        hooks.clear();
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MiddlewareError;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Test hook that counts events
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

    // Test hook that fails
    struct FailingHook;

    #[async_trait]
    impl Hook for FailingHook {
        async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
            Err(MiddlewareError::Internal(anyhow::anyhow!(
                "Hook failed"
            )))
        }
    }

    // Test hook that panics (should be handled gracefully)
    struct NoOpHook;

    #[async_trait]
    impl Hook for NoOpHook {
        async fn on_event(&self, _event: &HookEvent) -> Result<(), MiddlewareError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_new_registry() {
        let registry = HookRegistry::new();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_default_registry() {
        let registry = HookRegistry::default();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_register_hook() {
        let registry = HookRegistry::new();
        registry.register(NoOpHook).await;

        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_register_multiple_hooks() {
        let registry = HookRegistry::new();

        registry.register(NoOpHook).await;
        registry.register(NoOpHook).await;
        registry.register(NoOpHook).await;

        assert_eq!(registry.count().await, 3);
    }

    #[tokio::test]
    async fn test_emit_to_single_hook() {
        let hook = Arc::new(CountingHook::new());
        let registry = HookRegistry::new();

        registry.register(hook.clone()).await;

        registry.emit(&HookEvent::ServerStarted).await;
        assert_eq!(hook.count(), 1);

        registry.emit(&HookEvent::ClientConnected).await;
        assert_eq!(hook.count(), 2);
    }

    #[tokio::test]
    async fn test_emit_to_multiple_hooks() {
        let registry = HookRegistry::new();

        let hook1 = Arc::new(CountingHook::new());
        let hook2 = Arc::new(CountingHook::new());

        registry.register(hook1.clone()).await;
        registry.register(hook2.clone()).await;

        registry.emit(&HookEvent::ServerStarted).await;

        assert_eq!(hook1.count(), 1);
        assert_eq!(hook2.count(), 1);
    }

    #[tokio::test]
    async fn test_failing_hook_doesnt_stop_others() {
        let registry = HookRegistry::new();

        let hook1 = Arc::new(CountingHook::new());

        // Register a counting hook
        registry.register(hook1.clone()).await;

        // Register a failing hook
        registry.register(FailingHook).await;

        // Register another counting hook
        let hook2 = Arc::new(CountingHook::new());
        registry.register(hook2.clone()).await;

        // Emit event - all hooks should be called despite the failing one
        registry.emit(&HookEvent::ServerStarted).await;

        assert_eq!(hook1.count(), 1);
        assert_eq!(hook2.count(), 1);
    }

    #[tokio::test]
    async fn test_clear_hooks() {
        let registry = HookRegistry::new();

        registry.register(NoOpHook).await;
        registry.register(NoOpHook).await;
        assert_eq!(registry.count().await, 2);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_emit_different_event_types() {
        let hook = Arc::new(CountingHook::new());
        let registry = HookRegistry::new();

        registry.register(hook.clone()).await;

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
                error: "test".to_string(),
            },
        ];

        for event in &events {
            registry.emit(event).await;
        }

        assert_eq!(hook.count(), events.len());
    }

    #[tokio::test]
    async fn test_registry_clone() {
        let registry = HookRegistry::new();
        registry.register(NoOpHook).await;

        let cloned = registry.clone();
        assert_eq!(cloned.count().await, 1);

        // Registering on clone affects original
        cloned.register(NoOpHook).await;
        assert_eq!(registry.count().await, 2);
        assert_eq!(cloned.count().await, 2);
    }

    #[tokio::test]
    async fn test_registry_debug() {
        let registry = HookRegistry::new();
        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("HookRegistry"));
    }

    #[tokio::test]
    async fn test_concurrent_hook_registration() {
        let registry = Arc::new(HookRegistry::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let reg = registry.clone();
            let handle = tokio::spawn(async move {
                reg.register(NoOpHook).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(registry.count().await, 10);
    }

    #[tokio::test]
    async fn test_concurrent_event_emission() {
        let hook = Arc::new(CountingHook::new());
        let registry = Arc::new(HookRegistry::new());

        registry.register(hook.clone()).await;

        let mut handles = vec![];

        for i in 0..10 {
            let reg = registry.clone();
            let handle = tokio::spawn(async move {
                let event = HookEvent::ToolCalled {
                    name: format!("tool_{}", i),
                    args: json!({}),
                };
                reg.emit(&event).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(hook.count(), 10);
    }

    #[tokio::test]
    async fn test_emit_with_no_hooks() {
        let registry = HookRegistry::new();
        // Should not panic or error
        registry.emit(&HookEvent::ServerStarted).await;
    }

    #[tokio::test]
    async fn test_clear_and_re_register() {
        let registry = HookRegistry::new();

        registry.register(NoOpHook).await;
        assert_eq!(registry.count().await, 1);

        registry.clear().await;
        assert_eq!(registry.count().await, 0);

        registry.register(NoOpHook).await;
        assert_eq!(registry.count().await, 1);
    }

    // Helper function hook for testing
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

    // Simplified hook registration for testing
    impl HookRegistry {
        async fn register_fn<F, Fut>(&self, f: F)
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
    async fn test_function_hook() {
        let registry = HookRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        registry
            .register_fn(move |_event: &HookEvent| {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            })
            .await;

        registry.emit(&HookEvent::ServerStarted).await;
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
