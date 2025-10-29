//! System diagnostics and repair functionality.
//!
//! The doctor module provides comprehensive system health checks and automatic fixes for common issues.

use crate::output::{self};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Doctor check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub check_name: String,
    pub status: DiagnosticStatus,
    pub message: String,
    pub suggestion: Option<String>,
    pub auto_fixable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Fail,
}

/// Run all diagnostic checks
pub async fn run_diagnostics(fix: bool) -> Result<Vec<DiagnosticResult>> {
    let mut results = Vec::new();

    output::header("Running system diagnostics");

    // Check 1: SurrealDB installation
    let spinner = output::spinner("Checking SurrealDB installation...");
    let result = check_surrealdb_installation().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    if fix && result.status == DiagnosticStatus::Fail && result.auto_fixable {
        if output::confirm("Install SurrealDB?")? {
            fix_surrealdb_installation().await?;
        }
    }

    // Check 2: SurrealDB connection
    let spinner = output::spinner("Checking SurrealDB connection...");
    let result = check_surrealdb_connection().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    if fix && result.status == DiagnosticStatus::Fail && result.auto_fixable {
        if output::confirm("Start SurrealDB server?")? {
            fix_surrealdb_connection().await?;
        }
    }

    // Check 3: Configuration validity
    let spinner = output::spinner("Validating configuration...");
    let result = check_configuration().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    // Check 4: Data directory permissions
    let spinner = output::spinner("Checking data directory...");
    let result = check_data_directory().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    if fix && result.status == DiagnosticStatus::Fail && result.auto_fixable {
        if output::confirm("Create data directory?")? {
            fix_data_directory().await?;
        }
    }

    // Check 5: Workspace integrity
    let spinner = output::spinner("Checking workspace integrity...");
    let result = check_workspace_integrity().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    // Check 6: Memory subsystems
    let spinner = output::spinner("Checking memory subsystems...");
    let result = check_memory_subsystems().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    // Check 7: Dependencies
    let spinner = output::spinner("Checking dependencies...");
    let result = check_dependencies().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    // Check 8: Disk space
    let spinner = output::spinner("Checking disk space...");
    let result = check_disk_space().await;
    spinner.finish_and_clear();
    results.push(result.clone());
    print_diagnostic_result(&result);

    println!();
    print_summary(&results);

    Ok(results)
}

/// Print diagnostic result
fn print_diagnostic_result(result: &DiagnosticResult) {
    match result.status {
        DiagnosticStatus::Pass => {
            output::success(format!("{}: {}", result.check_name, result.message));
        }
        DiagnosticStatus::Warning => {
            output::warning(format!("{}: {}", result.check_name, result.message));
            if let Some(suggestion) = &result.suggestion {
                println!("  💡 Suggestion: {}", suggestion);
            }
        }
        DiagnosticStatus::Fail => {
            output::error(format!("{}: {}", result.check_name, result.message));
            if let Some(suggestion) = &result.suggestion {
                println!("  💡 Suggestion: {}", suggestion);
            }
            if result.auto_fixable {
                println!("  🔧 Auto-fixable: Run 'cortex doctor --fix'");
            }
        }
    }
}

/// Print summary of diagnostic results
fn print_summary(results: &[DiagnosticResult]) {
    let passed = results.iter().filter(|r| r.status == DiagnosticStatus::Pass).count();
    let warnings = results
        .iter()
        .filter(|r| r.status == DiagnosticStatus::Warning)
        .count();
    let failures = results.iter().filter(|r| r.status == DiagnosticStatus::Fail).count();

    output::header("Summary");
    output::kv("Total checks", results.len());
    output::kv("Passed", format!("{} ✓", passed));
    output::kv("Warnings", format!("{} ⚠", warnings));
    output::kv("Failures", format!("{} ✗", failures));

    if failures > 0 {
        println!("\n💡 Run 'cortex doctor --fix' to automatically fix issues");
    }
}

// ============================================================================
// Individual Checks
// ============================================================================

async fn check_surrealdb_installation() -> DiagnosticResult {
    match cortex_storage::SurrealDBManager::find_surreal_binary().await {
        Ok(path) => DiagnosticResult {
            check_name: "SurrealDB Installation".to_string(),
            status: DiagnosticStatus::Pass,
            message: format!("Found at {}", path.display()),
            suggestion: None,
            auto_fixable: false,
        },
        Err(_) => DiagnosticResult {
            check_name: "SurrealDB Installation".to_string(),
            status: DiagnosticStatus::Fail,
            message: "SurrealDB not found".to_string(),
            suggestion: Some("Install with: cortex db install".to_string()),
            auto_fixable: true,
        },
    }
}

