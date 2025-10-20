use crate::storage::{Storage, Snapshot, WriteOp, serialize, deserialize};
use crate::types::{
    CodeSymbol, Delta, Query, QueryResult, Session, SessionId, SymbolId, TokenCount, ChangeType,
};
use crate::indexer::TreeSitterParser;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session manager for isolated work sessions with copy-on-write
pub struct SessionManager {
    storage: Arc<dyn Storage>,
    sessions: Arc<DashMap<SessionId, Arc<RwLock<SessionState>>>>,
    config: SessionConfig,
    parser: Arc<RwLock<TreeSitterParser>>,
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Maximum number of concurrent sessions
    pub max_sessions: usize,
    /// Session timeout duration
    pub timeout: Duration,
    /// Enable automatic timeout cleanup
    pub auto_cleanup: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_sessions: 10,
            timeout: Duration::hours(1),
            auto_cleanup: true,
        }
    }
}

/// Internal session state
struct SessionState {
    /// Session metadata
    session: Session,
    /// Base snapshot from main index
    base_snapshot: Arc<Box<dyn Snapshot>>,
    /// Delta overlay for session changes
    deltas: Vec<Delta>,
    /// Index overlay: symbol_id -> CodeSymbol
    symbol_overlay: HashMap<SymbolId, OverlayEntry<CodeSymbol>>,
    /// File overlay: path -> content
    file_overlay: HashMap<PathBuf, OverlayEntry<String>>,
    /// Symbols affected by changes
    affected_symbols: HashSet<SymbolId>,
    /// Last access time for timeout management
    last_access: DateTime<Utc>,
}

/// Overlay entry with modification state
#[derive(Debug, Clone)]
enum OverlayEntry<T> {
    /// Added in session
    Added(T),
    /// Modified in session
    Modified(T),
    /// Deleted in session
    Deleted,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(storage: Arc<dyn Storage>, config: SessionConfig) -> Result<Self> {
        let parser = TreeSitterParser::new()
            .context("Failed to create TreeSitterParser")?;

        Ok(Self {
            storage,
            sessions: Arc::new(DashMap::new()),
            config,
            parser: Arc::new(RwLock::new(parser)),
        })
    }

    /// Create with default configuration
    pub fn with_storage(storage: Arc<dyn Storage>) -> Result<Self> {
        Self::new(storage, SessionConfig::default())
    }

