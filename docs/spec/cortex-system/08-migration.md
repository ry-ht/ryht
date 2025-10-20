# Cortex: Migration Plan from v2

## Executive Summary

This document outlines the migration strategy from legacy system (current implementation with 103 tools) to Cortex (revolutionary memory-first architecture with 150+ tools). The migration is designed to be incremental, allowing continuous operation during the transition.

## Migration Overview

### Phased Approach

```
Phase 1: Foundation (Weeks 1-2)
├─ Set up v3 infrastructure alongside v2
├─ Implement data model and storage layer
└─ Create compatibility layer

Phase 2: Core Migration (Weeks 3-4)
├─ Migrate VFS and basic operations
├─ Port essential MCP tools
└─ Implement session management

Phase 3: Feature Parity (Weeks 5-6)
├─ Complete tool migration
├─ Implement semantic graph
└─ Add multi-agent coordination

Phase 4: Cutover (Week 7)
├─ Switch primary operations to v3
├─ Maintain v2 compatibility mode
└─ Monitor and optimize

Phase 5: Deprecation (Week 8+)
├─ Remove v2 compatibility layer
├─ Archive v2 codebase
└─ Complete documentation
```

## Pre-Migration Assessment

### Current State Analysis

```rust
struct V2Assessment {
    total_tools: usize,              // 103
    storage_size_gb: f64,            // Current database size
    active_workspaces: Vec<String>,  // Active projects
    episodes_count: usize,           // Historical episodes
    code_units_count: usize,         // Indexed code units
    custom_tools: Vec<String>,       // User-defined tools
}

async fn assess_v2_system() -> Result<V2Assessment> {
    let assessment = V2Assessment {
        total_tools: count_registered_tools().await?,
        storage_size_gb: get_database_size().await?,
        active_workspaces: list_active_workspaces().await?,
        episodes_count: count_episodes().await?,
        code_units_count: count_code_units().await?,
        custom_tools: find_custom_tools().await?,
    };

    // Generate report
    generate_assessment_report(&assessment)?;

    Ok(assessment)
}
```

### Compatibility Matrix

| Component | v2 Status | v3 Status | Migration Complexity |
|-----------|-----------|-----------|---------------------|
| MCP Protocol | JSON-RPC | JSON-RPC | None |
| Storage | SQLite + Files | SurrealDB | High |
| Memory System | 5-tier | Enhanced 5-tier | Medium |
| VFS | None | Full | New Feature |
| Sessions | Basic | Advanced | High |
| Tool Count | 103 | 150+ | Medium |

## Phase 1: Foundation

### 1.1 Infrastructure Setup

```bash
#!/bin/bash
# setup_v3_infrastructure.sh

# Create v3 directory structure
mkdir -p ~/.cortex/{data,config,logs,backup}

# Install SurrealDB
curl -sSf https://install.surrealdb.com | sh

# Clone v3 repository
git clone https://github.com/cortex.git
cd v3

# Build v3 server
cargo build --release

# Verify installation
./target/release/cortex --version
```

### 1.2 Compatibility Layer

```rust
// compatibility/src/v2_adapter.rs

/// Adapter to route v2 tool calls to v3 implementation
pub struct V2Adapter {
    v2_registry: Arc<V2ToolRegistry>,
    v3_client: Arc<V3Client>,
}

impl V2Adapter {
    pub async fn handle_v2_tool(&self, name: &str, params: Value) -> Result<Value> {
        // Map v2 tool name to v3
        let v3_name = self.map_tool_name(name)?;

        // Transform parameters
        let v3_params = self.transform_params(name, params)?;

        // Call v3 tool
        let result = self.v3_client.call_tool(v3_name, v3_params).await?;

        // Transform result back to v2 format
        self.transform_result(name, result)
    }

    fn map_tool_name(&self, v2_name: &str) -> Result<String> {
        Ok(match v2_name {
            "memory.find_similar_episodes" => "cortex.memory.find_similar_episodes",
            "code.search_symbols" => "cortex.code.get_symbols",
            "task.create_task" => "cortex.task.create",
            // ... mappings for all 103 tools
            _ => return Err(Error::UnknownV2Tool(v2_name.to_string())),
        })
    }

    fn transform_params(&self, tool: &str, v2_params: Value) -> Result<Value> {
        // Handle parameter differences between v2 and v3
        match tool {
            "code.search_symbols" => {
                // v2: { query, type, detail_level }
                // v3: { query, symbol_types, include_private }
                json!({
                    "query": v2_params["query"],
                    "symbol_types": v2_params["type"],
                    "include_private": v2_params["detail_level"] == "full"
                })
            },
            // ... transformations for each tool
            _ => Ok(v2_params),
        }
    }
}
```

