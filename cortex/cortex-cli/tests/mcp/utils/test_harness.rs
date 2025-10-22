//! Test harness for MCP tools testing
//!
//! Provides comprehensive setup and utilities for testing all MCP tools:
//! - In-memory database configuration
//! - Temporary directory management
//! - Tool context creation with all dependencies
//! - Performance and token tracking
//! - Common operations (workspace creation, project loading, etc.)

use cortex_parser::CodeParser;
use cortex_storage::{ConnectionManager, connection::ConnectionConfig};
use cortex_vfs::{VirtualFileSystem, ExternalProjectLoader, MaterializationEngine, FileIngestionPipeline, Workspace, WorkspaceType, SourceType};
use cortex_memory::SemanticMemorySystem;
use mcp_sdk::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;

/// Main test harness for MCP tools
pub struct TestHarness {
    /// Temporary directory for test files
    temp_dir: TempDir,
    /// Storage manager
    pub storage: Arc<ConnectionManager>,
    /// Virtual filesystem
    pub vfs: Arc<VirtualFileSystem>,
    /// Project loader
    pub loader: Arc<ExternalProjectLoader>,
    /// Materialization engine
    pub engine: Arc<MaterializationEngine>,
    /// Code parser
    pub parser: Arc<tokio::sync::Mutex<CodeParser>>,
    /// Semantic memory system
    pub semantic_memory: Arc<SemanticMemorySystem>,
    /// File ingestion pipeline
    pub ingestion: Arc<FileIngestionPipeline>,
    /// Performance metrics
    metrics: TestMetrics,
}

/// Performance metrics for test execution
#[derive(Debug, Default, Clone)]
pub struct TestMetrics {
    pub operations: Vec<OperationMetric>,
    pub total_tokens_traditional: usize,
    pub total_tokens_cortex: usize,
}

#[derive(Debug, Clone)]
pub struct OperationMetric {
    pub name: String,
    pub duration_ms: u64,
    pub tokens_used: usize,
}

