//! MCP Tool Context Extensions for Cortex
//!
//! This module provides Cortex-specific context information for MCP tools,
//! including session-based workspace management.

use mcp_sdk::ToolContext;
use cortex_core::id::CortexId;
use cortex_core::error::{CortexError, Result};
use uuid::Uuid;

/// Cortex-specific context extracted from MCP ToolContext
///
/// This provides access to session and workspace information that tools
/// need to operate correctly. It replaces the deprecated "active workspace"
/// concept with session-based workspace binding.
#[derive(Debug, Clone)]
pub struct CortexToolContext {
    /// Session ID if this tool is being called within a session
    pub session_id: Option<CortexId>,

    /// Workspace ID for this operation
    /// Can come from session metadata or be explicitly specified
    pub workspace_id: Option<Uuid>,

    /// Agent ID making the request (if available)
    pub agent_id: Option<String>,
}

impl CortexToolContext {
    /// Extract Cortex context from MCP ToolContext
    ///
    /// This examines the MCP context's metadata to find:
    /// - session_id: The active session making this call
    /// - workspace_id: The workspace this session is bound to
    /// - agent_id: The agent making the request
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mcp_sdk::ToolContext;
    /// use cortex::mcp::context::CortexToolContext;
    ///
    /// async fn my_tool(context: &ToolContext) {
    ///     let cortex_ctx = CortexToolContext::from_mcp_context(context);
    ///     if let Some(workspace_id) = cortex_ctx.workspace_id {
    ///         // Use workspace_id...
    ///     }
    /// }
    /// ```
    pub fn from_mcp_context(mcp_context: &ToolContext) -> Self {
        // Extract session_id from metadata
        let session_id = mcp_context.metadata()
            .get("session_id")
            .and_then(|v| v.as_str())
            .and_then(|s| CortexId::parse(s).ok());

        // Extract workspace_id from metadata
        let workspace_id = mcp_context.metadata()
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        // Extract agent_id from metadata
        let agent_id = mcp_context.metadata()
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self {
            session_id,
            workspace_id,
            agent_id,
        }
    }

    /// Require workspace ID to be present
    ///
    /// Returns an error if workspace_id is not available.
    /// Use this in tools that absolutely need a workspace context.
    ///
    /// # Errors
    ///
    /// Returns `CortexError::InvalidInput` if workspace_id is not set
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use cortex::mcp::context::CortexToolContext;
    /// # async fn example(ctx: CortexToolContext) -> Result<(), Box<dyn std::error::Error>> {
    /// let workspace_id = ctx.require_workspace()?;
    /// // Use workspace_id...
    /// # Ok(())
    /// # }
    /// ```
    pub fn require_workspace(&self) -> Result<Uuid> {
        self.workspace_id
            .ok_or_else(|| CortexError::invalid_input(
                "workspace_id is required but not provided in context. \
                 Set workspace_id in session metadata or pass it as a tool parameter."
            ))
    }

    /// Require session ID to be present
    ///
    /// Returns an error if session_id is not available.
    /// Use this in tools that require session context.
    ///
    /// # Errors
    ///
    /// Returns `CortexError::InvalidInput` if session_id is not set
    pub fn require_session(&self) -> Result<CortexId> {
        self.session_id
            .clone()
            .ok_or_else(|| CortexError::invalid_input(
                "session_id is required but not provided in context. \
                 Ensure this tool is called within an active session."
            ))
    }

    /// Get workspace ID or use the provided default
    ///
    /// This is useful for tools that can work with a specified workspace
    /// or fall back to a default value.
    pub fn workspace_or(&self, default: Uuid) -> Uuid {
        self.workspace_id.unwrap_or(default)
    }

    /// Check if a workspace is set
    pub fn has_workspace(&self) -> bool {
        self.workspace_id.is_some()
    }

    /// Check if a session is set
    pub fn has_session(&self) -> bool {
        self.session_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_from_mcp_context_with_metadata() {
        let session_id = CortexId::new();
        let workspace_id = Uuid::new_v4();

        let mut metadata = HashMap::new();
        metadata.insert("session_id".to_string(), json!(session_id.to_string()));
        metadata.insert("workspace_id".to_string(), json!(workspace_id.to_string()));
        metadata.insert("agent_id".to_string(), json!("test-agent"));

        let tool_context = ToolContext {
            metadata: serde_json::Value::Object(
                metadata.into_iter()
                    .map(|(k, v)| (k, v))
                    .collect()
            ),
        };

        let ctx = CortexToolContext::from_mcp_context(&tool_context);

        assert!(ctx.has_session());
        assert!(ctx.has_workspace());
        assert_eq!(ctx.workspace_id, Some(workspace_id));
        assert_eq!(ctx.agent_id, Some("test-agent".to_string()));
    }

    #[test]
    fn test_from_mcp_context_empty() {
        let tool_context = ToolContext {
            metadata: json!({}),
        };

        let ctx = CortexToolContext::from_mcp_context(&tool_context);

        assert!(!ctx.has_session());
        assert!(!ctx.has_workspace());
        assert_eq!(ctx.workspace_id, None);
        assert_eq!(ctx.agent_id, None);
    }

    #[test]
    fn test_require_workspace_success() {
        let workspace_id = Uuid::new_v4();
        let ctx = CortexToolContext {
            session_id: None,
            workspace_id: Some(workspace_id),
            agent_id: None,
        };

        assert_eq!(ctx.require_workspace().unwrap(), workspace_id);
    }

    #[test]
    fn test_require_workspace_failure() {
        let ctx = CortexToolContext {
            session_id: None,
            workspace_id: None,
            agent_id: None,
        };

        assert!(ctx.require_workspace().is_err());
    }

    #[test]
    fn test_workspace_or() {
        let default_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let ctx_with = CortexToolContext {
            session_id: None,
            workspace_id: Some(workspace_id),
            agent_id: None,
        };
        assert_eq!(ctx_with.workspace_or(default_id), workspace_id);

        let ctx_without = CortexToolContext {
            session_id: None,
            workspace_id: None,
            agent_id: None,
        };
        assert_eq!(ctx_without.workspace_or(default_id), default_id);
    }
}
