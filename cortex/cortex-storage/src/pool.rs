//! Connection pooling for SurrealDB.

use crate::connection::ConnectionConfig;
use cortex_core::error::{CortexError, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

/// A connection pool for SurrealDB clients.
pub struct ConnectionPool {
    config: ConnectionConfig,
    connections: Arc<DashMap<usize, Arc<Surreal<Any>>>>,
    next_id: Arc<RwLock<usize>>,
    max_size: usize,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionConfig) -> Self {
        config.validate().expect("Invalid configuration");
        let max_size = config.pool_size;

        Self {
            config,
            connections: Arc::new(DashMap::new()),
            next_id: Arc::new(RwLock::new(0)),
            max_size,
        }
    }

    /// Initialize the pool with connections
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!(
            "Initializing connection pool with {} connections",
            self.max_size
        );

        for _ in 0..self.max_size {
            self.create_connection().await?;
        }

        Ok(())
    }

    /// Create a new connection
    async fn create_connection(&self) -> Result<Arc<Surreal<Any>>> {
        let conn_str = self.config.connection_string()?;

        tracing::debug!("Creating new SurrealDB connection: {}", conn_str);

        let db = surrealdb::engine::any::connect(conn_str)
            .await
            .map_err(|e| CortexError::database(format!("Failed to connect: {}", e)))?;

        // Use namespace and database
        db.use_ns(&self.config.namespace)
            .use_db(&self.config.database)
            .await
            .map_err(|e| CortexError::database(format!("Failed to use namespace/database: {}", e)))?;

        // Authenticate if credentials are provided
        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            db.signin(surrealdb::opt::auth::Root {
                username,
                password,
            })
            .await
            .map_err(|e| CortexError::database(format!("Authentication failed: {}", e)))?;
        }

        let db = Arc::new(db);
        let mut id = self.next_id.write();
        self.connections.insert(*id, db.clone());
        *id += 1;

        Ok(db)
    }

    /// Get a connection from the pool
    pub async fn get(&self) -> Result<Arc<Surreal<Any>>> {
        // Try to get an existing connection
        if let Some(entry) = self.connections.iter().next() {
            return Ok(entry.value().clone());
        }

        // Create a new connection if pool is not full
        if self.connections.len() < self.max_size {
            return self.create_connection().await;
        }

        Err(CortexError::internal("Connection pool exhausted"))
    }

    /// Get the current pool size
    pub fn size(&self) -> usize {
        self.connections.len()
    }

    /// Get the maximum pool size
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Close all connections
    pub async fn close(&self) {
        tracing::info!("Closing connection pool");
        self.connections.clear();
    }
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        tracing::debug!("Connection pool dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = ConnectionConfig::memory().with_pool_size(5);
        let pool = ConnectionPool::new(config);
        assert_eq!(pool.max_size(), 5);
    }

    #[tokio::test]
    async fn test_pool_initialization() {
        let config = ConnectionConfig::memory().with_pool_size(2);
        let pool = ConnectionPool::new(config);
        pool.initialize().await.unwrap();
        assert_eq!(pool.size(), 2);
    }

    #[tokio::test]
    async fn test_get_connection() {
        let config = ConnectionConfig::memory();
        let pool = ConnectionPool::new(config);
        pool.initialize().await.unwrap();

        let conn = pool.get().await.unwrap();
        assert!(Arc::strong_count(&conn) >= 1);
    }
}