### 1.3 Dual Operation Mode

```rust
pub struct DualModeServer {
    v2_server: Arc<V2Server>,
    v3_server: Arc<V3Server>,
    mode: Arc<RwLock<OperationMode>>,
}

enum OperationMode {
    V2Only,           // Current state
    V2Primary,        // v2 primary, v3 shadow
    Dual,            // Both active
    V3Primary,       // v3 primary, v2 fallback
    V3Only,          // Final state
}

impl DualModeServer {
    pub async fn handle_request(&self, request: Request) -> Response {
        let mode = self.mode.read().await.clone();

        match mode {
            OperationMode::V2Only => {
                self.v2_server.handle(request).await
            },
            OperationMode::V2Primary => {
                // Handle with v2, shadow with v3
                let v2_response = self.v2_server.handle(request.clone()).await;

                tokio::spawn(async move {
                    let _ = self.v3_server.handle(request).await;
                });

                v2_response
            },
            OperationMode::Dual => {
                // Route based on tool
                if self.is_v3_ready(&request) {
                    self.v3_server.handle(request).await
                } else {
                    self.v2_server.handle(request).await
                }
            },
            OperationMode::V3Primary => {
                // Try v3 first, fallback to v2
                match self.v3_server.handle(request.clone()).await {
                    Ok(response) => response,
                    Err(_) => self.v2_server.handle(request).await,
                }
            },
            OperationMode::V3Only => {
                self.v3_server.handle(request).await
            }
        }
    }
}
```

## Phase 2: Core Migration

### 2.1 Data Migration

```rust
pub struct DataMigrator {
    v2_db: Arc<V2Database>,
    v3_storage: Arc<SurrealStorage>,
    progress: Arc<RwLock<MigrationProgress>>,
}

impl DataMigrator {
    pub async fn migrate_all(&self) -> Result<MigrationReport> {
        let mut report = MigrationReport::default();

        // 1. Migrate episodes
        info!("Migrating episodes...");
        report.episodes = self.migrate_episodes().await?;

        // 2. Migrate code symbols
        info!("Migrating code symbols...");
        report.symbols = self.migrate_symbols().await?;

        // 3. Migrate tasks
        info!("Migrating tasks...");
        report.tasks = self.migrate_tasks().await?;

        // 4. Migrate specifications
        info!("Migrating specifications...");
        report.specs = self.migrate_specs().await?;

        // 5. Build relationships
        info!("Building relationships...");
        report.relationships = self.build_relationships().await?;

        Ok(report)
    }

    async fn migrate_episodes(&self) -> Result<MigrationStats> {
        let episodes = self.v2_db.get_all_episodes().await?;
        let total = episodes.len();
        let mut migrated = 0;

        for v2_episode in episodes {
            // Transform to v3 format
            let v3_episode = Episode {
                id: v2_episode.id,
                episode_type: self.map_episode_type(&v2_episode),
                task_description: v2_episode.task_description,
                agent_id: "v2_import".to_string(),
                entities_modified: self.extract_entities(&v2_episode),
                solution_summary: v2_episode.solution_summary,
                outcome: self.map_outcome(&v2_episode),
                duration_seconds: v2_episode.duration_seconds,
                embedding: self.regenerate_embedding(&v2_episode).await?,
                created_at: v2_episode.created_at,
            };

            // Store in v3
            self.v3_storage.create_episode(v3_episode).await?;
            migrated += 1;

            // Update progress
            self.update_progress("episodes", migrated, total).await;
        }

        Ok(MigrationStats { total, migrated, failed: total - migrated })
    }

    async fn migrate_symbols(&self) -> Result<MigrationStats> {
        let symbols = self.v2_db.get_all_symbols().await?;
        let total = symbols.len();
        let mut migrated = 0;

        // Group symbols by file
        let mut by_file: HashMap<String, Vec<V2Symbol>> = HashMap::new();
        for symbol in symbols {
            by_file.entry(symbol.file_path.clone()).or_default().push(symbol);
        }

        // Migrate file by file
        for (file_path, file_symbols) in by_file {
            // Create vnode for file
            let vnode = self.create_vnode_from_path(&file_path).await?;

            // Migrate symbols as code units
            for v2_symbol in file_symbols {
                let v3_unit = CodeUnit {
                    id: generate_id(),
                    unit_type: self.map_symbol_type(&v2_symbol.symbol_type),
                    name: v2_symbol.name,
                    qualified_name: v2_symbol.qualified_name,
                    file_node: vnode.id.clone(),
                    start_line: v2_symbol.start_line,
                    end_line: v2_symbol.end_line,
                    signature: v2_symbol.signature,
                    body: self.extract_body(&v2_symbol).await?,
                    language: detect_language(&file_path)?,
                    embedding: v2_symbol.embedding,
                    created_at: v2_symbol.created_at,
                    ..Default::default()
                };

                self.v3_storage.create_code_unit(v3_unit).await?;
                migrated += 1;
            }

            self.update_progress("symbols", migrated, total).await;
        }

        Ok(MigrationStats { total, migrated, failed: total - migrated })
    }
}
```

