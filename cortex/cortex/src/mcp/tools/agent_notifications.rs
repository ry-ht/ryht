//! Agent Notification Tools (3 tools)
//!
//! Provides tools for AI agents to subscribe to and query notifications about
//! code changes, metrics updates, security alerts, and architecture violations.

use async_trait::async_trait;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::services::{AgentNotification, EventType, NotificationService, Severity};

#[derive(Clone)]
pub struct AgentNotificationContext {
    notification_service: Arc<NotificationService>,
}

impl AgentNotificationContext {
    pub fn new(notification_service: Arc<NotificationService>) -> Self {
        Self {
            notification_service,
        }
    }
}

// =============================================================================
// Tool 1: Subscribe to Notifications
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentSubscribeInput {
    /// Agent ID to subscribe
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSubscribeOutput {
    pub success: bool,
    pub message: String,
    pub subscription_id: String,
}

pub struct AgentSubscribeTool;

#[async_trait]
impl Tool for AgentSubscribeTool {
    type Input = AgentSubscribeInput;
    type Output = AgentSubscribeOutput;
    type Error = CortexToolError;
    type Context = AgentNotificationContext;

    fn name(&self) -> String {
        "agent_subscribe".to_string()
    }

    fn description(&self) -> String {
        "Subscribe an agent to receive real-time notifications about code changes, metrics updates, security alerts, and architecture violations.".to_string()
    }

    async fn call(
        &self,
        ctx: &Self::Context,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        info!("Agent {} subscribing to notifications", input.agent_id);

        // Subscribe the agent (returns a receiver, but we don't store it here)
        let _receiver = ctx.notification_service.subscribe(&input.agent_id);

        Ok(AgentSubscribeOutput {
            success: true,
            message: format!(
                "Agent {} subscribed successfully. Use agent_get_notifications to retrieve notifications.",
                input.agent_id
            ),
            subscription_id: input.agent_id.clone(),
        })
    }
}

// =============================================================================
// Tool 2: Get Notification History
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetNotificationsInput {
    /// Optional filter by event types
    #[serde(default)]
    pub event_types: Option<Vec<String>>,

    /// Optional filter by minimum severity
    #[serde(default)]
    pub min_severity: Option<String>,

    /// Maximum number of notifications to retrieve (default: 50)
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNotificationsOutput {
    pub notifications: Vec<NotificationSummary>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSummary {
    pub id: String,
    pub event_type: String,
    pub workspace_id: String,
    pub file_paths: Vec<String>,
    pub severity: String,
    pub description: Option<String>,
    pub timestamp: String,
    pub tags: Vec<String>,
    pub data: serde_json::Value,
}

impl From<AgentNotification> for NotificationSummary {
    fn from(n: AgentNotification) -> Self {
        Self {
            id: n.id.to_string(),
            event_type: format!("{:?}", n.event_type),
            workspace_id: n.workspace_id.to_string(),
            file_paths: n.file_paths,
            severity: n.severity.as_str().to_string(),
            description: n.description,
            timestamp: n.timestamp.to_rfc3339(),
            tags: n.tags,
            data: n.data,
        }
    }
}

pub struct GetNotificationsTool;

#[async_trait]
impl Tool for GetNotificationsTool {
    type Input = GetNotificationsInput;
    type Output = GetNotificationsOutput;
    type Error = CortexToolError;
    type Context = AgentNotificationContext;

    fn name(&self) -> String {
        "agent_get_notifications".to_string()
    }

    fn description(&self) -> String {
        "Retrieve notification history with optional filtering by event type and severity. Returns recent notifications that agents may have missed.".to_string()
    }

    async fn call(
        &self,
        ctx: &Self::Context,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        debug!(
            "Retrieving notifications with filters: event_types={:?}, min_severity={:?}, limit={:?}",
            input.event_types, input.min_severity, input.limit
        );

        let notifications = if let Some(ref event_types) = input.event_types {
            // Filter by event types
            let types: Vec<EventType> = event_types
                .iter()
                .filter_map(|s| match s.to_lowercase().as_str() {
                    "code_changed" => Some(EventType::CodeChanged),
                    "metrics_updated" => Some(EventType::MetricsUpdated),
                    "security_alert" => Some(EventType::SecurityAlert),
                    "architecture_violation" => Some(EventType::ArchitectureViolation),
                    "parse_completed" => Some(EventType::ParseCompleted),
                    "build_completed" => Some(EventType::BuildCompleted),
                    "tests_completed" => Some(EventType::TestsCompleted),
                    "quality_issue" => Some(EventType::QualityIssue),
                    "dependency_updated" => Some(EventType::DependencyUpdated),
                    "workspace_modified" => Some(EventType::WorkspaceModified),
                    _ => None,
                })
                .collect();

            ctx.notification_service
                .get_history_filtered(types, input.limit)
                .await
        } else if let Some(ref severity_str) = input.min_severity {
            // Filter by severity
            let min_severity = match severity_str.to_lowercase().as_str() {
                "critical" => Severity::Critical,
                "high" => Severity::High,
                "medium" => Severity::Medium,
                "low" => Severity::Low,
                _ => Severity::Info,
            };

            ctx.notification_service
                .get_history_by_severity(min_severity, input.limit)
                .await
        } else {
            // No filters, get all
            ctx.notification_service
                .get_history(input.limit)
                .await
        };

        let total_count = notifications.len();
        let summaries: Vec<NotificationSummary> =
            notifications.into_iter().map(Into::into).collect();

        Ok(GetNotificationsOutput {
            notifications: summaries,
            total_count,
        })
    }
}

// =============================================================================
// Tool 3: Get Notification Statistics
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetNotificationStatsInput {
    // No input parameters needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNotificationStatsOutput {
    pub notifications_sent: u64,
    pub notifications_dropped: u64,
    pub active_subscriptions: u64,
    pub history_size: usize,
}

pub struct GetNotificationStatsTool;

#[async_trait]
impl Tool for GetNotificationStatsTool {
    type Input = GetNotificationStatsInput;
    type Output = GetNotificationStatsOutput;
    type Error = CortexToolError;
    type Context = AgentNotificationContext;

    fn name(&self) -> String {
        "agent_notification_stats".to_string()
    }

    fn description(&self) -> String {
        "Get statistics about the notification system including total notifications sent, active subscriptions, and history size.".to_string()
    }

    async fn call(
        &self,
        ctx: &Self::Context,
        _input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let stats = ctx.notification_service.get_stats();
        let history = ctx.notification_service.get_history(None).await;

        Ok(GetNotificationStatsOutput {
            notifications_sent: *stats.get("notifications_sent").unwrap_or(&0),
            notifications_dropped: *stats.get("notifications_dropped").unwrap_or(&0),
            active_subscriptions: *stats.get("active_subscriptions").unwrap_or(&0),
            history_size: history.len(),
        })
    }
}

// =============================================================================
// Error Type
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum CortexToolError {
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<CortexToolError> for ToolError {
    fn from(err: CortexToolError) -> Self {
        ToolError::ExecutionError(err.to_string())
    }
}

// =============================================================================
// Tool Registration
// =============================================================================

pub fn register_agent_notification_tools(registry: &mut ToolRegistry<AgentNotificationContext>) {
    registry.register_tool(Box::new(AgentSubscribeTool));
    registry.register_tool(Box::new(GetNotificationsTool));
    registry.register_tool(Box::new(GetNotificationStatsTool));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> AgentNotificationContext {
        let service = Arc::new(NotificationService::new(10));
        AgentNotificationContext::new(service)
    }

    #[tokio::test]
    async fn test_agent_subscribe() {
        let ctx = create_test_context();
        let tool = AgentSubscribeTool;

        let input = AgentSubscribeInput {
            agent_id: "test_agent_001".to_string(),
        };

        let result = tool.call(&ctx, input).await.unwrap();
        assert!(result.success);
        assert_eq!(result.subscription_id, "test_agent_001");
    }

    #[tokio::test]
    async fn test_get_notifications_no_filter() {
        let ctx = create_test_context();
        let tool = GetNotificationsTool;

        // Add some test notifications
        let workspace_id = uuid::Uuid::new_v4();
        ctx.notification_service.notify(
            AgentNotification::code_changed(
                workspace_id,
                vec!["test.rs".to_string()],
                serde_json::json!({}),
            ),
        );

        // Wait a bit for async storage
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let input = GetNotificationsInput {
            event_types: None,
            min_severity: None,
            limit: Some(10),
        };

        let result = tool.call(&ctx, input).await.unwrap();
        assert!(result.total_count > 0);
        assert!(!result.notifications.is_empty());
    }

    #[tokio::test]
    async fn test_get_notifications_with_event_filter() {
        let ctx = create_test_context();
        let tool = GetNotificationsTool;

        let workspace_id = uuid::Uuid::new_v4();

        // Add different types of notifications
        ctx.notification_service.notify(
            AgentNotification::code_changed(
                workspace_id,
                vec!["a.rs".to_string()],
                serde_json::json!({}),
            ),
        );

        ctx.notification_service.notify(
            AgentNotification::security_alert(
                workspace_id,
                vec!["b.rs".to_string()],
                serde_json::json!({}),
                Severity::High,
            ),
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Filter for security alerts only
        let input = GetNotificationsInput {
            event_types: Some(vec!["security_alert".to_string()]),
            min_severity: None,
            limit: Some(10),
        };

        let result = tool.call(&ctx, input).await.unwrap();
        assert!(result.total_count > 0);
        assert!(result
            .notifications
            .iter()
            .all(|n| n.event_type == "SecurityAlert"));
    }

    #[tokio::test]
    async fn test_get_notification_stats() {
        let ctx = create_test_context();
        let tool = GetNotificationStatsTool;

        // Send a notification
        let workspace_id = uuid::Uuid::new_v4();
        ctx.notification_service.notify(
            AgentNotification::code_changed(
                workspace_id,
                vec!["test.rs".to_string()],
                serde_json::json!({}),
            ),
        );

        let input = GetNotificationStatsInput {};
        let result = tool.call(&ctx, input).await.unwrap();

        assert!(result.notifications_sent > 0 || result.notifications_dropped > 0);
    }
}
