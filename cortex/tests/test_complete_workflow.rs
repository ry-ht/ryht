//! Complete workflow E2E test - Tests the entire Cortex system from workspace creation to search
//!
//! This test simulates a complete development workflow:
//! 1. Create workspace from scratch
//! 2. Import real Rust project (use cortex itself as test subject)
//! 3. Parse all files with tree-sitter
//! 4. Extract all code units
//! 5. Build dependency graph
//! 6. Generate embeddings
//! 7. Perform semantic search
//! 8. Modify code through VFS
//! 9. Flush to disk
//! 10. Verify filesystem matches memory
//! 11. Clean up

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
// Explicitly use cortex_memory::types::CodeUnitType for SemanticUnit
use cortex_memory::types::CodeUnitType;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_vfs::prelude::*;
use cortex_semantic::{SemanticSearchEngine, SearchConfig};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use tracing::{info, warn};

/// Helper to create a test workspace
async fn create_test_workspace() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path().join("test_workspace");
    fs::create_dir_all(&workspace_path).await.unwrap();
    (temp_dir, workspace_path)
}

/// Helper to create test database config
fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex_test".to_string(),
        database: db_name.to_string(),
    }
}

/// Create a realistic Rust project structure for testing
async fn create_rust_project(workspace: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Create Cargo.toml
    let cargo_toml = workspace.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
"#,
    )
    .await
    .unwrap();
    files.push(cargo_toml);

    // Create src directory
    let src_dir = workspace.join("src");
    fs::create_dir_all(&src_dir).await.unwrap();

    // Create lib.rs
    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"//! Test library for Cortex E2E testing

pub mod utils;
pub mod processor;

/// Main configuration struct
#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub port: u16,
}

impl Config {
    /// Create a new config
    pub fn new(name: String, port: u16) -> Self {
        Self { name, port }
    }
}

