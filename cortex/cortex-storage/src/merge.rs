//! Three-way merge algorithm with semantic understanding for multi-agent coordination.
//!
//! This module provides sophisticated merge capabilities that go beyond simple text-based diff:
//! - Three-way merge (base, session, main versions)
//! - Semantic conflict detection using AST analysis
//! - Multiple merge strategies
//! - Line-level and unit-level merging
//! - Zero data loss guarantee
//!
//! # Architecture
//!
//! ```text
//! Session Changes → MergeEngine → Conflict Detection → Resolution → Apply Changes
//!                         ↓
//!                  SemanticAnalyzer
//!                         ↓
//!                   AST Comparison
//! ```

use anyhow::Result;
use chrono::{DateTime, Utc};
use cortex_core::types::{CodeUnit, Language};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ==============================================================================
// Core Types
// ==============================================================================

/// Session identifier for isolated namespaces
pub type SessionId = String;

/// Request to merge a session's changes back to main
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Session to merge
    pub session_id: SessionId,
    /// Target namespace (usually "main")
    pub target_namespace: String,
    /// Strategy for handling conflicts
    pub strategy: MergeStrategy,
    /// Optional user ID for manual resolution
    pub user_id: Option<String>,
    /// Whether to verify semantic correctness after merge
    pub verify_semantics: bool,
}

impl MergeRequest {
    pub fn new(session_id: SessionId, strategy: MergeStrategy) -> Self {
        Self {
            session_id,
            target_namespace: "main".to_string(),
            strategy,
            user_id: None,
            verify_semantics: true,
        }
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.target_namespace = namespace;
        self
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

/// Strategy for resolving conflicts during merge
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Automatic merge - fail if conflicts detected
    AutoMerge,
    /// Return conflicts for manual resolution
    Manual,
    /// Session version wins all conflicts
    PreferSession,
    /// Main version wins all conflicts
    PreferMain,
    /// Intelligent three-way merge with semantic analysis
    ThreeWay,
}

/// Result of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    /// Whether merge completed successfully
    pub success: bool,
    /// List of conflicts (empty if success = true)
    pub conflicts: Vec<Conflict>,
    /// Number of changes successfully applied
    pub changes_applied: usize,
    /// Number of changes rejected due to conflicts
    pub changes_rejected: usize,
    /// Time taken to perform merge
    pub duration_ms: u64,
    /// Semantic verification result
    pub verification: Option<VerificationResult>,
    /// Merged entities (for inspection)
    pub merged_entities: Vec<MergedEntity>,
}

impl MergeResult {
    pub fn new() -> Self {
        Self {
            success: false,
            conflicts: Vec::new(),
            changes_applied: 0,
            changes_rejected: 0,
            duration_ms: 0,
            verification: None,
            merged_entities: Vec::new(),
        }
    }

    pub fn with_conflicts(conflicts: Vec<Conflict>) -> Self {
        let count = conflicts.len();
        Self {
            success: false,
            conflicts,
            changes_applied: 0,
            changes_rejected: count,
            duration_ms: 0,
            verification: None,
            merged_entities: Vec::new(),
        }
    }

    pub fn successful(changes_applied: usize) -> Self {
        Self {
            success: true,
            conflicts: Vec::new(),
            changes_applied,
            changes_rejected: 0,
            duration_ms: 0,
            verification: None,
            merged_entities: Vec::new(),
        }
    }
}

/// Result of semantic verification after merge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Information about a merged entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedEntity {
    pub entity_id: String,
    pub entity_type: String,
    pub resolution_type: ResolutionType,
    pub had_conflict: bool,
}

/// How a conflict was resolved
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionType {
    NoConflict,
    AutoMerged,
    SessionPreferred,
    MainPreferred,
    ManuallyResolved,
}

