//! Code Unit service layer
//!
//! Provides unified code unit operations for both API and MCP modules.
//! Eliminates duplication between API routes and MCP tools.

use anyhow::Result;
use chrono::{DateTime, Utc};
use cortex_core::types::CodeUnit;
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Code unit service for managing code units
#[derive(Clone)]
pub struct CodeUnitService {
    storage: Arc<ConnectionManager>,
}

impl CodeUnitService {
    /// Create a new code unit service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// List code units in a workspace with filters
    pub async fn list_code_units(
        &self,
        workspace_id: Uuid,
        unit_type: Option<String>,
        language: Option<String>,
        visibility: Option<String>,
        complexity_min: Option<i32>,
        limit: usize,
    ) -> Result<Vec<CodeUnitDetails>> {
        debug!(
            "Listing code units for workspace: {} with limit: {}",
            workspace_id, limit
        );

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Build query with filters
        let limit = limit.min(1000);
        let mut query = format!(
            "SELECT * FROM code_unit WHERE file_path CONTAINS '{}'",
            workspace_id
        );

        // Apply filters
        if let Some(unit_type) = &unit_type {
            query.push_str(&format!(" AND unit_type = '{}'", unit_type));
        }
        if let Some(visibility) = &visibility {
            query.push_str(&format!(" AND visibility = '{}'", visibility));
        }
        if let Some(language) = &language {
            query.push_str(&format!(" AND language = '{}'", language));
        }
        if let Some(min_complexity) = complexity_min {
            query.push_str(&format!(
                " AND complexity.cyclomatic >= {}",
                min_complexity
            ));
        }

        query.push_str(&format!(" LIMIT {}", limit));

        // Execute query
        let mut result = conn.query(&query).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        info!("Found {} code units", units.len());

        // Convert to details
        let details = units
            .into_iter()
            .map(CodeUnitDetails::from_code_unit)
            .collect();

        Ok(details)
    }

