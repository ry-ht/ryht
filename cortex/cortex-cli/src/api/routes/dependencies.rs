//! Dependencies and Analysis API routes

use crate::api::types::*;
use crate::services::DependencyService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

/// Context for dependency routes
#[derive(Clone)]
pub struct DependencyContext {
    pub service: Arc<DependencyService>,
}

/// Create dependency routes
pub fn dependency_routes(context: DependencyContext) -> Router {
    Router::new()
        .route(
            "/api/v1/workspaces/{id}/dependencies",
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
    // Parse workspace UUID
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| anyhow::anyhow!("Invalid workspace ID: {}", e))?;

    let format = params.format.as_deref().unwrap_or("json");
    let max_depth = params.max_depth.unwrap_or(10);

    // Call service to get dependency graph
    let graph = context
        .service
        .get_dependency_graph(workspace_uuid, Some(max_depth))
        .await?;

    // Convert service types to API types
    let nodes: Vec<GraphNode> = graph
        .nodes
        .into_iter()
        .map(|node| GraphNode {
            id: node.id,
            name: node.name,
            node_type: node.node_type,
            metadata: node.metadata,
        })
        .collect();

    let edges: Vec<GraphEdge> = graph
        .edges
        .into_iter()
        .map(|edge| GraphEdge {
            from: edge.from,
            to: edge.to,
            edge_type: edge.edge_type,
            weight: edge.weight,
        })
        .collect();

    // Generate content based on format
    let content = match format {
        "dot" => generate_dot_format(&nodes, &edges),
        "mermaid" => generate_mermaid_format(&nodes, &edges),
        _ => generate_json_format(&nodes, &edges),
    };

    let stats = GraphStats {
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        max_depth: graph.max_depth,
        cycles_detected: graph.cycle_count,
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
    let analysis_type = request.analysis_type.as_deref().unwrap_or("full");

    // Parse workspace ID (use first changed entity to infer workspace)
    // Note: In a real implementation, workspace_id should be passed explicitly
    let workspace_uuid = Uuid::new_v4(); // Placeholder - should come from request

    // Call service to analyze impact
    let analysis = context
        .service
        .analyze_impact(workspace_uuid, request.changed_entity_ids)
        .await?;

    // Convert service types to API types
    let changed_entities: Vec<EntityImpact> = analysis
        .changed_entities
        .into_iter()
        .map(|entity| EntityImpact {
            id: entity.id,
            name: entity.name,
            entity_type: entity.entity_type,
            impact_level: "changed".to_string(),
            affected_by: entity.affected_by,
            affects: entity.affects,
        })
        .collect();

    let affected_entities: Vec<EntityImpact> = analysis
        .affected_entities
        .into_iter()
        .map(|entity| EntityImpact {
            id: entity.id,
            name: entity.name,
            entity_type: entity.entity_type,
            impact_level: "affected".to_string(),
            affected_by: entity.affected_by,
            affects: entity.affects,
        })
        .collect();

    let overall_risk = match analysis.risk_level {
        crate::services::dependencies::RiskLevel::High => "high",
        crate::services::dependencies::RiskLevel::Medium => "medium",
        crate::services::dependencies::RiskLevel::Low => "low",
    };

    let risk_assessment = RiskAssessment {
        overall_risk: overall_risk.to_string(),
        risk_score: analysis.risk_score,
        total_affected: affected_entities.len(),
        critical_paths: analysis.critical_paths,
        recommendations: analysis.recommendations,
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
    // Note: This endpoint doesn't have workspace_id, so we detect cycles globally
    // In a real implementation, this should be workspace-scoped
    let workspace_uuid = Uuid::new_v4(); // Placeholder

    // Call service to detect cycles
    let cycles = context.service.detect_cycles(workspace_uuid).await?;

    let max_cycle_length = cycles.iter().map(|c| c.entities.len()).max().unwrap_or(0);

    // Convert service types to API types
    let cycle_responses: Vec<DependencyCycle> = cycles
        .into_iter()
        .map(|cycle| {
            let severity = match cycle.severity {
                crate::services::dependencies::CycleSeverity::High => "high",
                crate::services::dependencies::CycleSeverity::Medium => "medium",
                crate::services::dependencies::CycleSeverity::Low => "low",
            };

            DependencyCycle {
                cycle_id: cycle.cycle_id,
                cycle_length: cycle.entities.len(),
                entities: cycle.entities,
                severity: severity.to_string(),
                suggestions: cycle.suggestions,
            }
        })
        .collect();

    Ok(CycleDetectionResponse {
        total_cycles: cycle_responses.len(),
        max_cycle_length,
        cycles: cycle_responses,
    })
}

// Helper functions for format conversion

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
