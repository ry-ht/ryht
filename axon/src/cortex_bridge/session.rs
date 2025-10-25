//! Session management for Cortex integration
//!
//! This module handles session lifecycle: creation, file operations,
//! merging, and cleanup.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct CreateSessionRequest {
    agent_id: String,
    workspace_id: String,
    scope: SessionScopeRequest,
    isolation_level: String,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SessionScopeRequest {
    paths: Vec<String>,
    read_only_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub token: String,
    pub expires_at: String,
    pub base_version: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStatusResponse {
    pub session_id: String,
    pub agent_id: String,
    pub workspace_id: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub change_count: u32,
}

impl From<SessionStatusResponse> for SessionStatus {
    fn from(response: SessionStatusResponse) -> Self {
        Self {
            session_id: SessionId(response.session_id),
            agent_id: AgentId(response.agent_id),
            workspace_id: WorkspaceId(response.workspace_id),
            status: response.status,
            created_at: chrono::Utc::now(), // Parse from string in production
            expires_at: chrono::Utc::now(),  // Parse from string in production
            change_count: response.change_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct UpdateFileRequest {
    content: String,
    expected_version: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileContentResponse {
    pub content: String,
    pub encoding: String,
    pub version: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListFilesResponse {
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize)]
struct MergeSessionRequest {
    strategy: String,
    conflict_resolution: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MergeReportResponse {
    pub changes_merged: u32,
    pub conflicts_resolved: u32,
    pub new_version: u64,
}

// ============================================================================
// Session Manager
// ============================================================================

/// Session manager for Cortex sessions
pub struct SessionManager {
    client: CortexClient,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Create a new session
    pub async fn create_session(
        &self,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        scope: SessionScope,
    ) -> Result<SessionId> {
        let request = CreateSessionRequest {
            agent_id: agent_id.0.clone(),
            workspace_id: workspace_id.0.clone(),
            scope: SessionScopeRequest {
                paths: scope.paths.clone(),
                read_only_paths: scope.read_only_paths.clone(),
            },
            isolation_level: "snapshot".to_string(),
            ttl_seconds: 3600,
        };

        let response: CreateSessionResponse = self
            .client
            .post("/sessions", &request)
            .await?;

        let session_id = SessionId::from(response.session_id);
        info!("Created session {} for agent {}", session_id, agent_id);

        Ok(session_id)
    }

    /// Get session status
    pub async fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus> {
        let path = format!("/sessions/{}", session_id);
        let response: SessionStatusResponse = self.client.get(&path).await?;
        Ok(response.into())
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &SessionId) -> Result<()> {
        let path = format!("/sessions/{}", session_id);

        // DELETE may return empty response, so we use serde_json::Value
        let _: serde_json::Value = self.client.delete(&path).await?;

        info!("Closed session {}", session_id);
        Ok(())
    }

    /// Read a file from session
    pub async fn read_file(&self, session_id: &SessionId, path: &str) -> Result<String> {
        let encoded_path = urlencoding::encode(path);
        let url = format!("/sessions/{}/files/{}", session_id, encoded_path);

        let response: FileContentResponse = self.client.get(&url).await?;
        Ok(response.content)
    }

    /// Write a file to session
    pub async fn write_file(
        &self,
        session_id: &SessionId,
        path: &str,
        content: &str,
    ) -> Result<()> {
        let request = UpdateFileRequest {
            content: content.to_string(),
            expected_version: None,
        };

        let encoded_path = urlencoding::encode(path);
        let url = format!("/sessions/{}/files/{}", session_id, encoded_path);

        let _: serde_json::Value = self.client.put(&url, &request).await?;

        info!("Wrote file {} to session {}", path, session_id);
        Ok(())
    }

    /// List files in session
    pub async fn list_files(&self, session_id: &SessionId, path: &str) -> Result<Vec<FileInfo>> {
        let url = format!(
            "/sessions/{}/files?path={}&recursive=true",
            session_id,
            urlencoding::encode(path)
        );

        let response: ListFilesResponse = self.client.get(&url).await?;
        Ok(response.files)
    }

    /// Merge session changes
    pub async fn merge_session(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeReport> {
        let request = MergeSessionRequest {
            strategy: strategy.to_string(),
            conflict_resolution: None,
        };

        let url = format!("/sessions/{}/merge", session_id);
        let response: MergeReportResponse = self.client.post(&url, &request).await?;

        if response.conflicts_resolved > 0 {
            warn!(
                "Merged session {} with {} conflicts resolved",
                session_id, response.conflicts_resolved
            );
        } else {
            info!("Merged session {} successfully", session_id);
        }

        Ok(MergeReport {
            changes_merged: response.changes_merged,
            conflicts_resolved: response.conflicts_resolved,
            new_version: response.new_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_scope() {
        let scope = SessionScope {
            paths: vec!["src/".to_string()],
            read_only_paths: vec!["tests/".to_string()],
        };

        assert_eq!(scope.paths.len(), 1);
        assert_eq!(scope.read_only_paths.len(), 1);
    }

    #[test]
    fn test_merge_strategy_display() {
        assert_eq!(MergeStrategy::Auto.to_string(), "auto");
        assert_eq!(MergeStrategy::Manual.to_string(), "manual");
        assert_eq!(MergeStrategy::Theirs.to_string(), "theirs");
        assert_eq!(MergeStrategy::Mine.to_string(), "mine");
    }
}
