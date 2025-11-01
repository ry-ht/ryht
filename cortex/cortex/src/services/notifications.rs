//! Agent notification system for code changes, metrics updates, and events
//!
//! This module provides a centralized notification system that allows AI agents to
//! subscribe to events about code changes, metrics updates, security issues, and
//! architecture violations.
//!
//! # Architecture
//!
//! - Uses tokio::sync::broadcast channels for efficient event distribution
//! - Stores recent notifications (last 100) for late-joining agents
//! - Supports filtering by event type and severity
//! - Thread-safe and async-first design
//!
//! # Example
//!
//! ```no_run
//! use cortex::services::notifications::{NotificationService, AgentNotification, EventType};
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let service = Arc::new(NotificationService::new(100));
//!
//! // Agent subscribes to notifications
//! let mut receiver = service.subscribe("agent_001");
//!
//! // System sends notification about code change
//! service.notify(AgentNotification::code_changed(
//!     workspace_id,
//!     vec!["src/main.rs".to_string()],
//!     serde_json::json!({"lines_changed": 42}),
//! ));
//!
//! // Agent receives notification
//! while let Ok(notification) = receiver.recv().await {
//!     println!("Received: {:?}", notification);
//! }
//! # }
//! ```

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Maximum number of recent notifications to store
const DEFAULT_NOTIFICATION_HISTORY: usize = 100;

/// Notification service for distributing events to agents
#[derive(Clone)]
pub struct NotificationService {
    /// Broadcast channel for real-time notifications
    sender: broadcast::Sender<AgentNotification>,

    /// Recent notification history for late-joining agents
    history: Arc<RwLock<VecDeque<AgentNotification>>>,

    /// Maximum history size
    max_history: usize,

    /// Active agent subscriptions (agent_id -> subscription count)
    subscriptions: Arc<DashMap<String, usize>>,

    /// Statistics
    stats: Arc<DashMap<String, u64>>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(max_history: usize) -> Self {
        // Create broadcast channel with buffer for 1000 notifications
        let (sender, _) = broadcast::channel(1000);

        let stats = Arc::new(DashMap::new());
        stats.insert("notifications_sent".to_string(), 0);
        stats.insert("notifications_dropped".to_string(), 0);
        stats.insert("active_subscriptions".to_string(), 0);

        info!("NotificationService initialized with history size: {}", max_history);

        Self {
            sender,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            subscriptions: Arc::new(DashMap::new()),
            stats,
        }
    }

    /// Create with default history size
    pub fn with_default_history() -> Self {
        Self::new(DEFAULT_NOTIFICATION_HISTORY)
    }

    /// Subscribe an agent to notifications
    ///
    /// Returns a receiver that will receive all future notifications.
    /// The agent should keep the receiver alive and poll it for notifications.
    pub fn subscribe(&self, agent_id: &str) -> broadcast::Receiver<AgentNotification> {
        let receiver = self.sender.subscribe();

        // Track subscription
        self.subscriptions
            .entry(agent_id.to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // Update stats
        if let Some(mut count) = self.stats.get_mut("active_subscriptions") {
            *count = self.subscriptions.len() as u64;
        }

        info!("Agent {} subscribed to notifications", agent_id);
        receiver
    }

    /// Unsubscribe an agent from notifications
    pub fn unsubscribe(&self, agent_id: &str) {
        if let Some(mut entry) = self.subscriptions.get_mut(agent_id) {
            *entry -= 1;
            if *entry == 0 {
                drop(entry);
                self.subscriptions.remove(agent_id);
            }
        }

        // Update stats
        if let Some(mut count) = self.stats.get_mut("active_subscriptions") {
            *count = self.subscriptions.len() as u64;
        }

        debug!("Agent {} unsubscribed from notifications", agent_id);
    }

    /// Send a notification to all subscribed agents
    pub fn notify(&self, notification: AgentNotification) {
        debug!(
            "Sending notification: type={:?}, severity={:?}, workspace={}",
            notification.event_type,
            notification.severity,
            notification.workspace_id
        );

        // Send to broadcast channel
        match self.sender.send(notification.clone()) {
            Ok(receiver_count) => {
                debug!("Notification sent to {} receivers", receiver_count);
                if let Some(mut count) = self.stats.get_mut("notifications_sent") {
                    *count += 1;
                }
            }
            Err(_) => {
                warn!("No active receivers for notification");
                if let Some(mut count) = self.stats.get_mut("notifications_dropped") {
                    *count += 1;
                }
            }
        }

        // Store in history (async task to avoid blocking)
        let history = Arc::clone(&self.history);
        let max_history = self.max_history;
        tokio::spawn(async move {
            let mut hist = history.write().await;
            hist.push_back(notification);

            // Trim history if needed
            while hist.len() > max_history {
                hist.pop_front();
            }
        });
    }

    /// Get recent notification history
    ///
    /// Useful for agents that connect after notifications were sent.
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<AgentNotification> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(self.max_history).min(history.len());

        history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent history filtered by event type
    pub async fn get_history_filtered(
        &self,
        event_types: Vec<EventType>,
        limit: Option<usize>,
    ) -> Vec<AgentNotification> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(self.max_history);

        history.iter()
            .rev()
            .filter(|n| event_types.contains(&n.event_type))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get recent history filtered by severity
    pub async fn get_history_by_severity(
        &self,
        min_severity: Severity,
        limit: Option<usize>,
    ) -> Vec<AgentNotification> {
        let history = self.history.read().await;
        let limit = limit.unwrap_or(self.max_history);

        history.iter()
            .rev()
            .filter(|n| n.severity >= min_severity)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> std::collections::HashMap<String, u64> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }

    /// Get number of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Clear notification history
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
        info!("Notification history cleared");
    }
}

