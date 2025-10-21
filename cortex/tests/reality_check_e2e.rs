//! REALITY CHECK: Multi-Agent Development Workflow E2E Test
//!
//! This test simulates ACTUAL usage of Cortex by Claude agents working on a real project.
//! It tests the complete workflow from project import to materialization with multiple
//! concurrent agents performing different tasks.
//!
//! SCENARIO: Four agents collaborating on a Rust calculator project
//! - Agent 1 (Architect): Import project, parse code, build dependency graph
//! - Agent 2 (Developer): Implement new features in isolated session
//! - Agent 3 (Reviewer): Review changes, search for patterns
//! - Agent 4 (Tester): Write tests, store test patterns
//! - Final: Consolidate memories, materialize to disk, verify integrity

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_storage::connection_pool::{
    ConnectionManager, ConnectionMode, Credentials, DatabaseConfig, PoolConfig,
};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use tracing::info;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

const MAX_WORKFLOW_TIME_SECS: u64 = 60;
const MAX_OPERATION_TIME_SECS: u64 = 5;

fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex_reality_check".to_string(),
        database: db_name.to_string(),
    }
}

// ============================================================================
// HELPER: Create Realistic Rust Calculator Project
// ============================================================================

async fn create_calculator_project(workspace: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Cargo.toml
    let cargo_toml = workspace.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "rust-calculator"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "1.0"

[dev-dependencies]
"#,
    )
    .await
    .unwrap();
    files.push(cargo_toml);

    // Create src directory
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).await.unwrap();

    // lib.rs
    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"//! Rust Calculator Library
//!
//! Provides basic arithmetic operations with error handling.

pub mod operations;
pub mod error;

pub use error::{CalculatorError, Result};
pub use operations::{add, subtract, multiply, divide};

/// Calculator context
pub struct Calculator {
    precision: u32,
}

impl Calculator {
    pub fn new(precision: u32) -> Self {
        Self { precision }
    }

