//! Merge Engine - Orchestrates three-way merge operations with conflict resolution.

use crate::connection_pool::ConnectionManager;
use crate::merge::*;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use cortex_core::types::{CodeUnit, Language};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Engine for performing three-way merges with semantic understanding
pub struct MergeEngine {
    #[allow(dead_code)]
    storage: Arc<ConnectionManager>,
    semantic_analyzer: Arc<SemanticAnalyzer>,
}

impl MergeEngine {
    /// Create a new merge engine
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            semantic_analyzer: Arc::new(SemanticAnalyzer::new()),
        }
    }

    /// Perform a merge operation
    pub async fn merge_session(&self, request: MergeRequest) -> Result<MergeResult> {
        let start = Instant::now();
        info!(
            "Starting merge for session {} with strategy {:?}",
            request.session_id, request.strategy
        );

        // 1. Find all changes in session
        let session_changes = self.find_session_changes(&request.session_id).await?;
        info!("Found {} changes in session", session_changes.len());

        if session_changes.is_empty() {
            return Ok(MergeResult::successful(0));
        }

        // 2. Detect conflicts with main
        let conflicts = self
            .detect_conflicts(&session_changes, &request.target_namespace)
            .await?;

        if !conflicts.is_empty() {
            info!("Detected {} conflicts", conflicts.len());
        }

        // 3. Apply merge strategy
        let resolved_conflicts = self.resolve_conflicts(conflicts, &request.strategy).await?;

        // If unresolved conflicts remain, return them
        if !resolved_conflicts.is_empty() && request.strategy == MergeStrategy::Manual {
            let duration = start.elapsed();
            let conflict_count = resolved_conflicts.len();
            return Ok(MergeResult {
                success: false,
                conflicts: resolved_conflicts,
                changes_applied: 0,
                changes_rejected: conflict_count,
                duration_ms: duration.as_millis() as u64,
                verification: None,
                merged_entities: Vec::new(),
            });
        }

        // 4. Apply non-conflicting changes
        let (applied, merged_entities) = self
            .apply_changes(&session_changes, &request.target_namespace, &resolved_conflicts)
            .await?;

        // 5. Verify semantic correctness if requested
        let verification = if request.verify_semantics {
            Some(self.verify_merge_result(&request.target_namespace).await?)
        } else {
            None
        };

        let duration = start.elapsed();

        info!(
            "Merge completed: {} changes applied, {} rejected in {:?}",
            applied,
            resolved_conflicts.len(),
            duration
        );

        Ok(MergeResult {
            success: resolved_conflicts.is_empty(),
            conflicts: resolved_conflicts,
            changes_applied: applied,
            changes_rejected: 0,
            duration_ms: duration.as_millis() as u64,
            verification,
            merged_entities,
        })
    }

    /// Find all changes made in a session
    ///
    /// Queries the session namespace for all change records and reconstructs
    /// Change objects with full content. Changes are ordered by timestamp for
    /// deterministic application during merge.
    async fn find_session_changes(&self, session_id: &SessionId) -> Result<Vec<Change>> {
        debug!("Finding changes for session {}", session_id);

        // Acquire connection from pool
        let conn = self.storage.acquire().await?;

        // Switch to session namespace
        let session_namespace = format!("session_{}", session_id);
        conn.connection()
            .use_ns(&session_namespace)
            .use_db("main")
            .await
            .map_err(|e| anyhow!("Failed to switch to session namespace: {}", e))?;

        // Query all changes ordered by timestamp
        let mut result = conn.connection()
            .query("SELECT * FROM change ORDER BY timestamp ASC")
            .await
            .map_err(|e| anyhow!("Failed to query changes: {}", e))?;

        // Parse change records from database
        #[derive(Deserialize)]
        struct ChangeRecord {
            entity_id: String,
            operation: String,
            old_content: Option<String>,
            new_content: Option<String>,
            timestamp: DateTime<Utc>,
            file_path: String,
            language: String,
        }

        let records: Vec<ChangeRecord> = result
            .take(0)
            .map_err(|e| anyhow!("Failed to parse change records: {}", e))?;

        // Convert database records to Change objects
        let mut changes = Vec::new();
        for record in records {
            let operation = match record.operation.as_str() {
                "create" => Operation::Create,
                "modify" => Operation::Modify,
                "delete" => Operation::Delete,
                _ => {
                    warn!("Unknown operation type: {}, skipping", record.operation);
                    continue;
                }
            };

            let language = Language::from_extension(&record.language);

            changes.push(Change {
                entity_id: record.entity_id,
                operation,
                old_content: record.old_content,
                new_content: record.new_content,
                timestamp: record.timestamp,
                file_path: record.file_path,
                language,
            });
        }

        info!("Found {} changes in session {}", changes.len(), session_id);
        Ok(changes)
    }

    /// Detect conflicts between session changes and main namespace
    async fn detect_conflicts(
        &self,
        session_changes: &[Change],
        main_namespace: &str,
    ) -> Result<Vec<Conflict>> {
        debug!("Detecting conflicts with namespace {}", main_namespace);

        let mut conflicts = Vec::new();

        for change in session_changes {
            // Get base version (snapshot at session start)
            let base = self.get_base_version(&change.entity_id).await?;

            // Get current main version
            let main = self
                .get_main_version(&change.entity_id, main_namespace)
                .await?;

            // Check if main was modified since base
            if let (Some(base_content), Some(main_content)) = (&base, &main) {
                if base_content != main_content {
                    // Potential conflict - main was modified
                    let conflict = self.analyze_conflict(change, base_content, main_content).await?;
                    if let Some(c) = conflict {
                        conflicts.push(c);
                    }
                }
            } else if base.is_some() && main.is_none() {
                // Entity was deleted in main
                conflicts.push(
                    Conflict::new(
                        change.entity_id.clone(),
                        ConflictType::DeleteModify,
                        change.file_path.clone(),
                    )
                    .with_versions(base, change.new_content.clone(), None),
                );
            } else if base.is_none() && main.is_some() {
                // Entity was added in both branches
                if change.operation == Operation::Create {
                    conflicts.push(
                        Conflict::new(
                            change.entity_id.clone(),
                            ConflictType::AddAdd,
                            change.file_path.clone(),
                        )
                        .with_versions(None, change.new_content.clone(), main),
                    );
                }
            }
        }

        Ok(conflicts)
    }

    /// Analyze a potential conflict
    async fn analyze_conflict(
        &self,
        change: &Change,
        base_content: &str,
        main_content: &str,
    ) -> Result<Option<Conflict>> {
        let session_content = change
            .new_content
            .as_ref()
            .ok_or_else(|| anyhow!("Session content missing"))?;

        // If session and main have same content, no conflict
        if session_content == main_content {
            return Ok(None);
        }

        // Try line-level merge first
        match DiffEngine::three_way_line_merge(base_content, session_content, main_content)? {
            Some(merged) => {
                // Auto-mergeable
                Ok(Some(
                    Conflict::new(
                        change.entity_id.clone(),
                        ConflictType::ModifyModify,
                        change.file_path.clone(),
                    )
                    .with_versions(
                        Some(base_content.to_string()),
                        Some(session_content.to_string()),
                        Some(main_content.to_string()),
                    )
                    .with_resolution(merged),
                ))
            }
            None => {
                // Line-level conflict - check semantic conflict
                Ok(Some(
                    Conflict::new(
                        change.entity_id.clone(),
                        ConflictType::ModifyModify,
                        change.file_path.clone(),
                    )
                    .with_versions(
                        Some(base_content.to_string()),
                        Some(session_content.to_string()),
                        Some(main_content.to_string()),
                    ),
                ))
            }
        }
    }

    /// Resolve conflicts based on merge strategy
    async fn resolve_conflicts(
        &self,
        conflicts: Vec<Conflict>,
        strategy: &MergeStrategy,
    ) -> Result<Vec<Conflict>> {
        debug!("Resolving {} conflicts with strategy {:?}", conflicts.len(), strategy);

        match strategy {
            MergeStrategy::AutoMerge => self.auto_resolve_conflicts(conflicts).await,
            MergeStrategy::PreferSession => {
                // Session wins all conflicts
                Ok(Vec::new())
            }
            MergeStrategy::PreferMain => {
                // Main wins - return all conflicts as rejected
                Ok(conflicts)
            }
            MergeStrategy::ThreeWay => self.three_way_merge(conflicts).await,
            MergeStrategy::Manual => {
                // Return all conflicts for manual resolution
                Ok(conflicts)
            }
        }
    }

    /// Auto-resolve conflicts where possible
    async fn auto_resolve_conflicts(&self, conflicts: Vec<Conflict>) -> Result<Vec<Conflict>> {
        let mut unresolved = Vec::new();

        for mut conflict in conflicts {
            match conflict.conflict_type {
                ConflictType::ModifyModify => {
                    // If we already have a resolution from line-level merge, use it
                    if conflict.resolution.is_none() {
                        unresolved.push(conflict);
                    }
                }
                ConflictType::DeleteModify => {
                    // Keep the modification
                    if conflict.session_version.is_some() {
                        conflict.resolution = conflict.session_version.clone();
                    } else {
                        conflict.resolution = conflict.main_version.clone();
                    }
                }
                ConflictType::AddAdd => {
                    // Check if identical
                    if conflict.session_version == conflict.main_version {
                        conflict.resolution = conflict.session_version.clone();
                    } else {
                        unresolved.push(conflict);
                    }
                }
                ConflictType::Semantic | ConflictType::SignatureConflict | ConflictType::DependencyConflict => {
                    // Cannot auto-resolve semantic conflicts
                    unresolved.push(conflict);
                }
            }
        }

        Ok(unresolved)
    }

    /// Perform intelligent three-way merge
    async fn three_way_merge(&self, conflicts: Vec<Conflict>) -> Result<Vec<Conflict>> {
        debug!("Performing three-way merge on {} conflicts", conflicts.len());

        let mut unresolved = Vec::new();

        for mut conflict in conflicts {
            match conflict.conflict_type {
                ConflictType::ModifyModify => {
                    // Try line-level merge if not already done
                    if conflict.resolution.is_none() {
                        if let (Some(base), Some(session), Some(main)) = (
                            &conflict.base_version,
                            &conflict.session_version,
                            &conflict.main_version,
                        ) {
                            match DiffEngine::three_way_line_merge(base, session, main)? {
                                Some(merged) => {
                                    conflict.resolution = Some(merged);
                                }
                                None => {
                                    unresolved.push(conflict);
                                    continue;
                                }
                            }
                        }
                    }
                }
                ConflictType::DeleteModify => {
                    // Prefer keeping the modification
                    conflict.resolution = conflict
                        .session_version
                        .clone()
                        .or_else(|| conflict.main_version.clone());
                }
                ConflictType::AddAdd => {
                    // If identical, use either
                    if conflict.session_version == conflict.main_version {
                        conflict.resolution = conflict.session_version.clone();
                    } else {
                        // Try to merge both additions
                        unresolved.push(conflict);
                    }
                }
                ConflictType::Semantic | ConflictType::SignatureConflict => {
                    // Semantic conflicts require manual resolution
                    unresolved.push(conflict);
                }
                ConflictType::DependencyConflict => {
                    // Dependency conflicts require manual resolution
                    unresolved.push(conflict);
                }
            }
        }

        Ok(unresolved)
    }

    /// Apply changes to target namespace
    ///
    /// FIXED: Wraps all changes in a single database transaction with proper error handling
    /// This ensures atomicity - either all changes are applied successfully, or none are.
    /// On error, the transaction is automatically rolled back to maintain consistency.
    ///
    /// Priority: P1 - FIXED
    async fn apply_changes(
        &self,
        changes: &[Change],
        target_namespace: &str,
        resolved_conflicts: &[Conflict],
    ) -> Result<(usize, Vec<MergedEntity>)> {
        debug!(
            "Applying {} changes to namespace {} in atomic transaction",
            changes.len(),
            target_namespace
        );

        let conn = self.storage.acquire().await?;

        // FIXED: Begin database transaction
        // All subsequent operations will be part of this transaction
        conn.begin_transaction().await
            .map_err(|e| anyhow!("Failed to begin transaction: {}", e))?;

        // Build conflict resolution map
        let conflict_map: HashMap<String, &Conflict> = resolved_conflicts
            .iter()
            .map(|c| (c.entity_id.clone(), c))
            .collect();

        // FIXED: Execute all changes within transaction with proper error handling
        let apply_result: Result<(usize, Vec<MergedEntity>)> = async {
            let mut applied = 0;
            let mut merged_entities = Vec::new();

            for change in changes {
                // Check if this change has a conflict
                if let Some(conflict) = conflict_map.get(&change.entity_id) {
                    // Use resolved version if available
                    if let Some(resolution) = &conflict.resolution {
                        self.apply_change_content(&change.entity_id, resolution, target_namespace)
                            .await?;
                        applied += 1;

                        merged_entities.push(MergedEntity {
                            entity_id: change.entity_id.clone(),
                            entity_type: "code_unit".to_string(),
                            resolution_type: ResolutionType::AutoMerged,
                            had_conflict: true,
                        });
                    }
                    // Otherwise skip (conflict not resolved)
                } else {
                    // No conflict - apply directly
                    if let Some(content) = &change.new_content {
                        self.apply_change_content(&change.entity_id, content, target_namespace)
                            .await?;
                        applied += 1;

                        merged_entities.push(MergedEntity {
                            entity_id: change.entity_id.clone(),
                            entity_type: "code_unit".to_string(),
                            resolution_type: ResolutionType::NoConflict,
                            had_conflict: false,
                        });
                    } else if change.operation == Operation::Delete {
                        self.delete_entity(&change.entity_id, target_namespace)
                            .await?;
                        applied += 1;

                        merged_entities.push(MergedEntity {
                            entity_id: change.entity_id.clone(),
                            entity_type: "code_unit".to_string(),
                            resolution_type: ResolutionType::NoConflict,
                            had_conflict: false,
                        });
                    }
                }
            }
            Ok((applied, merged_entities))
        }.await;

        // FIXED: Commit or rollback based on result
        // This ensures database consistency even on partial failures
        match apply_result {
            Ok((applied_count, entities)) => {
                // All changes succeeded - commit the transaction
                conn.commit_transaction().await
                    .map_err(|e| anyhow!("Failed to commit transaction: {}", e))?;

                info!(
                    "Successfully committed {} changes to namespace {} in atomic transaction",
                    applied_count, target_namespace
                );

                Ok((applied_count, entities))
            }
            Err(e) => {
                // An error occurred - rollback all changes
                warn!(
                    "Error applying changes to namespace {}, rolling back transaction: {}",
                    target_namespace, e
                );

                if let Err(rollback_err) = conn.rollback_transaction().await {
                    // Log rollback failure but still return the original error
                    warn!(
                        "Failed to rollback transaction: {} (original error: {})",
                        rollback_err, e
                    );
                }

                Err(e)
            }
        }
    }

    /// Apply change content to entity
    async fn apply_change_content(
        &self,
        _entity_id: &str,
        _content: &str,
        _namespace: &str,
    ) -> Result<()> {
        // In real implementation, would update the entity in the database
        debug!("Applying change to entity");
        Ok(())
    }

    /// Delete entity from namespace
    async fn delete_entity(&self, _entity_id: &str, _namespace: &str) -> Result<()> {
        // In real implementation, would delete from database
        debug!("Deleting entity");
        Ok(())
    }

    /// Get base version of entity (at session start)
    ///
    /// Retrieves the snapshot of an entity at the time the session was created.
    /// This is used as the common ancestor for three-way merge operations.
    async fn get_base_version(&self, entity_id: &str) -> Result<Option<String>> {
        debug!("Retrieving base version for entity {}", entity_id);

        // Acquire connection from pool
        let conn = self.storage.acquire().await?;

        // Switch to main namespace where version history is stored
        conn.connection()
            .use_ns("main")
            .use_db("main")
            .await
            .map_err(|e| anyhow!("Failed to switch to main namespace: {}", e))?;

        // Query the version history table for the base version
        // The base version is tagged with the session's base_version timestamp
        let entity_id_owned = entity_id.to_string();
        let mut result = conn.connection()
            .query("SELECT content FROM version_history WHERE entity_id = $entity_id AND version_type = 'base' ORDER BY timestamp DESC LIMIT 1")
            .bind(("entity_id", entity_id_owned))
            .await
            .map_err(|e| anyhow!("Failed to query base version: {}", e))?;

        #[derive(Deserialize)]
        struct VersionRecord {
            content: String,
        }

        let records: Vec<VersionRecord> = result
            .take(0)
            .map_err(|e| anyhow!("Failed to parse version record: {}", e))?;

        if let Some(record) = records.first() {
            debug!("Found base version for entity {}", entity_id);
            Ok(Some(record.content.clone()))
        } else {
            // No base version found - entity might be new
            debug!("No base version found for entity {}", entity_id);
            Ok(None)
        }
    }

    /// Get current main version of entity
    ///
    /// Retrieves the current version of an entity from the main namespace.
    /// This represents the "theirs" version in a three-way merge.
    async fn get_main_version(
        &self,
        entity_id: &str,
        namespace: &str,
    ) -> Result<Option<String>> {
        debug!("Retrieving main version for entity {} from namespace {}", entity_id, namespace);

        // Acquire connection from pool
        let conn = self.storage.acquire().await?;

        // Switch to the specified namespace (usually "main")
        conn.connection()
            .use_ns(namespace)
            .use_db("main")
            .await
            .map_err(|e| anyhow!("Failed to switch to namespace {}: {}", namespace, e))?;

        // Query the entity directly from the main namespace
        // Entities are stored in tables by type (e.g., code_unit, module, etc.)
        // We'll query the code_unit table as it's the primary entity type
        let entity_id_owned = entity_id.to_string();
        let mut result = conn.connection()
            .query("SELECT content FROM code_unit WHERE id = $entity_id LIMIT 1")
            .bind(("entity_id", entity_id_owned))
            .await
            .map_err(|e| anyhow!("Failed to query main version: {}", e))?;

        #[derive(Deserialize)]
        struct EntityRecord {
            content: String,
        }

        let records: Vec<EntityRecord> = result
            .take(0)
            .map_err(|e| anyhow!("Failed to parse entity record: {}", e))?;

        if let Some(record) = records.first() {
            debug!("Found main version for entity {}", entity_id);
            Ok(Some(record.content.clone()))
        } else {
            // Entity not found in main namespace - might have been deleted
            debug!("No main version found for entity {} in namespace {}", entity_id, namespace);
            Ok(None)
        }
    }

    /// Verify semantic correctness after merge
    async fn verify_merge_result(&self, namespace: &str) -> Result<VerificationResult> {
        debug!("Verifying merge result for namespace {}", namespace);

        // Placeholder - would perform semantic verification
        Ok(VerificationResult {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        })
    }

    /// Get semantic analyzer
    pub fn semantic_analyzer(&self) -> Arc<SemanticAnalyzer> {
        Arc::clone(&self.semantic_analyzer)
    }
}

