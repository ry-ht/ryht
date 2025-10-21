//! Session-aware storage layer with copy-on-write semantics.
//!
//! This module provides namespace-isolated storage operations that respect
//! session boundaries and implement copy-on-write for data access.

use crate::session::{AgentSession, OperationType, SessionManager};
use cortex_core::error::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tracing::{debug, info, warn};

// ==============================================================================
// Session-Aware Storage Interface
// ==============================================================================

/// Session-aware storage that provides isolated data access
pub struct SessionAwareStorage {
    /// Database connection
    db: Arc<Surreal<Any>>,

    /// Session manager for session operations
    session_manager: Arc<SessionManager>,

    /// Main namespace
    main_namespace: String,

    /// Main database
    main_database: String,
}

impl SessionAwareStorage {
    /// Create a new session-aware storage instance
    pub fn new(
        db: Arc<Surreal<Any>>,
        session_manager: Arc<SessionManager>,
        main_namespace: String,
        main_database: String,
    ) -> Self {
        Self {
            db,
            session_manager,
            main_namespace,
            main_database,
        }
    }

    /// Store a code unit in session namespace with copy-on-write
    pub async fn store_unit_in_session(
        &self,
        session: &AgentSession,
        unit_id: &str,
        data: &impl Serialize,
    ) -> Result<()> {
        debug!("Storing unit {} in session {}", unit_id, session.id);

        // Serialize data to owned value
        let data_value = serde_json::to_value(data)
            .map_err(|e| CortexError::Serialization(e))?;

        let hash = Self::compute_hash(&data_value);

        // Switch to session namespace
        self.use_session_namespace(session).await?;

        // Store the unit
        self.db
            .query("UPSERT $unit_id CONTENT $data")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .bind(("data", data_value))
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to store unit: {}", e)))?;

        // Record the change
        self.session_manager
            .record_change(
                &session.id,
                unit_id.to_string(),
                OperationType::Modify,
                None,
                hash,
                HashMap::new(),
            )
            .await?;

        // Update statistics
        self.session_manager
            .update_statistics(&session.id, |stats| {
                stats.writes += 1;
                stats.updates += 1;
            })
            .await?;

        Ok(())
    }

