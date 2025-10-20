//! Connection configuration for SurrealDB.

use cortex_core::error::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Connection mode for SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionMode {
    /// In-memory database (for testing)
    Memory,
    /// RocksDB file storage
    RocksDb { path: PathBuf },
    /// Remote server
    Remote { endpoint: String },
}

/// Configuration for SurrealDB connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub mode: ConnectionMode,
    pub namespace: String,
    pub database: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pool_size: usize,
}

impl ConnectionConfig {
    /// Create a new in-memory configuration (for testing)
    pub fn memory() -> Self {
        Self {
            mode: ConnectionMode::Memory,
            namespace: "cortex".to_string(),
            database: "main".to_string(),
            username: None,
            password: None,
            pool_size: 10,
        }
    }

    /// Create a new RocksDB configuration
    pub fn rocksdb(path: PathBuf) -> Self {
        Self {
            mode: ConnectionMode::RocksDb { path },
            namespace: "cortex".to_string(),
            database: "main".to_string(),
            username: None,
            password: None,
            pool_size: 10,
        }
    }

    /// Create a new remote configuration
    pub fn remote(endpoint: String) -> Self {
        Self {
            mode: ConnectionMode::Remote { endpoint },
            namespace: "cortex".to_string(),
            database: "main".to_string(),
            username: None,
            password: None,
            pool_size: 10,
        }
    }

    /// Set the namespace
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
        self
    }

    /// Set the database name
    pub fn with_database(mut self, database: String) -> Self {
        self.database = database;
        self
    }

    /// Set authentication credentials
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(password);
        self
    }

    /// Set the connection pool size
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    /// Get the connection string for SurrealDB
    pub fn connection_string(&self) -> Result<String> {
        match &self.mode {
            ConnectionMode::Memory => Ok("mem://".to_string()),
            ConnectionMode::RocksDb { path } => {
                let path_str = path
                    .to_str()
                    .ok_or_else(|| CortexError::config("Invalid path for RocksDB"))?;
                Ok(format!("rocksdb://{}", path_str))
            }
            ConnectionMode::Remote { endpoint } => Ok(endpoint.clone()),
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.namespace.is_empty() {
            return Err(CortexError::config("Namespace cannot be empty"));
        }
        if self.database.is_empty() {
            return Err(CortexError::config("Database name cannot be empty"));
        }
        if self.pool_size == 0 {
            return Err(CortexError::config("Pool size must be greater than 0"));
        }
        Ok(())
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self::memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config() {
        let config = ConnectionConfig::memory();
        assert!(matches!(config.mode, ConnectionMode::Memory));
        assert_eq!(config.namespace, "cortex");
        assert_eq!(config.database, "main");
    }

    #[test]
    fn test_connection_string() {
        let config = ConnectionConfig::memory();
        assert_eq!(config.connection_string().unwrap(), "mem://");

        let config = ConnectionConfig::rocksdb(PathBuf::from("/tmp/cortex.db"));
        assert!(config.connection_string().unwrap().starts_with("rocksdb://"));
    }

    #[test]
    fn test_validation() {
        let config = ConnectionConfig::memory();
        assert!(config.validate().is_ok());

        let mut config = ConnectionConfig::memory();
        config.namespace = String::new();
        assert!(config.validate().is_err());
    }
}
