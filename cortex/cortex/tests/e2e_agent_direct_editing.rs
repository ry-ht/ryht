//! Comprehensive E2E Tests for Agent Direct Code Editing Workflow
//!
//! This test suite validates the complete workflow when AI agents directly edit
//! code files in the filesystem, including:
//!
//! 1. FileWatcher detecting changes
//! 2. Auto-reparse system automatically re-parsing files
//! 3. Notification system delivering events to agents
//! 4. CodeUnit cache being properly updated
//! 5. Metrics being collected throughout the workflow
//!
//! Test Scenarios:
//! 1. Full agent workflow with direct code editing
//! 2. Notification delivery to multiple agents
//! 3. Concurrent edits by multiple agents
//! 4. Performance metrics collection

use anyhow::Result;
use cortex::services::{
    create_auto_reparse_callback, create_file_watcher_callback, CacheStats, CodeUnitService,
    NotificationService, VfsService, WorkspaceService,
};
use cortex::services::workspace::CreateWorkspaceRequest;
use cortex_code_analysis::CodeParser;
use cortex_core::id::CortexId;
use cortex_core::types::{CodeUnit, Language};
use std::str::FromStr;
use cortex_memory::SemanticMemorySystem;
use cortex_storage::connection_pool::{
    ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::ingestion::FileIngestionPipeline;
use cortex_vfs::auto_reparse::AutoReparseHandle;
use cortex_vfs::{AutoReparseConfig, FileWatcher, VirtualFileSystem, VirtualPath, WatcherConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;
use tokio::task::JoinSet;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Complete test environment for agent direct editing workflow
struct TestEnvironment {
    workspace_id: Uuid,
    temp_dir: TempDir,
    #[allow(dead_code)]
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    #[allow(dead_code)]
    workspace_service: WorkspaceService,
    #[allow(dead_code)]
    vfs_service: VfsService,
    code_unit_service: CodeUnitService,
    notification_service: Arc<NotificationService>,
    auto_reparse: Arc<AutoReparseHandle>,
    watcher: Option<FileWatcher>,
}

impl TestEnvironment {
    /// Create a complete test environment with all components integrated
    async fn new() -> Result<Self> {
        // Initialize tracing for debugging
        let _ = FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .try_init();

        // 1. Setup storage
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
            namespace: "e2e_agent_test".to_string(),
            database: "test".to_string(),
        };

        let storage = Arc::new(ConnectionManager::new(config).await?);
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

        // 2. Setup services
        let workspace_service = WorkspaceService::new(storage.clone(), vfs.clone());
        let vfs_service = VfsService::new(vfs.clone());
        let code_unit_service = CodeUnitService::new(storage.clone());

        // 3. Create workspace
        let workspace_details = workspace_service
            .create_workspace(CreateWorkspaceRequest {
                name: "e2e_agent_editing".to_string(),
                source_path: None,
                sync_sources: None,
                read_only: None,
                metadata: None,
            })
            .await?;
        let workspace_id = Uuid::parse_str(&workspace_details.id)?;

        // 4. Setup notification system
        let notification_service = Arc::new(NotificationService::new(100));

        // 5. Setup auto-reparse with notifications
        let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new()?));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
        let ingestion_pipeline = Arc::new(FileIngestionPipeline::new(
            parser,
            vfs.clone(),
            semantic_memory,
        ));

        let auto_reparse_config = AutoReparseConfig {
            enabled: true,
            debounce_ms: 100, // Short for tests
            max_pending_changes: 10,
            background_parsing: true,
        };

        let reparse_callback = create_auto_reparse_callback(Arc::clone(&notification_service));
        let auto_reparse = Arc::new(AutoReparseHandle::with_notifications(
            auto_reparse_config,
            Some(ingestion_pipeline),
            reparse_callback,
        ));

        // 6. Create temp directory for physical files
        let temp_dir = TempDir::new()?;

        info!(
            "Test environment created: workspace={}, temp_dir={:?}",
            workspace_id,
            temp_dir.path()
        );

        Ok(Self {
            workspace_id,
            temp_dir,
            storage,
            vfs,
            workspace_service,
            vfs_service,
            code_unit_service,
            notification_service,
            auto_reparse,
            watcher: None,
        })
    }

    /// Start file watcher with full integration
    async fn start_watcher(&mut self) -> Result<()> {
        let mut watcher_config = WatcherConfig::default();
        watcher_config.enable_auto_sync = true;
        watcher_config.enable_auto_reparse = true;
        watcher_config.debounce_duration = Duration::from_millis(50);
        watcher_config.batch_interval = Duration::from_millis(100);

        let file_watcher_callback = create_file_watcher_callback(Arc::clone(&self.notification_service));

        let mut watcher = FileWatcher::with_integration(
            self.temp_dir.path(),
            self.workspace_id,
            watcher_config,
            self.vfs.clone(),
            Some(self.auto_reparse.clone()),
        )?;

        watcher.set_notification_callback(file_watcher_callback);

        self.watcher = Some(watcher);
        info!("FileWatcher started for {:?}", self.temp_dir.path());
        Ok(())
    }

    /// Process watcher events and wait for them to complete
    async fn process_events(&mut self) -> Result<usize> {
        if let Some(watcher) = self.watcher.as_mut() {
            tokio::time::sleep(Duration::from_millis(300)).await;

            let mut total_events = 0;
            while let Ok(Some(events)) = tokio::time::timeout(
                Duration::from_millis(200),
                watcher.process_events()
            ).await {
                total_events += events.len();
                if events.is_empty() {
                    break;
                }
            }

            // Extra time for auto-reparse to complete
            tokio::time::sleep(Duration::from_millis(300)).await;

            Ok(total_events)
        } else {
            Ok(0)
        }
    }

    /// Create a Rust file directly in filesystem (simulating agent edit)
    async fn agent_write_file(&self, relative_path: &str, content: &str) -> Result<()> {
        let file_path = self.temp_dir.path().join(relative_path);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&file_path, content).await?;
        info!("Agent wrote file: {:?}", file_path);
        Ok(())
    }

    /// Get code units for a file
    async fn get_code_units(&self, relative_path: &str) -> Result<Vec<CodeUnit>> {
        let file_path = format!("{}/{}", self.workspace_id, relative_path);
        let details = self.code_unit_service.get_units_by_file(&file_path).await?;

        // Convert details to CodeUnit (simplified for testing)
        let units = details.into_iter().map(|d| {
            CodeUnit {
                id: CortexId::from_str(&d.id).unwrap_or_else(|_| CortexId::new()),
                unit_type: cortex_core::types::CodeUnitType::Function,
                name: d.name,
                qualified_name: d.qualified_name,
                display_name: d.display_name,
                file_path: d.file_path,
                language: Language::Rust,
                start_line: d.start_line,
                end_line: d.end_line,
                start_column: d.start_column,
                end_column: d.end_column,
                start_byte: 0,
                end_byte: 0,
                signature: d.signature,
                body: d.body,
                docstring: d.docstring,
                comments: vec![],
                return_type: None,
                parameters: vec![],
                type_parameters: vec![],
                generic_constraints: vec![],
                throws: vec![],
                visibility: cortex_core::types::Visibility::Public,
                attributes: vec![],
                modifiers: vec![],
                is_async: d.is_async,
                is_unsafe: false,
                is_const: false,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_final: false,
                is_exported: d.is_exported,
                is_default_export: false,
                complexity: cortex_core::types::Complexity {
                    cyclomatic: d.complexity.cyclomatic,
                    cognitive: d.complexity.cognitive,
                    nesting: d.complexity.nesting,
                    lines: d.complexity.lines,
                    parameters: 0,
                    returns: 0,
                },
                test_coverage: None,
                has_tests: d.has_tests,
                has_documentation: d.has_documentation,
                language_specific: HashMap::new(),
                embedding: None,
                embedding_model: None,
                summary: None,
                purpose: None,
                ast_node_type: None,
                ast_metadata: None,
                status: cortex_core::types::CodeUnitStatus::Active,
                version: d.version,
                created_at: d.created_at,
                updated_at: d.updated_at,
                created_by: "system".to_string(),
                updated_by: "system".to_string(),
                tags: vec![],
                metadata: HashMap::new(),
            }
        }).collect();

        Ok(units)
    }

    /// Check if file exists in VFS
    async fn file_exists_in_vfs(&self, relative_path: &str) -> Result<bool> {
        let virtual_path = VirtualPath::new(relative_path)?;
        self.vfs.exists(&self.workspace_id, &virtual_path).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get watcher stats
    fn get_watcher_stats(&self) -> HashMap<String, u64> {
        if let Some(watcher) = &self.watcher {
            watcher.get_stats()
        } else {
            HashMap::new()
        }
    }

    /// Get notification stats
    fn get_notification_stats(&self) -> HashMap<String, u64> {
        self.notification_service.get_stats()
    }

    /// Get code unit cache stats
    fn get_cache_stats(&self) -> CacheStats {
        self.code_unit_service.cache_stats()
    }
}