    /// Begin a new isolated work session with copy-on-write
    pub async fn begin(
        &self,
        task_description: String,
        scope: Vec<PathBuf>,
        base_commit: Option<String>,
    ) -> Result<SessionId> {
        // Check if we need to evict old sessions
        if self.sessions.len() >= self.config.max_sessions {
            self.evict_oldest_session().await?;
        }

        // Check for timed out sessions if auto cleanup is enabled
        if self.config.auto_cleanup {
            self.cleanup_timed_out_sessions().await?;
        }

        // Create base snapshot from main index
        let base_snapshot = self.storage.snapshot().await
            .context("Failed to create base snapshot")?;

        // Create new session
        let session = Session {
            id: SessionId::new(),
            task_description,
            scope,
            base_commit,
            started_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let session_id = session.id.clone();

        // Create session state
        let state = SessionState {
            session,
            base_snapshot: Arc::new(base_snapshot),
            deltas: Vec::new(),
            symbol_overlay: HashMap::new(),
            file_overlay: HashMap::new(),
            affected_symbols: HashSet::new(),
            last_access: Utc::now(),
        };

        // Insert into sessions map
        self.sessions.insert(session_id.clone(), Arc::new(RwLock::new(state)));

        tracing::info!(
            session_id = %session_id.0,
            "Started new session with copy-on-write"
        );

        Ok(session_id)
    }

    /// Update session with file changes
    pub async fn update(
        &self,
        session_id: &SessionId,
        path: PathBuf,
        content: String,
        reindex: bool,
    ) -> Result<UpdateStatus> {
        let state_lock = self.sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?
            .clone();

        // Check if file exists in base (before acquiring write lock)
        let file_key = format!("file:{}", path.to_string_lossy());
        let snapshot = {
            let state = state_lock.read().await;
            state.base_snapshot.clone()
            // Lock is automatically dropped here
        };
        let file_exists_in_base = snapshot.get(file_key.as_bytes()).await?.is_some();

        let mut state = state_lock.write().await;
        state.last_access = Utc::now();

        // Detect change type
        let change_type = if state.file_overlay.contains_key(&path) || file_exists_in_base {
            ChangeType::FileModified { path: path.to_string_lossy().to_string() }
        } else {
            ChangeType::FileAdded { path: path.to_string_lossy().to_string() }
        };

        // Store file content in overlay
        state.file_overlay.insert(path.clone(), OverlayEntry::Modified(content.clone()));

        // Drop write lock before async operation
        drop(state);

        // If reindex is requested, parse and update symbols (without holding lock)
        let symbols = if reindex {
            self.parse_file_symbols(&path, &content).await?
        } else {
            Vec::new()
        };

        // Re-acquire lock to update state
        let mut state = state_lock.write().await;

        let affected_symbols = if !symbols.is_empty() {
            let symbol_ids: Vec<SymbolId> = symbols.iter().map(|s| s.id.clone()).collect();

            // Update symbol overlay
            for symbol in symbols {
                let symbol_id = symbol.id.clone();
                state.symbol_overlay.insert(
                    symbol_id.clone(),
                    OverlayEntry::Modified(symbol)
                );
                state.affected_symbols.insert(symbol_id);
            }

            symbol_ids
        } else {
            Vec::new()
        };

        // Create delta
        let delta = Delta {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            change_type,
            affected_symbols: affected_symbols.clone(),
        };

        state.deltas.push(delta);
        state.session.updated_at = Utc::now();

        Ok(UpdateStatus {
            affected_symbols: affected_symbols.into_iter()
                .filter_map(|id| state.symbol_overlay.get(&id).and_then(|e| {
                    match e {
                        OverlayEntry::Modified(s) | OverlayEntry::Added(s) => Some(s.clone()),
                        _ => None,
                    }
                }))
                .collect(),
            deltas_count: state.deltas.len(),
        })
    }

    /// Query in session context - prefers session changes over base
    pub async fn query(
        &self,
        session_id: &SessionId,
        query: Query,
        prefer_session: bool,
    ) -> Result<SessionQueryResult> {
        let state_lock = self.sessions.get(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?
            .clone();

        let mut state = state_lock.write().await;
        state.last_access = Utc::now();

        let mut from_session = 0;
        let mut from_base = 0;
        let mut results = Vec::new();
        let mut total_tokens = TokenCount::zero();

        // Search in session overlay first if prefer_session is true
        if prefer_session {
            for entry in state.symbol_overlay.values() {
                match entry {
                    OverlayEntry::Added(symbol) | OverlayEntry::Modified(symbol) => {
                        if self.matches_query(symbol, &query) {
                            total_tokens.add(symbol.metadata.token_cost);
                            results.push(symbol.clone());
                            from_session += 1;

                            // Check token limit
                            if let Some(max_tokens) = query.max_tokens {
                                if total_tokens >= max_tokens {
                                    return Ok(SessionQueryResult {
                                        result: QueryResult {
                                            symbols: results,
                                            total_tokens,
                                            truncated: true,
                                            total_matches: None,
                                            offset: None,
                                            has_more: None,
                                        },
                                        from_session,
                                        from_base,
                                    });
                                }
                            }
                        }
                    }
                    OverlayEntry::Deleted => {
                        // Skip deleted symbols
                    }
                }
            }
        }

        // Query base index
        let base_results = self.query_base_index(&state, &query).await?;

        for symbol in base_results {
            // Skip if symbol was modified/deleted in session overlay
            if state.symbol_overlay.contains_key(&symbol.id) {
                continue;
            }

            total_tokens.add(symbol.metadata.token_cost);
            results.push(symbol);
            from_base += 1;

            // Check token limit
            if let Some(max_tokens) = query.max_tokens {
                if total_tokens >= max_tokens {
                    return Ok(SessionQueryResult {
                        result: QueryResult {
                            symbols: results,
                            total_tokens,
                            truncated: true,
                            total_matches: None,
                            offset: None,
                            has_more: None,
                        },
                        from_session,
                        from_base,
                    });
                }
            }

            // Check max results
            if let Some(max_results) = query.max_results {
                if results.len() >= max_results {
                    break;
                }
            }
        }

        Ok(SessionQueryResult {
            result: QueryResult {
                symbols: results,
                total_tokens,
                truncated: false,
                total_matches: None,
                offset: None,
                has_more: None,
            },
            from_session,
            from_base,
        })
    }

    /// Complete session with commit, discard, or stash
    pub async fn complete(
        &self,
        session_id: &SessionId,
        action: SessionAction,
    ) -> Result<CompletionResult> {
        let state_lock = self.sessions.remove(session_id)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id.0))?
            .1;

        let state = state_lock.write().await;

        let changes_summary = ChangesSummary {
            total_deltas: state.deltas.len(),
            affected_symbols: state.affected_symbols.len(),
            files_modified: state.file_overlay.len(),
        };

        match action {
            SessionAction::Commit => {
                self.commit_session(&state).await?;
                tracing::info!(
                    session_id = %session_id.0,
                    deltas = state.deltas.len(),
                    symbols = state.affected_symbols.len(),
                    "Committed session changes to main index"
                );
            }
            SessionAction::Discard => {
                tracing::info!(
                    session_id = %session_id.0,
                    "Discarded session changes"
                );
                // Nothing to do, changes are dropped
            }
            SessionAction::Stash => {
                self.stash_session(session_id, &state).await?;
                tracing::info!(
                    session_id = %session_id.0,
                    "Stashed session changes for later"
                );
            }
        }

        Ok(CompletionResult {
            session_id: session_id.clone(),
            action,
            changes_summary,
        })
    }

