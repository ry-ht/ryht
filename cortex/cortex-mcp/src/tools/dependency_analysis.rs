//! Dependency Analysis Tools (10 tools)
//!
//! Comprehensive dependency analysis using graph algorithms:
//! - Find dependencies and dependents
//! - Shortest path analysis
//! - Cycle detection
//! - Impact analysis
//! - Architectural layering
//! - Hub detection
//! - Constraint checking
//! - Graph visualization

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};
use std::collections::HashMap;

use crate::graph_algorithms::{
    Graph, find_shortest_path, find_all_paths, find_cycles, topological_layers,
    find_reachable, find_roots, find_leaves, find_hubs, calculate_centrality,
};

#[derive(Clone)]
pub struct DependencyAnalysisContext {
    storage: Arc<ConnectionManager>,
}

impl DependencyAnalysisContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Build graph from database dependencies
    async fn build_graph(&self, scope_path: Option<&str>) -> std::result::Result<Graph, String> {
        // TODO: Query database for dependencies
        // For now, return empty graph
        let _ = scope_path;
        Ok(Graph::new())
    }

    /// Get unit name from ID
    async fn get_unit_name(&self, entity_id: &str) -> std::result::Result<String, String> {
        // TODO: Query database for unit name
        Ok(entity_id.to_string())
    }
}

// ============================================================================
// 1. GET DEPENDENCIES
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDependenciesInput {
    entity_id: String,
    #[serde(default = "default_outgoing")]
    direction: String,
    dependency_types: Option<Vec<String>>,
    #[serde(default = "default_depth_one")]
    max_depth: i32,
    #[serde(default)]
    include_transitive: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetDependenciesOutput {
    entity_id: String,
    dependencies: Vec<Dependency>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct Dependency {
    target_id: String,
    dependency_type: String,
    depth: i32,
    location: Option<String>,
}

pub struct DepsGetDependenciesTool {
    ctx: DependencyAnalysisContext,
}

impl DepsGetDependenciesTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: GetDependenciesInput) -> std::result::Result<GetDependenciesOutput, String> {
        debug!("Getting dependencies for entity: {}", input.entity_id);

        let graph = self.ctx.build_graph(None).await?;
        let mut dependencies = Vec::new();

        if input.direction == "outgoing" {
            // Get what this entity depends on
            if input.include_transitive && input.max_depth != 1 {
                let max_depth = if input.max_depth < 0 {
                    None
                } else {
                    Some(input.max_depth as usize)
                };

                // BFS to find all reachable dependencies
                let mut queue = std::collections::VecDeque::new();
                let mut visited = std::collections::HashSet::new();

                queue.push_back((input.entity_id.clone(), 0));
                visited.insert(input.entity_id.clone());

                while let Some((node, depth)) = queue.pop_front() {
                    if let Some(max) = max_depth {
                        if depth >= max as i32 {
                            continue;
                        }
                    }

                    for neighbor in graph.neighbors(&node) {
                        if !visited.contains(neighbor) {
                            visited.insert(neighbor.clone());
                            dependencies.push(Dependency {
                                target_id: neighbor.clone(),
                                dependency_type: "DEPENDS_ON".to_string(),
                                depth: depth + 1,
                                location: None,
                            });
                            queue.push_back((neighbor.clone(), depth + 1));
                        }
                    }
                }
            } else {
                // Direct dependencies only
                for neighbor in graph.neighbors(&input.entity_id) {
                    dependencies.push(Dependency {
                        target_id: neighbor.clone(),
                        dependency_type: "DEPENDS_ON".to_string(),
                        depth: 1,
                        location: None,
                    });
                }
            }
        } else {
            // Get what depends on this entity (reverse dependencies)
            if input.include_transitive && input.max_depth != 1 {
                let reachable = find_reachable(
                    &graph,
                    &input.entity_id,
                    if input.max_depth < 0 {
                        None
                    } else {
                        Some(input.max_depth as usize)
                    },
                );

                for (node, depth) in reachable {
                    if node != input.entity_id {
                        dependencies.push(Dependency {
                            target_id: node,
                            dependency_type: "DEPENDED_ON_BY".to_string(),
                            depth: depth as i32,
                            location: None,
                        });
                    }
                }
            } else {
                // Direct dependents only
                for neighbor in graph.reverse_neighbors(&input.entity_id) {
                    dependencies.push(Dependency {
                        target_id: neighbor.clone(),
                        dependency_type: "DEPENDED_ON_BY".to_string(),
                        depth: 1,
                        location: None,
                    });
                }
            }
        }

        Ok(GetDependenciesOutput {
            entity_id: input.entity_id,
            total_count: dependencies.len() as i32,
            dependencies,
        })
    }
}

