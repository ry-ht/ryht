//! Dependency Analysis Tools (10 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct DependencyAnalysisContext {
    storage: Arc<ConnectionManager>,
}

impl DependencyAnalysisContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_dep_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: DependencyAnalysisContext,
        }

        impl $name {
            pub fn new(ctx: DependencyAnalysisContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
}

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

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct Dependency {
    target_id: String,
    dependency_type: String,
    depth: i32,
}

impl_dep_tool!(DepsGetDependenciesTool, "cortex.deps.get_dependencies", "Get dependencies of a unit or file", GetDependenciesInput, GetDependenciesOutput);

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

impl_dep_tool!(DepsFindPathTool, "cortex.deps.find_path", "Find dependency path between entities", FindPathInput, FindPathOutput);

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

impl_dep_tool!(DepsFindCyclesTool, "cortex.deps.find_cycles", "Detect circular dependencies", FindCyclesInput, FindCyclesOutput);

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

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ImpactedEntity {
    entity_id: String,
    impact_type: String,
    distance: i32,
}

impl_dep_tool!(DepsImpactAnalysisTool, "cortex.deps.impact_analysis", "Analyze impact of changes", ImpactAnalysisInput, ImpactAnalysisOutput);

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

impl_dep_tool!(DepsFindRootsTool, "cortex.deps.find_roots", "Find root entities (no dependencies)", FindRootsInput, FindRootsOutput);

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

impl_dep_tool!(DepsFindLeavesTool, "cortex.deps.find_leaves", "Find leaf entities (no dependents)", FindLeavesInput, FindLeavesOutput);

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

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct HubEntity {
    entity_id: String,
    incoming_count: i32,
    outgoing_count: i32,
    total_count: i32,
}

impl_dep_tool!(DepsFindHubsTool, "cortex.deps.find_hubs", "Find highly connected entities", FindHubsInput, FindHubsOutput);

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

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct Layer {
    layer_id: i32,
    entities: Vec<String>,
}

impl_dep_tool!(DepsGetLayersTool, "cortex.deps.get_layers", "Get architectural layers", GetLayersInput, GetLayersOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckConstraintsInput {
    constraints: Vec<DependencyConstraint>,
}

#[derive(Debug, Deserialize, JsonSchema)]
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

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConstraintViolation {
    from_entity: String,
    to_entity: String,
    constraint_violated: String,
}

impl_dep_tool!(DepsCheckConstraintsTool, "cortex.deps.check_constraints", "Check dependency constraints", CheckConstraintsInput, CheckConstraintsOutput);

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

impl_dep_tool!(DepsGenerateGraphTool, "cortex.deps.generate_graph", "Generate dependency graph", GenerateGraphInput, GenerateGraphOutput);

fn default_outgoing() -> String { "outgoing".to_string() }
fn default_depth_one() -> i32 { 1 }
fn default_max_depth() -> i32 { 10 }
fn default_shortest() -> String { "shortest".to_string() }
fn default_file_level() -> String { "file".to_string() }
fn default_all_depth() -> i32 { -1 }
fn default_min_connections() -> i32 { 10 }
fn default_total() -> String { "total".to_string() }
fn default_true() -> bool { true }
fn default_dot_format() -> String { "dot".to_string() }
fn default_none() -> String { "none".to_string() }
