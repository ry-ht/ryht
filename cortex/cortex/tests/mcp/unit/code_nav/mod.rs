//! Unit Tests for Code Navigation MCP Tools
//!
//! This module contains comprehensive unit tests for all code navigation MCP tools:
//! - cortex.code.find_definition
//! - cortex.code.find_references
//! - cortex.code.get_symbols
//! - cortex.code.get_call_hierarchy
//! - cortex.code.get_type_hierarchy
//! - cortex.code.get_signature
//!
//! Each test module covers:
//! - Basic operations with sample code
//! - Cross-file references
//! - Multiple language support (Rust, TypeScript)
//! - Error handling
//! - Performance measurements
//! - Edge cases

mod test_find_definition;
mod test_find_references;
mod test_get_symbols;
mod test_call_hierarchy;
mod test_type_hierarchy;
mod test_get_signature;

// Re-export test helpers
pub use test_helpers::*;

/// Common test helpers and fixtures for code navigation tests
mod test_helpers {
    use cortex_cli::mcp::tools::code_nav::CodeNavContext;
    use cortex_core::id::CortexId;
    use cortex_core::types::{
        CodeUnit, CodeUnitType, Language, Visibility, Parameter, Complexity, CodeUnitStatus,
    };
    use cortex_memory::CognitiveManager;
    use cortex_memory::types::{Dependency, DependencyType};
    use cortex_storage::{ConnectionManager, connection::ConnectionConfig};
    use cortex_vfs::VirtualFileSystem;
    use mcp_sdk::prelude::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use uuid::Uuid;
    use chrono::Utc;

    /// Test fixture for code navigation testing
    pub struct CodeNavTestFixture {
        pub storage: Arc<ConnectionManager>,
        pub vfs: Arc<VirtualFileSystem>,
        pub ctx: CodeNavContext,
        pub cognitive_manager: CognitiveManager,
    }

    impl CodeNavTestFixture {
        /// Create a new test fixture with in-memory database
        pub async fn new() -> Self {
            let config = ConnectionConfig::memory();
            let storage = Arc::new(
                ConnectionManager::new(config)
                    .await
                    .expect("Failed to create test storage"),
            );

            let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
            let ctx = CodeNavContext::new(storage.clone());
            let cognitive_manager = CognitiveManager::new(storage.clone());

            Self {
                storage,
                vfs,
                ctx,
                cognitive_manager,
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

        /// Store a code unit in semantic memory
        pub async fn store_unit(&self, unit: &CodeUnit) -> Result<CortexId, String> {
            let semantic = self.cognitive_manager.semantic();
            semantic
                .store_unit(unit)
                .await
                .map_err(|e| format!("Failed to store unit: {}", e))
        }

        /// Store a dependency between code units
        pub async fn store_dependency(&self, dep: &Dependency) -> Result<(), String> {
            let semantic = self.cognitive_manager.semantic();
            semantic
                .store_dependency(dep)
                .await
                .map_err(|e| format!("Failed to store dependency: {}", e))
        }

        /// Get semantic memory instance
        pub fn semantic(&self) -> cortex_memory::SemanticMemorySystem {
            self.cognitive_manager.semantic()
        }
    }

    /// Sample code units and fixtures for testing
    pub mod fixtures {
        use super::*;

        /// Create a sample Rust function
        pub fn create_rust_function(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
        ) -> CodeUnit {
            let now = Utc::now();
            CodeUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Function,
                name: name.to_string(),
                qualified_name: qualified_name.to_string(),
                display_name: name.to_string(),
                file_path: file_path.to_string(),
                language: Language::Rust,
                start_line,
                end_line: start_line + 10,
                start_column: 0,
                end_column: 40,
                start_byte: 0,
                end_byte: 200,
                signature: format!("pub fn {}() -> Result<(), Error>", name),
                body: Some(format!("    println!(\"Function: {}\");\n    Ok(())", name)),
                docstring: Some(format!("/// {} function documentation", name)),
                comments: Vec::new(),
                return_type: Some("Result<(), Error>".to_string()),
                parameters: vec![],
                type_parameters: Vec::new(),
                generic_constraints: Vec::new(),
                throws: Vec::new(),
                visibility: Visibility::Public,
                attributes: Vec::new(),
                modifiers: vec!["pub".to_string()],
                is_async: false,
                is_unsafe: false,
                is_const: false,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_final: false,
                is_exported: true,
                is_default_export: false,
                complexity: Complexity {
                    cyclomatic: 1,
                    cognitive: 1,
                    nesting: 0,
                    lines: 10,
                    parameters: 0,
                    returns: 1,
                },
                test_coverage: Some(85.0),
                has_tests: true,
                has_documentation: true,
                language_specific: HashMap::new(),
                embedding: None,
                embedding_model: None,
                summary: Some(format!("Function: {}", name)),
                purpose: Some("Test function".to_string()),
                ast_node_type: None,
                ast_metadata: None,
                status: CodeUnitStatus::Active,
                version: 1,
                created_at: now,
                updated_at: now,
                created_by: "test".to_string(),
                updated_by: "test".to_string(),
                tags: Vec::new(),
                metadata: HashMap::new(),
            }
        }

        /// Create a sample Rust function with parameters
        pub fn create_rust_function_with_params(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
            params: Vec<Parameter>,
        ) -> CodeUnit {
            let mut unit = create_rust_function(name, qualified_name, file_path, start_line);

            let param_strs: Vec<String> = params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.param_type.as_ref().unwrap_or(&"_".to_string())))
                .collect();