impl TestHarness {
    /// Create a new test harness with in-memory database
    pub async fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        // Configure in-memory database
        let config = ConnectionConfig::memory();
        let storage = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager")
        );

        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let loader = Arc::new(ExternalProjectLoader::new((*vfs).clone()));
        let engine = Arc::new(MaterializationEngine::new((*vfs).clone()));
        let parser = Arc::new(tokio::sync::Mutex::new(
            CodeParser::new().expect("Failed to create parser")
        ));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
        let ingestion = Arc::new(FileIngestionPipeline::new(
            parser.clone(),
            vfs.clone(),
            semantic_memory.clone(),
        ));

        Self {
            temp_dir,
            storage,
            vfs,
            loader,
            engine,
            parser,
            semantic_memory,
            ingestion,
            metrics: TestMetrics::default(),
        }
    }

    /// Get path to temporary directory
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a test context for workspace tools
    pub fn workspace_context(&self) -> cortex_cli::mcp::tools::workspace::WorkspaceContext {
        cortex_cli::mcp::tools::workspace::WorkspaceContext::new(self.storage.clone())
            .expect("Failed to create workspace context")
    }

    /// Create a test context for VFS tools
    pub fn vfs_context(&self) -> cortex_cli::mcp::tools::vfs::VfsContext {
        cortex_cli::mcp::tools::vfs::VfsContext::new(
            self.storage.clone(),
            self.vfs.clone(),
            self.loader.clone(),
            self.engine.clone(),
        )
    }

    /// Create a test context for code navigation tools
    pub fn code_nav_context(&self) -> cortex_cli::mcp::tools::code_nav::CodeNavContext {
        cortex_cli::mcp::tools::code_nav::CodeNavContext::new(
            self.storage.clone(),
            self.vfs.clone(),
        )
    }

    /// Create a test context for code manipulation tools
    pub fn code_manipulation_context(&self) -> cortex_cli::mcp::tools::code_manipulation::CodeManipulationContext {
        cortex_cli::mcp::tools::code_manipulation::CodeManipulationContext::new(
            self.storage.clone(),
            self.vfs.clone(),
            self.parser.clone(),
        )
    }

    /// Create a test context for semantic search tools
    pub fn semantic_search_context(&self) -> cortex_cli::mcp::tools::semantic_search::SemanticSearchContext {
        cortex_cli::mcp::tools::semantic_search::SemanticSearchContext::new(
            self.storage.clone(),
            self.semantic_memory.clone(),
        )
    }

    /// Create a test context for dependency analysis tools
    pub fn dependency_context(&self) -> cortex_cli::mcp::tools::dependency_analysis::DependencyContext {
        cortex_cli::mcp::tools::dependency_analysis::DependencyContext::new(
            self.storage.clone(),
            self.vfs.clone(),
        )
    }

    /// Create a workspace with a test project
    pub async fn create_test_workspace(
        &self,
        name: &str,
        project_path: &Path,
    ) -> TestWorkspace {
        let workspace_id = Uuid::new_v4();
        let workspace = Workspace {
            id: workspace_id,
            name: name.to_string(),
            root_path: project_path.to_path_buf(),
            workspace_type: WorkspaceType::Code,
            source_type: SourceType::Local,
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_synced_at: None,
        };

        // Store workspace
        let conn = self.storage.acquire().await.expect("Failed to acquire connection");
        let _: Option<Workspace> = conn
            .connection()
            .create(("workspace", workspace_id.to_string()))
            .content(workspace.clone())
            .await
            .expect("Failed to store workspace");

        TestWorkspace {
            id: workspace_id,
            name: name.to_string(),
            path: project_path.to_path_buf(),
        }
    }

    /// Load a project into the VFS
    pub async fn load_project(&self, workspace_id: Uuid, path: &Path) -> LoadResult {
        let start = Instant::now();

        let result = self.loader
            .load_project(workspace_id, path, &Default::default())
            .await
            .expect("Failed to load project");

        let duration = start.elapsed();

        LoadResult {
            files_loaded: result.files_loaded,
            units_extracted: result.units_extracted,
            duration_ms: duration.as_millis() as u64,
        }
    }

    /// Ingest a file and extract code units
    pub async fn ingest_file(
        &self,
        workspace_id: Uuid,
        file_path: &Path,
        content: &str,
    ) -> IngestResult {
        let start = Instant::now();

        let virtual_path = file_path.to_string_lossy().to_string();

        let result = self.ingestion
            .ingest_file(workspace_id, &virtual_path, content)
            .await
            .expect("Failed to ingest file");

        let duration = start.elapsed();

        IngestResult {
            units_extracted: result.units_extracted,
            embeddings_created: result.embeddings_created,
            duration_ms: duration.as_millis() as u64,
        }
    }

    /// Record a test operation metric
    pub fn record_metric(&mut self, name: impl Into<String>, duration_ms: u64, tokens: usize) {
        self.metrics.operations.push(OperationMetric {
            name: name.into(),
            duration_ms,
            tokens_used: tokens,
        });
    }

    /// Record traditional approach tokens
    pub fn record_traditional_tokens(&mut self, tokens: usize) {
        self.metrics.total_tokens_traditional += tokens;
    }

    /// Record cortex approach tokens
    pub fn record_cortex_tokens(&mut self, tokens: usize) {
        self.metrics.total_tokens_cortex += tokens;
    }

    /// Get test metrics
    pub fn metrics(&self) -> &TestMetrics {
        &self.metrics
    }

    /// Calculate token savings percentage
    pub fn token_savings_percent(&self) -> f64 {
        if self.metrics.total_tokens_traditional == 0 {
            return 0.0;
        }

        let savings = self.metrics.total_tokens_traditional.saturating_sub(
            self.metrics.total_tokens_cortex
        );

        100.0 * savings as f64 / self.metrics.total_tokens_traditional as f64
    }

    /// Print performance summary
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("TEST PERFORMANCE SUMMARY");
        println!("{}", "=".repeat(80));

        for op in &self.metrics.operations {
            println!(
                "  {:<40} {:>8} ms  {:>8} tokens",
                op.name, op.duration_ms, op.tokens_used
            );
        }

        println!("{}", "-".repeat(80));
        println!("  Traditional approach: {} tokens", self.metrics.total_tokens_traditional);
        println!("  Cortex approach:      {} tokens", self.metrics.total_tokens_cortex);
        println!("  Savings:              {:.1}%", self.token_savings_percent());
        println!("{}", "=".repeat(80));
    }
}

