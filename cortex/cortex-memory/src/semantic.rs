//! Semantic memory implementation for code understanding and relationships.
//!
//! Semantic memory stores facts, code structures, relationships, and type information
//! enabling the system to understand codebases at a deep semantic level.

use crate::types::*;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::types::{CodeUnit, CodeUnitType as CoreCodeUnitType, Language, Visibility, Parameter, Complexity as CoreComplexity, CodeUnitStatus};
use cortex_storage::ConnectionManager;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Semantic memory system for code understanding
pub struct SemanticMemorySystem {
    connection_manager: Arc<ConnectionManager>,
}

impl SemanticMemorySystem {
    /// Create a new semantic memory system
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    // ========================================================================
    // Code Unit Operations
    // ========================================================================

    /// Store a code unit using the new comprehensive schema
    pub async fn store_unit(&self, unit: &CodeUnit) -> Result<CortexId> {
        info!(unit_id = %unit.id, name = %unit.name, "Storing code unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Create unit - let SurrealDB assign record ID, store our ID in content
        let _: Option<CodeUnit> = conn
            .connection()
            .create("code_unit")
            .content(unit.clone())
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(unit.id)
    }

    /// Helper method to convert SurrealDB JSON to SemanticUnit
    fn json_to_semantic_unit(mut unit_json: serde_json::Value) -> Result<SemanticUnit> {
        // Restore the original id field from cortex_id
        if let Some(obj) = unit_json.as_object_mut() {
            if let Some(cortex_id) = obj.remove("cortex_id") {
                obj.insert("id".to_string(), cortex_id);
            }
        }

        serde_json::from_value(unit_json)
            .map_err(|e| CortexError::storage(format!("Failed to deserialize semantic unit: {}", e)))
    }

    /// Get the SELECT clause for querying semantic units (with enum field conversion)
    fn semantic_unit_select_clause() -> &'static str {
        "SELECT
            cortex_id,
            type::string(unit_type) as unit_type,
            name,
            qualified_name,
            display_name,
            file_path,
            start_line,
            start_column,
            end_line,
            end_column,
            signature,
            body,
            docstring,
            visibility,
            modifiers,
            parameters,
            return_type,
            summary,
            purpose,
            complexity.cyclomatic as cyclomatic,
            complexity.cognitive as cognitive,
            complexity.nesting as nesting,
            complexity.lines as lines,
            test_coverage,
            has_tests,
            has_documentation,
            embedding,
            created_at,
            updated_at"
    }

    /// Helper to reconstruct complexity from flat fields
    fn json_to_semantic_unit_with_complexity(mut unit_json: serde_json::Value) -> Result<SemanticUnit> {
        // Reconstruct complexity object if it's been flattened
        if let Some(obj) = unit_json.as_object_mut() {
            if obj.contains_key("cyclomatic") {
                let complexity = serde_json::json!({
                    "cyclomatic": obj.remove("cyclomatic").unwrap_or(serde_json::Value::Null),
                    "cognitive": obj.remove("cognitive").unwrap_or(serde_json::Value::Null),
                    "nesting": obj.remove("nesting").unwrap_or(serde_json::Value::Null),
                    "lines": obj.remove("lines").unwrap_or(serde_json::Value::Null),
                });
                obj.insert("complexity".to_string(), complexity);
            }
        }

        Self::json_to_semantic_unit(unit_json)
    }

