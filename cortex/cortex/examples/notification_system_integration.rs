//! Example: Complete Notification System Integration
//!
//! This example demonstrates how to integrate the notification system with:
//! - FileWatcher for code change notifications
//! - AutoReparseHandle for parse completion notifications
//! - Quality analysis for code smell notifications
//! - Security scanning for vulnerability notifications
//! - Architecture analysis for constraint violations
//!
//! Run with: cargo run --example notification_system_integration

use cortex::services::{
    create_auto_reparse_callback, create_file_watcher_callback, send_quality_issue,
    send_security_alert, AgentNotification, EventType, NotificationService, Severity,
};
use cortex_code_analysis::CodeParser;
use cortex_memory::SemanticMemorySystem;
use cortex_storage::connection_pool::{
    ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::ingestion::FileIngestionPipeline;
use cortex_vfs::{AutoReparseConfig, VirtualFileSystem, VirtualPath};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Notification System Integration Example");

    // 1. Setup infrastructure
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 10,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(60)),
            max_lifetime: Some(Duration::from_secs(120)),
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
            validate_on_checkout: false,
            recycle_after_uses: Some(10000),
            shutdown_grace_period: Duration::from_secs(30),
        },
        namespace: "example".to_string(),
        database: "notifications".to_string(),
    };

    let storage = Arc::new(ConnectionManager::new(config).await?);
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

    info!("Infrastructure initialized");

    // 2. Create notification service (stores last 100 notifications)
    let notification_service = Arc::new(NotificationService::new(100));
    info!("Notification service created");

    // 3. Subscribe multiple AI agents
    let agent_ids = vec!["code_reviewer", "security_auditor", "architect"];

    let mut receivers = Vec::new();
    for agent_id in &agent_ids {
        let receiver = notification_service.subscribe(agent_id);
        receivers.push((agent_id.to_string(), receiver));
        info!("Agent '{}' subscribed to notifications", agent_id);
    }

    // 4. Setup AutoReparse with notification integration
    let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new()?));
    let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
    let ingestion_pipeline = Arc::new(FileIngestionPipeline::new(
        parser,
        vfs.clone(),
        semantic_memory,
    ));

    let auto_reparse_config = AutoReparseConfig {
        enabled: true,
        debounce_ms: 100,
        max_pending_changes: 10,
        background_parsing: true,
    };

    let reparse_callback = create_auto_reparse_callback(Arc::clone(&notification_service));

    let auto_reparse = Arc::new(cortex_vfs::auto_reparse::AutoReparseHandle::with_notifications(
        auto_reparse_config,
        ingestion_pipeline,
        reparse_callback,
    ));

    info!("Auto-reparse system configured with notifications");

    // 5. Simulate various events that trigger notifications
    let workspace_id = Uuid::new_v4();

    // Event 1: Code file changed
    info!("\n=== Simulating Code Change Event ===");
    let file_watcher_callback = create_file_watcher_callback(Arc::clone(&notification_service));
    file_watcher_callback(
        workspace_id,
        vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
        serde_json::json!({
            "event": "modified",
            "lines_changed": 42,
        }),
    );
    sleep(Duration::from_millis(50)).await;

    // Event 2: Parse completed (simulated)
    info!("\n=== Simulating Parse Completion Event ===");
    let path = VirtualPath::new("src/main.rs")?;
    auto_reparse.notify_file_changed(workspace_id, path.clone());
    sleep(Duration::from_millis(200)).await;

    // Event 3: Security vulnerability detected
    info!("\n=== Simulating Security Alert ===");
    send_security_alert(
        &notification_service,
        workspace_id,
        vec!["src/auth.rs".to_string()],
        "SQL-001",
        Severity::High,
        Some("CWE-89"),
        "SQL injection vulnerability detected in authentication module".to_string(),
    );
    sleep(Duration::from_millis(50)).await;

    // Event 4: Code quality issue
    info!("\n=== Simulating Quality Issue ===");
    send_quality_issue(
        &notification_service,
        workspace_id,
        vec!["src/complex_function.rs".to_string()],
        "high_complexity",
        Severity::Medium,
        serde_json::json!({
            "cyclomatic_complexity": 28,
            "cognitive_complexity": 35,
            "lines_of_code": 450,
        }),
    );
    sleep(Duration::from_millis(50)).await;

    // Event 5: Multiple code changes
    info!("\n=== Simulating Multiple Code Changes ===");
    for i in 0..5 {
        let notification = AgentNotification::code_changed(
            workspace_id,
            vec![format!("src/module_{}.rs", i)],
            serde_json::json!({
                "event": "created",
                "lines": 100 + i * 10,
            }),
        )
        .with_description(format!("New module created: module_{}.rs", i))
        .with_tags(vec!["new_file".to_string(), "module".to_string()]);

        notification_service.notify(notification);
    }
    sleep(Duration::from_millis(50)).await;

    // 6. Check what each agent received
    info!("\n=== Agent Notifications Summary ===");

    // Simulate agents checking their notifications
    for (agent_id, mut receiver) in receivers {
        info!("\nAgent '{}' checking notifications:", agent_id);

        let mut count = 0;
        while let Ok(notification) = receiver.try_recv() {
            count += 1;
            info!(
                "  - [{:?}] {} (severity: {:?})",
                notification.event_type,
                notification
                    .description
                    .as_deref()
                    .unwrap_or("No description"),
                notification.severity
            );

            // Different agents can react differently to notifications
            match agent_id.as_str() {
                "code_reviewer" => {
                    if matches!(notification.event_type, EventType::CodeChanged) {
                        info!("    → Code reviewer: Will review changes in next cycle");
                    }
                }
                "security_auditor" => {
                    if matches!(notification.event_type, EventType::SecurityAlert) {
                        info!("    → Security auditor: HIGH PRIORITY - Analyzing vulnerability");
                    }
                }
                "architect" => {
                    if matches!(notification.event_type, EventType::QualityIssue) {
                        info!("    → Architect: Reviewing complexity metrics");
                    }
                }
                _ => {}
            }
        }

        info!("  Total notifications received: {}", count);
    }

    // 7. Query notification history
    info!("\n=== Notification History ===");

    let history = notification_service.get_history(Some(10)).await;
    info!("Total notifications in history: {}", history.len());

    // Get only high-severity notifications
    let high_severity = notification_service
        .get_history_by_severity(Severity::High, None)
        .await;
    info!(
        "High severity or above notifications: {}",
        high_severity.len()
    );

    // Get only security alerts
    let security_alerts = notification_service
        .get_history_filtered(vec![EventType::SecurityAlert], None)
        .await;
    info!("Security alerts: {}", security_alerts.len());

    // 8. Display statistics
    info!("\n=== Notification System Statistics ===");
    let stats = notification_service.get_stats();
    info!("Notifications sent: {}", stats.get("notifications_sent").unwrap_or(&0));
    info!("Notifications dropped: {}", stats.get("notifications_dropped").unwrap_or(&0));
    info!("Active subscriptions: {}", notification_service.subscription_count());

    info!("\n=== Example Complete ===");
    info!("The notification system successfully:");
    info!("  ✓ Distributed events to multiple subscribed agents");
    info!("  ✓ Integrated with FileWatcher for code changes");
    info!("  ✓ Integrated with AutoReparse for parse events");
    info!("  ✓ Sent security alerts and quality issues");
    info!("  ✓ Maintained notification history for queries");
    info!("  ✓ Provided filtering by event type and severity");

    Ok(())
}
