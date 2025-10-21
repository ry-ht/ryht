//! Graph algorithms for dependency analysis
//!
//! Implements efficient graph algorithms for analyzing code dependencies:
//! - BFS for shortest path finding
//! - Tarjan's algorithm for strongly connected components (cycles)
//! - Topological sorting for layering
//! - Centrality measures for hub detection

use std::collections::{HashMap, HashSet, VecDeque};

/// Result of a shortest path search
#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<String>,
    pub length: usize,
}

/// Strongly connected component (cycle)
pub type Cycle = Vec<String>;

/// Layer in topological ordering
pub type Layer = Vec<String>;

/// Graph represented as adjacency lists
#[derive(Debug, Clone)]
pub struct Graph {
    /// Forward edges: node -> list of nodes it depends on
    pub adjacency: HashMap<String, Vec<String>>,
    /// Reverse edges: node -> list of nodes that depend on it
    pub reverse_adjacency: HashMap<String, Vec<String>>,
    /// All nodes in the graph
    pub nodes: HashSet<String>,
}

impl Graph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            reverse_adjacency: HashMap::new(),
            nodes: HashSet::new(),
        }
    }

    /// Add an edge from -> to
    pub fn add_edge(&mut self, from: String, to: String) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());

        self.adjacency
            .entry(from.clone())
            .or_insert_with(Vec::new)
            .push(to.clone());

        self.reverse_adjacency
            .entry(to)
            .or_insert_with(Vec::new)
            .push(from);
    }

    /// Get outgoing neighbors
    pub fn neighbors(&self, node: &str) -> &[String] {
        self.adjacency
            .get(node)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get incoming neighbors (reverse dependencies)
    pub fn reverse_neighbors(&self, node: &str) -> &[String] {
        self.reverse_adjacency
            .get(node)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get in-degree of a node
    pub fn in_degree(&self, node: &str) -> usize {
        self.reverse_adjacency
            .get(node)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get out-degree of a node
    pub fn out_degree(&self, node: &str) -> usize {
        self.adjacency
            .get(node)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get total degree (in + out)
    pub fn total_degree(&self, node: &str) -> usize {
        self.in_degree(node) + self.out_degree(node)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// Find shortest path between two nodes using BFS
pub fn find_shortest_path(graph: &Graph, from: &str, to: &str) -> Option<Path> {
    if from == to {
        return Some(Path {
            nodes: vec![from.to_string()],
            length: 0,
        });
    }

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();

    queue.push_back(from.to_string());
    visited.insert(from.to_string());

    while let Some(current) = queue.pop_front() {
        if current == to {
            // Reconstruct path
            let mut path = vec![to.to_string()];
            let mut node = to;

            while let Some(p) = parent.get(node) {
                path.push(p.clone());
                node = p;
            }

            path.reverse();
            let length = path.len() - 1;

            return Some(Path { nodes: path, length });
        }

        for neighbor in graph.neighbors(&current) {
            if !visited.contains(neighbor) {
                visited.insert(neighbor.clone());
                parent.insert(neighbor.clone(), current.clone());
                queue.push_back(neighbor.clone());
            }
        }
    }

    None
}

/// Find all paths up to max_length (useful for finding all paths, not just shortest)
pub fn find_all_paths(
    graph: &Graph,
    from: &str,
    to: &str,
    max_length: usize,
) -> Vec<Path> {
    let mut paths = Vec::new();
    let mut current_path = vec![from.to_string()];
    let mut visited = HashSet::new();
    visited.insert(from.to_string());

    fn dfs(
        graph: &Graph,
        current: &str,
        to: &str,
        max_length: usize,
        current_path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        paths: &mut Vec<Path>,
    ) {
        if current == to {
            paths.push(Path {
                nodes: current_path.clone(),
                length: current_path.len() - 1,
            });
            return;
        }

        if current_path.len() > max_length {
            return;
        }

        for neighbor in graph.neighbors(current) {
            if !visited.contains(neighbor) {
                visited.insert(neighbor.clone());
                current_path.push(neighbor.clone());

                dfs(graph, neighbor, to, max_length, current_path, visited, paths);

                current_path.pop();
                visited.remove(neighbor);
            }
        }
    }

    dfs(graph, from, to, max_length, &mut current_path, &mut visited, &mut paths);
    paths
}

/// Find all strongly connected components using Tarjan's algorithm
/// Returns cycles (SCCs with more than one node or self-loops)
pub fn find_cycles(graph: &Graph) -> Vec<Cycle> {
    let mut index = 0;
    let mut stack = Vec::new();
    let mut indices = HashMap::new();
    let mut low_links = HashMap::new();
    let mut on_stack = HashSet::new();
    let mut sccs = Vec::new();

    fn strong_connect(
        node: &str,
        graph: &Graph,
        index: &mut usize,
        stack: &mut Vec<String>,
        indices: &mut HashMap<String, usize>,
        low_links: &mut HashMap<String, usize>,
        on_stack: &mut HashSet<String>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(node.to_string(), *index);
        low_links.insert(node.to_string(), *index);
        *index += 1;
        stack.push(node.to_string());
        on_stack.insert(node.to_string());

        for neighbor in graph.neighbors(node) {
            if !indices.contains_key(neighbor) {
                strong_connect(
                    neighbor,
                    graph,
                    index,
                    stack,
                    indices,
                    low_links,
                    on_stack,
                    sccs,
                );
                let neighbor_low = *low_links.get(neighbor).unwrap();
                let current_low = *low_links.get(node).unwrap();
                low_links.insert(node.to_string(), current_low.min(neighbor_low));
            } else if on_stack.contains(neighbor) {
                let neighbor_index = *indices.get(neighbor).unwrap();
                let current_low = *low_links.get(node).unwrap();
                low_links.insert(node.to_string(), current_low.min(neighbor_index));
            }
        }

        let node_low = *low_links.get(node).unwrap();
        let node_index = *indices.get(node).unwrap();

        if node_low == node_index {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                scc.push(w.clone());
                if w == node {
                    break;
                }
            }

            // Only include if it's a real cycle (size > 1) or has self-loop
            if scc.len() > 1 || graph.neighbors(node).contains(&node.to_string()) {
                sccs.push(scc);
            }
        }
    }

    for node in &graph.nodes {
        if !indices.contains_key(node) {
            strong_connect(
                node,
                graph,
                &mut index,
                &mut stack,
                &mut indices,
                &mut low_links,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    sccs
}

/// Topological sort with layering
/// Returns layers where each layer only depends on previous layers
pub fn topological_layers(graph: &Graph) -> Vec<Layer> {
    let mut in_degree = HashMap::new();
    let mut layers = Vec::new();

    // Calculate in-degrees
    for node in &graph.nodes {
        in_degree.insert(node.clone(), graph.in_degree(node));
    }

    loop {
        // Find all nodes with in-degree 0
        let mut current_layer: Vec<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(node, _)| node.clone())
            .collect();

        if current_layer.is_empty() {
            break;
        }

        current_layer.sort();
        layers.push(current_layer.clone());

        // Remove these nodes and update in-degrees
        for node in &current_layer {
            in_degree.remove(node);
            for neighbor in graph.neighbors(node) {
                if let Some(degree) = in_degree.get_mut(neighbor) {
                    *degree = degree.saturating_sub(1);
                }
            }
        }
    }

    layers
}

/// Calculate betweenness centrality (approximate for large graphs)
pub fn calculate_centrality(graph: &Graph) -> HashMap<String, f64> {
    let mut centrality = HashMap::new();

    // Initialize all centrality scores to 0
    for node in &graph.nodes {
        centrality.insert(node.clone(), 0.0);
    }

    // For each node as source
    for source in &graph.nodes {
        let mut stack = Vec::new();
        let mut paths = HashMap::new();
        let mut sigma = HashMap::new();
        let mut distance = HashMap::new();
        let mut delta = HashMap::new();

        for node in &graph.nodes {
            paths.insert(node.clone(), Vec::new());
            sigma.insert(node.clone(), 0.0);
            distance.insert(node.clone(), -1);
            delta.insert(node.clone(), 0.0);
        }

        sigma.insert(source.clone(), 1.0);
        distance.insert(source.clone(), 0);

        let mut queue = VecDeque::new();
        queue.push_back(source.clone());

        // BFS
        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            let v_dist = *distance.get(&v).unwrap();
            let v_sigma = *sigma.get(&v).unwrap();

            for w in graph.neighbors(&v) {
                let w_dist = *distance.get(w).unwrap();

                // First visit to w?
                if w_dist < 0 {
                    queue.push_back(w.clone());
                    distance.insert(w.clone(), v_dist + 1);
                }

                // Shortest path to w via v?
                if w_dist == v_dist + 1 {
                    sigma.insert(w.clone(), sigma.get(w).unwrap() + v_sigma);
                    paths.get_mut(w).unwrap().push(v.clone());
                }
            }
        }

        // Accumulation
        while let Some(w) = stack.pop() {
            for v in paths.get(&w).unwrap() {
                let v_sigma = *sigma.get(v).unwrap();
                let w_sigma = *sigma.get(&w).unwrap();
                let w_delta = *delta.get(&w).unwrap();

                let contrib = (v_sigma / w_sigma) * (1.0 + w_delta);
                delta.insert(v.clone(), delta.get(v).unwrap() + contrib);
            }

            if &w != source {
                let w_delta = *delta.get(&w).unwrap();
                centrality.insert(w.clone(), centrality.get(&w).unwrap() + w_delta);
            }
        }
    }

    // Normalize by dividing by 2 (each edge counted twice)
    for value in centrality.values_mut() {
        *value /= 2.0;
    }

    centrality
}

/// Find all nodes reachable from a starting node (for impact analysis)
pub fn find_reachable(graph: &Graph, start: &str, max_depth: Option<usize>) -> HashMap<String, usize> {
    let mut reachable = HashMap::new();
    let mut queue = VecDeque::new();

    queue.push_back((start.to_string(), 0));
    reachable.insert(start.to_string(), 0);

    while let Some((node, depth)) = queue.pop_front() {
        if let Some(max) = max_depth {
            if depth >= max {
                continue;
            }
        }

        for neighbor in graph.reverse_neighbors(&node) {
            if !reachable.contains_key(neighbor) {
                reachable.insert(neighbor.clone(), depth + 1);
                queue.push_back((neighbor.clone(), depth + 1));
            }
        }
    }

    reachable
}

/// Find root nodes (no incoming edges)
pub fn find_roots(graph: &Graph) -> Vec<String> {
    graph
        .nodes
        .iter()
        .filter(|node| graph.in_degree(node) == 0)
        .cloned()
        .collect()
}

/// Find leaf nodes (no outgoing edges)
pub fn find_leaves(graph: &Graph) -> Vec<String> {
    graph
        .nodes
        .iter()
        .filter(|node| graph.out_degree(node) == 0)
        .cloned()
        .collect()
}

/// Find hub nodes (highly connected)
pub fn find_hubs(graph: &Graph, min_connections: usize) -> Vec<(String, usize, usize, usize)> {
    let mut hubs: Vec<_> = graph
        .nodes
        .iter()
        .map(|node| {
            let in_deg = graph.in_degree(node);
            let out_deg = graph.out_degree(node);
            let total = in_deg + out_deg;
            (node.clone(), in_deg, out_deg, total)
        })
        .filter(|(_, _, _, total)| *total >= min_connections)
        .collect();

    // Sort by total degree descending
    hubs.sort_by(|a, b| b.3.cmp(&a.3));
    hubs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> Graph {
        let mut graph = Graph::new();
        graph.add_edge("A".to_string(), "B".to_string());
        graph.add_edge("B".to_string(), "C".to_string());
        graph.add_edge("C".to_string(), "D".to_string());
        graph.add_edge("A".to_string(), "D".to_string());
        graph
    }

    fn create_cycle_graph() -> Graph {
        let mut graph = Graph::new();
        graph.add_edge("A".to_string(), "B".to_string());
        graph.add_edge("B".to_string(), "C".to_string());
        graph.add_edge("C".to_string(), "A".to_string());
        graph
    }

    #[test]
    fn test_shortest_path() {
        let graph = create_test_graph();
        let path = find_shortest_path(&graph, "A", "D");
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.length, 1); // Direct edge A->D
        assert_eq!(path.nodes, vec!["A", "D"]);
    }

    #[test]
    fn test_shortest_path_indirect() {
        let graph = create_test_graph();
        let path = find_shortest_path(&graph, "A", "C");
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.length, 2); // A->B->C
    }

    #[test]
    fn test_no_path() {
        let graph = create_test_graph();
        let path = find_shortest_path(&graph, "D", "A");
        assert!(path.is_none());
    }

    #[test]
    fn test_find_cycles() {
        let graph = create_cycle_graph();
        let cycles = find_cycles(&graph);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_no_cycles() {
        let graph = create_test_graph();
        let cycles = find_cycles(&graph);
        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_topological_layers() {
        let graph = create_test_graph();
        let layers = topological_layers(&graph);

        assert!(!layers.is_empty());
        // A should be in first layer (no dependencies)
        assert!(layers[0].contains(&"A".to_string()));
    }

    #[test]
    fn test_find_roots() {
        let graph = create_test_graph();
        let roots = find_roots(&graph);
        assert_eq!(roots.len(), 1);
        assert!(roots.contains(&"A".to_string()));
    }

    #[test]
    fn test_find_leaves() {
        let graph = create_test_graph();
        let leaves = find_leaves(&graph);
        assert_eq!(leaves.len(), 1);
        assert!(leaves.contains(&"D".to_string()));
    }

    #[test]
    fn test_find_hubs() {
        let mut graph = Graph::new();
        // Create a hub at B
        graph.add_edge("A".to_string(), "B".to_string());
        graph.add_edge("C".to_string(), "B".to_string());
        graph.add_edge("D".to_string(), "B".to_string());
        graph.add_edge("B".to_string(), "E".to_string());
        graph.add_edge("B".to_string(), "F".to_string());

        let hubs = find_hubs(&graph, 3);
        assert!(!hubs.is_empty());
        assert_eq!(hubs[0].0, "B"); // B should be the top hub
        assert_eq!(hubs[0].3, 5); // Total degree = 5
    }

    #[test]
    fn test_find_reachable() {
        let graph = create_test_graph();
        let reachable = find_reachable(&graph, "D", None);

        // From D, we can reach A (reverse direction)
        assert!(reachable.contains_key("A"));
    }

    #[test]
    fn test_in_out_degree() {
        let graph = create_test_graph();
        assert_eq!(graph.out_degree("A"), 2); // A->B, A->D
        assert_eq!(graph.in_degree("D"), 2); // C->D, A->D
    }

    #[test]
    fn test_all_paths() {
        let graph = create_test_graph();
        let paths = find_all_paths(&graph, "A", "D", 5);

        assert!(paths.len() >= 2); // At least A->D and A->B->C->D
    }
}
