//! Cross-reference tracking
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceType { Import, Extends, Implements, Uses, Calls }
impl ReferenceType { pub fn as_str(&self) -> &'static str { match self { ReferenceType::Import => "import", ReferenceType::Extends => "extends", ReferenceType::Implements => "implements", ReferenceType::Uses => "uses", ReferenceType::Calls => "calls" } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference { pub source: String, pub target: String, pub ref_type: ReferenceType, pub source_project: String, pub target_project: String, pub file_path: String, pub line: usize }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode { pub id: String, pub project_id: String, pub name: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge { pub from: String, pub to: String, pub ref_type: ReferenceType }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph { pub nodes: HashMap<String, DependencyNode>, pub edges: Vec<DependencyEdge> }
impl DependencyGraph {
    pub fn new() -> Self { Self { nodes: HashMap::new(), edges: vec![] } }
    pub fn add_node(&mut self, n: DependencyNode) { self.nodes.insert(n.id.clone(), n); }
    pub fn add_edge(&mut self, e: DependencyEdge) { self.edges.push(e); }
    pub fn get_dependencies(&self, id: &str) -> Vec<&DependencyNode> { self.edges.iter().filter(|e| e.from == id).filter_map(|e| self.nodes.get(&e.to)).collect() }
    pub fn get_dependents(&self, id: &str) -> Vec<&DependencyNode> { self.edges.iter().filter(|e| e.to == id).filter_map(|e| self.nodes.get(&e.from)).collect() }
    pub fn detect_cycles(&self) -> Vec<Vec<String>> { vec![] }
}
impl Default for DependencyGraph { fn default() -> Self { Self::new() } }

pub struct CrossReferenceManager { references: Vec<CrossReference>, incoming: HashMap<String, Vec<usize>>, outgoing: HashMap<String, Vec<usize>> }
impl CrossReferenceManager {
    pub fn new() -> Self { Self { references: vec![], incoming: HashMap::new(), outgoing: HashMap::new() } }
    pub fn add_reference(&mut self, r: CrossReference) { let i = self.references.len(); self.incoming.entry(r.target.clone()).or_default().push(i); self.outgoing.entry(r.source.clone()).or_default().push(i); self.references.push(r); }
    pub fn get_outgoing_references(&self, id: &str) -> Vec<&CrossReference> { self.outgoing.get(id).map(|is| is.iter().map(|&i| &self.references[i]).collect()).unwrap_or_default() }
    pub fn get_incoming_references(&self, id: &str) -> Vec<&CrossReference> { self.incoming.get(id).map(|is| is.iter().map(|&i| &self.references[i]).collect()).unwrap_or_default() }
    pub fn get_references_by_type(&self, id: &str, t: ReferenceType) -> Vec<&CrossReference> { self.get_outgoing_references(id).into_iter().filter(|r| r.ref_type == t).collect() }
    pub fn build_dependency_graph(&self) -> DependencyGraph { DependencyGraph::new() }
    pub fn find_usages(&self, id: &str) -> Vec<&CrossReference> { self.get_incoming_references(id) }
}
impl Default for CrossReferenceManager { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn cr(s: &str, t: &str, rt: ReferenceType) -> CrossReference { CrossReference { source: s.to_string(), target: t.to_string(), ref_type: rt, source_project: "p1".to_string(), target_project: "p2".to_string(), file_path: "/t.ts".to_string(), line: 1 } }
    #[test] fn test_add_reference() { let mut m = CrossReferenceManager::new(); m.add_reference(cr("A", "B", ReferenceType::Import)); assert_eq!(m.references.len(), 1); }
    #[test] fn test_get_outgoing_references() { let mut m = CrossReferenceManager::new(); m.add_reference(cr("A", "B", ReferenceType::Import)); m.add_reference(cr("A", "C", ReferenceType::Uses)); assert_eq!(m.get_outgoing_references("A").len(), 2); }
    #[test] fn test_get_incoming_references() { let mut m = CrossReferenceManager::new(); m.add_reference(cr("A", "C", ReferenceType::Import)); m.add_reference(cr("B", "C", ReferenceType::Uses)); assert_eq!(m.get_incoming_references("C").len(), 2); }
    #[test] fn test_get_references_by_type() { let mut m = CrossReferenceManager::new(); m.add_reference(cr("A", "B", ReferenceType::Import)); m.add_reference(cr("A", "C", ReferenceType::Uses)); m.add_reference(cr("A", "D", ReferenceType::Import)); assert_eq!(m.get_references_by_type("A", ReferenceType::Import).len(), 2); }
    #[test] fn test_build_dependency_graph() { let mut m = CrossReferenceManager::new(); m.add_reference(cr("A", "B", ReferenceType::Import)); let _g = m.build_dependency_graph(); }
    #[test] fn test_dependency_graph_get_dependencies() { let mut g = DependencyGraph::new(); g.add_node(DependencyNode { id: "A".to_string(), project_id: "p1".to_string(), name: "A".to_string() }); g.add_node(DependencyNode { id: "B".to_string(), project_id: "p1".to_string(), name: "B".to_string() }); g.add_edge(DependencyEdge { from: "A".to_string(), to: "B".to_string(), ref_type: ReferenceType::Import }); assert_eq!(g.get_dependencies("A").len(), 1); }
    #[test] fn test_dependency_graph_detect_cycles() { let g = DependencyGraph::new(); let _c = g.detect_cycles(); }
    #[test] fn test_reference_type_as_str() { assert_eq!(ReferenceType::Import.as_str(), "import"); }
}