### 2.2 Workspace Import

```rust
pub struct WorkspaceImporter {
    vfs: Arc<VirtualFileSystem>,
    parser: Arc<CodeParser>,
}

impl WorkspaceImporter {
    pub async fn import_v2_workspace(&self, path: &Path) -> Result<WorkspaceId> {
        // 1. Create v3 workspace
        let workspace = self.vfs.create_workspace(WorkspaceConfig {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            root_path: path.to_path_buf(),
            workspace_type: detect_workspace_type(path)?,
        }).await?;

        // 2. Walk directory tree
        let walker = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored(e));

        let mut total_files = 0;
        let mut processed_files = 0;

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if entry.file_type().is_file() {
                total_files += 1;

                // Create vnode
                let relative_path = path.strip_prefix(workspace.root_path)?;
                let vnode_path = format!("/{}", relative_path.display());

                // Read content
                let content = tokio::fs::read_to_string(path).await?;

                // Create file in VFS
                self.vfs.create_file(&vnode_path, &content).await?;

                // Parse if source file
                if let Some(language) = detect_language(path) {
                    self.parser.parse_file(&vnode_path, &content, language).await?;
                }

                processed_files += 1;

                // Report progress
                if processed_files % 100 == 0 {
                    info!("Imported {}/{} files", processed_files, total_files);
                }
            }
        }

        info!("Workspace import complete: {} files", processed_files);

        Ok(workspace.id)
    }
}
```

## Phase 3: Feature Parity

### 3.1 Tool Migration Tracker

```rust
pub struct ToolMigrationTracker {
    tools: HashMap<String, ToolMigrationStatus>,
}

pub struct ToolMigrationStatus {
    v2_name: String,
    v3_name: Option<String>,
    status: MigrationStatus,
    tested: bool,
    notes: String,
}

enum MigrationStatus {
    NotStarted,
    InProgress,
    Implemented,
    Tested,
    Deployed,
}

impl ToolMigrationTracker {
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# Tool Migration Status\n\n");

        let total = self.tools.len();
        let implemented = self.tools.values()
            .filter(|t| matches!(t.status, MigrationStatus::Implemented | MigrationStatus::Tested | MigrationStatus::Deployed))
            .count();

        report.push_str(&format!("Progress: {}/{} ({:.1}%)\n\n",
            implemented, total, (implemented as f64 / total as f64) * 100.0));

        report.push_str("| v2 Tool | v3 Tool | Status | Tested | Notes |\n");
        report.push_str("|---------|---------|--------|--------|-------|\n");

        for (_, tool) in &self.tools {
            report.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                tool.v2_name,
                tool.v3_name.as_deref().unwrap_or("-"),
                format!("{:?}", tool.status),
                if tool.tested { "✓" } else { "✗" },
                tool.notes
            ));
        }

        report
    }
}
```

### 3.2 Testing Framework

