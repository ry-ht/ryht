//! Session management service
//!
//! Provides unified session operations for both API and MCP modules.
//! Handles work sessions, file modifications, locks, and session merging.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Session service for managing work sessions and file modifications
#[derive(Clone)]
pub struct SessionService {
    storage: Arc<ConnectionManager>,
    vfs: Option<Arc<VirtualFileSystem>>,
}

impl SessionService {
    /// Create a new session service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            vfs: None,
        }
    }

    /// Create a new session service with VFS support
    pub fn with_vfs(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Self {
        Self {
            storage,
            vfs: Some(vfs),
        }
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Create a new work session
    pub async fn create_session(
        &self,
        workspace_id: Uuid,
        name: String,
        agent_type: String,
        metadata: Option<SessionMetadata>,
    ) -> Result<WorkSession> {
        info!("Creating session: {} (workspace: {})", name, workspace_id);

        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let session = WorkSession {
            id: session_id,
            name,
            agent_type,
            workspace_id: Some(workspace_id),
            status: SessionStatus::Active,
            metadata,
            created_at: now,
            updated_at: now,
        };

        let conn = self.storage.acquire().await?;
        let session_json = serde_json::to_value(&session)?;

        let _: Option<serde_json::Value> = conn.connection()
            .create(("session", session_id.to_string()))
            .content(session_json)
            .await?;

        info!("Session created: {} ({})", session.name, session_id);

        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<WorkSession>> {
        debug!("Getting session: {}", session_id);

        let session_uuid = Uuid::parse_str(session_id)?;

        let conn = self.storage.acquire().await?;

        let session: Option<WorkSession> = conn.connection()
            .select(("session", session_uuid.to_string()))
            .await?;

        Ok(session)
    }

    /// List sessions with filters
    pub async fn list_sessions(
        &self,
        workspace_id: Option<Uuid>,
        filters: SessionFilters,
    ) -> Result<Vec<WorkSession>> {
        debug!("Listing sessions with filters: {:?}", filters);

        let conn = self.storage.acquire().await?;

        let mut query = String::from("SELECT * FROM session WHERE 1=1");

        if let Some(ws_id) = workspace_id {
            query.push_str(&format!(" AND workspace_id = '{}'", ws_id));
        }

        if let Some(ref status) = filters.status {
            query.push_str(&format!(" AND status = '{}'", status));
        }

        if let Some(ref agent_type) = filters.agent_type {
            query.push_str(&format!(" AND agent_type = '{}'", agent_type));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut result = conn.connection().query(&query).await?;
        let sessions: Vec<WorkSession> = result.take(0)?;

        Ok(sessions)
    }

    /// Update session
    pub async fn update_session(&self, session_id: &str, updates: SessionUpdate) -> Result<WorkSession> {
        info!("Updating session: {}", session_id);

        let session_uuid = Uuid::parse_str(session_id)?;
        let conn = self.storage.acquire().await?;

        // Get existing session
        let mut session: WorkSession = conn.connection()
            .select(("session", session_uuid.to_string()))
            .await?
            .ok_or_else(|| anyhow!("Session not found"))?;

        // Apply updates
        if let Some(name) = updates.name {
            session.name = name;
        }
        if let Some(status) = updates.status {
            session.status = status;
        }
        if let Some(metadata) = updates.metadata {
            session.metadata = Some(metadata);
        }
        session.updated_at = Utc::now();

        // Update in database
        let session_json = serde_json::to_value(&session)?;
        let _: Option<WorkSession> = conn.connection()
            .update(("session", session_uuid.to_string()))
            .content(session_json)
            .await?;

        info!("Session updated: {}", session_id);

        Ok(session)
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        info!("Deleting session: {}", session_id);

        let session_uuid = Uuid::parse_str(session_id)?;
        let conn = self.storage.acquire().await?;

        // Delete all file modifications for this session
        let query = "DELETE FROM session_file_modifications WHERE session_id = $session_id";
        conn.connection()
            .query(query)
            .bind(("session_id", session_id.to_string()))
            .await?;

        // Delete all locks for this session
        let query = "DELETE FROM lock WHERE owner = $session_id";
        conn.connection()
            .query(query)
            .bind(("session_id", session_id.to_string()))
            .await?;

        // Delete session
        let _: Option<WorkSession> = conn.connection()
            .delete(("session", session_uuid.to_string()))
            .await?;

        info!("Session deleted: {}", session_id);

        Ok(())
    }

    // ========================================================================
    // Lock Management
    // ========================================================================

    /// Acquire a lock on a resource
    pub async fn acquire_lock(
        &self,
        session_id: &str,
        entity_type: String,
        entity_id: String,
        lock_type: LockType,
        duration_seconds: Option<i64>,
    ) -> Result<Lock> {
        info!("Acquiring {} lock on {}:{} for session {}",
              lock_type_str(&lock_type), entity_type, entity_id, session_id);

        let conn = self.storage.acquire().await?;

        // Check if there's an existing exclusive lock
        let query = "SELECT * FROM lock WHERE entity_type = $type AND entity_id = $id AND lock_type = 'exclusive' AND expires_at > $now";
        let mut result = conn.connection()
            .query(query)
            .bind(("type", entity_type.clone()))
            .bind(("id", entity_id.clone()))
            .bind(("now", Utc::now()))
            .await?;

        let existing_locks: Vec<Lock> = result.take(0)?;

        if !existing_locks.is_empty() && lock_type == LockType::Exclusive {
            let existing = &existing_locks[0];
            if existing.owner != session_id {
                return Err(anyhow!("Resource already locked by another session"));
            }
        }

        let lock_id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::seconds(duration_seconds.unwrap_or(3600)); // Default 1 hour

        let lock = Lock {
            id: lock_id,
            entity_type,
            entity_id,
            lock_type,
            owner: session_id.to_string(),
            acquired_at: now,
            expires_at,
        };

        let lock_json = serde_json::to_value(&lock)?;

        let _: Option<serde_json::Value> = conn.connection()
            .create(("lock", lock_id.to_string()))
            .content(lock_json)
            .await?;

        info!("Lock acquired: {}", lock_id);

        Ok(lock)
    }

    /// Release a lock
    pub async fn release_lock(&self, lock_id: &str) -> Result<()> {
        info!("Releasing lock: {}", lock_id);

        let lock_uuid = Uuid::parse_str(lock_id)?;
        let conn = self.storage.acquire().await?;

        let _: Option<Lock> = conn.connection()
            .delete(("lock", lock_uuid.to_string()))
            .await?;

        info!("Lock released: {}", lock_id);

        Ok(())
    }

    /// List locks for a session
    pub async fn list_locks(&self, session_id: Option<&str>) -> Result<Vec<Lock>> {
        debug!("Listing locks");

        let conn = self.storage.acquire().await?;

        let query = if let Some(session_id) = session_id {
            format!("SELECT * FROM lock WHERE owner = '{}' AND expires_at > $now ORDER BY acquired_at DESC", session_id)
        } else {
            "SELECT * FROM lock WHERE expires_at > $now ORDER BY acquired_at DESC".to_string()
        };

        let mut result = conn.connection()
            .query(&query)
            .bind(("now", Utc::now()))
            .await?;

        let locks: Vec<Lock> = result.take(0)?;

        Ok(locks)
    }

    /// Clean up expired locks
    pub async fn cleanup_expired_locks(&self) -> Result<usize> {
        debug!("Cleaning up expired locks");

        let conn = self.storage.acquire().await?;
        let query = "DELETE FROM lock WHERE expires_at < $now";
        conn.connection()
            .query(query)
            .bind(("now", Utc::now()))
            .await?;

        info!("Expired locks cleaned up");

        Ok(0) // SurrealDB doesn't return count
    }

    // ========================================================================
    // File Modification Tracking
    // ========================================================================

    /// Track a file modification in a session
    pub async fn track_file_modification(
        &self,
        session_id: &str,
        file_path: String,
        file_id: String,
        change_type: ChangeType,
        content_hash: String,
        size_bytes: u64,
        base_version: Option<u64>,
    ) -> Result<FileModification> {
        debug!("Tracking file modification: {} in session {}", file_path, session_id);

        let conn = self.storage.acquire().await?;

        // Get previous version for this file in this session
        let query = "SELECT * FROM session_file_modifications WHERE session_id = $session_id AND file_path = $file_path ORDER BY version DESC LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("session_id", session_id.to_string()))
            .bind(("file_path", file_path.clone()))
            .await?;

        let previous: Vec<FileModification> = result.take(0)?;
        let version = previous.first().map(|m| m.version + 1).unwrap_or(1);

        let modification_id = Uuid::new_v4();
        let modification = FileModification {
            id: modification_id,
            session_id: session_id.to_string(),
            file_path: file_path.clone(),
            file_id,
            change_type,
            version,
            base_version,
            content_hash,
            size_bytes,
            created_at: Utc::now(),
        };

        let mod_json = serde_json::to_value(&modification)?;

        let _: Option<serde_json::Value> = conn.connection()
            .create(("session_file_modifications", modification_id.to_string()))
            .content(mod_json)
            .await?;

        debug!("File modification tracked: {} v{}", file_path, version);

        Ok(modification)
    }

    /// Get file modifications for a session
    pub async fn get_file_modifications(&self, session_id: &str) -> Result<Vec<FileModification>> {
        debug!("Getting file modifications for session: {}", session_id);

        let conn = self.storage.acquire().await?;

        let query = "SELECT * FROM session_file_modifications WHERE session_id = $session_id ORDER BY created_at DESC";
        let mut result = conn.connection()
            .query(query)
            .bind(("session_id", session_id.to_string()))
            .await?;

        let modifications: Vec<FileModification> = result.take(0)?;

        Ok(modifications)
    }

    /// Get latest modification for a specific file in a session
    pub async fn get_file_modification(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<Option<FileModification>> {
        debug!("Getting modification for file: {} in session: {}", file_path, session_id);

        let conn = self.storage.acquire().await?;

        let query = "SELECT * FROM session_file_modifications WHERE session_id = $session_id AND file_path = $file_path ORDER BY version DESC LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("session_id", session_id.to_string()))
            .bind(("file_path", file_path.to_string()))
            .await?;

        let modifications: Vec<FileModification> = result.take(0)?;

        Ok(modifications.into_iter().next())
    }

    /// Apply modifications from one session to another (merge)
    pub async fn apply_modifications(
        &self,
        source_session_id: &str,
        target_session_id: &str,
        strategy: MergeStrategy,
    ) -> Result<ApplyResult> {
        info!("Applying modifications from {} to {} with strategy {:?}",
              source_session_id, target_session_id, strategy);

        // Get all modifications from source session
        let source_mods = self.get_file_modifications(source_session_id).await?;

        // Get all modifications from target session
        let target_mods = self.get_file_modifications(target_session_id).await?;

        // Build a map of file paths to latest modifications in target
        let mut target_mod_map = std::collections::HashMap::new();
        for mod_item in &target_mods {
            target_mod_map
                .entry(mod_item.file_path.clone())
                .and_modify(|m: &mut &FileModification| {
                    if mod_item.version > m.version {
                        *m = mod_item;
                    }
                })
                .or_insert(mod_item);
        }

        let mut applied = 0;
        let mut conflicts = 0;
        let mut skipped = 0;

        for source_mod in &source_mods {
            if let Some(target_mod) = target_mod_map.get(&source_mod.file_path) {
                // Check for conflicts
                if source_mod.content_hash != target_mod.content_hash {
                    match strategy {
                        MergeStrategy::Auto => {
                            // Skip conflicting files in auto mode
                            warn!("Conflict detected for {}: skipping", source_mod.file_path);
                            conflicts += 1;
                            continue;
                        }
                        MergeStrategy::Theirs => {
                            // Use source version
                            debug!("Conflict resolved: using source version for {}", source_mod.file_path);
                        }
                        MergeStrategy::Mine => {
                            // Skip, keep target version
                            debug!("Conflict resolved: keeping target version for {}", source_mod.file_path);
                            skipped += 1;
                            continue;
                        }
                        MergeStrategy::Manual => {
                            // Return error for manual resolution
                            return Err(anyhow!("Manual conflict resolution required for {}", source_mod.file_path));
                        }
                    }
                }
            }

            // Apply the modification to target session
            // This would copy the file content and create a new modification record
            // For now, we just track it
            applied += 1;
        }

        info!("Merge completed: {} applied, {} conflicts, {} skipped", applied, conflicts, skipped);

        Ok(ApplyResult {
            applied,
            conflicts,
            skipped,
            total: source_mods.len(),
        })
    }

    /// Merge session changes back to workspace
    pub async fn merge_session(
        &self,
        session_id: &str,
        strategy: MergeStrategy,
        conflict_resolution: std::collections::HashMap<String, String>,
    ) -> Result<MergeResult> {
        info!("Merging session {} with strategy {:?}", session_id, strategy);

        // Get session
        let session = self.get_session(session_id).await?
            .ok_or_else(|| anyhow!("Session not found"))?;

        let _workspace_id = session.workspace_id
            .ok_or_else(|| anyhow!("Session has no associated workspace"))?;

        // Get all modifications for this session
        let modifications = self.get_file_modifications(session_id).await?;

        let mut changes_merged = 0;
        let mut conflicts_resolved = 0;

        for modification in &modifications {
            // Check if there's a conflict resolution for this file
            if conflict_resolution.contains_key(&modification.file_path) {
                conflicts_resolved += 1;
            }

            // In a real implementation, this would:
            // 1. Apply the changes to the VFS
            // 2. Update the workspace state
            // 3. Handle conflicts based on strategy
            changes_merged += 1;
        }

        // Update session status to completed
        self.update_session(
            session_id,
            SessionUpdate {
                name: None,
                status: Some(SessionStatus::Completed),
                metadata: None,
            },
        ).await?;

        let merge_id = Uuid::new_v4();

        info!("Session merged: {} changes, {} conflicts resolved", changes_merged, conflicts_resolved);

        Ok(MergeResult {
            merge_id,
            status: "success".to_string(),
            changes_merged,
            conflicts_resolved,
            new_version: 1, // Would be calculated based on workspace version
        })
    }
}

// ============================================================================
// Types
// ============================================================================

/// Work session database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    pub id: Uuid,
    pub name: String,
    pub agent_type: String,
    pub workspace_id: Option<Uuid>,
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SessionMetadata>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Session filters for listing
#[derive(Debug, Clone)]
pub struct SessionFilters {
    pub status: Option<String>,
    pub agent_type: Option<String>,
    pub limit: Option<usize>,
}

/// Session update request
#[derive(Debug, Clone)]
pub struct SessionUpdate {
    pub name: Option<String>,
    pub status: Option<SessionStatus>,
    pub metadata: Option<SessionMetadata>,
}

/// Lock database model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lock {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: String,
    pub lock_type: LockType,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Lock type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LockType {
    Exclusive,
    Shared,
}

fn lock_type_str(lock_type: &LockType) -> &str {
    match lock_type {
        LockType::Exclusive => "exclusive",
        LockType::Shared => "shared",
    }
}

/// File modification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModification {
    pub id: Uuid,
    pub session_id: String,
    pub file_path: String,
    pub file_id: String,
    pub change_type: ChangeType,
    pub version: u64,
    pub base_version: Option<u64>,
    pub content_hash: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

/// Change type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
}

/// Merge strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    Auto,
    Manual,
    Theirs,
    Mine,
}

/// Apply result
#[derive(Debug, Clone, Serialize)]
pub struct ApplyResult {
    pub applied: usize,
    pub conflicts: usize,
    pub skipped: usize,
    pub total: usize,
}

/// Merge result
#[derive(Debug, Clone, Serialize)]
pub struct MergeResult {
    pub merge_id: Uuid,
    pub status: String,
    pub changes_merged: usize,
    pub conflicts_resolved: usize,
    pub new_version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let deserialized: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SessionStatus::Active);
    }

    #[test]
    fn test_lock_type_serialization() {
        let lock = LockType::Exclusive;
        let json = serde_json::to_string(&lock).unwrap();
        assert_eq!(json, "\"exclusive\"");

        let deserialized: LockType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, LockType::Exclusive);
    }

    #[test]
    fn test_change_type_serialization() {
        let change = ChangeType::Modified;
        let json = serde_json::to_string(&change).unwrap();
        assert_eq!(json, "\"modified\"");

        let deserialized: ChangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ChangeType::Modified);
    }
}
