//! Process registry implementation for managing concurrent Claude sessions.
//!
//! This module provides the core `ProcessRegistry` and `ProcessHandle` types for
//! tracking and managing multiple concurrent processes with thread-safe access.

use crate::core::SessionId;
use crate::result::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};
use tokio::process::Child;

/// Handle to a registered process with live output buffering.
///
/// This structure provides access to process information, the process handle itself,
/// and a shared buffer for capturing live output from the process.
///
/// # Thread Safety
///
/// All fields use `Arc<Mutex<_>>` to enable safe concurrent access from multiple threads.
///
/// # Examples
///
/// ```rust,no_run
/// use cc_sdk::process::ProcessHandle;
/// use cc_sdk::core::SessionId;
/// use tokio::process::Command;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let session_id = SessionId::generate();
///     let child = Command::new("claude").spawn()?;
///     let pid = child.id().unwrap();
///
///     let handle = ProcessHandle::new(session_id, child, pid);
///     println!("Created handle with PID: {}", handle.pid);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    /// Session identifier for this process
    pub session_id: SessionId,

    /// Process ID (PID)
    pub pid: u32,

    /// Timestamp when the process was started
    pub started_at: SystemTime,

    /// Live output buffer (shared across threads)
    pub output_buffer: Arc<Mutex<String>>,

    /// Child process handle (wrapped for thread-safe access)
    pub child: Arc<Mutex<Option<Child>>>,
}

impl ProcessHandle {
    /// Create a new process handle.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique identifier for the session
    /// * `child` - The spawned child process
    /// * `pid` - Process ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use cc_sdk::process::ProcessHandle;
    /// use cc_sdk::core::SessionId;
    /// use tokio::process::Command;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let session_id = SessionId::new("my-session");
    /// let child = Command::new("echo").arg("hello").spawn()?;
    /// let pid = child.id().unwrap();
    ///
    /// let handle = ProcessHandle::new(session_id, child, pid);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(session_id: SessionId, child: Child, pid: u32) -> Self {
        Self {
            session_id,
            pid,
            started_at: SystemTime::now(),
            output_buffer: Arc::new(Mutex::new(String::new())),
            child: Arc::new(Mutex::new(Some(child))),
        }
    }

    /// Append output to the live buffer.
    ///
    /// This method is thread-safe and can be called from multiple threads
    /// or tasks concurrently.
    ///
    /// # Arguments
    ///
    /// * `output` - The output string to append
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::process::ProcessHandle;
    /// use cc_sdk::core::SessionId;
    /// use tokio::process::Command;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_id = SessionId::new("test");
    /// # let child = Command::new("echo").spawn()?;
    /// # let pid = child.id().unwrap();
    /// let handle = ProcessHandle::new(session_id, child, pid);
    ///
    /// handle.append_output("Hello, world!\n")?;
    /// handle.append_output("More output...\n")?;
    ///
    /// let output = handle.get_output()?;
    /// assert!(output.contains("Hello, world!"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn append_output(&self, output: &str) -> Result<()> {
        let mut buffer = self.output_buffer
            .lock()
            .map_err(|e| crate::Error::protocol(format!("Failed to lock output buffer: {}", e)))?;
        buffer.push_str(output);
        Ok(())
    }

    /// Get a copy of the current output buffer.
    ///
    /// Returns a complete copy of all buffered output. This is thread-safe
    /// and can be called while other threads are appending to the buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cc_sdk::process::ProcessHandle;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_id = SessionId::new("test");
    /// # let child = Command::new("echo").spawn()?;
    /// # let pid = child.id().unwrap();
    /// let handle = ProcessHandle::new(session_id, child, pid);
    ///
    /// handle.append_output("test output")?;
    /// let output = handle.get_output()?;
    /// assert_eq!(output, "test output");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_output(&self) -> Result<String> {
        let buffer = self.output_buffer
            .lock()
            .map_err(|e| crate::Error::protocol(format!("Failed to lock output buffer: {}", e)))?;
        Ok(buffer.clone())
    }

    /// Clear the output buffer.
    ///
    /// This can be useful for memory management when dealing with long-running
    /// processes that produce large amounts of output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cc_sdk::process::ProcessHandle;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_id = SessionId::new("test");
    /// # let child = Command::new("echo").spawn()?;
    /// # let pid = child.id().unwrap();
    /// let handle = ProcessHandle::new(session_id, child, pid);
    ///
    /// handle.append_output("old output")?;
    /// handle.clear_output()?;
    ///
    /// let output = handle.get_output()?;
    /// assert!(output.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn clear_output(&self) -> Result<()> {
        let mut buffer = self.output_buffer
            .lock()
            .map_err(|e| crate::Error::protocol(format!("Failed to lock output buffer: {}", e)))?;
        buffer.clear();
        Ok(())
    }

    /// Check if the process is still running.
    ///
    /// Returns `true` if the process is running, `false` if it has exited.
    /// This method uses `try_wait()` which is non-blocking.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessHandle;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_id = SessionId::new("test");
    /// # let child = Command::new("sleep").arg("10").spawn()?;
    /// # let pid = child.id().unwrap();
    /// let handle = ProcessHandle::new(session_id, child, pid);
    ///
    /// if handle.is_running().await? {
    ///     println!("Process is still running");
    /// } else {
    ///     println!("Process has exited");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_running(&self) -> Result<bool> {
        let mut child_guard = self.child
            .lock()
            .map_err(|e| crate::Error::protocol(format!("Failed to lock child: {}", e)))?;

        if let Some(ref mut child) = *child_guard {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    // Process has exited
                    *child_guard = None;
                    Ok(false)
                }
                Ok(None) => {
                    // Process is still running
                    Ok(true)
                }
                Err(_) => {
                    // Error checking status, assume not running
                    *child_guard = None;
                    Ok(false)
                }
            }
        } else {
            // No child handle means process is not running
            Ok(false)
        }
    }

    /// Get the process uptime.
    ///
    /// Returns the duration since the process was started.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use cc_sdk::process::ProcessHandle;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_id = SessionId::new("test");
    /// # let child = Command::new("echo").spawn()?;
    /// # let pid = child.id().unwrap();
    /// let handle = ProcessHandle::new(session_id, child, pid);
    ///
    /// let uptime = handle.uptime()?;
    /// println!("Process has been running for {:?}", uptime);
    /// # Ok(())
    /// # }
    /// ```
    pub fn uptime(&self) -> Result<Duration> {
        self.started_at
            .elapsed()
            .map_err(|e| crate::Error::protocol(format!("Failed to get uptime: {}", e)))
    }
}

