//! Integration helpers for connecting notification service with other cortex components
//!
//! This module provides helpers to integrate the notification service with:
//! - FileWatcher
//! - AutoReparseHandle
//! - Quality analysis tools
//! - Security scanners
//! - Architecture analysis

use super::notifications::{AgentNotification, EventType, NotificationService, Severity};
use std::sync::Arc;
use uuid::Uuid;

/// Create a notification callback for FileWatcher
///
/// This callback will be invoked when files are changed, and will send
/// CodeChanged notifications to all subscribed agents.
pub fn create_file_watcher_callback(
    notification_service: Arc<NotificationService>,
) -> Arc<dyn Fn(Uuid, Vec<String>, serde_json::Value) + Send + Sync> {
    Arc::new(move |workspace_id, file_paths, data| {
        let notification = AgentNotification::code_changed(workspace_id, file_paths, data)
            .with_description("Files changed in workspace".to_string())
            .with_tags(vec!["file_watcher".to_string(), "code_change".to_string()]);

        notification_service.notify(notification);
    })
}

/// Create a notification callback for AutoReparseHandle
///
/// This callback will be invoked when file parsing completes, and will send
/// ParseCompleted notifications with metrics data.
pub fn create_auto_reparse_callback(
    notification_service: Arc<NotificationService>,
) -> Arc<dyn Fn(Uuid, &str, serde_json::Value) + Send + Sync> {
    Arc::new(move |workspace_id, file_path, data| {
        let notification = AgentNotification::parse_completed(
            workspace_id,
            vec![file_path.to_string()],
            data.clone(),
        )
        .with_description(format!("Parsing completed for {}", file_path))
        .with_tags(vec![
            "auto_reparse".to_string(),
            "parsing".to_string(),
            "metrics".to_string(),
        ]);

        // Determine severity based on parse result
        let severity = if data.get("status").and_then(|v| v.as_str()) == Some("error") {
            Severity::Medium
        } else if data.get("errors").and_then(|v| v.as_u64()).unwrap_or(0) > 0 {
            Severity::Low
        } else {
            Severity::Info
        };

        notification_service.notify(notification.with_severity(severity));
    })
}

/// Send security alert notification
///
/// Use this when security vulnerabilities are detected during scanning.
pub fn send_security_alert(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    file_paths: Vec<String>,
    vulnerability_id: &str,
    severity: Severity,
    cwe_id: Option<&str>,
    description: String,
) {
    let mut data = serde_json::json!({
        "vulnerability_id": vulnerability_id,
        "description": description,
    });

    if let Some(cwe) = cwe_id {
        data["cwe_id"] = serde_json::Value::String(cwe.to_string());
    }

    let notification = AgentNotification::security_alert(workspace_id, file_paths, data, severity)
        .with_description(description)
        .with_tags(vec![
            "security".to_string(),
            "vulnerability".to_string(),
            vulnerability_id.to_string(),
        ]);

    notification_service.notify(notification);
}

/// Send architecture violation notification
///
/// Use this when architectural constraints are violated.
pub fn send_architecture_violation(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    file_paths: Vec<String>,
    violation_type: &str,
    severity: Severity,
    description: String,
    details: serde_json::Value,
) {
    let mut data = details;
    data["violation_type"] = serde_json::Value::String(violation_type.to_string());

    let notification =
        AgentNotification::architecture_violation(workspace_id, file_paths, data, severity)
            .with_description(description)
            .with_tags(vec![
                "architecture".to_string(),
                "violation".to_string(),
                violation_type.to_string(),
            ]);

    notification_service.notify(notification);
}

/// Send quality issue notification
///
/// Use this when code quality issues are detected (code smells, complexity, etc.)
pub fn send_quality_issue(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    file_paths: Vec<String>,
    issue_type: &str,
    severity: Severity,
    metrics: serde_json::Value,
) {
    let data = serde_json::json!({
        "issue_type": issue_type,
        "metrics": metrics,
    });

    let description = format!("Code quality issue detected: {}", issue_type);

    let notification = AgentNotification::quality_issue(workspace_id, file_paths, data, severity)
        .with_description(description)
        .with_tags(vec![
            "quality".to_string(),
            "code_smell".to_string(),
            issue_type.to_string(),
        ]);

    notification_service.notify(notification);
}

/// Send metrics updated notification
///
/// Use this when code metrics are recalculated (complexity, maintainability, etc.)
pub fn send_metrics_update(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    file_paths: Vec<String>,
    metrics: serde_json::Value,
) {
    let notification = AgentNotification::metrics_updated(workspace_id, file_paths, metrics)
        .with_description("Code metrics updated".to_string())
        .with_tags(vec!["metrics".to_string(), "quality".to_string()]);

    notification_service.notify(notification);
}