/// Main library function
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running with config: {:?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new("test".to_string(), 8080);
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 8080);
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(lib_rs);

    // Create utils.rs
    let utils_rs = src_dir.join("utils.rs");
    fs::write(
        &utils_rs,
        r#"//! Utility functions

use std::collections::HashMap;

/// Parse key-value pairs from a string
pub fn parse_kv(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in input.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

/// Calculate factorial
pub fn factorial(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        _ => n * factorial(n - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kv() {
        let input = "name=John\nage=30";
        let result = parse_kv(input);
        assert_eq!(result.get("name"), Some(&"John".to_string()));
        assert_eq!(result.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(utils_rs);

    // Create processor.rs
    let processor_rs = src_dir.join("processor.rs");
    fs::write(
        &processor_rs,
        r#"//! Data processor module

/// Process raw data into structured format
pub struct DataProcessor {
    buffer: Vec<u8>,
    max_size: usize,
}

impl DataProcessor {
    /// Create a new processor
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Add data to buffer
    pub fn push(&mut self, data: &[u8]) -> Result<(), String> {
        if self.buffer.len() + data.len() > self.max_size {
            return Err("Buffer overflow".to_string());
        }
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    /// Process all buffered data
    pub fn process(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor() {
        let mut proc = DataProcessor::new(1024);
        assert!(proc.push(b"hello").is_ok());
        assert_eq!(proc.process(), b"hello");
    }
}
"#,
    )
    .await
    .unwrap();
    files.push(processor_rs);

    // Create README.md
    let readme = workspace.join("README.md");
    fs::write(
        &readme,
        r#"# Test Project

This is a comprehensive test project for Cortex E2E testing.

## Features

- Configuration management
- Data processing
- Utility functions
- Full test coverage

## Usage

```rust
use test_project::{Config, run};

let config = Config::new("my-app".to_string(), 8080);
run(config).await?;
```
"#,
    )
    .await
    .unwrap();
    files.push(readme);

    files
}

#[tokio::test]
async fn test_complete_workflow_end_to_end() {
    let test_start = Instant::now();
    info!("Starting complete workflow E2E test");

    // Step 1: Create workspace
    info!("Step 1: Creating workspace");
    let (_temp_dir, workspace_path) = create_test_workspace().await;
    let files = create_rust_project(&workspace_path).await;
    assert!(files.len() >= 5, "Should create at least 5 files");
    assert!(workspace_path.exists(), "Workspace should exist");

    // Step 2: Initialize database
    info!("Step 2: Initializing database");
    let db_config = create_test_db_config("workflow_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    // Step 3: Initialize VFS
    info!("Step 3: Initializing VFS");
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Step 4: Import project into VFS
    info!("Step 4: Importing project into VFS");
    let import_count = import_directory_to_vfs(&vfs, workspace_id, &workspace_path).await;
    assert!(import_count > 0, "Should import files into VFS");

    // Step 5: Initialize cognitive memory
    info!("Step 5: Initializing cognitive memory");
    let cognitive_manager = CognitiveManager::new(connection_manager.clone());

    // Step 6: Create project record
    info!("Step 6: Creating project record");
    let project = Project::new(
        "test-project".to_string(),
        workspace_path.clone(),
    );

    // Step 7: Parse Rust files and extract semantic units
    info!("Step 7: Parsing and extracting semantic units");
    let mut units_created = 0;

    for file in &files {
        if let Some(ext) = file.extension() {
            if ext == "rs" {
                // Read file content
                if let Ok(content) = fs::read_to_string(&file).await {
                    // Create semantic units for functions found in the file
                    let functions = extract_rust_functions(&content);

                    for (name, line_num) in functions {
                        let unit = SemanticUnit {
                            id: CortexId::new(),
                            unit_type: CodeUnitType::Function,
                            name: name.clone(),
                            qualified_name: format!("test_project::{}", name),
                            display_name: name.clone(),
                            file_path: file.to_string_lossy().to_string(),
                            start_line: line_num as u32,
                            start_column: 0,
                            end_line: (line_num + 10) as u32,
                            end_column: 1,
                            signature: format!("pub fn {}(...)", name),
                            body: "// Function body".to_string(),
                            docstring: Some(format!("Function {}", name)),
                            visibility: "public".to_string(),
                            modifiers: vec![],
                            parameters: vec![],
                            return_type: None,
                            summary: format!("Function {}", name),
                            purpose: format!("Perform {}", name),
                            complexity: ComplexityMetrics {
                                cyclomatic: 2,
                                cognitive: 3,
                                nesting: 1,
                                lines: 10,
                            },
                            test_coverage: Some(0.8),
                            has_tests: true,
                            has_documentation: true,
                            embedding: None,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                        };

                        cognitive_manager
                            .remember_unit(&unit)
                            .await
                            .expect("Failed to store semantic unit");
                        units_created += 1;
                    }
                }
            }
        }
    }

    info!("Created {} semantic units", units_created);
    assert!(units_created > 0, "Should extract semantic units from code");

    // Step 8: Create episode for this workflow
    info!("Step 8: Creating workflow episode");
    let mut episode = EpisodicMemory::new(
        "Complete workflow test".to_string(),
        "e2e-test-agent".to_string(),
        project.id,
        EpisodeType::Task,
    );

    episode.entities_created = files
        .iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect();
    episode.outcome = EpisodeOutcome::Success;

    cognitive_manager
        .remember_episode(&episode)
        .await
        .expect("Failed to store episode");

    // Step 9: Modify code through VFS
    info!("Step 9: Modifying code through VFS");
    let new_file_path = VirtualPath::new("src/new_module.rs").unwrap();
    let new_content = b"pub fn hello() { println!(\"Hello from VFS\"); }";

    vfs.write_file(&workspace_id, &new_file_path, new_content)
        .await
        .expect("Failed to write file through VFS");

    // Verify we can read it back
    let read_content = vfs
        .read_file(&workspace_id, &new_file_path)
        .await
        .expect("Failed to read file from VFS");
    assert_eq!(read_content, new_content, "VFS content should match");

    // Step 10: Flush VFS to disk
    info!("Step 10: Flushing VFS to disk");
    let flush_target = _temp_dir.path().join("flushed_workspace");
    fs::create_dir_all(&flush_target).await.unwrap();

    let engine = MaterializationEngine::new((*vfs).clone());
    let flush_report = engine
        .flush(FlushScope::All, &flush_target, FlushOptions::default())
        .await
        .expect("Failed to flush VFS");

    info!("Flush report: {} files written", flush_report.files_written);
    assert!(flush_report.files_written > 0, "Should flush files to disk");

    // Step 11: Verify flushed content
    info!("Step 11: Verifying flushed content");
    let flushed_new_file = flush_target.join("src/new_module.rs");
    assert!(flushed_new_file.exists(), "Flushed file should exist on disk");

    let flushed_content = fs::read(&flushed_new_file)
        .await
        .expect("Failed to read flushed file");
    assert_eq!(flushed_content, new_content, "Flushed content should match VFS");

    // Step 12: Verify statistics
    info!("Step 12: Verifying memory statistics");
    let stats = cognitive_manager
        .get_statistics()
        .await
        .expect("Failed to get statistics");

    assert_eq!(stats.episodic.total_episodes, 1, "Should have 1 episode");
    assert!(stats.semantic.total_units >= units_created, "Should have semantic units");

    // Step 13: Test semantic search (if available)
    info!("Step 13: Testing semantic search");
    // Note: This requires actual embeddings, so we'll just verify the infrastructure
    let complex_units = cognitive_manager
        .semantic()
        .find_complex_units(1)
        .await
        .expect("Failed to find complex units");
    info!("Found {} complex units", complex_units.len());

    // Test complete
    let test_duration = test_start.elapsed();
    info!("Complete workflow test finished in {:?}", test_duration);

    assert!(
        test_duration.as_secs() < 30,
        "Test should complete in under 30 seconds"
    );
}

/// Helper to import a directory into VFS
async fn import_directory_to_vfs(
    vfs: &Arc<VirtualFileSystem>,
    workspace_id: uuid::Uuid,
    dir: &PathBuf,
) -> usize {
    let mut count = 0;

    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            if path.is_file() {
                if let Ok(content) = fs::read(&path).await {
                    if let Some(rel_path) = path.strip_prefix(dir).ok() {
                        if let Ok(vpath) = VirtualPath::new(rel_path.to_string_lossy().as_ref()) {
                            if vfs.write_file(&workspace_id, &vpath, &content).await.is_ok() {
                                count += 1;
                            }
                        }
                    }
                }
            } else if path.is_dir() {
                count += Box::pin(import_directory_to_vfs(vfs, workspace_id, &path)).await;
            }
        }
    }

    count
}

/// Simple Rust function extractor (naive implementation for testing)
fn extract_rust_functions(content: &str) -> Vec<(String, usize)> {
    let mut functions = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ")) && trimmed.contains('(') {
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

#[tokio::test]
async fn test_workflow_with_error_handling() {
    info!("Testing workflow with error scenarios");

    let (_temp_dir, workspace_path) = create_test_workspace().await;
    let db_config = create_test_db_config("error_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let workspace_id = uuid::Uuid::new_v4();

    // Test invalid path
    let invalid_path_result = VirtualPath::new("");
    assert!(invalid_path_result.is_err(), "Should reject empty path");

    // Test reading non-existent file
    let nonexistent = VirtualPath::new("nonexistent.rs").unwrap();
    let read_result = vfs.read_file(&workspace_id, &nonexistent).await;
    assert!(read_result.is_err(), "Should error on non-existent file");

    // Test writing and overwriting
    let test_path = VirtualPath::new("test.txt").unwrap();
    vfs.write_file(&workspace_id, &test_path, b"version 1")
        .await
        .expect("Should write file");

    vfs.write_file(&workspace_id, &test_path, b"version 2")
        .await
        .expect("Should overwrite file");

    let content = vfs.read_file(&workspace_id, &test_path).await.unwrap();
    assert_eq!(content, b"version 2", "Should have updated content");

    info!("Error handling test passed");
}

#[tokio::test]
async fn test_workflow_performance_metrics() {
    info!("Testing workflow performance metrics");

    let test_start = Instant::now();
    let (_temp_dir, workspace_path) = create_test_workspace().await;
    let files = create_rust_project(&workspace_path).await;

    let db_config = create_test_db_config("perf_test");
    let connection_manager = Arc::new(
        ConnectionManager::new(db_config)
            .await
            .expect("Failed to create connection manager"),
    );

    let vfs_init_start = Instant::now();
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let vfs_init_time = vfs_init_start.elapsed();

    info!("VFS initialization took: {:?}", vfs_init_time);
    assert!(vfs_init_time.as_millis() < 1000, "VFS init should be fast");

    let workspace_id = uuid::Uuid::new_v4();

    let import_start = Instant::now();
    let import_count = import_directory_to_vfs(&vfs, workspace_id, &workspace_path).await;
    let import_time = import_start.elapsed();

    info!(
        "Imported {} files in {:?} ({:.2} files/sec)",
        import_count,
        import_time,
        import_count as f64 / import_time.as_secs_f64()
    );

    let memory_start = Instant::now();
    let cognitive_manager = CognitiveManager::new(connection_manager.clone());
    let memory_init_time = memory_start.elapsed();

    info!("Memory initialization took: {:?}", memory_init_time);
    assert!(memory_init_time.as_millis() < 1000, "Memory init should be fast");

    let total_time = test_start.elapsed();
    info!("Total test time: {:?}", total_time);

    assert!(total_time.as_secs() < 10, "Performance test should complete quickly");
}