    /// Get session information
    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        match self.sessions.get(session_id) {
            Some(s) => {
                let state = s.read().await;
                Some(state.session.clone())
            }
            None => None,
        }
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<Session> {
        let mut sessions = Vec::new();
        for entry in self.sessions.iter() {
            let state = entry.value().read().await;
            sessions.push(state.session.clone());
        }
        sessions
    }

    /// Get session changes summary
    pub async fn get_changes_summary(&self, session_id: &SessionId) -> Option<ChangesSummary> {
        match self.sessions.get(session_id) {
            Some(s) => {
                let state = s.read().await;
                Some(ChangesSummary {
                    total_deltas: state.deltas.len(),
                    affected_symbols: state.affected_symbols.len(),
                    files_modified: state.file_overlay.len(),
                })
            }
            None => None,
        }
    }

    /// Check for conflicts between sessions
    pub async fn detect_conflicts(
        &self,
        session_id1: &SessionId,
        session_id2: &SessionId,
    ) -> Result<ConflictReport> {
        let state1 = self.sessions.get(session_id1)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id1.0))?;
        let state2 = self.sessions.get(session_id2)
            .ok_or_else(|| anyhow!("Session not found: {}", session_id2.0))?;

        let s1 = state1.read().await;
        let s2 = state2.read().await;

        // Find overlapping symbols
        let symbol_conflicts: Vec<_> = s1.affected_symbols
            .intersection(&s2.affected_symbols)
            .cloned()
            .collect();

        // Find overlapping files
        let file_conflicts: Vec<_> = s1.file_overlay.keys()
            .filter(|path| s2.file_overlay.contains_key(*path))
            .cloned()
            .collect();

        let has_conflicts = !symbol_conflicts.is_empty() || !file_conflicts.is_empty();

        Ok(ConflictReport {
            has_conflicts,
            symbol_conflicts,
            file_conflicts,
        })
    }

    /// Clean up timed out sessions
    pub async fn cleanup_timed_out_sessions(&self) -> Result<usize> {
        let now = Utc::now();
        let timeout = self.config.timeout;
        let mut cleaned = 0;

        let mut timed_out = Vec::new();
        for entry in self.sessions.iter() {
            let state = entry.value().read().await;
            if now.signed_duration_since(state.last_access) > timeout {
                timed_out.push(entry.key().clone());
            }
        }

        for session_id in timed_out {
            tracing::warn!(
                session_id = %session_id.0,
                "Session timed out, stashing changes"
            );
            self.complete(&session_id, SessionAction::Stash).await?;
            cleaned += 1;
        }

        Ok(cleaned)
    }

    // Private helper methods

    /// Evict oldest session
    async fn evict_oldest_session(&self) -> Result<()> {
        let mut oldest: Option<(SessionId, DateTime<Utc>)> = None;
        for entry in self.sessions.iter() {
            let state = entry.value().read().await;
            let started_at = state.session.started_at;
            match &oldest {
                None => oldest = Some((entry.key().clone(), started_at)),
                Some((_, current_oldest)) => {
                    if started_at < *current_oldest {
                        oldest = Some((entry.key().clone(), started_at));
                    }
                }
            }
        }

        let oldest = oldest.map(|(id, _)| id);

        if let Some(session_id) = oldest {
            tracing::warn!(
                session_id = %session_id.0,
                "Evicting oldest session due to max sessions limit"
            );
            self.complete(&session_id, SessionAction::Stash).await?;
        }

        Ok(())
    }

    /// Commit session changes to main index
    async fn commit_session(&self, state: &SessionState) -> Result<()> {
        let mut operations = Vec::new();

        // Write file changes
        for (path, entry) in &state.file_overlay {
            match entry {
                OverlayEntry::Added(content) | OverlayEntry::Modified(content) => {
                    let key = format!("file:{}", path.to_string_lossy());
                    operations.push(WriteOp::Put {
                        key: key.into_bytes(),
                        value: content.as_bytes().to_vec(),
                    });
                }
                OverlayEntry::Deleted => {
                    let key = format!("file:{}", path.to_string_lossy());
                    operations.push(WriteOp::Delete {
                        key: key.into_bytes(),
                    });
                }
            }
        }

        // Write symbol changes
        for (symbol_id, entry) in &state.symbol_overlay {
            match entry {
                OverlayEntry::Added(symbol) | OverlayEntry::Modified(symbol) => {
                    let key = format!("symbol:{}", symbol_id.0);
                    let value = serialize(symbol)?;
                    operations.push(WriteOp::Put {
                        key: key.into_bytes(),
                        value,
                    });
                }
                OverlayEntry::Deleted => {
                    let key = format!("symbol:{}", symbol_id.0);
                    operations.push(WriteOp::Delete {
                        key: key.into_bytes(),
                    });
                }
            }
        }

        // Write deltas for audit trail
        let deltas_key = format!("session:{}:deltas", state.session.id.0);
        let deltas_value = serialize(&state.deltas)?;
        operations.push(WriteOp::Put {
            key: deltas_key.into_bytes(),
            value: deltas_value,
        });

        // Execute batch write
        self.storage.batch_write(operations).await?;

        Ok(())
    }

    /// Stash session changes for later
    async fn stash_session(&self, session_id: &SessionId, state: &SessionState) -> Result<()> {
        let stash_key = format!("stash:{}", session_id.0);

        let stash_data = StashedSession {
            session: state.session.clone(),
            deltas: state.deltas.clone(),
            symbol_overlay: state.symbol_overlay.clone(),
            file_overlay: state.file_overlay.clone(),
        };

        let value = serialize(&stash_data)?;
        self.storage.put(stash_key.as_bytes(), &value).await?;

        Ok(())
    }

    /// Query base index
    async fn query_base_index(
        &self,
        state: &SessionState,
        query: &Query,
    ) -> Result<Vec<CodeSymbol>> {
        // Get all symbol keys with prefix
        let prefix = b"symbol:";
        let keys = self.storage.get_keys_with_prefix(prefix).await?;

        let mut results = Vec::new();

        for key in keys {
            if let Some(data) = state.base_snapshot.get(&key).await? {
                let symbol: CodeSymbol = deserialize(&data)?;
                if self.matches_query(&symbol, query) {
                    results.push(symbol);
                }
            }
        }

        Ok(results)
    }

    /// Check if symbol matches query
    fn matches_query(&self, symbol: &CodeSymbol, query: &Query) -> bool {
        // Check symbol types filter
        if let Some(ref types) = query.symbol_types {
            if !types.contains(&symbol.kind) {
                return false;
            }
        }

        // Check scope filter
        if let Some(ref scope) = query.scope {
            if !symbol.location.file.starts_with(scope) {
                return false;
            }
        }

        // Simple text matching (can be enhanced with fuzzy search)
        let query_lower = query.text.to_lowercase();
        symbol.name.to_lowercase().contains(&query_lower)
            || symbol.signature.to_lowercase().contains(&query_lower)
    }

    /// Parse symbols from file content using TreeSitterParser
    async fn parse_file_symbols(&self, path: &PathBuf, content: &str) -> Result<Vec<CodeSymbol>> {
        let mut parser = self.parser.write().await;

        // Parse the file using TreeSitter
        let symbols = parser.parse_file(path, content)
            .with_context(|| format!("Failed to parse file: {:?}", path))?;

        tracing::debug!(
            "Parsed {} symbols from {}",
            symbols.len(),
            path.display()
        );

        Ok(symbols)
    }
}

