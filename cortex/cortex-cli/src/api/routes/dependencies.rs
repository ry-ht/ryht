//! Dependencies and Analysis API routes

use crate::api::types::*;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use cortex_storage::ConnectionManager;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// Context for dependency routes
#[derive(Clone)]
pub struct DependencyContext {
    pub storage: Arc<ConnectionManager>,
}

/// Create dependency routes
pub fn dependency_routes(context: DependencyContext) -> Router {
    Router::new()
        .route(
            "/api/v1/workspaces/:id/dependencies",
            get(get_dependency_graph),
        )
        .route("/api/v1/analysis/impact", post(analyze_impact))
        .route("/api/v1/analysis/cycles", get(detect_cycles))
        .with_state(context)
}

/// GET /api/v1/workspaces/{id}/dependencies - Get dependency graph
async fn get_dependency_graph(
    State(context): State<DependencyContext>,
    Path(workspace_id): Path<String>,
    Query(params): Query<DependencyGraphRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        workspace_id = %workspace_id,
        "Getting dependency graph"
    );

    match get_dependency_graph_impl(&context, &workspace_id, params).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to get dependency graph");
            let api_response = ApiResponse::<DependencyGraphResponse>::error(
                e.to_string(),
                request_id,
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn get_dependency_graph_impl(
    context: &DependencyContext,
    workspace_id: &str,
    params: DependencyGraphRequest,
) -> anyhow::Result<DependencyGraphResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    let format = params.format.as_deref().unwrap_or("json");
    let _level = params.level.as_deref().unwrap_or("file");
    let max_depth = params.max_depth.unwrap_or(10);
    let _include_external = params.include_external.unwrap_or(false);

    // Query code units
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

        nodes.push(GraphNode {
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

        edges.push(GraphEdge {
            from: from.clone(),
            to: to.clone(),
            edge_type,
            weight,
        });
    }

    // Detect cycles (simple cycle detection)
    let cycle_count = detect_cycles_in_graph(&nodes, &edges);

    // Generate content based on format
    let content = match format {
        "dot" => generate_dot_format(&nodes, &edges),
        "mermaid" => generate_mermaid_format(&nodes, &edges),
        _ => generate_json_format(&nodes, &edges),
    };

    let stats = GraphStats {
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        max_depth,
        cycles_detected: cycle_count,
    };

    Ok(DependencyGraphResponse {
        format: format.to_string(),
        content,
        nodes,
        edges,
        stats,
    })
}

/// POST /api/v1/analysis/impact - Analyze impact of changes
async fn analyze_impact(
    State(context): State<DependencyContext>,
    Json(request): Json<ImpactAnalysisRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        entity_count = request.changed_entity_ids.len(),
        "Analyzing impact"
    );

    match analyze_impact_impl(&context, request).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to analyze impact");
            let api_response = ApiResponse::<ImpactAnalysisResponse>::error(
                e.to_string(),
                request_id,
            );
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn analyze_impact_impl(
    context: &DependencyContext,
    request: ImpactAnalysisRequest,
) -> anyhow::Result<ImpactAnalysisResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();
    let analysis_type = request.analysis_type.as_deref().unwrap_or("full");

    let mut changed_entities = Vec::new();
    let mut affected_entities = Vec::new();
    let mut all_affected_ids: HashSet<String> = HashSet::new();

    // For each changed entity, find dependents
    for entity_id in &request.changed_entity_ids {
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

            changed_entities.push(EntityImpact {
                id: entity_id.clone(),
                name,
                entity_type,
                impact_level: "changed".to_string(),
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
                    .filter(|id| request.changed_entity_ids.contains(id))
                    .collect();

                affected_entities.push(EntityImpact {
                    id: affected_id.clone(),
                    name,
                    entity_type,
                    impact_level: "affected".to_string(),
                    affected_by,
                    affects: vec![],
                });
            }
        }
    }

    // Calculate risk
    let risk_score = (all_affected_ids.len() as f64 / 100.0).min(1.0);
    let overall_risk = if risk_score > 0.7 {
        "high"
    } else if risk_score > 0.3 {
        "medium"
    } else {
        "low"
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

    // Calculate critical paths using longest path algorithm
    let critical_paths = calculate_critical_paths(
        &context,
        &request.changed_entity_ids,
        &all_affected_ids,
    ).await?;

    let risk_assessment = RiskAssessment {
        overall_risk: overall_risk.to_string(),
        risk_score,
        total_affected: all_affected_ids.len(),
        critical_paths,
        recommendations,
    };

    Ok(ImpactAnalysisResponse {
        changed_entities,
        affected_entities,
        risk_assessment,
        analysis_type: analysis_type.to_string(),
    })
}

/// GET /api/v1/analysis/cycles - Detect circular dependencies
async fn detect_cycles(State(context): State<DependencyContext>) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(request_id = %request_id, "Detecting cycles");

    match detect_cycles_impl(&context).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to detect cycles");
            let api_response =
                ApiResponse::<CycleDetectionResponse>::error(e.to_string(), request_id);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn detect_cycles_impl(
    context: &DependencyContext,
) -> anyhow::Result<CycleDetectionResponse> {
    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    // Get all relations
    let relations_query = "SELECT * FROM relation";
    let mut result = conn.query(relations_query).await?;
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
    let cycles = find_cycles_dfs(&graph);

    let max_cycle_length = cycles.iter().map(|c| c.len()).max().unwrap_or(0);

    let cycle_responses: Vec<DependencyCycle> = cycles
        .into_iter()
        .enumerate()
        .map(|(idx, entities)| {
            let severity = if entities.len() > 5 {
                "high"
            } else if entities.len() > 3 {
                "medium"
            } else {
                "low"
            };

            DependencyCycle {
                cycle_id: format!("cycle_{}", idx),
                cycle_length: entities.len(),
                entities,
                severity: severity.to_string(),
                suggestions: vec![
                    "Consider extracting shared functionality".to_string(),
                    "Use dependency inversion".to_string(),
                    "Refactor to remove circular reference".to_string(),
                ],
            }
        })
        .collect();

    Ok(CycleDetectionResponse {
        total_cycles: cycle_responses.len(),
        max_cycle_length,
        cycles: cycle_responses,
    })
}

// Helper functions

fn detect_cycles_in_graph(_nodes: &[GraphNode], edges: &[GraphEdge]) -> usize {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    for edge in edges {
        graph
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }

    find_cycles_dfs(&graph).len()
}

fn find_cycles_dfs(graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            dfs_visit(
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
                dfs_visit(neighbor, graph, visited, rec_stack, path, cycles);
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

fn generate_dot_format(nodes: &[GraphNode], edges: &[GraphEdge]) -> String {
    let mut dot = String::from("digraph G {\n");

    for node in nodes {
        dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", node.id, node.name));
    }

    for edge in edges {
        dot.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            edge.from, edge.to, edge.edge_type
        ));
    }

    dot.push_str("}\n");
    dot
}

fn generate_mermaid_format(_nodes: &[GraphNode], edges: &[GraphEdge]) -> String {
    let mut mermaid = String::from("graph TD\n");

    for edge in edges {
        mermaid.push_str(&format!(
            "  {}[{}] -->|{}| {}[{}]\n",
            sanitize_mermaid_id(&edge.from),
            edge.from,
            edge.edge_type,
            sanitize_mermaid_id(&edge.to),
            edge.to
        ));
    }

    mermaid
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace([':', '-', '.'], "_")
}

fn generate_json_format(nodes: &[GraphNode], edges: &[GraphEdge]) -> String {
    serde_json::json!({
        "nodes": nodes,
        "edges": edges
    })
    .to_string()
}

/// Calculate critical paths in dependency graph using longest path algorithm
/// Returns paths from changed entities that represent critical dependencies
async fn calculate_critical_paths(
    context: &DependencyContext,
    changed_entity_ids: &[String],
    affected_ids: &HashSet<String>,
) -> anyhow::Result<Vec<Vec<String>>> {
    if changed_entity_ids.is_empty() || affected_ids.is_empty() {
        return Ok(vec![]);
    }

    let pooled = context.storage.acquire().await?;
    let conn = pooled.connection();

    // Build dependency graph for affected entities
    let mut graph: HashMap<String, Vec<(String, f64)>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all nodes
    for entity_id in changed_entity_ids.iter().chain(affected_ids.iter()) {
        in_degree.entry(entity_id.clone()).or_insert(0);
    }

    // Query relations for affected entities
    let all_entity_ids: Vec<String> = changed_entity_ids
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
            if affected_ids.contains(&target_id) || changed_entity_ids.contains(&target_id) {
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
    for entity_id in changed_entity_ids {
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
            if changed_entity_ids.contains(&current) {
                break;
            }
        }

        path.reverse();
        critical_paths.push(path);
    }

    Ok(critical_paths)
}
