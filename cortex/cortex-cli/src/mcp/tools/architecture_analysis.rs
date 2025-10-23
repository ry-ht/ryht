//! Architecture Analysis Tools (5 tools)
//!
//! Provides architecture visualization, pattern detection, and constraint checking

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use std::collections::{HashMap, HashSet};

use crate::mcp::graph_algorithms::{
    Graph, find_cycles, topological_layers, calculate_centrality,
};

#[derive(Clone)]
pub struct ArchitectureAnalysisContext {
    storage: Arc<ConnectionManager>,
}

impl ArchitectureAnalysisContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Build dependency graph from database
    async fn build_graph(&self, scope_path: Option<&str>) -> std::result::Result<Graph, String> {
        debug!("Building dependency graph for scope: {:?}", scope_path);

        // Convert to owned String early to avoid lifetime issues
        let scope_path_owned: Option<String> = scope_path.map(|s| s.to_string());

        let conn = self.storage
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        // Query dependencies from database
        let query = if scope_path_owned.is_some() {
            r#"
            SELECT source_id, target_id, dependency_type
            FROM DEPENDS_ON
            WHERE source_id IN (
                SELECT id FROM code_unit WHERE file_path CONTAINS $path
            )
            "#
        } else {
            "SELECT source_id, target_id, dependency_type FROM DEPENDS_ON"
        };

        let mut result = conn
            .connection()
            .query(query)
            .bind(("path", scope_path_owned.unwrap_or_default()))
            .await
            .map_err(|e| format!("Failed to query dependencies: {}", e))?;

        #[derive(serde::Deserialize)]
        struct DepEdge {
            source_id: String,
            target_id: String,
            #[allow(dead_code)]
            dependency_type: String,
        }

        let edges: Vec<DepEdge> = result
            .take(0)
            .map_err(|e| format!("Failed to extract dependencies: {}", e))?;

        info!("Building graph from {} dependency edges", edges.len());

        // Build graph
        let mut graph = Graph::new();
        for edge in edges {
            graph.add_edge(edge.source_id, edge.target_id);
        }

        Ok(graph)
    }

    /// Get code units and their metadata
    async fn get_code_units(&self, scope_path: Option<&str>) -> std::result::Result<Vec<CodeUnit>, String> {
        // Convert to owned String early to avoid lifetime issues
        let scope_path_owned: Option<String> = scope_path.map(|s| s.to_string());

        let conn = self.storage
            .acquire()
            .await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        let query = if scope_path_owned.is_some() {
            "SELECT id, qualified_name, kind, file_path, metadata FROM code_unit WHERE file_path CONTAINS $path"
        } else {
            "SELECT id, qualified_name, kind, file_path, metadata FROM code_unit"
        };

        let mut result = conn
            .connection()
            .query(query)
            .bind(("path", scope_path_owned.unwrap_or_default()))
            .await
            .map_err(|e| format!("Failed to query code units: {}", e))?;

        let units: Vec<CodeUnit> = result
            .take(0)
            .map_err(|e| format!("Failed to extract code units: {}", e))?;

        Ok(units)
    }
}

#[derive(Debug, serde::Deserialize)]
struct CodeUnit {
    id: String,
    qualified_name: String,
    kind: String,
    file_path: String,
    metadata: serde_json::Value,
}

// =============================================================================
// cortex.arch.visualize
// =============================================================================

pub struct ArchVisualizeTool {
    ctx: ArchitectureAnalysisContext,
}

