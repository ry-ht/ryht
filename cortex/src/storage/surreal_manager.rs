use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::Surreal;

/// Mode for SurrealDB operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurrealMode {
    /// Embedded mode (default) - SurrealDB runs in-process
    Embedded,
    /// Server mode - Connect to external SurrealDB server
    Server,
}

/// Configuration for SurrealDB manager
#[derive(Debug, Clone)]
pub struct SurrealManagerConfig {
    /// Operation mode
    pub mode: SurrealMode,
    /// Server URL (for server mode)
    pub server_url: String,
    /// Server port (for starting embedded server)
    pub port: u16,
    /// Database path (for embedded mode)
    pub db_path: PathBuf,
    /// SurrealDB binary path
    pub surreal_bin: PathBuf,
    /// Namespace
    pub namespace: String,
    /// Database name
    pub database: String,
    /// Username for authentication (optional)
    pub username: Option<String>,
    /// Password for authentication (optional)
    pub password: Option<String>,
}

impl Default for SurrealManagerConfig {
    fn default() -> Self {
        Self {
            mode: SurrealMode::Embedded,
            server_url: "127.0.0.1:8000".to_string(),
            port: 8000,
            db_path: PathBuf::from("data/surreal"),
            surreal_bin: PathBuf::from("/usr/local/bin/surreal"),
            namespace: "meridian".to_string(),
            database: "knowledge".to_string(),
            username: None,
            password: None,
        }
    }
}

impl SurrealManagerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(mode) = std::env::var("SURREAL_MODE") {
            config.mode = match mode.as_str() {
                "server" => SurrealMode::Server,
                _ => SurrealMode::Embedded,
            };
        }

        if let Ok(url) = std::env::var("SURREAL_URL") {
            config.server_url = url;
        }

        if let Ok(port) = std::env::var("SURREAL_PORT") {
            if let Ok(port_num) = port.parse() {
                config.port = port_num;
            }
        }

        if let Ok(path) = std::env::var("SURREAL_DB_PATH") {
            config.db_path = PathBuf::from(path);
        }

        if let Ok(bin) = std::env::var("SURREAL_BIN") {
            config.surreal_bin = PathBuf::from(bin);
        }

        if let Ok(ns) = std::env::var("SURREAL_NAMESPACE") {
            config.namespace = ns;
        }

        if let Ok(db) = std::env::var("SURREAL_DATABASE") {
            config.database = db;
        }

        if let Ok(user) = std::env::var("SURREAL_USER") {
            config.username = Some(user);
        }

        if let Ok(pass) = std::env::var("SURREAL_PASS") {
            config.password = Some(pass);
        }

        config
    }
}

/// Manager for SurrealDB lifecycle
pub struct SurrealManager {
    config: SurrealManagerConfig,
    server_process: Arc<RwLock<Option<Child>>>,
}

impl SurrealManager {
    /// Create a new SurrealDB manager
    pub fn new(config: SurrealManagerConfig) -> Self {
        Self {
            config,
            server_process: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a manager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SurrealManagerConfig::default())
    }

    /// Create a manager from environment variables
    pub fn from_env() -> Self {
        Self::new(SurrealManagerConfig::from_env())
    }

    /// Start the SurrealDB server (if in server mode)
    pub async fn start(&self) -> Result<()> {
        match self.config.mode {
            SurrealMode::Embedded => {
                tracing::info!("Using embedded SurrealDB mode - no server to start");
                Ok(())
            }
            SurrealMode::Server => {
                self.start_server().await
            }
        }
    }

    /// Start the SurrealDB server process
    async fn start_server(&self) -> Result<()> {
        let mut process_guard = self.server_process.write().await;

        if process_guard.is_some() {
            tracing::warn!("SurrealDB server is already running");
            return Ok(());
        }

        tracing::info!(
            port = self.config.port,
            path = ?self.config.db_path,
            "Starting SurrealDB server"
        );

        // Ensure database directory exists
        std::fs::create_dir_all(&self.config.db_path)
            .context("Failed to create database directory")?;

        // Build server command
        let mut cmd = Command::new(&self.config.surreal_bin);
        cmd.arg("start")
            .arg("--bind")
            .arg(format!("127.0.0.1:{}", self.config.port))
            .arg("--log")
            .arg("info")
            .arg("--user")
            .arg(self.config.username.as_ref().unwrap_or(&"root".to_string()))
            .arg("--pass")
            .arg(self.config.password.as_ref().unwrap_or(&"root".to_string()))
            .arg(format!("rocksdb:{}", self.config.db_path.display()));

        // Spawn the process
        let child = cmd
            .spawn()
            .context("Failed to start SurrealDB server")?;

        *process_guard = Some(child);

        // Wait for server to be ready
        self.wait_for_server().await?;

        tracing::info!("SurrealDB server started successfully");

        Ok(())
    }

