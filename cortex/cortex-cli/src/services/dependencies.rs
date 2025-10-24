//! Dependency service layer
//!
//! Provides unified dependency analysis operations for both API and MCP modules.
//! Eliminates duplication between API routes and MCP tools.

use anyhow::Result;
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Dependency service for dependency analysis and graph operations
#[derive(Clone)]
pub struct DependencyService {
    storage: Arc<ConnectionManager>,
}

impl DependencyService {
    /// Create a new dependency service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Get dependency graph for a workspace
    pub async fn get_dependency_graph(
        &self,
        workspace_id: Uuid,
        depth: Option<usize>,
    ) -> Result<DependencyGraph> {
        debug!(
            "Getting dependency graph for workspace: {} with depth: {:?}",
            workspace_id, depth
        );

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let max_depth = depth.unwrap_or(10);

        // Query code units in workspace
        let units_query = format!(
            "SELECT id, name, qualified_name, unit_type, file_path FROM code_unit WHERE file_path CONTAINS '{}'",
            workspace_id
        );
        let mut result = conn.query(&units_query).await?;
        let units: Vec<serde_json::Value> = result.take(0)?;

        // Query relations (dependencies)
        let relations_query = format!(
            "SELECT * FROM relation WHERE source_id IN (SELECT id FROM code_unit WHERE file_path CONTAINS '{}')",
            workspace_id
        );
        let mut relations_result = conn.query(&relations_query).await?;
        let relations: Vec<serde_json::Value> = relations_result.take(0)?;

        // Build graph nodes
        let mut nodes = Vec::new();
        let mut node_map = HashMap::new();

        for (idx, unit) in units.iter().enumerate() {
            let id = unit["id"].as_str().unwrap_or("unknown").to_string();
            let name = unit["name"].as_str().unwrap_or("unknown").to_string();
            let unit_type = unit["unit_type"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            nodes.push(DependencyNode {
                id: id.clone(),
                name: name.clone(),
                node_type: unit_type,
                metadata: unit.clone(),
            });

            node_map.insert(id, idx);
        }

        // Build graph edges
        let mut edges = Vec::new();

        for relation in &relations {
            let from = relation["source_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let to = relation["target_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let edge_type = relation["relation_type"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let weight = relation["weight"].as_f64().unwrap_or(1.0) as f32;

            edges.push(DependencyEdge {
                from: from.clone(),
                to: to.clone(),
                edge_type,
                weight,
            });
        }

        // Detect cycles
        let cycle_count = self.count_cycles_in_graph(&nodes, &edges);

        info!(
            "Built dependency graph with {} nodes, {} edges, {} cycles",
            nodes.len(),
            edges.len(),
            cycle_count
        );

        Ok(DependencyGraph {
            nodes,
            edges,
            max_depth,
            cycle_count,
        })
    }

    /// Detect circular dependencies
    pub async fn detect_cycles(&self, workspace_id: Uuid) -> Result<Vec<Cycle>> {
        debug!("Detecting cycles in workspace: {}", workspace_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Get all relations in workspace
        let relations_query = format!(
            "SELECT * FROM relation WHERE source_id IN (SELECT id FROM code_unit WHERE file_path CONTAINS '{}')",
            workspace_id
        );
        let mut result = conn.query(&relations_query).await?;
        let relations: Vec<serde_json::Value> = result.take(0)?;

        // Build adjacency list
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        for relation in &relations {
            let from = relation["source_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let to = relation["target_id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            graph.entry(from).or_default().push(to);
        }

        // Detect cycles using DFS
        let cycle_paths = self.find_cycles_dfs(&graph);

        info!("Found {} cycles", cycle_paths.len());

        // Convert to Cycle structs
        let cycles = cycle_paths
            .into_iter()
            .enumerate()
            .map(|(idx, entities)| {
                let severity = if entities.len() > 5 {
                    CycleSeverity::High
                } else if entities.len() > 3 {
                    CycleSeverity::Medium
                } else {
                    CycleSeverity::Low
                };

                Cycle {
                    cycle_id: format!("cycle_{}", idx),
                    entities,
                    severity,
                    suggestions: vec![
                        "Consider extracting shared functionality".to_string(),
                        "Use dependency inversion".to_string(),
                        "Refactor to remove circular reference".to_string(),
                    ],
                }
            })
            .collect();

        Ok(cycles)
    }

    /// Analyze impact of changes to code units
    pub async fn analyze_impact(
        &self,
        workspace_id: Uuid,
        changed_units: Vec<String>,
    ) -> Result<ImpactAnalysis> {
        debug!(
            "Analyzing impact of {} changed units in workspace: {}",
            changed_units.len(),
            workspace_id
        );

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        let mut changed_entities = Vec::new();
        let mut affected_entities = Vec::new();
        let mut all_affected_ids: HashSet<String> = HashSet::new();

        // For each changed entity, find dependents
        for entity_id in &changed_units {
            // Get entity info
            let entity_query = format!("SELECT * FROM code_unit WHERE id = '{}'", entity_id);
            let mut result = conn.query(&entity_query).await?;
            let entities: Vec<serde_json::Value> = result.take(0)?;

            if let Some(entity) = entities.first() {
                let name = entity["name"].as_str().unwrap_or("unknown").to_string();
                let entity_type = entity["unit_type"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();

                // Find direct dependents
                let dependents_query = format!(
                    "SELECT * FROM relation WHERE target_id = '{}'",
                    entity_id
                );
                let mut deps_result = conn.query(&dependents_query).await?;
                let dependents: Vec<serde_json::Value> = deps_result.take(0)?;

                let affects: Vec<String> = dependents
                    .iter()
                    .filter_map(|d| d["source_id"].as_str().map(String::from))
                    .collect();

                all_affected_ids.extend(affects.iter().cloned());

                changed_entities.push(ImpactedEntity {
                    id: entity_id.clone(),
                    name,
                    entity_type,
                    impact_level: ImpactLevel::Changed,
                    affected_by: vec![],
                    affects: affects.clone(),
                });
            }
        }

        // Get info for all affected entities
        if !all_affected_ids.is_empty() {
            for affected_id in &all_affected_ids {
                let entity_query = format!("SELECT * FROM code_unit WHERE id = '{}'", affected_id);
                let mut result = conn.query(&entity_query).await?;
                let entities: Vec<serde_json::Value> = result.take(0)?;

                if let Some(entity) = entities.first() {
                    let name = entity["name"].as_str().unwrap_or("unknown").to_string();
                    let entity_type = entity["unit_type"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string();

                    // Find what affects this entity
                    let dependencies_query = format!(
                        "SELECT target_id FROM relation WHERE source_id = '{}'",
                        affected_id
                    );
                    let mut deps_result = conn.query(&dependencies_query).await?;
                    let dependencies: Vec<serde_json::Value> = deps_result.take(0)?;

                    let affected_by: Vec<String> = dependencies
                        .iter()
                        .filter_map(|d| d["target_id"].as_str().map(String::from))
                        .filter(|id| changed_units.contains(id))
                        .collect();

                    affected_entities.push(ImpactedEntity {
                        id: affected_id.clone(),
                        name,
                        entity_type,
                        impact_level: ImpactLevel::Affected,
                        affected_by,
                        affects: vec![],
                    });
                }
            }
        }

        // Calculate risk
        let risk_score = (all_affected_ids.len() as f64 / 100.0).min(1.0);
        let risk_level = if risk_score > 0.7 {
            RiskLevel::High
        } else if risk_score > 0.3 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let recommendations = if risk_score > 0.5 {
            vec![
                "Consider breaking changes into smaller increments".to_string(),
                "Run comprehensive tests".to_string(),
                "Review all affected code paths".to_string(),
            ]
        } else {
            vec!["Run tests for affected areas".to_string()]
        };

        // Calculate critical paths
        let critical_paths = self
            .calculate_critical_paths(&changed_units, &all_affected_ids)
            .await?;

        info!(
            "Impact analysis: {} changed, {} affected, risk: {:?}",
            changed_entities.len(),
            affected_entities.len(),
            risk_level
        );

        Ok(ImpactAnalysis {
            changed_entities,
            affected_entities,
            risk_level,
            risk_score,
            critical_paths,
            recommendations,
        })
    }

    /// Get call graph for a code unit
    pub async fn get_call_graph(&self, unit_id: &str) -> Result<CallGraph> {
        debug!("Getting call graph for unit: {}", unit_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Get unit info
        let unit_query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
        let mut result = conn.query(&unit_query).await?;
        let units: Vec<serde_json::Value> = result.take(0)?;

        let unit = units
            .first()
            .ok_or_else(|| anyhow::anyhow!("Unit not found: {}", unit_id))?;

        let root_name = unit["qualified_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        // Get outgoing calls (what this unit calls)
        let calls_query = format!(
            "SELECT * FROM relation WHERE source_id = '{}' AND relation_type = 'calls'",
            unit_id
        );
        let mut calls_result = conn.query(&calls_query).await?;
        let calls: Vec<serde_json::Value> = calls_result.take(0)?;

        let calls_to: Vec<String> = calls
            .iter()
            .filter_map(|c| c["target_id"].as_str().map(String::from))
            .collect();

        // Get incoming calls (who calls this unit)
        let callers_query = format!(
            "SELECT * FROM relation WHERE target_id = '{}' AND relation_type = 'calls'",
            unit_id
        );
        let mut callers_result = conn.query(&callers_query).await?;
        let callers: Vec<serde_json::Value> = callers_result.take(0)?;

        let called_by: Vec<String> = callers
            .iter()
            .filter_map(|c| c["source_id"].as_str().map(String::from))
            .collect();

        info!(
            "Call graph for {}: {} calls, {} callers",
            root_name,
            calls_to.len(),
            called_by.len()
        );

        Ok(CallGraph {
            root_unit_id: unit_id.to_string(),
            root_name,
            calls_to,
            called_by,
        })
    }

    /// Find unused code units in workspace
    pub async fn find_unused_code(&self, workspace_id: Uuid) -> Result<Vec<UnusedUnit>> {
        debug!("Finding unused code in workspace: {}", workspace_id);

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Get all code units in workspace
        let units_query = format!(
            "SELECT id, name, qualified_name, unit_type FROM code_unit WHERE file_path CONTAINS '{}'",
            workspace_id
        );
        let mut result = conn.query(&units_query).await?;
        let units: Vec<serde_json::Value> = result.take(0)?;

        let mut unused_units = Vec::new();

        // Check each unit for incoming references
        for unit in units {
            let unit_id = unit["id"].as_str().unwrap_or("unknown");

            // Count incoming relations
            let refs_query = format!(
                "SELECT count() FROM relation WHERE target_id = '{}' GROUP ALL",
                unit_id
            );
            let mut refs_result = conn.query(&refs_query).await?;
            let ref_count: usize = refs_result
                .take::<Option<usize>>(0)?
                .unwrap_or(0);

            // If no incoming references, it's potentially unused
            if ref_count == 0 {
                let name = unit["name"].as_str().unwrap_or("unknown").to_string();
                let qualified_name = unit["qualified_name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let unit_type = unit["unit_type"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();

                let confidence = if unit_type == "private" { 0.9 } else { 0.5 };

                unused_units.push(UnusedUnit {
                    id: unit_id.to_string(),
                    name,
                    qualified_name,
                    unit_type,
                    confidence,
                });
            }
        }

        info!("Found {} potentially unused code units", unused_units.len());

        Ok(unused_units)
    }

    // Helper methods

    fn count_cycles_in_graph(&self, _nodes: &[DependencyNode], edges: &[DependencyEdge]) -> usize {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for edge in edges {
            graph
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }

        self.find_cycles_dfs(&graph).len()
    }

    fn find_cycles_dfs(&self, graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                self.dfs_visit(
                    node,
                    graph,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_visit(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_visit(neighbor, graph, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    if let Some(pos) = path.iter().position(|n| n == neighbor) {
                        cycles.push(path[pos..].to_vec());
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    async fn calculate_critical_paths(
        &self,
        changed_units: &[String],
        affected_ids: &HashSet<String>,
    ) -> Result<Vec<Vec<String>>> {
        if changed_units.is_empty() || affected_ids.is_empty() {
            return Ok(vec![]);
        }

        let pooled = self.storage.acquire().await?;
        let conn = pooled.connection();

        // Build dependency graph for affected entities
        let mut graph: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize all nodes
        for entity_id in changed_units.iter().chain(affected_ids.iter()) {
            in_degree.entry(entity_id.clone()).or_insert(0);
        }

        // Query relations for affected entities
        let all_entity_ids: Vec<String> = changed_units
            .iter()
            .chain(affected_ids.iter())
            .cloned()
            .collect();

        for entity_id in &all_entity_ids {
            let relations_query = format!(
                "SELECT * FROM relation WHERE source_id = '{}'",
                entity_id
            );
            let mut relations_result = conn.query(&relations_query).await?;
            let relations: Vec<serde_json::Value> = relations_result.take(0)?;

            for relation in relations {
                let target_id = relation["target_id"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();

                // Only consider edges within our affected set
                if affected_ids.contains(&target_id) || changed_units.contains(&target_id) {
                    let weight = relation["weight"].as_f64().unwrap_or(1.0);

                    graph
                        .entry(entity_id.clone())
                        .or_default()
                        .push((target_id.clone(), weight));

                    *in_degree.entry(target_id).or_insert(0) += 1;
                }
            }
        }

        // Find longest paths using topological sort + dynamic programming
        let mut longest_dist: HashMap<String, f64> = HashMap::new();
        let mut path_predecessor: HashMap<String, String> = HashMap::new();

        // Initialize distances for changed entities
        for entity_id in changed_units {
            longest_dist.insert(entity_id.clone(), 0.0);
        }

        // Topological sort using Kahn's algorithm
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut topo_order = Vec::new();

        while let Some(node) = queue.pop() {
            topo_order.push(node.clone());

            if let Some(neighbors) = graph.get(&node) {
                for (neighbor, _) in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Calculate longest distances using topological order
        for node in &topo_order {
            let current_dist = *longest_dist.get(node).unwrap_or(&f64::NEG_INFINITY);

            if let Some(neighbors) = graph.get(node) {
                for (neighbor, weight) in neighbors {
                    let new_dist = current_dist + weight;
                    let neighbor_dist = *longest_dist.get(neighbor).unwrap_or(&f64::NEG_INFINITY);

                    if new_dist > neighbor_dist {
                        longest_dist.insert(neighbor.clone(), new_dist);
                        path_predecessor.insert(neighbor.clone(), node.clone());
                    }
                }
            }
        }

        // Find top critical paths (nodes with longest distances)
        let mut critical_nodes: Vec<(String, f64)> = longest_dist
            .iter()
            .filter(|(id, dist)| **dist > 0.0 && affected_ids.contains(*id))
            .map(|(id, dist)| (id.clone(), *dist))
            .collect();

        critical_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 5 critical paths and reconstruct them
        let mut critical_paths = Vec::new();
        for (node, _) in critical_nodes.iter().take(5) {
            let mut path = vec![node.clone()];
            let mut current = node.clone();

            // Trace back to a changed entity
            while let Some(pred) = path_predecessor.get(&current) {
                path.push(pred.clone());
                current = pred.clone();
                if changed_units.contains(&current) {
                    break;
                }
            }

            path.reverse();
            critical_paths.push(path);
        }

        Ok(critical_paths)
    }
}

// ============================================================================
// Types
// ============================================================================

/// Dependency graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
    pub max_depth: usize,
    pub cycle_count: usize,
}

/// Graph node representing a code unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub metadata: serde_json::Value,
}

/// Graph edge representing a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub weight: f32,
}

/// Circular dependency cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cycle {
    pub cycle_id: String,
    pub entities: Vec<String>,
    pub severity: CycleSeverity,
    pub suggestions: Vec<String>,
}

/// Cycle severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CycleSeverity {
    Low,
    Medium,
    High,
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub changed_entities: Vec<ImpactedEntity>,
    pub affected_entities: Vec<ImpactedEntity>,
    pub risk_level: RiskLevel,
    pub risk_score: f64,
    pub critical_paths: Vec<Vec<String>>,
    pub recommendations: Vec<String>,
}

/// Entity affected by changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactedEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub impact_level: ImpactLevel,
    pub affected_by: Vec<String>,
    pub affects: Vec<String>,
}

/// Impact level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Changed,
    Affected,
}

/// Risk level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Call graph for a code unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraph {
    pub root_unit_id: String,
    pub root_name: String,
    pub calls_to: Vec<String>,
    pub called_by: Vec<String>,
}

/// Unused code unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedUnit {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub unit_type: String,
    pub confidence: f64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_serialization() {
        let graph = DependencyGraph {
            nodes: vec![DependencyNode {
                id: "node1".to_string(),
                name: "test_node".to_string(),
                node_type: "function".to_string(),
                metadata: serde_json::json!({}),
            }],
            edges: vec![DependencyEdge {
                from: "node1".to_string(),
                to: "node2".to_string(),
                edge_type: "calls".to_string(),
                weight: 1.0,
            }],
            max_depth: 10,
            cycle_count: 0,
        };

        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.contains("test_node"));

        let deserialized: DependencyGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.nodes.len(), 1);
        assert_eq!(deserialized.edges.len(), 1);
    }

    #[test]
    fn test_cycle_severity() {
        let cycle = Cycle {
            cycle_id: "cycle_1".to_string(),
            entities: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            severity: CycleSeverity::Medium,
            suggestions: vec!["Fix it".to_string()],
        };

        assert_eq!(cycle.entities.len(), 3);
        assert!(matches!(cycle.severity, CycleSeverity::Medium));
    }

    #[test]
    fn test_impact_analysis() {
        let analysis = ImpactAnalysis {
            changed_entities: vec![],
            affected_entities: vec![],
            risk_level: RiskLevel::Low,
            risk_score: 0.2,
            critical_paths: vec![],
            recommendations: vec!["Run tests".to_string()],
        };

        assert!(matches!(analysis.risk_level, RiskLevel::Low));
        assert_eq!(analysis.risk_score, 0.2);
    }

    #[test]
    fn test_call_graph() {
        let call_graph = CallGraph {
            root_unit_id: "unit1".to_string(),
            root_name: "main".to_string(),
            calls_to: vec!["unit2".to_string(), "unit3".to_string()],
            called_by: vec!["unit0".to_string()],
        };

        assert_eq!(call_graph.calls_to.len(), 2);
        assert_eq!(call_graph.called_by.len(), 1);
    }

    #[test]
    fn test_unused_unit() {
        let unused = UnusedUnit {
            id: "unit1".to_string(),
            name: "old_function".to_string(),
            qualified_name: "module::old_function".to_string(),
            unit_type: "function".to_string(),
            confidence: 0.9,
        };

        assert_eq!(unused.confidence, 0.9);
        assert!(unused.confidence > 0.8);
    }
}