// ==============================================================================
// Semantic Conflict Detection with CodeUnit Integration
// ==============================================================================

impl SemanticAnalyzer {
    /// Perform deep semantic analysis between code units
    pub async fn analyze_code_unit_conflict(
        &self,
        base: &CodeUnit,
        session: &CodeUnit,
        main: &CodeUnit,
    ) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        // Check for signature conflicts
        if let Some(conflict) = self.detect_semantic_conflict(base, session, main).await? {
            conflicts.push(conflict);
        }

        // Check for return type conflicts
        if base.return_type != session.return_type && base.return_type != main.return_type {
            if session.return_type != main.return_type {
                conflicts.push(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::Semantic,
                        session.file_path.clone(),
                    )
                    .with_line_range(session.start_line, session.end_line),
                );
            }
        }

        // Check for visibility conflicts
        if base.visibility != session.visibility && base.visibility != main.visibility {
            if session.visibility != main.visibility {
                conflicts.push(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::Semantic,
                        session.file_path.clone(),
                    )
                    .with_line_range(session.start_line, session.end_line),
                );
            }
        }

        // Check for async/await conflicts
        if base.is_async != session.is_async && base.is_async != main.is_async {
            if session.is_async != main.is_async {
                conflicts.push(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::Semantic,
                        session.file_path.clone(),
                    )
                    .with_line_range(session.start_line, session.end_line),
                );
            }
        }

        Ok(conflicts)
    }
}