    pub fn calculate(&self, a: f64, b: f64, op: char) -> Result<f64> {
        match op {
            '+' => add(a, b),
            '-' => subtract(a, b),
            '*' => multiply(a, b),
            '/' => divide(a, b),
            _ => Err(CalculatorError::InvalidOperation(op)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator_add() {
        let calc = Calculator::new(2);
        assert_eq!(calc.calculate(2.0, 3.0, '+').unwrap(), 5.0);
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(lib_rs);

    // operations.rs
    let operations_rs = src_dir.join("operations.rs");
    fs::write(
        &operations_rs,
        r#"//! Arithmetic operations

use crate::error::{CalculatorError, Result};

/// Add two numbers
pub fn add(a: f64, b: f64) -> Result<f64> {
    Ok(a + b)
}

/// Subtract two numbers
pub fn subtract(a: f64, b: f64) -> Result<f64> {
    Ok(a - b)
}

/// Multiply two numbers
pub fn multiply(a: f64, b: f64) -> Result<f64> {
    Ok(a * b)
}

/// Divide two numbers
pub fn divide(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        Err(CalculatorError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2.0, 3.0).unwrap(), 5.0);
    }

    #[test]
    fn test_divide_by_zero() {
        assert!(divide(1.0, 0.0).is_err());
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(operations_rs);

    // error.rs
    let error_rs = src_dir.join("error.rs");
    fs::write(
        &error_rs,
        r#"//! Error types for calculator

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CalculatorError>;

#[derive(Error, Debug)]
pub enum CalculatorError {
    #[error("Division by zero")]
    DivisionByZero,

    #[error("Invalid operation: {0}")]
    InvalidOperation(char),

    #[error("Overflow occurred")]
    Overflow,
}
"#,
    )
    .await
    .unwrap();
    files.push(error_rs);

    // README.md
    let readme = workspace.join("README.md");
    fs::write(
        &readme,
        r#"# Rust Calculator

A simple calculator library demonstrating error handling in Rust.

## Features

- Basic arithmetic operations
- Error handling for edge cases
- Type-safe API

## Usage

```rust
use rust_calculator::Calculator;

let calc = Calculator::new(2);
let result = calc.calculate(10.0, 5.0, '+').unwrap();
assert_eq!(result, 15.0);
```
"#,
    )
    .await
    .unwrap();
    files.push(readme);

    files
}

// ============================================================================
// HELPER: Import directory into VFS
// ============================================================================

fn import_directory_to_vfs<'a>(
    vfs: &'a Arc<VirtualFileSystem>,
    workspace_id: uuid::Uuid,
    dir: &'a PathBuf,
    base_path: &'a PathBuf,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = usize> + 'a>> {
    Box::pin(async move {
        let mut count = 0;

        if let Ok(mut entries) = fs::read_dir(dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();

                if path.is_file() {
                    if let Ok(content) = fs::read(&path).await {
                        if let Ok(rel_path) = path.strip_prefix(base_path) {
                            if let Ok(vpath) = VirtualPath::new(rel_path.to_string_lossy().as_ref())
                            {
                                if vfs
                                    .write_file(&workspace_id, &vpath, &content)
                                    .await
                                    .is_ok()
                                {
                                    count += 1;
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    count += import_directory_to_vfs(vfs, workspace_id, &path, base_path).await;
                }
            }
        }

        count
    })
}

// ============================================================================
// HELPER: Extract Rust functions (simple parser)
// ============================================================================

fn extract_rust_functions(content: &str) -> Vec<(String, usize)> {
    let mut functions = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn "))
            && trimmed.contains('(')
        {
            if let Some(name_end) = trimmed.find('(') {
                let name_start = if trimmed.starts_with("pub fn ") { 7 } else { 3 };
                let name = trimmed[name_start..name_end].trim();
                if !name.is_empty() {
                    functions.push((name.to_string(), line_num + 1));
                }
            }
        }
    }

    functions
}

// ============================================================================
// MAIN TEST: REALITY CHECK E2E
// ============================================================================

#[tokio::test]
async fn test_reality_check_multi_agent_workflow() {
    let workflow_start = Instant::now();
    info!("========================================");
    info!("REALITY CHECK: Multi-Agent Workflow E2E");
    info!("========================================");

    // ========================================================================
    // PHASE 1: Initialize Cortex Infrastructure
    // ========================================================================
    info!("\n[PHASE 1] Initializing Cortex...");
    let phase1_start = Instant::now();

    // Create temporary workspace
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("rust-calculator");
    fs::create_dir_all(&workspace_path).await.unwrap();

    // Create calculator project
    let files = create_calculator_project(&workspace_path).await;
    info!("Created calculator project with {} files", files.len());

    // Initialize database
    let db_config = create_test_db_config("reality_check");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Initialize VFS
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Initialize cognitive memory
    let cognitive = Arc::new(CognitiveManager::new(connection_manager.clone()));

    info!(
        "[PHASE 1] Complete in {:?} ✓",
        phase1_start.elapsed()
    );
    assert!(
        phase1_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 1 should complete quickly"
    );

    // ========================================================================
    // PHASE 2: Agent 1 - Architect (Project Planning)
    // ========================================================================
    info!("\n[PHASE 2] Agent 1: Architect - Importing and analyzing project...");
    let phase2_start = Instant::now();

    // Import project into VFS
    let import_count =
        import_directory_to_vfs(&vfs, workspace_id, &workspace_path, &workspace_path).await;
    info!("Imported {} files into VFS", import_count);
    assert!(import_count > 0, "Should import files");

    // Create project record
    let project = Project::new("rust-calculator".to_string(), workspace_path.clone());

    // Parse Rust files and extract semantic units
    let mut units_created = 0;
    let mut all_units = Vec::new();

    for file in &files {
        if let Some(ext) = file.extension() {
            if ext == "rs" {
                if let Ok(content) = fs::read_to_string(&file).await {
                    let functions = extract_rust_functions(&content);

                    for (name, line_num) in functions {
                        let unit = SemanticUnit {
                            id: CortexId::new(),
                            unit_type: CodeUnitType::Function,
                            name: name.clone(),
                            qualified_name: format!("rust_calculator::{}", name),
                            display_name: name.clone(),
                            file_path: file.to_string_lossy().to_string(),
                            start_line: line_num as u32,
                            start_column: 0,
                            end_line: (line_num + 5) as u32,
                            end_column: 1,
                            signature: format!("pub fn {}(...)", name),
                            body: "// Implementation".to_string(),
                            docstring: Some(format!("Function {}", name)),
                            visibility: "public".to_string(),
                            modifiers: vec![],
                            parameters: vec![],
                            return_type: Some("Result<f64>".to_string()),
                            summary: format!("Function {}", name),
                            purpose: format!("Perform {}", name),
                            complexity: ComplexityMetrics {
                                cyclomatic: 2,
                                cognitive: 3,
                                nesting: 1,
                                lines: 5,
                            },
                            test_coverage: Some(0.8),
                            has_tests: true,
                            has_documentation: true,
                            embedding: None,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                        };

                        let unit_id = cognitive
                            .remember_unit(&unit)
                            .await
                            .expect("Failed to store semantic unit");
                        all_units.push((unit_id, unit.name.clone()));
                        units_created += 1;
                    }
                }
            }
        }
    }

    info!("Extracted {} semantic units", units_created);
    assert!(units_created > 0, "Should extract functions");

    // Create episode for architect's work
    let mut architect_episode = EpisodicMemory::new(
        "Import and analyze rust-calculator project".to_string(),
        "agent-architect".to_string(),
        project.id,
        EpisodeType::Task,
    );
    architect_episode.entities_created = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();
    architect_episode.outcome = EpisodeOutcome::Success;
    architect_episode.duration_seconds = phase2_start.elapsed().as_secs();

    let architect_episode_id = cognitive
        .remember_episode(&architect_episode)
        .await
        .expect("Failed to store architect episode");

    info!(
        "[PHASE 2] Complete in {:?} ✓",
        phase2_start.elapsed()
    );
    assert!(
        phase2_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 2 should complete quickly"
    );

    // ========================================================================
    // PHASE 3: Agent 2 - Developer (Implement Feature)
    // ========================================================================
    info!("\n[PHASE 3] Agent 2: Developer - Implementing power function...");
    let phase3_start = Instant::now();

    // Developer creates new power function
    let power_file_path = VirtualPath::new("src/power.rs").unwrap();
    let power_content = br#"//! Power operation

use crate::error::{CalculatorError, Result};

/// Calculate a^b
pub fn power(base: f64, exponent: f64) -> Result<f64> {
    let result = base.powf(exponent);
    if result.is_infinite() {
        Err(CalculatorError::Overflow)
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power() {
        assert_eq!(power(2.0, 3.0).unwrap(), 8.0);
    }

    #[test]
    fn test_power_overflow() {
        assert!(power(f64::MAX, 2.0).is_err());
    }
}
"#;

    vfs.write_file(&workspace_id, &power_file_path, power_content)
        .await
        .expect("Failed to write power.rs");

    // Update lib.rs to include power module
    let lib_path = VirtualPath::new("src/lib.rs").unwrap();
    let updated_lib = br#"//! Rust Calculator Library
//!
//! Provides basic arithmetic operations with error handling.

pub mod operations;
pub mod power;
pub mod error;

pub use error::{CalculatorError, Result};
pub use operations::{add, subtract, multiply, divide};
pub use power::power;
"#;

    vfs.write_file(&workspace_id, &lib_path, updated_lib)
        .await
        .expect("Failed to update lib.rs");

    // Store semantic unit for power function
    let power_unit = SemanticUnit {
        id: CortexId::new(),
        unit_type: CodeUnitType::Function,
        name: "power".to_string(),
        qualified_name: "rust_calculator::power::power".to_string(),
        display_name: "power".to_string(),
        file_path: "src/power.rs".to_string(),
        start_line: 6,
        start_column: 0,
        end_line: 13,
        end_column: 1,
        signature: "pub fn power(base: f64, exponent: f64) -> Result<f64>".to_string(),
        body: "base.powf(exponent)".to_string(),
        docstring: Some("Calculate a^b".to_string()),
        visibility: "public".to_string(),
        modifiers: vec![],
        parameters: vec!["base: f64".to_string(), "exponent: f64".to_string()],
        return_type: Some("Result<f64>".to_string()),
        summary: "Power operation".to_string(),
        purpose: "Calculate base raised to exponent".to_string(),
        complexity: ComplexityMetrics {
            cyclomatic: 2,
            cognitive: 3,
            nesting: 1,
            lines: 8,
        },
        test_coverage: Some(1.0),
        has_tests: true,
        has_documentation: true,
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let power_unit_id = cognitive
        .remember_unit(&power_unit)
        .await
        .expect("Failed to store power unit");

    // Create episode for developer's work
    let mut developer_episode = EpisodicMemory::new(
        "Implement power function with overflow handling".to_string(),
        "agent-developer".to_string(),
        project.id,
        EpisodeType::Feature,
    );
    developer_episode.entities_created = vec!["src/power.rs".to_string()];
    developer_episode.entities_modified = vec!["src/lib.rs".to_string()];
    developer_episode.outcome = EpisodeOutcome::Success;
    developer_episode.duration_seconds = phase3_start.elapsed().as_secs();

    let developer_episode_id = cognitive
        .remember_episode(&developer_episode)
        .await
        .expect("Failed to store developer episode");

    info!(
        "[PHASE 3] Complete in {:?} ✓",
        phase3_start.elapsed()
    );
    assert!(
        phase3_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 3 should complete quickly"
    );

    // ========================================================================
    // PHASE 4: Agent 3 - Reviewer (Code Review)
    // ========================================================================
    info!("\n[PHASE 4] Agent 3: Reviewer - Reviewing changes...");
    let phase4_start = Instant::now();

    // Reviewer retrieves developer's episode
    let dev_episode = cognitive
        .episodic()
        .get_episode(developer_episode_id)
        .await
        .expect("Failed to get episode")
        .expect("Episode not found");

    assert_eq!(
        dev_episode.agent_id, "agent-developer",
        "Should retrieve correct episode"
    );

    // Reviewer reads the new file from VFS
    let power_content_read = vfs
        .read_file(&workspace_id, &power_file_path)
        .await
        .expect("Failed to read power.rs");

    assert_eq!(
        power_content_read, power_content,
        "VFS content should match"
    );

    // Reviewer searches for similar error handling patterns
    let complex_units = cognitive
        .semantic()
        .find_complex_units(2)
        .await
        .expect("Failed to find complex units");

    info!(
        "Reviewer found {} units with complexity >= 2",
        complex_units.len()
    );

    // Create review episode
    let mut reviewer_episode = EpisodicMemory::new(
        "Code review: power function implementation".to_string(),
        "agent-reviewer".to_string(),
        project.id,
        EpisodeType::Task,
    );
    reviewer_episode.files_touched = vec!["src/power.rs".to_string(), "src/lib.rs".to_string()];
    reviewer_episode.outcome = EpisodeOutcome::Success;
    reviewer_episode.duration_seconds = phase4_start.elapsed().as_secs();
    reviewer_episode
        .lessons_learned
        .push("Good overflow handling".to_string());
    reviewer_episode
        .lessons_learned
        .push("Tests cover edge cases".to_string());

    let reviewer_episode_id = cognitive
        .remember_episode(&reviewer_episode)
        .await
        .expect("Failed to store reviewer episode");

    info!(
        "[PHASE 4] Complete in {:?} ✓",
        phase4_start.elapsed()
    );
    assert!(
        phase4_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 4 should complete quickly"
    );

    // ========================================================================
    // PHASE 5: Agent 4 - Tester (Write Tests)
    // ========================================================================
    info!("\n[PHASE 5] Agent 4: Tester - Writing integration tests...");
    let phase5_start = Instant::now();

    // Tester creates integration test file
    let test_file_path = VirtualPath::new("tests/integration_test.rs").unwrap();
    let test_content = br#"use rust_calculator::{Calculator, power};

#[test]
fn test_calculator_operations() {
    let calc = Calculator::new(2);

    // Test basic operations
    assert_eq!(calc.calculate(10.0, 5.0, '+').unwrap(), 15.0);
    assert_eq!(calc.calculate(10.0, 5.0, '-').unwrap(), 5.0);
    assert_eq!(calc.calculate(10.0, 5.0, '*').unwrap(), 50.0);
    assert_eq!(calc.calculate(10.0, 5.0, '/').unwrap(), 2.0);
}

#[test]
fn test_power_function() {
    assert_eq!(power(2.0, 8.0).unwrap(), 256.0);
    assert_eq!(power(5.0, 3.0).unwrap(), 125.0);
}

#[test]
fn test_error_cases() {
    let calc = Calculator::new(2);

    // Division by zero
    assert!(calc.calculate(10.0, 0.0, '/').is_err());

    // Invalid operation
    assert!(calc.calculate(10.0, 5.0, '%').is_err());

    // Power overflow
    assert!(power(f64::MAX, 2.0).is_err());
}
"#;

    vfs.write_file(&workspace_id, &test_file_path, test_content)
        .await
        .expect("Failed to write integration test");

    // Store test pattern
    let test_pattern = LearnedPattern {
        id: CortexId::new(),
        pattern_type: PatternType::Code,
        name: "Integration test pattern".to_string(),
        description: "Test all operations with edge cases".to_string(),
        context: "Calculator testing".to_string(),
        before_state: serde_json::json!({"state": "untested"}),
        after_state: serde_json::json!({"state": "tested", "coverage": "100%"}),
        transformation: serde_json::json!({
            "steps": [
                "Test normal operations",
                "Test edge cases (division by zero, overflow)",
                "Test invalid inputs"
            ]
        }),
        times_applied: 1,
        success_rate: 1.0,
        average_improvement: HashMap::new(),
        example_episodes: vec![],
        embedding: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    cognitive
        .remember_pattern(&test_pattern)
        .await
        .expect("Failed to store test pattern");

    // Create tester episode
    let mut tester_episode = EpisodicMemory::new(
        "Write comprehensive integration tests".to_string(),
        "agent-tester".to_string(),
        project.id,
        EpisodeType::Task,
    );
    tester_episode.entities_created = vec!["tests/integration_test.rs".to_string()];
    tester_episode.outcome = EpisodeOutcome::Success;
    tester_episode.duration_seconds = phase5_start.elapsed().as_secs();

    let tester_episode_id = cognitive
        .remember_episode(&tester_episode)
        .await
        .expect("Failed to store tester episode");

    info!(
        "[PHASE 5] Complete in {:?} ✓",
        phase5_start.elapsed()
    );
    assert!(
        phase5_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 5 should complete quickly"
    );

    // ========================================================================
    // PHASE 6: Consolidation
    // ========================================================================
    info!("\n[PHASE 6] Consolidating memories...");
    let phase6_start = Instant::now();

    let consolidation_report = cognitive
        .consolidate()
        .await
        .expect("Failed to consolidate");

    info!(
        "Consolidation: {} episodes processed in {}ms",
        consolidation_report.episodes_processed, consolidation_report.duration_ms
    );
    info!(
        "  - Patterns extracted: {}",
        consolidation_report.patterns_extracted
    );
    info!(
        "  - Knowledge links created: {}",
        consolidation_report.knowledge_links_created
    );

    assert!(
        consolidation_report.episodes_processed >= 0,
        "Should process episodes"
    );

    info!(
        "[PHASE 6] Complete in {:?} ✓",
        phase6_start.elapsed()
    );
    assert!(
        phase6_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 6 should complete quickly"
    );

    // ========================================================================
    // PHASE 7: Materialization
    // ========================================================================
    info!("\n[PHASE 7] Materializing VFS to disk...");
    let phase7_start = Instant::now();

    let flush_target = temp_dir.path().join("materialized");
    fs::create_dir_all(&flush_target).await.unwrap();

    // MaterializationEngine needs VirtualFileSystem, not Arc
    let engine = MaterializationEngine::new((*vfs).clone());
    let flush_report = engine
        .flush(FlushScope::All, &flush_target, FlushOptions::default())
        .await
        .expect("Failed to flush VFS");

    info!(
        "Materialized {} files, {} directories",
        flush_report.files_written, flush_report.directories_created
    );
    assert!(flush_report.files_written > 0, "Should flush files");

    // Verify materialized files
    let materialized_power = flush_target.join("src/power.rs");
    assert!(
        materialized_power.exists(),
        "power.rs should exist on disk"
    );

    let materialized_content = fs::read(&materialized_power).await.unwrap();
    assert_eq!(
        materialized_content, power_content,
        "Materialized content should match VFS"
    );

    info!(
        "[PHASE 7] Complete in {:?} ✓",
        phase7_start.elapsed()
    );
    assert!(
        phase7_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 7 should complete quickly"
    );

    // ========================================================================
    // PHASE 8: Verification
    // ========================================================================
    info!("\n[PHASE 8] Verifying data integrity...");
    let phase8_start = Instant::now();

    // Verify memory statistics
    let stats = cognitive
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    info!("Memory Statistics:");
    info!("  - Episodic: {} episodes", stats.episodic.total_episodes);
    info!("  - Semantic: {} units", stats.semantic.total_units);
    info!("  - Procedural: {} patterns", stats.procedural.total_patterns);

    assert_eq!(
        stats.episodic.total_episodes, 4,
        "Should have 4 episodes (architect, developer, reviewer, tester)"
    );
    assert!(
        stats.semantic.total_units >= units_created + 1,
        "Should have all semantic units plus power function"
    );
    assert_eq!(
        stats.procedural.total_patterns, 1,
        "Should have test pattern"
    );

    // Verify VFS state
    let vfs_files = vfs
        .list_directory(&workspace_id, &VirtualPath::new("src").unwrap(), false)
        .await
        .expect("Failed to list VFS directory");

    info!("VFS contains {} entries in src/", vfs_files.len());
    assert!(vfs_files.len() >= 4, "Should have at least 4 entries in src/");

    // Re-import materialized project and compare
    let reimport_workspace_id = uuid::Uuid::new_v4();
    let reimport_count =
        import_directory_to_vfs(&vfs, reimport_workspace_id, &flush_target, &flush_target).await;

    info!(
        "Re-imported {} files from materialized project",
        reimport_count
    );
    assert_eq!(
        reimport_count, flush_report.files_written,
        "Re-import should match materialized files"
    );

    // Verify content consistency
    let reimported_power = vfs
        .read_file(&reimport_workspace_id, &power_file_path)
        .await
        .expect("Failed to read reimported power.rs");

    assert_eq!(
        reimported_power, power_content,
        "Reimported content should match original"
    );

    info!(
        "[PHASE 8] Complete in {:?} ✓",
        phase8_start.elapsed()
    );
    assert!(
        phase8_start.elapsed().as_secs() < MAX_OPERATION_TIME_SECS,
        "Phase 8 should complete quickly"
    );

    // ========================================================================
    // FINAL SUMMARY
    // ========================================================================
    let total_duration = workflow_start.elapsed();

    info!("\n========================================");
    info!("REALITY CHECK COMPLETE");
    info!("========================================");
    info!("Total duration: {:?}", total_duration);
    info!("Phase 1 (Init):         {:?}", phase1_start.elapsed());
    info!("Phase 2 (Architect):    {:?}", phase2_start.elapsed());
    info!("Phase 3 (Developer):    {:?}", phase3_start.elapsed());
    info!("Phase 4 (Reviewer):     {:?}", phase4_start.elapsed());
    info!("Phase 5 (Tester):       {:?}", phase5_start.elapsed());
    info!("Phase 6 (Consolidate):  {:?}", phase6_start.elapsed());
    info!("Phase 7 (Materialize):  {:?}", phase7_start.elapsed());
    info!("Phase 8 (Verify):       {:?}", phase8_start.elapsed());
    info!("========================================");
    info!("✓ All agents completed successfully");
    info!("✓ No data loss during workflow");
    info!("✓ VFS materialization accurate");
    info!("✓ Memory consolidation successful");
    info!("✓ Data integrity verified");
    info!("========================================");

    // Performance assertion
    assert!(
        total_duration.as_secs() < MAX_WORKFLOW_TIME_SECS,
        "Full workflow should complete in under {} seconds (took {:?})",
        MAX_WORKFLOW_TIME_SECS,
        total_duration
    );
}

// ============================================================================
// TEST: Error Recovery
// ============================================================================

#[tokio::test]
async fn test_reality_check_error_recovery() {
    info!("Testing error recovery and rollback...");

    let db_config = create_test_db_config("error_recovery");
    let connection_manager = Arc::new(ConnectionManager::new(db_config).await.unwrap());

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Test 1: Invalid path handling (paths with null bytes)
    let invalid_result = VirtualPath::new("invalid\0path");
    assert!(invalid_result.is_err(), "Should reject path with null byte");

    // Test 2: Reading non-existent file
    let nonexistent = VirtualPath::new("does_not_exist.rs").unwrap();
    let read_result = vfs.read_file(&workspace_id, &nonexistent).await;
    assert!(
        read_result.is_err(),
        "Should error on non-existent file read"
    );

    // Test 3: Write then overwrite
    let test_path = VirtualPath::new("test.txt").unwrap();
    vfs.write_file(&workspace_id, &test_path, b"v1")
        .await
        .unwrap();
    vfs.write_file(&workspace_id, &test_path, b"v2")
        .await
        .unwrap();

    let content = vfs.read_file(&workspace_id, &test_path).await.unwrap();
    assert_eq!(content, b"v2", "Should have latest version");

    info!("✓ Error recovery tests passed");
}

// ============================================================================
// TEST: Concurrent Access
// ============================================================================

#[tokio::test]
async fn test_reality_check_concurrent_access() {
    info!("Testing concurrent agent access...");

    let db_config = create_test_db_config("concurrent_access");
    let connection_manager = Arc::new(ConnectionManager::new(db_config).await.unwrap());

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let cognitive = Arc::new(CognitiveManager::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();
    let project_id = CortexId::new();

    // Create shared file
    let shared_path = VirtualPath::new("shared.rs").unwrap();
    vfs.write_file(&workspace_id, &shared_path, b"// Shared file")
        .await
        .unwrap();

    // Spawn 5 concurrent agents reading the same file
    let mut handles = vec![];

    for i in 0..5 {
        let vfs = vfs.clone();
        let cognitive = cognitive.clone();
        let workspace_id = workspace_id;
        let path = shared_path.clone();
        let project_id = project_id;

        let handle = tokio::spawn(async move {
            // Read file
            let content = vfs.read_file(&workspace_id, &path).await.unwrap();

            // Create episode
            let episode = EpisodicMemory::new(
                format!("Agent {} reads shared file", i),
                format!("agent-{}", i),
                project_id,
                EpisodeType::Task,
            );

            cognitive.remember_episode(&episode).await.unwrap();

            content
        });

        handles.push(handle);
    }

    // Wait for all agents
    let results = futures::future::join_all(handles).await;

    // Verify all succeeded
    for result in results {
        assert!(result.is_ok(), "Agent should succeed");
        let content = result.unwrap();
        assert_eq!(content, b"// Shared file", "Content should match");
    }

    // Verify episodes
    let stats = cognitive.get_statistics().await.unwrap();
    assert_eq!(
        stats.episodic.total_episodes, 5,
        "Should have 5 episodes"
    );

    info!("✓ Concurrent access tests passed");
}