// ============================================================================
// TEST 1: Complete Agent Direct Edit Workflow
// ============================================================================

#[tokio::test]
async fn test_agent_direct_edit_workflow() -> Result<()> {
    println!("\n=== TEST 1: Complete Agent Direct Edit Workflow ===\n");

    let mut env = TestEnvironment::new().await?;

    // Subscribe an agent to notifications
    let mut agent_receiver = env.notification_service.subscribe("test_agent");
    println!("✓ Agent subscribed to notifications");

    // Start file watcher
    env.start_watcher().await?;
    println!("✓ FileWatcher started");

    // Step 1: Agent creates a new Rust file
    println!("\nStep 1: Agent creates new file...");
    let file_content = r#"
//! Test module

/// A test function
pub fn hello_world() -> String {
    "Hello, World!".to_string()
}

/// Another function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    env.agent_write_file("src/lib.rs", file_content).await?;
    println!("✓ Agent wrote src/lib.rs");

    // Step 2: Process watcher events
    println!("\nStep 2: FileWatcher detecting change...");
    let events = env.process_events().await?;
    println!("✓ Processed {} events", events);

    let watcher_stats = env.get_watcher_stats();
    println!("  - Files synced: {}", watcher_stats.get("files_synced").unwrap_or(&0));
    println!("  - Files reparsed: {}", watcher_stats.get("files_reparsed").unwrap_or(&0));

    // Step 3: Verify file was synced to VFS
    println!("\nStep 3: Verifying VFS sync...");
    let exists = env.file_exists_in_vfs("src/lib.rs").await?;
    assert!(exists, "File should exist in VFS");
    println!("✓ File exists in VFS");

    // Step 4: Verify auto-reparse created CodeUnits
    println!("\nStep 4: Verifying auto-reparse...");
    tokio::time::sleep(Duration::from_millis(500)).await; // Wait for parsing

    let units = env.get_code_units("src/lib.rs").await?;
    assert!(units.len() >= 2, "Expected at least 2 functions, got {}", units.len());
    println!("✓ Auto-reparse created {} code units:", units.len());
    for unit in &units {
        println!("  - {} ({})", unit.name, unit.qualified_name);
    }

    // Step 5: Verify agent received notifications
    println!("\nStep 5: Verifying agent notifications...");
    let mut notifications_received = Vec::new();

    // Try to receive notifications with timeout
    for _ in 0..5 {
        match tokio::time::timeout(Duration::from_millis(100), agent_receiver.recv()).await {
            Ok(Ok(notification)) => {
                notifications_received.push(notification);
            }
            _ => break,
        }
    }

    assert!(!notifications_received.is_empty(), "Agent should receive notifications");
    println!("✓ Agent received {} notifications:", notifications_received.len());

    for notif in &notifications_received {
        println!("  - {:?}: {:?}", notif.event_type, notif.description);
    }

    // Step 6: Verify cache is populated
    println!("\nStep 6: Verifying CodeUnit cache...");
    let cache_stats = env.get_cache_stats();
    println!("✓ Cache stats:");
    println!("  - Hits: {}", cache_stats.hits);
    println!("  - Misses: {}", cache_stats.misses);
    println!("  - Hit rate: {:.1}%", cache_stats.hit_rate);

    // Step 7: Agent modifies the file
    println!("\nStep 7: Agent modifies file...");
    let modified_content = r#"
//! Test module (modified)

/// A test function (modified)
pub fn hello_world() -> String {
    "Hello, Modified World!".to_string()
}

/// Another function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// New function
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;

    env.agent_write_file("src/lib.rs", modified_content).await?;
    println!("✓ Agent modified src/lib.rs");

    // Step 8: Process events again
    println!("\nStep 8: Processing modification...");
    let events = env.process_events().await?;
    println!("✓ Processed {} events", events);

    // Step 9: Verify updated CodeUnits
    println!("\nStep 9: Verifying updated CodeUnits...");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let updated_units = env.get_code_units("src/lib.rs").await?;
    assert!(updated_units.len() >= 3, "Expected at least 3 functions, got {}", updated_units.len());

    let has_multiply = updated_units.iter().any(|u| u.name == "multiply");
    assert!(has_multiply, "New function 'multiply' should exist");
    println!("✓ Updated CodeUnits verified:");
    for unit in &updated_units {
        println!("  - {}", unit.name);
    }

    // Step 10: Verify metrics
    println!("\nStep 10: Final metrics...");
    let final_watcher_stats = env.get_watcher_stats();
    let final_notification_stats = env.get_notification_stats();
    let final_cache_stats = env.get_cache_stats();

    println!("✓ Watcher stats:");
    println!("  - Events processed: {}", final_watcher_stats.get("events_processed").unwrap_or(&0));
    println!("  - Files synced: {}", final_watcher_stats.get("files_synced").unwrap_or(&0));
    println!("  - Files reparsed: {}", final_watcher_stats.get("files_reparsed").unwrap_or(&0));

    println!("✓ Notification stats:");
    println!("  - Sent: {}", final_notification_stats.get("notifications_sent").unwrap_or(&0));
    println!("  - Active subscriptions: {}", env.notification_service.subscription_count());

    println!("✓ Cache stats:");
    println!("  - Requests: {}", final_cache_stats.total_requests);
    println!("  - Invalidations: {}", final_cache_stats.invalidations);

    println!("\n✅ TEST 1 PASSED: Complete agent workflow validated\n");
    Ok(())
}

