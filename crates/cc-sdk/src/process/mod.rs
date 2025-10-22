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
//! use cc_sdk::process::{ProcessRegistry, ProcessHandle};
//! use cc_sdk::core::SessionId;
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

pub use registry::{ProcessHandle, ProcessRegistry};
