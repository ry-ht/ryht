//! Global configuration system for Cortex.
//!
//! This module manages all configuration in ~/.ryht/cortex/, including:
//! - Configuration file loading, saving, and validation
//! - Directory structure creation and management
//! - Environment variable overrides
//! - Atomic configuration updates
//! - Hot-reload support with thread-safe access
//! - Multiple configuration profiles (dev, prod, test)
//! - Configuration migration support
//! - Import/export functionality
//!
//! # Configuration Location
//!
//! By default, configuration is stored at `~/.ryht/cortex/config.toml`.
//! This can be overridden with the `CORTEX_CONFIG_PATH` environment variable.
//!
//! # Directory Structure
//!
//! ```text
//! ~/.ryht/cortex/
//! ├── config.toml          # Main configuration
//! ├── surrealdb/          # SurrealDB data and logs
//! ├── cache/              # Content cache
//! ├── sessions/           # Agent sessions
//! ├── temp/               # Temporary files
//! ├── data/               # Additional data files
//! ├── logs/               # Log files
//! ├── run/                # PID files
//! └── workspaces/         # Workspace metadata
//! ```
//!
//! # Example
//!
//! ```no_run
//! use cortex_core::config::{GlobalConfig, ConfigManager};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Use the global singleton
//! let config_manager = ConfigManager::global().await?;
//!
//! // Read configuration
//! {
//!     let config = config_manager.read().await;
//!     println!("Database mode: {}", config.database().mode);
//!     println!("Log level: {}", config.general().log_level);
//! }
//!
//! // Update configuration
//! {
//!     let mut config = config_manager.write().await;
//!     config.general_mut().log_level = "debug".to_string();
//! }
//!
//! // Save configuration
//! config_manager.save().await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{CortexError, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::OnceCell;
use tracing::{debug, info, warn};

/// The current configuration version for migration support
pub const CONFIG_VERSION: &str = "0.1.0";

/// Environment variable prefix for all Cortex configuration overrides
pub const ENV_PREFIX: &str = "CORTEX_";

// Environment variable names
pub const ENV_CONFIG_PATH: &str = "CORTEX_CONFIG_PATH";
pub const ENV_CONFIG_PROFILE: &str = "CORTEX_CONFIG_PROFILE";
pub const ENV_LOG_LEVEL: &str = "CORTEX_LOG_LEVEL";
pub const ENV_DB_MODE: &str = "CORTEX_DB_MODE";
pub const ENV_DB_URL: &str = "CORTEX_DB_URL";
pub const ENV_DB_LOCAL_BIND: &str = "CORTEX_DB_LOCAL_BIND";
pub const ENV_DB_USERNAME: &str = "CORTEX_DB_USERNAME";
pub const ENV_DB_PASSWORD: &str = "CORTEX_DB_PASSWORD";
pub const ENV_DB_NAMESPACE: &str = "CORTEX_DB_NAMESPACE";
pub const ENV_DB_DATABASE: &str = "CORTEX_DB_DATABASE";
pub const ENV_MCP_SERVER_BIND: &str = "CORTEX_MCP_SERVER_BIND";
pub const ENV_CACHE_SIZE_MB: &str = "CORTEX_CACHE_SIZE_MB";
pub const ENV_CACHE_REDIS_URL: &str = "CORTEX_CACHE_REDIS_URL";
pub const ENV_JWT_SECRET: &str = "JWT_SECRET";

/// Configuration profile enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigProfile {
    /// Development profile with verbose logging and debug features
    Dev,
    /// Production profile with optimized settings
    Prod,
    /// Test profile for automated testing
    Test,
}

impl ConfigProfile {
    /// Get profile from environment variable or default to Dev
    pub fn from_env() -> Self {
        std::env::var(ENV_CONFIG_PROFILE)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Self::Dev)
    }

    /// Get the profile name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Prod => "prod",
            Self::Test => "test",
        }
    }
}