// ============================================================================
// TEST 2: Agent Notification Delivery
// ============================================================================

#[tokio::test]
async fn test_agent_notification_delivery() -> Result<()> {
    println!("\n=== TEST 2: Agent Notification Delivery ===\n");

    let mut env = TestEnvironment::new().await?;

    // Subscribe multiple agents
    let mut agent1_receiver = env.notification_service.subscribe("agent_1");
    let mut agent2_receiver = env.notification_service.subscribe("agent_2");
    let mut agent3_receiver = env.notification_service.subscribe("agent_3");
    println!("✓ 3 agents subscribed");

    env.start_watcher().await?;
    println!("✓ FileWatcher started");

    // Agent 1 writes a file
    println!("\nStep 1: Agent writes file...");
    env.agent_write_file("test.rs", "pub fn test() {}").await?;

    env.process_events().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // All agents should receive notifications
    println!("\nStep 2: Checking notification delivery...");

    let mut agent1_count = 0;
    while agent1_receiver.try_recv().is_ok() {
        agent1_count += 1;
    }

    let mut agent2_count = 0;
    while agent2_receiver.try_recv().is_ok() {
        agent2_count += 1;
    }

    let mut agent3_count = 0;
    while agent3_receiver.try_recv().is_ok() {
        agent3_count += 1;
    }

    println!("✓ Notification delivery:");
    println!("  - Agent 1: {} notifications", agent1_count);
    println!("  - Agent 2: {} notifications", agent2_count);
    println!("  - Agent 3: {} notifications", agent3_count);

    assert!(agent1_count > 0, "Agent 1 should receive notifications");
    assert!(agent2_count > 0, "Agent 2 should receive notifications");
    assert!(agent3_count > 0, "Agent 3 should receive notifications");

    // Check notification history
    println!("\nStep 3: Checking notification history...");
    let history = env.notification_service.get_history(Some(10)).await;
    println!("✓ Notification history: {} entries", history.len());

    for (i, notif) in history.iter().enumerate() {
        println!("  {}. {:?} - {:?}", i + 1, notif.event_type, notif.file_paths);
    }

    println!("\n✅ TEST 2 PASSED: Notification delivery verified\n");
    Ok(())
}

