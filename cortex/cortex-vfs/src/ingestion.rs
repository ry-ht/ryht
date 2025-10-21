//! File ingestion pipeline for code parsing and semantic memory storage.
//!
//! This module implements the connection between the parser (cortex-parser),
//! virtual filesystem (cortex-vfs), and semantic memory (cortex-memory).
//!
//! # Architecture
//!
//! The ingestion pipeline follows these steps:
//! 1. Read file content from VFS
//! 2. Detect language from file extension
//! 3. Parse file with cortex-parser
//! 4. Convert parsed structures to CodeUnit types
//! 5. Store code units in semantic memory
//! 6. Update VNode metadata with units_count
//!
//! # Example
//!
//! ```no_run
//! use cortex_vfs::ingestion::FileIngestionPipeline;
//! use cortex_vfs::{VirtualFileSystem, VirtualPath};
//! use cortex_parser::CodeParser;
//! use cortex_memory::SemanticMemorySystem;
//! use cortex_storage::ConnectionManager;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = Arc::new(ConnectionManager::default());
//! let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
//! let parser = Arc::new(CodeParser::new()?);
//! let semantic_memory = Arc::new(SemanticMemorySystem::new(storage));
//!
//! let pipeline = FileIngestionPipeline::new(parser, vfs, semantic_memory);
//!
//! let workspace_id = uuid::Uuid::new_v4();
//! let path = VirtualPath::new("src/main.rs")?;
//!
//! // Ingest a file
//! let result = pipeline.ingest_file(&workspace_id, &path).await?;
//! println!("Ingested {} code units", result.units_stored);
//! # Ok(())
//! # }
//! ```

use crate::path::VirtualPath;
use crate::virtual_filesystem::VirtualFileSystem;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::types::{
    CodeUnit, CodeUnitType, Language, Visibility, Parameter, TypeParameter,
    Attribute, Complexity, CodeUnitStatus,
};
use cortex_memory::SemanticMemorySystem;
use cortex_parser::{CodeParser, FunctionInfo, StructInfo, EnumInfo, TraitInfo, ImplInfo};
use std::sync::Arc;
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use chrono::Utc;

/// Result of ingesting a single file.
#[derive(Debug, Clone)]
pub struct IngestionResult {
    /// File path that was ingested
    pub file_path: String,

    /// Number of code units extracted and stored
    pub units_stored: usize,

    /// IDs of the stored code units
    pub unit_ids: Vec<CortexId>,

    /// Language detected
    pub language: Language,

    /// Any errors encountered during ingestion
    pub errors: Vec<String>,

    /// Time taken in milliseconds
    pub duration_ms: u64,
}

/// Result of ingesting an entire workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceIngestionResult {
    /// Workspace ID
    pub workspace_id: Uuid,

    /// Number of files processed
    pub files_processed: usize,

    /// Total code units stored
    pub total_units: usize,

    /// Files that had errors
    pub files_with_errors: Vec<String>,

    /// Individual file results
    pub file_results: Vec<IngestionResult>,

    /// Time taken in milliseconds
    pub duration_ms: u64,
}

/// File ingestion pipeline connecting parser, VFS, and semantic memory.
pub struct FileIngestionPipeline {
    /// Code parser for extracting structure
    parser: Arc<tokio::sync::Mutex<CodeParser>>,

    /// Virtual filesystem for reading files
    vfs: Arc<VirtualFileSystem>,

    /// Semantic memory for storing code units
    semantic_memory: Arc<SemanticMemorySystem>,
}

impl FileIngestionPipeline {
    /// Create a new file ingestion pipeline.
    pub fn new(
        parser: Arc<tokio::sync::Mutex<CodeParser>>,
        vfs: Arc<VirtualFileSystem>,
        semantic_memory: Arc<SemanticMemorySystem>,
    ) -> Self {
        Self {
            parser,
            vfs,
            semantic_memory,
        }
    }