```rust
pub struct MigrationTester {
    v2_client: Arc<V2Client>,
    v3_client: Arc<V3Client>,
}

impl MigrationTester {
    pub async fn test_tool_parity(&self, tool_name: &str) -> TestResult {
        let test_cases = self.generate_test_cases(tool_name)?;
        let mut results = Vec::new();

        for test_case in test_cases {
            // Run on v2
            let v2_result = self.v2_client
                .call_tool(tool_name, test_case.params.clone())
                .await;

            // Run on v3
            let v3_tool = map_to_v3_tool(tool_name)?;
            let v3_params = transform_params(tool_name, test_case.params.clone())?;
            let v3_result = self.v3_client
                .call_tool(&v3_tool, v3_params)
                .await;

            // Compare results
            let comparison = self.compare_results(
                &v2_result,
                &v3_result,
                &test_case.expected_fields
            )?;

            results.push(TestCaseResult {
                name: test_case.name,
                passed: comparison.is_equivalent,
                differences: comparison.differences,
            });
        }

        TestResult {
            tool: tool_name.to_string(),
            total_cases: results.len(),
            passed: results.iter().filter(|r| r.passed).count(),
            results,
        }
    }

    fn compare_results(&self, v2: &Value, v3: &Value, expected: &[String]) -> ComparisonResult {
        let mut differences = Vec::new();
        let mut is_equivalent = true;

        for field in expected {
            let v2_value = v2.get(field);
            let v3_value = v3.get(field);

            if !self.values_equivalent(v2_value, v3_value) {
                is_equivalent = false;
                differences.push(format!(
                    "{}: v2={:?}, v3={:?}",
                    field, v2_value, v3_value
                ));
            }
        }

        ComparisonResult {
            is_equivalent,
            differences,
        }
    }
}
```

## Phase 4: Cutover

### 4.1 Cutover Procedure

```rust
pub struct CutoverManager {
    dual_server: Arc<DualModeServer>,
    health_checker: Arc<HealthChecker>,
    rollback_manager: Arc<RollbackManager>,
}

impl CutoverManager {
    pub async fn execute_cutover(&self) -> Result<CutoverReport> {
        info!("Starting cutover to v3...");

        // 1. Create backup
        info!("Creating backup...");
        let backup_id = self.rollback_manager.create_backup().await?;

        // 2. Health check v3
        info!("Running v3 health checks...");
        let health = self.health_checker.check_v3().await?;
        if !health.is_healthy {
            return Err(Error::V3NotHealthy(health));
        }

        // 3. Switch to v3 primary mode
        info!("Switching to v3 primary mode...");
        self.dual_server.set_mode(OperationMode::V3Primary).await?;

        // 4. Monitor for issues
        info!("Monitoring for 5 minutes...");
        let monitoring_result = self.monitor_operations(Duration::from_secs(300)).await?;

        if monitoring_result.error_rate > 0.01 {
            // Too many errors, rollback
            warn!("High error rate detected: {:.2}%", monitoring_result.error_rate * 100.0);
            info!("Initiating rollback...");
            self.rollback(backup_id).await?;
            return Err(Error::CutoverFailed("High error rate"));
        }

        // 5. Complete cutover
        info!("Cutover successful, switching to v3-only mode");
        self.dual_server.set_mode(OperationMode::V3Only).await?;

        Ok(CutoverReport {
            success: true,
            duration: monitoring_result.duration,
            requests_processed: monitoring_result.total_requests,
            error_rate: monitoring_result.error_rate,
        })
    }

    async fn rollback(&self, backup_id: BackupId) -> Result<()> {
        // Switch back to v2
        self.dual_server.set_mode(OperationMode::V2Only).await?;

        // Restore v2 state if needed
        self.rollback_manager.restore(backup_id).await?;

        Ok(())
    }
}
```

### 4.2 Monitoring & Validation

```rust
pub struct MigrationMonitor {
    metrics: Arc<MetricsCollector>,
    alert_manager: Arc<AlertManager>,
}

impl MigrationMonitor {
    pub async fn monitor_migration(&self) -> MonitoringReport {
        let mut report = MonitoringReport::default();

        // Check request patterns
        report.v2_requests = self.metrics.get_counter("v2_requests").await;
        report.v3_requests = self.metrics.get_counter("v3_requests").await;

        // Check error rates
        report.v2_errors = self.metrics.get_counter("v2_errors").await;
        report.v3_errors = self.metrics.get_counter("v3_errors").await;

        // Check performance
        report.v2_p99_latency = self.metrics.get_histogram_percentile("v2_latency", 0.99).await;
        report.v3_p99_latency = self.metrics.get_histogram_percentile("v3_latency", 0.99).await;

        // Alert if issues detected
        if report.v3_error_rate() > 0.05 {
            self.alert_manager.send_alert(Alert {
                severity: Severity::Critical,
                message: format!("High v3 error rate: {:.2}%", report.v3_error_rate() * 100.0),
            }).await;
        }

        report
    }
}
```

## Phase 5: Deprecation

### 5.1 V2 Shutdown