/// Send build completed notification
pub fn send_build_completed(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    success: bool,
    duration_ms: u64,
    output: Option<String>,
) {
    let data = serde_json::json!({
        "success": success,
        "duration_ms": duration_ms,
        "output": output,
    });

    let severity = if success {
        Severity::Info
    } else {
        Severity::Medium
    };

    let description = if success {
        "Build completed successfully".to_string()
    } else {
        "Build failed".to_string()
    };

    let notification = AgentNotification::new(
        EventType::BuildCompleted,
        workspace_id,
        vec![],
        data,
        severity,
    )
    .with_description(description)
    .with_tags(vec!["build".to_string()]);

    notification_service.notify(notification);
}

/// Send tests completed notification
pub fn send_tests_completed(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    total: u32,
    passed: u32,
    failed: u32,
    skipped: u32,
    duration_ms: u64,
) {
    let data = serde_json::json!({
        "total": total,
        "passed": passed,
        "failed": failed,
        "skipped": skipped,
        "duration_ms": duration_ms,
    });

    let severity = if failed > 0 {
        Severity::Medium
    } else {
        Severity::Info
    };

    let description = format!(
        "Tests completed: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    let notification = AgentNotification::new(
        EventType::TestsCompleted,
        workspace_id,
        vec![],
        data,
        severity,
    )
    .with_description(description)
    .with_tags(vec!["tests".to_string()]);

    notification_service.notify(notification);
}

/// Send dependency updated notification
pub fn send_dependency_updated(
    notification_service: &NotificationService,
    workspace_id: Uuid,
    dependency_name: &str,
    old_version: Option<&str>,
    new_version: &str,
    breaking_changes: bool,
) {
    let data = serde_json::json!({
        "dependency_name": dependency_name,
        "old_version": old_version,
        "new_version": new_version,
        "breaking_changes": breaking_changes,
    });

    let severity = if breaking_changes {
        Severity::High
    } else {
        Severity::Info
    };

    let description = if let Some(old) = old_version {
        format!(
            "Dependency {} updated from {} to {}",
            dependency_name, old, new_version
        )
    } else {
        format!("Dependency {} added at version {}", dependency_name, new_version)
    };

    let notification = AgentNotification::new(
        EventType::DependencyUpdated,
        workspace_id,
        vec![],
        data,
        severity,
    )
    .with_description(description)
    .with_tags(vec!["dependencies".to_string(), dependency_name.to_string()]);

    notification_service.notify(notification);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_watcher_callback() {
        let service = Arc::new(NotificationService::new(10));
        let mut receiver = service.subscribe("test_agent");

        let callback = create_file_watcher_callback(Arc::clone(&service));

        let workspace_id = Uuid::new_v4();
        callback(
            workspace_id,
            vec!["test.rs".to_string()],
            serde_json::json!({"event": "modified"}),
        );

        let notification = receiver.recv().await.unwrap();
        assert_eq!(notification.event_type, EventType::CodeChanged);
        assert_eq!(notification.workspace_id, workspace_id);
    }

    #[tokio::test]
    async fn test_auto_reparse_callback() {
        let service = Arc::new(NotificationService::new(10));
        let mut receiver = service.subscribe("test_agent");

        let callback = create_auto_reparse_callback(Arc::clone(&service));

        let workspace_id = Uuid::new_v4();
        callback(
            workspace_id,
            "test.rs",
            serde_json::json!({
                "status": "success",
                "units_stored": 5,
                "duration_ms": 123
            }),
        );

        let notification = receiver.recv().await.unwrap();
        assert_eq!(notification.event_type, EventType::ParseCompleted);
        assert_eq!(notification.severity, Severity::Info);
    }

    #[test]
    fn test_security_alert() {
        let service = NotificationService::new(10);

        send_security_alert(
            &service,
            Uuid::new_v4(),
            vec!["vulnerable.rs".to_string()],
            "SQL-001",
            Severity::High,
            Some("CWE-89"),
            "SQL injection vulnerability detected".to_string(),
        );

        let stats = service.get_stats();
        assert!(stats.get("notifications_sent").unwrap_or(&0) > &0);
    }

    #[test]
    fn test_quality_issue() {
        let service = NotificationService::new(10);

        send_quality_issue(
            &service,
            Uuid::new_v4(),
            vec!["complex.rs".to_string()],
            "high_complexity",
            Severity::Medium,
            serde_json::json!({
                "cyclomatic_complexity": 25,
                "cognitive_complexity": 30
            }),
        );

        let stats = service.get_stats();
        assert!(stats.get("notifications_sent").unwrap_or(&0) > &0);
    }
}
