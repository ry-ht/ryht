//! Test helper utilities for E2E tests
//!
//! This module provides test utilities that use SurrealDB's embedded in-memory mode
//! instead of requiring an external server.

use cortex_core::error::{CortexError, Result};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// NOTE: This module is currently disabled due to SurrealDB API changes in version 2.3.10
// The `engine::local::Mem` API has changed and requires refactoring to support the new API.
// For now, tests should use the production ConnectionManager with a real database instance.

/// Placeholder: Test-only ConnectionManager that uses embedded in-memory SurrealDB
///
/// This is currently disabled due to SurrealDB API changes.
/// Tests should use cortex_storage::ConnectionManager directly.
#[allow(dead_code)]
pub struct TestConnectionManager {
    namespace: String,
    database: String,
}

#[allow(dead_code)]
impl TestConnectionManager {
    /// Create a new test connection manager
    ///
    /// NOTE: This is currently a placeholder and will panic if used.
    /// Use cortex_storage::ConnectionManager instead.
    pub async fn new(namespace: &str, database: &str) -> Result<Self> {
        panic!("TestConnectionManager is disabled. Use cortex_storage::ConnectionManager with a real database instance.")
    }
}

// Tests disabled - use cortex_storage::ConnectionManager directly in integration tests
