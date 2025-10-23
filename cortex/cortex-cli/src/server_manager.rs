//! REST API Server Manager - Process lifecycle management

use cortex_core::error::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
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
        let home = dirs::home_dir().expect("Could not determine home directory");
        let ryht_dir = home.join(".ryht").join("cortex").join("api-server");
        
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: None,
            log_file: ryht_dir.join("logs").join("api-server.log"),
            pid_file: ryht_dir.join("api-server.pid"),
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
    config: ServerConfig,
}

impl ServerManager {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }
    
    /// Start the REST API server in background
    pub async fn start(&self) -> Result<()> {
        // Check if already running
        if self.is_running().await {
            return Err(CortexError::config("REST API server is already running"));
        }
        
        // Create log directory
        if let Some(parent) = self.config.log_file.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Create PID directory  
        if let Some(parent) = self.config.pid_file.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Get current executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| CortexError::config(format!("Failed to get executable path: {}", e)))?;
        
        info!("Starting REST API server on {}:{}", self.config.host, self.config.port);
        
        // Build command - use hidden internal-server-run command
        let mut cmd = Command::new(&exe_path);
        cmd.arg("internal-server-run")
            .arg("--host")
            .arg(&self.config.host)
            .arg("--port")
            .arg(self.config.port.to_string());

        if let Some(workers) = self.config.workers {
            cmd.arg("--workers").arg(workers.to_string());
        }
        
        // Redirect stdout/stderr to log file
        // We need to use std::fs::File for Stdio, not tokio::fs::File
        let log_file_std = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)?;

        let log_file_std_clone = log_file_std.try_clone()?;

        cmd.stdout(Stdio::from(log_file_std));
        cmd.stderr(Stdio::from(log_file_std_clone));
        cmd.kill_on_drop(false); // Don't kill on drop
        
        // Spawn the process
        let child = cmd.spawn()
            .map_err(|e| CortexError::config(format!("Failed to start server: {}", e)))?;
        
        let pid = child.id()
            .ok_or_else(|| CortexError::config("Failed to get process ID"))?;
        
        info!("REST API server started with PID: {}", pid);
        
        // Write PID file
        let mut pid_file = fs::File::create(&self.config.pid_file).await?;
        pid_file.write_all(pid.to_string().as_bytes()).await?;
        pid_file.flush().await?;
        
        // Wait for server to be ready
        self.wait_for_ready().await?;
        
        Ok(())
    }
    
    /// Stop the REST API server
    pub async fn stop(&self) -> Result<()> {
        if !self.is_running().await {
            return Err(CortexError::config("REST API server is not running"));
        }
        
        let pid = self.read_pid().await?;
        
        info!("Stopping REST API server (PID: {})", pid);
        
        // Send SIGTERM
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .map_err(|e| CortexError::config(format!("Failed to stop server: {}", e)))?;
        }
        
        #[cfg(not(unix))]
        {
            return Err(CortexError::config("Server stop is only supported on Unix systems"));
        }
        
        // Wait for process to exit
        for _ in 0..30 {
            sleep(Duration::from_millis(100)).await;
            if !self.is_running().await {
                // Remove PID file
                let _ = fs::remove_file(&self.config.pid_file).await;
                info!("REST API server stopped");
                return Ok(());
            }
        }
        
        warn!("Server did not stop gracefully, forcing kill");
        
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            kill(Pid::from_raw(pid as i32), Signal::SIGKILL)
                .map_err(|e| CortexError::config(format!("Failed to kill server: {}", e)))?;
        }
        
        let _ = fs::remove_file(&self.config.pid_file).await;
        Ok(())
    }
    
    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        if let Ok(pid) = self.read_pid().await {
            self.is_process_running(pid)
        } else {
            false
        }
    }
    
    /// Get server status
    pub async fn status(&self) -> ServerStatus {
        if self.is_running().await {
            ServerStatus::Running
        } else {
            ServerStatus::Stopped
        }
    }
    
    /// Read PID from file
    async fn read_pid(&self) -> Result<u32> {
        let content = fs::read_to_string(&self.config.pid_file).await
            .map_err(|_| CortexError::config("PID file not found"))?;
        
        content.trim().parse::<u32>()
            .map_err(|e| CortexError::config(format!("Invalid PID in file: {}", e)))
    }
    
    /// Check if process is running
    fn is_process_running(&self, pid: u32) -> bool {
        #[cfg(unix)]
        {
            use nix::sys::signal::kill;
            use nix::unistd::Pid;

            // Signal 0 (None) is a special case - it doesn't send a signal but checks if process exists
            kill(Pid::from_raw(pid as i32), None).is_ok()
        }

        #[cfg(not(unix))]
        {
            false
        }
    }
    
    /// Wait for server to be ready
    async fn wait_for_ready(&self) -> Result<()> {
        let url = format!("http://{}:{}/health", self.config.host, self.config.port);
        let client = reqwest::Client::new();
        
        for attempt in 1..=30 {
            sleep(Duration::from_millis(500)).await;
            
            if let Ok(response) = client.get(&url).send().await {
                if response.status().is_success() {
                    info!("REST API server is ready after {} attempts", attempt);
                    return Ok(());
                }
            }
        }
        
        Err(CortexError::config("Server failed to become ready within timeout"))
    }
    
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}