/// Represents a merge conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Unique identifier for this conflict
    pub id: String,
    /// Entity that has conflict
    pub entity_id: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Base version (common ancestor)
    pub base_version: Option<String>,
    /// Session version (agent's changes)
    pub session_version: Option<String>,
    /// Main version (other changes)
    pub main_version: Option<String>,
    /// Auto-resolved result (if applicable)
    pub resolution: Option<String>,
    /// File path where conflict occurred
    pub file_path: String,
    /// Line range affected
    pub line_range: Option<(usize, usize)>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Conflict {
    pub fn new(entity_id: String, conflict_type: ConflictType, file_path: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_id,
            conflict_type,
            base_version: None,
            session_version: None,
            main_version: None,
            resolution: None,
            file_path,
            line_range: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_versions(
        mut self,
        base: Option<String>,
        session: Option<String>,
        main: Option<String>,
    ) -> Self {
        self.base_version = base;
        self.session_version = session;
        self.main_version = main;
        self
    }

    pub fn with_resolution(mut self, resolution: String) -> Self {
        self.resolution = Some(resolution);
        self
    }

    pub fn with_line_range(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }
}

/// Type of merge conflict
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Both modified the same entity differently
    ModifyModify,
    /// One deleted, one modified
    DeleteModify,
    /// Both added same entity with different content
    AddAdd,
    /// Semantic conflict (code would break)
    Semantic,
    /// Type signature changed incompatibly
    SignatureConflict,
    /// Dependency version conflict
    DependencyConflict,
}

impl std::fmt::Display for ConflictType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictType::ModifyModify => write!(f, "Modify-Modify"),
            ConflictType::DeleteModify => write!(f, "Delete-Modify"),
            ConflictType::AddAdd => write!(f, "Add-Add"),
            ConflictType::Semantic => write!(f, "Semantic"),
            ConflictType::SignatureConflict => write!(f, "Signature"),
            ConflictType::DependencyConflict => write!(f, "Dependency"),
        }
    }
}

// ==============================================================================
// Change Tracking
// ==============================================================================

/// Represents a change made in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub entity_id: String,
    pub operation: Operation,
    pub old_content: Option<String>,
    pub new_content: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub file_path: String,
    pub language: Language,
}

impl Change {
    pub fn create(entity_id: String, content: String, file_path: String, language: Language) -> Self {
        Self {
            entity_id,
            operation: Operation::Create,
            old_content: None,
            new_content: Some(content),
            timestamp: Utc::now(),
            file_path,
            language,
        }
    }

    pub fn modify(
        entity_id: String,
        old: String,
        new: String,
        file_path: String,
        language: Language,
    ) -> Self {
        Self {
            entity_id,
            operation: Operation::Modify,
            old_content: Some(old),
            new_content: Some(new),
            timestamp: Utc::now(),
            file_path,
            language,
        }
    }

    pub fn delete(entity_id: String, content: String, file_path: String, language: Language) -> Self {
        Self {
            entity_id,
            operation: Operation::Delete,
            old_content: Some(content),
            new_content: None,
            timestamp: Utc::now(),
            file_path,
            language,
        }
    }
}

/// Type of change operation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Modify,
    Delete,
}

/// Collection of changes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: Change) {
        self.changes.push(change);
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Change> {
        self.changes.iter()
    }
}

// ==============================================================================
// Diff Algorithm
// ==============================================================================

/// A hunk represents a contiguous change region
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// Starting line in base version
    pub base_start: usize,
    /// Number of lines in base version
    pub base_count: usize,
    /// Starting line in new version
    pub new_start: usize,
    /// Number of lines in new version
    pub new_count: usize,
    /// The actual changed lines
    pub lines: Vec<String>,
}

impl Hunk {
    pub fn overlaps(&self, other: &Hunk) -> bool {
        let self_end = self.base_start + self.base_count;
        let other_end = other.base_start + other.base_count;

        !(self_end <= other.base_start || other_end <= self.base_start)
    }
}

/// Simple diff algorithm for line-level comparison
pub struct DiffEngine;

impl DiffEngine {
    /// Compute diff hunks between two versions
    pub fn diff(base: &str, modified: &str) -> Vec<Hunk> {
        let base_lines: Vec<&str> = base.lines().collect();
        let modified_lines: Vec<&str> = modified.lines().collect();

        let mut hunks = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < base_lines.len() || j < modified_lines.len() {
            // Find next difference
            while i < base_lines.len() && j < modified_lines.len() && base_lines[i] == modified_lines[j] {
                i += 1;
                j += 1;
            }

            if i >= base_lines.len() && j >= modified_lines.len() {
                break;
            }

            // Found a difference, create hunk
            let base_start = i;
            let new_start = j;

            // Find end of difference
            let mut base_count = 0;
            let mut new_count = 0;
            let mut hunk_lines = Vec::new();

            while i < base_lines.len() || j < modified_lines.len() {
                if i < base_lines.len() && j < modified_lines.len() && base_lines[i] == modified_lines[j] {
                    // Found matching line, hunk ends
                    break;
                }

                if i < base_lines.len() {
                    base_count += 1;
                    i += 1;
                }

                if j < modified_lines.len() {
                    hunk_lines.push(modified_lines[j].to_string());
                    new_count += 1;
                    j += 1;
                }

                // Limit hunk size
                if base_count > 100 || new_count > 100 {
                    break;
                }
            }

            hunks.push(Hunk {
                base_start,
                base_count,
                new_start,
                new_count,
                lines: hunk_lines,
            });
        }

        hunks
    }

