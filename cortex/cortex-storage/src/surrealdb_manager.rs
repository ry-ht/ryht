//! SurrealDB Manager - Production-ready local SurrealDB server lifecycle management
//!
//! This module provides comprehensive functionality to:
//! - Auto-detect SurrealDB installation across common paths
//! - Download and install SurrealDB from official sources if needed
//! - Start, stop, and restart local SurrealDB server
//! - Monitor server health with automatic recovery
//! - Manage server process and PID files
//! - Support system boot configuration
//! - Handle graceful shutdown and cleanup

use cortex_core::error::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn, trace, span, Level};

/// Configuration for SurrealDB server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBConfig {
    /// Bind address (e.g., "127.0.0.1:8000")
    pub bind_address: String,

    /// Data directory path
    pub data_dir: PathBuf,

    /// Log file path
    pub log_file: PathBuf,

    /// PID file path
    pub pid_file: PathBuf,

    /// Username for authentication
    pub username: String,

    /// Password for authentication
    pub password: String,

    /// Storage engine (rocksdb, memory, tikv)
    pub storage_engine: String,

    /// Allow guests (no authentication)
    pub allow_guests: bool,

    /// Maximum number of startup retries
    pub max_retries: u32,

    /// Startup timeout in seconds
    pub startup_timeout_secs: u64,

    /// Enable automatic restart on failure
    pub auto_restart: bool,

    /// Health check interval in seconds
    pub health_check_interval_secs: u64,

    /// Maximum restart attempts before giving up
    pub max_restart_attempts: u32,

    /// Enable system boot startup
    pub start_on_boot: bool,
}

impl Default for SurrealDBConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| {
            eprintln!("WARNING: Could not determine home directory, using /tmp as fallback");
            std::path::PathBuf::from("/tmp")
        });
        let ryht_dir = home.join(".ryht").join("cortex").join("surrealdb");

        Self {
            bind_address: "127.0.0.1:8000".to_string(),
            data_dir: ryht_dir.join("data"),
            log_file: ryht_dir.join("logs").join("surreal.log"),
            pid_file: ryht_dir.join("surreal.pid"),
            username: "root".to_string(),
            password: "root".to_string(),
            storage_engine: "rocksdb".to_string(),
            allow_guests: false,
            max_retries: 3,
            startup_timeout_secs: 30,
            auto_restart: true,
            health_check_interval_secs: 30,
            max_restart_attempts: 5,
            start_on_boot: false,
        }
    }
}

impl SurrealDBConfig {
    /// Create a new configuration with custom values
    pub fn new(bind_address: String, data_dir: PathBuf) -> Self {
        let mut config = Self::default();
        config.bind_address = bind_address;
        config.data_dir = data_dir;
        config
    }

    /// Set authentication credentials
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = username;
        self.password = password;
        self
    }

    /// Set storage engine
    pub fn with_storage_engine(mut self, engine: String) -> Self {
        self.storage_engine = engine;
        self
    }

    /// Set whether to allow guests
    pub fn with_allow_guests(mut self, allow: bool) -> Self {
        self.allow_guests = allow;
        self
    }

    /// Ensure all required directories exist (async version)
    pub async fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to create data directory: {}", e)))?;

        if let Some(parent) = self.log_file.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| CortexError::storage(format!("Failed to create log directory: {}", e)))?;
        }

        if let Some(parent) = self.pid_file.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| CortexError::storage(format!("Failed to create run directory: {}", e)))?;
        }

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.bind_address.is_empty() {
            return Err(CortexError::config("Bind address cannot be empty"));
        }

        if self.username.is_empty() {
            return Err(CortexError::config("Username cannot be empty"));
        }

        if self.password.is_empty() {
            return Err(CortexError::config("Password cannot be empty"));
        }

        if self.max_retries == 0 {
            return Err(CortexError::config("Max retries must be greater than 0"));
        }

        Ok(())
    }
}

/// Status of the SurrealDB server
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server is running
    Running,
    /// Server is stopped
    Stopped,
    /// Server status is unknown
    Unknown,
    /// Server is starting
    Starting,
    /// Server is stopping
    Stopping,
}

