//! Global documentation catalog
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub symbol_count: usize,
    pub coverage: f32,
    pub dependencies: Vec<String>,
    pub description: Option<String>,
    // Enhanced metadata
    pub total_modules: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub total_interfaces: usize,
    pub total_types: usize,
    pub documented_symbols: usize,
    pub documentation_coverage: f32,
    pub examples_count: usize,
    pub tests_count: usize,
    pub last_indexed: Option<i64>,
    pub last_modified: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchScope {
    Local,
    Dependencies,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocResult {
    pub project_id: String,
    pub symbol_name: String,
    pub content: String,
    pub file_path: String,
    pub relevance: f32,
    pub symbol_type: Option<String>,
    pub quality_score: Option<f32>,
}

/// Search schema for Tantivy
struct SearchSchema {
    project_id: Field,
    symbol_name: Field,
    content: Field,
    file_path: Field,
}

pub struct GlobalCatalog {
    projects: HashMap<String, ProjectMetadata>,
    docs: HashMap<String, HashMap<String, String>>,
    // Tantivy search index
    index: Option<Index>,
    reader: Option<IndexReader>,
    writer: Option<Arc<parking_lot::Mutex<IndexWriter>>>,
    schema: Option<SearchSchema>,
}

impl GlobalCatalog {
    pub fn new() -> Self {
        Self {
            projects: HashMap::new(),
            docs: HashMap::new(),
            index: None,
            reader: None,
            writer: None,
            schema: None,
        }
    }

    /// Create a new GlobalCatalog with Tantivy search enabled
    pub fn with_search(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let project_id = schema_builder.add_text_field("project_id", TEXT | STORED);
        let symbol_name = schema_builder.add_text_field("symbol_name", TEXT | STORED);
        let content = schema_builder.add_text_field("content", TEXT | STORED);
        let file_path = schema_builder.add_text_field("file_path", TEXT | STORED);

        let schema = schema_builder.build();

        // Create index directory
        std::fs::create_dir_all(index_path)?;

        // Create or open index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let writer = index.writer(50_000_000)?; // 50MB buffer

        Ok(Self {
            projects: HashMap::new(),
            docs: HashMap::new(),
            index: Some(index),
            reader: Some(reader),
            writer: Some(Arc::new(parking_lot::Mutex::new(writer))),
            schema: Some(SearchSchema {
                project_id,
                symbol_name,
                content,
                file_path,
            }),
        })
    }

    pub fn index_project(&mut self, m: ProjectMetadata) -> Result<()> {
        let id = m.id.clone();
        self.projects.insert(id.clone(), m);
        self.docs.entry(id).or_default();
        Ok(())
    }

    pub fn get_project(&self, id: &str) -> Option<&ProjectMetadata> {
        self.projects.get(id)
    }

    pub fn get_project_by_name(&self, n: &str) -> Option<&ProjectMetadata> {
        self.projects.values().find(|p| p.name == n)
    }

    pub fn get_project_by_path(&self, p: &PathBuf) -> Option<&ProjectMetadata> {
        self.projects.values().find(|m| &m.path == p)
    }

    pub fn list_projects(&self) -> Vec<&ProjectMetadata> {
        self.projects.values().collect()
    }

    pub fn add_documentation(&mut self, pid: &str, sym: &str, content: &str) -> Result<()> {
        // Add to in-memory docs
        self.docs
            .entry(pid.to_string())
            .or_default()
            .insert(sym.to_string(), content.to_string());

        // Index in Tantivy if available
        if let (Some(writer), Some(schema)) = (&self.writer, &self.schema) {
            let project = self.projects.get(pid);
            let file_path = project.map(|p| p.path.display().to_string())
                .unwrap_or_default();

            let doc = doc!(
                schema.project_id => pid,
                schema.symbol_name => sym,
                schema.content => content,
                schema.file_path => file_path,
            );

            writer.lock().add_document(doc)?;
        }

        Ok(())
    }

    pub fn get_documentation(&self, pid: &str, sym: &str) -> Option<&String> {
        self.docs.get(pid).and_then(|d| d.get(sym))
    }

    /// Commit changes to the search index
    pub fn commit(&mut self) -> Result<()> {
        if let Some(writer) = &self.writer {
            writer.lock().commit()?;
        }
        Ok(())
    }

    /// Search for documentation with relevance ranking
    pub fn search(
        &self,
        query_text: &str,
        scope: SearchScope,
        current_project: Option<&str>,
    ) -> Result<Vec<DocResult>> {
        // Handle empty query
        if query_text.trim().is_empty() {
            return Ok(vec![]);
        }

        // If Tantivy is not available, fall back to simple text search
        if self.reader.is_none() || self.schema.is_none() {
            return self.simple_search(query_text, scope, current_project);
        }

        let reader = self.reader.as_ref().unwrap();
        let schema = self.schema.as_ref().unwrap();
        let index = self.index.as_ref().unwrap();

        // Reload reader to see latest changes
        reader.reload()?;
        let searcher = reader.searcher();

        // Create query parser for multiple fields
        let query_parser = QueryParser::for_index(
            index,
            vec![schema.symbol_name, schema.content, schema.file_path],
        );

        let query = query_parser
            .parse_query(query_text)
            .context("Failed to parse search query")?;

        // Search with higher limit to allow for filtering
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1000))?;

        let mut results = Vec::new();

        // Determine which projects to search based on scope
        let project_filter = match scope {
            SearchScope::Local => {
                current_project.map(|pid| HashSet::from([pid.to_string()]))
            }
            SearchScope::Dependencies => {
                if let Some(pid) = current_project {
                    let mut deps = HashSet::new();
                    deps.insert(pid.to_string());
                    if let Some(project) = self.projects.get(pid) {
                        for dep in &project.dependencies {
                            deps.insert(dep.clone());
                        }
                    }
                    Some(deps)
                } else {
                    None
                }
            }
            SearchScope::Global => None,
        };

        // Process search results
        for (score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let project_id = retrieved_doc
                .get_first(schema.project_id)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Apply project filter
            if let Some(ref filter) = project_filter {
                if !filter.contains(&project_id) {
                    continue;
                }
            }

            let symbol_name = retrieved_doc
                .get_first(schema.symbol_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(schema.content)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let file_path = retrieved_doc
                .get_first(schema.file_path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(DocResult {
                project_id,
                symbol_name,
                content,
                file_path,
                relevance: score,
                symbol_type: None, // TODO: Extract from metadata
                quality_score: None, // TODO: Calculate from content
            });
        }

        // Sort by relevance (highest first)
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Simple fallback search when Tantivy is not available
    fn simple_search(
        &self,
        query_text: &str,
        scope: SearchScope,
        current_project: Option<&str>,
    ) -> Result<Vec<DocResult>> {
        let query_lower = query_text.to_lowercase();
        let mut results = Vec::new();

        // Determine which projects to search (collect IDs as Strings to avoid lifetime issues)
        let projects_to_search: Vec<String> = match scope {
            SearchScope::Local => {
                if let Some(pid) = current_project {
                    vec![pid.to_string()]
                } else {
                    vec![]
                }
            }
            SearchScope::Dependencies => {
                if let Some(pid) = current_project {
                    let mut deps = vec![pid.to_string()];
                    if let Some(project) = self.projects.get(pid) {
                        deps.extend(project.dependencies.iter().cloned());
                    }
                    deps
                } else {
                    vec![]
                }
            }
            SearchScope::Global => self.projects.keys().cloned().collect(),
        };

        // Search through documentation
        for project_id in projects_to_search {
            if let Some(project_docs) = self.docs.get(&project_id) {
                let project = self.projects.get(&project_id);
                let file_path = project
                    .map(|p| p.path.display().to_string())
                    .unwrap_or_default();

                for (symbol_name, content) in project_docs {
                    let symbol_lower = symbol_name.to_lowercase();
                    let content_lower = content.to_lowercase();

                    // Calculate simple relevance score
                    let mut relevance = 0.0;

                    if symbol_lower.contains(&query_lower) {
                        relevance += 2.0; // Symbol name match is more important
                    }

                    if content_lower.contains(&query_lower) {
                        relevance += 1.0;
                    }

                    if relevance > 0.0 {
                        results.push(DocResult {
                            project_id: project_id.clone(),
                            symbol_name: symbol_name.clone(),
                            content: content.clone(),
                            file_path: file_path.clone(),
                            relevance,
                            symbol_type: None, // TODO: Extract from metadata
                            quality_score: None, // TODO: Calculate from content
                        });
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    pub fn get_project_docs(&self, pid: &str) -> Option<&HashMap<String, String>> {
        self.docs.get(pid)
    }

    pub fn remove_project(&mut self, pid: &str) -> Result<()> {
        self.projects.remove(pid);
        self.docs.remove(pid);
        Ok(())
    }

    pub fn update_project(&mut self, m: ProjectMetadata) -> Result<()> {
        self.projects.insert(m.id.clone(), m);
        Ok(())
    }
}

impl Default for GlobalCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn pm(id: &str, n: &str, p: &str) -> ProjectMetadata {
        ProjectMetadata {
            id: id.to_string(),
            name: n.to_string(),
            path: PathBuf::from(p),
            symbol_count: 0,
            coverage: 0.0,
            dependencies: vec![],
            description: None,
            total_modules: 0,
            total_functions: 0,
            total_classes: 0,
            total_interfaces: 0,
            total_types: 0,
            documented_symbols: 0,
            documentation_coverage: 0.0,
            examples_count: 0,
            tests_count: 0,
            last_indexed: None,
            last_modified: None,
        }
    }

    #[test]
    fn test_index_project() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        assert!(c.get_project("p1").is_some());
    }

    #[test]
    fn test_get_project_by_name() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        assert!(c.get_project_by_name("proj").is_some());
    }

    #[test]
    fn test_get_project_by_path() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        assert!(c.get_project_by_path(&PathBuf::from("/p")).is_some());
    }

    #[test]
    fn test_add_and_get_documentation() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.add_documentation("p1", "f", "doc").unwrap();
        assert!(c.get_documentation("p1", "f").is_some());
    }

    #[test]
    fn test_simple_search_local_scope() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.add_documentation("p1", "test_function", "This is a test function").unwrap();

        let results = c.search("test", SearchScope::Local, Some("p1")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_name, "test_function");
    }

    #[test]
    fn test_simple_search_global_scope() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj1", "/p1")).unwrap();
        c.index_project(pm("p2", "proj2", "/p2")).unwrap();
        c.add_documentation("p1", "func1", "Documentation for function one").unwrap();
        c.add_documentation("p2", "func2", "Documentation for function two").unwrap();

        let results = c.search("function", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_simple_search_dependencies_scope() {
        let mut c = GlobalCatalog::new();
        let mut p1 = pm("p1", "proj1", "/p1");
        p1.dependencies = vec!["p2".to_string()];

        c.index_project(p1).unwrap();
        c.index_project(pm("p2", "proj2", "/p2")).unwrap();
        c.index_project(pm("p3", "proj3", "/p3")).unwrap();

        c.add_documentation("p1", "func1", "Main function").unwrap();
        c.add_documentation("p2", "func2", "Dependency function").unwrap();
        c.add_documentation("p3", "func3", "Other function").unwrap();

        let results = c.search("function", SearchScope::Dependencies, Some("p1")).unwrap();
        // Should find func1 and func2, but not func3
        assert_eq!(results.len(), 2);
        let project_ids: Vec<String> = results.iter().map(|r| r.project_id.clone()).collect();
        assert!(project_ids.contains(&"p1".to_string()));
        assert!(project_ids.contains(&"p2".to_string()));
        assert!(!project_ids.contains(&"p3".to_string()));
    }

    #[test]
    fn test_search_empty_query() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.add_documentation("p1", "func", "documentation").unwrap();

        let results = c.search("", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_no_results() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.add_documentation("p1", "func", "documentation").unwrap();

        let results = c.search("nonexistent", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_relevance_ordering() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.add_documentation("p1", "test", "Some content").unwrap();
        c.add_documentation("p1", "other", "This mentions test in content").unwrap();

        let results = c.search("test", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 2);
        // "test" symbol should have higher relevance than "other"
        assert_eq!(results[0].symbol_name, "test");
    }

    #[test]
    fn test_tantivy_search_with_index() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("catalog_index");
        let mut c = GlobalCatalog::with_search(&index_path).unwrap();

        c.index_project(pm("p1", "proj1", "/p1")).unwrap();
        c.add_documentation("p1", "calculate", "Calculates the sum of two numbers").unwrap();
        c.add_documentation("p1", "process", "Processes data").unwrap();
        c.commit().unwrap();

        let results = c.search("calculate", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol_name, "calculate");
    }

    #[test]
    fn test_tantivy_search_multiple_fields() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("catalog_index");
        let mut c = GlobalCatalog::with_search(&index_path).unwrap();

        c.index_project(pm("p1", "proj1", "/p1")).unwrap();
        c.add_documentation("p1", "func1", "This function handles authentication").unwrap();
        c.add_documentation("p1", "authentication", "Helper for auth").unwrap();
        c.commit().unwrap();

        let results = c.search("authentication", SearchScope::Global, None).unwrap();
        assert_eq!(results.len(), 2);
        // Results should be sorted by relevance
        assert!(results[0].relevance >= results[1].relevance);
    }

    #[test]
    fn test_tantivy_search_local_scope() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("catalog_index");
        let mut c = GlobalCatalog::with_search(&index_path).unwrap();

        c.index_project(pm("p1", "proj1", "/p1")).unwrap();
        c.index_project(pm("p2", "proj2", "/p2")).unwrap();
        c.add_documentation("p1", "func1", "Test function in p1").unwrap();
        c.add_documentation("p2", "func2", "Test function in p2").unwrap();
        c.commit().unwrap();

        let results = c.search("Test", SearchScope::Local, Some("p1")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].project_id, "p1");
    }

    #[test]
    fn test_tantivy_search_dependencies_scope() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("catalog_index");
        let mut c = GlobalCatalog::with_search(&index_path).unwrap();

        let mut p1 = pm("p1", "proj1", "/p1");
        p1.dependencies = vec!["p2".to_string()];

        c.index_project(p1).unwrap();
        c.index_project(pm("p2", "proj2", "/p2")).unwrap();
        c.index_project(pm("p3", "proj3", "/p3")).unwrap();

        c.add_documentation("p1", "func1", "Helper function").unwrap();
        c.add_documentation("p2", "func2", "Helper in dependency").unwrap();
        c.add_documentation("p3", "func3", "Helper in other").unwrap();
        c.commit().unwrap();

        let results = c.search("Helper", SearchScope::Dependencies, Some("p1")).unwrap();
        assert_eq!(results.len(), 2);
        let project_ids: Vec<String> = results.iter().map(|r| r.project_id.clone()).collect();
        assert!(project_ids.contains(&"p1".to_string()));
        assert!(project_ids.contains(&"p2".to_string()));
        assert!(!project_ids.contains(&"p3".to_string()));
    }

    #[test]
    fn test_remove_project() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        c.remove_project("p1").unwrap();
        assert!(c.get_project("p1").is_none());
    }

    #[test]
    fn test_update_project() {
        let mut c = GlobalCatalog::new();
        c.index_project(pm("p1", "proj", "/p")).unwrap();
        let mut m = pm("p1", "proj", "/p");
        m.symbol_count = 100;
        c.update_project(m).unwrap();
        assert_eq!(c.get_project("p1").unwrap().symbol_count, 100);
    }

    #[test]
    fn test_complex_query_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let index_path = temp_dir.path().join("catalog_index");
        let mut c = GlobalCatalog::with_search(&index_path).unwrap();

        c.index_project(pm("p1", "proj1", "/p1")).unwrap();
        c.add_documentation("p1", "user_auth", "User authentication and authorization").unwrap();
        c.add_documentation("p1", "session", "Session management").unwrap();
        c.commit().unwrap();

        // Test multi-word query
        let results = c.search("user authentication", SearchScope::Global, None).unwrap();
        assert!(!results.is_empty());
    }
}