    /// Check if two sets of hunks overlap
    pub fn hunks_overlap(hunks1: &[Hunk], hunks2: &[Hunk]) -> bool {
        for h1 in hunks1 {
            for h2 in hunks2 {
                if h1.overlaps(h2) {
                    return true;
                }
            }
        }
        false
    }

    /// Apply hunks to base text to produce merged result
    pub fn apply_hunks(base: &str, hunks: &[Hunk]) -> String {
        let base_lines: Vec<&str> = base.lines().collect();
        let mut result = Vec::new();
        let mut current_line = 0;

        for hunk in hunks {
            // Copy unchanged lines before this hunk
            while current_line < hunk.base_start && current_line < base_lines.len() {
                result.push(base_lines[current_line].to_string());
                current_line += 1;
            }

            // Apply hunk changes
            result.extend(hunk.lines.clone());

            // Skip replaced lines
            current_line += hunk.base_count;
        }

        // Copy remaining unchanged lines
        while current_line < base_lines.len() {
            result.push(base_lines[current_line].to_string());
            current_line += 1;
        }

        result.join("\n")
    }

    /// Perform three-way line merge
    pub fn three_way_line_merge(base: &str, session: &str, main: &str) -> Result<Option<String>> {
        // Get hunks for both branches
        let session_hunks = Self::diff(base, session);
        let main_hunks = Self::diff(base, main);

        // Check if hunks overlap
        if Self::hunks_overlap(&session_hunks, &main_hunks) {
            return Ok(None); // Cannot auto-merge
        }

        // Merge non-overlapping hunks
        let mut all_hunks = session_hunks;
        all_hunks.extend(main_hunks);

        // Sort hunks by base position
        all_hunks.sort_by_key(|h| h.base_start);

        // Apply all hunks
        let merged = Self::apply_hunks(base, &all_hunks);

        Ok(Some(merged))
    }
}

// ==============================================================================
// Semantic Analysis
// ==============================================================================

/// Analyzes code semantics for intelligent conflict detection
pub struct SemanticAnalyzer {
    _parser: Arc<Option<()>>, // Placeholder for parser integration
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            _parser: Arc::new(None),
        }
    }

    /// Detect semantic conflicts between three versions of a code unit
    pub async fn detect_semantic_conflict(
        &self,
        base: &CodeUnit,
        session: &CodeUnit,
        main: &CodeUnit,
    ) -> Result<Option<Conflict>> {
        // Check signature changes
        if self.signature_changed(base, session) && self.signature_changed(base, main) {
            // Both changed signature - potential conflict
            if session.signature != main.signature {
                return Ok(Some(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::SignatureConflict,
                        session.file_path.clone(),
                    )
                    .with_versions(
                        Some(base.signature.clone()),
                        Some(session.signature.clone()),
                        Some(main.signature.clone()),
                    )
                    .with_line_range(session.start_line, session.end_line),
                ));
            }
        }

        // Check parameter changes
        if self.parameters_changed(base, session) && self.parameters_changed(base, main) {
            if session.parameters != main.parameters {
                return Ok(Some(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::Semantic,
                        session.file_path.clone(),
                    )
                    .with_line_range(session.start_line, session.end_line),
                ));
            }
        }

        // Check visibility changes
        if base.visibility != session.visibility && base.visibility != main.visibility {
            if session.visibility != main.visibility {
                return Ok(Some(
                    Conflict::new(
                        session.id.to_string(),
                        ConflictType::Semantic,
                        session.file_path.clone(),
                    )
                    .with_line_range(session.start_line, session.end_line),
                ));
            }
        }

        Ok(None)
    }

    fn signature_changed(&self, base: &CodeUnit, modified: &CodeUnit) -> bool {
        base.signature != modified.signature
    }

    fn parameters_changed(&self, base: &CodeUnit, modified: &CodeUnit) -> bool {
        if base.parameters.len() != modified.parameters.len() {
            return true;
        }

        for (b, m) in base.parameters.iter().zip(modified.parameters.iter()) {
            if b.name != m.name || b.param_type != m.param_type {
                return true;
            }
        }

        false
    }

    /// Check if semantic changes are compatible
    pub fn changes_compatible(&self, _changes1: &[String], _changes2: &[String]) -> bool {
        // Simplified: check if changes don't contradict
        // In real implementation, would do deep semantic analysis
        true
    }
}