// ============================================================================
// TEST 3: Concurrent Agent Edits
// ============================================================================

#[tokio::test]
async fn test_concurrent_agent_edits() -> Result<()> {
    println!("\n=== TEST 3: Concurrent Agent Edits ===\n");

    let mut env = TestEnvironment::new().await?;
    let mut agent_receiver = env.notification_service.subscribe("monitoring_agent");

    env.start_watcher().await?;
    println!("✓ FileWatcher started");

    // Simulate 5 agents editing different files concurrently
    println!("\nStep 1: Simulating 5 concurrent agents...");
    let start = Instant::now();

    let mut tasks = JoinSet::new();
    for i in 0..5 {
        let temp_dir = env.temp_dir.path().to_path_buf();
        tasks.spawn(async move {
            let file_name = format!("agent_{}.rs", i);
            let content = format!(
                "pub fn agent_{}_function() -> i32 {{ {} }}",
                i, i * 100
            );
            let file_path = temp_dir.join(&file_name);
            fs::write(&file_path, content).await?;
            Ok::<_, anyhow::Error>(file_name)
        });
    }

    let mut files_created = Vec::new();
    while let Some(result) = tasks.join_next().await {
        files_created.push(result??);
    }

    let create_time = start.elapsed();
    println!("✓ Created {} files in {:.2}s", files_created.len(), create_time.as_secs_f64());

    // Process all events
    println!("\nStep 2: Processing all events...");
    env.process_events().await?;
    tokio::time::sleep(Duration::from_millis(800)).await; // Wait for parsing

    let watcher_stats = env.get_watcher_stats();
    println!("✓ Watcher processed:");
    println!("  - Files synced: {}", watcher_stats.get("files_synced").unwrap_or(&0));
    println!("  - Files reparsed: {}", watcher_stats.get("files_reparsed").unwrap_or(&0));

    // Verify all files were parsed
    println!("\nStep 3: Verifying all files parsed...");
    for file in &files_created {
        let units = env.get_code_units(file).await?;
        assert!(!units.is_empty(), "File {} should have code units", file);
        println!("  - {}: {} units", file, units.len());
    }

    // Count notifications received
    println!("\nStep 4: Counting notifications...");
    let mut notification_count = 0;
    while agent_receiver.try_recv().is_ok() {
        notification_count += 1;
    }
    println!("✓ Monitoring agent received {} notifications", notification_count);
    assert!(notification_count >= 5, "Should receive notifications for all files");

    println!("\n✅ TEST 3 PASSED: Concurrent edits handled correctly\n");
    Ok(())
}

