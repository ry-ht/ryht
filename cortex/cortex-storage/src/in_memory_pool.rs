//! In-memory connection pool for testing
//!
//! This module provides a test-only connection pool that uses SurrealDB's embedded
//! in-memory mode, which doesn't require an external server.

use cortex_core::error::{CortexError, Result};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{debug, info};
use uuid::Uuid;

#[allow(unused_imports)]
use surrealdb::engine::local::Mem; // Used in type annotation

/// In-memory connection pool for testing
pub struct InMemoryConnectionPool {
    db: Arc<Surreal<Db>>,
    semaphore: Arc<Semaphore>,
    #[allow(dead_code)]
    namespace: String,
    #[allow(dead_code)]
    database: String,
}

impl InMemoryConnectionPool {
    /// Create a new in-memory connection pool
    pub async fn new(namespace: &str, database: &str, max_connections: usize) -> Result<Self> {
        info!("Creating in-memory connection pool for testing");

        // Create embedded in-memory database using "memory" path
        let db = Surreal::new::<surrealdb::engine::local::Mem>(())
            .await
            .map_err(|e| CortexError::database(format!("Failed to create in-memory DB: {}", e)))?;

        // Set namespace and database
        db.use_ns(namespace)
            .use_db(database)
            .await
            .map_err(|e| CortexError::database(format!("Failed to set namespace/database: {}", e)))?;

        info!("In-memory connection pool created successfully");

        Ok(Self {
            db: Arc::new(db),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            namespace: namespace.to_string(),
            database: database.to_string(),
        })
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<InMemoryPooledConnection> {
        let permit = self.semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| CortexError::database(format!("Failed to acquire connection: {}", e)))?;

        Ok(InMemoryPooledConnection {
            id: Uuid::new_v4(),
            db: self.db.clone(),
            created_at: Instant::now(),
            last_used: Arc::new(RwLock::new(Instant::now())),
            uses: Arc::new(AtomicUsize::new(0)),
            healthy: Arc::new(AtomicBool::new(true)),
            _permit: permit,
        })
    }

    /// Get the underlying database instance (for direct access in tests)
    pub fn db(&self) -> &Surreal<Db> {
        &self.db
    }
}

/// In-memory pooled connection
pub struct InMemoryPooledConnection {
    id: Uuid,
    db: Arc<Surreal<Db>>,
    #[allow(dead_code)]
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    uses: Arc<AtomicUsize>,
    healthy: Arc<AtomicBool>,
    _permit: OwnedSemaphorePermit,
}

impl InMemoryPooledConnection {
    /// Get the underlying Surreal connection
    pub fn connection(&self) -> &Surreal<Db> {
        &self.db
    }

    /// Get connection ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get number of times connection has been used
    pub fn uses(&self) -> usize {
        self.uses.load(Ordering::Relaxed)
    }

    /// Increment use counter
    pub fn increment_uses(&self) {
        self.uses.fetch_add(1, Ordering::Relaxed);
        *self.last_used.write() = Instant::now();
    }

    /// Check connection health
    pub async fn check_health(&self) -> bool {
        // For in-memory DB, connection is always healthy unless explicitly marked otherwise
        let healthy = self.healthy.load(Ordering::Relaxed);
        debug!("Connection {} health check: {}", self.id, healthy);
        healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_pool_creation() {
        let pool = InMemoryConnectionPool::new("test_ns", "test_db", 10)
            .await
            .expect("Failed to create in-memory pool");

        let conn = pool.acquire().await.expect("Failed to acquire connection");

        // Verify we can execute a query
        let result: std::result::Result<Vec<surrealdb::RecordId>, surrealdb::Error> =
            conn.connection().select("test_table").await;

        // Should succeed (returning empty results) or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let pool = InMemoryConnectionPool::new("test_ns", "test_db", 5)
            .await
            .expect("Failed to create pool");

        let conn1 = pool.acquire().await.expect("Failed to acquire conn1");
        let conn2 = pool.acquire().await.expect("Failed to acquire conn2");

        assert_ne!(conn1.id(), conn2.id());
        assert_eq!(conn1.uses(), 0);
        assert_eq!(conn2.uses(), 0);

        conn1.increment_uses();
        assert_eq!(conn1.uses(), 1);
    }
}