#[async_trait]
impl Tool for DepsGetDependenciesTool {
    fn name(&self) -> &str {
        "cortex.deps.get_dependencies"
    }

    fn description(&self) -> Option<&str> {
        Some("Get dependencies of a code unit (functions, types, etc.) with optional transitive resolution")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetDependenciesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GetDependenciesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 2. FIND PATH
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindPathInput {
    from_id: String,
    to_id: String,
    #[serde(default = "default_max_depth")]
    max_depth: i32,
    #[serde(default = "default_shortest")]
    path_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindPathOutput {
    paths: Vec<Vec<String>>,
    shortest_length: i32,
}

pub struct DepsFindPathTool {
    ctx: DependencyAnalysisContext,
}

impl DepsFindPathTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: FindPathInput) -> std::result::Result<FindPathOutput, String> {
        debug!("Finding path from {} to {}", input.from_id, input.to_id);

        let graph = self.ctx.build_graph(None).await?;

        if input.path_type == "shortest" {
            if let Some(path) = find_shortest_path(&graph, &input.from_id, &input.to_id) {
                Ok(FindPathOutput {
                    paths: vec![path.nodes],
                    shortest_length: path.length as i32,
                })
            } else {
                Ok(FindPathOutput {
                    paths: vec![],
                    shortest_length: -1,
                })
            }
        } else {
            // Find all paths
            let paths = find_all_paths(
                &graph,
                &input.from_id,
                &input.to_id,
                input.max_depth as usize,
            );

            let shortest_length = paths
                .iter()
                .map(|p| p.length)
                .min()
                .unwrap_or(0) as i32;

            Ok(FindPathOutput {
                paths: paths.into_iter().map(|p| p.nodes).collect(),
                shortest_length,
            })
        }
    }
}

#[async_trait]
impl Tool for DepsFindPathTool {
    fn name(&self) -> &str {
        "cortex.deps.find_path"
    }

    fn description(&self) -> Option<&str> {
        Some("Find dependency path between two code units using BFS")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindPathInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindPathInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 3. FIND CYCLES
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindCyclesInput {
    scope_path: Option<String>,
    #[serde(default = "default_max_depth")]
    max_cycle_length: i32,
    #[serde(default = "default_file_level")]
    entity_level: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindCyclesOutput {
    cycles: Vec<Vec<String>>,
    total_cycles: i32,
}

pub struct DepsFindCyclesTool {
    ctx: DependencyAnalysisContext,
}

impl DepsFindCyclesTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: FindCyclesInput) -> std::result::Result<FindCyclesOutput, String> {
        debug!("Finding cycles in scope: {:?}", input.scope_path);

        let graph = self.ctx.build_graph(input.scope_path.as_deref()).await?;
        let mut cycles = find_cycles(&graph);

        // Filter by max cycle length if specified
        if input.max_cycle_length > 0 {
            cycles.retain(|cycle| cycle.len() <= input.max_cycle_length as usize);
        }

        Ok(FindCyclesOutput {
            total_cycles: cycles.len() as i32,
            cycles,
        })
    }
}

#[async_trait]
impl Tool for DepsFindCyclesTool {
    fn name(&self) -> &str {
        "cortex.deps.find_cycles"
    }

