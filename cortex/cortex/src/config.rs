//! Configuration management for Cortex CLI.
//!
//! NOTE: This module is DEPRECATED. Cortex now uses the unified configuration
//! system from cortex-core, which stores config at ~/.ryht/config.toml.
//!
//! This module is kept for backward compatibility with legacy code that may
//! still reference it, but new code should use cortex_core::config::GlobalConfig
//! directly.
//!
//! This module handles configuration loading from multiple sources:
//! 1. Default values
//! 2. System-wide config (~/.ryht/config.toml) - UNIFIED CONFIG
//! 3. Project-specific config (.cortex/config.toml) - DEPRECATED
//! 4. Environment variables (CORTEX_*)
//! 5. Command-line flags
//!
//! Priority: CLI flags > ENV vars > Project config > System config > Defaults

use cortex_core::error::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cortex CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    /// Database configuration
    pub database: DatabaseConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// MCP server configuration
    pub mcp: McpConfig,

    /// Active workspace (DEPRECATED - use session-based workspace selection)
    /// This field is kept for backward compatibility but should not be used
    #[serde(skip_serializing_if = "Option::is_none")]
    #[deprecated(note = "Use session-based workspace selection instead")]
    pub active_workspace: Option<String>,

    /// Default workspace for new sessions (if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_workspace: Option<String>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection string (CORTEX_DB_URL)
    pub connection_string: String,

    /// Database namespace (CORTEX_DB_NAMESPACE)
    pub namespace: String,

    /// Database name (CORTEX_DB_NAME)
    pub database: String,

    /// Connection pool size (CORTEX_DB_POOL_SIZE)
    pub pool_size: usize,

    /// Username for authentication (CORTEX_DB_USER)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Password for authentication (CORTEX_DB_PASSWORD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory (CORTEX_DATA_DIR)
    pub data_dir: PathBuf,

    /// Cache size in MB (CORTEX_CACHE_SIZE_MB)
    pub cache_size_mb: usize,

    /// Enable compression (CORTEX_COMPRESSION)
    pub compression_enabled: bool,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Enable MCP server (CORTEX_MCP_ENABLED)
    pub enabled: bool,

    /// Server address (CORTEX_MCP_ADDRESS)
    pub address: String,

    /// Server port (CORTEX_MCP_PORT)
    pub port: u16,
}

impl Default for CortexConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cortex");

        Self {
            database: DatabaseConfig {
                connection_string: "ws://127.0.0.1:8000".to_string(),
                namespace: "cortex".to_string(),
                database: "main".to_string(),
                pool_size: 10,
                username: Some("root".to_string()),
                password: Some("root".to_string()),
            },
            storage: StorageConfig {
                data_dir,
                cache_size_mb: 1024,
                compression_enabled: true,
            },
            mcp: McpConfig {
                enabled: true,
                address: "127.0.0.1".to_string(),
                port: 3000,
            },
            #[allow(deprecated)]
            active_workspace: None,
            default_workspace: None,
        }
    }
}

impl CortexConfig {
    /// Get the default config file path
    ///
    /// DEPRECATED: Use cortex_core::config::GlobalConfig::config_path() instead.
    /// This now points to the unified config at ~/.ryht/config.toml for compatibility.
    pub fn default_path() -> Result<PathBuf> {
        // Use the unified config path from cortex-core
        cortex_core::config::GlobalConfig::config_path()
            .map_err(|e| CortexError::config(format!("Failed to get config path: {}", e)))
    }

    /// Get the project-specific config path
    pub fn project_path() -> PathBuf {
        PathBuf::from(".cortex").join("config.toml")
    }

    /// Load configuration with full priority chain
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Try to load system-wide config
        if let Ok(system_path) = Self::default_path() {
            if system_path.exists() {
                if let Ok(system_config) = Self::from_file(&system_path) {
                    config = Self::merge(config, system_config);
                }
            }
        }

