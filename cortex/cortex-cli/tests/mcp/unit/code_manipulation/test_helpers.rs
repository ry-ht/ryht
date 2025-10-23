//! Test Helpers for Code Manipulation Tests
//!
//! Provides shared utilities and fixtures for testing code manipulation tools.

use crate::mcp::utils::test_harness::TestHarness;
use cortex_parser::{CodeParser, Language as ParserLanguage};
use mcp_sdk::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Test fixture for code manipulation tests
pub struct CodeManipulationFixture {
    pub harness: TestHarness,
    pub workspace_id: Uuid,
}

impl CodeManipulationFixture {
    /// Create a new test fixture
    pub async fn new() -> Self {
        let harness = TestHarness::new().await;
        let workspace_id = Uuid::new_v4();

        // Create workspace in database
        let workspace = cortex_vfs::Workspace {
            id: workspace_id,
            name: "test_workspace".to_string(),
            root_path: harness.temp_path().to_path_buf(),
            workspace_type: cortex_vfs::WorkspaceType::Code,
            source_type: cortex_vfs::SourceType::Local,
            metadata: Default::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_synced_at: None,
        };

        let conn = harness.storage.acquire().await.expect("Failed to acquire connection");
        let _: Option<cortex_vfs::Workspace> = conn
            .connection()
            .create(("workspace", workspace_id.to_string()))
            .content(workspace)
            .await
            .expect("Failed to store workspace");

        Self {
            harness,
            workspace_id,
        }
    }

    /// Get the code manipulation context
    pub fn context(&self) -> cortex_cli::mcp::tools::code_manipulation::CodeManipulationContext {
        self.harness.code_manipulation_context()
    }

    /// Create a file in the VFS
    pub async fn create_file(&self, path: &str, content: &str) -> Result<String, String> {
        use cortex_vfs::VirtualPath;

        let vpath = VirtualPath::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        self.harness.vfs
            .write_file(&self.workspace_id, &vpath, content.as_bytes())
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(path.to_string())
    }

    /// Read a file from the VFS
    pub async fn read_file(&self, path: &str) -> Result<String, String> {
        use cortex_vfs::VirtualPath;

        let vpath = VirtualPath::new(path).map_err(|e| format!("Invalid path: {}", e))?;
        let bytes = self.harness.vfs
            .read_file(&self.workspace_id, &vpath)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        String::from_utf8(bytes).map_err(|e| format!("File is not UTF-8: {}", e))
    }

    /// Validate that code is syntactically valid using AST parser
    pub async fn validate_syntax(&self, path: &str, content: &str) -> bool {
        let language = match path {
            p if p.ends_with(".rs") => ParserLanguage::Rust,
            p if p.ends_with(".ts") || p.ends_with(".tsx") => ParserLanguage::TypeScript,
            p if p.ends_with(".js") || p.ends_with(".jsx") => ParserLanguage::JavaScript,
            p if p.ends_with(".py") => ParserLanguage::Python,
            _ => return false,
        };

        let mut parser = match CodeParser::for_language(language) {
            Ok(p) => p,
            Err(_) => return false,
        };

        parser.parse_file(path, content, language).is_ok()
    }

    /// Execute a tool and measure performance
    pub async fn execute_tool<T: Tool>(
        &self,
        tool: &T,
        input: Value,
    ) -> (ToolResult, u64) {
        let start = Instant::now();
        let result = tool.execute(input).await;
        let duration = start.elapsed().as_millis() as u64;
        (result, duration)
    }

    /// Count tokens in content (simple whitespace-based estimation)
    pub fn count_tokens(&self, content: &str) -> usize {
        content.split_whitespace().count()
    }

    /// Calculate token efficiency percentage
    pub fn token_efficiency(&self, traditional: usize, cortex: usize) -> f64 {
        if traditional == 0 {
            return 0.0;
        }
        let savings = traditional.saturating_sub(cortex);
        100.0 * savings as f64 / traditional as f64
    }
}

/// Test metrics for tracking performance
#[derive(Debug, Default)]
pub struct TestMetrics {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_duration_ms: u64,
    pub ast_validations: usize,
    pub ast_validation_failures: usize,
    pub total_tokens_traditional: usize,
    pub total_tokens_cortex: usize,
}

impl TestMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_pass(&mut self, duration_ms: u64) {
        self.total_tests += 1;
        self.passed += 1;
        self.total_duration_ms += duration_ms;
    }

    pub fn record_fail(&mut self, duration_ms: u64) {
        self.total_tests += 1;
        self.failed += 1;
        self.total_duration_ms += duration_ms;
    }

    pub fn record_ast_validation(&mut self, passed: bool) {
        self.ast_validations += 1;
        if !passed {
            self.ast_validation_failures += 1;
        }
    }

    pub fn token_savings_percent(&self) -> f64 {
        if self.total_tokens_traditional == 0 {
            return 0.0;
        }
        let savings = self.total_tokens_traditional.saturating_sub(self.total_tokens_cortex);
        100.0 * savings as f64 / self.total_tokens_traditional as f64
    }

    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("CODE MANIPULATION TEST SUMMARY");
        println!("{}", "=".repeat(80));
        println!("Total Tests:              {}", self.total_tests);
        println!("Passed:                   {} ({:.1}%)",
            self.passed,
            100.0 * self.passed as f64 / self.total_tests.max(1) as f64
        );
        println!("Failed:                   {}", self.failed);
        println!("Total Duration:           {}ms", self.total_duration_ms);
        println!("Avg Duration/Test:        {:.2}ms",
            self.total_duration_ms as f64 / self.total_tests.max(1) as f64
        );
        println!("\nAST Validation:");
        println!("  Total Validations:      {}", self.ast_validations);
        println!("  Failures:               {}", self.ast_validation_failures);
        println!("  Success Rate:           {:.1}%",
            100.0 * (self.ast_validations - self.ast_validation_failures) as f64
            / self.ast_validations.max(1) as f64
        );
        println!("\nToken Efficiency:");
        println!("  Traditional:            {} tokens", self.total_tokens_traditional);
        println!("  Cortex:                 {} tokens", self.total_tokens_cortex);
        println!("  Savings:                {:.1}%", self.token_savings_percent());
        println!("{}", "=".repeat(80));
    }
}

/// Sample Rust code for testing
pub mod fixtures {
    pub const SIMPLE_RUST_FUNCTION: &str = r#"
pub fn foo() -> i32 {
    42
}
"#;

    pub const RUST_WITH_IMPORTS: &str = r#"
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub fn process_data(input: &str) -> HashMap<String, String> {
    HashMap::new()
}
"#;

    pub const RUST_STRUCT: &str = r#"
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    pub age: u32,
}

impl Person {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    pub fn is_adult(&self) -> bool {
        self.age >= 18
    }
}
"#;

    pub const TYPESCRIPT_CLASS: &str = r#"
export class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }
}
"#;

    pub const RUST_WITH_TRAIT: &str = r#"
pub trait Drawable {
    fn draw(&self);
}

pub struct Circle {
    radius: f64,
}
"#;

    pub fn rust_function_with_params(name: &str, params: &[(&str, &str)], return_type: &str, body: &str) -> String {
        let param_list = params
            .iter()
            .map(|(name, ty)| format!("{}: {}", name, ty))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "pub fn {}({}) -> {} {{\n{}\n}}",
            name, param_list, return_type, body
        )
    }
}
