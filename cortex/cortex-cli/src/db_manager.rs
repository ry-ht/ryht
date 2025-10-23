//! Unified database management for both SurrealDB and Qdrant.
//!
//! This module provides a unified interface for managing both SurrealDB (relational/metadata)
//! and Qdrant (vector database) together as a single database infrastructure.
//!
//! # Features
//!
//! - Coordinated startup/shutdown sequences
//! - Parallel health checks
//! - Combined status reporting
//! - Process management with timeouts
//! - Graceful error handling
//! - Docker Compose and direct binary support

use crate::output;
use anyhow::{Context, Result, bail};
use cortex_storage::{QdrantClient, QdrantConfig, SurrealDBManager, SurrealDBConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info};

/// Database backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    SurrealDB,
    Qdrant,
}

impl DatabaseBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseBackend::SurrealDB => "SurrealDB",
            DatabaseBackend::Qdrant => "Qdrant",
        }
    }
}

/// Combined database status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStatus {
    pub surrealdb: ComponentStatus,
    pub qdrant: ComponentStatus,
    pub overall_healthy: bool,
}

/// Status of an individual database component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub running: bool,
    pub healthy: bool,
    pub url: String,
    pub error: Option<String>,
    pub metrics: Option<ComponentMetrics>,
}

/// Metrics for a database component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub uptime_seconds: Option<u64>,
    pub memory_mb: Option<u64>,
    pub connections: Option<usize>,
    pub requests_per_sec: Option<f64>,
}

/// Configuration for the unified database manager
#[derive(Debug, Clone)]
pub struct DatabaseManagerConfig {
    pub use_docker_compose: bool,
    pub docker_compose_file: PathBuf,
    pub startup_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub health_check_interval: Duration,
    pub health_check_retries: u32,
}

impl Default for DatabaseManagerConfig {
    fn default() -> Self {
        Self {
            use_docker_compose: true,
            docker_compose_file: PathBuf::from("docker-compose.yml"),
            startup_timeout: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(2),
            health_check_retries: 15,
        }
    }
}

/// Unified database manager for SurrealDB and Qdrant
pub struct DatabaseManager {
    config: DatabaseManagerConfig,
    surrealdb_config: SurrealDBConfig,
    qdrant_config: QdrantConfig,
}

impl DatabaseManager {
    /// Create a new database manager with the given configuration
    pub async fn new(
        config: DatabaseManagerConfig,
        surrealdb_config: SurrealDBConfig,
        qdrant_config: QdrantConfig,
    ) -> Result<Self> {
        Ok(Self {
            config,
            surrealdb_config,
            qdrant_config,
        })
    }

    /// Start both databases in the correct sequence
    ///
    /// 1. SurrealDB starts first (metadata/relational)
    /// 2. Wait for SurrealDB health check
    /// 3. Qdrant starts second (vector database)
    /// 4. Wait for Qdrant health check
    /// 5. Verify both are healthy
    pub async fn start(&self) -> Result<()> {
        info!("Starting database infrastructure...");

        if self.config.use_docker_compose {
            self.start_with_docker_compose().await
        } else {
            self.start_with_binaries().await
        }
    }

    /// Start databases using Docker Compose
    async fn start_with_docker_compose(&self) -> Result<()> {
        info!("Starting databases with Docker Compose");

        // Check if docker-compose is available
        let compose_available = self.check_docker_compose_available().await?;
        if !compose_available {
            bail!("docker-compose is not available. Install it or use direct binary mode.");
        }

        // Check if docker-compose file exists
        if !self.config.docker_compose_file.exists() {
            bail!("Docker compose file not found: {}", self.config.docker_compose_file.display());
        }

        output::info("Starting SurrealDB...");

        // Start SurrealDB service
        let result = timeout(
            self.config.startup_timeout,
            Command::new("docker-compose")
                .arg("-f")
                .arg(&self.config.docker_compose_file)
                .arg("up")
                .arg("-d")
                .arg("surrealdb")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if !output_result.status.success() {
                    let stderr = String::from_utf8_lossy(&output_result.stderr);
                    bail!("Failed to start SurrealDB: {}", stderr);
                }
                output::success("SurrealDB container started");
            }
            Ok(Err(e)) => bail!("Failed to execute docker-compose: {}", e),
            Err(_) => bail!("SurrealDB startup timed out after {:?}", self.config.startup_timeout),
        }

