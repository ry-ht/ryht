//! Unit Tests for Architecture Analysis MCP Tools
//!
//! This module contains comprehensive unit tests for all architecture analysis MCP tools:
//! - cortex.arch.visualize
//! - cortex.arch.detect_patterns
//! - cortex.arch.suggest_boundaries
//! - cortex.arch.check_violations
//! - cortex.arch.analyze_drift
//!
//! Each test module covers:
//! - Basic operations
//! - Graph building and analysis
//! - Pattern detection
//! - Error handling
//! - Edge cases

mod test_visualize;
mod test_detect_patterns;
mod test_suggest_boundaries;
mod test_check_violations;
mod test_analyze_drift;

// Re-export test helpers
pub use test_helpers::*;

/// Common test helpers and fixtures
mod test_helpers {
    use cortex_mcp::tools::architecture_analysis::ArchitectureAnalysisContext;
    use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig};
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;

    /// Test fixture for architecture analysis tools testing
    pub struct ArchAnalysisTestFixture {
        pub storage: Arc<ConnectionManager>,
        pub workspace_id: Uuid,
        pub ctx: ArchitectureAnalysisContext,
    }

    impl ArchAnalysisTestFixture {
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
                database: "cortex_arch_test".to_string(),
            };

            let storage = Arc::new(
                ConnectionManager::new(database_config)
                    .await
                    .expect("Failed to create test storage"),
            );

            let workspace_id = Uuid::new_v4();
            let ctx = ArchitectureAnalysisContext::new(storage.clone());

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

        /// Create a test code unit
        pub async fn create_code_unit(
            &self,
            id: &str,
            qualified_name: &str,
            kind: &str,
            file_path: &str,
        ) -> Result<(), String> {
            let conn = self.storage.acquire().await
                .map_err(|e| format!("Failed to acquire connection: {}", e))?;

            let query = r#"
                CREATE code_unit CONTENT {
                    id: $id,
                    qualified_name: $qualified_name,
                    kind: $kind,
                    file_path: $file_path,
                    metadata: {},
                    workspace_id: $workspace_id
                }
            "#;

            conn.connection().query(query)
                .bind(("id", id))
                .bind(("qualified_name", qualified_name))
                .bind(("kind", kind))
                .bind(("file_path", file_path))
                .bind(("workspace_id", self.workspace_id.to_string()))
                .await
                .map_err(|e| format!("Failed to create unit: {}", e))?;

            Ok(())
        }

        /// Create a dependency between code units
        pub async fn create_dependency(
            &self,
            source_id: &str,
            target_id: &str,
            dependency_type: &str,
        ) -> Result<(), String> {
            let conn = self.storage.acquire().await
                .map_err(|e| format!("Failed to acquire connection: {}", e))?;

            let query = r#"
                RELATE (SELECT * FROM code_unit WHERE id = $source_id)->DEPENDS_ON->(SELECT * FROM code_unit WHERE id = $target_id)
                SET dependency_type = $dependency_type
            "#;

            conn.connection().query(query)
                .bind(("source_id", source_id))
                .bind(("target_id", target_id))
                .bind(("dependency_type", dependency_type))
                .await
                .map_err(|e| format!("Failed to create dependency: {}", e))?;

            Ok(())
        }

        /// Create a layered architecture for testing
        pub async fn create_layered_architecture(&self) -> Result<(), String> {
            // Create presentation layer
            self.create_code_unit("ui_controller", "app::ui::Controller", "class", "src/ui/controller.rs").await?;
            self.create_code_unit("ui_view", "app::ui::View", "class", "src/ui/view.rs").await?;

            // Create business layer
            self.create_code_unit("service_user", "app::service::UserService", "class", "src/service/user.rs").await?;
            self.create_code_unit("service_order", "app::service::OrderService", "class", "src/service/order.rs").await?;

            // Create data layer
            self.create_code_unit("repo_user", "app::repository::UserRepo", "class", "src/repository/user.rs").await?;
            self.create_code_unit("repo_order", "app::repository::OrderRepo", "class", "src/repository/order.rs").await?;

            // Create proper layer dependencies
            self.create_dependency("ui_controller", "service_user", "uses").await?;
            self.create_dependency("ui_controller", "service_order", "uses").await?;
            self.create_dependency("service_user", "repo_user", "uses").await?;
            self.create_dependency("service_order", "repo_order", "uses").await?;

            Ok(())
        }

        /// Create a circular dependency for testing
        pub async fn create_circular_dependency(&self) -> Result<(), String> {
            self.create_code_unit("module_a", "app::ModuleA", "module", "src/module_a.rs").await?;
            self.create_code_unit("module_b", "app::ModuleB", "module", "src/module_b.rs").await?;
            self.create_code_unit("module_c", "app::ModuleC", "module", "src/module_c.rs").await?;

            // Create cycle: A -> B -> C -> A
            self.create_dependency("module_a", "module_b", "imports").await?;
            self.create_dependency("module_b", "module_c", "imports").await?;
            self.create_dependency("module_c", "module_a", "imports").await?;

            Ok(())
        }

        /// Create a hub-and-spoke pattern
        pub async fn create_hub_and_spoke(&self) -> Result<(), String> {
            // Create hub
            self.create_code_unit("core_hub", "app::Core", "module", "src/core.rs").await?;

            // Create spokes
            for i in 1..=5 {
                let id = format!("spoke_{}", i);
                let name = format!("app::Spoke{}", i);
                let path = format!("src/spoke_{}.rs", i);
                self.create_code_unit(&id, &name, "module", &path).await?;

                // Connect spoke to hub
                self.create_dependency(&id, "core_hub", "uses").await?;
            }

            Ok(())
        }
    }
}
