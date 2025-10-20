//! Storage layer for Cortex using SurrealDB.
//!
//! This crate provides connection pooling, query execution, and data persistence
//! for the Cortex cognitive memory system.

pub mod connection;
pub mod surreal;
pub mod pool;
pub mod query;
pub mod schema;
pub mod surrealdb_manager;
pub mod connection_pool;

pub use connection::ConnectionConfig;
pub use surreal::SurrealStorage;
pub use pool::ConnectionPool;
pub use surrealdb_manager::{SurrealDBConfig, SurrealDBManager, ServerStatus};

// Re-export production-ready connection pool types
pub use connection_pool::{
    AgentSession, CircuitBreakerState, ConnectionManager, ConnectionMode as PoolConnectionMode,
    Credentials, DatabaseConfig, HealthStatus, LoadBalancingStrategy, MetricsSnapshot,
    PoolConfig, PoolMetrics, PooledConnection, PoolStatistics, ResourceLimits, RetryPolicy,
    SessionStatistics, Transaction, TransactionOperation, TransactionStatus,
};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::connection::{ConnectionConfig, ConnectionMode};
    pub use crate::surreal::SurrealStorage;
    pub use crate::pool::ConnectionPool;
    pub use crate::surrealdb_manager::{SurrealDBConfig, SurrealDBManager, ServerStatus};

    // Production-ready connection pool
    pub use crate::connection_pool::{
        AgentSession, CircuitBreakerState, ConnectionManager,
        ConnectionMode as PoolConnectionMode, Credentials, DatabaseConfig, HealthStatus,
        LoadBalancingStrategy, MetricsSnapshot, PoolConfig, PoolMetrics, PooledConnection,
        PoolStatistics, ResourceLimits, RetryPolicy, SessionStatistics, Transaction,
        TransactionOperation, TransactionStatus,
    };
}