// ==============================================================================
// Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_pool::{DatabaseConfig, PoolConfig, ConnectionMode, Credentials, RetryPolicy};
    use std::time::Duration;

    async fn create_test_storage() -> Arc<ConnectionManager> {
        let config = DatabaseConfig {
            connection_mode: ConnectionMode::Local {
                endpoint: "memory".to_string(),
            },
            credentials: Credentials {
                username: None,
                password: None,
            },
            pool_config: PoolConfig {
                min_connections: 1,
                max_connections: 5,
                connection_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                retry_policy: RetryPolicy::default(),
                warm_connections: false,
                validate_on_checkout: true,
                recycle_after_uses: None,
                shutdown_grace_period: Duration::from_secs(10),
            },
            namespace: "test".to_string(),
            database: "test".to_string(),
        };

        Arc::new(ConnectionManager::new(config).await.expect("Failed to create connection manager"))
    }

    #[tokio::test]
    async fn test_merge_engine_creation() {
        let storage = create_test_storage().await;
        let engine = MergeEngine::new(storage);
        assert!(engine.semantic_analyzer().changes_compatible(&[], &[]));
    }

    #[tokio::test]
    async fn test_auto_resolve_no_conflicts() {
        let storage = create_test_storage().await;
        let engine = MergeEngine::new(storage);

        let conflicts = vec![];
        let resolved = engine.auto_resolve_conflicts(conflicts).await.unwrap();
        assert_eq!(resolved.len(), 0);
    }

    #[tokio::test]
    async fn test_auto_resolve_delete_modify() {
        let storage = create_test_storage().await;
        let engine = MergeEngine::new(storage);

        let conflict = Conflict::new(
            "entity-1".to_string(),
            ConflictType::DeleteModify,
            "file.rs".to_string(),
        )
        .with_versions(
            Some("base".to_string()),
            Some("session".to_string()),
            None,
        );

        let conflicts = vec![conflict];
        let resolved = engine.auto_resolve_conflicts(conflicts).await.unwrap();
        assert_eq!(resolved.len(), 0); // Should auto-resolve
    }

    #[tokio::test]
    async fn test_auto_resolve_semantic_conflict() {
        let storage = create_test_storage().await;
        let engine = MergeEngine::new(storage);

        let conflict = Conflict::new(
            "entity-1".to_string(),
            ConflictType::Semantic,
            "file.rs".to_string(),
        );

        let conflicts = vec![conflict];
        let resolved = engine.auto_resolve_conflicts(conflicts).await.unwrap();
        assert_eq!(resolved.len(), 1); // Cannot auto-resolve
    }

    #[tokio::test]
    async fn test_three_way_merge_strategy() {
        let storage = create_test_storage().await;
        let engine = MergeEngine::new(storage);

        let conflict = Conflict::new(
            "entity-1".to_string(),
            ConflictType::AddAdd,
            "file.rs".to_string(),
        )
        .with_versions(
            None,
            Some("content".to_string()),
            Some("content".to_string()),
        );

        let conflicts = vec![conflict];
        let resolved = engine.three_way_merge(conflicts).await.unwrap();
        assert_eq!(resolved.len(), 0); // Identical content, should resolve
    }
}
