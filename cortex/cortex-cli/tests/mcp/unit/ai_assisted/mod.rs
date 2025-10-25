//! Unit Tests for AI-Assisted Development MCP Tools
//!
//! This module contains comprehensive unit tests for all AI-assisted MCP tools:
//! - cortex.ai.suggest_refactoring
//! - cortex.ai.explain_code
//! - cortex.ai.suggest_optimization
//! - cortex.ai.suggest_fix
//! - cortex.ai.generate_docstring
//! - cortex.ai.review_code
//!
//! Each test module covers:
//! - Basic operations
//! - Edge cases
//! - Error handling
//! - Confidence scoring
//! - Multiple suggestion scenarios

mod test_suggest_refactoring;
mod test_explain_code;
mod test_suggest_optimization;
mod test_suggest_fix;
mod test_generate_docstring;
mod test_review_code;

// Re-export test helpers
pub use test_helpers::*;

/// Common test helpers and fixtures
mod test_helpers {
    use cortex_mcp::tools::ai_assisted::AiAssistedContext;
    use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig};
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

    /// Test fixture for AI-assisted tools testing
    pub struct AiAssistedTestFixture {
        pub storage: Arc<ConnectionManager>,
        pub workspace_id: Uuid,
        pub ctx: AiAssistedContext,
    }

    impl AiAssistedTestFixture {
        /// Create a new test fixture with in-memory database
        pub async fn new() -> Self {
            use cortex_storage::connection_pool::{ConnectionMode, PoolConfig};

            let database_config = DatabaseConfig {
                connection_mode: ConnectionMode::Local {
                    endpoint: "mem://".to_string(),
                },
                credentials: Credentials {
                    username: Some("root".to_string()),
                    password: Some("root".to_string()),
                },
                pool_config: PoolConfig::default(),
                namespace: format!("test_{}", Uuid::new_v4().to_string().replace("-", "")),
                database: "cortex_ai_test".to_string(),
            };

            let storage = Arc::new(
                ConnectionManager::new(database_config)
                    .await
                    .expect("Failed to create test storage"),
            );

            let workspace_id = Uuid::new_v4();
            let ctx = AiAssistedContext::new(storage.clone());

            Self {
                storage,
                workspace_id,
                ctx,
            }
        }

        /// Helper to execute a tool and measure performance
        pub async fn execute_tool(
            &self,
            tool: &dyn Tool,
            input: serde_json::Value,
        ) -> (Result<ToolResult, ToolError>, u128) {
            let start = Instant::now();
            let result = tool.execute(input, &ToolContext::default()).await;
            let duration = start.elapsed().as_millis();
            (result, duration)
        }

        /// Create a test code unit for analysis
        pub async fn create_test_unit(&self, name: &str, body: &str, file_path: &str, start_line: usize) -> Result<String, String> {
            let conn = self.storage.acquire().await
                .map_err(|e| format!("Failed to acquire connection: {}", e))?;

            let unit_id = Uuid::new_v4().to_string();

            let query = r#"
                CREATE code_unit CONTENT {
                    id: $id,
                    name: $name,
                    qualified_name: $qualified_name,
                    unit_type: "function",
                    file_path: $file_path,
                    start_line: $start_line,
                    end_line: $end_line,
                    body: $body,
                    signature: $signature,
                    language: "rust",
                    workspace_id: $workspace_id,
                    visibility: "public"
                }
            "#;

            let end_line = start_line + body.lines().count();
            let signature = format!("fn {}()", name);

            conn.connection().query(query)
                .bind(("id", unit_id.clone()))
                .bind(("name", name))
                .bind(("qualified_name", name))
                .bind(("file_path", file_path))
                .bind(("start_line", start_line))
                .bind(("end_line", end_line))
                .bind(("body", body))
                .bind(("signature", signature))
                .bind(("workspace_id", self.workspace_id.to_string()))
                .await
                .map_err(|e| format!("Failed to create unit: {}", e))?;

            Ok(unit_id)
        }
    }

    /// Sample code fixtures for testing
    pub mod fixtures {
        /// Long function (> 50 lines) for refactoring suggestions
        pub const LONG_FUNCTION: &str = r#"
fn process_data() {
    // Line 1
    let mut data = Vec::new();
    // Line 3
    for i in 0..100 {
        data.push(i);
    }
    // More lines...
    // Line 10
    println!("Processing");
    // Line 12
    let result = data.iter().sum::<i32>();
    // Line 14
    println!("Result: {}", result);
    // Line 16
    // ... (imagine 50+ more lines)
}"#;

        /// Function with nested loops for optimization
        pub const NESTED_LOOPS: &str = r#"
fn find_pairs(items: &[i32], others: &[i32]) -> Vec<(i32, i32)> {
    let mut pairs = Vec::new();
    for item in items {
        for other in others {
            if item == other {
                pairs.push((*item, *other));
            }
        }
    }
    pairs
}
"#;

        /// Function with excessive cloning
        pub const EXCESSIVE_CLONES: &str = r#"
fn process_strings(data: Vec<String>) -> Vec<String> {
    let copy1 = data.clone();
    let copy2 = data.clone();
    let copy3 = data.clone();
    let copy4 = data.clone();
    let copy5 = data.clone();
    let copy6 = data.clone();

    let mut result = Vec::new();
    result.extend(copy1.clone());
    result.extend(copy2.clone());
    result
}
"#;

        /// Code with complex conditionals
        pub const COMPLEX_CONDITIONALS: &str = r#"
fn validate(a: bool, b: bool, c: bool, d: bool) -> bool {
    if a && b || c && d {
        if a || b && c || d {
            if a && (b || c) && d {
                if a || (b && c) || d {
                    if a && b && c && d {
                        if a || b || c || d {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}
"#;

        /// Function for explanation
        pub const DOCUMENTED_FUNCTION: &str = r#"
fn calculate_fibonacci(n: u32) -> u32 {
    if n <= 1 {
        return n;
    }
    calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2)
}
"#;

        /// Code with error for fix suggestions
        pub const ERROR_CONTEXT: &str = r#"
fn use_value() {
    let data = String::from("test");
    process(data);
    println!("{}", data); // Error: value used after move
}
"#;

        /// Function needing documentation
        pub const UNDOCUMENTED_FUNCTION: &str = r#"
fn process_user_data(user_id: i32, data: &str) -> Result<String, String> {
    if user_id < 0 {
        return Err("Invalid user ID".to_string());
    }

    let processed = data.to_uppercase();
    Ok(processed)
}
"#;

        /// Code with quality issues for review
        pub const QUALITY_ISSUES: &str = r#"
fn bad_code(x: i32) -> i32 {
    let y = x.clone(); // unnecessary clone on Copy type
    let z = y.clone();

    if x > 0 {
        if x > 10 {
            if x > 100 {
                return z.unwrap(); // unwrap on non-Option/Result
            }
        }
    }

    0
}
"#;

        /// Function with security issues
        pub const SECURITY_ISSUES: &str = r#"
fn unsafe_operation(ptr: *mut i32) {
    unsafe {
        *ptr = 42; // Unsafe block without documentation
    }

    let value = vec![1, 2, 3];
    let result = value.get(10).unwrap(); // Can panic
    println!("{}", result);
}
"#;
    }
}