    fn description(&self) -> Option<&str> {
        Some("Detect circular dependencies using Tarjan's strongly connected components algorithm")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindCyclesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindCyclesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 4. IMPACT ANALYSIS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImpactAnalysisInput {
    changed_entities: Vec<String>,
    impact_types: Option<Vec<String>>,
    #[serde(default = "default_all_depth")]
    max_depth: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ImpactAnalysisOutput {
    impacted_entities: Vec<ImpactedEntity>,
    total_impact: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct ImpactedEntity {
    entity_id: String,
    impact_type: String,
    distance: i32,
}

pub struct DepsImpactAnalysisTool {
    ctx: DependencyAnalysisContext,
}

impl DepsImpactAnalysisTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: ImpactAnalysisInput) -> std::result::Result<ImpactAnalysisOutput, String> {
        debug!("Analyzing impact for {} entities", input.changed_entities.len());

        let graph = self.ctx.build_graph(None).await?;
        let mut all_impacted = HashMap::new();

        // For each changed entity, find all entities that depend on it
        for entity in &input.changed_entities {
            let max_depth = if input.max_depth < 0 {
                None
            } else {
                Some(input.max_depth as usize)
            };

            let reachable = find_reachable(&graph, entity, max_depth);

            for (node, distance) in reachable {
                if !input.changed_entities.contains(&node) {
                    all_impacted
                        .entry(node.clone())
                        .and_modify(|d: &mut usize| *d = (*d).min(distance))
                        .or_insert(distance);
                }
            }
        }

        let impacted_entities: Vec<_> = all_impacted
            .into_iter()
            .map(|(entity_id, distance)| ImpactedEntity {
                entity_id,
                impact_type: "TRANSITIVE_DEPENDENT".to_string(),
                distance: distance as i32,
            })
            .collect();

        Ok(ImpactAnalysisOutput {
            total_impact: impacted_entities.len() as i32,
            impacted_entities,
        })
    }
}

#[async_trait]
impl Tool for DepsImpactAnalysisTool {
    fn name(&self) -> &str {
        "cortex.deps.impact_analysis"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze impact of changes by finding all units affected by modifying given units")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ImpactAnalysisInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ImpactAnalysisInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 5. FIND ROOTS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindRootsInput {
    scope_path: String,
    #[serde(default = "default_file_level")]
    entity_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindRootsOutput {
    root_entities: Vec<String>,
    total_count: i32,
}

pub struct DepsFindRootsTool {
    ctx: DependencyAnalysisContext,
}

impl DepsFindRootsTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: FindRootsInput) -> std::result::Result<FindRootsOutput, String> {
        debug!("Finding root entities in scope: {}", input.scope_path);

        let graph = self.ctx.build_graph(Some(&input.scope_path)).await?;
        let root_entities = find_roots(&graph);

        Ok(FindRootsOutput {
            total_count: root_entities.len() as i32,
            root_entities,
        })
    }
}

#[async_trait]
impl Tool for DepsFindRootsTool {
    fn name(&self) -> &str {
        "cortex.deps.find_roots"
    }

    fn description(&self) -> Option<&str> {
        Some("Find root entities with no incoming dependencies (in-degree = 0)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindRootsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindRootsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 6. FIND LEAVES
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindLeavesInput {
    scope_path: String,
    #[serde(default = "default_file_level")]
    entity_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindLeavesOutput {
    leaf_entities: Vec<String>,
    total_count: i32,
}

pub struct DepsFindLeavesTool {
    ctx: DependencyAnalysisContext,
}

impl DepsFindLeavesTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: FindLeavesInput) -> std::result::Result<FindLeavesOutput, String> {
        debug!("Finding leaf entities in scope: {}", input.scope_path);

        let graph = self.ctx.build_graph(Some(&input.scope_path)).await?;
        let leaf_entities = find_leaves(&graph);

        Ok(FindLeavesOutput {
            total_count: leaf_entities.len() as i32,
            leaf_entities,
        })
    }
}

#[async_trait]
impl Tool for DepsFindLeavesTool {
    fn name(&self) -> &str {
        "cortex.deps.find_leaves"
    }

    fn description(&self) -> Option<&str> {
        Some("Find leaf entities with no outgoing dependencies (out-degree = 0)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindLeavesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindLeavesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 7. FIND HUBS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindHubsInput {
    scope_path: Option<String>,
    #[serde(default = "default_min_connections")]
    min_connections: i32,
    #[serde(default = "default_total")]
    connection_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindHubsOutput {
    hubs: Vec<HubEntity>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct HubEntity {
    entity_id: String,
    incoming_count: i32,
    outgoing_count: i32,
    total_count: i32,
}

pub struct DepsFindHubsTool {
    ctx: DependencyAnalysisContext,
}

impl DepsFindHubsTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: FindHubsInput) -> std::result::Result<FindHubsOutput, String> {
        debug!("Finding hub entities with min_connections: {}", input.min_connections);

        let graph = self.ctx.build_graph(input.scope_path.as_deref()).await?;
        let hubs_data = find_hubs(&graph, input.min_connections as usize);

        let hubs: Vec<_> = hubs_data
            .into_iter()
            .map(|(entity_id, in_deg, out_deg, total)| HubEntity {
                entity_id,
                incoming_count: in_deg as i32,
                outgoing_count: out_deg as i32,
                total_count: total as i32,
            })
            .collect();

        Ok(FindHubsOutput {
            total_count: hubs.len() as i32,
            hubs,
        })
    }
}

#[async_trait]
impl Tool for DepsFindHubsTool {
    fn name(&self) -> &str {
        "cortex.deps.find_hubs"
    }

    fn description(&self) -> Option<&str> {
        Some("Find highly connected entities (hubs) sorted by total degree")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindHubsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindHubsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 8. GET LAYERS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLayersInput {
    scope_path: String,
    #[serde(default = "default_true")]
    detect_violations: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetLayersOutput {
    layers: Vec<Layer>,
    violations: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct Layer {
    layer_id: i32,
    entities: Vec<String>,
}

pub struct DepsGetLayersTool {
    ctx: DependencyAnalysisContext,
}

impl DepsGetLayersTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: GetLayersInput) -> std::result::Result<GetLayersOutput, String> {
        debug!("Computing architectural layers for scope: {}", input.scope_path);

        let graph = self.ctx.build_graph(Some(&input.scope_path)).await?;
        let layer_data = topological_layers(&graph);

        let layers: Vec<_> = layer_data
            .into_iter()
            .enumerate()
            .map(|(idx, entities)| Layer {
                layer_id: idx as i32,
                entities,
            })
            .collect();

        let mut violations = Vec::new();

        if input.detect_violations {
            // Detect layering violations (edges from higher to lower layers)
            let layer_map: HashMap<String, i32> = layers
                .iter()
                .flat_map(|layer| {
                    layer
                        .entities
                        .iter()
                        .map(move |e| (e.clone(), layer.layer_id))
                })
                .collect();

            for node in &graph.nodes {
                if let Some(&node_layer) = layer_map.get(node) {
                    for neighbor in graph.neighbors(node) {
                        if let Some(&neighbor_layer) = layer_map.get(neighbor) {
                            if neighbor_layer <= node_layer {
                                violations.push(format!(
                                    "Layer violation: {} (layer {}) -> {} (layer {})",
                                    node, node_layer, neighbor, neighbor_layer
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(GetLayersOutput { layers, violations })
    }
}

#[async_trait]
impl Tool for DepsGetLayersTool {
    fn name(&self) -> &str {
        "cortex.deps.get_layers"
    }

    fn description(&self) -> Option<&str> {
        Some("Get architectural layers using topological sorting with violation detection")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetLayersInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GetLayersInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 9. CHECK CONSTRAINTS
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckConstraintsInput {
    constraints: Vec<DependencyConstraint>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct DependencyConstraint {
    from_pattern: String,
    to_pattern: String,
    allowed: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CheckConstraintsOutput {
    violations: Vec<ConstraintViolation>,
    total_violations: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default, Clone)]
pub struct ConstraintViolation {
    from_entity: String,
    to_entity: String,
    constraint_violated: String,
}

pub struct DepsCheckConstraintsTool {
    ctx: DependencyAnalysisContext,
}

impl DepsCheckConstraintsTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: CheckConstraintsInput) -> std::result::Result<CheckConstraintsOutput, String> {
        debug!("Checking {} constraints", input.constraints.len());

        let graph = self.ctx.build_graph(None).await?;
        let mut violations = Vec::new();

        for constraint in &input.constraints {
            // Convert pattern to regex
            let from_regex = regex::Regex::new(&constraint.from_pattern.replace("*", ".*"))
                .map_err(|e| format!("Invalid from_pattern regex: {}", e))?;
            let to_regex = regex::Regex::new(&constraint.to_pattern.replace("*", ".*"))
                .map_err(|e| format!("Invalid to_pattern regex: {}", e))?;

            // Check all edges
            for node in &graph.nodes {
                if from_regex.is_match(node) {
                    for neighbor in graph.neighbors(node) {
                        if to_regex.is_match(neighbor) {
                            if !constraint.allowed {
                                violations.push(ConstraintViolation {
                                    from_entity: node.clone(),
                                    to_entity: neighbor.clone(),
                                    constraint_violated: format!(
                                        "{} -> {} (not allowed)",
                                        constraint.from_pattern, constraint.to_pattern
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(CheckConstraintsOutput {
            total_violations: violations.len() as i32,
            violations,
        })
    }
}

#[async_trait]
impl Tool for DepsCheckConstraintsTool {
    fn name(&self) -> &str {
        "cortex.deps.check_constraints"
    }

    fn description(&self) -> Option<&str> {
        Some("Check architectural constraints and detect violations (e.g., 'UI cannot depend on Database')")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CheckConstraintsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CheckConstraintsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// 10. GENERATE GRAPH
// ============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GenerateGraphInput {
    scope_path: String,
    #[serde(default = "default_dot_format")]
    format: String,
    #[serde(default)]
    include_external: bool,
    #[serde(default = "default_none")]
    cluster_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GenerateGraphOutput {
    graph_data: String,
    format: String,
    node_count: i32,
    edge_count: i32,
}

pub struct DepsGenerateGraphTool {
    ctx: DependencyAnalysisContext,
}

impl DepsGenerateGraphTool {
    pub fn new(ctx: DependencyAnalysisContext) -> Self {
        Self { ctx }
    }

    async fn execute_impl(&self, input: GenerateGraphInput) -> std::result::Result<GenerateGraphOutput, String> {
        debug!("Generating dependency graph in format: {}", input.format);

        let graph = self.ctx.build_graph(Some(&input.scope_path)).await?;

        let graph_data = match input.format.as_str() {
            "dot" => self.generate_dot(&graph, &input)?,
            "json" => self.generate_json(&graph)?,
            _ => return Err(format!("Unsupported format: {}", input.format)),
        };

        Ok(GenerateGraphOutput {
            graph_data,
            format: input.format,
            node_count: graph.nodes.len() as i32,
            edge_count: graph.adjacency.values().map(|v| v.len()).sum::<usize>() as i32,
        })
    }

    fn generate_dot(&self, graph: &Graph, input: &GenerateGraphInput) -> std::result::Result<String, String> {
        let mut dot = String::from("digraph Dependencies {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Add nodes
        for node in &graph.nodes {
            let label = node.split("::").last().unwrap_or(node);
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", node, label));
        }

        dot.push_str("\n");

        // Add edges
        for (from, neighbors) in &graph.adjacency {
            for to in neighbors {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", from, to));
            }
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    fn generate_json(&self, graph: &Graph) -> std::result::Result<String, String> {
        let nodes: Vec<_> = graph.nodes.iter().cloned().collect();
        let edges: Vec<_> = graph
            .adjacency
            .iter()
            .flat_map(|(from, neighbors)| {
                neighbors
                    .iter()
                    .map(move |to| serde_json::json!({"from": from, "to": to}))
            })
            .collect();

        let json = serde_json::json!({
            "nodes": nodes,
            "edges": edges,
        });

        serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Tool for DepsGenerateGraphTool {
    fn name(&self) -> &str {
        "cortex.deps.generate_graph"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate dependency graph visualization in DOT or JSON format")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GenerateGraphInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GenerateGraphInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.execute_impl(input).await {
            Ok(output) => Ok(ToolResult::success_json(serde_json::to_value(output).unwrap())),
            Err(e) => Err(ToolError::ExecutionFailed(e)),
        }
    }
}

// ============================================================================
// DEFAULT FUNCTIONS
// ============================================================================

fn default_outgoing() -> String {
    "outgoing".to_string()
}
fn default_depth_one() -> i32 {
    1
}
fn default_max_depth() -> i32 {
    10
}
fn default_shortest() -> String {
    "shortest".to_string()
}
fn default_file_level() -> String {
    "file".to_string()
}
fn default_all_depth() -> i32 {
    -1
}
fn default_min_connections() -> i32 {
    10
}
fn default_total() -> String {
    "total".to_string()
}
fn default_true() -> bool {
    true
}
fn default_dot_format() -> String {
    "dot".to_string()
}
fn default_none() -> String {
    "none".to_string()
}