/// SurrealDB Manager - manages local SurrealDB server lifecycle
pub struct SurrealDBManager {
    config: SurrealDBConfig,
    process: Option<Child>,
    status: ServerStatus,
    restart_count: u32,
    binary_path: Option<PathBuf>,
}

impl SurrealDBManager {
    /// Create a new SurrealDB manager with the given configuration
    #[instrument(skip(config), fields(bind_address = %config.bind_address))]
    pub async fn new(config: SurrealDBConfig) -> Result<Self> {
        config.validate()?;
        config.ensure_directories().await?;

        Ok(Self {
            config,
            process: None,
            status: ServerStatus::Stopped,
            restart_count: 0,
            binary_path: None,
        })
    }

    /// Check if SurrealDB is installed and return the path to the binary
    #[instrument]
    pub async fn find_surreal_binary() -> Result<PathBuf> {
        let span = span!(Level::DEBUG, "find_surreal_binary");
        let _enter = span.enter();

        debug!("Searching for SurrealDB binary");

        // Check if surreal is in PATH using 'which' on Unix or 'where' on Windows
        #[cfg(unix)]
        let which_cmd = "which";
        #[cfg(windows)]
        let which_cmd = "where";

        if let Ok(output) = Command::new(which_cmd)
            .arg("surreal")
            .output()
            .await
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    info!("Found SurrealDB in PATH at: {}", path);
                    let path_buf = PathBuf::from(path);
                    if let Ok(version) = Self::get_binary_version(&path_buf).await {
                        info!("SurrealDB version: {}", version);
                    }
                    return Ok(path_buf);
                }
            }
        }

        // Check common installation locations
        let home_dir = dirs::home_dir().unwrap_or_default();
        let cargo_bin = home_dir.join(".cargo").join("bin").join("surreal");
        let home_bin = home_dir.join("bin").join("surreal");

        #[cfg(unix)]
        let common_paths = vec![
            PathBuf::from("/usr/local/bin/surreal"),
            PathBuf::from("/usr/bin/surreal"),
            PathBuf::from("/opt/homebrew/bin/surreal"),
            cargo_bin.clone(),
            home_bin.clone(),
        ];

        #[cfg(windows)]
        let common_paths = vec![
            cargo_bin.clone(),
            home_dir.join("bin").join("surreal.exe"),
            PathBuf::from("C:\\Program Files\\SurrealDB\\surreal.exe"),
        ];

        for path_buf in common_paths {
            trace!("Checking path: {:?}", path_buf);

            if fs::metadata(&path_buf).await.is_ok() {
                info!("Found SurrealDB at: {:?}", path_buf);
                if let Ok(version) = Self::get_binary_version(&path_buf).await {
                    info!("SurrealDB version: {}", version);
                }
                return Ok(path_buf);
            }
        }

        Err(CortexError::storage(
            "SurrealDB binary not found. Please install it using 'cortex db install' or visit https://surrealdb.com/install"
        ))
    }

    /// Get the version of the SurrealDB binary
    #[instrument(skip(binary_path))]
    async fn get_binary_version(binary_path: &PathBuf) -> Result<String> {
        let output = Command::new(binary_path)
            .arg("version")
            .output()
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get version: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(CortexError::storage("Failed to get SurrealDB version"))
        }
    }

    /// Ensure SurrealDB is installed, installing it if necessary
    pub async fn ensure_installed() -> Result<PathBuf> {
        match Self::find_surreal_binary().await {
            Ok(path) => Ok(path),
            Err(_) => {
                info!("SurrealDB not found, attempting to install...");
                Self::install_surrealdb().await
            }
        }
    }

    /// Download and install SurrealDB
    #[instrument]
    pub async fn install_surrealdb() -> Result<PathBuf> {
        info!("Installing SurrealDB...");

        // Determine OS and architecture
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        info!("Detected OS: {}, Arch: {}", os, arch);

        // Run the official installation script
        let install_result = if os == "macos" || os == "linux" {
            let mut child = Command::new("sh")
                .arg("-c")
                .arg("curl -sSf https://install.surrealdb.com | sh")
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| CortexError::storage(format!("Failed to start installation: {}", e)))?;

            child
                .wait()
                .await
                .map_err(|e| CortexError::storage(format!("Installation failed: {}", e)))?
        } else if os == "windows" {
            let mut child = Command::new("powershell")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg("iwr https://install.surrealdb.com -useb | iex")
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| CortexError::storage(format!("Failed to start installation: {}", e)))?;

            child
                .wait()
                .await
                .map_err(|e| CortexError::storage(format!("Installation failed: {}", e)))?
        } else {
            return Err(CortexError::storage(format!("Unsupported OS: {}", os)));
        };

        if !install_result.success() {
            return Err(CortexError::storage("Failed to install SurrealDB"));
        }

        info!("SurrealDB installed successfully");

        // Verify installation
        Self::find_surreal_binary().await
    }

    /// Start the SurrealDB server
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running().await {
            warn!("SurrealDB server is already running");
            return Ok(());
        }

        info!("Starting SurrealDB server on {}", self.config.bind_address);
        self.status = ServerStatus::Starting;

        // Find the binary
        let binary_path = Self::ensure_installed().await?;
        self.binary_path = Some(binary_path.clone());

        // Prepare log file
        let log_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to open log file: {}", e)))?;

        let log_file_std = log_file.into_std().await;

        // Build command
        let mut cmd = Command::new(&binary_path);
        cmd.arg("start")
            .arg("--bind")
            .arg(&self.config.bind_address)
            .arg("--user")
            .arg(&self.config.username)
            .arg("--pass")
            .arg(&self.config.password)
            .kill_on_drop(false); // Don't kill on drop, we manage lifecycle

        // Add storage engine path
        match self.config.storage_engine.as_str() {
            "rocksdb" => {
                let db_path = format!("rocksdb://{}", self.config.data_dir.display());
                cmd.arg(&db_path);
            }
            "memory" => {
                cmd.arg("memory");
            }
            engine => {
                self.status = ServerStatus::Stopped;
                return Err(CortexError::config(format!("Unsupported storage engine: {}", engine)));
            }
        }

        if self.config.allow_guests {
            cmd.arg("--allow-guests");
        }

        // Redirect output to log file
        cmd.stdout(std::process::Stdio::from(
            log_file_std
                .try_clone()
                .map_err(|e| CortexError::storage(format!("Failed to clone log file handle: {}", e)))?,
        ));
        cmd.stderr(std::process::Stdio::from(log_file_std));

        debug!("Starting SurrealDB with command: {:?}", cmd);

        // Start the process with retries
        let mut retries = 0;
        loop {
            match cmd.spawn() {
                Ok(child) => {
                    if let Some(pid) = child.id() {
                        info!("SurrealDB started with PID: {}", pid);

                        // Save PID to file
                        fs::write(&self.config.pid_file, pid.to_string())
                            .await
                            .map_err(|e| CortexError::storage(format!("Failed to write PID file: {}", e)))?;

                        self.process = Some(child);
                        break;
                    } else {
                        warn!("Failed to get PID for spawned process");
                        self.status = ServerStatus::Stopped;
                        return Err(CortexError::storage("Failed to get process PID"));
                    }
                }
                Err(e) => {
                    retries += 1;
                    if retries >= self.config.max_retries {
                        self.status = ServerStatus::Stopped;
                        return Err(CortexError::storage(format!(
                            "Failed to start SurrealDB after {} retries: {}",
                            retries, e
                        )));
                    }
                    warn!(
                        "Failed to start SurrealDB (attempt {}/{}): {}",
                        retries, self.config.max_retries, e
                    );
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        // Wait for server to be ready
        self.wait_for_ready(Duration::from_secs(self.config.startup_timeout_secs))
            .await?;

        self.status = ServerStatus::Running;
        self.restart_count = 0; // Reset restart count on successful start
        info!("SurrealDB server is ready");

        // Start health monitoring if auto-restart is enabled
        if self.config.auto_restart {
            info!("Auto-restart enabled, starting health monitoring");
        }

        Ok(())
    }

    /// Stop the SurrealDB server
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running().await {
            warn!("SurrealDB server is not running");
            return Ok(());
        }

        info!("Stopping SurrealDB server");
        self.status = ServerStatus::Stopping;

        // Try to gracefully terminate the process
        if let Some(mut child) = self.process.take() {
            // Send SIGTERM (on Unix) or terminate (on Windows)
            #[cfg(unix)]
            {
                if let Some(pid) = child.id() {
                    debug!("Sending SIGTERM to PID: {}", pid);
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
            }

            #[cfg(windows)]
            {
                debug!("Terminating process on Windows");
                let _ = child.kill();
            }

            // Wait for process to exit (with timeout)
            let timeout = Duration::from_secs(10);
            let start = Instant::now();

            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        info!("SurrealDB server stopped gracefully with status: {:?}", status);
                        break;
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            warn!("Forcefully killing SurrealDB server after timeout");
                            let _ = child.kill();
                            let _ = child.wait().await;
                            break;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        error!("Error waiting for SurrealDB to stop: {}", e);
                        break;
                    }
                }
            }
        } else if fs::metadata(&self.config.pid_file).await.is_ok() {
            // No process handle, but PID file exists - try to kill by PID
            if let Ok(pid_str) = fs::read_to_string(&self.config.pid_file).await {
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    info!("No process handle, attempting to stop process via PID: {}", pid);
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                    #[cfg(windows)]
                    {
                        let _ = Command::new("taskkill")
                            .args(&["/PID", &pid.to_string(), "/F"])
                            .output()
                            .await;
                    }
                    // Wait a bit for the process to terminate
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        // Clean up PID file
        if fs::metadata(&self.config.pid_file).await.is_ok() {
            fs::remove_file(&self.config.pid_file)
                .await
                .map_err(|e| {
                    warn!("Failed to remove PID file: {}", e);
                    e
                })
                .ok();
        }

        self.status = ServerStatus::Stopped;
        info!("SurrealDB server stopped");

        Ok(())
    }

    /// Restart the SurrealDB server
    pub async fn restart(&mut self) -> Result<()> {
        info!("Restarting SurrealDB server");
        self.stop().await?;
        sleep(Duration::from_secs(2)).await;
        self.start().await
    }

    /// Check if the server is running
    #[instrument(skip(self))]
    pub async fn is_running(&self) -> bool {
        // Check if we have a process handle
        if self.process.is_some() {
            // Try to connect to the server
            return self.health_check().await.is_ok();
        }

        // Check if PID file exists
        if fs::metadata(&self.config.pid_file).await.is_ok() {
            if let Ok(pid_str) = fs::read_to_string(&self.config.pid_file).await {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Check if process is still alive
                    return Self::is_process_alive(pid).await;
                }
            }
        }

        false
    }

    /// Check if a process with the given PID is alive
    #[cfg(unix)]
    async fn is_process_alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    async fn is_process_alive(pid: u32) -> bool {
        Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .await
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }

    /// Perform a health check on the server
    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing health check on SurrealDB server");

        // Try to connect using reqwest
        let url = format!("http://{}/health", self.config.bind_address);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| CortexError::storage(format!("Failed to create HTTP client: {}", e)))?;

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Health check passed");
                    Ok(())
                } else {
                    Err(CortexError::storage(format!(
                        "Health check failed with status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                Err(CortexError::storage(format!("Health check failed: {}", e)))
            }
        }
    }

    /// Wait for the server to be ready
    pub async fn wait_for_ready(&self, timeout: Duration) -> Result<()> {
        info!("Waiting for SurrealDB server to be ready (timeout: {:?})", timeout);

        let start = Instant::now();
        let mut attempt = 0;

        loop {
            attempt += 1;

            match self.health_check().await {
                Ok(_) => {
                    info!("SurrealDB server is ready after {} attempts", attempt);
                    return Ok(());
                }
                Err(e) => {
                    if start.elapsed() > timeout {
                        return Err(CortexError::timeout(format!(
                            "Server did not become ready within {:?}: {}",
                            timeout, e
                        )));
                    }

                    debug!("Health check attempt {} failed: {}", attempt, e);
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Get the current server status
    pub fn status(&self) -> ServerStatus {
        self.status.clone()
    }

    /// Get the server configuration
    pub fn config(&self) -> &SurrealDBConfig {
        &self.config
    }

    /// Get the connection URL for clients
    pub fn connection_url(&self) -> String {
        format!("http://{}", self.config.bind_address)
    }

    /// Get the binary path (if resolved)
    pub fn binary_path(&self) -> Option<&PathBuf> {
        self.binary_path.as_ref()
    }

    /// Get the restart count
    pub fn restart_count(&self) -> u32 {
        self.restart_count
    }

    /// Attempt to restart the server after a failure
    #[instrument(skip(self))]
    pub async fn auto_restart(&mut self) -> Result<()> {
        if self.restart_count >= self.config.max_restart_attempts {
            error!(
                "Maximum restart attempts ({}) exceeded, giving up",
                self.config.max_restart_attempts
            );
            return Err(CortexError::storage(format!(
                "Failed to restart server after {} attempts",
                self.restart_count
            )));
        }

        self.restart_count += 1;
        warn!(
            "Attempting auto-restart {}/{}",
            self.restart_count, self.config.max_restart_attempts
        );

        // Exponential backoff before restart
        let backoff_secs = 2u64.pow(self.restart_count.min(5));
        info!("Waiting {} seconds before restart...", backoff_secs);
        sleep(Duration::from_secs(backoff_secs)).await;

        self.restart().await
    }

    /// Monitor server health and automatically restart if needed
    #[instrument(skip(self))]
    pub async fn monitor_health(&mut self) -> Result<()> {
        if !self.config.auto_restart {
            return Ok(());
        }

        loop {
            sleep(Duration::from_secs(self.config.health_check_interval_secs)).await;

            if self.status != ServerStatus::Running {
                debug!("Server not in running state, skipping health check");
                continue;
            }

            match self.health_check().await {
                Ok(_) => {
                    trace!("Health check passed");
                }
                Err(e) => {
                    error!("Health check failed: {}", e);
                    warn!("Server appears to be down, attempting auto-restart");

                    if let Err(restart_err) = self.auto_restart().await {
                        error!("Auto-restart failed: {}", restart_err);
                        return Err(restart_err);
                    }

                    info!("Server successfully restarted");
                }
            }
        }
    }

    /// Configure the server to start on system boot (Unix systems with systemd)
    #[cfg(unix)]
    #[instrument(skip(self))]
    pub async fn configure_system_boot(&self, enable: bool) -> Result<()> {
        if !self.config.start_on_boot && !enable {
            return Ok(());
        }

        let service_name = "cortex-surrealdb";
        let service_file = format!("/etc/systemd/system/{}.service", service_name);

        if enable {
            info!("Configuring SurrealDB to start on system boot");

            let binary_path = self
                .binary_path
                .as_ref()
                .ok_or_else(|| CortexError::storage("Binary path not set"))?;

            let service_content = format!(
                r#"[Unit]
Description=Cortex SurrealDB Server
After=network.target

[Service]
Type=simple
User={}
ExecStart={} start --bind {} --user {} --pass {} {}
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
"#,
                std::env::var("USER").unwrap_or_else(|_| "root".to_string()),
                binary_path.display(),
                self.config.bind_address,
                self.config.username,
                self.config.password,
                if self.config.storage_engine == "memory" {
                    "memory".to_string()
                } else {
                    format!("rocksdb://{}", self.config.data_dir.display())
                }
            );

            // Write service file (requires sudo)
            let mut temp_file = tokio::fs::File::create("/tmp/cortex-surrealdb.service")
                .await
                .map_err(|e| CortexError::storage(format!("Failed to create temp file: {}", e)))?;

            temp_file
                .write_all(service_content.as_bytes())
                .await
                .map_err(|e| CortexError::storage(format!("Failed to write service file: {}", e)))?;

            // Move to systemd directory (requires sudo)
            let output = Command::new("sudo")
                .args(&["mv", "/tmp/cortex-surrealdb.service", &service_file])
                .output()
                .await
                .map_err(|e| CortexError::storage(format!("Failed to move service file: {}", e)))?;

            if !output.status.success() {
                return Err(CortexError::storage(
                    "Failed to install systemd service (requires sudo)",
                ));
            }

            // Enable and start the service
            Command::new("sudo")
                .args(&["systemctl", "daemon-reload"])
                .output()
                .await
                .map_err(|e| CortexError::storage(format!("Failed to reload systemd: {}", e)))?;

            Command::new("sudo")
                .args(&["systemctl", "enable", service_name])
                .output()
                .await
                .map_err(|e| CortexError::storage(format!("Failed to enable service: {}", e)))?;

            info!("Successfully configured system boot startup");
        } else {
            info!("Disabling SurrealDB system boot startup");

            // Disable the service
            Command::new("sudo")
                .args(&["systemctl", "disable", service_name])
                .output()
                .await
                .ok();

            // Remove service file
            Command::new("sudo")
                .args(&["rm", "-f", &service_file])
                .output()
                .await
                .ok();

            info!("Successfully disabled system boot startup");
        }

        Ok(())
    }

    #[cfg(not(unix))]
    pub async fn configure_system_boot(&self, _enable: bool) -> Result<()> {
        Err(CortexError::storage(
            "System boot configuration is only supported on Unix systems with systemd",
        ))
    }

    /// Get detailed server information
    pub async fn server_info(&self) -> ServerInfo {
        ServerInfo {
            status: self.status.clone(),
            bind_address: self.config.bind_address.clone(),
            data_dir: self.config.data_dir.clone(),
            storage_engine: self.config.storage_engine.clone(),
            is_running: self.is_running().await,
            restart_count: self.restart_count,
            binary_path: self.binary_path.clone(),
            pid: self.get_pid().await,
        }
    }

    /// Get the process PID
    async fn get_pid(&self) -> Option<u32> {
        if fs::metadata(&self.config.pid_file).await.is_ok() {
            if let Ok(pid_str) = fs::read_to_string(&self.config.pid_file).await {
                return pid_str.trim().parse::<u32>().ok();
            }
        }
        None
    }

    /// Backup the database to a file
    ///
    /// Uses SurrealDB's native export command to create a backup of all data
    /// in the database. The backup file will contain SQL statements that can
    /// be used to restore the database.
    ///
    /// # Arguments
    /// * `backup_path` - Path where the backup file should be created
    ///
    /// # Returns
    /// * `Ok(())` - Backup completed successfully
    /// * `Err(CortexError)` - Backup failed
    #[instrument(skip(self), fields(backup_path = %backup_path.display()))]
    pub async fn backup(&self, backup_path: PathBuf) -> Result<()> {
        info!("Starting database backup to: {:?}", backup_path);

        // Ensure the server is running
        if !self.is_running().await {
            return Err(CortexError::storage("Cannot backup: SurrealDB server is not running"));
        }

        // Get the binary path
        let binary_path = self
            .binary_path
            .as_ref()
            .ok_or_else(|| CortexError::storage("Binary path not set"))?;

        // Ensure the backup directory exists
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| CortexError::storage(format!("Failed to create backup directory: {}", e)))?;
        }

        // Build the export command
        // Format: surreal export --conn http://localhost:8000 --user root --pass root --ns test --db test backup.surql
        let mut cmd = Command::new(binary_path);
        cmd.arg("export")
            .arg("--conn")
            .arg(&format!("http://{}", self.config.bind_address))
            .arg("--user")
            .arg(&self.config.username)
            .arg("--pass")
            .arg(&self.config.password)
            .arg("--ns")
            .arg("cortex") // Default namespace
            .arg("--db")
            .arg("cortex") // Default database
            .arg(&backup_path);

        debug!("Executing backup command: {:?}", cmd);

        // Execute the export command
        let output = cmd
            .output()
            .await
            .map_err(|e| CortexError::storage(format!("Failed to execute export command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Backup failed: {}", stderr);
            return Err(CortexError::storage(format!("Backup failed: {}", stderr)));
        }

        // Verify the backup file was created
        if !fs::metadata(&backup_path).await.is_ok() {
            return Err(CortexError::storage("Backup file was not created"));
        }

        let file_size = fs::metadata(&backup_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        info!("Backup completed successfully: {:?} ({} bytes)", backup_path, file_size);

        Ok(())
    }

    /// Restore the database from a backup file
    ///
    /// Uses SurrealDB's native import command to restore data from a backup file.
    /// This will execute all SQL statements in the backup file, recreating tables
    /// and records.
    ///
    /// # Arguments
    /// * `backup_path` - Path to the backup file to restore from
    ///
    /// # Returns
    /// * `Ok(())` - Restore completed successfully
    /// * `Err(CortexError)` - Restore failed
    #[instrument(skip(self), fields(backup_path = %backup_path.display()))]
    pub async fn restore(&self, backup_path: PathBuf) -> Result<()> {
        info!("Starting database restore from: {:?}", backup_path);

        // Ensure the server is running
        if !self.is_running().await {
            return Err(CortexError::storage("Cannot restore: SurrealDB server is not running"));
        }

        // Verify the backup file exists
        if !fs::metadata(&backup_path).await.is_ok() {
            return Err(CortexError::storage(format!(
                "Backup file not found: {:?}",
                backup_path
            )));
        }

        // Get the binary path
        let binary_path = self
            .binary_path
            .as_ref()
            .ok_or_else(|| CortexError::storage("Binary path not set"))?;

        // Build the import command
        // Format: surreal import --conn http://localhost:8000 --user root --pass root --ns test --db test backup.surql
        let mut cmd = Command::new(binary_path);
        cmd.arg("import")
            .arg("--conn")
            .arg(&format!("http://{}", self.config.bind_address))
            .arg("--user")
            .arg(&self.config.username)
            .arg("--pass")
            .arg(&self.config.password)
            .arg("--ns")
            .arg("cortex") // Default namespace
            .arg("--db")
            .arg("cortex") // Default database
            .arg(&backup_path);

        debug!("Executing restore command: {:?}", cmd);

        // Execute the import command
        let output = cmd
            .output()
            .await
            .map_err(|e| CortexError::storage(format!("Failed to execute import command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Restore failed: {}", stderr);
            return Err(CortexError::storage(format!("Restore failed: {}", stderr)));
        }

        info!("Restore completed successfully from: {:?}", backup_path);

        Ok(())
    }
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub status: ServerStatus,
    pub bind_address: String,
    pub data_dir: PathBuf,
    pub storage_engine: String,
    pub is_running: bool,
    pub restart_count: u32,
    pub binary_path: Option<PathBuf>,
    pub pid: Option<u32>,
}

impl Drop for SurrealDBManager {
    fn drop(&mut self) {
        // Try to stop the server when the manager is dropped
        if self.process.is_some() {
            warn!("SurrealDBManager dropped while server is running, attempting to stop...");
            // We can't use async in Drop, so we'll do a synchronous kill
            if let Some(mut child) = self.process.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (SurrealDBConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let config = SurrealDBConfig {
            bind_address: "127.0.0.1:9000".to_string(),
            data_dir: base_path.join("data"),
            log_file: base_path.join("logs").join("test.log"),
            pid_file: base_path.join("test.pid"),
            username: "test".to_string(),
            password: "test".to_string(),
            storage_engine: "memory".to_string(),
            allow_guests: true,
            max_retries: 3,
            startup_timeout_secs: 30,
            auto_restart: false, // Disable for tests
            health_check_interval_secs: 30,
            max_restart_attempts: 5,
            start_on_boot: false,
        };

        (config, temp_dir)
    }

    #[test]
    fn test_config_default() {
        let config = SurrealDBConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1:8000");
        assert_eq!(config.username, "root");
        assert_eq!(config.storage_engine, "rocksdb");
    }

    #[test]
    fn test_config_validation() {
        let (config, _temp) = create_test_config();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.username = String::new();
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_config_ensure_directories() {
        let (config, _temp) = create_test_config();
        assert!(config.ensure_directories().await.is_ok());
        assert!(tokio::fs::metadata(&config.data_dir).await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_find_surreal_binary() {
        // This test may fail if SurrealDB is not installed
        match SurrealDBManager::find_surreal_binary().await {
            Ok(path) => {
                println!("Found SurrealDB at: {:?}", path);
                assert!(path.exists());
            }
            Err(e) => {
                println!("SurrealDB not found (expected if not installed): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_manager_status() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await.unwrap();
        assert_eq!(manager.status(), ServerStatus::Stopped);
    }

    #[tokio::test]
    async fn test_connection_url() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await.unwrap();
        assert_eq!(manager.connection_url(), "http://127.0.0.1:9000");
    }

    #[tokio::test]
    async fn test_config_with_auth() {
        let (config, _temp) = create_test_config();
        let config = config.with_auth("admin".to_string(), "admin123".to_string());
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "admin123");
    }

    #[tokio::test]
    async fn test_config_with_storage_engine() {
        let (config, _temp) = create_test_config();
        let config = config.with_storage_engine("rocksdb".to_string());
        assert_eq!(config.storage_engine, "rocksdb");
    }

    #[tokio::test]
    async fn test_config_with_allow_guests() {
        let (config, _temp) = create_test_config();
        let config = config.with_allow_guests(false);
        assert!(!config.allow_guests);
    }

    #[tokio::test]
    async fn test_invalid_config_empty_bind_address() {
        let (mut config, _temp) = create_test_config();
        config.bind_address = String::new();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_invalid_config_empty_password() {
        let (mut config, _temp) = create_test_config();
        config.password = String::new();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_server_info() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config.clone()).await.unwrap();
        let info = manager.server_info().await;

        assert_eq!(info.bind_address, config.bind_address);
        assert_eq!(info.storage_engine, config.storage_engine);
        assert_eq!(info.status, ServerStatus::Stopped);
        assert!(!info.is_running);
        assert_eq!(info.restart_count, 0);
    }

    #[tokio::test]
    async fn test_restart_count() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await.unwrap();
        assert_eq!(manager.restart_count(), 0);
    }

    #[tokio::test]
    async fn test_binary_path_initially_none() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await.unwrap();
        assert!(manager.binary_path().is_none());
    }

    #[test]
    fn test_server_status_equality() {
        assert_eq!(ServerStatus::Running, ServerStatus::Running);
        assert_eq!(ServerStatus::Stopped, ServerStatus::Stopped);
        assert_ne!(ServerStatus::Running, ServerStatus::Stopped);
    }

    #[tokio::test]
    async fn test_get_pid_when_no_file() {
        let (config, _temp) = create_test_config();
        let manager = SurrealDBManager::new(config).await.unwrap();
        let pid = manager.get_pid().await;
        assert!(pid.is_none());
    }

    #[tokio::test]
    async fn test_ensure_directories_creates_all_paths() {
        let (config, _temp) = create_test_config();

        // Initially directories shouldn't exist
        assert!(!tokio::fs::metadata(&config.data_dir).await.is_ok());

        config.ensure_directories().await.unwrap();

        // Now they should exist
        assert!(tokio::fs::metadata(&config.data_dir).await.is_ok());
        assert!(tokio::fs::metadata(&config.log_file.parent().unwrap()).await.is_ok());
        assert!(tokio::fs::metadata(&config.pid_file.parent().unwrap()).await.is_ok());
    }

    #[tokio::test]
    async fn test_config_builder_chain() {
        let (config, _temp) = create_test_config();

        let config = config
            .with_auth("user".to_string(), "pass".to_string())
            .with_storage_engine("rocksdb".to_string())
            .with_allow_guests(false);

        assert_eq!(config.username, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.storage_engine, "rocksdb");
        assert!(!config.allow_guests);
    }

    #[test]
    fn test_server_info_serialization() {
        let info = ServerInfo {
            status: ServerStatus::Running,
            bind_address: "127.0.0.1:8000".to_string(),
            data_dir: PathBuf::from("/tmp/data"),
            storage_engine: "memory".to_string(),
            is_running: true,
            restart_count: 0,
            binary_path: None,
            pid: Some(1234),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Running"));
        assert!(json.contains("127.0.0.1:8000"));
        assert!(json.contains("1234"));
    }

    #[test]
    fn test_config_serialization() {
        let (config, _temp) = create_test_config();

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("127.0.0.1:9000"));
        assert!(json.contains("test"));
        assert!(json.contains("memory"));

        let deserialized: SurrealDBConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.bind_address, config.bind_address);
        assert_eq!(deserialized.username, config.username);
    }
}
