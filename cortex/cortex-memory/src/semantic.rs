//! Semantic memory implementation for code understanding and relationships.
//!
//! Semantic memory stores facts, code structures, relationships, and type information
//! enabling the system to understand codebases at a deep semantic level.

use crate::types::*;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
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

    /// Store a semantic code unit
    pub async fn store_unit(&self, unit: &SemanticUnit) -> Result<CortexId> {
        info!(unit_id = %unit.id, name = %unit.name, "Storing semantic unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let query = "
            CREATE code_unit CONTENT {
                id: $id,
                unit_type: $unit_type,
                name: $name,
                qualified_name: $qualified_name,
                display_name: $display_name,
                file_path: $file_path,
                start_line: $start_line,
                start_column: $start_column,
                end_line: $end_line,
                end_column: $end_column,
                signature: $signature,
                body: $body,
                docstring: $docstring,
                visibility: $visibility,
                modifiers: $modifiers,
                parameters: $parameters,
                return_type: $return_type,
                summary: $summary,
                purpose: $purpose,
                complexity: $complexity,
                test_coverage: $test_coverage,
                has_tests: $has_tests,
                has_documentation: $has_documentation,
                embedding: $embedding,
                created_at: $created_at,
                updated_at: $updated_at
            }
        ";

        conn.connection().query(query)
            .bind(("id", unit.id.to_string()))
            .bind(("unit_type", unit.unit_type))
            .bind(("name", unit.name.clone()))
            .bind(("qualified_name", unit.qualified_name.clone()))
            .bind(("display_name", unit.display_name.clone()))
            .bind(("file_path", unit.file_path.clone()))
            .bind(("start_line", unit.start_line))
            .bind(("start_column", unit.start_column))
            .bind(("end_line", unit.end_line))
            .bind(("end_column", unit.end_column))
            .bind(("signature", unit.signature.clone()))
            .bind(("body", unit.body.clone()))
            .bind(("docstring", unit.docstring.clone()))
            .bind(("visibility", unit.visibility.clone()))
            .bind(("modifiers", unit.modifiers.clone()))
            .bind(("parameters", unit.parameters.clone()))
            .bind(("return_type", unit.return_type.clone()))
            .bind(("summary", unit.summary.clone()))
            .bind(("purpose", unit.purpose.clone()))
            .bind(("complexity", unit.complexity.clone()))
            .bind(("test_coverage", unit.test_coverage))
            .bind(("has_tests", unit.has_tests))
            .bind(("has_documentation", unit.has_documentation))
            .bind(("embedding", unit.embedding.clone()))
            .bind(("created_at", unit.created_at))
            .bind(("updated_at", unit.updated_at))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(unit.id)
    }

    /// Retrieve a code unit by ID
    pub async fn get_unit(&self, id: CortexId) -> Result<Option<SemanticUnit>> {
        debug!(unit_id = %id, "Retrieving semantic unit");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE id = $id")
            .bind(("id", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let unit: Option<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(unit)
    }

    /// Search for code units by semantic similarity
    pub async fn search_units(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<SemanticUnit>>> {
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

        let units: Vec<(SemanticUnit, f32)> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;

        let results = units
            .into_iter()
            .map(|(unit, similarity)| MemorySearchResult {
                item: unit.clone(),
                similarity_score: 1.0 - similarity,
                relevance_score: self.calculate_unit_relevance(&unit),
            })
            .collect();

        Ok(results)
    }

    /// Get all units in a file
    pub async fn get_units_in_file(&self, file_path: &str) -> Result<Vec<SemanticUnit>> {
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

        let units: Vec<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Search units by qualified name (e.g., "module::Class::method")
    pub async fn find_by_qualified_name(&self, qualified_name: &str) -> Result<Option<SemanticUnit>> {
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

        let unit: Option<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(unit)
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

        let query = "
            CREATE DEPENDS_ON CONTENT {
                id: $id,
                in: $source_id,
                out: $target_id,
                dependency_type: $dependency_type,
                is_direct: $is_direct,
                is_runtime: $is_runtime,
                is_dev: $is_dev,
                metadata: $metadata
            }
        ";

        conn.connection().query(query)
            .bind(("id", dependency.id.to_string()))
            .bind(("source_id", format!("code_unit:{}", dependency.source_id)))
            .bind(("target_id", format!("code_unit:{}", dependency.target_id)))
            .bind(("dependency_type", dependency.dependency_type))
            .bind(("is_direct", dependency.is_direct))
            .bind(("is_runtime", dependency.is_runtime))
            .bind(("is_dev", dependency.is_dev))
            .bind(("metadata", dependency.metadata.clone()))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        Ok(dependency.id)
    }

    /// Get dependencies of a code unit (what it depends on)
    pub async fn get_dependencies(&self, unit_id: CortexId) -> Result<Vec<Dependency>> {
        debug!(unit_id = %unit_id, "Getting dependencies");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM DEPENDS_ON WHERE in = $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let deps: Vec<Dependency> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(deps)
    }

    /// Get dependents of a code unit (what depends on it)
    pub async fn get_dependents(&self, unit_id: CortexId) -> Result<Vec<Dependency>> {
        debug!(unit_id = %unit_id, "Getting dependents");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM DEPENDS_ON WHERE out = $unit_id")
            .bind(("unit_id", format!("code_unit:{}", unit_id)))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let deps: Vec<Dependency> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(deps)
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

    /// Calculate relevance score for a semantic unit
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
    pub async fn find_complex_units(&self, complexity_threshold: u32) -> Result<Vec<SemanticUnit>> {
        info!(threshold = complexity_threshold, "Finding complex units");

        let conn = self
            .connection_manager
            .acquire()
            .await?;

        let mut result = conn
            .connection()
            .query("SELECT * FROM code_unit WHERE complexity.cyclomatic > $threshold ORDER BY complexity.cyclomatic DESC")
            .bind(("threshold", complexity_threshold))
            .await
            .map_err(|e| CortexError::storage(e.to_string()))?;

        let units: Vec<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Find units without tests
    pub async fn find_untested_units(&self) -> Result<Vec<SemanticUnit>> {
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

        let units: Vec<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
        Ok(units)
    }

    /// Find units without documentation
    pub async fn find_undocumented_units(&self) -> Result<Vec<SemanticUnit>> {
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

        let units: Vec<SemanticUnit> = result.take(0).map_err(|e| CortexError::storage(e.to_string()))?;
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
    use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, PoolConfig, ConnectionMode, Credentials};

    async fn create_test_memory() -> SemanticMemorySystem {
        let config = ConnectionConfig::memory();
        let pool_config = PoolConfig::default();
        let manager = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager"),
        );
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
            .store_unit(&unit)
            .await
            .expect("Failed to store unit");

        let retrieved = memory
            .get_unit(id)
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

        let mut unit = SemanticUnit {
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
            .store_unit(&unit)
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
