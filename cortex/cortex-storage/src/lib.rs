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
pub mod session;
pub mod merge;
pub mod merge_engine;
pub mod session_aware_storage;
pub mod locks;

// In-memory pool for testing (available in all builds for integration tests)
pub mod in_memory_pool;

pub use connection::ConnectionConfig;
pub use surreal::SurrealStorage;
pub use pool::ConnectionPool;
pub use surrealdb_manager::{SurrealDBConfig, SurrealDBManager, ServerStatus};

// Re-export production-ready connection pool types
pub use connection_pool::{
    CircuitBreakerState, ConnectionManager, ConnectionMode as PoolConnectionMode,
    Credentials, DatabaseConfig, HealthStatus, LoadBalancingStrategy, MetricsSnapshot,
    PoolConfig, PoolMetrics, PooledConnection, PoolStatistics, ResourceLimits, RetryPolicy,
    Transaction, TransactionOperation, TransactionStatus,
};

// Re-export session management types (with aliases to avoid conflicts)
pub use session::{
    AgentSession, ChangeRecord, ConflictType as SessionConflictType, IsolationLevel,
    MergeConflict as SessionMergeConflict, MergeResult as SessionMergeResult,
    OperationType, ResolutionStrategy, SessionId, SessionManager, SessionMetadata, SessionScope,
    SessionState, SessionStatistics, WorkspaceId,
};

// Re-export merge types
pub use merge::{
    Change, ChangeSet, Conflict, ConflictType, DiffEngine, Hunk,
    MergeRequest, MergeResult, MergeStrategy, MergedEntity,
    Operation, ResolutionType, SemanticAnalyzer, VerificationResult,
};

pub use merge_engine::MergeEngine;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::connection::{ConnectionConfig, ConnectionMode};
    pub use crate::surreal::SurrealStorage;
    pub use crate::pool::ConnectionPool;
    pub use crate::surrealdb_manager::{SurrealDBConfig, SurrealDBManager, ServerStatus};

    // Production-ready connection pool
    pub use crate::connection_pool::{
        CircuitBreakerState, ConnectionManager,
        ConnectionMode as PoolConnectionMode, Credentials, DatabaseConfig, HealthStatus,
        LoadBalancingStrategy, MetricsSnapshot, PoolConfig, PoolMetrics, PooledConnection,
        PoolStatistics, ResourceLimits, RetryPolicy, Transaction,
        TransactionOperation, TransactionStatus,
    };

    // Session management
    pub use crate::session::{
        AgentSession, ChangeRecord, ConflictType as SessionConflictType, IsolationLevel,
        MergeConflict as SessionMergeConflict, MergeResult as SessionMergeResult,
        OperationType, ResolutionStrategy, SessionId, SessionManager, SessionMetadata,
        SessionScope, SessionState, SessionStatistics, WorkspaceId,
    };

    // Merge operations
    pub use crate::merge::{
        Change, ChangeSet, Conflict, ConflictType, DiffEngine, Hunk,
        MergeRequest, MergeResult, MergeStrategy, MergedEntity,
        Operation, ResolutionType, SemanticAnalyzer, VerificationResult,
    };
    pub use crate::merge_engine::MergeEngine;
}