        // Wait for SurrealDB to be healthy
        output::info("Waiting for SurrealDB to become healthy...");
        self.wait_for_health(DatabaseBackend::SurrealDB).await?;
        output::success("SurrealDB is healthy");

        output::info("Starting Qdrant...");

        // Start Qdrant service
        let result = timeout(
            self.config.startup_timeout,
            Command::new("docker-compose")
                .arg("-f")
                .arg(&self.config.docker_compose_file)
                .arg("up")
                .arg("-d")
                .arg("qdrant")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if !output_result.status.success() {
                    let stderr = String::from_utf8_lossy(&output_result.stderr);
                    bail!("Failed to start Qdrant: {}", stderr);
                }
                output::success("Qdrant container started");
            }
            Ok(Err(e)) => bail!("Failed to execute docker-compose: {}", e),
            Err(_) => bail!("Qdrant startup timed out after {:?}", self.config.startup_timeout),
        }

        // Wait for Qdrant to be healthy
        output::info("Waiting for Qdrant to become healthy...");
        self.wait_for_health(DatabaseBackend::Qdrant).await?;
        output::success("Qdrant is healthy");

        info!("All databases started successfully");
        Ok(())
    }

    /// Start databases using direct binaries
    async fn start_with_binaries(&self) -> Result<()> {
        info!("Starting databases with direct binaries");

        output::info("Starting SurrealDB...");

        // Start SurrealDB
        let mut surreal_manager = SurrealDBManager::new(self.surrealdb_config.clone()).await?;
        surreal_manager.start().await.context("Failed to start SurrealDB")?;

        output::success("SurrealDB process started");

        // Wait for SurrealDB health
        output::info("Waiting for SurrealDB to become healthy...");
        self.wait_for_health(DatabaseBackend::SurrealDB).await?;
        output::success("SurrealDB is healthy");

        // Prevent SurrealDB manager from stopping on drop
        std::mem::forget(surreal_manager);

        output::info("Starting Qdrant...");

        // Start Qdrant (assumes it's managed externally or via docker)
        // For binary mode, we'll use docker for Qdrant as it's more common
        let result = timeout(
            self.config.startup_timeout,
            Command::new("docker")
                .arg("run")
                .arg("-d")
                .arg("--name")
                .arg("cortex-qdrant")
                .arg("-p")
                .arg(format!("{}:6333", self.qdrant_config.port))
                .arg("-p")
                .arg(format!("{}:6334", self.qdrant_config.grpc_port.unwrap_or(6334)))
                .arg("-v")
                .arg("qdrant_storage:/qdrant/storage")
                .arg("qdrant/qdrant:v1.12.5")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if !output_result.status.success() {
                    let stderr = String::from_utf8_lossy(&output_result.stderr);
                    // Check if container already exists
                    if !stderr.contains("already in use") {
                        bail!("Failed to start Qdrant container: {}", stderr);
                    }
                    output::warning("Qdrant container already exists, attempting to start...");

                    // Try to start existing container
                    Command::new("docker")
                        .arg("start")
                        .arg("cortex-qdrant")
                        .output()
                        .await
                        .context("Failed to start existing Qdrant container")?;
                }
                output::success("Qdrant container started");
            }
            Ok(Err(e)) => bail!("Failed to execute docker: {}", e),
            Err(_) => bail!("Qdrant startup timed out after {:?}", self.config.startup_timeout),
        }

        // Wait for Qdrant health
        output::info("Waiting for Qdrant to become healthy...");
        self.wait_for_health(DatabaseBackend::Qdrant).await?;
        output::success("Qdrant is healthy");

        info!("All databases started successfully");
        Ok(())
    }

    /// Stop both databases in reverse order (Qdrant first, then SurrealDB)
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping database infrastructure...");

        if self.config.use_docker_compose {
            self.stop_with_docker_compose().await
        } else {
            self.stop_with_binaries().await
        }
    }

    /// Stop databases using Docker Compose
    async fn stop_with_docker_compose(&self) -> Result<()> {
        info!("Stopping databases with Docker Compose");

        let mut errors = Vec::new();

        // Stop Qdrant first
        output::info("Stopping Qdrant...");
        let result = timeout(
            self.config.shutdown_timeout,
            Command::new("docker-compose")
                .arg("-f")
                .arg(&self.config.docker_compose_file)
                .arg("stop")
                .arg("qdrant")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if output_result.status.success() {
                    output::success("Qdrant stopped");
                } else {
                    let err = format!("Failed to stop Qdrant: {}",
                        String::from_utf8_lossy(&output_result.stderr));
                    output::error(&err);
                    errors.push(err);
                }
            }
            Ok(Err(e)) => {
                let err = format!("Failed to execute docker-compose: {}", e);
                output::error(&err);
                errors.push(err);
            }
            Err(_) => {
                let err = format!("Qdrant shutdown timed out after {:?}", self.config.shutdown_timeout);
                output::warning(&err);
                errors.push(err);
            }
        }

        // Stop SurrealDB second
        output::info("Stopping SurrealDB...");
        let result = timeout(
            self.config.shutdown_timeout,
            Command::new("docker-compose")
                .arg("-f")
                .arg(&self.config.docker_compose_file)
                .arg("stop")
                .arg("surrealdb")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if output_result.status.success() {
                    output::success("SurrealDB stopped");
                } else {
                    let err = format!("Failed to stop SurrealDB: {}",
                        String::from_utf8_lossy(&output_result.stderr));
                    output::error(&err);
                    errors.push(err);
                }
            }
            Ok(Err(e)) => {
                let err = format!("Failed to execute docker-compose: {}", e);
                output::error(&err);
                errors.push(err);
            }
            Err(_) => {
                let err = format!("SurrealDB shutdown timed out after {:?}", self.config.shutdown_timeout);
                output::warning(&err);
                errors.push(err);
            }
        }

        if errors.is_empty() {
            info!("All databases stopped successfully");
            Ok(())
        } else {
            bail!("Some databases failed to stop gracefully: {}", errors.join(", "))
        }
    }

    /// Stop databases using direct binaries
    async fn stop_with_binaries(&self) -> Result<()> {
        info!("Stopping databases with direct binaries");

        let mut errors = Vec::new();

        // Stop Qdrant first (docker container)
        output::info("Stopping Qdrant...");
        let result = timeout(
            self.config.shutdown_timeout,
            Command::new("docker")
                .arg("stop")
                .arg("cortex-qdrant")
                .output()
        ).await;

        match result {
            Ok(Ok(output_result)) => {
                if output_result.status.success() {
                    output::success("Qdrant stopped");
                } else {
                    let stderr = String::from_utf8_lossy(&output_result.stderr);
                    if !stderr.contains("No such container") {
                        let err = format!("Failed to stop Qdrant: {}", stderr);
                        output::error(&err);
                        errors.push(err);
                    } else {
                        output::warning("Qdrant container not found");
                    }
                }
            }
            Ok(Err(e)) => {
                let err = format!("Failed to execute docker: {}", e);
                output::error(&err);
                errors.push(err);
            }
            Err(_) => {
                let err = format!("Qdrant shutdown timed out");
                output::warning(&err);
                errors.push(err);
            }
        }

        // Stop SurrealDB
        output::info("Stopping SurrealDB...");
        let mut surreal_manager = SurrealDBManager::new(self.surrealdb_config.clone()).await?;
        match surreal_manager.stop().await {
            Ok(_) => output::success("SurrealDB stopped"),
            Err(e) => {
                let err = format!("Failed to stop SurrealDB: {}", e);
                output::error(&err);
                errors.push(err);
            }
        }

        if errors.is_empty() {
            info!("All databases stopped successfully");
            Ok(())
        } else {
            bail!("Some databases failed to stop gracefully: {}", errors.join(", "))
        }
    }

    /// Get combined status of both databases
    pub async fn status(&self) -> Result<DatabaseStatus> {
        info!("Checking database infrastructure status");

        // Check both databases in parallel
        let (surrealdb_status, qdrant_status) = tokio::join!(
            self.check_component_status(DatabaseBackend::SurrealDB),
            self.check_component_status(DatabaseBackend::Qdrant)
        );

        let surrealdb = surrealdb_status?;
        let qdrant = qdrant_status?;

        let overall_healthy = surrealdb.healthy && qdrant.healthy;

        Ok(DatabaseStatus {
            surrealdb,
            qdrant,
            overall_healthy,
        })
    }

    /// Check the status of a single database component
    async fn check_component_status(&self, backend: DatabaseBackend) -> Result<ComponentStatus> {
        match backend {
            DatabaseBackend::SurrealDB => {
                let url = self.surrealdb_connection_url();
                let manager = SurrealDBManager::new(self.surrealdb_config.clone()).await?;

                let running = manager.is_running().await;
                let (healthy, error) = if running {
                    match manager.health_check().await {
                        Ok(_) => (true, None),
                        Err(e) => (false, Some(e.to_string())),
                    }
                } else {
                    (false, Some("Process not running".to_string()))
                };

                Ok(ComponentStatus {
                    running,
                    healthy,
                    url,
                    error,
                    metrics: None, // Could be enhanced with actual metrics
                })
            }
            DatabaseBackend::Qdrant => {
                let url = self.qdrant_connection_url();

                match QdrantClient::new(self.qdrant_config.clone()).await {
                    Ok(client) => {
                        match client.health().await {
                            Ok(_health) => {
                                Ok(ComponentStatus {
                                    running: true,
                                    healthy: true,
                                    url,
                                    error: None,
                                    metrics: None,
                                })
                            }
                            Err(e) => {
                                Ok(ComponentStatus {
                                    running: false,
                                    healthy: false,
                                    url,
                                    error: Some(e.to_string()),
                                    metrics: None,
                                })
                            }
                        }
                    }
                    Err(e) => {
                        Ok(ComponentStatus {
                            running: false,
                            healthy: false,
                            url,
                            error: Some(e.to_string()),
                            metrics: None,
                        })
                    }
                }
            }
        }
    }

    /// Wait for a database component to become healthy
    async fn wait_for_health(&self, backend: DatabaseBackend) -> Result<()> {
        let mut retries = 0;

        loop {
            tokio::time::sleep(self.config.health_check_interval).await;

            match self.check_component_status(backend).await {
                Ok(status) if status.healthy => {
                    debug!("{} is healthy", backend.as_str());
                    return Ok(());
                }
                Ok(status) => {
                    debug!("{} not yet healthy: {:?}", backend.as_str(), status.error);
                }
                Err(e) => {
                    debug!("{} health check error: {}", backend.as_str(), e);
                }
            }

            retries += 1;
            if retries >= self.config.health_check_retries {
                bail!(
                    "{} did not become healthy after {} retries",
                    backend.as_str(),
                    self.config.health_check_retries
                );
            }
        }
    }

    /// Check if docker-compose is available
    async fn check_docker_compose_available(&self) -> Result<bool> {
        let result = Command::new("docker-compose")
            .arg("--version")
            .output()
            .await;

        Ok(result.is_ok() && result.unwrap().status.success())
    }

    /// Get SurrealDB connection URL
    fn surrealdb_connection_url(&self) -> String {
        format!("http://{}", self.surrealdb_config.bind_address)
    }

    /// Get Qdrant connection URL
    fn qdrant_connection_url(&self) -> String {
        let protocol = if self.qdrant_config.use_https { "https" } else { "http" };
        format!("{}://{}:{}", protocol, self.qdrant_config.host, self.qdrant_config.port)
    }

    /// Restart both databases
    pub async fn restart(&self) -> Result<()> {
        info!("Restarting database infrastructure...");

        output::info("Stopping databases...");
        self.stop().await?;

        output::info("Starting databases...");
        self.start().await?;

        output::success("Databases restarted successfully");
        Ok(())
    }
}

/// Create a database manager from global configuration
pub async fn create_from_global_config() -> Result<DatabaseManager> {
    let global_config = cortex_core::config::GlobalConfig::load_or_create_default().await?;
    let db_config = global_config.database();

    // SurrealDB configuration
    let mut surrealdb_config = SurrealDBConfig::default();
    surrealdb_config.username = db_config.username.clone();
    surrealdb_config.password = db_config.password.clone();
    surrealdb_config.bind_address = db_config.local_bind.clone();

    // Qdrant configuration
    let qdrant_config = QdrantConfig {
        host: std::env::var("QDRANT_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("QDRANT_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(6333),
        grpc_port: std::env::var("QDRANT_GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok()),
        api_key: std::env::var("QDRANT_API_KEY").ok(),
        use_https: std::env::var("QDRANT_USE_HTTPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false),
        timeout: Duration::from_secs(10),
        request_timeout: Duration::from_secs(60),
    };

    // Database manager configuration
    let manager_config = DatabaseManagerConfig::default();

    DatabaseManager::new(manager_config, surrealdb_config, qdrant_config).await
}