    /// Store a semantic unit (legacy compatibility)
    pub async fn store_semantic_unit(&self, unit: &SemanticUnit) -> Result<CortexId> {
        info!(unit_id = %unit.id, name = %unit.name, "Storing semantic unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Serialize the unit to JSON and rename the id field to avoid SurrealDB record ID conflicts
        let mut unit_json = serde_json::to_value(unit.clone())
            .map_err(|e| CortexError::storage(format!("Failed to serialize semantic unit: {}", e)))?;

        // Rename 'id' to 'cortex_id' to avoid SurrealDB treating it as a record ID
        if let Some(obj) = unit_json.as_object_mut() {
            if let Some(id_val) = obj.remove("id") {
                obj.insert("cortex_id".to_string(), id_val);
            }
        }

        // Create semantic unit with the modified JSON
        let query = "CREATE semantic_unit CONTENT $data";
        conn
            .connection()
            .query(query)
            .bind(("data", unit_json))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store semantic unit: {}", e)))?;

        debug!(unit_id = %unit.id, "Semantic unit stored successfully");
        Ok(unit.id)
    }

    /// Convert legacy SemanticUnit to new CodeUnit
    fn convert_semantic_to_code_unit(&self, unit: &SemanticUnit) -> CodeUnit {
        CodeUnit {
            id: unit.id,
            unit_type: self.convert_unit_type(unit.unit_type),
            name: unit.name.clone(),
            qualified_name: unit.qualified_name.clone(),
            display_name: unit.display_name.clone(),
            file_path: unit.file_path.clone(),
            language: Language::Unknown, // Will need to infer from file_path
            start_line: unit.start_line as usize,
            end_line: unit.end_line as usize,
            start_column: unit.start_column as usize,
            end_column: unit.end_column as usize,
            start_byte: 0,
            end_byte: 0,
            signature: unit.signature.clone(),
            body: Some(unit.body.clone()),
            docstring: unit.docstring.clone(),
            comments: Vec::new(),
            return_type: unit.return_type.clone(),
            parameters: unit.parameters.iter().map(|p| Parameter {
                name: p.clone(),
                param_type: None,
                default_value: None,
                is_optional: false,
                is_variadic: false,
                attributes: Vec::new(),
            }).collect(),
            type_parameters: Vec::new(),
            generic_constraints: Vec::new(),
            throws: Vec::new(),
            visibility: self.convert_visibility(&unit.visibility),
            attributes: Vec::new(),
            modifiers: unit.modifiers.clone(),
            is_async: unit.modifiers.contains(&"async".to_string()),
            is_unsafe: unit.modifiers.contains(&"unsafe".to_string()),
            is_const: unit.modifiers.contains(&"const".to_string()),
            is_static: unit.modifiers.contains(&"static".to_string()),
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: false,
            is_default_export: false,
            complexity: CoreComplexity {
                cyclomatic: unit.complexity.cyclomatic,
                cognitive: unit.complexity.cognitive,
                nesting: unit.complexity.nesting,
                lines: unit.complexity.lines,
                parameters: 0,
                returns: 0,
            },
            test_coverage: unit.test_coverage.map(|c| c as f64),
            has_tests: unit.has_tests,
            has_documentation: unit.has_documentation,
            language_specific: HashMap::new(),
            embedding: unit.embedding.clone(),
            embedding_model: Some("text-embedding-3-small".to_string()),
            summary: Some(unit.summary.clone()),
            purpose: Some(unit.purpose.clone()),
            ast_node_type: None,
            ast_metadata: None,
            status: CodeUnitStatus::Active,
            version: 1,
            created_at: unit.created_at,
            updated_at: unit.updated_at,
            created_by: "system".to_string(),
            updated_by: "system".to_string(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    fn convert_unit_type(&self, unit_type: CodeUnitType) -> CoreCodeUnitType {
        match unit_type {
            CodeUnitType::Function => CoreCodeUnitType::Function,
            CodeUnitType::Method => CoreCodeUnitType::Method,
            CodeUnitType::AsyncFunction => CoreCodeUnitType::AsyncFunction,
            CodeUnitType::Generator => CoreCodeUnitType::Generator,
            CodeUnitType::Lambda => CoreCodeUnitType::Lambda,
            CodeUnitType::Class => CoreCodeUnitType::Class,
            CodeUnitType::Struct => CoreCodeUnitType::Struct,
            CodeUnitType::Enum => CoreCodeUnitType::Enum,
            CodeUnitType::Union => CoreCodeUnitType::Union,
            CodeUnitType::Interface => CoreCodeUnitType::Interface,
            CodeUnitType::Trait => CoreCodeUnitType::Trait,
            CodeUnitType::TypeAlias => CoreCodeUnitType::TypeAlias,
            CodeUnitType::Typedef => CoreCodeUnitType::Typedef,
            CodeUnitType::Const => CoreCodeUnitType::Const,
            CodeUnitType::Static => CoreCodeUnitType::Static,
            CodeUnitType::Variable => CoreCodeUnitType::Variable,
            CodeUnitType::Module => CoreCodeUnitType::Module,
            CodeUnitType::Namespace => CoreCodeUnitType::Namespace,
            CodeUnitType::Package => CoreCodeUnitType::Package,
            CodeUnitType::ImplBlock => CoreCodeUnitType::ImplBlock,
            CodeUnitType::Decorator => CoreCodeUnitType::Decorator,
            CodeUnitType::Macro => CoreCodeUnitType::Macro,
            CodeUnitType::Template => CoreCodeUnitType::Template,
            CodeUnitType::Test => CoreCodeUnitType::Test,
            CodeUnitType::Benchmark => CoreCodeUnitType::Benchmark,
            CodeUnitType::Example => CoreCodeUnitType::Example,
        }
    }

    fn convert_visibility(&self, visibility: &str) -> Visibility {
        match visibility.to_lowercase().as_str() {
            "public" => Visibility::Public,
            "private" => Visibility::Private,
            "protected" => Visibility::Protected,
            "internal" => Visibility::Internal,
            "package" => Visibility::Package,
            _ => Visibility::Private,
        }
    }

    /// Retrieve a code unit by ID
    pub async fn get_unit(&self, id: CortexId) -> Result<Option<CodeUnit>> {
        debug!(unit_id = %id, "Retrieving code unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Query by id field in content, not by record ID
        let query = "SELECT * FROM code_unit WHERE id = $id LIMIT 1";
        let mut result = conn
            .connection()
            .query(query)
            .bind(("id", id))
            .await
            .map_err(|e| CortexError::storage(format!("Query failed: {}", e)))?;

        let units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(format!("Failed to deserialize: {}", e)))?;
        Ok(units.into_iter().next())
    }

    /// Retrieve a semantic unit by ID (legacy compatibility)
    pub async fn get_semantic_unit(&self, id: CortexId) -> Result<Option<SemanticUnit>> {
        debug!(unit_id = %id, "Retrieving semantic unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Query by cortex_id field (we renamed id to cortex_id to avoid SurrealDB record ID conflicts)
        let query = format!("{} FROM semantic_unit WHERE cortex_id = $cortex_id LIMIT 1", Self::semantic_unit_select_clause());
        let mut result = conn
            .connection()
            .query(&query)
            .bind(("cortex_id", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Query failed: {}", e)))?;

        let units: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to deserialize: {}", e)))?;

        // Convert the JSON back to SemanticUnit, handling the cortex_id -> id conversion
        if let Some(unit_json) = units.into_iter().next() {
            Ok(Some(Self::json_to_semantic_unit_with_complexity(unit_json)?))
        } else {
            Ok(None)
        }
    }

    /// Convert new CodeUnit back to legacy SemanticUnit
    pub fn convert_code_to_semantic_unit(&self, unit: &CodeUnit) -> SemanticUnit {
        SemanticUnit {
            id: unit.id,
            unit_type: self.convert_core_unit_type(unit.unit_type),
            name: unit.name.clone(),
            qualified_name: unit.qualified_name.clone(),
            display_name: unit.display_name.clone(),
            file_path: unit.file_path.clone(),
            start_line: unit.start_line as u32,
            start_column: unit.start_column as u32,
            end_line: unit.end_line as u32,
            end_column: unit.end_column as u32,
            signature: unit.signature.clone(),
            body: unit.body.clone().unwrap_or_default(),
            docstring: unit.docstring.clone(),
            visibility: self.convert_core_visibility(unit.visibility),
            modifiers: unit.modifiers.clone(),
            parameters: unit.parameters.iter().map(|p| p.name.clone()).collect(),
            return_type: unit.return_type.clone(),
            summary: unit.summary.clone().unwrap_or_default(),
            purpose: unit.purpose.clone().unwrap_or_default(),
            complexity: ComplexityMetrics {
                cyclomatic: unit.complexity.cyclomatic,
                cognitive: unit.complexity.cognitive,
                nesting: unit.complexity.nesting,
                lines: unit.complexity.lines,
            },
            test_coverage: unit.test_coverage.map(|c| c as f32),
            has_tests: unit.has_tests,
            has_documentation: unit.has_documentation,
            embedding: unit.embedding.clone(),
            created_at: unit.created_at,
            updated_at: unit.updated_at,
        }
    }

    fn convert_core_unit_type(&self, unit_type: CoreCodeUnitType) -> CodeUnitType {
        match unit_type {
            CoreCodeUnitType::Function => CodeUnitType::Function,
            CoreCodeUnitType::Method => CodeUnitType::Method,
            CoreCodeUnitType::AsyncFunction => CodeUnitType::AsyncFunction,
            CoreCodeUnitType::Generator => CodeUnitType::Generator,
            CoreCodeUnitType::Lambda => CodeUnitType::Lambda,
            CoreCodeUnitType::Class => CodeUnitType::Class,
            CoreCodeUnitType::Struct => CodeUnitType::Struct,
            CoreCodeUnitType::Enum => CodeUnitType::Enum,
            CoreCodeUnitType::Union => CodeUnitType::Union,
            CoreCodeUnitType::Interface => CodeUnitType::Interface,
            CoreCodeUnitType::Trait => CodeUnitType::Trait,
            CoreCodeUnitType::TypeAlias => CodeUnitType::TypeAlias,
            CoreCodeUnitType::Typedef => CodeUnitType::Typedef,
            CoreCodeUnitType::Const => CodeUnitType::Const,
            CoreCodeUnitType::Static => CodeUnitType::Static,
            CoreCodeUnitType::Variable => CodeUnitType::Variable,
            CoreCodeUnitType::Module => CodeUnitType::Module,
            CoreCodeUnitType::Namespace => CodeUnitType::Namespace,
            CoreCodeUnitType::Package => CodeUnitType::Package,
            CoreCodeUnitType::ImplBlock => CodeUnitType::ImplBlock,
            CoreCodeUnitType::Decorator => CodeUnitType::Decorator,
            CoreCodeUnitType::Macro => CodeUnitType::Macro,
            CoreCodeUnitType::Template => CodeUnitType::Template,
            CoreCodeUnitType::Test => CodeUnitType::Test,
            CoreCodeUnitType::Benchmark => CodeUnitType::Benchmark,
            CoreCodeUnitType::Example => CodeUnitType::Example,
        }
    }

    fn convert_core_visibility(&self, visibility: Visibility) -> String {
        match visibility {
            Visibility::Public => "public".to_string(),
            Visibility::Private => "private".to_string(),
            Visibility::Protected => "protected".to_string(),
            Visibility::Internal => "internal".to_string(),
            Visibility::Package => "package".to_string(),
        }
    }

    /// Search for code units by semantic similarity
    pub async fn search_units(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<CodeUnit>>> {
        info!(query = %query.query_text, "Searching for similar code units");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let query_str = "
            SELECT *,
                   vector::distance::cosine(embedding, $query_embedding) AS similarity
            FROM code_unit
            WHERE embedding IS NOT NONE
              AND vector::distance::cosine(embedding, $query_embedding) <= $threshold
            ORDER BY similarity ASC
            LIMIT $limit
        ";

        let mut result = conn
            .connection()
            .query(query_str)
            .bind(("query_embedding", embedding.to_vec()))
            .bind(("threshold", 1.0 - query.similarity_threshold))
            .bind(("limit", query.limit))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<(CodeUnit, f32)> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        let results = units
            .into_iter()
            .map(|(unit, similarity)| MemorySearchResult {
                item: unit.clone(),
                similarity_score: 1.0 - similarity,
                relevance_score: self.calculate_code_unit_relevance(&unit),
            })
            .collect();

        Ok(results)
    }

    /// Get all units in a file
    pub async fn get_units_in_file(&self, file_path: &str) -> Result<Vec<CodeUnit>> {
        debug!(file_path, "Retrieving units in file");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE file_path = $path ORDER BY start_line ASC")
            .bind(("path", file_path.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Search units by qualified name (e.g., "module::Class::method")
    pub async fn find_by_qualified_name(&self, qualified_name: &str) -> Result<Option<CodeUnit>> {
        debug!(qualified_name, "Finding unit by qualified name");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE qualified_name = $name")
            .bind(("name", qualified_name.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let unit: Option<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(unit)
    }

    /// Search units by name (e.g., "VirtualFileSystem")
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<CodeUnit>> {
        debug!(name, "Finding units by name");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE name = $name")
            .bind(("name", name.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Get a connection for custom queries (primarily for testing)
    pub async fn get_connection(&self) -> Result<cortex_storage::PooledConnection> {
        self.connection_manager.acquire().await
    }

    // ========================================================================
    // Dependency Operations
    // ========================================================================

    /// Store a dependency relationship
    pub async fn store_dependency(&self, dependency: &Dependency) -> Result<CortexId> {
        debug!(dep_id = %dependency.id, "Storing dependency");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Serialize the dependency to JSON to ensure proper enum serialization
        let mut dep_json = serde_json::to_value(dependency.clone())
            .map_err(|e| CortexError::storage(format!("Failed to serialize dependency: {}", e)))?;

        // Rename 'id' to 'cortex_id' to avoid SurrealDB treating it as a record ID
        if let Some(obj) = dep_json.as_object_mut() {
            if let Some(id_val) = obj.remove("id") {
                obj.insert("cortex_id".to_string(), id_val);
            }
            // Add SurrealDB edge fields (in and out)
            obj.insert("in".to_string(), serde_json::json!(format!("code_unit:{}", dependency.source_id)));
            obj.insert("out".to_string(), serde_json::json!(format!("code_unit:{}", dependency.target_id)));
        }

        // Create the edge with the JSON content
        let query = "CREATE DEPENDS_ON CONTENT $data";
        conn
            .connection()
            .query(query)
            .bind(("data", dep_json))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store dependency: {}", e)))?;

        debug!(dep_id = %dependency.id, "Dependency stored successfully");
        Ok(dependency.id)
    }

    /// Get dependencies of a code unit (what it depends on)
    pub async fn get_dependencies(&self, unit_id: CortexId) -> Result<Vec<Dependency>> {
        debug!(unit_id = %unit_id, "Getting dependencies");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Use the RELATE statement syntax to query edges
        // Convert the enum field to string and use cortex_id for proper deserialization
        let query = "SELECT cortex_id, source_id, target_id, type::string(dependency_type) as dependency_type, is_direct, is_runtime, is_dev, metadata FROM DEPENDS_ON WHERE source_id = $unit_id";
        let mut result = conn
            .connection()
            .query(query)
            .bind(("unit_id", unit_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let deps_json: Vec<serde_json::Value> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        // Deserialize each dependency, restoring the id field from cortex_id
        let dependencies = deps_json.into_iter()
            .filter_map(|mut dep_json| {
                // Restore the original id field from cortex_id
                if let Some(obj) = dep_json.as_object_mut() {
                    if let Some(cortex_id) = obj.remove("cortex_id") {
                        obj.insert("id".to_string(), cortex_id);
                    }
                }
                serde_json::from_value::<Dependency>(dep_json).ok()
            })
            .collect();

        Ok(dependencies)
    }

    /// Get dependents of a code unit (what depends on it)
    pub async fn get_dependents(&self, unit_id: CortexId) -> Result<Vec<Dependency>> {
        debug!(unit_id = %unit_id, "Getting dependents");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Convert the enum field to string and use cortex_id for proper deserialization
        let query = "SELECT cortex_id, source_id, target_id, type::string(dependency_type) as dependency_type, is_direct, is_runtime, is_dev, metadata FROM DEPENDS_ON WHERE target_id = $unit_id";
        let mut result = conn
            .connection()
            .query(query)
            .bind(("unit_id", unit_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let deps_json: Vec<serde_json::Value> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        // Deserialize each dependency, restoring the id field from cortex_id
        let dependencies = deps_json.into_iter()
            .filter_map(|mut dep_json| {
                // Restore the original id field from cortex_id
                if let Some(obj) = dep_json.as_object_mut() {
                    if let Some(cortex_id) = obj.remove("cortex_id") {
                        obj.insert("id".to_string(), cortex_id);
                    }
                }
                serde_json::from_value::<Dependency>(dep_json).ok()
            })
            .collect();

        Ok(dependencies)
    }

    /// Get dependency graph for a set of units
    pub async fn get_dependency_graph(&self, unit_ids: &[CortexId]) -> Result<HashMap<CortexId, Vec<CortexId>>> {
        info!(unit_count = unit_ids.len(), "Building dependency graph");

        let mut graph: HashMap<CortexId, Vec<CortexId>> = HashMap::new();

        for unit_id in unit_ids {
            let deps = self.get_dependencies(*unit_id).await?;
            let targets: Vec<CortexId> = deps.into_iter().map(|d| d.target_id).collect();
            graph.insert(*unit_id, targets);
        }

        Ok(graph)
    }

    // ========================================================================
    // Analysis Operations
    // ========================================================================

    /// Calculate relevance score for a code unit
    fn calculate_code_unit_relevance(&self, unit: &CodeUnit) -> f32 {
        let mut score = 0.5; // Base score

        // Bonus for documentation
        if unit.has_documentation {
            score += 0.1;
        }

        // Bonus for tests
        if unit.has_tests {
            score += 0.1;
        }

        // Penalty for high complexity
        if unit.complexity.cyclomatic > 10 {
            score -= 0.1;
        }

        // Bonus for test coverage
        if let Some(coverage) = unit.test_coverage {
            score += (coverage as f32) * 0.2;
        }

        // Bonus for public visibility
        if unit.visibility == Visibility::Public {
            score += 0.05;
        }

        // Small bonus for type information
        if unit.return_type.is_some() && !unit.parameters.is_empty() {
            score += 0.05;
        }

        score.clamp(0.0, 1.0)
    }

    /// Calculate relevance score for a semantic unit (legacy)
    #[allow(dead_code)]
    fn calculate_unit_relevance(&self, unit: &SemanticUnit) -> f32 {
        let mut score = 0.5; // Base score

        // Bonus for documentation
        if unit.has_documentation {
            score += 0.1;
        }

        // Bonus for tests
        if unit.has_tests {
            score += 0.1;
        }

        // Penalty for high complexity
        if unit.complexity.cyclomatic > 10 {
            score -= 0.1;
        }

        // Bonus for test coverage
        if let Some(coverage) = unit.test_coverage {
            score += coverage * 0.2;
        }

        score.clamp(0.0, 1.0)
    }

    /// Analyze complexity metrics for a file
    pub async fn analyze_file_complexity(&self, file_path: &str) -> Result<ComplexityMetrics> {
        debug!(file_path, "Analyzing file complexity");

        let units = self.get_units_in_file(file_path).await?;

        if units.is_empty() {
            return Ok(ComplexityMetrics::default());
        }

        let total_cyclomatic: u32 = units.iter().map(|u| u.complexity.cyclomatic).sum();
        let total_cognitive: u32 = units.iter().map(|u| u.complexity.cognitive).sum();
        let max_nesting: u32 = units.iter().map(|u| u.complexity.nesting).max().unwrap_or(0);
        let total_lines: u32 = units.iter().map(|u| u.complexity.lines).sum();

        Ok(ComplexityMetrics {
            cyclomatic: total_cyclomatic / units.len() as u32,
            cognitive: total_cognitive / units.len() as u32,
            nesting: max_nesting,
            lines: total_lines,
        })
    }

    /// Find units with high complexity that need refactoring
    pub async fn find_complex_units(&self, complexity_threshold: u32) -> Result<Vec<CodeUnit>> {
        info!(threshold = complexity_threshold, "Finding complex units");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        // Query both code_unit and semantic_unit tables for compatibility
        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE complexity.cyclomatic > $threshold ORDER BY complexity.cyclomatic DESC")
            .bind(("threshold", complexity_threshold))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let mut units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        // Also check semantic_unit table (legacy) and convert to CodeUnit
        let query = format!("{} FROM semantic_unit WHERE complexity.cyclomatic > $threshold ORDER BY cyclomatic DESC", Self::semantic_unit_select_clause());
        let mut result2 = conn
            .connection()
            .query(&query)
            .bind(("threshold", complexity_threshold))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let semantic_units_json: Vec<serde_json::Value> = result2.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        for unit_json in semantic_units_json {
            if let Ok(semantic_unit) = Self::json_to_semantic_unit_with_complexity(unit_json) {
                units.push(self.convert_semantic_to_code_unit(&semantic_unit));
            }
        }

        Ok(units)
    }

    /// Find units without tests
    pub async fn find_untested_units(&self) -> Result<Vec<CodeUnit>> {
        info!("Finding untested code units");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE has_tests = false AND unit_type IN ['function', 'method', 'class']")
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Find units without documentation
    pub async fn find_undocumented_units(&self) -> Result<Vec<CodeUnit>> {
        info!("Finding undocumented code units");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE has_documentation = false AND visibility = 'public'")
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<CodeUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    // ========================================================================
    // Cross-reference Operations
    // ========================================================================

    /// Find all references to a code unit
    pub async fn find_references(&self, unit_id: CortexId) -> Result<Vec<CortexId>> {
        debug!(unit_id = %unit_id, "Finding references");

        let dependents = self.get_dependents(unit_id).await?;
        let references: Vec<CortexId> = dependents
            .into_iter()
            .filter(|d| matches!(d.dependency_type, DependencyType::Calls | DependencyType::Reads))
            .map(|d| d.source_id)
            .collect();

        Ok(references)
    }

    /// Find all definitions used by a code unit
    pub async fn find_definitions(&self, unit_id: CortexId) -> Result<Vec<CortexId>> {
        debug!(unit_id = %unit_id, "Finding definitions");

        let dependencies = self.get_dependencies(unit_id).await?;
        let definitions: Vec<CortexId> = dependencies
            .into_iter()
            .filter(|d| matches!(d.dependency_type, DependencyType::Imports | DependencyType::UsesType))
            .map(|d| d.target_id)
            .collect();

        Ok(definitions)
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get semantic memory statistics
    pub async fn get_statistics(&self) -> Result<SemanticStats> {
        debug!("Retrieving semantic memory statistics");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let query = "
            SELECT
                count() AS total_units,
                math::mean(complexity.cyclomatic) AS avg_complexity,
                count(has_tests = true) AS tested_units
            FROM code_unit
            GROUP ALL
        ";

        let mut result = conn
            .connection()
            .query(query)
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let stats: Option<serde_json::Value> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        // Get dependency count
        let mut dep_result = conn
            .connection()
            .query("SELECT count() AS total FROM DEPENDS_ON GROUP ALL")
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let dep_stats: Option<serde_json::Value> = dep_result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        if let Some(stats) = stats {
            let total_units = stats["total_units"].as_u64().unwrap_or(0);
            let tested_units = stats["tested_units"].as_u64().unwrap_or(0);
            let coverage = if total_units > 0 {
                (tested_units as f32 / total_units as f32) * 100.0
            } else {
                0.0
            };

            Ok(SemanticStats {
                total_units,
                total_dependencies: dep_stats
                    .and_then(|s| s["total"].as_u64())
                    .unwrap_or(0),
                average_complexity: stats["avg_complexity"].as_f64().unwrap_or(0.0),
                coverage_percentage: coverage,
            })
        } else {
            Ok(SemanticStats {
                total_units: 0,
                total_dependencies: 0,
                average_complexity: 0.0,
                coverage_percentage: 0.0,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig, RetryPolicy};
    use std::time::Duration;

    async fn create_test_memory() -> SemanticMemorySystem {
        // Use a temporary file-based database for tests to ensure persistence
        let temp_db = format!("file:/tmp/cortex_semantic_test_{}.db", CortexId::new());

        let config = DatabaseConfig {
            connection_mode: ConnectionMode::Local {
                endpoint: temp_db,
            },
            credentials: Credentials {
                username: None,
                password: None,
            },
            pool_config: PoolConfig {
                min_connections: 1,
                max_connections: 1, // Force single connection
                connection_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                retry_policy: RetryPolicy {
                    max_attempts: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    multiplier: 2.0,
                },
                warm_connections: true,
                validate_on_checkout: false,
                recycle_after_uses: None,
                shutdown_grace_period: Duration::from_secs(5),
            },
            namespace: "test".to_string(),
            database: "test".to_string(),
        };

        let manager = Arc::new(ConnectionManager::new(config).await.unwrap());
        SemanticMemorySystem::new(manager)
    }

    #[tokio::test]
    async fn test_store_and_retrieve_unit() {
        let memory = create_test_memory().await;

        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: "test_function".to_string(),
            qualified_name: "module::test_function".to_string(),
            display_name: "test_function".to_string(),
            file_path: "src/test.rs".to_string(),
            start_line: 10,
            start_column: 0,
            end_line: 20,
            end_column: 1,
            signature: "fn test_function() -> Result<()>".to_string(),
            body: "// function body".to_string(),
            docstring: Some("Test function".to_string()),
            visibility: "public".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: Some("Result<()>".to_string()),
            summary: "A test function".to_string(),
            purpose: "Testing".to_string(),
            complexity: ComplexityMetrics::default(),
            test_coverage: Some(0.8),
            has_tests: true,
            has_documentation: true,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let id = memory
            .store_semantic_unit(&unit)
            .await
            .expect("Failed to store unit");

        let retrieved = memory
            .get_semantic_unit(id)
            .await
            .expect("Failed to retrieve unit");

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "test_function");
    }

    #[tokio::test]
    async fn test_dependency_tracking() {
        let memory = create_test_memory().await;

        let source_id = CortexId::new();
        let target_id = CortexId::new();

        let dependency = Dependency {
            id: CortexId::new(),
            source_id,
            target_id,
            dependency_type: DependencyType::Calls,
            is_direct: true,
            is_runtime: true,
            is_dev: false,
            metadata: HashMap::new(),
        };

        memory
            .store_dependency(&dependency)
            .await
            .expect("Failed to store dependency");

        let deps = memory
            .get_dependencies(source_id)
            .await
            .expect("Failed to get dependencies");

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].target_id, target_id);
    }

    #[tokio::test]
    async fn test_complexity_analysis() {
        let memory = create_test_memory().await;

        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: "complex_function".to_string(),
            qualified_name: "module::complex_function".to_string(),
            display_name: "complex_function".to_string(),
            file_path: "src/complex.rs".to_string(),
            start_line: 10,
            start_column: 0,
            end_line: 100,
            end_column: 1,
            signature: "fn complex_function()".to_string(),
            body: "// complex body".to_string(),
            docstring: None,
            visibility: "private".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: "Complex function".to_string(),
            purpose: "Testing complexity".to_string(),
            complexity: ComplexityMetrics {
                cyclomatic: 15,
                cognitive: 20,
                nesting: 4,
                lines: 90,
            },
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        memory
            .store_semantic_unit(&unit)
            .await
            .expect("Failed to store unit");

        let complex_units = memory
            .find_complex_units(10)
            .await
            .expect("Failed to find complex units");

        assert_eq!(complex_units.len(), 1);
        assert_eq!(complex_units[0].name, "complex_function");
    }
}
