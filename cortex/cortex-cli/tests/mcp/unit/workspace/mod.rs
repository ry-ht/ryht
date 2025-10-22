//! Unit tests for workspace MCP tools
//!
//! This module contains comprehensive unit tests for workspace management tools:
//! - workspace.create - Import existing project
//! - workspace.get - Get workspace info
//! - workspace.list - List all workspaces
//! - workspace.activate - Set active workspace
//!
//! Each test module covers:
//! - Successful operations
//! - Error cases (invalid paths, missing workspace, etc.)
//! - Edge cases (empty workspace, large workspace, special characters)
//! - Performance (should complete quickly)
//! - Token efficiency (compare to traditional approach)

pub mod test_create;
pub mod test_get;
pub mod test_list;
pub mod test_activate;

/// Common test utilities and harness
pub mod utils {
    use cortex_mcp::tools::workspace::WorkspaceContext;
    use cortex_storage::ConnectionManager;
    use cortex_storage::connection::ConnectionConfig;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tokio::fs;

    /// Test harness for workspace tests
    pub struct TestHarness {
        pub ctx: WorkspaceContext,
        pub temp_dir: TempDir,
    }

    impl TestHarness {
        /// Create a new test harness with in-memory database
        pub async fn new() -> Self {
            let temp_dir = TempDir::new().unwrap();
            let config = ConnectionConfig::memory();
            let storage = Arc::new(ConnectionManager::new(config).await.unwrap());
            let ctx = WorkspaceContext::new(storage).unwrap();

            Self { ctx, temp_dir }
        }

        /// Get the temporary directory path
        pub fn temp_path(&self) -> &std::path::Path {
            self.temp_dir.path()
        }

        /// Create a simple Rust project for testing
        pub async fn create_rust_project(&self, name: &str) -> std::io::Result<std::path::PathBuf> {
            let project_dir = self.temp_path().join(name);
            fs::create_dir(&project_dir).await?;

            // Create Cargo.toml
            let cargo_toml = format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
                name
            );
            fs::write(project_dir.join("Cargo.toml"), cargo_toml).await?;

            // Create src directory
            fs::create_dir(project_dir.join("src")).await?;

            // Create src/lib.rs
            let lib_rs = r#"//! Test library

/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A simple point structure
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Calculate distance from origin
    pub fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;
            fs::write(project_dir.join("src").join("lib.rs"), lib_rs).await?;

            // Create src/main.rs
            let main_rs = r#"fn main() {
    println!("Hello, world!");
}
"#;
            fs::write(project_dir.join("src").join("main.rs"), main_rs).await?;

            // Create .gitignore
            let gitignore = r#"target/
Cargo.lock
.DS_Store
"#;
            fs::write(project_dir.join(".gitignore"), gitignore).await?;

            Ok(project_dir)
        }

        /// Create an empty directory
        pub async fn create_empty_dir(&self, name: &str) -> std::io::Result<std::path::PathBuf> {
            let dir = self.temp_path().join(name);
            fs::create_dir(&dir).await?;
            Ok(dir)
        }

        /// Create a large project with many files for performance testing
        pub async fn create_large_project(
            &self,
            name: &str,
            file_count: usize,
        ) -> std::io::Result<std::path::PathBuf> {
            let project_dir = self.create_rust_project(name).await?;

            // Create many additional files
            for i in 0..file_count {
                let filename = format!("module_{}.rs", i);
                let content = format!(
                    r#"//! Module {}

pub fn function_{}() -> i32 {{
    {}
}}

pub struct Struct{} {{
    pub field: i32,
}}
"#,
                    i, i, i, i
                );
                fs::write(project_dir.join("src").join(filename), content).await?;
            }

            Ok(project_dir)
        }

        /// Create a project with special characters in filenames
        pub async fn create_special_chars_project(
            &self,
            name: &str,
        ) -> std::io::Result<std::path::PathBuf> {
            let project_dir = self.create_rust_project(name).await?;

            // Add files with special characters (but valid for filesystems)
            let special_files = vec![
                "file-with-dashes.rs",
                "file_with_underscores.rs",
                "file.with.dots.rs",
                "file123.rs",
            ];

            for filename in special_files {
                let content = format!("// File: {}\npub fn test() {{}}\n", filename);
                fs::write(project_dir.join("src").join(filename), content).await?;
            }

            Ok(project_dir)
        }

        /// Get default MCP tool context
        pub fn tool_context() -> mcp_sdk::prelude::ToolContext {
            mcp_sdk::prelude::ToolContext::default()
        }
    }
}
