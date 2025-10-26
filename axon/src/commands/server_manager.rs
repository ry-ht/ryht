//! REST API Server Manager - Process lifecycle management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Configuration for REST API server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Number of worker threads
    pub workers: Option<usize>,

    /// Log file path
    pub log_file: PathBuf,

    /// PID file path
    pub pid_file: PathBuf,

    /// Startup timeout in seconds
    pub startup_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| {
            tracing::warn!("Could not determine home directory, using /tmp as fallback");
            PathBuf::from("/tmp")
        });
        let axon_dir = home.join(".ryht").join("axon").join("api-server");

        Self {
            host: "127.0.0.1".to_string(),
            port: 9090,
            workers: None,
            log_file: axon_dir.join("logs").join("api-server.log"),
            pid_file: axon_dir.join("api-server.pid"),
            startup_timeout_secs: 30,
        }
    }
}

/// Server status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Running,
    Stopped,
    Unknown,
}

pub struct ServerManager {
    config: Option<ServerConfig>,
    pid: Arc<RwLock<Option<u32>>>,
}

impl ServerManager {
    /// Create a new server manager with config
    pub fn with_config(config: ServerConfig) -> Self {
        Self {
            config: Some(config),
            pid: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new server manager without config (for CLI use)
    pub fn new() -> Self {
        Self {
            config: None,
            pid: Arc::new(RwLock::new(None)),
        }
    }

    /// Store the server PID (for CLI use)
    pub async fn set_server_pid(&self, pid: u32) {
        let mut guard = self.pid.write().await;
        *guard = Some(pid);
    }

    /// Get the stored server PID (for CLI use)
    pub async fn get_server_pid(&self) -> Option<u32> {
        *self.pid.read().await
    }

    /// Clear the stored server PID (for CLI use)
    pub async fn clear_server_pid(&self) {
        let mut guard = self.pid.write().await;
        *guard = None;
    }

    /// Start the REST API server in background
    pub async fn start(&self) -> Result<()> {
        let config = self.config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ServerManager requires config for start operation"))?;

        // Check if already running
        if self.is_running().await {
            return Err(anyhow::anyhow!("REST API server is already running"));
        }

        // Create log directory
        if let Some(parent) = config.log_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Create PID directory
        if let Some(parent) = config.pid_file.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Get current executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("Failed to get executable path: {}", e))?;

        info!("Starting REST API server on {}:{}", config.host, config.port);

        // Build command - use hidden internal-server-run command
        let mut cmd = Command::new(&exe_path);
        cmd.arg("internal-server-run")
            .arg("--host")
            .arg(&config.host)
            .arg("--port")
            .arg(config.port.to_string());

        if let Some(workers) = config.workers {
            cmd.arg("--workers").arg(workers.to_string());
        }

        // Redirect stdout/stderr to log file
        let log_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_file)
            .await?;

        cmd.stdout(Stdio::from(log_file.try_clone().await?));
        cmd.stderr(Stdio::from(log_file));
        cmd.stdin(Stdio::null());

        // Spawn the process
        let child = cmd.spawn()
            .context("Failed to spawn server process")?;

        let pid = child.id()
            .ok_or_else(|| anyhow::anyhow!("Failed to get process ID"))?;

        // Write PID file
        let mut pid_file = fs::File::create(&config.pid_file).await?;
        pid_file.write_all(pid.to_string().as_bytes()).await?;

        info!("Server process started with PID: {}", pid);

        // Wait for server to be ready
        let timeout = Duration::from_secs(config.startup_timeout_secs);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            if self.check_health().await {
                info!("Server is ready");
                return Ok(());
            }
            sleep(Duration::from_millis(500)).await;
        }

        warn!("Server started but health check timed out");
        Ok(())
    }

    /// Stop the REST API server
    pub async fn stop(&self) -> Result<()> {
        let config = self.config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ServerManager requires config for stop operation"))?;

        if !self.is_running().await {
            return Err(anyhow::anyhow!("REST API server is not running"));
        }

        // Read PID
        let pid_str = fs::read_to_string(&config.pid_file).await
            .context("Failed to read PID file")?;
        let pid: u32 = pid_str.trim().parse()
            .context("Invalid PID in file")?;

        info!("Stopping server with PID: {}", pid);

        // Send SIGTERM on Unix, TerminateProcess on Windows
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .context("Failed to send SIGTERM")?;
        }

        #[cfg(windows)]
        {
            // On Windows, use taskkill
            let output = std::process::Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("Failed to kill process"));
            }
        }

        // Wait for process to terminate
        for _ in 0..20 {
            if !self.is_running().await {
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }

        // Remove PID file
        if config.pid_file.exists() {
            fs::remove_file(&config.pid_file).await
                .context("Failed to remove PID file")?;
        }

        info!("Server stopped successfully");
        Ok(())
    }

    /// Check server status
    pub async fn status(&self) -> Result<ServerStatus> {
        if self.is_running().await {
            Ok(ServerStatus::Running)
        } else {
            Ok(ServerStatus::Stopped)
        }
    }

    /// Check if server is running
    async fn is_running(&self) -> bool {
        // If we don't have config, check using the simple PID tracking
        let config = match self.config.as_ref() {
            Some(c) => c,
            None => {
                // Use simple PID tracking
                return self.get_server_pid().await.is_some();
            }
        };

        // Check if PID file exists
        if !config.pid_file.exists() {
            return false;
        }

        // Read PID and check if process is alive
        if let Ok(pid_str) = fs::read_to_string(&config.pid_file).await {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                return self.is_process_alive(pid);
            }
        }

        false
    }

    /// Check if process is alive
    fn is_process_alive(&self, pid: u32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            // Send signal 0 to check if process exists (null signal doesn't kill the process)
            match kill(Pid::from_raw(pid as i32), None) {
                Ok(_) => true,
                Err(_) => false,
            }
        }

        #[cfg(windows)]
        {
            // On Windows, check if process exists using tasklist
            if let Ok(output) = std::process::Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid)])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains(&pid.to_string())
            } else {
                false
            }
        }
    }

    /// Check server health via HTTP
    async fn check_health(&self) -> bool {
        let config = match self.config.as_ref() {
            Some(c) => c,
            None => {
                // Default to localhost:3000 if no config
                let url = "http://127.0.0.1:3000/health";
                return reqwest::get(url)
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);
            }
        };

        let url = format!("http://{}:{}/health", config.host, config.port);

        reqwest::get(&url)
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(unix)]
use nix;