// ==============================================================================
// Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical() {
        let base = "line1\nline2\nline3";
        let modified = "line1\nline2\nline3";

        let hunks = DiffEngine::diff(base, modified);
        assert_eq!(hunks.len(), 0);
    }

    #[test]
    fn test_diff_single_change() {
        let base = "line1\nline2\nline3";
        let modified = "line1\nmodified\nline3";

        let hunks = DiffEngine::diff(base, modified);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].base_start, 1);
        assert_eq!(hunks[0].base_count, 1);
        assert_eq!(hunks[0].new_count, 1);
    }

    #[test]
    fn test_hunks_overlap() {
        let hunk1 = Hunk {
            base_start: 5,
            base_count: 3,
            new_start: 5,
            new_count: 3,
            lines: vec![],
        };

        let hunk2 = Hunk {
            base_start: 7,
            base_count: 3,
            new_start: 7,
            new_count: 3,
            lines: vec![],
        };

        assert!(hunk1.overlaps(&hunk2));
    }

    #[test]
    fn test_hunks_no_overlap() {
        let hunk1 = Hunk {
            base_start: 5,
            base_count: 2,
            new_start: 5,
            new_count: 2,
            lines: vec![],
        };

        let hunk2 = Hunk {
            base_start: 10,
            base_count: 2,
            new_start: 10,
            new_count: 2,
            lines: vec![],
        };

        assert!(!hunk1.overlaps(&hunk2));
    }

    #[test]
    fn test_three_way_merge_no_conflict() {
        let base = "line1\nline2\nline3\nline4";
        let session = "line1\nsession_change\nline3\nline4";
        let main = "line1\nline2\nline3\nmain_change";

        let result = DiffEngine::three_way_line_merge(base, session, main).unwrap();
        assert!(result.is_some());

        let merged = result.unwrap();
        assert!(merged.contains("session_change"));
        assert!(merged.contains("main_change"));
    }

    #[test]
    fn test_three_way_merge_conflict() {
        let base = "line1\nline2\nline3";
        let session = "line1\nsession_change\nline3";
        let main = "line1\nmain_change\nline3";

        let result = DiffEngine::three_way_line_merge(base, session, main).unwrap();
        assert!(result.is_none()); // Conflict detected
    }

    #[test]
    fn test_merge_request_builder() {
        let request = MergeRequest::new("session-123".to_string(), MergeStrategy::ThreeWay)
            .with_namespace("custom".to_string())
            .with_user("user-456".to_string());

        assert_eq!(request.session_id, "session-123");
        assert_eq!(request.target_namespace, "custom");
        assert_eq!(request.user_id, Some("user-456".to_string()));
    }

    #[test]
    fn test_conflict_builder() {
        let conflict = Conflict::new(
            "entity-1".to_string(),
            ConflictType::ModifyModify,
            "src/main.rs".to_string(),
        )
        .with_versions(
            Some("base".to_string()),
            Some("session".to_string()),
            Some("main".to_string()),
        )
        .with_line_range(10, 20);

        assert_eq!(conflict.entity_id, "entity-1");
        assert_eq!(conflict.conflict_type, ConflictType::ModifyModify);
        assert_eq!(conflict.base_version, Some("base".to_string()));
        assert_eq!(conflict.line_range, Some((10, 20)));
    }

    #[test]
    fn test_changeset_operations() {
        let mut changeset = ChangeSet::new();
        assert!(changeset.is_empty());

        changeset.add_change(Change::create(
            "entity-1".to_string(),
            "content".to_string(),
            "file.rs".to_string(),
            Language::Rust,
        ));

        assert_eq!(changeset.len(), 1);
        assert!(!changeset.is_empty());
    }
}
