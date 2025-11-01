//! Cortex HTTP Server Launcher
//!
//! Automatically starts Cortex HTTP server if not already running.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Cortex launcher for managing HTTP server lifecycle
pub struct CortexLauncher {
    /// Path to cortex binary
    cortex_binary: PathBuf,
    /// HTTP server address
    address: String,
    /// HTTP server port
    port: u16,
    /// Child process handle
    process: Option<Child>,
}

impl CortexLauncher {
    /// Create a new Cortex launcher
    pub fn new(cortex_binary: Option<PathBuf>, address: String, port: u16) -> Result<Self> {
        let cortex_binary = if let Some(binary) = cortex_binary {
            binary
        } else {
            // Try to find cortex binary
            Self::find_cortex_binary()?
        };

        Ok(Self {
            cortex_binary,
            address,
            port,
            process: None,
        })
    }

    /// Find cortex binary in common locations
    fn find_cortex_binary() -> Result<PathBuf> {
        // Check environment variable
        if let Ok(cortex_path) = std::env::var("CORTEX_BINARY") {
            let path = PathBuf::from(cortex_path);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check in current directory dist/
        let current_dir = std::env::current_dir()?;
        let dist_cortex = current_dir.join("dist/cortex");
        if dist_cortex.exists() {
            return Ok(dist_cortex);
        }

        // Check in parent directory dist/
        if let Some(parent) = current_dir.parent() {
            let parent_dist_cortex = parent.join("dist/cortex");
            if parent_dist_cortex.exists() {
                return Ok(parent_dist_cortex);
            }
        }

        // Check in PATH
        if let Ok(output) = Command::new("which").arg("cortex").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let path = PathBuf::from(path_str.trim());
                if path.exists() {
                    return Ok(path);
                }
            }
        }

        anyhow::bail!("Could not find cortex binary. Set CORTEX_BINARY environment variable or ensure cortex is in PATH or dist/ directory")
    }

    /// Check if Cortex HTTP server is already running
    pub async fn is_running(&self) -> bool {
        let url = format!("http://{}:{}/api/v1/health", self.address, self.port);

        debug!("Checking Cortex health at: {}", url);

        match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Cortex HTTP server is already running");
                    true
                } else {
                    debug!("Cortex health check failed with status: {}", response.status());
                    false
                }
            }
            Err(e) => {
                debug!("Cortex health check failed: {}", e);
                false
            }
        }
    }

    /// Start Cortex HTTP server in background
    pub async fn start(&mut self) -> Result<()> {
        // Check if already running
        if self.is_running().await {
            info!("Cortex HTTP server is already running");
            return Ok(());
        }

        info!("Starting Cortex HTTP server at {}:{}", self.address, self.port);

        // Get log file path
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let log_dir = PathBuf::from(&home).join(".axon/logs");
        std::fs::create_dir_all(&log_dir)?;
        let log_file = log_dir.join("cortex-http.log");

        // Open log file
        let log_file_handle = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .context("Failed to open cortex log file")?;

        // Start cortex HTTP server
        let child = Command::new(&self.cortex_binary)
            .args(&[
                "server",
                "start",
                "--host",
                &self.address,
                "--port",
                &self.port.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file_handle.try_clone()?))
            .stderr(Stdio::from(log_file_handle))
            .spawn()
            .context("Failed to start cortex HTTP server")?;

        info!("Cortex HTTP server process started with PID: {}", child.id());
        info!("Cortex logs: {}", log_file.display());

        self.process = Some(child);

        // Wait for server to be ready with exponential backoff
        let max_attempts = 60; // Увеличено до 60 попыток (до 30 секунд)
        let mut attempts = 0;
        let mut wait_ms = 500;

        while attempts < max_attempts {
            sleep(Duration::from_millis(wait_ms)).await;

            if self.is_running().await {
                info!("Cortex HTTP server is ready after {} attempts!", attempts + 1);
                return Ok(());
            }

            attempts += 1;

            // Exponential backoff: 500ms, 500ms, 750ms, 1000ms, 1000ms...
            if attempts == 2 {
                wait_ms = 750;
            } else if attempts >= 3 {
                wait_ms = 1000;
            }

            if attempts % 5 == 0 {
                info!("Still waiting for Cortex to start... ({}/{} attempts)", attempts, max_attempts);
            } else {
                debug!("Waiting for Cortex to start... ({}/{})", attempts, max_attempts);
            }
        }

        // If we get here, server didn't start
        warn!("Cortex HTTP server failed to start within timeout after {} attempts", max_attempts);
        self.stop();
        anyhow::bail!("Cortex HTTP server failed to start within 30 seconds timeout")
    }

    /// Stop Cortex HTTP server
    pub fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            info!("Stopping Cortex HTTP server (PID: {})", process.id());

            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(process.id() as i32);
                if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to cortex: {}", e);
                }

                // Wait a bit for graceful shutdown
                std::thread::sleep(Duration::from_secs(2));
            }

            // Force kill if still running
            if let Err(e) = process.kill() {
                warn!("Failed to kill cortex process: {}", e);
            }

            // Wait for process to exit
            if let Err(e) = process.wait() {
                warn!("Failed to wait for cortex process: {}", e);
            }
        }
    }

    /// Get the HTTP server URL
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }
}

impl Drop for CortexLauncher {
    fn drop(&mut self) {
        // Don't stop the server on drop - let it keep running
        // This allows other axon instances to use the same server
        if self.process.is_some() {
            debug!("CortexLauncher dropped but leaving server running");
            // Detach the process so it continues running
            let _ = self.process.take();
        }
    }
}