/// Thread-safe registry for tracking multiple concurrent processes.
///
/// `ProcessRegistry` provides a centralized location for managing process lifecycle,
/// tracking status, and accessing process output. It uses interior mutability with
/// `RwLock` to allow concurrent reads and exclusive writes.
///
/// # Thread Safety
///
/// The registry is designed for multi-threaded use:
/// - Multiple threads can read (get, list) simultaneously
/// - Write operations (register, unregister) take exclusive locks
/// - All methods are safe to call concurrently
///
/// # Examples
///
/// ```rust,no_run
/// use cc_sdk::process::ProcessRegistry;
/// use cc_sdk::core::SessionId;
/// use tokio::process::Command;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let registry = ProcessRegistry::new();
///
///     // Register processes
///     let session1 = SessionId::new("session-1");
///     let child1 = Command::new("claude").arg("chat").spawn()?;
///     registry.register(session1.clone(), child1)?;
///
///     let session2 = SessionId::new("session-2");
///     let child2 = Command::new("claude").arg("chat").spawn()?;
///     registry.register(session2, child2)?;
///
///     // List all active sessions
///     let active = registry.list_active();
///     println!("Active sessions: {} processes", active.len());
///
///     // Gracefully kill a session
///     registry.kill(&session1, true).await?;
///
///     // Clean up any finished processes
///     let cleaned = registry.cleanup_finished().await;
///     println!("Cleaned up {} finished processes", cleaned);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ProcessRegistry {
    /// Internal process storage (SessionId -> ProcessHandle)
    processes: Arc<RwLock<HashMap<SessionId, ProcessHandle>>>,
}