    /// Wait for the server to be ready
    async fn wait_for_server(&self) -> Result<()> {
        let max_attempts = 30;
        let delay = Duration::from_millis(100);

        for attempt in 1..=max_attempts {
            match self.connect().await {
                Ok(_) => {
                    tracing::debug!("SurrealDB server is ready after {} attempts", attempt);
                    return Ok(());
                }
                Err(e) => {
                    if attempt == max_attempts {
                        return Err(e).context("Server failed to become ready");
                    }
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(anyhow::anyhow!("Server did not become ready in time"))
    }

    /// Stop the SurrealDB server
    pub async fn stop(&self) -> Result<()> {
        let mut process_guard = self.server_process.write().await;

        if let Some(mut child) = process_guard.take() {
            tracing::info!("Stopping SurrealDB server");

            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(child.id() as i32);
                if let Err(e) = kill(pid, Signal::SIGTERM) {
                    tracing::warn!("Failed to send SIGTERM: {}", e);
                }
            }

            // Wait for process to exit
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Force kill if still running
            if let Err(e) = child.kill() {
                tracing::warn!("Failed to kill SurrealDB process: {}", e);
            }

            let _ = child.wait();

            tracing::info!("SurrealDB server stopped");
        }

        Ok(())
    }

    /// Connect to SurrealDB (works for both embedded and server modes)
    pub async fn connect(&self) -> Result<Surreal<Client>> {
        match self.config.mode {
            SurrealMode::Embedded => {
                Err(anyhow::anyhow!(
                    "Use SurrealDBStorage::new() for embedded mode instead of connect()"
                ))
            }
            SurrealMode::Server => {
                self.connect_to_server().await
            }
        }
    }

    /// Connect to SurrealDB server
    async fn connect_to_server(&self) -> Result<Surreal<Client>> {
        let url = format!("ws://{}", self.config.server_url);

        tracing::debug!(url = %url, "Connecting to SurrealDB server");

        let db = Surreal::new::<Ws>(&url)
            .await
            .context("Failed to connect to SurrealDB server")?;

        // Authenticate if credentials are provided
        if let (Some(user), Some(pass)) = (&self.config.username, &self.config.password) {
            db.signin(surrealdb::opt::auth::Root {
                username: user,
                password: pass,
            })
            .await
            .context("Failed to authenticate with SurrealDB")?;
        }

        // Use namespace and database
        db.use_ns(&self.config.namespace)
            .use_db(&self.config.database)
            .await
            .context("Failed to set namespace and database")?;

        Ok(db)
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        let process_guard = self.server_process.read().await;
        process_guard.is_some()
    }

    /// Get the configuration
    pub fn config(&self) -> &SurrealManagerConfig {
        &self.config
    }
}

impl Drop for SurrealManager {
    fn drop(&mut self) {
        // Note: We can't call async stop() here, so we do a best-effort cleanup
        if let Ok(mut guard) = self.server_process.try_write() {
            if let Some(mut child) = guard.take() {
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

    #[tokio::test]
    async fn test_manager_creation() {
        let config = SurrealManagerConfig::default();
        let manager = SurrealManager::new(config);
        assert!(!manager.is_running().await);
    }

    #[tokio::test]
    async fn test_embedded_mode() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = SurrealManagerConfig::default();
        config.mode = SurrealMode::Embedded;
        config.db_path = temp_dir.path().to_path_buf();

        let manager = SurrealManager::new(config);

        // Start should succeed but not start a process
        assert!(manager.start().await.is_ok());
        assert!(!manager.is_running().await);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("SURREAL_MODE", "server");
        std::env::set_var("SURREAL_PORT", "9000");
        std::env::set_var("SURREAL_NAMESPACE", "test");

        let config = SurrealManagerConfig::from_env();

        assert_eq!(config.mode, SurrealMode::Server);
        assert_eq!(config.port, 9000);
        assert_eq!(config.namespace, "test");

        // Clean up
        std::env::remove_var("SURREAL_MODE");
        std::env::remove_var("SURREAL_PORT");
        std::env::remove_var("SURREAL_NAMESPACE");
    }
}