    /// Get a specific code unit by ID
    pub async fn get_code_unit(&self, unit_id: &str) -> Result<CodeUnitDetails> {
        debug!("Getting code unit: {}", unit_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
        let mut result = conn.query(&query).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        let unit = units
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Code unit not found: {}", unit_id))?;

        Ok(CodeUnitDetails::from_code_unit(unit))
    }

    /// Search code units with query and filters
    pub async fn search_code_units(
        &self,
        workspace_id: Uuid,
        query: String,
        filters: CodeUnitFilters,
    ) -> Result<Vec<CodeUnitDetails>> {
        debug!(
            "Searching code units in workspace: {} with query: {}",
            workspace_id, query
        );

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Build search query
        let mut sql = format!(
            "SELECT * FROM code_unit WHERE file_path CONTAINS '{}' AND (name CONTAINS '{}' OR qualified_name CONTAINS '{}')",
            workspace_id, query, query
        );

        // Apply filters
        if let Some(unit_type) = &filters.unit_type {
            sql.push_str(&format!(" AND unit_type = '{}'", unit_type));
        }
        if let Some(language) = &filters.language {
            sql.push_str(&format!(" AND language = '{}'", language));
        }
        if let Some(visibility) = &filters.visibility {
            sql.push_str(&format!(" AND visibility = '{}'", visibility));
        }
        if filters.has_tests {
            sql.push_str(" AND has_tests = true");
        }
        if filters.has_documentation {
            sql.push_str(" AND has_documentation = true");
        }

        let limit = filters.limit.unwrap_or(50).min(1000);
        sql.push_str(&format!(" LIMIT {}", limit));

        // Execute search
        let mut result = conn.query(&sql).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        info!("Found {} matching code units", units.len());

        // Convert to details
        let details = units
            .into_iter()
            .map(CodeUnitDetails::from_code_unit)
            .collect();

        Ok(details)
    }

    /// Get complexity metrics for a code unit
    pub async fn get_complexity_metrics(&self, unit_id: &str) -> Result<ComplexityMetrics> {
        debug!("Getting complexity metrics for unit: {}", unit_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let query = format!("SELECT complexity FROM code_unit WHERE id = '{}'", unit_id);
        let mut result = conn.query(&query).await?;
        let complexities: Vec<serde_json::Value> = result.take(0)?;

        let complexity_value = complexities
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Code unit not found: {}", unit_id))?;

        let cyclomatic = complexity_value["cyclomatic"]
            .as_u64()
            .unwrap_or(0) as u32;
        let cognitive = complexity_value["cognitive"]
            .as_u64()
            .unwrap_or(0) as u32;
        let nesting = complexity_value["nesting"]
            .as_u64()
            .unwrap_or(0) as u32;
        let lines = complexity_value["lines"]
            .as_u64()
            .unwrap_or(0) as u32;

        // Calculate score (higher is more complex)
        let score = (cyclomatic as f64 * 0.4)
            + (cognitive as f64 * 0.3)
            + (nesting as f64 * 0.2)
            + (lines as f64 * 0.1);

        Ok(ComplexityMetrics {
            cyclomatic,
            cognitive,
            nesting,
            lines,
            score,
        })
    }

    /// Batch get multiple code units by IDs
    pub async fn batch_get_units(&self, unit_ids: Vec<String>) -> Result<Vec<CodeUnitDetails>> {
        if unit_ids.is_empty() {
            return Ok(vec![]);
        }

        debug!("Batch getting {} code units", unit_ids.len());

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Build IN clause
        let ids_str = unit_ids
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!("SELECT * FROM code_unit WHERE id IN ({})", ids_str);
        let mut result = conn.query(&query).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        info!("Retrieved {} of {} requested code units", units.len(), unit_ids.len());

        // Convert to details
        let details = units
            .into_iter()
            .map(CodeUnitDetails::from_code_unit)
            .collect();

        Ok(details)
    }

    /// Update code unit body and docstring
    pub async fn update_code_unit(
        &self,
        unit_id: &str,
        body: Option<String>,
        docstring: Option<String>,
        expected_version: Option<u32>,
    ) -> Result<CodeUnitDetails> {
        debug!("Updating code unit: {}", unit_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // First, get the existing unit
        let query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
        let mut result = conn.query(&query).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        let mut unit = units
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Code unit not found: {}", unit_id))?;

        // Check version if provided
        if let Some(expected_version) = expected_version {
            if unit.version != expected_version {
                anyhow::bail!(
                    "Version mismatch: expected {}, found {}",
                    expected_version,
                    unit.version
                );
            }
        }

        // Update fields
        if let Some(body) = body {
            unit.body = Some(body);
        }
        if let Some(docstring) = docstring {
            unit.docstring = Some(docstring);
            unit.has_documentation = true;
        }

        // Increment version and update timestamp
        unit.version += 1;
        unit.updated_at = Utc::now();

        // Save to database
        let update_query = format!(
            "UPDATE code_unit:{} SET body = $body, docstring = $docstring, version = $version, updated_at = $updated_at, has_documentation = $has_documentation",
            unit_id
        );

        conn.query(&update_query)
            .bind(("body", unit.body.clone()))
            .bind(("docstring", unit.docstring.clone()))
            .bind(("version", unit.version))
            .bind(("updated_at", unit.updated_at))
            .bind(("has_documentation", unit.has_documentation))
            .await?;

        info!("Updated code unit: {} to version {}", unit_id, unit.version);

        Ok(CodeUnitDetails::from_code_unit(unit))
    }

    /// Get code units by file path
    pub async fn get_units_by_file(&self, file_path: &str) -> Result<Vec<CodeUnitDetails>> {
        debug!("Getting code units for file: {}", file_path);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let query = format!("SELECT * FROM code_unit WHERE file_path = '{}'", file_path);
        let mut result = conn.query(&query).await?;
        let units: Vec<CodeUnit> = result.take(0)?;

        info!("Found {} code units in file", units.len());

        let details = units
            .into_iter()
            .map(CodeUnitDetails::from_code_unit)
            .collect();

        Ok(details)
    }

    /// Count code units in workspace
    pub async fn count_units(&self, workspace_id: Uuid, filters: CodeUnitFilters) -> Result<usize> {
        debug!("Counting code units in workspace: {}", workspace_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let mut query = format!(
            "SELECT count() FROM code_unit WHERE file_path CONTAINS '{}'",
            workspace_id
        );

        // Apply filters
        if let Some(unit_type) = &filters.unit_type {
            query.push_str(&format!(" AND unit_type = '{}'", unit_type));
        }
        if let Some(language) = &filters.language {
            query.push_str(&format!(" AND language = '{}'", language));
        }
        if let Some(visibility) = &filters.visibility {
            query.push_str(&format!(" AND visibility = '{}'", visibility));
        }

        query.push_str(" GROUP ALL");

        let mut result = conn.query(&query).await?;
        let count: usize = result
            .take::<Option<usize>>(0)?
            .unwrap_or(0);

        Ok(count)
    }
}

// ============================================================================
// Types
// ============================================================================

/// Code unit details response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnitDetails {
    pub id: String,
    pub unit_type: String,
    pub name: String,
    pub qualified_name: String,
    pub display_name: String,
    pub file_path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub signature: String,
    pub body: Option<String>,
    pub docstring: Option<String>,
    pub visibility: String,
    pub is_async: bool,
    pub is_exported: bool,
    pub complexity: ComplexityMetrics,
    pub has_tests: bool,
    pub has_documentation: bool,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CodeUnitDetails {
    /// Convert from CodeUnit
    pub fn from_code_unit(unit: CodeUnit) -> Self {
        let complexity_score = unit.complexity_score();
        Self {
            id: unit.id.to_string(),
            unit_type: format!("{:?}", unit.unit_type).to_lowercase(),
            name: unit.name,
            qualified_name: unit.qualified_name,
            display_name: unit.display_name,
            file_path: unit.file_path,
            language: format!("{:?}", unit.language).to_lowercase(),
            start_line: unit.start_line,
            end_line: unit.end_line,
            start_column: unit.start_column,
            end_column: unit.end_column,
            signature: unit.signature,
            body: unit.body,
            docstring: unit.docstring,
            visibility: format!("{:?}", unit.visibility).to_lowercase(),
            is_async: unit.is_async,
            is_exported: unit.is_exported,
            complexity: ComplexityMetrics {
                cyclomatic: unit.complexity.cyclomatic,
                cognitive: unit.complexity.cognitive,
                nesting: unit.complexity.nesting,
                lines: unit.complexity.lines,
                score: complexity_score,
            },
            has_tests: unit.has_tests,
            has_documentation: unit.has_documentation,
            version: unit.version,
            created_at: unit.created_at,
            updated_at: unit.updated_at,
        }
    }
}

/// Complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub lines: u32,
    pub score: f64,
}

/// Filters for code unit queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeUnitFilters {
    pub unit_type: Option<String>,
    pub language: Option<String>,
    pub visibility: Option<String>,
    pub has_tests: bool,
    pub has_documentation: bool,
    pub limit: Option<usize>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_unit_details_serialization() {
        let details = CodeUnitDetails {
            id: "test123".to_string(),
            unit_type: "function".to_string(),
            name: "test_function".to_string(),
            qualified_name: "module::test_function".to_string(),
            display_name: "test_function".to_string(),
            file_path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            start_line: 1,
            end_line: 10,
            start_column: 0,
            end_column: 0,
            signature: "fn test_function()".to_string(),
            body: Some("{ println!(\"test\"); }".to_string()),
            docstring: Some("Test function".to_string()),
            visibility: "public".to_string(),
            is_async: false,
            is_exported: true,
            complexity: ComplexityMetrics {
                cyclomatic: 1,
                cognitive: 0,
                nesting: 0,
                lines: 10,
                score: 0.4,
            },
            has_tests: true,
            has_documentation: true,
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("test_function"));

        let deserialized: CodeUnitDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test_function");
    }

    #[test]
    fn test_complexity_metrics() {
        let metrics = ComplexityMetrics {
            cyclomatic: 10,
            cognitive: 5,
            nesting: 3,
            lines: 100,
            score: 15.3,
        };

        assert_eq!(metrics.cyclomatic, 10);
        assert_eq!(metrics.score, 15.3);
    }

    #[test]
    fn test_code_unit_filters() {
        let filters = CodeUnitFilters {
            unit_type: Some("function".to_string()),
            language: Some("rust".to_string()),
            visibility: None,
            has_tests: true,
            has_documentation: false,
            limit: Some(50),
        };

        assert_eq!(filters.unit_type, Some("function".to_string()));
        assert!(filters.has_tests);
        assert!(!filters.has_documentation);
    }
}