/// Type of notification event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// File was created, modified, or deleted
    CodeChanged,

    /// Code metrics were updated (complexity, quality, etc.)
    MetricsUpdated,

    /// Security vulnerability detected
    SecurityAlert,

    /// Architecture constraint violated
    ArchitectureViolation,

    /// File parsing completed
    ParseCompleted,

    /// Build completed (success or failure)
    BuildCompleted,

    /// Tests completed
    TestsCompleted,

    /// Code quality issue detected
    QualityIssue,

    /// Dependency updated
    DependencyUpdated,

    /// Workspace modified
    WorkspaceModified,
}

/// Severity level for notifications
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        }
    }
}

/// Notification sent to agents about events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNotification {
    /// Unique notification ID
    pub id: Uuid,

    /// Type of event
    pub event_type: EventType,

    /// Workspace ID where event occurred
    pub workspace_id: Uuid,

    /// File paths affected by the event
    pub file_paths: Vec<String>,

    /// Additional event data
    pub data: Value,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Severity level
    pub severity: Severity,

    /// Optional description
    pub description: Option<String>,

    /// Optional tags for categorization
    pub tags: Vec<String>,
}

impl AgentNotification {
    /// Create a new notification
    pub fn new(
        event_type: EventType,
        workspace_id: Uuid,
        file_paths: Vec<String>,
        data: Value,
        severity: Severity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            workspace_id,
            file_paths,
            data,
            timestamp: Utc::now(),
            severity,
            description: None,
            tags: Vec::new(),
        }
    }

    /// Create a code changed notification
    pub fn code_changed(workspace_id: Uuid, file_paths: Vec<String>, data: Value) -> Self {
        Self::new(
            EventType::CodeChanged,
            workspace_id,
            file_paths,
            data,
            Severity::Info,
        )
    }

    /// Create a metrics updated notification
    pub fn metrics_updated(workspace_id: Uuid, file_paths: Vec<String>, data: Value) -> Self {
        Self::new(
            EventType::MetricsUpdated,
            workspace_id,
            file_paths,
            data,
            Severity::Info,
        )
    }

    /// Create a security alert notification
    pub fn security_alert(
        workspace_id: Uuid,
        file_paths: Vec<String>,
        data: Value,
        severity: Severity,
    ) -> Self {
        Self::new(
            EventType::SecurityAlert,
            workspace_id,
            file_paths,
            data,
            severity,
        )
    }

    /// Create an architecture violation notification
    pub fn architecture_violation(
        workspace_id: Uuid,
        file_paths: Vec<String>,
        data: Value,
        severity: Severity,
    ) -> Self {
        Self::new(
            EventType::ArchitectureViolation,
            workspace_id,
            file_paths,
            data,
            severity,
        )
    }

    /// Create a parse completed notification
    pub fn parse_completed(workspace_id: Uuid, file_paths: Vec<String>, data: Value) -> Self {
        Self::new(
            EventType::ParseCompleted,
            workspace_id,
            file_paths,
            data,
            Severity::Info,
        )
    }

    /// Create a quality issue notification
    pub fn quality_issue(
        workspace_id: Uuid,
        file_paths: Vec<String>,
        data: Value,
        severity: Severity,
    ) -> Self {
        Self::new(
            EventType::QualityIssue,
            workspace_id,
            file_paths,
            data,
            severity,
        )
    }

    /// Add a description to the notification
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add tags to the notification
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Update severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_notification_service_creation() {
        let service = NotificationService::new(50);
        assert_eq!(service.subscription_count(), 0);

        let stats = service.get_stats();
        assert_eq!(stats.get("notifications_sent"), Some(&0));
        assert_eq!(stats.get("active_subscriptions"), Some(&0));
    }

    #[tokio::test]
    async fn test_subscribe_and_notify() {
        let service = NotificationService::new(10);
        let mut receiver = service.subscribe("test_agent");

        assert_eq!(service.subscription_count(), 1);

        // Send notification
        let workspace_id = Uuid::new_v4();
        let notification = AgentNotification::code_changed(
            workspace_id,
            vec!["src/main.rs".to_string()],
            serde_json::json!({"lines_changed": 42}),
        );

        service.notify(notification.clone());

        // Receive notification
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.event_type, EventType::CodeChanged);
        assert_eq!(received.workspace_id, workspace_id);
        assert_eq!(received.file_paths, vec!["src/main.rs".to_string()]);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let service = NotificationService::new(10);
        let mut receiver1 = service.subscribe("agent_1");
        let mut receiver2 = service.subscribe("agent_2");

        assert_eq!(service.subscription_count(), 2);

        let workspace_id = Uuid::new_v4();
        let notification = AgentNotification::security_alert(
            workspace_id,
            vec!["src/vulnerable.rs".to_string()],
            serde_json::json!({"cwe": "CWE-79"}),
            Severity::High,
        );

        service.notify(notification.clone());

        // Both receivers should get the notification
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.event_type, EventType::SecurityAlert);
        assert_eq!(received2.event_type, EventType::SecurityAlert);
        assert_eq!(received1.severity, Severity::High);
    }

    #[tokio::test]
    async fn test_notification_history() {
        let service = NotificationService::new(5);
        let workspace_id = Uuid::new_v4();

        // Send multiple notifications
        for i in 0..7 {
            let notification = AgentNotification::metrics_updated(
                workspace_id,
                vec![format!("file_{}.rs", i)],
                serde_json::json!({"complexity": i}),
            );
            service.notify(notification);
        }

        // Small delay to allow history to be updated
        sleep(Duration::from_millis(10)).await;

        // Should only keep last 5
        let history = service.get_history(None).await;
        assert_eq!(history.len(), 5);

        // Most recent should be file_6.rs
        assert_eq!(history[0].file_paths[0], "file_6.rs");
    }

    #[tokio::test]
    async fn test_filtered_history() {
        let service = NotificationService::new(50);
        let workspace_id = Uuid::new_v4();

        // Send different types of notifications
        service.notify(AgentNotification::code_changed(
            workspace_id,
            vec!["a.rs".to_string()],
            serde_json::json!({}),
        ));

        service.notify(AgentNotification::security_alert(
            workspace_id,
            vec!["b.rs".to_string()],
            serde_json::json!({}),
            Severity::High,
        ));

        service.notify(AgentNotification::code_changed(
            workspace_id,
            vec!["c.rs".to_string()],
            serde_json::json!({}),
        ));

        sleep(Duration::from_millis(10)).await;

        // Filter by event type
        let filtered = service
            .get_history_filtered(vec![EventType::CodeChanged], None)
            .await;

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|n| n.event_type == EventType::CodeChanged));
    }

    #[tokio::test]
    async fn test_severity_filtering() {
        let service = NotificationService::new(50);
        let workspace_id = Uuid::new_v4();

        // Send notifications with different severities
        service.notify(AgentNotification::code_changed(
            workspace_id,
            vec!["a.rs".to_string()],
            serde_json::json!({}),
        )); // Info

        service.notify(AgentNotification::security_alert(
            workspace_id,
            vec!["b.rs".to_string()],
            serde_json::json!({}),
            Severity::Critical,
        ));

        service.notify(AgentNotification::quality_issue(
            workspace_id,
            vec!["c.rs".to_string()],
            serde_json::json!({}),
            Severity::Medium,
        ));

        sleep(Duration::from_millis(10)).await;

        // Get only high severity and above
        let high_severity = service
            .get_history_by_severity(Severity::High, None)
            .await;

        assert_eq!(high_severity.len(), 1);
        assert_eq!(high_severity[0].severity, Severity::Critical);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let service = NotificationService::new(10);
        let _receiver = service.subscribe("agent_1");

        assert_eq!(service.subscription_count(), 1);

        service.unsubscribe("agent_1");
        assert_eq!(service.subscription_count(), 0);
    }

    #[tokio::test]
    async fn test_notification_builders() {
        let workspace_id = Uuid::new_v4();

        // Test builder methods
        let notification = AgentNotification::code_changed(
            workspace_id,
            vec!["test.rs".to_string()],
            serde_json::json!({}),
        )
        .with_description("File was modified".to_string())
        .with_tags(vec!["urgent".to_string(), "review".to_string()])
        .with_severity(Severity::Medium);

        assert_eq!(notification.severity, Severity::Medium);
        assert_eq!(notification.description, Some("File was modified".to_string()));
        assert_eq!(notification.tags.len(), 2);
    }
}