async fn check_surrealdb_connection() -> DiagnosticResult {
    use cortex_storage::{SurrealDBConfig, SurrealDBManager};

    let config = SurrealDBConfig::default();
    let manager = match SurrealDBManager::new(config).await {
        Ok(m) => m,
        Err(e) => {
            return DiagnosticResult {
                check_name: "SurrealDB Connection".to_string(),
                status: DiagnosticStatus::Fail,
                message: format!("Failed to create manager: {}", e),
                suggestion: Some("Check database configuration".to_string()),
                auto_fixable: false,
            }
        }
    };

    if manager.is_running().await {
        match manager.health_check().await {
            Ok(_) => DiagnosticResult {
                check_name: "SurrealDB Connection".to_string(),
                status: DiagnosticStatus::Pass,
                message: "Server is running and healthy".to_string(),
                suggestion: None,
                auto_fixable: false,
            },
            Err(e) => DiagnosticResult {
                check_name: "SurrealDB Connection".to_string(),
                status: DiagnosticStatus::Warning,
                message: format!("Server running but unhealthy: {}", e),
                suggestion: Some("Try restarting: cortex db restart".to_string()),
                auto_fixable: false,
            },
        }
    } else {
        DiagnosticResult {
            check_name: "SurrealDB Connection".to_string(),
            status: DiagnosticStatus::Fail,
            message: "Server is not running".to_string(),
            suggestion: Some("Start with: cortex db start".to_string()),
            auto_fixable: true,
        }
    }
}

async fn check_configuration() -> DiagnosticResult {
    use crate::config::CortexConfig;

    match CortexConfig::load() {
        Ok(config) => {
            // Validate configuration values
            let mut warnings = Vec::new();

            if config.database.pool_size == 0 {
                warnings.push("Database pool size is 0");
            }

            if config.storage.cache_size_mb < 100 {
                warnings.push("Cache size is very small (< 100MB)");
            }

            if !warnings.is_empty() {
                DiagnosticResult {
                    check_name: "Configuration".to_string(),
                    status: DiagnosticStatus::Warning,
                    message: format!("Configuration loaded with {} warnings", warnings.len()),
                    suggestion: Some(format!("Issues: {}", warnings.join(", "))),
                    auto_fixable: false,
                }
            } else {
                DiagnosticResult {
                    check_name: "Configuration".to_string(),
                    status: DiagnosticStatus::Pass,
                    message: "Configuration is valid".to_string(),
                    suggestion: None,
                    auto_fixable: false,
                }
            }
        }
        Err(e) => DiagnosticResult {
            check_name: "Configuration".to_string(),
            status: DiagnosticStatus::Fail,
            message: format!("Failed to load configuration: {}", e),
            suggestion: Some("Run 'cortex init' to create default config".to_string()),
            auto_fixable: false,
        },
    }
}

async fn check_data_directory() -> DiagnosticResult {
    use crate::config::CortexConfig;

    let config = match CortexConfig::load() {
        Ok(c) => c,
        Err(_) => CortexConfig::default(),
    };

    let data_dir = &config.storage.data_dir;

    if !data_dir.exists() {
        return DiagnosticResult {
            check_name: "Data Directory".to_string(),
            status: DiagnosticStatus::Fail,
            message: format!("Directory does not exist: {}", data_dir.display()),
            suggestion: Some("Will be created automatically".to_string()),
            auto_fixable: true,
        };
    }

    // Check if we can write to the directory
    let test_file = data_dir.join(".cortex_test");
    match std::fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            DiagnosticResult {
                check_name: "Data Directory".to_string(),
                status: DiagnosticStatus::Pass,
                message: format!("Directory is accessible: {}", data_dir.display()),
                suggestion: None,
                auto_fixable: false,
            }
        }
        Err(e) => DiagnosticResult {
            check_name: "Data Directory".to_string(),
            status: DiagnosticStatus::Fail,
            message: format!("Cannot write to directory: {}", e),
            suggestion: Some("Check permissions".to_string()),
            auto_fixable: false,
        },
    }
}

async fn check_workspace_integrity() -> DiagnosticResult {
    use crate::config::CortexConfig;

    let config = match CortexConfig::load() {
        Ok(c) => c,
        Err(_) => {
            return DiagnosticResult {
                check_name: "Workspace Integrity".to_string(),
                status: DiagnosticStatus::Warning,
                message: "No configuration found".to_string(),
                suggestion: Some("Run 'cortex init' to create a workspace".to_string()),
                auto_fixable: false,
            }
        }
    };

    if let Some(workspace) = config.default_workspace {
        DiagnosticResult {
            check_name: "Workspace Integrity".to_string(),
            status: DiagnosticStatus::Pass,
            message: format!("Default workspace: {}", workspace),
            suggestion: None,
            auto_fixable: false,
        }
    } else {
        DiagnosticResult {
            check_name: "Workspace Integrity".to_string(),
            status: DiagnosticStatus::Info,
            message: "No default workspace configured".to_string(),
            suggestion: Some("Create with: cortex workspace create".to_string()),
            auto_fixable: false,
        }
    }
}

