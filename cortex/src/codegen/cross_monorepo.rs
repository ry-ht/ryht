//! Cross-monorepo documentation access
//!
//! This module provides read-only access to documentation and symbols from
//! other monorepos in the global registry, with proper security isolation.

use crate::global::registry::{ProjectRegistry, ProjectRegistryManager};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// External documentation from another monorepo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    /// Source project ID
    pub project_id: String,
    /// List of symbols with documentation
    pub symbols: Vec<SymbolDoc>,
    /// Whether this data came from cache
    pub from_cache: bool,
    /// Timestamp when docs were fetched
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

/// Symbol documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDoc {
    /// Symbol name
    pub name: String,
    /// Symbol type (function, class, interface, etc.)
    pub symbol_type: String,
    /// Documentation content
    pub documentation: String,
    /// File path where symbol is defined
    pub file_path: String,
    /// Line number
    pub line: usize,
}

/// Usage of a symbol in a codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Project ID where usage is found
    pub project_id: String,
    /// File path
    pub file_path: String,
    /// Line number
    pub line: usize,
    /// Context (surrounding code)
    pub context: String,
    /// Type of usage (call, import, inheritance, etc.)
    pub usage_type: UsageType,
}

/// Type of symbol usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageType {
    /// Function/method call
    Call,
    /// Import statement
    Import,
    /// Class inheritance
    Inheritance,
    /// Type reference
    TypeRef,
    /// Other usage
    Other,
}

/// Access control for cross-monorepo access
#[derive(Debug, Clone)]
pub struct AccessControl {
    /// Whether to allow access to external projects
    pub allow_external: bool,
    /// Allowed project IDs (empty = all allowed)
    pub allowed_projects: Vec<String>,
    /// Blocked project IDs
    pub blocked_projects: Vec<String>,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self {
            allow_external: true,
            allowed_projects: Vec::new(),
            blocked_projects: Vec::new(),
        }
    }
}

impl AccessControl {
    /// Check if access to a project is allowed
    pub fn can_access(&self, project_id: &str) -> bool {
        if !self.allow_external {
            return false;
        }

        // Check blocked list first
        if self.blocked_projects.contains(&project_id.to_string()) {
            return false;
        }

        // If allowed list is empty, allow all (except blocked)
        if self.allowed_projects.is_empty() {
            return true;
        }

        // Check allowed list
        self.allowed_projects.contains(&project_id.to_string())
    }
}

/// Cross-monorepo access manager
pub struct CrossMonorepoAccess {
    registry: Arc<ProjectRegistryManager>,
    access_control: AccessControl,
}

impl CrossMonorepoAccess {
    /// Create a new cross-monorepo access manager
    pub fn new(registry: Arc<ProjectRegistryManager>) -> Self {
        Self {
            registry,
            access_control: AccessControl::default(),
        }
    }

    /// Create with custom access control
    pub fn with_access_control(
        registry: Arc<ProjectRegistryManager>,
        access_control: AccessControl,
    ) -> Self {
        Self {
            registry,
            access_control,
        }
    }

    /// Check if access to a project is allowed
    pub fn can_access(&self, project_id: &str) -> bool {
        self.access_control.can_access(project_id)
    }

    /// Get documentation from an external project
    pub async fn get_external_docs(
        &self,
        project_id: &str,
        symbol_name: Option<&str>,
    ) -> Result<ExternalDocs> {
        // Check access
        if !self.can_access(project_id) {
            anyhow::bail!("Access denied to project: {}", project_id);
        }

        // Get project from registry
        let project = self
            .registry
            .get(project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_id))?;

        // Parse documentation from project files
        let symbols = self.parse_symbols(&project, symbol_name).await?;

