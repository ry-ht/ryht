//! Session Management Tools
//!
//! TODO (Phase 6): Remove cortex_bridge, use cortex-vfs session types

use crate::cortex_bridge::{AgentId, CortexBridge, MergeStrategy, SessionId, SessionScope, WorkspaceId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SessionCreateInput {
    pub workspace_id: String,
}

#[derive(Debug, Serialize)]
pub struct SessionCreateOutput {
    pub session_id: String,
}

pub struct SessionCreateTool {
    cortex: Arc<CortexBridge>,
}

impl SessionCreateTool {
    /// Create a new SessionCreateTool with CortexBridge
    pub fn new(cortex: Arc<CortexBridge>) -> Self {
        Self { cortex }
    }

    pub async fn create(&self, input: SessionCreateInput) -> Result<SessionCreateOutput> {
        // Ensure Cortex is initialized
        self.cortex.ensure_initialized().await?;

        // Generate a unique agent ID for this session
        let agent_id = AgentId::new();

        // Create workspace ID from input
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

        // Create session via CortexBridge
        let session_id = self.cortex
            .create_session(agent_id.clone(), workspace_id, scope)
            .await?;

        info!("Created session {} successfully", session_id);

        Ok(SessionCreateOutput {
            session_id: session_id.0,
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
    cortex: Arc<CortexBridge>,
}

impl SessionMergeTool {
    /// Create a new SessionMergeTool with CortexBridge
    pub fn new(cortex: Arc<CortexBridge>) -> Self {
        Self { cortex }
    }

    pub async fn merge(&self, input: SessionMergeInput) -> Result<SessionMergeOutput> {
        // Ensure Cortex is initialized
        self.cortex.ensure_initialized().await?;

        let session_id = SessionId::from(input.session_id.clone());

        info!("Merging session {}", session_id);

        // Use Auto merge strategy by default
        let strategy = MergeStrategy::Auto;

        // Merge session via CortexBridge
        let merge_report = self.cortex
            .merge_session(&session_id, strategy)
            .await?;

        if merge_report.conflicts_resolved > 0 {
            warn!(
                "Session {} merged with {} conflicts resolved",
                session_id, merge_report.conflicts_resolved
            );
        } else {
            info!(
                "Session {} merged successfully with {} changes",
                session_id, merge_report.changes_merged
            );
        }

        Ok(SessionMergeOutput {
            success: true,
            changes_merged: merge_report.changes_merged,
            conflicts_resolved: merge_report.conflicts_resolved,
        })
    }
}