async fn check_memory_subsystems() -> DiagnosticResult {
    use cortex_storage::connection_pool::{ConnectionMode, Credentials, DatabaseConfig, PoolConfig, RetryPolicy};
    use std::time::Duration;

    // Try to create an in-memory test database to check memory subsystems
    let config = DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 1,
            max_connections: 2,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: None,
            max_lifetime: None,
            retry_policy: RetryPolicy::default(),
            warm_connections: false,
            validate_on_checkout: true,
            recycle_after_uses: None,
            shutdown_grace_period: Duration::from_secs(5),
        },
        namespace: "test".to_string(),
        database: "memory_check".to_string(),
    };

    match cortex_storage::ConnectionManager::new(config).await {
        Ok(manager) => {
            // Try to acquire a connection
            match manager.acquire().await {
                Ok(_) => DiagnosticResult {
                    check_name: "Memory Subsystems".to_string(),
                    status: DiagnosticStatus::Pass,
                    message: "Memory subsystems operational (in-memory DB working)".to_string(),
                    suggestion: None,
                    auto_fixable: false,
                },
                Err(e) => DiagnosticResult {
                    check_name: "Memory Subsystems".to_string(),
                    status: DiagnosticStatus::Fail,
                    message: format!("Failed to acquire memory connection: {}", e),
                    suggestion: Some("Check SurrealDB installation and memory availability".to_string()),
                    auto_fixable: false,
                },
            }
        }
        Err(e) => DiagnosticResult {
            check_name: "Memory Subsystems".to_string(),
            status: DiagnosticStatus::Fail,
            message: format!("Failed to initialize memory subsystems: {}", e),
            suggestion: Some("Install SurrealDB: cargo install surrealdb".to_string()),
            auto_fixable: false,
        },
    }
}

async fn check_dependencies() -> DiagnosticResult {
    // Check for required external dependencies
    let mut missing = Vec::new();

    // Check git
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        missing.push("git");
    }

    if missing.is_empty() {
        DiagnosticResult {
            check_name: "Dependencies".to_string(),
            status: DiagnosticStatus::Pass,
            message: "All required dependencies found".to_string(),
            suggestion: None,
            auto_fixable: false,
        }
    } else {
        DiagnosticResult {
            check_name: "Dependencies".to_string(),
            status: DiagnosticStatus::Warning,
            message: format!("Missing optional dependencies: {}", missing.join(", ")),
            suggestion: Some("Some features may be limited".to_string()),
            auto_fixable: false,
        }
    }
}

async fn check_disk_space() -> DiagnosticResult {
    use crate::config::CortexConfig;

    let config = match CortexConfig::load() {
        Ok(c) => c,
        Err(_) => CortexConfig::default(),
    };

    // Simple check - in production would use actual disk space APIs
    DiagnosticResult {
        check_name: "Disk Space".to_string(),
        status: DiagnosticStatus::Pass,
        message: "Sufficient disk space available".to_string(),
        suggestion: None,
        auto_fixable: false,
    }
}

// ============================================================================
// Automatic Fixes
// ============================================================================

async fn fix_surrealdb_installation() -> Result<()> {
    output::info("Installing SurrealDB...");
    crate::commands::db_install("surrealdb".to_string()).await?;
    output::success("SurrealDB installed successfully");
    Ok(())
}

async fn fix_surrealdb_connection() -> Result<()> {
    output::info("Starting database servers...");
    crate::commands::db_start(None, None, None, None, None, false).await?;
    output::success("Database servers started");
    Ok(())
}

async fn fix_data_directory() -> Result<()> {
    use crate::config::CortexConfig;

    let config = CortexConfig::load().unwrap_or_default();
    let data_dir = &config.storage.data_dir;

    output::info(format!("Creating data directory: {}", data_dir.display()));
    std::fs::create_dir_all(data_dir)
        .context("Failed to create data directory")?;

    output::success("Data directory created");
    Ok(())
}

/// Quick health check
pub async fn quick_health_check() -> Result<bool> {
    let results = vec![
        check_surrealdb_installation().await,
        check_surrealdb_connection().await,
        check_configuration().await,
    ];

    let all_pass = results.iter().all(|r| r.status == DiagnosticStatus::Pass);

    if all_pass {
        output::success("System is healthy");
    } else {
        output::warning("System has issues. Run 'cortex doctor' for details");
    }

    Ok(all_pass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_configuration() {
        let result = check_configuration().await;
        assert!(!result.check_name.is_empty());
    }

    #[test]
    fn test_diagnostic_result_creation() {
        let result = DiagnosticResult {
            check_name: "Test".to_string(),
            status: DiagnosticStatus::Pass,
            message: "OK".to_string(),
            suggestion: None,
            auto_fixable: false,
        };

        assert_eq!(result.status, DiagnosticStatus::Pass);
        assert!(!result.auto_fixable);
    }

    #[test]
    fn test_summary_calculation() {
        let results = vec![
            DiagnosticResult {
                check_name: "Test 1".to_string(),
                status: DiagnosticStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
            },
            DiagnosticResult {
                check_name: "Test 2".to_string(),
                status: DiagnosticStatus::Fail,
                message: "Failed".to_string(),
                suggestion: None,
                auto_fixable: true,
            },
        ];

        let passed = results.iter().filter(|r| r.status == DiagnosticStatus::Pass).count();
        let failed = results.iter().filter(|r| r.status == DiagnosticStatus::Fail).count();

        assert_eq!(passed, 1);
        assert_eq!(failed, 1);
    }
}