/// Session action
#[derive(Debug, Clone, Copy)]
pub enum SessionAction {
    /// Commit changes to main index
    Commit,
    /// Discard all changes
    Discard,
    /// Stash changes for later
    Stash,
}

/// Update status
#[derive(Debug, Clone)]
pub struct UpdateStatus {
    pub affected_symbols: Vec<CodeSymbol>,
    pub deltas_count: usize,
}

/// Session query result with source tracking
#[derive(Debug, Clone)]
pub struct SessionQueryResult {
    pub result: QueryResult,
    pub from_session: usize,
    pub from_base: usize,
}

/// Completion result
#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub session_id: SessionId,
    pub action: SessionAction,
    pub changes_summary: ChangesSummary,
}

/// Changes summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChangesSummary {
    pub total_deltas: usize,
    pub affected_symbols: usize,
    pub files_modified: usize,
}

/// Conflict report between sessions
#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub has_conflicts: bool,
    pub symbol_conflicts: Vec<SymbolId>,
    pub file_conflicts: Vec<PathBuf>,
}

/// Stashed session data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StashedSession {
    session: Session,
    deltas: Vec<Delta>,
    symbol_overlay: HashMap<SymbolId, OverlayEntry<CodeSymbol>>,
    file_overlay: HashMap<PathBuf, OverlayEntry<String>>,
}