        // Try to load project-specific config
        let project_path = Self::project_path();
        if project_path.exists() {
            if let Ok(project_config) = Self::from_file(&project_path) {
                config = Self::merge(config, project_config);
            }
        }

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| CortexError::config(format!("Failed to read config: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| CortexError::config(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CortexError::config(format!("Failed to create config dir: {}", e)))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| CortexError::config(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| CortexError::config(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Save to default system-wide config location
    pub fn save_default(&self) -> Result<()> {
        let path = Self::default_path()?;
        self.save(&path)
    }

    /// Save to project-specific config location
    pub fn save_project(&self) -> Result<()> {
        let path = Self::project_path();
        self.save(&path)
    }

    /// Merge two configurations (second takes precedence)
    fn merge(mut base: Self, overlay: Self) -> Self {
        // Merge database config
        if overlay.database.connection_string != base.database.connection_string {
            base.database.connection_string = overlay.database.connection_string;
        }
        if overlay.database.namespace != base.database.namespace {
            base.database.namespace = overlay.database.namespace;
        }
        if overlay.database.database != base.database.database {
            base.database.database = overlay.database.database;
        }
        if overlay.database.pool_size != base.database.pool_size {
            base.database.pool_size = overlay.database.pool_size;
        }
        if overlay.database.username.is_some() {
            base.database.username = overlay.database.username;
        }
        if overlay.database.password.is_some() {
            base.database.password = overlay.database.password;
        }

        // Merge storage config
        if overlay.storage.data_dir != base.storage.data_dir {
            base.storage.data_dir = overlay.storage.data_dir;
        }
        if overlay.storage.cache_size_mb != base.storage.cache_size_mb {
            base.storage.cache_size_mb = overlay.storage.cache_size_mb;
        }
        base.storage.compression_enabled = overlay.storage.compression_enabled;

        // Merge MCP config
        base.mcp.enabled = overlay.mcp.enabled;
        if overlay.mcp.address != base.mcp.address {
            base.mcp.address = overlay.mcp.address;
        }
        if overlay.mcp.port != base.mcp.port {
            base.mcp.port = overlay.mcp.port;
        }

        // Merge workspace
        #[allow(deprecated)]
        {
            if overlay.active_workspace.is_some() {
                base.active_workspace = overlay.active_workspace;
            }
        }
        if overlay.default_workspace.is_some() {
            base.default_workspace = overlay.default_workspace;
        }

        base
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("CORTEX_DB_URL") {
            self.database.connection_string = val;
        }
        if let Ok(val) = std::env::var("CORTEX_DB_NAMESPACE") {
            self.database.namespace = val;
        }
        if let Ok(val) = std::env::var("CORTEX_DB_NAME") {
            self.database.database = val;
        }
        if let Ok(val) = std::env::var("CORTEX_DB_POOL_SIZE") {
            if let Ok(size) = val.parse() {
                self.database.pool_size = size;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_DB_USER") {
            self.database.username = Some(val);
        }
        if let Ok(val) = std::env::var("CORTEX_DB_PASSWORD") {
            self.database.password = Some(val);
        }

        if let Ok(val) = std::env::var("CORTEX_DATA_DIR") {
            self.storage.data_dir = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("CORTEX_CACHE_SIZE_MB") {
            if let Ok(size) = val.parse() {
                self.storage.cache_size_mb = size;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_COMPRESSION") {
            self.storage.compression_enabled = val.parse().unwrap_or(true);
        }

        if let Ok(val) = std::env::var("CORTEX_MCP_ENABLED") {
            self.mcp.enabled = val.parse().unwrap_or(true);
        }
        if let Ok(val) = std::env::var("CORTEX_MCP_ADDRESS") {
            self.mcp.address = val;
        }
        if let Ok(val) = std::env::var("CORTEX_MCP_PORT") {
            if let Ok(port) = val.parse() {
                self.mcp.port = port;
            }
        }

        // Keep for backward compatibility
        #[allow(deprecated)]
        if let Ok(val) = std::env::var("CORTEX_WORKSPACE") {
            self.active_workspace = Some(val.clone());
            // Also set as default if default is not set
            if self.default_workspace.is_none() {
                self.default_workspace = Some(val);
            }
        }

        if let Ok(val) = std::env::var("CORTEX_DEFAULT_WORKSPACE") {
            self.default_workspace = Some(val);
        }
    }

    /// Get a specific config value
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "database.connection_string" => Some(self.database.connection_string.clone()),
            "database.namespace" => Some(self.database.namespace.clone()),
            "database.database" => Some(self.database.database.clone()),
            "database.pool_size" => Some(self.database.pool_size.to_string()),
            "storage.data_dir" => Some(self.storage.data_dir.display().to_string()),
            "storage.cache_size_mb" => Some(self.storage.cache_size_mb.to_string()),
            "storage.compression_enabled" => Some(self.storage.compression_enabled.to_string()),
            "mcp.enabled" => Some(self.mcp.enabled.to_string()),
            "mcp.address" => Some(self.mcp.address.clone()),
            "mcp.port" => Some(self.mcp.port.to_string()),
            "active_workspace" => self.active_workspace.clone(),
            _ => None,
        }
    }

    /// Set a specific config value
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "database.connection_string" => {
                self.database.connection_string = value.to_string();
            }
            "database.namespace" => {
                self.database.namespace = value.to_string();
            }
            "database.database" => {
                self.database.database = value.to_string();
            }
            "database.pool_size" => {
                self.database.pool_size = value.parse()
                    .map_err(|_| CortexError::config("Invalid pool size"))?;
            }
            "storage.data_dir" => {
                self.storage.data_dir = PathBuf::from(value);
            }
            "storage.cache_size_mb" => {
                self.storage.cache_size_mb = value.parse()
                    .map_err(|_| CortexError::config("Invalid cache size"))?;
            }
            "storage.compression_enabled" => {
                self.storage.compression_enabled = value.parse()
                    .map_err(|_| CortexError::config("Invalid boolean value"))?;
            }
            "mcp.enabled" => {
                self.mcp.enabled = value.parse()
                    .map_err(|_| CortexError::config("Invalid boolean value"))?;
            }
            "mcp.address" => {
                self.mcp.address = value.to_string();
            }
            "mcp.port" => {
                self.mcp.port = value.parse()
                    .map_err(|_| CortexError::config("Invalid port number"))?;
            }
            "active_workspace" => {
                self.active_workspace = Some(value.to_string());
            }
            _ => {
                return Err(CortexError::config(format!("Unknown config key: {}", key)));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CortexConfig::default();
        assert_eq!(config.database.namespace, "cortex");
        assert_eq!(config.database.database, "main");
        assert_eq!(config.database.pool_size, 10);
        assert_eq!(config.storage.cache_size_mb, 1024);
        assert!(config.storage.compression_enabled);
        assert!(config.mcp.enabled);
        assert_eq!(config.mcp.port, 3000);
    }

    #[test]
    fn test_get_set() {
        let mut config = CortexConfig::default();

        assert_eq!(config.get("database.namespace"), Some("cortex".to_string()));

        config.set("database.namespace", "test").unwrap();
        assert_eq!(config.get("database.namespace"), Some("test".to_string()));

        config.set("mcp.port", "4000").unwrap();
        assert_eq!(config.get("mcp.port"), Some("4000".to_string()));
    }

    #[test]
    fn test_invalid_key() {
        let config = CortexConfig::default();
        assert_eq!(config.get("invalid.key"), None);
    }
}
