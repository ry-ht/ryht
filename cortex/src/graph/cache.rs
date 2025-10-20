use crate::storage::Storage;
use crate::types::SymbolId;
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Edge type in the dependency graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    /// Type reference (e.g., function parameter type)
    TypeReference,
    /// Import/use statement
    Import,
    /// Function call
    Call,
    /// Inheritance/trait implementation
    Implements,
    /// Generic/other dependency
    Other,
}

/// Configuration for graph cache
#[derive(Debug, Clone)]
pub struct GraphCacheConfig {
    /// Maximum nodes to cache (default: 100,000)
    pub max_nodes: usize,
    /// Whether to enable cache (default: true)
    pub enabled: bool,
}

impl Default for GraphCacheConfig {
    fn default() -> Self {
        Self {
            max_nodes: 100_000,
            enabled: true,
        }
    }
}

/// Hybrid graph store: RocksDB persistence + in-memory petgraph cache
///
/// This provides the best of both worlds:
/// - RocksDB: Persistent, low memory, good for writes
/// - petgraph: In-memory, 10x faster reads for traversals
pub struct GraphCache {
    /// In-memory graph for fast traversals
    graph: Arc<RwLock<DiGraph<SymbolId, EdgeKind>>>,
    /// Mapping from SymbolId to graph NodeIndex
    node_map: Arc<RwLock<HashMap<SymbolId, NodeIndex>>>,
    /// Configuration
    config: GraphCacheConfig,
}