// ============================================================================
// TEST 4: Watcher Performance Metrics
// ============================================================================

#[tokio::test]
async fn test_watcher_performance_metrics() -> Result<()> {
    println!("\n=== TEST 4: Watcher Performance Metrics ===\n");

    let mut env = TestEnvironment::new().await?;
    env.start_watcher().await?;

    println!("Step 1: Creating 10 test files...");
    let start = Instant::now();

    for i in 0..10 {
        let content = format!(
            "pub fn func_{}() -> i32 {{ {} }}\npub fn helper_{}() {{ println!(\"test\"); }}",
            i, i, i
        );
        env.agent_write_file(&format!("perf_{}.rs", i), &content).await?;
    }

    let create_time = start.elapsed();
    println!("✓ Created 10 files in {:.2}s", create_time.as_secs_f64());

    // Process and measure
    println!("\nStep 2: Processing events and measuring performance...");
    let process_start = Instant::now();
    env.process_events().await?;
    tokio::time::sleep(Duration::from_millis(1000)).await; // Wait for all parsing
    let process_time = process_start.elapsed();

    println!("✓ Processing completed in {:.2}s", process_time.as_secs_f64());

    // Collect all metrics
    println!("\nStep 3: Collecting metrics...");

    let watcher_stats = env.get_watcher_stats();
    println!("✓ FileWatcher metrics:");
    for (key, value) in &watcher_stats {
        println!("  - {}: {}", key, value);
    }

    let notification_stats = env.get_notification_stats();
    println!("\n✓ Notification system metrics:");
    for (key, value) in &notification_stats {
        println!("  - {}: {}", key, value);
    }

    let cache_stats = env.get_cache_stats();
    println!("\n✓ CodeUnit cache metrics:");
    println!("  - Total requests: {}", cache_stats.total_requests);
    println!("  - Hits: {}", cache_stats.hits);
    println!("  - Misses: {}", cache_stats.misses);
    println!("  - Hit rate: {:.1}%", cache_stats.hit_rate);
    println!("  - Invalidations: {}", cache_stats.invalidations);

    // Verify metrics are being collected
    println!("\nStep 4: Verifying metrics collection...");
    assert!(watcher_stats.get("files_synced").unwrap_or(&0) >= &10,
            "Should have synced at least 10 files");
    assert!(notification_stats.get("notifications_sent").unwrap_or(&0) > &0,
            "Should have sent notifications");

    println!("✓ All metrics verified");

    // Performance assertions
    println!("\nStep 5: Performance assertions...");
    assert!(process_time.as_secs_f64() < 5.0,
            "Processing 10 files should take less than 5 seconds");
    println!("✓ Performance within acceptable bounds");

    println!("\n✅ TEST 4 PASSED: Performance metrics collected and validated\n");
    Ok(())
}

// ============================================================================
// Integration Test Suite
// ============================================================================
//
// Each test runs independently with the #[tokio::test] attribute.
// To run all tests: cargo test --test e2e_agent_direct_editing
//
// Individual tests:
// - test_agent_direct_edit_workflow: Complete workflow validation
// - test_agent_notification_delivery: Notification system validation
// - test_concurrent_agent_edits: Concurrent editing validation
// - test_watcher_performance_metrics: Performance metrics validation
// ============================================================================