impl ProcessRegistry {
    /// Create a new process registry.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cc_sdk::process::ProcessRegistry;
    ///
    /// let registry = ProcessRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new process.
    ///
    /// Adds a process to the registry and returns a handle for accessing it.
    /// If a process with the same session ID already exists, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique identifier for the session
    /// * `child` - The spawned child process
    ///
    /// # Returns
    ///
    /// Returns a `ProcessHandle` that can be used to interact with the process.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use cc_sdk::process::ProcessRegistry;
    /// use cc_sdk::core::SessionId;
    /// use tokio::process::Command;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = ProcessRegistry::new();
    /// let session_id = SessionId::generate();
    /// let child = Command::new("claude").spawn()?;
    ///
    /// let handle = registry.register(session_id, child)?;
    /// println!("Registered process with PID: {}", handle.pid);
    /// # Ok(())
    /// # }
    /// ```
    pub fn register(&self, session_id: SessionId, child: Child) -> Result<ProcessHandle> {
        let pid = child.id().ok_or_else(|| {
            crate::Error::protocol("Failed to get process ID from child")
        })?;

        let handle = ProcessHandle::new(session_id.clone(), child, pid);

        let mut processes = self.processes
            .write()
            .map_err(|e| crate::Error::protocol(format!("Failed to lock registry: {}", e)))?;

        processes.insert(session_id, handle.clone());

        Ok(handle)
    }

