//! Error types for the Cortex system.

/// Result type alias for Cortex operations.
pub type Result<T> = std::result::Result<T, CortexError>;

/// Main error type for the Cortex system.
#[derive(Debug, thiserror::Error)]
pub enum CortexError {
    /// Storage layer errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Database connection errors
    #[error("Database error: {0}")]
    Database(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Not found errors
    #[error("Not found: {resource} with id {id}")]
    NotFound { resource: String, id: String },

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Virtual filesystem errors
    #[error("VFS error: {0}")]
    Vfs(String),

    /// Memory system errors
    #[error("Memory error: {0}")]
    Memory(String),

    /// Ingestion errors
    #[error("Ingestion error: {0}")]
    Ingestion(String),

    /// MCP protocol errors
    #[error("MCP error: {0}")]
    Mcp(String),

    /// Concurrent access errors
    #[error("Concurrent access error: {0}")]
    Concurrency(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Deadlock errors
    #[error("Deadlock detected: {0}")]
    Deadlock(String),

    /// Generic internal errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Wrapped anyhow errors for compatibility
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl CortexError {
    /// Create a new storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a new database error
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    /// Create a new not found error
    pub fn not_found(resource: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            id: id.into(),
        }
    }

    /// Create a new invalid input error
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput(msg.into())
    }

    /// Create a new config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new VFS error
    pub fn vfs(msg: impl Into<String>) -> Self {
        Self::Vfs(msg.into())
    }

    /// Create a new memory error
    pub fn memory(msg: impl Into<String>) -> Self {
        Self::Memory(msg.into())
    }

    /// Create a new ingestion error
    pub fn ingestion(msg: impl Into<String>) -> Self {
        Self::Ingestion(msg.into())
    }

    /// Create a new MCP error
    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    /// Create a new concurrency error
    pub fn concurrency(msg: impl Into<String>) -> Self {
        Self::Concurrency(msg.into())
    }

    /// Create a new timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create a new deadlock error
    pub fn deadlock(msg: impl Into<String>) -> Self {
        Self::Deadlock(msg.into())
    }

    /// Create a new internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create a new connection error (maps to Internal)
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Internal(format!("Connection error: {}", msg.into()))
    }

    /// Create a new serialization error from a string message
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Internal(format!("Serialization error: {}", msg.into()))
    }

    /// Create a new migration error (maps to Internal)
    pub fn migration(msg: impl Into<String>) -> Self {
        Self::Internal(format!("Migration error: {}", msg.into()))
    }

    /// Check if this is a not found error
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Check if this is a storage error
    pub fn is_storage(&self) -> bool {
        matches!(self, Self::Storage(_))
    }

    /// Check if this is a database error
    pub fn is_database(&self) -> bool {
        matches!(self, Self::Database(_))
    }
}