```rust
pub struct V2Deprecation {
    v2_server: Arc<V2Server>,
    archive_manager: Arc<ArchiveManager>,
}

impl V2Deprecation {
    pub async fn deprecate_v2(&self) -> Result<()> {
        // 1. Final backup
        info!("Creating final v2 backup...");
        let backup = self.archive_manager.create_final_backup().await?;

        // 2. Export v2 data
        info!("Exporting v2 data...");
        self.export_v2_data(&backup).await?;

        // 3. Stop v2 server
        info!("Stopping v2 server...");
        self.v2_server.shutdown().await?;

        // 4. Archive v2 code
        info!("Archiving v2 codebase...");
        self.archive_manager.archive_codebase().await?;

        // 5. Clean up v2 resources
        info!("Cleaning up v2 resources...");
        self.cleanup_v2_resources().await?;

        info!("V2 deprecation complete");
        Ok(())
    }

    async fn cleanup_v2_resources(&self) -> Result<()> {
        // Remove v2 databases
        tokio::fs::remove_dir_all("~/.cortex/v2/data").await?;

        // Remove v2 configs
        tokio::fs::remove_dir_all("~/.cortex/v2/config").await?;

        // Keep logs for audit
        info!("V2 logs preserved at ~/.cortex/v2/logs");

        Ok(())
    }
}
```

## Migration Validation

### Success Criteria

```rust
pub struct MigrationValidator {
    pub async fn validate_migration(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        // 1. Data integrity
        report.data_integrity = self.validate_data_integrity().await?;

        // 2. Tool functionality
        report.tool_functionality = self.validate_all_tools().await?;

        // 3. Performance benchmarks
        report.performance = self.run_performance_benchmarks().await?;

        // 4. Memory system
        report.memory_system = self.validate_memory_system().await?;

        // 5. Multi-agent coordination
        report.coordination = self.validate_coordination().await?;

        report
    }

    async fn validate_data_integrity(&self) -> DataIntegrityReport {
        let mut report = DataIntegrityReport::default();

        // Check episode count
        let v2_episodes = self.count_v2_episodes().await?;
        let v3_episodes = self.count_v3_episodes().await?;
        report.episodes_match = v2_episodes == v3_episodes;

        // Check code units
        let v2_units = self.count_v2_code_units().await?;
        let v3_units = self.count_v3_code_units().await?;
        report.units_match = v2_units == v3_units;

        // Sample and verify content
        report.content_verification = self.verify_sample_content().await?;

        report
    }
}
```

## Rollback Plan

### Emergency Rollback

```rust
pub struct RollbackPlan {
    pub async fn emergency_rollback(&self) -> Result<()> {
        error!("EMERGENCY ROLLBACK INITIATED");

        // 1. Stop v3 immediately
        self.stop_v3_server().await?;

        // 2. Start v2 server
        self.start_v2_server().await?;

        // 3. Restore v2 data from backup
        self.restore_v2_backup().await?;

        // 4. Notify administrators
        self.send_emergency_notification().await?;

        // 5. Create incident report
        self.create_incident_report().await?;

        error!("Emergency rollback complete - system running on v2");
        Ok(())
    }
}
```

## Post-Migration

### Documentation Updates

1. Update all tool documentation
2. Create migration guide for users
3. Update API documentation
4. Create troubleshooting guide
5. Update performance benchmarks

### Training Materials

1. Video tutorials for new features
2. Interactive demos
3. Best practices guide
4. Architecture documentation
5. Developer onboarding

## Timeline

| Week | Phase | Milestone |
|------|-------|-----------|
| 1-2 | Foundation | v3 infrastructure ready |
| 3-4 | Core Migration | Data migrated, basic tools working |
| 5-6 | Feature Parity | All tools migrated and tested |
| 7 | Cutover | v3 in production |
| 8+ | Deprecation | v2 retired |

## Risk Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Data loss | Low | Critical | Multiple backups, validation |
| Performance degradation | Medium | High | Extensive benchmarking |
| Tool incompatibility | Medium | Medium | Compatibility layer |
| Downtime | Low | High | Dual operation mode |
| User disruption | Medium | Medium | Gradual migration |

## Conclusion

This migration plan ensures a smooth transition from legacy system to v3 with:

1. **Zero downtime** through dual operation mode
2. **Data integrity** through validation and backups
3. **Gradual transition** allowing testing at each phase
4. **Rollback capability** at every stage
5. **Comprehensive validation** before final cutover

The migration preserves all existing functionality while enabling the revolutionary new features of Cortex's memory-first architecture.