/// Test context for general MCP operations
#[derive(Clone)]
pub struct TestContext {
    pub storage: Arc<ConnectionManager>,
    pub vfs: Arc<VirtualFileSystem>,
}

impl TestContext {
    pub async fn new() -> Self {
        let config = ConnectionConfig::memory();
        let storage = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager")
        );
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

        Self { storage, vfs }
    }
}

/// A test workspace
#[derive(Debug, Clone)]
pub struct TestWorkspace {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
}

/// Result from loading a project
#[derive(Debug)]
pub struct LoadResult {
    pub files_loaded: usize,
    pub units_extracted: usize,
    pub duration_ms: u64,
}

/// Result from ingesting a file
#[derive(Debug)]
pub struct IngestResult {
    pub units_extracted: usize,
    pub embeddings_created: usize,
    pub duration_ms: u64,
}

// Helper functions for common test operations

/// Create a temporary Rust project
pub async fn create_rust_project(dir: &Path, name: &str) -> std::io::Result<()> {
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.0", features = ["full"] }}
"#,
        name
    );
    fs::write(dir.join("Cargo.toml"), cargo_toml).await?;

    fs::create_dir(dir.join("src")).await?;

    let lib_rs = r#"//! Library crate

use serde::{Deserialize, Serialize};

/// A user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    /// Create a new user
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }

    /// Validate the user's email
    pub fn validate_email(&self) -> bool {
        self.email.contains('@')
    }
}

/// Add two numbers
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiply two numbers
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_multiply() {
        assert_eq!(multiply(3, 4), 12);
    }
}
"#;
    fs::write(dir.join("src/lib.rs"), lib_rs).await?;

    let main_rs = r#"use std::io::{self, Write};

fn main() {
    println!("Hello, world!");

    let user = test_project::User::new(
        1,
        "Alice".to_string(),
        "alice@example.com".to_string(),
    );

    println!("User: {:?}", user);
    println!("Email valid: {}", user.validate_email());
}
"#;
    fs::write(dir.join("src/main.rs"), main_rs).await?;

    Ok(())
}

/// Create a temporary TypeScript project
pub async fn create_typescript_project(dir: &Path, name: &str) -> std::io::Result<()> {
    let package_json = format!(
        r#"{{
  "name": "{}",
  "version": "1.0.0",
  "description": "Test TypeScript project",
  "main": "dist/index.js",
  "scripts": {{
    "build": "tsc",
    "test": "jest"
  }},
  "dependencies": {{
    "express": "^4.18.0"
  }},
  "devDependencies": {{
    "@types/node": "^20.0.0",
    "@types/express": "^4.17.0",
    "typescript": "^5.0.0",
    "jest": "^29.0.0"
  }}
}}
"#,
        name
    );
    fs::write(dir.join("package.json"), package_json).await?;

    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true
  }
}
"#;
    fs::write(dir.join("tsconfig.json"), tsconfig).await?;

    fs::create_dir(dir.join("src")).await?;

    let index_ts = r#"import express from 'express';

interface User {
  id: number;
  name: string;
  email: string;
}

class UserService {
  private users: Map<number, User> = new Map();

  createUser(name: string, email: string): User {
    const id = this.users.size + 1;
    const user: User = { id, name, email };
    this.users.set(id, user);
    return user;
  }

  getUser(id: number): User | undefined {
    return this.users.get(id);
  }

  getAllUsers(): User[] {
    return Array.from(this.users.values());
  }
}

const app = express();
const userService = new UserService();

app.use(express.json());

app.post('/users', (req, res) => {
  const { name, email } = req.body;
  const user = userService.createUser(name, email);
  res.json(user);
});

app.get('/users/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const user = userService.getUser(id);
  if (user) {
    res.json(user);
  } else {
    res.status(404).json({ error: 'User not found' });
  }
});

app.get('/users', (req, res) => {
  res.json(userService.getAllUsers());
});

export { app, UserService };
"#;
    fs::write(dir.join("src/index.ts"), index_ts).await?;

    Ok(())
}