    /// Get a process handle by session ID.
    ///
    /// Returns `None` if no process with the given session ID is registered.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier to look up
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = ProcessRegistry::new();
    /// # let session_id = SessionId::generate();
    /// # let child = Command::new("echo").spawn()?;
    /// # registry.register(session_id.clone(), child)?;
    /// if let Some(handle) = registry.get(&session_id) {
    ///     println!("Found process with PID: {}", handle.pid);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, session_id: &SessionId) -> Option<ProcessHandle> {
        let processes = self.processes.read().ok()?;
        processes.get(session_id).cloned()
    }

    /// Unregister a process from the registry.
    ///
    /// Removes the process from tracking and returns its handle if it existed.
    /// This does not kill the process - use `kill()` for that.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier to unregister
    ///
    /// # Returns
    ///
    /// Returns the `ProcessHandle` if the session was registered, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = ProcessRegistry::new();
    /// # let session_id = SessionId::generate();
    /// # let child = Command::new("echo").spawn()?;
    /// # registry.register(session_id.clone(), child)?;
    /// if let Some(handle) = registry.unregister(&session_id) {
    ///     println!("Unregistered process with PID: {}", handle.pid);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn unregister(&self, session_id: &SessionId) -> Option<ProcessHandle> {
        let mut processes = self.processes.write().ok()?;
        processes.remove(session_id)
    }

    /// List all active session IDs.
    ///
    /// Returns a vector of all session IDs currently registered in the registry.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = ProcessRegistry::new();
    /// # let session_id = SessionId::generate();
    /// # let child = Command::new("echo").spawn()?;
    /// # registry.register(session_id.clone(), child)?;
    /// let active_sessions = registry.list_active();
    /// println!("Active sessions: {:?}", active_sessions);
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_active(&self) -> Vec<SessionId> {
        self.processes
            .read()
            .map(|processes| processes.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the output buffer for a specific session.
    ///
    /// Returns a copy of the current output buffer contents, or `None` if
    /// the session is not registered.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # use cc_sdk::core::SessionId;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = ProcessRegistry::new();
    /// # let session_id = SessionId::generate();
    /// if let Some(output) = registry.get_output(&session_id) {
    ///     println!("Output: {}", output);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_output(&self, session_id: &SessionId) -> Option<String> {
        let handle = self.get(session_id)?;
        handle.get_output().ok()
    }

    /// Kill a process with optional graceful shutdown.
    ///
    /// If `graceful` is true, attempts to terminate the process with SIGTERM (Unix)
    /// or a graceful shutdown (Windows), waiting up to 5 seconds before escalating
    /// to SIGKILL/force termination. If `graceful` is false, immediately sends
    /// SIGKILL/force termination.
    ///
    /// After killing, the process is unregistered from the registry.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `graceful` - Whether to attempt graceful shutdown first
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the process was killed successfully, or an error if
    /// the process could not be found or killed.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # use cc_sdk::core::SessionId;
    /// # use tokio::process::Command;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let registry = ProcessRegistry::new();
    /// # let session_id = SessionId::generate();
    /// # let child = Command::new("sleep").arg("100").spawn()?;
    /// # registry.register(session_id.clone(), child)?;
    /// // Graceful shutdown
    /// registry.kill(&session_id, true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn kill(&self, session_id: &SessionId, graceful: bool) -> Result<()> {
        let handle = self.get(session_id).ok_or_else(|| {
            crate::Error::protocol(format!("Process not found: {}", session_id))
        })?;

        if graceful {
            self.kill_graceful(&handle).await?;
        } else {
            self.kill_forced(&handle).await?;
        }

        // Unregister after killing
        self.unregister(session_id);

        Ok(())
    }

    /// Gracefully kill a process (SIGTERM → SIGKILL escalation).
    async fn kill_graceful(&self, handle: &ProcessHandle) -> Result<()> {
        // Try to terminate gracefully first
        {
            let mut child_guard = handle.child
                .lock()
                .map_err(|e| crate::Error::protocol(format!("Failed to lock child: {}", e)))?;

            if let Some(ref mut child) = *child_guard {
                // Send termination signal
                if let Err(_e) = self.send_term_signal(handle.pid) {
                    // If SIGTERM fails, fall back to start_kill
                    let _ = child.start_kill();
                }
            } else {
                // Process already gone
                return Ok(());
            }
        }

        // Wait up to 5 seconds for graceful exit
        let timeout = Duration::from_secs(5);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if !handle.is_running().await? {
                // Process exited gracefully
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Timeout - escalate to SIGKILL
        self.kill_forced(handle).await
    }

    /// Force kill a process (SIGKILL/taskkill -F).
    async fn kill_forced(&self, handle: &ProcessHandle) -> Result<()> {
        // Try direct child.kill() first
        {
            let mut child_guard = handle.child
                .lock()
                .map_err(|e| crate::Error::protocol(format!("Failed to lock child: {}", e)))?;

            if let Some(ref mut child) = *child_guard {
                let _ = child.start_kill();

                // Wait a bit for kill to take effect
                tokio::time::sleep(Duration::from_millis(100)).await;

                match child.try_wait() {
                    Ok(Some(_)) => {
                        *child_guard = None;
                        return Ok(());
                    }
                    _ => {}
                }
            } else {
                return Ok(());
            }
        }

        // Fallback to system kill command
        self.kill_by_pid(handle.pid).await
    }

    /// Send SIGTERM signal to a process (Unix) or graceful termination (Windows).
    fn send_term_signal(&self, pid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()
                .map_err(|e| crate::Error::protocol(format!("Failed to send SIGTERM: {}", e)))?;

            if !output.status.success() {
                return Err(crate::Error::protocol(format!(
                    "SIGTERM failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }

        #[cfg(windows)]
        {
            // On Windows, there's no direct equivalent to SIGTERM
            // We'll use taskkill without /F flag for graceful termination
            use std::process::Command;
            let output = Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .output()
                .map_err(|e| crate::Error::protocol(format!("Failed to terminate process: {}", e)))?;

            if !output.status.success() {
                return Err(crate::Error::protocol(format!(
                    "Graceful termination failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(crate::Error::protocol("Unsupported platform for SIGTERM"))
        }
    }

    /// Kill a process by PID using system commands (SIGKILL/taskkill -F).
    async fn kill_by_pid(&self, pid: u32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .output()
                .map_err(|e| crate::Error::protocol(format!("Failed to send SIGKILL: {}", e)))?;

            if !output.status.success() {
                return Err(crate::Error::protocol(format!(
                    "SIGKILL failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .map_err(|e| crate::Error::protocol(format!("Failed to force kill: {}", e)))?;

            if !output.status.success() {
                return Err(crate::Error::protocol(format!(
                    "Force kill failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }

        #[cfg(not(any(unix, windows)))]
        {
            Err(crate::Error::protocol("Unsupported platform for force kill"))
        }
    }

    /// Clean up finished processes.
    ///
    /// Checks all registered processes and removes those that have exited.
    /// Returns the number of processes that were cleaned up.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use cc_sdk::process::ProcessRegistry;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let registry = ProcessRegistry::new();
    ///
    /// // ... register and run processes ...
    ///
    /// let cleaned = registry.cleanup_finished().await;
    /// println!("Cleaned up {} finished processes", cleaned);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cleanup_finished(&self) -> usize {
        let session_ids = self.list_active();
        let mut cleanup_count = 0;

        for session_id in session_ids {
            if let Some(handle) = self.get(&session_id) {
                match handle.is_running().await {
                    Ok(false) => {
                        // Process has finished
                        self.unregister(&session_id);
                        cleanup_count += 1;
                    }
                    _ => {}
                }
            }
        }

        cleanup_count
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_handle_creation() {
        let session_id = SessionId::new("test-session");
        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");
        let pid = child.id().expect("Failed to get PID");

        let handle = ProcessHandle::new(session_id.clone(), child, pid);

        assert_eq!(handle.session_id, session_id);
        assert_eq!(handle.pid, pid);
    }

    #[tokio::test]
    async fn test_output_buffering() {
        let session_id = SessionId::new("test-session");
        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");
        let pid = child.id().expect("Failed to get PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Test appending output
        handle.append_output("Line 1\n").expect("Failed to append");
        handle.append_output("Line 2\n").expect("Failed to append");

        let output = handle.get_output().expect("Failed to get output");
        assert_eq!(output, "Line 1\nLine 2\n");

        // Test clearing
        handle.clear_output().expect("Failed to clear");
        let output = handle.get_output().expect("Failed to get output");
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_registry_registration() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-session");

        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");

        let handle = registry.register(session_id.clone(), child)
            .expect("Failed to register");

        assert_eq!(handle.session_id, session_id);

        // Verify it's in the registry
        let retrieved = registry.get(&session_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, session_id);
    }

    #[tokio::test]
    async fn test_registry_unregister() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-session");

        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");

        registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Unregister
        let handle = registry.unregister(&session_id);
        assert!(handle.is_some());

        // Verify it's gone
        let retrieved = registry.get(&session_id);
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_active() {
        let registry = ProcessRegistry::new();

        // Register multiple sessions
        for i in 0..3 {
            let session_id = SessionId::new(format!("session-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg("hello")
                .spawn()
                .expect("Failed to spawn process");
            registry.register(session_id, child).expect("Failed to register");
        }

        let active = registry.list_active();
        assert_eq!(active.len(), 3);
    }

    #[tokio::test]
    async fn test_cleanup_finished() {
        let registry = ProcessRegistry::new();

        // Register a short-lived process
        let session_id = SessionId::new("short-lived");
        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");
        registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Wait for process to finish
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Clean up
        let cleaned = registry.cleanup_finished().await;
        assert!(cleaned > 0);

        // Verify it's gone
        let retrieved = registry.get(&session_id);
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_process_uptime() {
        let session_id = SessionId::new("test-session");
        let child = tokio::process::Command::new("sleep")
            .arg("0.1")
            .spawn()
            .expect("Failed to spawn process");
        let pid = child.id().expect("Failed to get PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        tokio::time::sleep(Duration::from_millis(50)).await;

        let uptime = handle.uptime().expect("Failed to get uptime");
        assert!(uptime >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_get_output_from_registry() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-session");

        let child = tokio::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("Failed to spawn process");

        let handle = registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Append some output
        handle.append_output("test output").expect("Failed to append");

        // Get output through registry
        let output = registry.get_output(&session_id);
        assert!(output.is_some());
        assert_eq!(output.unwrap(), "test output");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_graceful_kill() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-session");

        // Spawn a long-running process
        let child = tokio::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("Failed to spawn process");

        registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Gracefully kill it
        let result = registry.kill(&session_id, true).await;
        assert!(result.is_ok());

        // Verify it's unregistered
        assert!(registry.get(&session_id).is_none());
    }

    #[tokio::test]
    async fn test_force_kill() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-session");

        // Spawn a process
        let child = tokio::process::Command::new("sleep")
            .arg("10")
            .spawn()
            .expect("Failed to spawn process");

        registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Force kill it
        let result = registry.kill(&session_id, false).await;
        assert!(result.is_ok());

        // Verify it's unregistered
        assert!(registry.get(&session_id).is_none());
    }

    // ===== NEW COMPREHENSIVE TESTS =====

    /// Test concurrent registration of multiple processes from different threads
    #[tokio::test]
    async fn test_concurrent_registration() {
        use std::sync::Arc;

        let registry = Arc::new(ProcessRegistry::new());
        let mut handles = vec![];

        // Spawn 20 concurrent registration tasks
        for i in 0..20 {
            let registry = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let session_id = SessionId::new(format!("concurrent-{}", i));
                let child = tokio::process::Command::new("echo")
                    .arg(format!("test-{}", i))
                    .spawn()
                    .expect("Failed to spawn");

                registry.register(session_id.clone(), child)
                    .expect("Failed to register");

                // Also test concurrent reads
                assert!(registry.get(&session_id).is_some());
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Verify all registered
        assert_eq!(registry.list_active().len(), 20);
    }

    /// Test concurrent reads and writes
    #[tokio::test]
    async fn test_concurrent_reads_and_writes() {
        use std::sync::Arc;

        let registry = Arc::new(ProcessRegistry::new());

        // Pre-register some processes
        for i in 0..5 {
            let session_id = SessionId::new(format!("session-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg("test")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        let mut handles = vec![];

        // Spawn reader tasks
        for _ in 0..10 {
            let registry = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                for i in 0..5 {
                    let session_id = SessionId::new(format!("session-{}", i));
                    let _ = registry.get(&session_id);
                    let _ = registry.list_active();
                }
            });
            handles.push(handle);
        }

        // Spawn writer tasks
        for i in 5..10 {
            let registry = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                let session_id = SessionId::new(format!("session-{}", i));
                let child = tokio::process::Command::new("echo")
                    .arg("test")
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

        // Should have 10 registered processes
        assert_eq!(registry.list_active().len(), 10);
    }

    /// Test registry iteration and filtering
    #[tokio::test]
    async fn test_registry_iteration() {
        let registry = ProcessRegistry::new();
        let mut expected_ids = Vec::new();

        // Register processes with specific naming pattern
        for i in 0..10 {
            let session_id = SessionId::new(format!("test-{:03}", i));
            let child = tokio::process::Command::new("echo")
                .arg("test")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id.clone(), child)
                .expect("Failed to register");
            expected_ids.push(session_id);
        }

        let active = registry.list_active();
        assert_eq!(active.len(), 10);

        // Verify we can find each expected ID
        for expected_id in &expected_ids {
            assert!(active.contains(expected_id), "Missing ID: {:?}", expected_id);
        }
    }

    /// Test process lookup by PID through handle
    #[tokio::test]
    async fn test_process_lookup_by_pid() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("test-pid-lookup");

        let child = tokio::process::Command::new("echo")
            .arg("test")
            .spawn()
            .expect("Failed to spawn");
        let expected_pid = child.id().expect("No PID");

        let handle = registry.register(session_id.clone(), child)
            .expect("Failed to register");

        // Verify we can look up by session and get correct PID
        let retrieved_handle = registry.get(&session_id).expect("Handle not found");
        assert_eq!(retrieved_handle.pid, expected_pid);
        assert_eq!(handle.pid, expected_pid);
    }

    /// Test registry cleanup with mixed finished/running processes
    #[tokio::test]
    async fn test_cleanup_mixed_processes() {
        let registry = ProcessRegistry::new();

        // Register short-lived processes
        for i in 0..3 {
            let session_id = SessionId::new(format!("short-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg("quick")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        // Register longer-lived processes
        for i in 0..3 {
            let session_id = SessionId::new(format!("long-{}", i));
            let child = tokio::process::Command::new("sleep")
                .arg("0.5")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        // Wait for short-lived to finish
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Cleanup
        let cleaned = registry.cleanup_finished().await;

        // Should have cleaned up the echo processes
        assert!(cleaned >= 3, "Expected at least 3 cleaned, got {}", cleaned);

        // Should still have the sleep processes
        let remaining = registry.list_active().len();
        assert!(remaining <= 3, "Expected at most 3 remaining, got {}", remaining);
    }

    /// Test error handling for missing processes
    #[tokio::test]
    async fn test_error_missing_process() {
        let registry = ProcessRegistry::new();
        let missing_id = SessionId::new("does-not-exist");

        // Try to get non-existent process
        assert!(registry.get(&missing_id).is_none());

        // Try to get output from non-existent process
        assert!(registry.get_output(&missing_id).is_none());

        // Try to unregister non-existent process
        assert!(registry.unregister(&missing_id).is_none());

        // Try to kill non-existent process
        let result = registry.kill(&missing_id, false).await;
        assert!(result.is_err());
    }

    /// Test registry cleanup edge case: empty registry
    #[tokio::test]
    async fn test_cleanup_empty_registry() {
        let registry = ProcessRegistry::new();
        let cleaned = registry.cleanup_finished().await;
        assert_eq!(cleaned, 0);
    }

    /// Test registry cleanup edge case: all processes still running
    #[tokio::test]
    async fn test_cleanup_all_running() {
        let registry = ProcessRegistry::new();

        // Register processes that will still be running
        for i in 0..3 {
            let session_id = SessionId::new(format!("running-{}", i));
            let child = tokio::process::Command::new("sleep")
                .arg("1")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        let cleaned = registry.cleanup_finished().await;
        assert_eq!(cleaned, 0);

        // All should still be active
        assert_eq!(registry.list_active().len(), 3);
    }

    /// Test process state transitions
    #[tokio::test]
    async fn test_process_state_transitions() {
        let session_id = SessionId::new("state-test");
        let child = tokio::process::Command::new("sleep")
            .arg("0.1")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Initially running
        assert!(handle.is_running().await.expect("Failed to check status"));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should be finished
        assert!(!handle.is_running().await.expect("Failed to check status"));

        // Second check should also return false
        assert!(!handle.is_running().await.expect("Failed to check status"));
    }

    /// Test output buffer with large content
    #[tokio::test]
    async fn test_output_buffer_large_content() {
        let session_id = SessionId::new("large-output");
        let child = tokio::process::Command::new("echo")
            .arg("test")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Append large content
        let large_string = "X".repeat(10000);
        for _ in 0..10 {
            handle.append_output(&large_string).expect("Failed to append");
        }

        let output = handle.get_output().expect("Failed to get output");
        assert_eq!(output.len(), 100000);
    }

    /// Test output buffer clearing
    #[tokio::test]
    async fn test_output_buffer_clear_and_reuse() {
        let session_id = SessionId::new("clear-test");
        let child = tokio::process::Command::new("echo")
            .arg("test")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Add, clear, add pattern
        handle.append_output("first\n").expect("Failed to append");
        assert_eq!(handle.get_output().unwrap(), "first\n");

        handle.clear_output().expect("Failed to clear");
        assert_eq!(handle.get_output().unwrap(), "");

        handle.append_output("second\n").expect("Failed to append");
        assert_eq!(handle.get_output().unwrap(), "second\n");
    }

    /// Test ProcessHandle cloning
    #[tokio::test]
    async fn test_process_handle_clone() {
        let session_id = SessionId::new("clone-test");
        let child = tokio::process::Command::new("echo")
            .arg("test")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle1 = ProcessHandle::new(session_id.clone(), child, pid);
        let handle2 = handle1.clone();

        // Both should have same metadata
        assert_eq!(handle1.session_id, handle2.session_id);
        assert_eq!(handle1.pid, handle2.pid);

        // Output should be shared
        handle1.append_output("shared\n").expect("Failed to append");
        assert_eq!(handle2.get_output().unwrap(), "shared\n");
    }

    /// Test registry replacement of existing session
    #[tokio::test]
    async fn test_registry_replace_existing_session() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("replace-test");

        // Register first process
        let child1 = tokio::process::Command::new("echo")
            .arg("first")
            .spawn()
            .expect("Failed to spawn");
        let pid1 = child1.id().expect("No PID");

        let handle1 = registry.register(session_id.clone(), child1)
            .expect("Failed to register");
        assert_eq!(handle1.pid, pid1);

        // Register second process with same session ID (should replace)
        let child2 = tokio::process::Command::new("echo")
            .arg("second")
            .spawn()
            .expect("Failed to spawn");
        let pid2 = child2.id().expect("No PID");

        let handle2 = registry.register(session_id.clone(), child2)
            .expect("Failed to register");
        assert_eq!(handle2.pid, pid2);

        // Should only have one entry
        assert_eq!(registry.list_active().len(), 1);

        // Should have the second process
        let retrieved = registry.get(&session_id).expect("Not found");
        assert_eq!(retrieved.pid, pid2);
    }

    /// Test uptime calculation accuracy
    #[tokio::test]
    async fn test_uptime_accuracy() {
        let session_id = SessionId::new("uptime-test");
        let child = tokio::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Check uptime immediately
        let uptime1 = handle.uptime().expect("Failed to get uptime");
        assert!(uptime1 < Duration::from_millis(100));

        // Wait 200ms
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check uptime again
        let uptime2 = handle.uptime().expect("Failed to get uptime");
        assert!(uptime2 >= Duration::from_millis(200));
        assert!(uptime2 < Duration::from_millis(500));
    }

    /// Test thread safety with output buffer
    #[tokio::test]
    async fn test_thread_safe_output_buffer() {
        use std::sync::Arc;

        let session_id = SessionId::new("thread-safe-output");
        let child = tokio::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = Arc::new(ProcessHandle::new(session_id, child, pid));
        let mut handles = vec![];

        // Spawn multiple tasks that append concurrently
        for i in 0..50 {
            let handle = Arc::clone(&handle);
            let task = tokio::spawn(async move {
                handle.append_output(&format!("line-{}\n", i))
                    .expect("Failed to append");
            });
            handles.push(task);
        }

        // Wait for all tasks
        for task in handles {
            task.await.expect("Task panicked");
        }

        // Verify we have all 50 lines
        let output = handle.get_output().expect("Failed to get output");
        let line_count = output.lines().count();
        assert_eq!(line_count, 50);
    }

    /// Test is_running after process completes
    #[tokio::test]
    async fn test_is_running_after_completion() {
        let session_id = SessionId::new("completion-test");
        let child = tokio::process::Command::new("echo")
            .arg("done")
            .spawn()
            .expect("Failed to spawn");
        let pid = child.id().expect("No PID");

        let handle = ProcessHandle::new(session_id, child, pid);

        // Should be running initially
        assert!(handle.is_running().await.expect("Failed to check"));

        // Wait for completion
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should not be running anymore
        assert!(!handle.is_running().await.expect("Failed to check"));
    }

    /// Test registry with SessionId::generate()
    #[tokio::test]
    async fn test_registry_with_generated_ids() {
        let registry = ProcessRegistry::new();
        let mut generated_ids = Vec::new();

        // Register processes with generated IDs
        for _ in 0..5 {
            let session_id = SessionId::generate();
            let child = tokio::process::Command::new("echo")
                .arg("test")
                .spawn()
                .expect("Failed to spawn");

            registry.register(session_id.clone(), child)
                .expect("Failed to register");
            generated_ids.push(session_id);
        }

        // Verify all registered
        assert_eq!(registry.list_active().len(), 5);

        // Verify each ID is unique and retrievable
        for id in &generated_ids {
            assert!(registry.get(id).is_some());
        }
    }

    /// Test multiple cleanup cycles
    #[tokio::test]
    async fn test_multiple_cleanup_cycles() {
        let registry = ProcessRegistry::new();

        // First batch
        for i in 0..3 {
            let session_id = SessionId::new(format!("batch1-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg("test")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        let cleaned1 = registry.cleanup_finished().await;
        assert!(cleaned1 > 0);

        // Second batch
        for i in 0..3 {
            let session_id = SessionId::new(format!("batch2-{}", i));
            let child = tokio::process::Command::new("echo")
                .arg("test")
                .spawn()
                .expect("Failed to spawn");
            registry.register(session_id, child).expect("Failed to register");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        let cleaned2 = registry.cleanup_finished().await;
        assert!(cleaned2 > 0);

        // Cleanup again should return 0
        let cleaned3 = registry.cleanup_finished().await;
        assert_eq!(cleaned3, 0);
    }

    /// Test get_output returns None for unregistered session
    #[tokio::test]
    async fn test_get_output_unregistered() {
        let registry = ProcessRegistry::new();
        let session_id = SessionId::new("unregistered");

        let output = registry.get_output(&session_id);
        assert!(output.is_none());
    }
}