    /// Ingest a single file: parse and store code units.
    pub async fn ingest_file(
        &self,
        workspace_id: &Uuid,
        path: &VirtualPath,
    ) -> Result<IngestionResult> {
        let start = std::time::Instant::now();
        info!("Ingesting file: {} in workspace {}", path, workspace_id);

        let file_path = path.to_string();
        let mut errors = Vec::new();
        let mut unit_ids = Vec::new();

        // Read file content from VFS
        let content = self.vfs.read_file(workspace_id, path).await?;
        let content_str = String::from_utf8(content)
            .map_err(|e| CortexError::invalid_input(format!("Invalid UTF-8: {}", e)))?;

        // Detect language from extension
        let language = if let Some(ext) = path.extension() {
            Language::from_extension(ext)
        } else {
            Language::Unknown
        };

        // Skip non-code files
        if matches!(language, Language::Unknown) {
            debug!("Skipping non-code file: {}", path);
            return Ok(IngestionResult {
                file_path,
                units_stored: 0,
                unit_ids: vec![],
                language,
                errors: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        // Parse file with cortex-parser
        let parsed_file = {
            let mut parser = self.parser.lock().await;
            match parser.parse_file_auto(&file_path, &content_str) {
                Ok(parsed) => parsed,
                Err(e) => {
                    error!("Failed to parse file {}: {}", path, e);
                    errors.push(format!("Parse error: {}", e));
                    return Ok(IngestionResult {
                        file_path,
                        units_stored: 0,
                        unit_ids: vec![],
                        language,
                        errors,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        };

        // Convert and store functions
        for func in &parsed_file.functions {
            match self.convert_and_store_function(func, &file_path, language).await {
                Ok(id) => {
                    unit_ids.push(id);
                }
                Err(e) => {
                    warn!("Failed to store function {}: {}", func.name, e);
                    errors.push(format!("Failed to store function {}: {}", func.name, e));
                }
            }
        }

        // Convert and store structs
        for struct_info in &parsed_file.structs {
            match self.convert_and_store_struct(struct_info, &file_path, language).await {
                Ok(id) => {
                    unit_ids.push(id);
                }
                Err(e) => {
                    warn!("Failed to store struct {}: {}", struct_info.name, e);
                    errors.push(format!("Failed to store struct {}: {}", struct_info.name, e));
                }
            }
        }

        // Convert and store enums
        for enum_info in &parsed_file.enums {
            match self.convert_and_store_enum(enum_info, &file_path, language).await {
                Ok(id) => {
                    unit_ids.push(id);
                }
                Err(e) => {
                    warn!("Failed to store enum {}: {}", enum_info.name, e);
                    errors.push(format!("Failed to store enum {}: {}", enum_info.name, e));
                }
            }
        }

        // Convert and store traits
        for trait_info in &parsed_file.traits {
            match self.convert_and_store_trait(trait_info, &file_path, language).await {
                Ok(id) => {
                    unit_ids.push(id);
                }
                Err(e) => {
                    warn!("Failed to store trait {}: {}", trait_info.name, e);
                    errors.push(format!("Failed to store trait {}: {}", trait_info.name, e));
                }
            }
        }

        // Convert and store impl blocks
        for impl_info in &parsed_file.impls {
            match self.convert_and_store_impl(impl_info, &file_path, language).await {
                Ok(ids) => {
                    unit_ids.extend(ids);
                }
                Err(e) => {
                    warn!("Failed to store impl for {}: {}", impl_info.type_name, e);
                    errors.push(format!("Failed to store impl for {}: {}", impl_info.type_name, e));
                }
            }
        }

        let units_stored = unit_ids.len();

        // Update VNode metadata with units count
        if units_stored > 0 {
            if let Err(e) = self.vfs.update_file_units_count(workspace_id, path, units_stored).await {
                warn!("Failed to update file units count: {}", e);
                errors.push(format!("Failed to update metadata: {}", e));
            }
        }

        info!(
            "Ingested {} code units from {} in {}ms",
            units_stored,
            path,
            start.elapsed().as_millis()
        );

        Ok(IngestionResult {
            file_path,
            units_stored,
            unit_ids,
            language,
            errors,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Ingest all files in a workspace.
    pub async fn ingest_workspace(
        &self,
        workspace_id: &Uuid,
    ) -> Result<WorkspaceIngestionResult> {
        let start = std::time::Instant::now();
        info!("Ingesting workspace: {}", workspace_id);

        let mut file_results = Vec::new();
        let mut files_with_errors = Vec::new();
        let mut total_units = 0;

        // List all files in workspace
        let root = VirtualPath::root();
        let files = self.vfs.list_directory(workspace_id, &root, true).await?;

        // Filter to only process files (not directories)
        let code_files: Vec<_> = files.into_iter()
            .filter(|vnode| vnode.is_file())
            .collect();

        for vnode in &code_files {
            debug!("Processing file: {}", vnode.path);

            match self.ingest_file(workspace_id, &vnode.path).await {
                Ok(result) => {
                    total_units += result.units_stored;
                    if !result.errors.is_empty() {
                        files_with_errors.push(result.file_path.clone());
                    }
                    file_results.push(result);
                }
                Err(e) => {
                    error!("Failed to ingest file {}: {}", vnode.path, e);
                    files_with_errors.push(vnode.path.to_string());
                }
            }
        }

        let files_processed = code_files.len();
        info!(
            "Ingested workspace {} in {}ms: {} files, {} units",
            workspace_id,
            start.elapsed().as_millis(),
            files_processed,
            total_units
        );

        Ok(WorkspaceIngestionResult {
            workspace_id: *workspace_id,
            files_processed,
            total_units,
            files_with_errors,
            file_results,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    // ========================================================================
    // Conversion Methods: ParsedFile → CodeUnit
    // ========================================================================

    /// Convert and store a function.
    async fn convert_and_store_function(
        &self,
        func: &FunctionInfo,
        file_path: &str,
        language: Language,
    ) -> Result<CortexId> {
        let unit_type = if func.is_async {
            CodeUnitType::AsyncFunction
        } else {
            CodeUnitType::Function
        };

        let now = Utc::now();
        let mut code_unit = CodeUnit {
            id: CortexId::new(),
            unit_type,
            name: func.name.clone(),
            qualified_name: func.qualified_name.clone(),
            display_name: func.name.clone(),
            file_path: file_path.to_string(),
            language,

            // Location
            start_line: func.start_line,
            end_line: func.end_line,
            start_column: 0,
            end_column: 0,
            start_byte: 0,
            end_byte: 0,

            // Content
            signature: format!(
                "fn {}({}){}",
                func.name,
                func.parameters.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "),
                func.return_type.as_ref().map(|t| format!(" -> {}", t)).unwrap_or_default()
            ),
            body: Some(func.body.clone()),
            docstring: func.docstring.clone(),
            comments: vec![],

            // Type information
            return_type: func.return_type.clone(),
            parameters: func.parameters.iter().map(|p| Parameter {
                name: p.name.clone(),
                param_type: Some(p.param_type.clone()),
                default_value: p.default_value.clone(),
                is_optional: false,
                is_variadic: false,
                attributes: vec![],
            }).collect(),
            type_parameters: func.generics.iter().map(|g| TypeParameter {
                name: g.clone(),
                bounds: vec![],
                default_type: None,
                variance: None,
            }).collect(),
            generic_constraints: if let Some(wc) = &func.where_clause {
                vec![wc.clone()]
            } else {
                vec![]
            },
            throws: vec![],

            // Metadata
            visibility: Self::convert_visibility(func.visibility),
            attributes: func.attributes.iter().map(|attr| Attribute {
                name: attr.clone(),
                arguments: vec![],
                metadata: std::collections::HashMap::new(),
            }).collect(),
            modifiers: vec![],
            is_async: func.is_async,
            is_unsafe: func.is_unsafe,
            is_const: func.is_const,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: matches!(func.visibility, cortex_parser::types::Visibility::Public),
            is_default_export: false,

            // Metrics
            complexity: Complexity {
                cyclomatic: func.complexity.unwrap_or(1),
                cognitive: func.complexity.unwrap_or(1),
                nesting: 0,
                lines: (func.end_line - func.start_line) as u32,
                parameters: func.parameters.len() as u32,
                returns: if func.return_type.is_some() { 1 } else { 0 },
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: func.docstring.is_some(),

            // Language-specific
            language_specific: std::collections::HashMap::new(),

            // Embedding (to be generated later)
            embedding: None,
            embedding_model: None,

            // Semantic
            summary: None,
            purpose: None,

            // AST
            ast_node_type: Some("function".to_string()),
            ast_metadata: None,

            // Versioning
            status: CodeUnitStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: "ingestion_pipeline".to_string(),
            updated_by: "ingestion_pipeline".to_string(),

            // Tags
            tags: vec![],
            metadata: std::collections::HashMap::new(),
        };

        // Store in semantic memory
        self.semantic_memory.store_unit(&code_unit).await
    }

    /// Convert and store a struct.
    async fn convert_and_store_struct(
        &self,
        struct_info: &StructInfo,
        file_path: &str,
        language: Language,
    ) -> Result<CortexId> {
        let now = Utc::now();
        let code_unit = CodeUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Struct,
            name: struct_info.name.clone(),
            qualified_name: struct_info.qualified_name.clone(),
            display_name: struct_info.name.clone(),
            file_path: file_path.to_string(),
            language,

            start_line: struct_info.start_line,
            end_line: struct_info.end_line,
            start_column: 0,
            end_column: 0,
            start_byte: 0,
            end_byte: 0,

            signature: format!("struct {}", struct_info.name),
            body: None,
            docstring: struct_info.docstring.clone(),
            comments: vec![],

            return_type: None,
            parameters: vec![],
            type_parameters: struct_info.generics.iter().map(|g| TypeParameter {
                name: g.clone(),
                bounds: vec![],
                default_type: None,
                variance: None,
            }).collect(),
            generic_constraints: if let Some(wc) = &struct_info.where_clause {
                vec![wc.clone()]
            } else {
                vec![]
            },
            throws: vec![],

            visibility: Self::convert_visibility(struct_info.visibility),
            attributes: struct_info.attributes.iter().map(|attr| Attribute {
                name: attr.clone(),
                arguments: vec![],
                metadata: std::collections::HashMap::new(),
            }).collect(),
            modifiers: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: matches!(struct_info.visibility, cortex_parser::types::Visibility::Public),
            is_default_export: false,

            complexity: Complexity {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 0,
                lines: (struct_info.end_line - struct_info.start_line) as u32,
                parameters: 0,
                returns: 0,
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: struct_info.docstring.is_some(),

            language_specific: std::collections::HashMap::new(),
            embedding: None,
            embedding_model: None,
            summary: None,
            purpose: None,
            ast_node_type: Some("struct".to_string()),
            ast_metadata: None,

            status: CodeUnitStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: "ingestion_pipeline".to_string(),
            updated_by: "ingestion_pipeline".to_string(),

            tags: vec![],
            metadata: std::collections::HashMap::new(),
        };

        self.semantic_memory.store_unit(&code_unit).await
    }

    /// Convert and store an enum.
    async fn convert_and_store_enum(
        &self,
        enum_info: &EnumInfo,
        file_path: &str,
        language: Language,
    ) -> Result<CortexId> {
        let now = Utc::now();
        let code_unit = CodeUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Enum,
            name: enum_info.name.clone(),
            qualified_name: enum_info.qualified_name.clone(),
            display_name: enum_info.name.clone(),
            file_path: file_path.to_string(),
            language,

            start_line: enum_info.start_line,
            end_line: enum_info.end_line,
            start_column: 0,
            end_column: 0,
            start_byte: 0,
            end_byte: 0,

            signature: format!("enum {}", enum_info.name),
            body: None,
            docstring: enum_info.docstring.clone(),
            comments: vec![],

            return_type: None,
            parameters: vec![],
            type_parameters: enum_info.generics.iter().map(|g| TypeParameter {
                name: g.clone(),
                bounds: vec![],
                default_type: None,
                variance: None,
            }).collect(),
            generic_constraints: if let Some(wc) = &enum_info.where_clause {
                vec![wc.clone()]
            } else {
                vec![]
            },
            throws: vec![],

            visibility: Self::convert_visibility(enum_info.visibility),
            attributes: enum_info.attributes.iter().map(|attr| Attribute {
                name: attr.clone(),
                arguments: vec![],
                metadata: std::collections::HashMap::new(),
            }).collect(),
            modifiers: vec![],
            is_async: false,
            is_unsafe: false,
            is_const: false,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: matches!(enum_info.visibility, cortex_parser::types::Visibility::Public),
            is_default_export: false,

            complexity: Complexity {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 0,
                lines: (enum_info.end_line - enum_info.start_line) as u32,
                parameters: 0,
                returns: 0,
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: enum_info.docstring.is_some(),

            language_specific: std::collections::HashMap::new(),
            embedding: None,
            embedding_model: None,
            summary: None,
            purpose: None,
            ast_node_type: Some("enum".to_string()),
            ast_metadata: None,

            status: CodeUnitStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: "ingestion_pipeline".to_string(),
            updated_by: "ingestion_pipeline".to_string(),

            tags: vec![],
            metadata: std::collections::HashMap::new(),
        };

        self.semantic_memory.store_unit(&code_unit).await
    }

    /// Convert and store a trait.
    async fn convert_and_store_trait(
        &self,
        trait_info: &TraitInfo,
        file_path: &str,
        language: Language,
    ) -> Result<CortexId> {
        let now = Utc::now();
        let code_unit = CodeUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Trait,
            name: trait_info.name.clone(),
            qualified_name: trait_info.qualified_name.clone(),
            display_name: trait_info.name.clone(),
            file_path: file_path.to_string(),
            language,

            start_line: trait_info.start_line,
            end_line: trait_info.end_line,
            start_column: 0,
            end_column: 0,
            start_byte: 0,
            end_byte: 0,

            signature: format!("trait {}", trait_info.name),
            body: None,
            docstring: trait_info.docstring.clone(),
            comments: vec![],

            return_type: None,
            parameters: vec![],
            type_parameters: trait_info.generics.iter().map(|g| TypeParameter {
                name: g.clone(),
                bounds: vec![],
                default_type: None,
                variance: None,
            }).collect(),
            generic_constraints: if let Some(wc) = &trait_info.where_clause {
                vec![wc.clone()]
            } else {
                vec![]
            },
            throws: vec![],

            visibility: Self::convert_visibility(trait_info.visibility),
            attributes: trait_info.attributes.iter().map(|attr| Attribute {
                name: attr.clone(),
                arguments: vec![],
                metadata: std::collections::HashMap::new(),
            }).collect(),
            modifiers: vec![],
            is_async: false,
            is_unsafe: trait_info.is_unsafe,
            is_const: false,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: matches!(trait_info.visibility, cortex_parser::types::Visibility::Public),
            is_default_export: false,

            complexity: Complexity {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 0,
                lines: (trait_info.end_line - trait_info.start_line) as u32,
                parameters: 0,
                returns: 0,
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: trait_info.docstring.is_some(),

            language_specific: std::collections::HashMap::new(),
            embedding: None,
            embedding_model: None,
            summary: None,
            purpose: None,
            ast_node_type: Some("trait".to_string()),
            ast_metadata: None,

            status: CodeUnitStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: "ingestion_pipeline".to_string(),
            updated_by: "ingestion_pipeline".to_string(),

            tags: vec![],
            metadata: std::collections::HashMap::new(),
        };

        self.semantic_memory.store_unit(&code_unit).await
    }

    /// Convert and store an impl block (returns multiple units for methods).
    async fn convert_and_store_impl(
        &self,
        impl_info: &ImplInfo,
        file_path: &str,
        language: Language,
    ) -> Result<Vec<CortexId>> {
        let mut unit_ids = Vec::new();

        // Store methods from the impl block
        for method in &impl_info.methods {
            let qualified_name = if let Some(trait_name) = &impl_info.trait_name {
                format!("{}::{}::{}", impl_info.type_name, trait_name, method.name)
            } else {
                format!("{}::{}", impl_info.type_name, method.name)
            };

            let now = Utc::now();
            let code_unit = CodeUnit {
                id: CortexId::new(),
                unit_type: CodeUnitType::Method,
                name: method.name.clone(),
                qualified_name,
                display_name: method.name.clone(),
                file_path: file_path.to_string(),
                language,

                start_line: method.start_line,
                end_line: method.end_line,
                start_column: 0,
                end_column: 0,
                start_byte: 0,
                end_byte: 0,

                signature: format!(
                    "fn {}({}){}",
                    method.name,
                    method.parameters.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "),
                    method.return_type.as_ref().map(|t| format!(" -> {}", t)).unwrap_or_default()
                ),
                body: Some(method.body.clone()),
                docstring: method.docstring.clone(),
                comments: vec![],

                return_type: method.return_type.clone(),
                parameters: method.parameters.iter().map(|p| Parameter {
                    name: p.name.clone(),
                    param_type: Some(p.param_type.clone()),
                    default_value: p.default_value.clone(),
                    is_optional: false,
                    is_variadic: false,
                    attributes: vec![],
                }).collect(),
                type_parameters: method.generics.iter().map(|g| TypeParameter {
                    name: g.clone(),
                    bounds: vec![],
                    default_type: None,
                    variance: None,
                }).collect(),
                generic_constraints: if let Some(wc) = &method.where_clause {
                    vec![wc.clone()]
                } else {
                    vec![]
                },
                throws: vec![],

                visibility: Self::convert_visibility(method.visibility),
                attributes: method.attributes.iter().map(|attr| Attribute {
                    name: attr.clone(),
                    arguments: vec![],
                    metadata: std::collections::HashMap::new(),
                }).collect(),
                modifiers: vec![],
                is_async: method.is_async,
                is_unsafe: method.is_unsafe,
                is_const: method.is_const,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_final: false,
                is_exported: matches!(method.visibility, cortex_parser::types::Visibility::Public),
                is_default_export: false,

                complexity: Complexity {
                    cyclomatic: method.complexity.unwrap_or(1),
                    cognitive: method.complexity.unwrap_or(1),
                    nesting: 0,
                    lines: (method.end_line - method.start_line) as u32,
                    parameters: method.parameters.len() as u32,
                    returns: if method.return_type.is_some() { 1 } else { 0 },
                },
                test_coverage: None,
                has_tests: false,
                has_documentation: method.docstring.is_some(),

                language_specific: std::collections::HashMap::new(),
                embedding: None,
                embedding_model: None,
                summary: None,
                purpose: None,
                ast_node_type: Some("method".to_string()),
                ast_metadata: None,

                status: CodeUnitStatus::Active,
                version: 1,
                created_at: now,
                updated_at: now,
                created_by: "ingestion_pipeline".to_string(),
                updated_by: "ingestion_pipeline".to_string(),

                tags: vec![],
                metadata: std::collections::HashMap::new(),
            };

            let id = self.semantic_memory.store_unit(&code_unit).await?;
            unit_ids.push(id);
        }

        Ok(unit_ids)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Convert parser visibility to core visibility.
    fn convert_visibility(vis: cortex_parser::types::Visibility) -> Visibility {
        match vis {
            cortex_parser::types::Visibility::Public => Visibility::Public,
            cortex_parser::types::Visibility::PublicCrate => Visibility::Internal,
            cortex_parser::types::Visibility::PublicSuper => Visibility::Protected,
            cortex_parser::types::Visibility::PublicIn => Visibility::Package,
            cortex_parser::types::Visibility::Private => Visibility::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::ConnectionManager;
    use cortex_storage::connection::ConnectionConfig;

    async fn create_test_pipeline() -> (FileIngestionPipeline, Arc<VirtualFileSystem>, Uuid) {
        let config = ConnectionConfig::memory();
        let storage = Arc::new(ConnectionManager::new(config).await.unwrap());

        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let parser = Arc::new(tokio::sync::Mutex::new(CodeParser::new().unwrap()));
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage));

        let pipeline = FileIngestionPipeline::new(parser, vfs.clone(), semantic_memory);
        let workspace_id = Uuid::new_v4();

        (pipeline, vfs, workspace_id)
    }

    #[tokio::test]
    async fn test_ingest_simple_rust_file() {
        let (pipeline, vfs, workspace_id) = create_test_pipeline().await;

        // Write a simple Rust file
        let path = VirtualPath::new("src/lib.rs").unwrap();
        let content = r#"
/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A test struct.
pub struct Point {
    pub x: i32,
    pub y: i32,
}
"#;

        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .unwrap();

        // Ingest the file
        let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

        assert_eq!(result.language, Language::Rust);
        assert_eq!(result.units_stored, 2); // 1 function + 1 struct
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_ingest_with_methods() {
        let (pipeline, vfs, workspace_id) = create_test_pipeline().await;

        let path = VirtualPath::new("src/calculator.rs").unwrap();
        let content = r#"
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn add(&mut self, x: i32) {
        self.value += x;
    }
}
"#;

        vfs.write_file(&workspace_id, &path, content.as_bytes())
            .await
            .unwrap();

        let result = pipeline.ingest_file(&workspace_id, &path).await.unwrap();

        // Should have: 1 struct + 2 methods
        assert!(result.units_stored >= 2);
        assert!(result.errors.is_empty());
    }
}