impl std::str::FromStr for ConfigProfile {
    type Err = CortexError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Dev),
            "prod" | "production" => Ok(Self::Prod),
            "test" | "testing" => Ok(Self::Test),
            _ => Err(CortexError::Config(format!(
                "Invalid config profile '{}'. Must be one of: dev, prod, test",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ConfigProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Main global configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    general: GeneralConfig,
    database: DatabaseConfig,
    pool: PoolConfig,
    cache: CacheConfig,
    vfs: VfsConfig,
    ingestion: IngestionConfig,
    mcp: McpConfig,
    auth: AuthConfig,
    /// Configuration profile (dev, prod, test)
    #[serde(default)]
    profile: ConfigProfile,
}

/// General configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Configuration version for migration support
    pub version: String,
    /// Log level: trace, debug, info, warn, error
    pub log_level: String,
    /// Enable hot-reload of configuration
    #[serde(default = "default_true")]
    pub hot_reload: bool,
    /// Hot-reload check interval in seconds
    #[serde(default = "default_reload_interval")]
    pub hot_reload_interval_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_reload_interval() -> u64 {
    5
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database mode: local, remote, or hybrid
    pub mode: String,
    /// Local binding address for embedded database
    pub local_bind: String,
    /// Remote database URLs (for remote or hybrid mode)
    pub remote_urls: Vec<String>,
    /// Database username
    pub username: String,
    /// Database password
    pub password: String,
    /// SurrealDB namespace
    pub namespace: String,
    /// SurrealDB database name
    pub database: String,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Idle connection timeout in milliseconds
    pub idle_timeout_ms: u64,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// In-memory cache size in megabytes
    pub memory_size_mb: u64,
    /// Default TTL for cache entries in seconds
    pub ttl_seconds: u64,
    /// Optional Redis URL for distributed caching
    pub redis_url: String,
}

/// Virtual filesystem configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VfsConfig {
    /// Maximum file size in megabytes
    pub max_file_size_mb: u64,
    /// Enable automatic flushing to disk
    pub auto_flush: bool,
    /// Flush interval in seconds
    pub flush_interval_seconds: u64,
}

/// Ingestion pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Number of parallel workers for ingestion
    pub parallel_workers: usize,
    /// Chunk size for batch processing
    pub chunk_size: usize,
    /// Enable automatic embedding generation
    pub generate_embeddings: bool,
    /// Embedding model to use
    pub embedding_model: String,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Server binding address
    pub server_bind: String,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Maximum request size in megabytes
    pub max_request_size_mb: u64,
    /// Log file for stdio mode
    pub log_file_stdio: String,
    /// Log file for HTTP mode
    pub log_file_http: String,
    /// Log level for MCP server (trace, debug, info, warn, error)
    pub log_level: String,
}

/// Authentication and security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key (override with JWT_SECRET env var)
    pub jwt_secret: String,
    /// Access token expiration time in minutes
    pub access_token_expiry_mins: i64,
    /// Refresh token expiration time in days
    pub refresh_token_expiry_days: i64,
    /// JWT issuer
    pub jwt_issuer: String,
    /// JWT audience
    pub jwt_audience: String,
    /// Enable API key authentication
    pub api_keys_enabled: bool,
    /// Maximum number of active sessions per user
    pub max_sessions_per_user: usize,
}

impl Default for ConfigProfile {
    fn default() -> Self {
        Self::Dev
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION.to_string(),
            log_level: "info".to_string(),
            hot_reload: true,
            hot_reload_interval_secs: 5,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            mode: "local".to_string(),
            local_bind: "127.0.0.1:8000".to_string(),
            remote_urls: vec![],
            username: "root".to_string(),
            password: "root".to_string(),
            namespace: "cortex".to_string(),
            database: "knowledge".to_string(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 2,
            max_connections: 10,
            connection_timeout_ms: 5000,
            idle_timeout_ms: 300000,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            memory_size_mb: 512,
            ttl_seconds: 300,
            redis_url: String::new(),
        }
    }
}

impl Default for VfsConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: 100,
            auto_flush: false,
            flush_interval_seconds: 60,
        }
    }
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            parallel_workers: 4,
            chunk_size: 1000,
            generate_embeddings: true,
            embedding_model: "text-embedding-3-small".to_string(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        let ryht_dir = home.join(".ryht").join("cortex").join("mcp");

        Self {
            server_bind: "127.0.0.1:3000".to_string(),
            cors_enabled: true,
            max_request_size_mb: 10,
            log_file_stdio: ryht_dir.join("logs").join("mcp-stdio.log").to_string_lossy().to_string(),
            log_file_http: ryht_dir.join("logs").join("mcp-http.log").to_string_lossy().to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "cortex-dev-secret-change-in-production".to_string(),
            access_token_expiry_mins: 15,
            refresh_token_expiry_days: 7,
            jwt_issuer: "cortex-api".to_string(),
            jwt_audience: "cortex-client".to_string(),
            api_keys_enabled: true,
            max_sessions_per_user: 5,
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            database: DatabaseConfig::default(),
            pool: PoolConfig::default(),
            cache: CacheConfig::default(),
            vfs: VfsConfig::default(),
            ingestion: IngestionConfig::default(),
            mcp: McpConfig::default(),
            auth: AuthConfig::default(),
            profile: ConfigProfile::default(),
        }
    }
}

impl GlobalConfig {
    /// Create a new configuration with the specified profile
    pub fn with_profile(profile: ConfigProfile) -> Self {
        let mut config = match profile {
            ConfigProfile::Dev => Self::dev_defaults(),
            ConfigProfile::Prod => Self::prod_defaults(),
            ConfigProfile::Test => Self::test_defaults(),
        };
        config.profile = profile;
        config
    }

    /// Get development profile defaults
    fn dev_defaults() -> Self {
        let mut config = Self::default();
        config.general.log_level = "debug".to_string();
        config.general.hot_reload = true;
        config.pool.max_connections = 5;
        config.cache.memory_size_mb = 256;
        config
    }

    /// Get production profile defaults
    fn prod_defaults() -> Self {
        let mut config = Self::default();
        config.general.log_level = "info".to_string();
        config.general.hot_reload = false;
        config.pool.max_connections = 20;
        config.cache.memory_size_mb = 2048;
        config
    }

    /// Get test profile defaults
    fn test_defaults() -> Self {
        let mut config = Self::default();
        config.general.log_level = "warn".to_string();
        config.general.hot_reload = false;
        config.pool.max_connections = 2;
        config.cache.memory_size_mb = 128;
        config.database.namespace = "cortex_test".to_string();
        config.database.database = "test".to_string();
        config
    }