impl GraphCache {
    /// Create a new graph cache
    pub fn new(config: GraphCacheConfig) -> Self {
        info!("Initializing graph cache (max_nodes: {})", config.max_nodes);

        Self {
            graph: Arc::new(RwLock::new(DiGraph::new())),
            node_map: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Load graph from storage into memory
    ///
    /// This should be called on server startup to populate the cache
    pub async fn load_from_storage(&self, storage: Arc<dyn Storage>) -> Result<usize> {
        if !self.config.enabled {
            return Ok(0);
        }

        info!("Loading dependency graph into memory...");

        let mut graph = self.graph.write().await;
        let mut node_map = self.node_map.write().await;

        // Load all symbol IDs from storage
        // Format: "symbol:{id}" -> SymbolMetadata
        let keys = storage.get_keys_with_prefix(b"symbol:").await?;

        let mut loaded_nodes = 0;
        

        // First pass: create all nodes
        for key in &keys {
            if loaded_nodes >= self.config.max_nodes {
                warn!("Reached max_nodes limit ({}), stopping load", self.config.max_nodes);
                break;
            }

            // Extract symbol ID from key
            let key_str = String::from_utf8_lossy(key);
            if let Some(id_str) = key_str.strip_prefix("symbol:") {
                let symbol_id = SymbolId(id_str.to_string());

                // Add node to graph
                let node_index = graph.add_node(symbol_id.clone());
                node_map.insert(symbol_id, node_index);
                loaded_nodes += 1;
            }
        }

        // Second pass: create edges from dependencies
        // Dependencies stored as: "deps:{symbol_id}" -> serialized data
        // For now, skip loading edges as we don't have the exact storage format
        // This will be populated incrementally as symbols are indexed
        drop(graph);
        drop(node_map);

        // Re-acquire locks for stats
        let graph = self.graph.read().await;
        let loaded_edges = graph.edge_count();

        info!(
            "Graph cache loaded: {} nodes, {} edges ({}MB estimated)",
            loaded_nodes,
            loaded_edges,
            (loaded_nodes * 100 + loaded_edges * 24) / 1_000_000
        );

        Ok(loaded_nodes)
    }

    /// Get all dependencies of a symbol up to specified depth (BFS traversal)
    ///
    /// Performance: O(V + E) in-memory, vs O(V * read_latency) from storage
    /// Expected: ~5ms for 3-hop vs ~50ms from RocksDB
    pub async fn get_dependencies(
        &self,
        symbol_id: &SymbolId,
        max_depth: usize,
    ) -> Result<Vec<SymbolId>> {
        let graph = self.graph.read().await;
        let node_map = self.node_map.read().await;

        let start_node = match node_map.get(symbol_id) {
            Some(&node) => node,
            None => {
                debug!("Symbol {} not found in graph cache", symbol_id);
                return Ok(vec![]);
            }
        };

        // BFS traversal with depth tracking
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Add all outgoing edges (dependencies)
            for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let dep_id = &graph[neighbor];
                    result.push(dep_id.clone());
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Get all symbols that depend on this symbol (reverse dependencies)
    pub async fn get_dependents(
        &self,
        symbol_id: &SymbolId,
        max_depth: usize,
    ) -> Result<Vec<SymbolId>> {
        let graph = self.graph.read().await;
        let node_map = self.node_map.read().await;

        let start_node = match node_map.get(symbol_id) {
            Some(&node) => node,
            None => return Ok(vec![]),
        };

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        queue.push_back((start_node, 0));
        visited.insert(start_node);

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Add all incoming edges (dependents)
            for neighbor in graph.neighbors_directed(node, Direction::Incoming) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let dependent_id = &graph[neighbor];
                    result.push(dependent_id.clone());
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Check if there's a path from source to target
    pub async fn has_path(&self, from: &SymbolId, to: &SymbolId) -> Result<bool> {
        let graph = self.graph.read().await;
        let node_map = self.node_map.read().await;

        let from_node = match node_map.get(from) {
            Some(&node) => node,
            None => return Ok(false),
        };

        let to_node = match node_map.get(to) {
            Some(&node) => node,
            None => return Ok(false),
        };

        // BFS to check reachability
        let mut bfs = Bfs::new(&*graph, from_node);
        while let Some(node) = bfs.next(&*graph) {
            if node == to_node {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Find shortest path between two symbols
    pub async fn shortest_path(
        &self,
        from: &SymbolId,
        to: &SymbolId,
    ) -> Result<Option<Vec<SymbolId>>> {
        use petgraph::algo::dijkstra;

        let graph = self.graph.read().await;
        let node_map = self.node_map.read().await;

        let from_node = match node_map.get(from) {
            Some(&node) => node,
            None => return Ok(None),
        };

        let to_node = match node_map.get(to) {
            Some(&node) => node,
            None => return Ok(None),
        };

        // Dijkstra with unit weights
        let distances = dijkstra(&*graph, from_node, Some(to_node), |_| 1);

        if !distances.contains_key(&to_node) {
            return Ok(None);
        }

        // Reconstruct path (simplified - just return reachable status for now)
        // TODO: Implement full path reconstruction if needed
        Ok(Some(vec![from.clone(), to.clone()]))
    }

    /// Get graph statistics
    pub async fn stats(&self) -> (usize, usize) {
        let graph = self.graph.read().await;
        (graph.node_count(), graph.edge_count())
    }

    /// Add a node to the cache
    pub async fn add_node(&self, symbol_id: SymbolId) -> Result<()> {
        let mut graph = self.graph.write().await;
        let mut node_map = self.node_map.write().await;

        if node_map.contains_key(&symbol_id) {
            return Ok(()); // Already exists
        }

        if node_map.len() >= self.config.max_nodes {
            warn!("Graph cache full ({}), not adding node", self.config.max_nodes);
            return Ok(());
        }

        let node_index = graph.add_node(symbol_id.clone());
        node_map.insert(symbol_id, node_index);

        Ok(())
    }

    /// Add an edge to the cache
    pub async fn add_edge(
        &self,
        from: &SymbolId,
        to: &SymbolId,
        kind: EdgeKind,
    ) -> Result<()> {
        let mut graph = self.graph.write().await;
        let node_map = self.node_map.read().await;

        let from_node = match node_map.get(from) {
            Some(&node) => node,
            None => {
                warn!("Cannot add edge: source node {} not in cache", from);
                return Ok(());
            }
        };

        let to_node = match node_map.get(to) {
            Some(&node) => node,
            None => {
                warn!("Cannot add edge: target node {} not in cache", to);
                return Ok(());
            }
        };

        graph.add_edge(from_node, to_node, kind);
        Ok(())
    }

    /// Clear the cache
    pub async fn clear(&self) {
        let mut graph = self.graph.write().await;
        let mut node_map = self.node_map.write().await;

        graph.clear();
        node_map.clear();

        info!("Graph cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_graph_cache_basic() {
        let cache = GraphCache::new(GraphCacheConfig::default());

        let id1 = SymbolId::new("symbol1");
        let id2 = SymbolId::new("symbol2");
        let id3 = SymbolId::new("symbol3");

        // Add nodes
        cache.add_node(id1.clone()).await.unwrap();
        cache.add_node(id2.clone()).await.unwrap();
        cache.add_node(id3.clone()).await.unwrap();

        // Add edges: 1 → 2 → 3
        cache.add_edge(&id1, &id2, EdgeKind::TypeReference).await.unwrap();
        cache.add_edge(&id2, &id3, EdgeKind::TypeReference).await.unwrap();

        // Test dependencies
        let deps = cache.get_dependencies(&id1, 2).await.unwrap();
        assert_eq!(deps.len(), 2); // Should find id2 and id3

        // Test path existence
        assert!(cache.has_path(&id1, &id3).await.unwrap());
        assert!(!cache.has_path(&id3, &id1).await.unwrap());

        // Test stats
        let (nodes, edges) = cache.stats().await;
        assert_eq!(nodes, 3);
        assert_eq!(edges, 2);
    }

    #[tokio::test]
    async fn test_get_dependents() {
        let cache = GraphCache::new(GraphCacheConfig::default());

        let id1 = SymbolId::new("symbol1");
        let id2 = SymbolId::new("symbol2");
        let id3 = SymbolId::new("symbol3");

        cache.add_node(id1.clone()).await.unwrap();
        cache.add_node(id2.clone()).await.unwrap();
        cache.add_node(id3.clone()).await.unwrap();

        // 1 → 3, 2 → 3 (both depend on 3)
        cache.add_edge(&id1, &id3, EdgeKind::TypeReference).await.unwrap();
        cache.add_edge(&id2, &id3, EdgeKind::TypeReference).await.unwrap();

        // Get dependents of id3
        let dependents = cache.get_dependents(&id3, 1).await.unwrap();
        assert_eq!(dependents.len(), 2); // Should find id1 and id2
    }
}