// Implement serde for OverlayEntry
impl<T: serde::Serialize> serde::Serialize for OverlayEntry<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStructVariant;
        match self {
            OverlayEntry::Added(value) => {
                let mut sv = serializer.serialize_struct_variant("OverlayEntry", 0, "Added", 1)?;
                sv.serialize_field("value", value)?;
                sv.end()
            }
            OverlayEntry::Modified(value) => {
                let mut sv = serializer.serialize_struct_variant("OverlayEntry", 1, "Modified", 1)?;
                sv.serialize_field("value", value)?;
                sv.end()
            }
            OverlayEntry::Deleted => {
                serializer.serialize_unit_variant("OverlayEntry", 2, "Deleted")
            }
        }
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for OverlayEntry<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Visitor, MapAccess};
        use std::fmt;

        #[derive(serde::Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        #[allow(dead_code)]
        enum Field { Value }

        struct OverlayEntryVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: serde::Deserialize<'de>> Visitor<'de> for OverlayEntryVisitor<T> {
            type Value = OverlayEntry<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum OverlayEntry")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let variant = map.next_key::<String>()?
                    .ok_or_else(|| serde::de::Error::custom("expected variant"))?;

                match variant.as_str() {
                    "Added" => {
                        let value = map.next_value()?;
                        Ok(OverlayEntry::Added(value))
                    }
                    "Modified" => {
                        let value = map.next_value()?;
                        Ok(OverlayEntry::Modified(value))
                    }
                    "Deleted" => Ok(OverlayEntry::Deleted),
                    _ => Err(serde::de::Error::unknown_variant(&variant, &["Added", "Modified", "Deleted"]))
                }
            }
        }

        deserializer.deserialize_enum(
            "OverlayEntry",
            &["Added", "Modified", "Deleted"],
            OverlayEntryVisitor(std::marker::PhantomData)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;

    async fn create_test_storage() -> (Arc<dyn Storage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = MemoryStorage::new();
        (Arc::new(storage) as Arc<dyn Storage>, temp_dir)
    }

    #[tokio::test]
    async fn test_begin_session() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![PathBuf::from("src/")],
            None,
        ).await.unwrap();

        let session = manager.get_session(&session_id).await.unwrap();
        assert_eq!(session.task_description, "Test task");
        assert_eq!(session.scope.len(), 1);
    }

    #[tokio::test]
    async fn test_session_update() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        let status = manager.update(
            &session_id,
            PathBuf::from("test.rs"),
            "fn main() {}".to_string(),
            false,
        ).await.unwrap();

        assert_eq!(status.deltas_count, 1);
    }

    #[tokio::test]
    async fn test_session_query() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        let query = Query::new("test".to_string());
        let result = manager.query(&session_id, query, true).await.unwrap();

        assert_eq!(result.from_session, 0);
        assert_eq!(result.from_base, 0);
    }

    #[tokio::test]
    async fn test_session_commit() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        manager.update(
            &session_id,
            PathBuf::from("test.rs"),
            "fn test() {}".to_string(),
            false,
        ).await.unwrap();

        let result = manager.complete(&session_id, SessionAction::Commit).await.unwrap();

        assert_eq!(result.changes_summary.total_deltas, 1);
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_discard() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        manager.update(
            &session_id,
            PathBuf::from("test.rs"),
            "fn test() {}".to_string(),
            false,
        ).await.unwrap();

        let result = manager.complete(&session_id, SessionAction::Discard).await.unwrap();

        assert_eq!(result.changes_summary.total_deltas, 1);
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_stash() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage.clone()).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        manager.update(
            &session_id,
            PathBuf::from("test.rs"),
            "fn test() {}".to_string(),
            false,
        ).await.unwrap();

        let result = manager.complete(&session_id, SessionAction::Stash).await.unwrap();

        assert_eq!(result.changes_summary.total_deltas, 1);

        // Verify stash was saved
        let stash_key = format!("stash:{}", session_id.0);
        let stashed = storage.get(stash_key.as_bytes()).await.unwrap();
        assert!(stashed.is_some());
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session1 = manager.begin(
            "Task 1".to_string(),
            vec![],
            None,
        ).await.unwrap();

        let session2 = manager.begin(
            "Task 2".to_string(),
            vec![],
            None,
        ).await.unwrap();

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 2);

        manager.complete(&session1, SessionAction::Discard).await.unwrap();
        manager.complete(&session2, SessionAction::Discard).await.unwrap();
    }

    #[tokio::test]
    async fn test_detect_conflicts() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session1 = manager.begin(
            "Task 1".to_string(),
            vec![],
            None,
        ).await.unwrap();

        let session2 = manager.begin(
            "Task 2".to_string(),
            vec![],
            None,
        ).await.unwrap();

        // Both sessions modify the same file
        let path = PathBuf::from("test.rs");
        manager.update(&session1, path.clone(), "content1".to_string(), false).await.unwrap();
        manager.update(&session2, path.clone(), "content2".to_string(), false).await.unwrap();

        let conflicts = manager.detect_conflicts(&session1, &session2).await.unwrap();
        assert!(conflicts.has_conflicts);
        assert_eq!(conflicts.file_conflicts.len(), 1);

        manager.complete(&session1, SessionAction::Discard).await.unwrap();
        manager.complete(&session2, SessionAction::Discard).await.unwrap();
    }

    #[tokio::test]
    async fn test_max_sessions_eviction() {
        let (storage, _temp) = create_test_storage().await;
        let config = SessionConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let manager = SessionManager::new(storage, config).unwrap();

        let session1 = manager.begin("Task 1".to_string(), vec![], None).await.unwrap();
        let session2 = manager.begin("Task 2".to_string(), vec![], None).await.unwrap();

        // This should trigger eviction of session1
        let session3 = manager.begin("Task 3".to_string(), vec![], None).await.unwrap();

        // session1 should be evicted (stashed)
        assert!(manager.get_session(&session1).await.is_none());
        assert!(manager.get_session(&session2).await.is_some());
        assert!(manager.get_session(&session3).await.is_some());

        manager.complete(&session2, SessionAction::Discard).await.unwrap();
        manager.complete(&session3, SessionAction::Discard).await.unwrap();
    }

    #[tokio::test]
    async fn test_session_timeout() {
        let (storage, _temp) = create_test_storage().await;
        let config = SessionConfig {
            max_sessions: 10,
            timeout: Duration::milliseconds(100),
            auto_cleanup: true,
        };
        let manager = SessionManager::new(storage, config).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Trigger cleanup
        let cleaned = manager.cleanup_timed_out_sessions().await.unwrap();
        assert_eq!(cleaned, 1);

        // Session should be gone
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_changes_summary() {
        let (storage, _temp) = create_test_storage().await;
        let manager = SessionManager::with_storage(storage).unwrap();

        let session_id = manager.begin(
            "Test task".to_string(),
            vec![],
            None,
        ).await.unwrap();

        manager.update(
            &session_id,
            PathBuf::from("test1.rs"),
            "content1".to_string(),
            false,
        ).await.unwrap();

        manager.update(
            &session_id,
            PathBuf::from("test2.rs"),
            "content2".to_string(),
            false,
        ).await.unwrap();

        let summary = manager.get_changes_summary(&session_id).await.unwrap();
        assert_eq!(summary.total_deltas, 2);
        assert_eq!(summary.files_modified, 2);

        manager.complete(&session_id, SessionAction::Discard).await.unwrap();
    }
}