    /// Get the current configuration profile
    pub fn profile(&self) -> ConfigProfile {
        self.profile
    }

    /// Set the configuration profile
    pub fn set_profile(&mut self, profile: ConfigProfile) {
        self.profile = profile;
    }
}

impl GlobalConfig {
    /// Load configuration from the default location
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration file cannot be read or parsed
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from_path(&config_path).await
    }

    /// Load configuration from a specific path
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed
    pub async fn load_from_path(path: &Path) -> Result<Self> {
        debug!("Loading configuration from: {}", path.display());

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| CortexError::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Self = toml::from_str(&content)
            .map_err(|e| CortexError::Config(format!("Failed to parse config file: {}", e)))?;

        // Apply environment variable overrides
        config.merge_env_vars()?;

        // Validate the configuration
        config.validate()?;

        info!("Configuration loaded successfully from {}", path.display());
        Ok(config)
    }

    /// Load configuration or create default if it doesn't exist
    ///
    /// This will:
    /// 1. Ensure all directories exist
    /// 2. Create a default config file if none exists
    /// 3. Load and validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if directories cannot be created or config cannot be saved/loaded
    pub async fn load_or_create_default() -> Result<Self> {
        // Ensure all directories exist
        Self::ensure_directories().await?;

        let config_path = Self::config_path()?;

        if config_path.exists() {
            debug!("Loading existing configuration");
            Self::load().await
        } else {
            info!("Creating default configuration at {}", config_path.display());
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }

    /// Save configuration to the default location
    ///
    /// Uses atomic write (write to temp file, then rename) to ensure consistency
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be serialized or written
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        self.save_to_path(&config_path).await
    }

    /// Save configuration to a specific path atomically
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be serialized or written
    pub async fn save_to_path(&self, path: &Path) -> Result<()> {
        debug!("Saving configuration to: {}", path.display());

        // Validate before saving
        self.validate()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| CortexError::Config(format!("Failed to create config directory: {}", e)))?;
            }
        }

        // Serialize to TOML
        let content = toml::to_string_pretty(self)
            .map_err(|e| CortexError::Config(format!("Failed to serialize config: {}", e)))?;

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("toml.tmp");

        tokio::fs::write(&temp_path, content)
            .await
            .map_err(|e| CortexError::Config(format!("Failed to write config file: {}", e)))?;

        tokio::fs::rename(&temp_path, path)
            .await
            .map_err(|e| CortexError::Config(format!("Failed to rename config file: {}", e)))?;

        info!("Configuration saved successfully to {}", path.display());
        Ok(())
    }

    /// Validate the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration values are invalid
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        let valid_log_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_log_levels.contains(&self.general.log_level.as_str()) {
            return Err(CortexError::Config(format!(
                "Invalid log level '{}'. Must be one of: {}",
                self.general.log_level,
                valid_log_levels.join(", ")
            )));
        }

        // Validate database mode
        let valid_db_modes = ["local", "remote", "hybrid"];
        if !valid_db_modes.contains(&self.database.mode.as_str()) {
            return Err(CortexError::Config(format!(
                "Invalid database mode '{}'. Must be one of: {}",
                self.database.mode,
                valid_db_modes.join(", ")
            )));
        }

        // Validate remote URLs are provided for remote/hybrid modes
        if (self.database.mode == "remote" || self.database.mode == "hybrid")
            && self.database.remote_urls.is_empty()
        {
            return Err(CortexError::Config(
                "Remote database URLs must be provided for remote/hybrid mode".to_string(),
            ));
        }

        // Validate pool configuration
        if self.pool.min_connections > self.pool.max_connections {
            return Err(CortexError::Config(
                "min_connections cannot be greater than max_connections".to_string(),
            ));
        }

        if self.pool.max_connections == 0 {
            return Err(CortexError::Config(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        // Validate cache configuration
        if self.cache.memory_size_mb == 0 {
            warn!("Cache memory size is 0, caching will be disabled");
        }

        // Validate VFS configuration
        if self.vfs.max_file_size_mb == 0 {
            return Err(CortexError::Config(
                "max_file_size_mb must be greater than 0".to_string(),
            ));
        }

        // Validate ingestion configuration
        if self.ingestion.parallel_workers == 0 {
            return Err(CortexError::Config(
                "parallel_workers must be greater than 0".to_string(),
            ));
        }

        if self.ingestion.chunk_size == 0 {
            return Err(CortexError::Config(
                "chunk_size must be greater than 0".to_string(),
            ));
        }

        // Validate MCP configuration
        if self.mcp.max_request_size_mb == 0 {
            return Err(CortexError::Config(
                "max_request_size_mb must be greater than 0".to_string(),
            ));
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Merge environment variable overrides into the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if environment variables contain invalid values
    pub fn merge_env_vars(&mut self) -> Result<()> {
        debug!("Merging environment variable overrides");

        // General
        if let Ok(log_level) = std::env::var(ENV_LOG_LEVEL) {
            debug!("Overriding log_level from environment: {}", log_level);
            self.general.log_level = log_level;
        }

        // Database
        if let Ok(db_mode) = std::env::var(ENV_DB_MODE) {
            debug!("Overriding database mode from environment: {}", db_mode);
            self.database.mode = db_mode;
        }

        if let Ok(db_url) = std::env::var(ENV_DB_URL) {
            debug!("Overriding database URL from environment");
            self.database.remote_urls = vec![db_url];
        }

        if let Ok(local_bind) = std::env::var(ENV_DB_LOCAL_BIND) {
            debug!("Overriding local bind from environment: {}", local_bind);
            self.database.local_bind = local_bind;
        }

        if let Ok(username) = std::env::var(ENV_DB_USERNAME) {
            debug!("Overriding database username from environment");
            self.database.username = username;
        }

        if let Ok(password) = std::env::var(ENV_DB_PASSWORD) {
            debug!("Overriding database password from environment");
            self.database.password = password;
        }

        if let Ok(namespace) = std::env::var(ENV_DB_NAMESPACE) {
            debug!("Overriding database namespace from environment: {}", namespace);
            self.database.namespace = namespace;
        }

        if let Ok(database) = std::env::var(ENV_DB_DATABASE) {
            debug!("Overriding database name from environment: {}", database);
            self.database.database = database;
        }

        // MCP
        if let Ok(server_bind) = std::env::var(ENV_MCP_SERVER_BIND) {
            debug!("Overriding MCP server bind from environment: {}", server_bind);
            self.mcp.server_bind = server_bind;
        }

        // Cache
        if let Ok(cache_size) = std::env::var(ENV_CACHE_SIZE_MB) {
            let size = cache_size.parse::<u64>().map_err(|e| {
                CortexError::Config(format!("Invalid cache size in environment: {}", e))
            })?;
            debug!("Overriding cache size from environment: {} MB", size);
            self.cache.memory_size_mb = size;
        }

        if let Ok(redis_url) = std::env::var(ENV_CACHE_REDIS_URL) {
            debug!("Overriding Redis URL from environment");
            self.cache.redis_url = redis_url;
        }

        // Authentication
        if let Ok(jwt_secret) = std::env::var(ENV_JWT_SECRET) {
            debug!("Overriding JWT secret from environment");
            self.auth.jwt_secret = jwt_secret;
        }

        Ok(())
    }

    /// Ensure all required directories exist
    ///
    /// Creates the following directory structure:
    /// - ~/.ryht/cortex/
    /// - ~/.ryht/cortex/surrealdb/
    /// - ~/.ryht/cortex/cache/
    /// - ~/.ryht/cortex/sessions/
    /// - ~/.ryht/cortex/temp/
    /// - ~/.ryht/cortex/data/
    /// - ~/.ryht/cortex/logs/
    /// - ~/.ryht/cortex/run/
    /// - ~/.ryht/cortex/workspaces/
    ///
    /// # Errors
    ///
    /// Returns an error if directories cannot be created
    pub async fn ensure_directories() -> Result<()> {
        let dirs = vec![
            Self::base_dir()?,
            Self::surrealdb_dir()?,
            Self::cache_dir()?,
            Self::sessions_dir()?,
            Self::temp_dir()?,
            Self::data_dir()?,
            Self::logs_dir()?,
            Self::run_dir()?,
            Self::workspaces_dir()?,
        ];

        for dir in dirs {
            if !dir.exists() {
                debug!("Creating directory: {}", dir.display());
                tokio::fs::create_dir_all(&dir)
                    .await
                    .map_err(|e| {
                        CortexError::Config(format!(
                            "Failed to create directory {}: {}",
                            dir.display(),
                            e
                        ))
                    })?;
                info!("Created directory: {}", dir.display());
            }
        }

        Ok(())
    }

    /// Get the base Cortex directory path (~/.ryht/cortex/)
    ///
    /// Can be overridden with CORTEX_CONFIG_PATH environment variable
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined
    pub fn base_dir() -> Result<PathBuf> {
        if let Ok(config_path) = std::env::var(ENV_CONFIG_PATH) {
            let path = PathBuf::from(config_path);
            if let Some(parent) = path.parent() {
                return Ok(parent.to_path_buf());
            }
        }

        let base_dirs = BaseDirs::new()
            .ok_or_else(|| CortexError::Config("Could not determine home directory".to_string()))?;

        Ok(base_dirs.home_dir().join(".ryht").join("cortex"))
    }

    /// Get the configuration file path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn config_path() -> Result<PathBuf> {
        if let Ok(config_path) = std::env::var(ENV_CONFIG_PATH) {
            return Ok(PathBuf::from(config_path));
        }

        Ok(Self::base_dir()?.join("config.toml"))
    }

    /// Get the data directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn data_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("data"))
    }

    /// Get the SurrealDB data directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn surrealdb_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("surrealdb"))
    }

    /// Get the cache directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn cache_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("cache"))
    }

    /// Get the sessions directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn sessions_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("sessions"))
    }

    /// Get the temp directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn temp_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("temp"))
    }

    /// Get the logs directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn logs_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("logs"))
    }

    /// Get the run directory path (for PID files)
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn run_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("run"))
    }

    /// Get the workspaces directory path
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be determined
    pub fn workspaces_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("workspaces"))
    }

    // Accessor methods for configuration sections

    /// Get general configuration
    pub fn general(&self) -> &GeneralConfig {
        &self.general
    }

    /// Get mutable general configuration
    pub fn general_mut(&mut self) -> &mut GeneralConfig {
        &mut self.general
    }

    /// Get database configuration
    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    /// Get mutable database configuration
    pub fn database_mut(&mut self) -> &mut DatabaseConfig {
        &mut self.database
    }

    /// Get pool configuration
    pub fn pool(&self) -> &PoolConfig {
        &self.pool
    }

    /// Get mutable pool configuration
    pub fn pool_mut(&mut self) -> &mut PoolConfig {
        &mut self.pool
    }

    /// Get cache configuration
    pub fn cache(&self) -> &CacheConfig {
        &self.cache
    }

    /// Get mutable cache configuration
    pub fn cache_mut(&mut self) -> &mut CacheConfig {
        &mut self.cache
    }

    /// Get VFS configuration
    pub fn vfs(&self) -> &VfsConfig {
        &self.vfs
    }

    /// Get mutable VFS configuration
    pub fn vfs_mut(&mut self) -> &mut VfsConfig {
        &mut self.vfs
    }

    /// Get ingestion configuration
    pub fn ingestion(&self) -> &IngestionConfig {
        &self.ingestion
    }

    /// Get mutable ingestion configuration
    pub fn ingestion_mut(&mut self) -> &mut IngestionConfig {
        &mut self.ingestion
    }

    /// Get MCP configuration
    pub fn mcp(&self) -> &McpConfig {
        &self.mcp
    }

    /// Get mutable MCP configuration
    pub fn mcp_mut(&mut self) -> &mut McpConfig {
        &mut self.mcp
    }

    /// Get authentication configuration
    pub fn auth(&self) -> &AuthConfig {
        &self.auth
    }

    /// Get mutable authentication configuration
    pub fn auth_mut(&mut self) -> &mut AuthConfig {
        &mut self.auth
    }

    /// Export configuration to a JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails
    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| CortexError::Config(format!("Failed to export config to JSON: {}", e)))
    }

    /// Import configuration from a JSON string
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization or validation fails
    pub fn import_json(json: &str) -> Result<Self> {
        let config: Self = serde_json::from_str(json)
            .map_err(|e| CortexError::Config(format!("Failed to import config from JSON: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Export configuration to a TOML string
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails
    pub fn export_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| CortexError::Config(format!("Failed to export config to TOML: {}", e)))
    }

    /// Import configuration from a TOML string
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization or validation fails
    pub fn import_toml(toml_str: &str) -> Result<Self> {
        let config: Self = toml::from_str(toml_str)
            .map_err(|e| CortexError::Config(format!("Failed to import config from TOML: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Migrate configuration from an older version
    ///
    /// This method handles schema changes between configuration versions
    ///
    /// # Errors
    ///
    /// Returns an error if migration fails
    pub fn migrate(mut self) -> Result<Self> {
        let current_version = self.general.version.clone();

        if current_version == CONFIG_VERSION {
            debug!("Configuration is already at current version {}", CONFIG_VERSION);
            return Ok(self);
        }

        info!("Migrating configuration from {} to {}", current_version, CONFIG_VERSION);

        // Migration logic for different versions
        // This is where version-specific migrations would be implemented
        match current_version.as_str() {
            "0.0.1" => {
                // Example: migrate from 0.0.1 to 0.1.0
                debug!("Migrating from 0.0.1 to 0.1.0");
                // Add any new fields with defaults
                self.general.version = "0.1.0".to_string();
            }
            _ => {
                warn!("Unknown configuration version {}, using as-is", current_version);
            }
        }

        // Update to current version
        self.general.version = CONFIG_VERSION.to_string();

        // Validate after migration
        self.validate()?;

        info!("Configuration migration completed successfully");
        Ok(self)
    }

    /// Get configuration metadata
    pub fn metadata(&self) -> ConfigMetadata {
        ConfigMetadata {
            version: self.general.version.clone(),
            profile: self.profile,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Configuration metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: String,
    pub profile: ConfigProfile,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Thread-safe configuration manager with hot-reload support
pub struct ConfigManager {
    config: Arc<RwLock<GlobalConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: GlobalConfig, config_path: PathBuf) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }

    /// Get the global singleton instance
    ///
    /// Initializes the configuration on first access
    ///
    /// # Errors
    ///
    /// Returns an error if configuration cannot be loaded
    pub async fn global() -> Result<&'static ConfigManager> {
        static INSTANCE: OnceCell<ConfigManager> = OnceCell::new();

        if let Some(instance) = INSTANCE.get() {
            return Ok(instance);
        }

        let config_path = GlobalConfig::config_path()?;
        let config = GlobalConfig::load_or_create_default().await?;
        let manager = ConfigManager::new(config, config_path);

        INSTANCE.set(manager)
            .map_err(|_| CortexError::Config("Failed to initialize global config".to_string()))?;

        Ok(INSTANCE.get().unwrap())
    }

    /// Get read access to the configuration
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, GlobalConfig> {
        self.config.read().await
    }

    /// Get write access to the configuration
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, GlobalConfig> {
        self.config.write().await
    }

    /// Save the current configuration to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved
    pub async fn save(&self) -> Result<()> {
        let config = self.config.read().await;
        config.save_to_path(&self.config_path).await
    }

    /// Reload configuration from disk
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be loaded
    pub async fn reload(&self) -> Result<()> {
        let new_config = GlobalConfig::load_from_path(&self.config_path).await?;
        let mut config = self.config.write().await;
        *config = new_config;
        info!("Configuration reloaded from {}", self.config_path.display());
        Ok(())
    }

    /// Start hot-reload monitoring
    ///
    /// Periodically checks for configuration changes and reloads if modified
    ///
    /// # Errors
    ///
    /// Returns an error if monitoring cannot be started
    pub async fn start_hot_reload(self: Arc<Self>) -> Result<()> {
        let interval = {
            let config = self.config.read().await;
            if !config.general().hot_reload {
                return Ok(());
            }
            config.general().hot_reload_interval_secs
        };

        tokio::spawn(async move {
            let mut last_modified = std::fs::metadata(&self.config_path)
                .and_then(|m| m.modified())
                .ok();

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval));

            loop {
                interval.tick().await;

                if let Ok(metadata) = std::fs::metadata(&self.config_path) {
                    if let Ok(modified) = metadata.modified() {
                        if last_modified.map_or(true, |last| modified > last) {
                            if let Err(e) = self.reload().await {
                                warn!("Failed to reload configuration: {}", e);
                            } else {
                                last_modified = Some(modified);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Clone the current configuration
    pub async fn clone_config(&self) -> GlobalConfig {
        self.config.read().await.clone()
    }

    /// Update configuration with a closure
    ///
    /// # Errors
    ///
    /// Returns an error if the update function returns an error
    pub async fn update<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut GlobalConfig) -> Result<R>,
    {
        let mut config = self.config.write().await;
        f(&mut config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    /// Helper to create a temporary config environment
    fn create_temp_config_env() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        (temp_dir, config_path)
    }

    #[tokio::test]
    async fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.general.version, CONFIG_VERSION);
        assert_eq!(config.general.log_level, "info");
        assert_eq!(config.database.mode, "local");
        assert_eq!(config.pool.min_connections, 2);
        assert_eq!(config.pool.max_connections, 10);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let mut config = GlobalConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid log level
        config.general.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
        config.general.log_level = "info".to_string();

        // Invalid database mode
        config.database.mode = "invalid".to_string();
        assert!(config.validate().is_err());
        config.database.mode = "local".to_string();

        // Invalid pool config
        config.pool.min_connections = 20;
        config.pool.max_connections = 10;
        assert!(config.validate().is_err());
        config.pool.min_connections = 2;

        // Remote mode without URLs
        config.database.mode = "remote".to_string();
        config.database.remote_urls.clear();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_save_and_load_config() {
        let (_temp_dir, config_path) = create_temp_config_env();

        let mut config = GlobalConfig::default();
        config.general.log_level = "debug".to_string();
        config.database.namespace = "test".to_string();

        // Save config
        config.save_to_path(&config_path).await.unwrap();
        assert!(config_path.exists());

        // Load config
        let loaded_config = GlobalConfig::load_from_path(&config_path).await.unwrap();
        assert_eq!(loaded_config.general.log_level, "debug");
        assert_eq!(loaded_config.database.namespace, "test");
    }

    #[tokio::test]
    async fn test_atomic_save() {
        let (_temp_dir, config_path) = create_temp_config_env();

        let config = GlobalConfig::default();
        config.save_to_path(&config_path).await.unwrap();

        // Verify temp file was cleaned up
        let temp_path = config_path.with_extension("toml.tmp");
        assert!(!temp_path.exists());
        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_env_var_overrides() {
        let mut config = GlobalConfig::default();

        // Set environment variables
        unsafe {
            env::set_var(ENV_LOG_LEVEL, "debug");
            env::set_var(ENV_DB_MODE, "remote");
            env::set_var(ENV_DB_URL, "ws://localhost:8001");
            env::set_var(ENV_CACHE_SIZE_MB, "1024");
        }

        // Merge env vars
        config.merge_env_vars().unwrap();

        assert_eq!(config.general.log_level, "debug");
        assert_eq!(config.database.mode, "remote");
        assert_eq!(config.database.remote_urls, vec!["ws://localhost:8001"]);
        assert_eq!(config.cache.memory_size_mb, 1024);

        // Cleanup
        unsafe {
            env::remove_var(ENV_LOG_LEVEL);
            env::remove_var(ENV_DB_MODE);
            env::remove_var(ENV_DB_URL);
            env::remove_var(ENV_CACHE_SIZE_MB);
        }
    }

    #[tokio::test]
    async fn test_invalid_env_var() {
        let mut config = GlobalConfig::default();

        // Set invalid cache size
        unsafe {
            env::set_var(ENV_CACHE_SIZE_MB, "invalid");
        }

        let result = config.merge_env_vars();
        assert!(result.is_err());

        // Cleanup
        unsafe {
            env::remove_var(ENV_CACHE_SIZE_MB);
        }
    }

    #[tokio::test]
    async fn test_config_serialization() {
        let config = GlobalConfig::default();

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Verify all sections are present
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[database]"));
        assert!(toml_str.contains("[pool]"));
        assert!(toml_str.contains("[cache]"));
        assert!(toml_str.contains("[vfs]"));
        assert!(toml_str.contains("[ingestion]"));
        assert!(toml_str.contains("[mcp]"));

        // Deserialize back
        let deserialized: GlobalConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.general.version, config.general.version);
    }

    #[tokio::test]
    async fn test_partial_config_update() {
        let (_temp_dir, config_path) = create_temp_config_env();

        // Create and save initial config
        let config = GlobalConfig::default();
        config.save_to_path(&config_path).await.unwrap();

        // Load and modify
        let mut loaded_config = GlobalConfig::load_from_path(&config_path).await.unwrap();
        loaded_config.general.log_level = "trace".to_string();
        loaded_config.database.namespace = "updated".to_string();

        // Save updated config
        loaded_config.save_to_path(&config_path).await.unwrap();

        // Verify changes persisted
        let final_config = GlobalConfig::load_from_path(&config_path).await.unwrap();
        assert_eq!(final_config.general.log_level, "trace");
        assert_eq!(final_config.database.namespace, "updated");
        // Other fields should remain unchanged
        assert_eq!(final_config.pool.max_connections, 10);
    }

    #[tokio::test]
    async fn test_validation_before_save() {
        let (_temp_dir, config_path) = create_temp_config_env();

        let mut config = GlobalConfig::default();
        config.general.log_level = "invalid".to_string();

        // Save should fail due to validation
        let result = config.save_to_path(&config_path).await;
        assert!(result.is_err());
        assert!(!config_path.exists());
    }

    #[tokio::test]
    async fn test_accessor_methods() {
        let mut config = GlobalConfig::default();

        // Test immutable accessors
        assert_eq!(config.general().log_level, "info");
        assert_eq!(config.database().mode, "local");
        assert_eq!(config.pool().max_connections, 10);
        assert_eq!(config.cache().memory_size_mb, 512);
        assert_eq!(config.vfs().max_file_size_mb, 100);
        assert_eq!(config.ingestion().parallel_workers, 4);
        assert_eq!(config.mcp().cors_enabled, true);

        // Test mutable accessors
        config.general_mut().log_level = "debug".to_string();
        assert_eq!(config.general().log_level, "debug");

        config.database_mut().namespace = "test".to_string();
        assert_eq!(config.database().namespace, "test");
    }

    #[tokio::test]
    async fn test_config_profiles() {
        // Test dev profile
        let dev_config = GlobalConfig::with_profile(ConfigProfile::Dev);
        assert_eq!(dev_config.profile(), ConfigProfile::Dev);
        assert_eq!(dev_config.general().log_level, "debug");
        assert!(dev_config.general().hot_reload);
        assert_eq!(dev_config.pool().max_connections, 5);

        // Test prod profile
        let prod_config = GlobalConfig::with_profile(ConfigProfile::Prod);
        assert_eq!(prod_config.profile(), ConfigProfile::Prod);
        assert_eq!(prod_config.general().log_level, "info");
        assert!(!prod_config.general().hot_reload);
        assert_eq!(prod_config.pool().max_connections, 20);

        // Test test profile
        let test_config = GlobalConfig::with_profile(ConfigProfile::Test);
        assert_eq!(test_config.profile(), ConfigProfile::Test);
        assert_eq!(test_config.general().log_level, "warn");
        assert!(!test_config.general().hot_reload);
        assert_eq!(test_config.database().namespace, "cortex_test");
    }

    #[tokio::test]
    async fn test_profile_parsing() {
        assert_eq!("dev".parse::<ConfigProfile>().unwrap(), ConfigProfile::Dev);
        assert_eq!("development".parse::<ConfigProfile>().unwrap(), ConfigProfile::Dev);
        assert_eq!("prod".parse::<ConfigProfile>().unwrap(), ConfigProfile::Prod);
        assert_eq!("production".parse::<ConfigProfile>().unwrap(), ConfigProfile::Prod);
        assert_eq!("test".parse::<ConfigProfile>().unwrap(), ConfigProfile::Test);
        assert_eq!("testing".parse::<ConfigProfile>().unwrap(), ConfigProfile::Test);
        assert!("invalid".parse::<ConfigProfile>().is_err());
    }

    #[tokio::test]
    async fn test_export_import_json() {
        let config = GlobalConfig::default();

        // Export to JSON
        let json = config.export_json().unwrap();
        assert!(json.contains("\"general\""));
        assert!(json.contains("\"database\""));

        // Import from JSON
        let imported = GlobalConfig::import_json(&json).unwrap();
        assert_eq!(imported.general().version, config.general().version);
        assert_eq!(imported.database().mode, config.database().mode);
    }

    #[tokio::test]
    async fn test_export_import_toml() {
        let config = GlobalConfig::default();

        // Export to TOML
        let toml_str = config.export_toml().unwrap();
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[database]"));

        // Import from TOML
        let imported = GlobalConfig::import_toml(&toml_str).unwrap();
        assert_eq!(imported.general().version, config.general().version);
        assert_eq!(imported.database().mode, config.database().mode);
    }

    #[tokio::test]
    async fn test_invalid_import() {
        // Invalid JSON
        let result = GlobalConfig::import_json("invalid json");
        assert!(result.is_err());

        // Invalid TOML
        let result = GlobalConfig::import_toml("invalid = toml = syntax");
        assert!(result.is_err());

        // Valid syntax but invalid configuration
        let invalid_config = r#"
        [general]
        version = "0.1.0"
        log_level = "invalid_level"
        "#;
        let result = GlobalConfig::import_toml(invalid_config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_migration() {
        let mut config = GlobalConfig::default();
        config.general.version = "0.0.1".to_string();

        let migrated = config.migrate().unwrap();
        assert_eq!(migrated.general().version, CONFIG_VERSION);
    }

    #[tokio::test]
    async fn test_migration_already_current() {
        let config = GlobalConfig::default();
        let migrated = config.migrate().unwrap();
        assert_eq!(migrated.general().version, CONFIG_VERSION);
    }

    #[tokio::test]
    async fn test_config_metadata() {
        let config = GlobalConfig::default();
        let metadata = config.metadata();

        assert_eq!(metadata.version, CONFIG_VERSION);
        assert_eq!(metadata.profile, ConfigProfile::Dev);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let (_temp_dir, config_path) = create_temp_config_env();
        let config = GlobalConfig::default();

        let manager = ConfigManager::new(config.clone(), config_path.clone());

        // Test read access
        {
            let read_config = manager.read().await;
            assert_eq!(read_config.general().log_level, "info");
        }

        // Test write access
        {
            let mut write_config = manager.write().await;
            write_config.general_mut().log_level = "debug".to_string();
        }

        // Verify changes persisted
        {
            let read_config = manager.read().await;
            assert_eq!(read_config.general().log_level, "debug");
        }
    }

    #[tokio::test]
    async fn test_config_manager_save_reload() {
        let (_temp_dir, config_path) = create_temp_config_env();
        let mut config = GlobalConfig::default();
        config.general.log_level = "trace".to_string();

        let manager = ConfigManager::new(config, config_path.clone());

        // Save configuration
        manager.save().await.unwrap();
        assert!(config_path.exists());

        // Modify in-memory config
        {
            let mut write_config = manager.write().await;
            write_config.general_mut().log_level = "error".to_string();
        }

        // Reload should restore from disk
        manager.reload().await.unwrap();
        {
            let read_config = manager.read().await;
            assert_eq!(read_config.general().log_level, "trace");
        }
    }

    #[tokio::test]
    async fn test_config_manager_update() {
        let (_temp_dir, config_path) = create_temp_config_env();
        let config = GlobalConfig::default();
        let manager = ConfigManager::new(config, config_path);

        // Update with closure
        manager
            .update(|cfg| {
                cfg.general_mut().log_level = "warn".to_string();
                cfg.pool_mut().max_connections = 15;
                Ok(())
            })
            .await
            .unwrap();

        // Verify changes
        let read_config = manager.read().await;
        assert_eq!(read_config.general().log_level, "warn");
        assert_eq!(read_config.pool().max_connections, 15);
    }

    #[tokio::test]
    async fn test_directory_helpers() {
        // Test all directory path methods
        let base = GlobalConfig::base_dir().unwrap();
        let surrealdb = GlobalConfig::surrealdb_dir().unwrap();
        let cache = GlobalConfig::cache_dir().unwrap();
        let sessions = GlobalConfig::sessions_dir().unwrap();
        let temp = GlobalConfig::temp_dir().unwrap();
        let data = GlobalConfig::data_dir().unwrap();
        let logs = GlobalConfig::logs_dir().unwrap();
        let run = GlobalConfig::run_dir().unwrap();
        let workspaces = GlobalConfig::workspaces_dir().unwrap();

        // Verify they're all under base directory
        assert!(surrealdb.starts_with(&base));
        assert!(cache.starts_with(&base));
        assert!(sessions.starts_with(&base));
        assert!(temp.starts_with(&base));
        assert!(data.starts_with(&base));
        assert!(logs.starts_with(&base));
        assert!(run.starts_with(&base));
        assert!(workspaces.starts_with(&base));

        // Verify directory names
        assert!(surrealdb.ends_with("surrealdb"));
        assert!(cache.ends_with("cache"));
        assert!(sessions.ends_with("sessions"));
        assert!(temp.ends_with("temp"));
        assert!(data.ends_with("data"));
        assert!(logs.ends_with("logs"));
        assert!(run.ends_with("run"));
        assert!(workspaces.ends_with("workspaces"));
    }

    #[tokio::test]
    async fn test_profile_display() {
        assert_eq!(ConfigProfile::Dev.to_string(), "dev");
        assert_eq!(ConfigProfile::Prod.to_string(), "prod");
        assert_eq!(ConfigProfile::Test.to_string(), "test");
    }

    #[tokio::test]
    async fn test_hot_reload_disabled() {
        let (_temp_dir, config_path) = create_temp_config_env();
        let mut config = GlobalConfig::default();
        config.general.hot_reload = false;

        let manager = Arc::new(ConfigManager::new(config, config_path));

        // Should return Ok but not start monitoring
        let result = manager.clone().start_hot_reload().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clone_config() {
        let (_temp_dir, config_path) = create_temp_config_env();
        let config = GlobalConfig::default();
        let manager = ConfigManager::new(config.clone(), config_path);

        let cloned = manager.clone_config().await;
        assert_eq!(cloned.general().version, config.general().version);
        assert_eq!(cloned.profile(), config.profile());
    }
}
