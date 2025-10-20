//! Dependency graph for cross-monorepo dependency resolution
//!
//! This module provides dependency graph building and analysis for projects
//! across multiple monorepos, supporting transitive dependencies, cycle detection,
//! and dependency traversal.

use super::registry::{ProjectRegistry, ProjectRegistryManager};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;

/// Type of dependency relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    /// Runtime dependency (production)
    Runtime,
    /// Development dependency
    Dev,
    /// Peer dependency
    Peer,
}

/// Node in the dependency graph representing a project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectNode {
    /// Unique project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
}

/// Edge in the dependency graph representing a dependency relationship
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source project ID
    pub from: String,
    /// Target project ID (dependency)
    pub to: String,
    /// Type of dependency
    pub dep_type: DependencyType,
}

/// Complete dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All project nodes indexed by ID
    nodes: HashMap<String, ProjectNode>,
    /// All dependency edges
    edges: Vec<DependencyEdge>,
    /// Adjacency list for fast lookups (outgoing edges)
    adjacency: HashMap<String, Vec<String>>,
    /// Reverse adjacency list (incoming edges)
    reverse_adjacency: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
        }
    }

    /// Add a project node to the graph
    pub fn add_node(&mut self, node: ProjectNode) {
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        self.adjacency.entry(id.clone()).or_default();
        self.reverse_adjacency.entry(id).or_default();
    }

    /// Add a dependency edge to the graph
    pub fn add_edge(&mut self, edge: DependencyEdge) {
        // Ensure both nodes exist
        if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
            return;
        }

        // Add to adjacency lists
        self.adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());

        self.reverse_adjacency
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());

        // Add edge
        self.edges.push(edge);
    }

    /// Build the dependency graph from package.json files
    pub async fn build(&mut self, registry: &ProjectRegistryManager) -> Result<()> {
        let projects = registry.list_all().await?;

        // First pass: add all nodes
        for project in &projects {
            let node = ProjectNode {
                id: project.identity.full_id.clone(),
                name: project.identity.id.clone(),
                version: project.identity.version.clone(),
            };
            self.add_node(node);
        }

        // Second pass: parse dependencies and add edges
        for project in &projects {
            let deps = Self::parse_dependencies(&project.current_path).await?;
            for (dep_name, dep_type) in deps {
                // Find the dependency in our registry
                if let Some(dep_project) = Self::find_dependency_project(&projects, &dep_name) {
                    let edge = DependencyEdge {
                        from: project.identity.full_id.clone(),
                        to: dep_project.identity.full_id.clone(),
                        dep_type,
                    };
                    self.add_edge(edge);
                }
            }
        }

        Ok(())
    }

    /// Parse dependencies from package.json or Cargo.toml
    async fn parse_dependencies(path: &Path) -> Result<Vec<(String, DependencyType)>> {
        let mut deps = Vec::new();

        // Try package.json first
        let package_json = path.join("package.json");
        if package_json.exists() {
            let content = tokio::fs::read_to_string(&package_json).await?;
            let pkg: serde_json::Value = serde_json::from_str(&content)?;

            // Runtime dependencies
            if let Some(dependencies) = pkg["dependencies"].as_object() {
                for (name, _) in dependencies {
                    deps.push((name.clone(), DependencyType::Runtime));
                }
            }

            // Dev dependencies
            if let Some(dev_dependencies) = pkg["devDependencies"].as_object() {
                for (name, _) in dev_dependencies {
                    deps.push((name.clone(), DependencyType::Dev));
                }
            }

            // Peer dependencies
            if let Some(peer_dependencies) = pkg["peerDependencies"].as_object() {
                for (name, _) in peer_dependencies {
                    deps.push((name.clone(), DependencyType::Peer));
                }
            }
        }

        // Try Cargo.toml
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = tokio::fs::read_to_string(&cargo_toml).await?;
            let cargo: toml::Value = toml::from_str(&content)?;

            // Runtime dependencies
            if let Some(dependencies) = cargo.get("dependencies").and_then(|v| v.as_table()) {
                for (name, _) in dependencies {
                    deps.push((name.clone(), DependencyType::Runtime));
                }
            }

            // Dev dependencies
            if let Some(dev_dependencies) = cargo.get("dev-dependencies").and_then(|v| v.as_table()) {
                for (name, _) in dev_dependencies {
                    deps.push((name.clone(), DependencyType::Dev));
                }
            }
        }

        Ok(deps)
    }

    /// Find a project by dependency name
    fn find_dependency_project<'a>(projects: &'a [ProjectRegistry], dep_name: &str) -> Option<&'a ProjectRegistry> {
        projects.iter().find(|p| {
            p.identity.id == dep_name || p.identity.id.ends_with(&format!("/{}", dep_name))
        })
    }

    /// Find all dependencies of a project up to a certain depth
    pub fn find_dependencies(&self, project_id: &str, depth: usize) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((project_id.to_string(), 0));
        visited.insert(project_id.to_string());

        while let Some((current_id, current_depth)) = queue.pop_front() {
            if current_depth >= depth {
                continue;
            }

            if let Some(deps) = self.adjacency.get(&current_id) {
                for dep_id in deps {
                    if !visited.contains(dep_id) {
                        visited.insert(dep_id.clone());
                        result.push(dep_id.clone());
                        queue.push_back((dep_id.clone(), current_depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Find all projects that depend on a given project (reverse dependencies)
    pub fn find_dependents(&self, project_id: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(project_id.to_string());
        visited.insert(project_id.to_string());

        while let Some(current_id) = queue.pop_front() {
            if let Some(dependents) = self.reverse_adjacency.get(&current_id) {
                for dependent_id in dependents {
                    if !visited.contains(dependent_id) {
                        visited.insert(dependent_id.clone());
                        result.push(dependent_id.clone());
                        queue.push_back(dependent_id.clone());
                    }
                }
            }
        }

        result
    }

    /// Detect circular dependencies in the graph
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                self.dfs_detect_cycles(
                    node_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// DFS helper for cycle detection
    fn dfs_detect_cycles(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_detect_cycles(neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle - extract it from path
                    if let Some(cycle_start) = path.iter().position(|id| id == neighbor) {
                        let cycle = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Get all nodes in the graph
    pub fn nodes(&self) -> &HashMap<String, ProjectNode> {
        &self.nodes
    }

    /// Get all edges in the graph
    pub fn edges(&self) -> &[DependencyEdge] {
        &self.edges
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ProjectNode> {
        self.nodes.get(id)
    }

    /// Filter dependencies by type
    pub fn filter_by_type(&self, project_id: &str, dep_type: DependencyType) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.from == project_id && e.dep_type == dep_type)
            .map(|e| e.to.clone())
            .collect()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency graph manager with caching
pub struct DependencyGraphManager {
    registry: Arc<ProjectRegistryManager>,
    cached_graph: Option<DependencyGraph>,
}

impl DependencyGraphManager {
    /// Create a new dependency graph manager
    pub fn new(registry: Arc<ProjectRegistryManager>) -> Self {
        Self {
            registry,
            cached_graph: None,
        }
    }

    /// Get the dependency graph, building it if necessary
    pub async fn get_graph(&mut self) -> Result<&DependencyGraph> {
        if self.cached_graph.is_none() {
            let mut graph = DependencyGraph::new();
            graph.build(&self.registry).await?;
            self.cached_graph = Some(graph);
        }
        Ok(self.cached_graph.as_ref().unwrap())
    }

    /// Invalidate the cached graph
    pub fn invalidate(&mut self) {
        self.cached_graph = None;
    }

    /// Rebuild the graph
    pub async fn rebuild(&mut self) -> Result<()> {
        self.invalidate();
        self.get_graph().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_node(id: &str, name: &str, version: &str) -> ProjectNode {
        ProjectNode {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    fn create_edge(from: &str, to: &str, dep_type: DependencyType) -> DependencyEdge {
        DependencyEdge {
            from: from.to_string(),
            to: to.to_string(),
            dep_type,
        }
    }

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();
        let node = create_node("p1@1.0.0", "p1", "1.0.0");
        graph.add_node(node.clone());

        assert_eq!(graph.nodes().len(), 1);
        assert!(graph.get_node("p1@1.0.0").is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));

        assert_eq!(graph.edges().len(), 1);
        assert_eq!(graph.adjacency.get("p1@1.0.0").unwrap().len(), 1);
    }

    #[test]
    fn test_find_dependencies_depth_1() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));
        graph.add_node(create_node("p3@1.0.0", "p3", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p2@1.0.0", "p3@1.0.0", DependencyType::Runtime));

        let deps = graph.find_dependencies("p1@1.0.0", 1);
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"p2@1.0.0".to_string()));
    }

    #[test]
    fn test_find_dependencies_depth_2() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));
        graph.add_node(create_node("p3@1.0.0", "p3", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p2@1.0.0", "p3@1.0.0", DependencyType::Runtime));

        let deps = graph.find_dependencies("p1@1.0.0", 2);
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"p2@1.0.0".to_string()));
        assert!(deps.contains(&"p3@1.0.0".to_string()));
    }

    #[test]
    fn test_find_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));
        graph.add_node(create_node("p3@1.0.0", "p3", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p3@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p2@1.0.0", "p3@1.0.0", DependencyType::Runtime));

        let dependents = graph.find_dependents("p3@1.0.0");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"p1@1.0.0".to_string()));
        assert!(dependents.contains(&"p2@1.0.0".to_string()));
    }

    #[test]
    fn test_detect_cycles_no_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));

        let cycles = graph.detect_cycles();
        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_detect_cycles_with_cycle() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));
        graph.add_node(create_node("p3@1.0.0", "p3", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p2@1.0.0", "p3@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p3@1.0.0", "p1@1.0.0", DependencyType::Runtime));

        let cycles = graph.detect_cycles();
        assert!(cycles.len() > 0);
    }

    #[test]
    fn test_filter_by_type() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_node("p1@1.0.0", "p1", "1.0.0"));
        graph.add_node(create_node("p2@1.0.0", "p2", "1.0.0"));
        graph.add_node(create_node("p3@1.0.0", "p3", "1.0.0"));

        graph.add_edge(create_edge("p1@1.0.0", "p2@1.0.0", DependencyType::Runtime));
        graph.add_edge(create_edge("p1@1.0.0", "p3@1.0.0", DependencyType::Dev));

        let runtime_deps = graph.filter_by_type("p1@1.0.0", DependencyType::Runtime);
        assert_eq!(runtime_deps.len(), 1);
        assert!(runtime_deps.contains(&"p2@1.0.0".to_string()));

        let dev_deps = graph.filter_by_type("p1@1.0.0", DependencyType::Dev);
        assert_eq!(dev_deps.len(), 1);
        assert!(dev_deps.contains(&"p3@1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_parse_package_json_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        tokio::fs::write(
            &package_json,
            r#"{
                "name": "test-pkg",
                "dependencies": {
                    "dep1": "^1.0.0",
                    "dep2": "^2.0.0"
                },
                "devDependencies": {
                    "dev-dep": "^1.0.0"
                },
                "peerDependencies": {
                    "peer-dep": "^3.0.0"
                }
            }"#,
        )
        .await
        .unwrap();

        let deps = DependencyGraph::parse_dependencies(temp_dir.path())
            .await
            .unwrap();

        assert_eq!(deps.len(), 4);
        assert!(deps.contains(&("dep1".to_string(), DependencyType::Runtime)));
        assert!(deps.contains(&("dep2".to_string(), DependencyType::Runtime)));
        assert!(deps.contains(&("dev-dep".to_string(), DependencyType::Dev)));
        assert!(deps.contains(&("peer-dep".to_string(), DependencyType::Peer)));
    }
}