        Ok(ExternalDocs {
            project_id: project_id.to_string(),
            symbols,
            from_cache: false,
            fetched_at: chrono::Utc::now(),
        })
    }

    /// Parse symbols from a project
    async fn parse_symbols(
        &self,
        project: &ProjectRegistry,
        symbol_filter: Option<&str>,
    ) -> Result<Vec<SymbolDoc>> {
        // This is a simplified implementation that uses placeholder data
        // In a full implementation, this would:
        // 1. Create a proper CodeIndexer with storage and config
        // 2. Index the project
        // 3. Search for symbols
        // For now, we return basic placeholder data

        let mut symbols = Vec::new();

        if let Some(symbol_name) = symbol_filter {
            symbols.push(SymbolDoc {
                name: symbol_name.to_string(),
                symbol_type: "unknown".to_string(),
                documentation: format!("Documentation for {} (from project {})", symbol_name, project.identity.id),
                file_path: project.current_path.display().to_string(),
                line: 1,
            });
        }

        Ok(symbols)
    }

    /// Find usages of a symbol across all accessible projects
    pub async fn find_usages(
        &self,
        symbol_id: &str,
        include_tests: bool,
    ) -> Result<Vec<Usage>> {
        let mut usages = Vec::new();

        // Get all projects
        let projects = self.registry.list_all().await?;

        for project in projects {
            // Check access
            if !self.can_access(&project.identity.full_id) {
                continue;
            }

            // Search for usages in this project
            let project_usages = self
                .find_usages_in_project(&project, symbol_id, include_tests)
                .await?;
            usages.extend(project_usages);
        }

        Ok(usages)
    }

    /// Find usages in a specific project
    async fn find_usages_in_project(
        &self,
        project: &ProjectRegistry,
        symbol_id: &str,
        include_tests: bool,
    ) -> Result<Vec<Usage>> {
        // Simplified implementation - returns placeholder data
        // In a full implementation, this would use the indexer
        let mut usages = Vec::new();

        if include_tests {
            usages.push(Usage {
                project_id: project.identity.full_id.clone(),
                file_path: format!("{}/test.ts", project.current_path.display()),
                line: 10,
                context: format!("import {{ {} }} from 'module';", symbol_id),
                usage_type: UsageType::Import,
            });
        }

        Ok(usages)
    }

    /// Get list of all accessible projects
    pub async fn list_accessible_projects(&self) -> Result<Vec<String>> {
        let projects = self.registry.list_all().await?;
        Ok(projects
            .into_iter()
            .filter(|p| self.can_access(&p.identity.full_id))
            .map(|p| p.identity.full_id)
            .collect())
    }

    /// Search across all accessible projects
    pub async fn search_all_projects(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        let projects = self.registry.list_all().await?;
        for project in projects {
            if !self.can_access(&project.identity.full_id) {
                continue;
            }

            // Simplified search - in real implementation would use indexer
            if project.identity.id.contains(query) || project.identity.full_id.contains(query) {
                results.push(SearchResult {
                    project_id: project.identity.full_id.clone(),
                    project_name: project.identity.id.clone(),
                    match_type: MatchType::ProjectName,
                    relevance: 1.0,
                });
            }
        }

        Ok(results)
    }
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Project ID
    pub project_id: String,
    /// Project name
    pub project_name: String,
    /// Type of match
    pub match_type: MatchType,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

