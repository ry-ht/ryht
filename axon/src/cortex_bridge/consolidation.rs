//! Memory consolidation and pattern learning for Cortex integration
//!
//! This module handles the transfer of memories from working/short-term storage
//! to long-term episodic and semantic memory, and performs pattern extraction.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ConsolidateSessionRequest {
    pub agent_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsolidateSessionResponse {
    pub report: ConsolidationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractPatternsRequest {
    pub workspace_id: String,
    pub min_occurrences: u32,
    pub episode_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractPatternsResponse {
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DreamResponse {
    pub report: DreamReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct MaterializeCodeRequest {
    pub representation: CodeRepresentation,
    pub target_language: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MaterializeCodeResponse {
    pub code: MaterializedCode,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncSessionRequest {
    pub session_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncSessionResponse {
    pub report: SyncReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeAndIndexRequest {
    pub workspace_id: String,
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeAndIndexResponse {
    pub result: CodeAnalysisResult,
}

// ============================================================================
// Consolidation Manager
// ============================================================================

/// Consolidation manager for memory operations
pub struct ConsolidationManager {
    client: CortexClient,
}

impl ConsolidationManager {
    /// Create a new consolidation manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Consolidate working memory for a session into long-term memory
    pub async fn consolidate_session(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<ConsolidationReport> {
        let request = ConsolidateSessionRequest {
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
        };

        let response: ConsolidateSessionResponse = self
            .client
            .post("/memory/consolidate", &request)
            .await?;

        info!(
            "Consolidated session {} for agent {}: {} items, {} patterns",
            session_id, agent_id, response.report.items_consolidated, response.report.patterns_extracted
        );

        Ok(response.report)
    }

    /// Extract patterns from recent episodes
    pub async fn extract_patterns(
        &self,
        workspace_id: &WorkspaceId,
        min_occurrences: u32,
    ) -> Result<Vec<Pattern>> {
        let request = ExtractPatternsRequest {
            workspace_id: workspace_id.to_string(),
            min_occurrences,
            episode_types: vec![
                "feature".to_string(),
                "refactor".to_string(),
                "bugfix".to_string(),
            ],
        };

        let response: ExtractPatternsResponse = self
            .client
            .post("/memory/patterns/extract", &request)
            .await?;

        info!(
            "Extracted {} patterns from workspace {}",
            response.patterns.len(),
            workspace_id
        );

        Ok(response.patterns)
    }

    /// Perform dream-like consolidation (offline learning)
    pub async fn dream(&self) -> Result<DreamReport> {
        let response: DreamResponse = self
            .client
            .post("/memory/dream", &serde_json::json!({}))
            .await?;

        info!(
            "Dream consolidation complete: {} new patterns, {} refined, {} forgotten",
            response.report.new_patterns, response.report.patterns_refined, response.report.memories_forgotten
        );

        Ok(response.report)
    }

    /// Materialize code from memory representation
    pub async fn materialize_code(
        &self,
        session_id: &SessionId,
        representation: CodeRepresentation,
    ) -> Result<MaterializedCode> {
        let request = MaterializeCodeRequest {
            representation,
            target_language: None,
        };

        let response: MaterializeCodeResponse = self
            .client
            .post(&format!("/sessions/{}/materialize", session_id), &request)
            .await?;

        info!(
            "Materialized code for session {}: {} units extracted",
            session_id, response.code.analysis.units_extracted
        );

        Ok(response.code)
    }

    /// Sync code changes from session to semantic memory
    pub async fn sync_session(
        &self,
        session_id: &SessionId,
        workspace_id: &WorkspaceId,
    ) -> Result<SyncReport> {
        let request = SyncSessionRequest {
            session_id: session_id.to_string(),
            workspace_id: workspace_id.to_string(),
        };

        let response: SyncSessionResponse = self
            .client
            .post("/memory/sync", &request)
            .await?;

        info!(
            "Synced session {} to workspace {}: {} files, {} units",
            session_id, workspace_id, response.report.files_synced, response.report.units_updated
        );

        Ok(response.report)
    }

    /// Analyze and index code for semantic search
    pub async fn analyze_and_index(
        &self,
        workspace_id: &WorkspaceId,
        file_path: &str,
        content: &str,
    ) -> Result<CodeAnalysisResult> {
        let request = AnalyzeAndIndexRequest {
            workspace_id: workspace_id.to_string(),
            file_path: file_path.to_string(),
            content: content.to_string(),
        };

        let response: AnalyzeAndIndexResponse = self
            .client
            .post("/code/analyze", &request)
            .await?;

        info!(
            "Analyzed and indexed {}: {} units, {} dependencies",
            file_path, response.result.units_extracted, response.result.dependencies_found
        );

        Ok(response.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consolidation_report_structure() {
        let report = ConsolidationReport {
            items_consolidated: 10,
            patterns_extracted: 2,
            semantic_units_created: 5,
            duration_ms: 1500,
            memory_freed_bytes: 2048,
        };

        assert_eq!(report.items_consolidated, 10);
        assert_eq!(report.patterns_extracted, 2);
    }

    #[test]
    fn test_dream_report_structure() {
        let report = DreamReport {
            new_patterns: 3,
            patterns_refined: 5,
            memories_forgotten: 10,
            duration_ms: 5000,
        };

        assert_eq!(report.new_patterns, 3);
        assert_eq!(report.patterns_refined, 5);
    }

    #[test]
    fn test_code_representation() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("author".to_string(), "test".to_string());

        let repr = CodeRepresentation {
            repr_type: "semantic".to_string(),
            language: "rust".to_string(),
            description: "Test function".to_string(),
            structure: serde_json::json!({"type": "function"}),
            dependencies: vec!["std::fmt".to_string()],
            metadata,
        };

        assert_eq!(repr.language, "rust");
        assert_eq!(repr.dependencies.len(), 1);
    }

    #[test]
    fn test_sync_report_structure() {
        let report = SyncReport {
            files_synced: 5,
            units_updated: 15,
            dependencies_updated: 20,
            conflicts: 0,
            duration_ms: 2000,
        };

        assert_eq!(report.files_synced, 5);
        assert_eq!(report.conflicts, 0);
    }
}