            unit.signature = format!("pub fn {}({}) -> Result<(), Error>", name, param_strs.join(", "));
            unit.parameters = params;
            unit.complexity.parameters = unit.parameters.len() as u32;
            unit
        }

        /// Create a sample Rust struct
        pub fn create_rust_struct(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
        ) -> CodeUnit {
            let now = Utc::now();
            CodeUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Struct,
                name: name.to_string(),
                qualified_name: qualified_name.to_string(),
                display_name: name.to_string(),
                file_path: file_path.to_string(),
                language: Language::Rust,
                start_line,
                end_line: start_line + 15,
                start_column: 0,
                end_column: 1,
                start_byte: 0,
                end_byte: 300,
                signature: format!("pub struct {} {{ ... }}", name),
                body: Some(format!("pub struct {} {{\n    id: u64,\n    name: String,\n}}", name)),
                docstring: Some(format!("/// {} struct", name)),
                comments: Vec::new(),
                return_type: None,
                parameters: Vec::new(),
                type_parameters: Vec::new(),
                generic_constraints: Vec::new(),
                throws: Vec::new(),
                visibility: Visibility::Public,
                attributes: vec!["#[derive(Debug, Clone)]".to_string()],
                modifiers: vec!["pub".to_string()],
                is_async: false,
                is_unsafe: false,
                is_const: false,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_final: false,
                is_exported: true,
                is_default_export: false,
                complexity: Complexity {
                    cyclomatic: 1,
                    cognitive: 1,
                    nesting: 0,
                    lines: 15,
                    parameters: 0,
                    returns: 0,
                },
                test_coverage: None,
                has_tests: false,
                has_documentation: true,
                language_specific: HashMap::new(),
                embedding: None,
                embedding_model: None,
                summary: Some(format!("Struct: {}", name)),
                purpose: Some("Data structure".to_string()),
                ast_node_type: None,
                ast_metadata: None,
                status: CodeUnitStatus::Active,
                version: 1,
                created_at: now,
                updated_at: now,
                created_by: "test".to_string(),
                updated_by: "test".to_string(),
                tags: Vec::new(),
                metadata: HashMap::new(),
            }
        }

        /// Create a sample Rust trait
        pub fn create_rust_trait(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
        ) -> CodeUnit {
            let mut unit = create_rust_struct(name, qualified_name, file_path, start_line);
            unit.unit_type = CodeUnitType::Trait;
            unit.signature = format!("pub trait {} {{ ... }}", name);
            unit.body = Some(format!("pub trait {} {{\n    fn process(&self);\n}}", name));
            unit
        }

        /// Create a sample TypeScript class
        pub fn create_typescript_class(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
        ) -> CodeUnit {
            let now = Utc::now();
            CodeUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Class,
                name: name.to_string(),
                qualified_name: qualified_name.to_string(),
                display_name: name.to_string(),
                file_path: file_path.to_string(),
                language: Language::TypeScript,
                start_line,
                end_line: start_line + 20,
                start_column: 0,
                end_column: 1,
                start_byte: 0,
                end_byte: 400,
                signature: format!("export class {} {{ ... }}", name),
                body: Some(format!("export class {} {{\n  constructor() {{}}\n}}", name)),
                docstring: Some(format!("/** {} class */", name)),
                comments: Vec::new(),
                return_type: None,
                parameters: Vec::new(),
                type_parameters: Vec::new(),
                generic_constraints: Vec::new(),
                throws: Vec::new(),
                visibility: Visibility::Public,
                attributes: Vec::new(),
                modifiers: vec!["export".to_string()],
                is_async: false,
                is_unsafe: false,
                is_const: false,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_final: false,
                is_exported: true,
                is_default_export: false,
                complexity: Complexity {
                    cyclomatic: 1,
                    cognitive: 1,
                    nesting: 0,
                    lines: 20,
                    parameters: 0,
                    returns: 0,
                },
                test_coverage: None,
                has_tests: true,
                has_documentation: true,
                language_specific: HashMap::new(),
                embedding: None,
                embedding_model: None,
                summary: Some(format!("Class: {}", name)),
                purpose: Some("TypeScript class".to_string()),
                ast_node_type: None,
                ast_metadata: None,
                status: CodeUnitStatus::Active,
                version: 1,
                created_at: now,
                updated_at: now,
                created_by: "test".to_string(),
                updated_by: "test".to_string(),
                tags: Vec::new(),
                metadata: HashMap::new(),
            }
        }

        /// Create a sample TypeScript method
        pub fn create_typescript_method(
            name: &str,
            qualified_name: &str,
            file_path: &str,
            start_line: usize,
        ) -> CodeUnit {
            let mut unit = create_rust_function(name, qualified_name, file_path, start_line);
            unit.unit_type = CodeUnitType::Method;
            unit.language = Language::TypeScript;
            unit.signature = format!("public {}(): Promise<void>", name);
            unit.return_type = Some("Promise<void>".to_string());
            unit.modifiers = vec!["public".to_string()];
            unit
        }

        /// Create a call dependency (A calls B)
        pub fn create_call_dependency(source_id: CortexId, target_id: CortexId) -> Dependency {
            let now = Utc::now();
            Dependency {
                id: CortexId::new(),
                source_id,
                target_id,
                dependency_type: DependencyType::Calls,
                is_direct: true,
                is_runtime: true,
                is_optional: false,
                confidence: 1.0,
                context: None,
                metadata: HashMap::new(),
                created_at: now,
                updated_at: now,
            }
        }

        /// Create an extends dependency (A extends B)
        pub fn create_extends_dependency(source_id: CortexId, target_id: CortexId) -> Dependency {
            let now = Utc::now();
            Dependency {
                id: CortexId::new(),
                source_id,
                target_id,
                dependency_type: DependencyType::Extends,
                is_direct: true,
                is_runtime: true,
                is_optional: false,
                confidence: 1.0,
                context: None,
                metadata: HashMap::new(),
                created_at: now,
                updated_at: now,
            }
        }

        /// Create an implements dependency (A implements B)
        pub fn create_implements_dependency(source_id: CortexId, target_id: CortexId) -> Dependency {
            let now = Utc::now();
            Dependency {
                id: CortexId::new(),
                source_id,
                target_id,
                dependency_type: DependencyType::Implements,
                is_direct: true,
                is_runtime: true,
                is_optional: false,
                confidence: 1.0,
                context: None,
                metadata: HashMap::new(),
                created_at: now,
                updated_at: now,
            }
        }

        /// Sample Rust code
        pub const RUST_CODE: &str = r#"
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn create_user(id: u64, name: String) -> User {
    User::new(id, name)
}
"#;

        /// Sample TypeScript code
        pub const TYPESCRIPT_CODE: &str = r#"
export class User {
    constructor(
        public id: number,
        public name: string
    ) {}

    getName(): string {
        return this.name;
    }
}

export function createUser(id: number, name: string): User {
    return new User(id, name);
}
"#;
    }
}