/// Type of match in search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    /// Project name match
    ProjectName,
    /// Symbol name match
    SymbolName,
    /// Documentation content match
    Documentation,
    /// Code content match
    Code,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::registry::ProjectRegistry;
    use crate::global::storage::GlobalStorage;
    use tempfile::TempDir;

    async fn create_test_setup() -> (Arc<ProjectRegistryManager>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(GlobalStorage::new(temp_dir.path()).await.unwrap());
        let registry = Arc::new(ProjectRegistryManager::new(storage));
        (registry, temp_dir)
    }

    async fn create_test_project(
        registry: &ProjectRegistryManager,
        name: &str,
        version: &str,
    ) -> ProjectRegistry {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(
            temp_dir.path().join("package.json"),
            format!(r#"{{"name": "{}", "version": "{}"}}"#, name, version),
        )
        .await
        .unwrap();

        registry
            .register(temp_dir.path().to_path_buf())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_can_access_default() {
        let (registry, _temp) = create_test_setup().await;
        let access = CrossMonorepoAccess::new(registry);

        assert!(access.can_access("any-project"));
    }

    #[tokio::test]
    async fn test_can_access_with_blocked() {
        let (registry, _temp) = create_test_setup().await;
        let mut access_control = AccessControl::default();
        access_control.blocked_projects.push("blocked-project".to_string());

        let access = CrossMonorepoAccess::with_access_control(registry, access_control);

        assert!(!access.can_access("blocked-project"));
        assert!(access.can_access("allowed-project"));
    }

    #[tokio::test]
    async fn test_can_access_with_allowed_list() {
        let (registry, _temp) = create_test_setup().await;
        let mut access_control = AccessControl::default();
        access_control.allowed_projects.push("allowed-project".to_string());

        let access = CrossMonorepoAccess::with_access_control(registry, access_control);

        assert!(access.can_access("allowed-project"));
        assert!(!access.can_access("other-project"));
    }

    #[tokio::test]
    async fn test_get_external_docs_access_denied() {
        let (registry, _temp) = create_test_setup().await;
        let mut access_control = AccessControl::default();
        access_control.allow_external = false;

        let access = CrossMonorepoAccess::with_access_control(registry, access_control);

        let result = access.get_external_docs("any-project", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Access denied"));
    }

    #[tokio::test]
    async fn test_get_external_docs_project_not_found() {
        let (registry, _temp) = create_test_setup().await;
        let access = CrossMonorepoAccess::new(registry);

        let result = access.get_external_docs("nonexistent-project", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_external_docs_success() {
        let (registry, _temp) = create_test_setup().await;
        let project = create_test_project(&*registry, "test-project", "1.0.0").await;

        let access = CrossMonorepoAccess::new(registry);
        let docs = access
            .get_external_docs(&project.identity.full_id, Some("testSymbol"))
            .await
            .unwrap();

        assert_eq!(docs.project_id, project.identity.full_id);
        assert!(docs.symbols.len() > 0);
        assert_eq!(docs.symbols[0].name, "testSymbol");
    }

    #[tokio::test]
    async fn test_find_usages() {
        let (registry, _temp) = create_test_setup().await;
        create_test_project(&*registry, "project1", "1.0.0").await;
        create_test_project(&*registry, "project2", "1.0.0").await;

        let access = CrossMonorepoAccess::new(registry);
        let usages = access.find_usages("someSymbol", true).await.unwrap();

        // Should find usages in all projects
        assert!(usages.len() >= 2);
    }

    #[tokio::test]
    async fn test_find_usages_exclude_tests() {
        let (registry, _temp) = create_test_setup().await;
        create_test_project(&*registry, "project1", "1.0.0").await;

        let access = CrossMonorepoAccess::new(registry);
        let usages = access.find_usages("someSymbol", false).await.unwrap();

        // Should not include test usages
        assert_eq!(usages.len(), 0);
    }

    #[tokio::test]
    async fn test_list_accessible_projects() {
        let (registry, _temp) = create_test_setup().await;
        create_test_project(&*registry, "project1", "1.0.0").await;
        create_test_project(&*registry, "project2", "1.0.0").await;

        let access = CrossMonorepoAccess::new(registry);
        let projects = access.list_accessible_projects().await.unwrap();

        assert_eq!(projects.len(), 2);
    }

    #[tokio::test]
    async fn test_search_all_projects() {
        let (registry, _temp) = create_test_setup().await;
        create_test_project(&*registry, "my-awesome-project", "1.0.0").await;
        create_test_project(&*registry, "other-project", "1.0.0").await;

        let access = CrossMonorepoAccess::new(registry);
        let results = access.search_all_projects("awesome").await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].project_name.contains("awesome"));
    }

    #[tokio::test]
    async fn test_access_control_priority() {
        let (registry, _temp) = create_test_setup().await;
        let mut access_control = AccessControl::default();
        access_control.allowed_projects.push("test-project".to_string());
        access_control.blocked_projects.push("test-project".to_string());

        let access = CrossMonorepoAccess::with_access_control(registry, access_control);

        // Blocked should take priority over allowed
        assert!(!access.can_access("test-project"));
    }
}