    /// Get a code unit from session with copy-on-write fallback
    pub async fn get_unit_from_session<T: for<'de> Deserialize<'de>>(
        &self,
        session: &AgentSession,
        unit_id: &str,
    ) -> Result<T> {
        debug!("Getting unit {} from session {}", unit_id, session.id);

        // First try session namespace
        self.use_session_namespace(session).await?;

        let mut result = self
            .db
            .query("SELECT * FROM $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to query unit: {}", e)))?;

        let session_data: Option<T> = result
            .take(0)
            .map_err(|e| CortexError::Storage(format!("Failed to parse unit: {}", e)))?;

        if let Some(data) = session_data {
            // Update read statistics
            self.session_manager
                .update_statistics(&session.id, |stats| {
                    stats.reads += 1;
                })
                .await?;

            return Ok(data);
        }

        // Not found in session namespace - copy-on-write from main
        info!("Unit {} not in session, performing copy-on-write", unit_id);

        self.copy_unit_to_session(session, unit_id).await?;

        // Now retrieve from session
        self.use_session_namespace(session).await?;

        let mut result = self
            .db
            .query("SELECT * FROM $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to query copied unit: {}", e)))?;

        let data: Option<T> = result
            .take(0)
            .map_err(|e| CortexError::Storage(format!("Failed to parse copied unit: {}", e)))?;

        data.ok_or_else(|| CortexError::not_found("unit", unit_id))
    }

    /// Query units in session namespace with fallback to main
    pub async fn query_units_in_session<T: for<'de> Deserialize<'de>>(
        &self,
        session: &AgentSession,
        query: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<T>> {
        debug!("Querying units in session {}: {}", session.id, query);

        // Query session namespace
        self.use_session_namespace(session).await?;

        let mut db_query = self.db.query(query);
        for (key, value) in params.into_iter() {
            db_query = db_query.bind((key, value));
        }

        let mut result = db_query
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to execute query: {}", e)))?;

        let units: Vec<T> = result
            .take(0)
            .map_err(|e| CortexError::Storage(format!("Failed to parse query results: {}", e)))?;

        // Update statistics
        self.session_manager
            .update_statistics(&session.id, |stats| {
                stats.reads += units.len() as u64;
            })
            .await?;

        Ok(units)
    }

    /// Delete a unit from session namespace
    pub async fn delete_unit_from_session(
        &self,
        session: &AgentSession,
        unit_id: &str,
    ) -> Result<()> {
        debug!("Deleting unit {} from session {}", unit_id, session.id);

        // Check if session allows delete
        if !session.metadata.scope.allow_delete {
            return Err(CortexError::invalid_input(
                "Session does not have delete permissions",
            ));
        }

        // Switch to session namespace
        self.use_session_namespace(session).await?;

        // Delete the unit
        self.db
            .query("DELETE $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to delete unit: {}", e)))?;

        // Record the change
        self.session_manager
            .record_change(
                &session.id,
                unit_id.to_string(),
                OperationType::Delete,
                None,
                String::new(),
                HashMap::new(),
            )
            .await?;

        // Update statistics
        self.session_manager
            .update_statistics(&session.id, |stats| {
                stats.deletes += 1;
            })
            .await?;

        Ok(())
    }

    /// Create a new unit in session namespace
    pub async fn create_unit_in_session(
        &self,
        session: &AgentSession,
        unit_id: &str,
        data: &impl Serialize,
    ) -> Result<()> {
        debug!("Creating unit {} in session {}", unit_id, session.id);

        // Check if session allows create
        if !session.metadata.scope.allow_create {
            return Err(CortexError::invalid_input(
                "Session does not have create permissions",
            ));
        }

        // Serialize data to owned value
        let data_value = serde_json::to_value(data)
            .map_err(|e| CortexError::Serialization(e))?;

        let hash = Self::compute_hash(&data_value);

        // Switch to session namespace
        self.use_session_namespace(session).await?;

        // Create the unit
        let _: Option<serde_json::Value> = self.db
            .create(format!("code_unit:{}", unit_id))
            .content(data_value)
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to create unit: {}", e)))?;

        // Record the change
        self.session_manager
            .record_change(
                &session.id,
                unit_id.to_string(),
                OperationType::Create,
                None,
                hash,
                HashMap::new(),
            )
            .await?;

        // Update statistics
        self.session_manager
            .update_statistics(&session.id, |stats| {
                stats.creates += 1;
            })
            .await?;

        Ok(())
    }

    /// Check if a path is in session scope
    pub fn is_path_in_scope(&self, session: &AgentSession, path: &str) -> bool {
        let scope = &session.metadata.scope;

        // Check writable paths
        for pattern in &scope.paths {
            if Self::path_matches(path, pattern) {
                return true;
            }
        }

        // Check read-only paths
        for pattern in &scope.read_only_paths {
            if Self::path_matches(path, pattern) {
                return true;
            }
        }

        false
    }

    /// Check if a path is writable in session scope
    pub fn is_path_writable(&self, session: &AgentSession, path: &str) -> bool {
        let scope = &session.metadata.scope;

        // Must be in writable paths
        let in_writable = scope
            .paths
            .iter()
            .any(|pattern| Self::path_matches(path, pattern));

        // Must NOT be in read-only paths
        let in_readonly = scope
            .read_only_paths
            .iter()
            .any(|pattern| Self::path_matches(path, pattern));

        in_writable && !in_readonly
    }

    /// Batch copy multiple units to session namespace
    pub async fn batch_copy_to_session(
        &self,
        session: &AgentSession,
        unit_ids: &[String],
    ) -> Result<usize> {
        info!(
            "Batch copying {} units to session {}",
            unit_ids.len(),
            session.id
        );

        let mut copied = 0;
        for unit_id in unit_ids {
            match self.copy_unit_to_session(session, unit_id).await {
                Ok(_) => copied += 1,
                Err(e) => warn!("Failed to copy unit {}: {}", unit_id, e),
            }
        }

        Ok(copied)
    }

    // ==============================================================================
    // Private Helper Methods
    // ==============================================================================

    /// Switch database to session namespace
    async fn use_session_namespace(&self, session: &AgentSession) -> Result<()> {
        self.db
            .use_ns(&session.namespace)
            .use_db(&self.main_database)
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to switch namespace: {}", e)))
    }

    /// Switch database to main namespace
    async fn use_main_namespace(&self) -> Result<()> {
        self.db
            .use_ns(&self.main_namespace)
            .use_db(&self.main_database)
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to switch namespace: {}", e)))
    }

    /// Copy a single unit from main namespace to session namespace
    async fn copy_unit_to_session(&self, session: &AgentSession, unit_id: &str) -> Result<()> {
        debug!("Copying unit {} to session {}", unit_id, session.id);

        // Get from main namespace
        self.use_main_namespace().await?;

        let mut result = self
            .db
            .query("SELECT * FROM $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to query main unit: {}", e)))?;

        let unit_data: Option<serde_json::Value> = result
            .take(0)
            .map_err(|e| CortexError::Storage(format!("Failed to parse main unit: {}", e)))?;

        let unit_data = unit_data
            .ok_or_else(|| CortexError::not_found("unit", unit_id))?;

        let hash = Self::compute_hash(&unit_data);

        // Copy to session namespace
        self.use_session_namespace(session).await?;

        let _: Option<serde_json::Value> = self.db
            .create(format!("code_unit:{}", unit_id))
            .content(unit_data)
            .await
            .map_err(|e| CortexError::Storage(format!("Failed to copy unit to session: {}", e)))?;

        // Record copy-on-write operation
        self.session_manager
            .record_change(
                &session.id,
                unit_id.to_string(),
                OperationType::CopyOnWrite,
                None,
                hash,
                HashMap::new(),
            )
            .await?;

        // Update statistics
        self.session_manager
            .update_statistics(&session.id, |stats| {
                stats.cow_operations += 1;
            })
            .await?;

        Ok(())
    }

    /// Simple path pattern matching (supports wildcards)
    fn path_matches(path: &str, pattern: &str) -> bool {
        // Simple glob-style matching
        if pattern.ends_with("**") {
            // Prefix match for directory patterns like "src/**"
            let prefix = pattern.trim_end_matches("**");
            path.starts_with(prefix)
        } else if pattern.contains('*') {
            // Simple wildcard matching
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                path.starts_with(parts[0]) && path.ends_with(parts[1])
            } else {
                false
            }
        } else {
            // Exact match
            path == pattern
        }
    }

    /// Compute a simple hash of serialized data
    fn compute_hash(data: &impl Serialize) -> String {
        match serde_json::to_string(data) {
            Ok(json) => format!("{:x}", md5::compute(json.as_bytes())),
            Err(_) => String::from("unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matching() {
        assert!(SessionAwareStorage::path_matches("src/main.rs", "src/**"));
        assert!(SessionAwareStorage::path_matches("src/lib/mod.rs", "src/**"));
        assert!(!SessionAwareStorage::path_matches("tests/test.rs", "src/**"));

        assert!(SessionAwareStorage::path_matches("README.md", "*.md"));
        assert!(!SessionAwareStorage::path_matches("README.txt", "*.md"));

        assert!(SessionAwareStorage::path_matches("src/main.rs", "src/main.rs"));
        assert!(!SessionAwareStorage::path_matches("src/lib.rs", "src/main.rs"));
    }

    #[test]
    fn test_compute_hash() {
        #[derive(Serialize)]
        struct TestData {
            value: i32,
        }

        let data1 = TestData { value: 42 };
        let data2 = TestData { value: 42 };
        let data3 = TestData { value: 43 };

        let hash1 = SessionAwareStorage::compute_hash(&data1);
        let hash2 = SessionAwareStorage::compute_hash(&data2);
        let hash3 = SessionAwareStorage::compute_hash(&data3);

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