impl ArchVisualizeTool {
    pub fn new(ctx: ArchitectureAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct VisualizeArchitectureInput {
    scope_path: String,
    #[serde(default = "default_mermaid")]
    output_format: String,
    #[serde(default = "default_dependency_view")]
    view_type: String,
    #[serde(default = "default_depth")]
    max_depth: i32,
    #[serde(default)]
    include_external: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct VisualizeArchitectureOutput {
    diagram: String,
    format: String,
    view_type: String,
    metadata: DiagramMetadata,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DiagramMetadata {
    total_nodes: i32,
    total_edges: i32,
    layers_detected: i32,
    complexity_score: f32,
}

impl Default for VisualizeArchitectureOutput {
    fn default() -> Self {
        Self {
            diagram: String::new(),
            format: "mermaid".to_string(),
            view_type: "dependency".to_string(),
            metadata: DiagramMetadata {
                total_nodes: 0,
                total_edges: 0,
                layers_detected: 0,
                complexity_score: 0.0,
            },
        }
    }
}

#[async_trait]
impl Tool for ArchVisualizeTool {
    fn name(&self) -> &str {
        "cortex.arch.visualize"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate architecture diagrams (dependency graphs, layer diagrams, component views)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(VisualizeArchitectureInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: VisualizeArchitectureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Generating architecture visualization for: {}", input.scope_path);

        // Build dependency graph
        let scope = if input.scope_path.is_empty() {
            None
        } else {
            Some(input.scope_path.as_str())
        };

        let graph = self.ctx.build_graph(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build graph: {}", e)))?;

        // Get code units for metadata
        let units = self.ctx.get_code_units(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code units: {}", e)))?;

        // Build unit lookup map
        let unit_map: HashMap<String, &CodeUnit> = units.iter()
            .map(|u| (u.id.clone(), u))
            .collect();

        // Detect layers
        let layers = topological_layers(&graph);

        // Calculate metrics
        let total_nodes = graph.nodes.len() as i32;
        let total_edges: i32 = graph.adjacency.values().map(|v| v.len() as i32).sum();
        let layers_detected = layers.len() as i32;

        // Calculate complexity score (based on coupling and cyclomatic complexity)
        let avg_degree = if total_nodes > 0 {
            total_edges as f32 / total_nodes as f32
        } else {
            0.0
        };
        let complexity_score = (avg_degree * 10.0).min(100.0);

        // Generate diagram based on format
        let diagram = match input.output_format.as_str() {
            "mermaid" => generate_mermaid_diagram(&graph, &unit_map, &input.view_type, input.max_depth, input.include_external),
            "dot" | "graphviz" => generate_dot_diagram(&graph, &unit_map, &input.view_type, input.max_depth),
            "graphml" => generate_graphml_diagram(&graph, &unit_map, &layers),
            _ => generate_mermaid_diagram(&graph, &unit_map, &input.view_type, input.max_depth, input.include_external),
        };

        let output = VisualizeArchitectureOutput {
            diagram,
            format: input.output_format,
            view_type: input.view_type,
            metadata: DiagramMetadata {
                total_nodes,
                total_edges,
                layers_detected,
                complexity_score,
            },
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.arch.detect_patterns
// =============================================================================

pub struct ArchDetectPatternsTool {
    ctx: ArchitectureAnalysisContext,
}

impl ArchDetectPatternsTool {
    pub fn new(ctx: ArchitectureAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DetectPatternsInput {
    scope_path: String,
    #[serde(default = "default_all_pattern_categories")]
    pattern_categories: Vec<String>,
    #[serde(default = "default_high_confidence")]
    min_confidence: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DetectPatternsOutput {
    patterns: Vec<DetectedPattern>,
    total_count: i32,
    pattern_summary: Vec<PatternSummary>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DetectedPattern {
    pattern_id: String,
    pattern_name: String,
    category: String,
    location: String,
    participants: Vec<String>,
    confidence: f32,
    description: String,
    benefits: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct PatternSummary {
    category: String,
    count: i32,
    patterns: Vec<String>,
}

impl Default for DetectPatternsOutput {
    fn default() -> Self {
        Self {
            patterns: vec![],
            total_count: 0,
            pattern_summary: vec![],
        }
    }
}

#[async_trait]
impl Tool for ArchDetectPatternsTool {
    fn name(&self) -> &str {
        "cortex.arch.detect_patterns"
    }

    fn description(&self) -> Option<&str> {
        Some("Detect design patterns (Singleton, Factory, Observer, etc.) in codebase")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DetectPatternsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DetectPatternsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Detecting design patterns at: {}", input.scope_path);

        // Build dependency graph
        let scope = if input.scope_path.is_empty() {
            None
        } else {
            Some(input.scope_path.as_str())
        };

        let graph = self.ctx.build_graph(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build graph: {}", e)))?;

        // Get code units for structural analysis
        let units = self.ctx.get_code_units(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code units: {}", e)))?;

        let mut detected_patterns = Vec::new();

        // Detect patterns based on requested categories
        for category in &input.pattern_categories {
            match category.as_str() {
                "creational" => {
                    detected_patterns.extend(detect_creational_patterns(&units, &graph));
                }
                "structural" => {
                    detected_patterns.extend(detect_structural_patterns(&units, &graph));
                }
                "behavioral" => {
                    detected_patterns.extend(detect_behavioral_patterns(&units, &graph));
                }
                "architectural" => {
                    detected_patterns.extend(detect_architectural_patterns(&units, &graph));
                }
                "anti-patterns" => {
                    detected_patterns.extend(detect_anti_patterns(&units, &graph));
                }
                _ => {
                    warn!("Unknown pattern category: {}", category);
                }
            }
        }

        // Filter by confidence threshold
        detected_patterns.retain(|p| p.confidence >= input.min_confidence);

        // Generate summary
        let mut pattern_summary = HashMap::new();
        for pattern in &detected_patterns {
            pattern_summary
                .entry(pattern.category.clone())
                .or_insert_with(|| (0, Vec::new()))
                .0 += 1;
            pattern_summary
                .get_mut(&pattern.category)
                .unwrap()
                .1.push(pattern.pattern_name.clone());
        }

        let pattern_summary: Vec<PatternSummary> = pattern_summary
            .into_iter()
            .map(|(category, (count, patterns))| {
                let mut unique_patterns = patterns;
                unique_patterns.sort();
                unique_patterns.dedup();
                PatternSummary {
                    category,
                    count,
                    patterns: unique_patterns,
                }
            })
            .collect();

        let output = DetectPatternsOutput {
            total_count: detected_patterns.len() as i32,
            patterns: detected_patterns,
            pattern_summary,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.arch.suggest_boundaries
// =============================================================================

pub struct ArchSuggestBoundariesTool {
    ctx: ArchitectureAnalysisContext,
}

impl ArchSuggestBoundariesTool {
    pub fn new(ctx: ArchitectureAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SuggestBoundariesInput {
    scope_path: String,
    #[serde(default = "default_high_cohesion")]
    target_cohesion: f32,
    #[serde(default = "default_low_coupling")]
    max_coupling: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SuggestBoundariesOutput {
    suggested_modules: Vec<SuggestedModule>,
    refactoring_suggestions: Vec<String>,
    boundary_violations: Vec<BoundaryViolation>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SuggestedModule {
    module_name: String,
    components: Vec<String>,
    cohesion_score: f32,
    coupling_score: f32,
    reasoning: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct BoundaryViolation {
    from_module: String,
    to_module: String,
    violation_type: String,
    suggestion: String,
}

impl Default for SuggestBoundariesOutput {
    fn default() -> Self {
        Self {
            suggested_modules: vec![],
            refactoring_suggestions: vec![],
            boundary_violations: vec![],
        }
    }
}

#[async_trait]
impl Tool for ArchSuggestBoundariesTool {
    fn name(&self) -> &str {
        "cortex.arch.suggest_boundaries"
    }

    fn description(&self) -> Option<&str> {
        Some("Suggest optimal module boundaries based on cohesion and coupling analysis")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SuggestBoundariesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SuggestBoundariesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Analyzing module boundaries at: {}", input.scope_path);

        // Build dependency graph
        let scope = if input.scope_path.is_empty() {
            None
        } else {
            Some(input.scope_path.as_str())
        };

        let graph = self.ctx.build_graph(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build graph: {}", e)))?;

        // Get code units
        let units = self.ctx.get_code_units(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code units: {}", e)))?;

        // Perform clustering based on coupling and cohesion
        let clusters = perform_clustering(&graph, &units, input.target_cohesion, input.max_coupling);

        // Generate module suggestions
        let mut suggested_modules = Vec::new();
        for (idx, cluster) in clusters.iter().enumerate() {
            let module_name = format!("module_{}", idx + 1);
            let components: Vec<String> = cluster.iter()
                .filter_map(|id| units.iter().find(|u| &u.id == id))
                .map(|u| u.qualified_name.clone())
                .collect();

            // Calculate cohesion and coupling metrics
            let (cohesion, coupling) = calculate_module_metrics(cluster, &graph);

            let reasoning = if cohesion >= input.target_cohesion && coupling <= input.max_coupling {
                format!("Well-balanced module with high cohesion ({:.2}) and low coupling ({:.2})", cohesion, coupling)
            } else if cohesion < input.target_cohesion {
                format!("Module has low cohesion ({:.2}), consider splitting further", cohesion)
            } else {
                format!("Module has high coupling ({:.2}), consider reducing dependencies", coupling)
            };

            suggested_modules.push(SuggestedModule {
                module_name,
                components,
                cohesion_score: cohesion,
                coupling_score: coupling,
                reasoning,
            });
        }

        // Detect boundary violations
        let boundary_violations = detect_boundary_violations(&graph, &clusters, &units);

        // Generate refactoring suggestions
        let refactoring_suggestions = generate_refactoring_suggestions(
            &suggested_modules,
            &boundary_violations,
            input.target_cohesion,
            input.max_coupling,
        );

        let output = SuggestBoundariesOutput {
            suggested_modules,
            refactoring_suggestions,
            boundary_violations,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.arch.check_violations
// =============================================================================

pub struct ArchCheckViolationsTool {
    ctx: ArchitectureAnalysisContext,
}

impl ArchCheckViolationsTool {
    pub fn new(ctx: ArchitectureAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CheckViolationsInput {
    scope_path: String,
    rules: Vec<ArchitectureRule>,
    #[serde(default = "default_true")]
    auto_detect_layers: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ArchitectureRule {
    rule_type: String,
    from_layer: Option<String>,
    to_layer: Option<String>,
    allowed: bool,
    description: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CheckViolationsOutput {
    violations: Vec<ArchitectureViolation>,
    total_violations: i32,
    rules_checked: i32,
    compliance_score: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ArchitectureViolation {
    rule: String,
    from_component: String,
    to_component: String,
    violation_type: String,
    severity: String,
    description: String,
    suggestion: String,
}

impl Default for CheckViolationsOutput {
    fn default() -> Self {
        Self {
            violations: vec![],
            total_violations: 0,
            rules_checked: 0,
            compliance_score: 100.0,
        }
    }
}

#[async_trait]
impl Tool for ArchCheckViolationsTool {
    fn name(&self) -> &str {
        "cortex.arch.check_violations"
    }

    fn description(&self) -> Option<&str> {
        Some("Check architectural constraints and detect violations (layering, dependency rules)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CheckViolationsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CheckViolationsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Checking architectural violations at: {}", input.scope_path);

        // Build dependency graph
        let scope = if input.scope_path.is_empty() {
            None
        } else {
            Some(input.scope_path.as_str())
        };

        let graph = self.ctx.build_graph(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build graph: {}", e)))?;

        // Get code units
        let units = self.ctx.get_code_units(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code units: {}", e)))?;

        let mut violations = Vec::new();
        let mut rules_checked = 0;

        // Auto-detect layers if requested
        let layers = if input.auto_detect_layers {
            Some(topological_layers(&graph))
        } else {
            None
        };

        // Check for circular dependencies
        let cycles = find_cycles(&graph);
        for cycle in cycles {
            let cycle_str = cycle.join(" -> ");
            violations.push(ArchitectureViolation {
                rule: "No Circular Dependencies".to_string(),
                from_component: cycle.first().cloned().unwrap_or_default(),
                to_component: cycle.last().cloned().unwrap_or_default(),
                violation_type: "circular_dependency".to_string(),
                severity: "high".to_string(),
                description: format!("Circular dependency detected: {}", cycle_str),
                suggestion: "Break the cycle by introducing an interface or refactoring dependencies".to_string(),
            });
        }
        rules_checked += 1;

        // Check user-defined rules
        for rule in &input.rules {
            rules_checked += 1;

            match rule.rule_type.as_str() {
                "layering" => {
                    violations.extend(check_layering_violations(
                        &graph,
                        &units,
                        &layers,
                        &rule.from_layer,
                        &rule.to_layer,
                        rule.allowed,
                        &rule.description,
                    ));
                }
                "dependency_direction" => {
                    violations.extend(check_dependency_direction_violations(
                        &graph,
                        &units,
                        &rule.from_layer,
                        &rule.to_layer,
                        rule.allowed,
                        &rule.description,
                    ));
                }
                "module_isolation" => {
                    violations.extend(check_module_isolation_violations(
                        &graph,
                        &units,
                        &rule.from_layer,
                        &rule.description,
                    ));
                }
                _ => {
                    warn!("Unknown rule type: {}", rule.rule_type);
                }
            }
        }

        // Calculate compliance score
        let total_edges = graph.adjacency.values().map(|v| v.len()).sum::<usize>() as f32;
        let compliance_score = if total_edges > 0.0 {
            ((total_edges - violations.len() as f32) / total_edges * 100.0).max(0.0)
        } else {
            100.0
        };

        let output = CheckViolationsOutput {
            total_violations: violations.len() as i32,
            violations,
            rules_checked,
            compliance_score,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.arch.analyze_drift
// =============================================================================

pub struct ArchAnalyzeDriftTool {
    ctx: ArchitectureAnalysisContext,
}

impl ArchAnalyzeDriftTool {
    pub fn new(ctx: ArchitectureAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzeDriftInput {
    scope_path: String,
    baseline_version: Option<String>,
    #[serde(default = "default_all_drift_types")]
    drift_types: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AnalyzeDriftOutput {
    drift_score: f32,
    drifts: Vec<ArchitectureDrift>,
    total_drifts: i32,
    trend: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ArchitectureDrift {
    drift_type: String,
    component: String,
    description: String,
    severity: String,
    detected_at: String,
    recommendation: String,
}

impl Default for AnalyzeDriftOutput {
    fn default() -> Self {
        Self {
            drift_score: 0.0,
            drifts: vec![],
            total_drifts: 0,
            trend: "stable".to_string(),
        }
    }
}

#[async_trait]
impl Tool for ArchAnalyzeDriftTool {
    fn name(&self) -> &str {
        "cortex.arch.analyze_drift"
    }

    fn description(&self) -> Option<&str> {
        Some("Detect architectural drift from intended design over time")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyzeDriftInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyzeDriftInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Analyzing architectural drift at: {}", input.scope_path);

        // Build current dependency graph
        let scope = if input.scope_path.is_empty() {
            None
        } else {
            Some(input.scope_path.as_str())
        };

        let current_graph = self.ctx.build_graph(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build current graph: {}", e)))?;

        // Get current code units
        let current_units = self.ctx.get_code_units(scope)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code units: {}", e)))?;

        // Calculate current metrics
        let current_metrics = calculate_architecture_metrics(&current_graph, &current_units);

        let mut drifts = Vec::new();
        let mut drift_score = 0.0;

        // Analyze drift based on requested types
        for drift_type in &input.drift_types {
            match drift_type.as_str() {
                "dependency" => {
                    // Check for new dependencies and removed dependencies
                    let cycles = find_cycles(&current_graph);
                    if !cycles.is_empty() {
                        drift_score += 20.0;
                        for cycle in cycles {
                            drifts.push(ArchitectureDrift {
                                drift_type: "dependency".to_string(),
                                component: cycle.join(" -> "),
                                description: "New circular dependency introduced".to_string(),
                                severity: "high".to_string(),
                                detected_at: chrono::Utc::now().to_rfc3339(),
                                recommendation: "Break the dependency cycle using dependency inversion or refactoring".to_string(),
                            });
                        }
                    }
                }
                "complexity" => {
                    // Check for increasing complexity
                    if current_metrics.avg_degree > 5.0 {
                        drift_score += 15.0;
                        drifts.push(ArchitectureDrift {
                            drift_type: "complexity".to_string(),
                            component: "overall".to_string(),
                            description: format!("High average coupling detected: {:.2} dependencies per component", current_metrics.avg_degree),
                            severity: "medium".to_string(),
                            detected_at: chrono::Utc::now().to_rfc3339(),
                            recommendation: "Refactor to reduce coupling between components".to_string(),
                        });
                    }

                    if current_metrics.max_degree > 20 {
                        drift_score += 20.0;
                        if let Some(hub) = &current_metrics.highest_coupled_component {
                            drifts.push(ArchitectureDrift {
                                drift_type: "complexity".to_string(),
                                component: hub.clone(),
                                description: format!("Component has {} dependencies (god object)", current_metrics.max_degree),
                                severity: "high".to_string(),
                                detected_at: chrono::Utc::now().to_rfc3339(),
                                recommendation: "Split this component into smaller, focused components".to_string(),
                            });
                        }
                    }
                }
                "coupling" => {
                    // Detect high coupling between modules
                    let centrality = calculate_centrality(&current_graph);
                    let high_centrality: Vec<_> = centrality.iter()
                        .filter(|&(_, &score)| score > 0.5)
                        .collect();

                    if !high_centrality.is_empty() {
                        drift_score += 10.0 * high_centrality.len() as f32;
                        for (component, score) in high_centrality {
                            drifts.push(ArchitectureDrift {
                                drift_type: "coupling".to_string(),
                                component: component.clone(),
                                description: format!("High betweenness centrality: {:.2}", score),
                                severity: "medium".to_string(),
                                detected_at: chrono::Utc::now().to_rfc3339(),
                                recommendation: "This component is a bottleneck; consider reducing its responsibilities".to_string(),
                            });
                        }
                    }
                }
                "layering" => {
                    // Check for layer violations
                    let layers = topological_layers(&current_graph);
                    if layers.is_empty() && !current_graph.nodes.is_empty() {
                        drift_score += 30.0;
                        drifts.push(ArchitectureDrift {
                            drift_type: "layering".to_string(),
                            component: "overall".to_string(),
                            description: "Cannot establish clear layering due to circular dependencies".to_string(),
                            severity: "high".to_string(),
                            detected_at: chrono::Utc::now().to_rfc3339(),
                            recommendation: "Refactor to establish clear architectural layers".to_string(),
                        });
                    }
                }
                _ => {
                    warn!("Unknown drift type: {}", drift_type);
                }
            }
        }

        // Determine trend
        let trend = if drift_score < 20.0 {
            "stable"
        } else if drift_score < 50.0 {
            "degrading"
        } else {
            "critical"
        };

        let output = AnalyzeDriftOutput {
            drift_score: drift_score.min(100.0),
            total_drifts: drifts.len() as i32,
            drifts,
            trend: trend.to_string(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate Mermaid diagram
fn generate_mermaid_diagram(
    graph: &Graph,
    unit_map: &HashMap<String, &CodeUnit>,
    view_type: &str,
    max_depth: i32,
    include_external: bool,
) -> String {
    let mut output = String::from("graph TD\n");

    match view_type {
        "dependency" => {
            // Show dependency graph
            let mut nodes_added = HashSet::new();
            let mut depth_map = HashMap::new();

            // BFS to limit depth
            for root in graph.nodes.iter() {
                if graph.reverse_adjacency.get(root).map(|v| v.is_empty()).unwrap_or(true) {
                    depth_map.insert(root.clone(), 0);
                }
            }

            for node in &graph.nodes {
                if !nodes_added.contains(node) && (include_external || unit_map.contains_key(node)) {
                    let label = unit_map.get(node)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| node.clone());
                    output.push_str(&format!("    {}[\"{}\"]\n", sanitize_id(node), label));
                    nodes_added.insert(node.clone());
                }

                for dep in graph.neighbors(node) {
                    if max_depth < 0 || depth_map.get(node).unwrap_or(&0) < &max_depth {
                        if include_external || (unit_map.contains_key(node) && unit_map.contains_key(dep)) {
                            output.push_str(&format!("    {} --> {}\n", sanitize_id(node), sanitize_id(dep)));
                        }
                    }
                }
            }
        }
        "layer" => {
            // Show layered architecture
            let layers = topological_layers(graph);
            for (layer_idx, layer) in layers.iter().enumerate() {
                output.push_str(&format!("    subgraph Layer{}\n", layer_idx));
                for node in layer {
                    if include_external || unit_map.contains_key(node) {
                        let label = unit_map.get(node)
                            .map(|u| u.qualified_name.clone())
                            .unwrap_or_else(|| node.clone());
                        output.push_str(&format!("        {}[\"{}\"]\n", sanitize_id(node), label));
                    }
                }
                output.push_str("    end\n");
            }

            // Add edges
            for node in &graph.nodes {
                for dep in graph.neighbors(node) {
                    if include_external || (unit_map.contains_key(node) && unit_map.contains_key(dep)) {
                        output.push_str(&format!("    {} --> {}\n", sanitize_id(node), sanitize_id(dep)));
                    }
                }
            }
        }
        _ => {
            // Default to dependency view
            return generate_mermaid_diagram(graph, unit_map, "dependency", max_depth, include_external);
        }
    }

    output
}

/// Generate GraphViz DOT diagram
fn generate_dot_diagram(
    graph: &Graph,
    unit_map: &HashMap<String, &CodeUnit>,
    view_type: &str,
    max_depth: i32,
) -> String {
    let mut output = String::from("digraph Architecture {\n");
    output.push_str("    rankdir=TB;\n");
    output.push_str("    node [shape=box, style=rounded];\n\n");

    match view_type {
        "layer" => {
            let layers = topological_layers(graph);
            for (layer_idx, layer) in layers.iter().enumerate() {
                output.push_str(&format!("    subgraph cluster_{} {{\n", layer_idx));
                output.push_str(&format!("        label=\"Layer {}\";\n", layer_idx));
                for node in layer {
                    let label = unit_map.get(node)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| node.clone());
                    output.push_str(&format!("        \"{}\" [label=\"{}\"];\n", node, label));
                }
                output.push_str("    }\n\n");
            }
        }
        _ => {
            // Add nodes
            for node in &graph.nodes {
                let label = unit_map.get(node)
                    .map(|u| u.qualified_name.clone())
                    .unwrap_or_else(|| node.clone());
                output.push_str(&format!("    \"{}\" [label=\"{}\"];\n", node, label));
            }
        }
    }

    // Add edges with depth limiting
    let mut depth_map = HashMap::new();
    for root in graph.nodes.iter() {
        if graph.reverse_adjacency.get(root).map(|v| v.is_empty()).unwrap_or(true) {
            depth_map.insert(root.clone(), 0);
        }
    }

    for node in &graph.nodes {
        for dep in graph.neighbors(node) {
            if max_depth < 0 || depth_map.get(node).unwrap_or(&0) < &max_depth {
                output.push_str(&format!("    \"{}\" -> \"{}\";\n", node, dep));
            }
        }
    }

    output.push_str("}\n");
    output
}

/// Generate GraphML diagram
fn generate_graphml_diagram(
    graph: &Graph,
    unit_map: &HashMap<String, &CodeUnit>,
    layers: &[Vec<String>],
) -> String {
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n");
    output.push_str("  <key id=\"label\" for=\"node\" attr.name=\"label\" attr.type=\"string\"/>\n");
    output.push_str("  <key id=\"layer\" for=\"node\" attr.name=\"layer\" attr.type=\"int\"/>\n");
    output.push_str("  <graph id=\"G\" edgedefault=\"directed\">\n");

    // Add nodes
    for node in &graph.nodes {
        let label = unit_map.get(node)
            .map(|u| u.qualified_name.clone())
            .unwrap_or_else(|| node.clone());

        let layer_idx = layers.iter()
            .position(|layer| layer.contains(node))
            .unwrap_or(0);

        output.push_str(&format!("    <node id=\"{}\">\n", node));
        output.push_str(&format!("      <data key=\"label\">{}</data>\n", label));
        output.push_str(&format!("      <data key=\"layer\">{}</data>\n", layer_idx));
        output.push_str("    </node>\n");
    }

    // Add edges
    let mut edge_id = 0;
    for node in &graph.nodes {
        for dep in graph.neighbors(node) {
            output.push_str(&format!("    <edge id=\"e{}\" source=\"{}\" target=\"{}\"/>\n", edge_id, node, dep));
            edge_id += 1;
        }
    }

    output.push_str("  </graph>\n");
    output.push_str("</graphml>\n");
    output
}

fn sanitize_id(id: &str) -> String {
    id.replace([':', '/', '.', '-'], "_")
}

/// Detect creational patterns (Factory, Singleton, Builder)
fn detect_creational_patterns(units: &[CodeUnit], graph: &Graph) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    for unit in units {
        let name = &unit.qualified_name;
        let kind = &unit.kind;

        // Factory pattern: classes with "Factory" in name and create methods
        if name.contains("Factory") && kind == "class" {
            let dependents = graph.reverse_neighbors(&unit.id);
            patterns.push(DetectedPattern {
                pattern_id: format!("factory_{}", unit.id),
                pattern_name: "Factory".to_string(),
                category: "creational".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.85,
                description: "Factory pattern for object creation".to_string(),
                benefits: vec![
                    "Encapsulates object creation".to_string(),
                    "Promotes loose coupling".to_string(),
                ],
            });
        }

        // Singleton pattern: static instance field
        if unit.metadata.get("has_static_instance").and_then(|v| v.as_bool()).unwrap_or(false) {
            patterns.push(DetectedPattern {
                pattern_id: format!("singleton_{}", unit.id),
                pattern_name: "Singleton".to_string(),
                category: "creational".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.9,
                description: "Singleton pattern for global instance".to_string(),
                benefits: vec![
                    "Ensures single instance".to_string(),
                    "Global access point".to_string(),
                ],
            });
        }

        // Builder pattern: classes with "Builder" in name
        if name.contains("Builder") {
            patterns.push(DetectedPattern {
                pattern_id: format!("builder_{}", unit.id),
                pattern_name: "Builder".to_string(),
                category: "creational".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.8,
                description: "Builder pattern for complex object construction".to_string(),
                benefits: vec![
                    "Step-by-step construction".to_string(),
                    "Fluent interface".to_string(),
                ],
            });
        }
    }

    patterns
}

/// Detect structural patterns (Adapter, Decorator, Facade)
fn detect_structural_patterns(units: &[CodeUnit], _graph: &Graph) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    for unit in units {
        let name = &unit.qualified_name;

        // Adapter pattern
        if name.contains("Adapter") {
            patterns.push(DetectedPattern {
                pattern_id: format!("adapter_{}", unit.id),
                pattern_name: "Adapter".to_string(),
                category: "structural".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.85,
                description: "Adapter pattern for interface compatibility".to_string(),
                benefits: vec![
                    "Converts incompatible interfaces".to_string(),
                    "Enables reuse of existing code".to_string(),
                ],
            });
        }

        // Decorator pattern
        if name.contains("Decorator") || name.contains("Wrapper") {
            patterns.push(DetectedPattern {
                pattern_id: format!("decorator_{}", unit.id),
                pattern_name: "Decorator".to_string(),
                category: "structural".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.8,
                description: "Decorator pattern for adding behavior".to_string(),
                benefits: vec![
                    "Adds functionality dynamically".to_string(),
                    "Alternative to subclassing".to_string(),
                ],
            });
        }

        // Facade pattern
        if name.contains("Facade") || name.contains("Manager") {
            patterns.push(DetectedPattern {
                pattern_id: format!("facade_{}", unit.id),
                pattern_name: "Facade".to_string(),
                category: "structural".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.75,
                description: "Facade pattern for simplified interface".to_string(),
                benefits: vec![
                    "Simplifies complex subsystem".to_string(),
                    "Reduces coupling".to_string(),
                ],
            });
        }
    }

    patterns
}

/// Detect behavioral patterns (Observer, Strategy, Command)
fn detect_behavioral_patterns(units: &[CodeUnit], _graph: &Graph) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    for unit in units {
        let name = &unit.qualified_name;

        // Observer pattern
        if name.contains("Observer") || name.contains("Listener") || name.contains("Subscriber") {
            patterns.push(DetectedPattern {
                pattern_id: format!("observer_{}", unit.id),
                pattern_name: "Observer".to_string(),
                category: "behavioral".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.85,
                description: "Observer pattern for event notification".to_string(),
                benefits: vec![
                    "Loose coupling between objects".to_string(),
                    "Dynamic subscription".to_string(),
                ],
            });
        }

        // Strategy pattern
        if name.contains("Strategy") {
            patterns.push(DetectedPattern {
                pattern_id: format!("strategy_{}", unit.id),
                pattern_name: "Strategy".to_string(),
                category: "behavioral".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.8,
                description: "Strategy pattern for algorithm selection".to_string(),
                benefits: vec![
                    "Interchangeable algorithms".to_string(),
                    "Open/closed principle".to_string(),
                ],
            });
        }

        // Command pattern
        if name.contains("Command") {
            patterns.push(DetectedPattern {
                pattern_id: format!("command_{}", unit.id),
                pattern_name: "Command".to_string(),
                category: "behavioral".to_string(),
                location: unit.file_path.clone(),
                participants: vec![name.clone()],
                confidence: 0.8,
                description: "Command pattern for action encapsulation".to_string(),
                benefits: vec![
                    "Encapsulates requests as objects".to_string(),
                    "Supports undo/redo".to_string(),
                ],
            });
        }
    }

    patterns
}

/// Detect architectural patterns (MVC, Layered, Microservices)
fn detect_architectural_patterns(units: &[CodeUnit], graph: &Graph) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    // Detect MVC pattern
    let has_model = units.iter().any(|u| u.qualified_name.contains("Model"));
    let has_view = units.iter().any(|u| u.qualified_name.contains("View"));
    let has_controller = units.iter().any(|u| u.qualified_name.contains("Controller"));

    if has_model && has_view && has_controller {
        patterns.push(DetectedPattern {
            pattern_id: "mvc_pattern".to_string(),
            pattern_name: "Model-View-Controller".to_string(),
            category: "architectural".to_string(),
            location: "project".to_string(),
            participants: vec!["Model".to_string(), "View".to_string(), "Controller".to_string()],
            confidence: 0.8,
            description: "MVC architectural pattern detected".to_string(),
            benefits: vec![
                "Separation of concerns".to_string(),
                "Easier testing and maintenance".to_string(),
            ],
        });
    }

    // Detect layered architecture
    let layers = topological_layers(graph);
    if layers.len() >= 3 {
        patterns.push(DetectedPattern {
            pattern_id: "layered_architecture".to_string(),
            pattern_name: "Layered Architecture".to_string(),
            category: "architectural".to_string(),
            location: "project".to_string(),
            participants: vec![format!("{} layers detected", layers.len())],
            confidence: 0.85,
            description: format!("Layered architecture with {} layers", layers.len()),
            benefits: vec![
                "Clear separation of concerns".to_string(),
                "Easier to maintain and test".to_string(),
            ],
        });
    }

    patterns
}

/// Detect anti-patterns (God Object, Spaghetti Code, Circular Dependencies)
fn detect_anti_patterns(units: &[CodeUnit], graph: &Graph) -> Vec<DetectedPattern> {
    let mut patterns = Vec::new();

    // God Object: high coupling
    for unit in units {
        let total_degree = graph.total_degree(&unit.id);
        if total_degree > 20 {
            patterns.push(DetectedPattern {
                pattern_id: format!("god_object_{}", unit.id),
                pattern_name: "God Object".to_string(),
                category: "anti-patterns".to_string(),
                location: unit.file_path.clone(),
                participants: vec![unit.qualified_name.clone()],
                confidence: 0.9,
                description: format!("Component with {} dependencies (god object)", total_degree),
                benefits: vec![
                    "Should be refactored into smaller components".to_string(),
                ],
            });
        }
    }

    // Circular dependencies
    let cycles = find_cycles(graph);
    for cycle in cycles {
        patterns.push(DetectedPattern {
            pattern_id: format!("circular_dep_{}", cycle.join("_")),
            pattern_name: "Circular Dependency".to_string(),
            category: "anti-patterns".to_string(),
            location: "multiple".to_string(),
            participants: cycle.clone(),
            confidence: 1.0,
            description: format!("Circular dependency: {}", cycle.join(" -> ")),
            benefits: vec![
                "Should be broken using dependency inversion".to_string(),
            ],
        });
    }

    patterns
}

/// Perform clustering based on coupling and cohesion
fn perform_clustering(
    graph: &Graph,
    _units: &[CodeUnit],
    _target_cohesion: f32,
    _max_coupling: f32,
) -> Vec<HashSet<String>> {
    // Use strongly connected components as initial clusters
    let mut clusters = Vec::new();
    let mut visited = HashSet::new();

    // Group nodes by connected components
    for node in &graph.nodes {
        if !visited.contains(node) {
            let mut cluster = HashSet::new();
            let mut queue = vec![node.clone()];

            while let Some(current) = queue.pop() {
                if visited.insert(current.clone()) {
                    cluster.insert(current.clone());

                    // Add neighbors (both directions)
                    queue.extend(graph.neighbors(&current).iter().cloned());
                    queue.extend(graph.reverse_neighbors(&current).iter().cloned());
                }
            }

            if !cluster.is_empty() {
                clusters.push(cluster);
            }
        }
    }

    // If no clusters formed, create single cluster with all nodes
    if clusters.is_empty() && !graph.nodes.is_empty() {
        clusters.push(graph.nodes.iter().cloned().collect());
    }

    clusters
}

/// Calculate module metrics (cohesion and coupling)
fn calculate_module_metrics(cluster: &HashSet<String>, graph: &Graph) -> (f32, f32) {
    if cluster.is_empty() {
        return (0.0, 0.0);
    }

    // Calculate internal edges (cohesion)
    let mut internal_edges = 0;
    let mut external_edges = 0;

    for node in cluster {
        for neighbor in graph.neighbors(node) {
            if cluster.contains(neighbor) {
                internal_edges += 1;
            } else {
                external_edges += 1;
            }
        }
    }

    let total_possible = cluster.len() * (cluster.len() - 1);
    let cohesion = if total_possible > 0 {
        internal_edges as f32 / total_possible as f32
    } else {
        0.0
    };

    let coupling = if internal_edges + external_edges > 0 {
        external_edges as f32 / (internal_edges + external_edges) as f32
    } else {
        0.0
    };

    (cohesion, coupling)
}

/// Detect boundary violations between clusters
fn detect_boundary_violations(
    graph: &Graph,
    clusters: &[HashSet<String>],
    units: &[CodeUnit],
) -> Vec<BoundaryViolation> {
    let mut violations = Vec::new();

    // Create cluster lookup
    let mut node_to_cluster = HashMap::new();
    for (idx, cluster) in clusters.iter().enumerate() {
        for node in cluster {
            node_to_cluster.insert(node.clone(), idx);
        }
    }

    // Check for high coupling between modules
    for (node, &from_cluster) in &node_to_cluster {
        for neighbor in graph.neighbors(node) {
            if let Some(&to_cluster) = node_to_cluster.get(neighbor) {
                if from_cluster != to_cluster {
                    let from_name = units.iter()
                        .find(|u| &u.id == node)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| node.clone());

                    let to_name = units.iter()
                        .find(|u| &u.id == neighbor)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| neighbor.clone());

                    violations.push(BoundaryViolation {
                        from_module: format!("module_{}", from_cluster + 1),
                        to_module: format!("module_{}", to_cluster + 1),
                        violation_type: "cross_module_dependency".to_string(),
                        suggestion: format!("Consider moving {} to the same module as {}, or introduce an interface", from_name, to_name),
                    });
                }
            }
        }
    }

    // Deduplicate violations
    violations.sort_by(|a, b| {
        a.from_module.cmp(&b.from_module)
            .then(a.to_module.cmp(&b.to_module))
    });
    violations.dedup_by(|a, b| {
        a.from_module == b.from_module && a.to_module == b.to_module
    });

    violations
}

/// Generate refactoring suggestions
fn generate_refactoring_suggestions(
    modules: &[SuggestedModule],
    violations: &[BoundaryViolation],
    target_cohesion: f32,
    max_coupling: f32,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    // Suggest splitting high-coupling modules
    for module in modules {
        if module.coupling_score > max_coupling {
            suggestions.push(format!(
                "{}: High coupling ({:.2}), consider splitting into smaller modules",
                module.module_name, module.coupling_score
            ));
        }
        if module.cohesion_score < target_cohesion {
            suggestions.push(format!(
                "{}: Low cohesion ({:.2}), consider reorganizing components",
                module.module_name, module.cohesion_score
            ));
        }
    }

    // Suggest fixing boundary violations
    if violations.len() > 5 {
        suggestions.push(format!(
            "Found {} boundary violations - consider introducing facade interfaces",
            violations.len()
        ));
    }

    if suggestions.is_empty() {
        suggestions.push("Architecture appears well-structured".to_string());
    }

    suggestions
}

/// Check layering violations
fn check_layering_violations(
    graph: &Graph,
    units: &[CodeUnit],
    layers: &Option<Vec<Vec<String>>>,
    from_layer: &Option<String>,
    to_layer: &Option<String>,
    allowed: bool,
    description: &str,
) -> Vec<ArchitectureViolation> {
    let mut violations = Vec::new();

    if let Some(layers) = layers {
        // Build layer lookup
        let mut node_to_layer = HashMap::new();
        for (idx, layer) in layers.iter().enumerate() {
            for node in layer {
                node_to_layer.insert(node.clone(), idx);
            }
        }

        // Check violations
        for node in &graph.nodes {
            if let Some(&from_idx) = node_to_layer.get(node) {
                for neighbor in graph.neighbors(node) {
                    if let Some(&to_idx) = node_to_layer.get(neighbor) {
                        // Check if dependency violates layer rules
                        let violates = if !allowed {
                            // If not allowed, check if this matches the rule
                            if let (Some(from), Some(to)) = (from_layer, to_layer) {
                                from == &format!("layer_{}", from_idx) && to == &format!("layer_{}", to_idx)
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        if violates {
                            let from_name = units.iter()
                                .find(|u| &u.id == node)
                                .map(|u| u.qualified_name.clone())
                                .unwrap_or_else(|| node.clone());

                            let to_name = units.iter()
                                .find(|u| &u.id == neighbor)
                                .map(|u| u.qualified_name.clone())
                                .unwrap_or_else(|| neighbor.clone());

                            violations.push(ArchitectureViolation {
                                rule: description.to_string(),
                                from_component: from_name,
                                to_component: to_name,
                                violation_type: "layering".to_string(),
                                severity: "medium".to_string(),
                                description: "Dependency violates layering rules".to_string(),
                                suggestion: "Refactor to follow architectural layers".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Check dependency direction violations
fn check_dependency_direction_violations(
    graph: &Graph,
    units: &[CodeUnit],
    from_pattern: &Option<String>,
    to_pattern: &Option<String>,
    allowed: bool,
    description: &str,
) -> Vec<ArchitectureViolation> {
    let mut violations = Vec::new();

    if let (Some(from), Some(to)) = (from_pattern, to_pattern) {
        for node in &graph.nodes {
            let node_unit = units.iter().find(|u| &u.id == node);
            if let Some(unit) = node_unit {
                if unit.qualified_name.contains(from) {
                    for neighbor in graph.neighbors(node) {
                        let neighbor_unit = units.iter().find(|u| &u.id == neighbor);
                        if let Some(dep_unit) = neighbor_unit {
                            if dep_unit.qualified_name.contains(to) && !allowed {
                                violations.push(ArchitectureViolation {
                                    rule: description.to_string(),
                                    from_component: unit.qualified_name.clone(),
                                    to_component: dep_unit.qualified_name.clone(),
                                    violation_type: "dependency_direction".to_string(),
                                    severity: "high".to_string(),
                                    description: "Dependency violates architectural rules".to_string(),
                                    suggestion: "Reverse the dependency or introduce an abstraction".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    violations
}

/// Check module isolation violations
fn check_module_isolation_violations(
    graph: &Graph,
    units: &[CodeUnit],
    module_pattern: &Option<String>,
    description: &str,
) -> Vec<ArchitectureViolation> {
    let mut violations = Vec::new();

    if let Some(pattern) = module_pattern {
        // Find all nodes in the module
        let module_nodes: HashSet<String> = units.iter()
            .filter(|u| u.qualified_name.contains(pattern))
            .map(|u| u.id.clone())
            .collect();

        // Check for external dependencies
        for node in &module_nodes {
            for neighbor in graph.neighbors(node) {
                if !module_nodes.contains(neighbor) {
                    let node_name = units.iter()
                        .find(|u| &u.id == node)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| node.clone());

                    let neighbor_name = units.iter()
                        .find(|u| &u.id == neighbor)
                        .map(|u| u.qualified_name.clone())
                        .unwrap_or_else(|| neighbor.clone());

                    violations.push(ArchitectureViolation {
                        rule: description.to_string(),
                        from_component: node_name,
                        to_component: neighbor_name,
                        violation_type: "module_isolation".to_string(),
                        severity: "medium".to_string(),
                        description: "Module has external dependencies".to_string(),
                        suggestion: "Minimize external dependencies or use dependency injection".to_string(),
                    });
                }
            }
        }
    }

    violations
}

struct ArchitectureMetrics {
    avg_degree: f32,
    max_degree: usize,
    highest_coupled_component: Option<String>,
}

/// Calculate architecture metrics
fn calculate_architecture_metrics(graph: &Graph, _units: &[CodeUnit]) -> ArchitectureMetrics {
    let total_degree: usize = graph.nodes.iter()
        .map(|n| graph.total_degree(n))
        .sum();

    let avg_degree = if !graph.nodes.is_empty() {
        total_degree as f32 / graph.nodes.len() as f32
    } else {
        0.0
    };

    let (max_degree, highest_coupled_component) = graph.nodes.iter()
        .map(|n| (graph.total_degree(n), n.clone()))
        .max_by_key(|(degree, _)| *degree)
        .unwrap_or((0, String::new()));

    ArchitectureMetrics {
        avg_degree,
        max_degree,
        highest_coupled_component: if max_degree > 0 {
            Some(highest_coupled_component)
        } else {
            None
        },
    }
}

fn default_mermaid() -> String {
    "mermaid".to_string()
}

fn default_dependency_view() -> String {
    "dependency".to_string()
}

fn default_depth() -> i32 {
    3
}

fn default_all_pattern_categories() -> Vec<String> {
    vec![
        "creational".to_string(),
        "structural".to_string(),
        "behavioral".to_string(),
    ]
}

fn default_high_confidence() -> f32 {
    0.8
}

fn default_high_cohesion() -> f32 {
    0.7
}

fn default_low_coupling() -> f32 {
    0.3
}

fn default_true() -> bool {
    true
}

fn default_all_drift_types() -> Vec<String> {
    vec![
        "dependency".to_string(),
        "complexity".to_string(),
        "coupling".to_string(),
    ]
}
