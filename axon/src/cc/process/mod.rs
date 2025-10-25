//! Process management and registry for concurrent Claude sessions.
//!
//! This module provides a thread-safe registry for tracking multiple concurrent Claude processes,
//! their lifecycle, live output buffering, and status checking. It's designed to enable applications
//! to manage multiple Claude sessions simultaneously.
//!
//! # Features
//!
//! - **Concurrent Process Tracking**: Track multiple processes by session ID with thread-safe access
//! - **Live Output Buffering**: Buffer process output in memory for real-time monitoring
//! - **Process Lifecycle Management**: Register, track, and gracefully terminate processes
//! - **Cross-Platform**: Works on Unix and Windows with platform-specific process control
//! - **Status Checking**: Check if processes are running and clean up finished processes
//!
//! # Examples
//!
//! ```rust,no_run
//! use crate::cc::process::{ProcessRegistry, ProcessHandle};
//! use crate::cc::core::SessionId;
//! use tokio::process::Command;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a registry
//!     let registry = ProcessRegistry::new();
//!
//!     // Spawn a process
//!     let session_id = SessionId::generate();
//!     let child = Command::new("claude")
//!         .arg("chat")
//!         .spawn()?;
//!
//!     // Register it
//!     let handle = registry.register(session_id.clone(), child)?;
//!     println!("Registered process with PID: {}", handle.pid);
//!
//!     // List active sessions
//!     let active = registry.list_active();
//!     println!("Active sessions: {:?}", active);
//!
//!     // Get output
//!     if let Some(output) = registry.get_output(&session_id) {
//!         println!("Output: {}", output);
//!     }
//!
//!     // Gracefully shutdown
//!     registry.kill(&session_id, true).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod registry;

pub use registry::{ProcessHandle, ProcessInfo, ProcessRegistry, ProcessType};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cc::core::SessionId;
    use std::time::Duration;

    /// Test that ProcessRegistry is re-exported correctly
    #[test]
    fn test_process_registry_export() {
        let registry = ProcessRegistry::new();
        assert_eq!(registry.list_active().len(), 0);
    }

    /// Test that ProcessHandle is re-exported correctly
    #[tokio::test]
    async fn test_process_handle_export() {
        let session_id = SessionId::new("test");
        let child = tokio::process::Command::new("echo")
            .arg("test")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id.clone(), child, pid);
        assert_eq!(handle.session_id, session_id);
        assert_eq!(handle.pid, pid);
    }

    /// Test module integration: registry with multiple processes
    #[tokio::test]
    async fn test_module_integration_multiple_processes() {
        let registry = ProcessRegistry::new();
        let mut session_ids = Vec::new();

        // Register multiple processes
        for i in 0..5 {
            let session_id = SessionId::new(format!("session-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg(format!("test-{}", i))
                .spawn()
                .expect("Failed to spawn");

            registry.register(session_id.clone(), child).expect("Failed to register");
            session_ids.push(session_id);
        }

        // Verify all registered
        assert_eq!(registry.list_active().len(), 5);

        // Verify we can get each one
        for session_id in &session_ids {
            assert!(registry.get(session_id).is_some());
        }
    }

    /// Test module integration: process lifecycle
    #[tokio::test]
    async fn test_module_integration_process_lifecycle() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::generate();

        // 1. Register
        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn");
        let handle = registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // 2. Append output
        handle.append_output("test output\n").await.expect("Failed to append");

        // 3. Get output through registry
        let output = registry.get_output(&session_id).await;
        assert!(output.is_some());
        assert_eq!(output.unwrap(), "test output\n");

        // 4. Wait for process to finish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 5. Cleanup
        let cleaned = registry.cleanup_finished().await;
        assert!(cleaned > 0);

        // 6. Verify removed
        assert!(registry.get(&session_id).is_none());
    }

    /// Test module integration: concurrent access
    #[tokio::test]
    async fn test_module_integration_concurrent_access() {
        use std::sync::Arc;

        let registry = Arc::new(ProcessRegistry::new());
        let mut handles = vec![];

        // Spawn multiple tasks that register processes concurrently
        for i in 0..10 {
            let registry = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let session_id = SessionId::new(format!("concurrent-{}", i));
                let child = tokio::process::Command::new("echo")
                    .arg(format!("output-{}", i))
                    .spawn()
                    .expect("Failed to spawn");

                registry.register(session_id, child).expect("Failed to register");
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify all registered
        assert_eq!(registry.list_active().len(), 10);
    }

    /// Test that ProcessHandle tracks process metadata correctly
    #[tokio::test]
    async fn test_process_metadata_tracking() {
        let session_id = SessionId::new("metadata-test");
        let child = tokio::process::Command::new("sleep")
            .arg("0.1")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id.clone(), child, pid);

        // Verify metadata
        assert_eq!(handle.session_id, session_id);
        assert_eq!(handle.pid, pid);

        // Verify started_at is recent
        let uptime = handle.uptime().expect("Failed to get uptime");
        assert!(uptime < Duration::from_secs(1));

        // Verify process is initially running
        assert!(handle.is_running().await.expect("Failed to check status"));
    }

    /// Test that SessionId works correctly with the module
    #[tokio::test]
    async fn test_session_id_integration() {
        let registry = ProcessRegistry::new();

        // Test with generated ID
        let session1 = SessionId::generate();
        let child1 = tokio::process::Command::new("echo")
            .arg("test1")
            .spawn()
            .expect("Failed to spawn");
        registry.register(session1.clone(), child1).expect("Failed to register");

        // Test with named ID
        let session2 = SessionId::new("named-session");
        let child2 = tokio::process::Command::new("echo")
            .arg("test2")
            .spawn()
            .expect("Failed to spawn");
        registry.register(session2.clone(), child2).expect("Failed to register");

        // Verify both are tracked
        assert!(registry.get(&session1).is_some());
        assert!(registry.get(&session2).is_some());
        assert_eq!(registry.list_active().len(), 2);
    }

    /// Test error handling when process fails to spawn
    #[tokio::test]
    async fn test_error_handling_spawn_failure() {
        let result = tokio::process::Command::new("nonexistent-command-xyz")
            .spawn();

        assert!(result.is_err());
    }

    /// Test that module exports work with Default trait
    #[test]
    fn test_default_trait() {
        let registry1 = ProcessRegistry::default();
        let registry2 = ProcessRegistry::new();

        assert_eq!(registry1.list_active().len(), 0);
        assert_eq!(registry2.list_active().len(), 0);
    }
}
