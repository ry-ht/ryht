//! Session Management Tools
//!
//! Direct integration with Cortex subsystems (no HTTP bridge).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use cortex_storage::{ConnectionManager, SessionManager, WorkspaceId, SessionId, AgentSession, IsolationLevel, SessionScope};
use cortex_storage::locks::LockManager;
use mcp_sdk::prelude::*;
use async_trait::async_trait;

/// Context for session management with direct subsystem references
#[derive(Clone)]
pub struct SessionContext {
    /// Session manager for creating and managing sessions
    pub sessions: Arc<SessionManager>,
    /// Lock manager for resource coordination
    pub locks: Arc<LockManager>,
    /// Storage backend
    pub storage: Arc<ConnectionManager>,
}

impl SessionContext {
    /// Create a new SessionContext
    pub fn new(
        sessions: Arc<SessionManager>,
        locks: Arc<LockManager>,
        storage: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            sessions,
            locks,
            storage,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionCreateInput {
    pub workspace_id: String,
}

#[derive(Debug, Serialize)]
pub struct SessionCreateOutput {
    pub session_id: String,
}

pub struct SessionCreateTool {
    context: SessionContext,
}

impl SessionCreateTool {
    /// Create a new SessionCreateTool with SessionContext
    pub fn new(context: SessionContext) -> Self {
        Self { context }
    }

    pub async fn create(&self, input: SessionCreateInput) -> Result<SessionCreateOutput> {
        // Generate a unique agent ID for this session
        let agent_id = format!("agent-{}", uuid::Uuid::new_v4());

        // Parse workspace ID from input
        let workspace_id = WorkspaceId::from(input.workspace_id.clone());

        // Create default session scope (all paths accessible)
        let scope = SessionScope {
            paths: vec!["/".to_string()],
            read_only_paths: vec![],
        };

        info!(
            "Creating session for workspace {} with agent {}",
            workspace_id, agent_id
        );

        // Create session directly via SessionManager
        let session = AgentSession::new(
            agent_id.clone(),
            workspace_id.clone(),
            IsolationLevel::Snapshot,
            scope,
        );

        // Store session
        let session_id = self.context.sessions.create_session(session).await?;

        info!("Created session {} successfully", session_id);

        Ok(SessionCreateOutput {
            session_id: session_id.to_string(),
        })
    }
}

#[async_trait]
impl Tool for SessionCreateTool {
    fn name(&self) -> &str {
        "axon.session.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create an isolated work session for an agent")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SessionCreateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.create(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: false,
        })
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionMergeInput {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct SessionMergeOutput {
    pub success: bool,
    pub changes_merged: u32,
    pub conflicts_resolved: u32,
}

pub struct SessionMergeTool {
    context: SessionContext,
}

impl SessionMergeTool {
    /// Create a new SessionMergeTool with SessionContext
    pub fn new(context: SessionContext) -> Self {
        Self { context }
    }

    pub async fn merge(&self, input: SessionMergeInput) -> Result<SessionMergeOutput> {
        let session_id = SessionId::from(input.session_id.clone());

        info!("Merging session {}", session_id);

        // Use Auto merge strategy by default
        let strategy = cortex_storage::merge::MergeStrategy::Auto;

        // Merge session directly via SessionManager
        let merge_result = self.context.sessions
            .merge_session(&session_id, strategy)
            .await?;

        let conflicts_count = merge_result.conflicts.len() as u32;
        let changes_count = merge_result.merged_entities.len() as u32;

        if conflicts_count > 0 {
            warn!(
                "Session {} merged with {} conflicts",
                session_id, conflicts_count
            );
        } else {
            info!(
                "Session {} merged successfully with {} changes",
                session_id, changes_count
            );
        }

        Ok(SessionMergeOutput {
            success: merge_result.success,
            changes_merged: changes_count,
            conflicts_resolved: conflicts_count,
        })
    }
}

#[async_trait]
impl Tool for SessionMergeTool {
    fn name(&self) -> &str {
        "axon.session.merge"
    }

    fn description(&self) -> Option<&str> {
        Some("Merge an isolated session back into the main workspace")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SessionMergeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionMergeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.merge(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: false,
        })
    }
}
