//! Agent Launch Tool - Launch specialized agents for tasks
//!
//! Direct integration with Cortex subsystems (no HTTP bridge).

use crate::mcp::tools::agent_registry::{AgentExecution, AgentRegistry, ExecutionStatus};
use cortex_intelligence::CortexBridge;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use cortex_memory::SemanticMemorySystem;
use mcp_sdk::prelude::*;
use async_trait::async_trait;

/// Agent launch tool input
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct AgentLaunchInput {
    /// Agent type to launch
    pub agent_type: String,

    /// Task description
    pub task: String,

    /// Workspace ID (optional)
    pub workspace_id: Option<String>,

    /// Additional parameters (agent-specific)
    pub params: Option<serde_json::Value>,
}

/// Agent launch tool output
#[derive(Debug, Serialize)]
pub struct AgentLaunchOutput {
    /// Agent ID
    pub agent_id: String,

    /// Agent type
    pub agent_type: String,

    /// Status
    pub status: String,

    /// Message
    pub message: String,
}

/// Context for agent launch with direct subsystem references
#[derive(Clone)]
pub struct AgentLaunchContext {
    /// Agent registry for tracking executions
    pub registry: Arc<AgentRegistry>,
    /// Virtual filesystem for file access
    pub vfs: Arc<VirtualFileSystem>,
    /// Semantic memory system
    pub memory: Arc<SemanticMemorySystem>,
    /// Storage backend
    pub storage: Arc<ConnectionManager>,
    /// Legacy bridge for agent execution (temporary)
    pub cortex: Arc<CortexBridge>,
}

impl AgentLaunchContext {
    /// Create a new AgentLaunchContext
    pub fn new(
        registry: Arc<AgentRegistry>,
        vfs: Arc<VirtualFileSystem>,
        memory: Arc<SemanticMemorySystem>,
        storage: Arc<ConnectionManager>,
        cortex: Arc<CortexBridge>,
    ) -> Self {
        Self {
            registry,
            vfs,
            memory,
            storage,
            cortex,
        }
    }
}

/// Agent launch tool
pub struct AgentLaunchTool {
    context: AgentLaunchContext,
}

impl AgentLaunchTool {
    /// Create new agent launch tool
    pub fn new(context: AgentLaunchContext) -> Self {
        Self { context }
    }

    /// Launch agent
    pub async fn launch(&self, input: AgentLaunchInput) -> Result<AgentLaunchOutput> {
        let agent_id = format!("{}-{}", input.agent_type, Uuid::new_v4());

        // Create execution record
        let execution = AgentExecution::new(
            agent_id.clone(),
            input.agent_type.clone(),
            input.task.clone(),
            input.workspace_id.clone(),
            None,
        );

        self.context.registry.register(execution).await?;

        // Launch agent based on type
        let agent_type = input.agent_type.clone();
        let agent_type_str = agent_type.clone();
        let task = input.task.clone();
        let workspace_id = input.workspace_id.clone();
        let params = input.params.clone();

        let registry = Arc::clone(&self.context.registry);
        let cortex = Arc::clone(&self.context.cortex);
        let agent_id_clone = agent_id.clone();

        // Update status to Running before spawning execution
        if let Err(e) = self.context.registry.update_status(&agent_id, ExecutionStatus::Running).await {
            tracing::error!(agent_id = %agent_id, error = %e, "Failed to update agent status to Running");
            return Err(e);
        }
        tracing::info!(agent_id = %agent_id, agent_type = %agent_type, "Agent status updated to Running");

        // Spawn agent task
        tokio::spawn(async move {
            tracing::info!(agent_id = %agent_id_clone, agent_type = %agent_type_str, "Agent execution task started");

            let result = Self::execute_agent(
                &agent_type_str,
                &task,
                workspace_id.as_deref(),
                params,
                cortex,
            )
            .await;

            match result {
                Ok(output) => {
                    tracing::info!(agent_id = %agent_id_clone, "Agent execution completed successfully");

                    if let Err(e) = registry.update_status(&agent_id_clone, ExecutionStatus::Completed).await {
                        tracing::error!(agent_id = %agent_id_clone, error = %e, "Failed to update agent status to Completed");
                    }

                    if let Err(e) = registry.set_result(&agent_id_clone, output).await {
                        tracing::error!(agent_id = %agent_id_clone, error = %e, "Failed to set agent result");
                    }
                }
                Err(e) => {
                    tracing::error!(agent_id = %agent_id_clone, error = %e, "Agent execution failed");

                    if let Err(err) = registry.set_error(&agent_id_clone, e.to_string()).await {
                        tracing::error!(agent_id = %agent_id_clone, error = %err, "Failed to set agent error");
                    }
                }
            }

            tracing::info!(agent_id = %agent_id_clone, "Agent execution task finished");
        });

        Ok(AgentLaunchOutput {
            agent_id,
            agent_type,
            status: "launched".to_string(),
            message: "Agent launched successfully".to_string(),
        })
    }

    /// Execute agent based on type
    async fn execute_agent(
        agent_type: &str,
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        match agent_type {
            "developer" => {
                Self::execute_developer(task, workspace_id, params, cortex).await
            }
            "tester" => {
                Self::execute_tester(task, workspace_id, params, cortex).await
            }
            "reviewer" => {
                Self::execute_reviewer(task, workspace_id, params, cortex).await
            }
            "architect" => {
                Self::execute_architect(task, workspace_id, params, cortex).await
            }
            "researcher" => {
                Self::execute_researcher(task, workspace_id, params, cortex).await
            }
            "optimizer" => {
                Self::execute_optimizer(task, workspace_id, params, cortex).await
            }
            "documenter" => {
                Self::execute_documenter(task, workspace_id, params, cortex).await
            }
            _ => anyhow::bail!("Unknown agent type: {}", agent_type),
        }
    }

    async fn execute_developer(
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::developer::{DeveloperAgent, CodeSpec};
        use cortex_types::WorkspaceId;

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let target_path = params.get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("src/generated.rs")
            .to_string();
        let language = params.get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("rust")
            .to_string();
        let feature_type = params.get("feature_type")
            .and_then(|v| v.as_str())
            .unwrap_or("function")
            .to_string();

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .unwrap_or_else(|| WorkspaceId::from("default".to_string()));

        // Create code specification
        let spec = CodeSpec {
            description: task.to_string(),
            target_path,
            language: language.clone(),
            workspace_id: ws_id,
            feature_type,
        };

        // Create developer agent with Cortex
        let agent = DeveloperAgent::with_cortex("mcp-developer".to_string(), cortex);

        // Generate code
        let result = agent.generate_code(spec).await?;

        // Return result as JSON
        Ok(serde_json::json!({
            "status": "completed",
            "code": {
                "content": result.content,
                "language": result.language,
                "path": result.path,
            },
            "metadata": {
                "patterns_used": result.metadata.patterns_used,
                "similar_code_count": result.metadata.similar_code_count,
                "episodes_consulted": result.metadata.episodes_consulted,
                "generation_time_ms": result.metadata.generation_time_ms,
            }
        }))
    }

    async fn execute_tester(
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::tester::{TesterAgent, TestSpec, TestType};
        use cortex_types::WorkspaceId;

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let target_path = params.get("target_path")
            .and_then(|v| v.as_str())
            .unwrap_or("src/lib.rs")
            .to_string();
        let test_type = params.get("test_type")
            .and_then(|v| v.as_str())
            .map(|t| match t {
                "integration" => TestType::Integration,
                "e2e" => TestType::EndToEnd,
                "property" => TestType::Property,
                _ => TestType::Unit,
            })
            .unwrap_or(TestType::Unit);
        let coverage_target = params.get("coverage_target")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8) as f32;

        // Determine operation mode
        let mode = params.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("generate");

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .unwrap_or_else(|| WorkspaceId::from("default".to_string()));

        // Create tester agent with Cortex
        let agent = TesterAgent::with_cortex("mcp-tester".to_string(), cortex);

        match mode {
            "generate" => {
                // Generate tests
                let spec = TestSpec {
                    target_path,
                    test_type,
                    coverage_target,
                    workspace_id: ws_id,
                };

                let result = agent.generate_tests(spec).await?;

                Ok(serde_json::json!({
                    "status": "completed",
                    "mode": "generate",
                    "test_suite": {
                        "path": result.path,
                        "content": result.content,
                        "test_count": result.test_count,
                        "estimated_coverage": result.estimated_coverage,
                    },
                    "metadata": {
                        "patterns_used": result.metadata.patterns_used,
                        "similar_tests_count": result.metadata.similar_tests_count,
                        "episodes_consulted": result.metadata.episodes_consulted,
                        "generation_time_ms": result.metadata.generation_time_ms,
                    }
                }))
            }
            "execute" => {
                // Execute existing tests
                let test_suite_path = params.get("test_suite_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&target_path);

                let result = agent.execute_tests(&ws_id, test_suite_path).await?;

                Ok(serde_json::json!({
                    "status": "completed",
                    "mode": "execute",
                    "results": {
                        "suite_path": result.suite_path,
                        "passed": result.passed,
                        "failed": result.failed,
                        "skipped": result.skipped,
                        "coverage": result.coverage,
                        "execution_time_ms": result.execution_time_ms,
                    },
                    "failures": result.failures.iter().map(|f| {
                        serde_json::json!({
                            "test_name": f.test_name,
                            "error": f.error,
                            "stack_trace": f.stack_trace,
                        })
                    }).collect::<Vec<_>>(),
                }))
            }
            _ => {
                anyhow::bail!("Unknown tester mode: {}", mode);
            }
        }
    }

    async fn execute_reviewer(
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::reviewer::ReviewerAgent;
        use cortex_types::{WorkspaceId, SessionId};

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let file_path = params.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("file_path parameter is required for reviewer"))?;
        let session_id = params.get("session_id")
            .and_then(|v| v.as_str())
            .map(|id| SessionId::from(id.to_string()))
            .ok_or_else(|| anyhow::anyhow!("session_id parameter is required for reviewer"))?;

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .unwrap_or_else(|| WorkspaceId::from("default".to_string()));

        // Create reviewer agent with Cortex
        let agent = ReviewerAgent::with_cortex("mcp-reviewer".to_string(), cortex);

        // Review code
        let report = agent.review_code(&ws_id, &session_id, file_path).await?;

        // Return result as JSON
        Ok(serde_json::json!({
            "status": "completed",
            "review": {
                "summary": report.summary,
                "quality_score": report.quality_score,
                "test_coverage": report.test_coverage,
                "is_acceptable": report.is_acceptable(),
                "issues": report.issues.iter().map(|issue| {
                    serde_json::json!({
                        "severity": format!("{:?}", issue.severity),
                        "category": issue.category,
                        "description": issue.description,
                        "file_path": issue.file_path,
                        "line_number": issue.line_number,
                        "suggestion": issue.suggestion,
                        "pattern_name": issue.pattern_name,
                    })
                }).collect::<Vec<_>>(),
                "static_analysis": {
                    "avg_cyclomatic": report.static_analysis.complexity_metrics.avg_cyclomatic,
                    "avg_cognitive": report.static_analysis.complexity_metrics.avg_cognitive,
                    "total_loc": report.static_analysis.complexity_metrics.total_loc,
                },
                "security": {
                    "vulnerabilities": report.security_analysis.vulnerabilities,
                },
                "best_practices_score": report.best_practices.score,
                "performance_bottlenecks": report.performance_analysis.bottlenecks,
            }
        }))
    }

    async fn execute_architect(
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::architect::{ArchitectAgent, SystemRequirements, ScaleRequirements, QualityAttribute};
        use cortex_types::WorkspaceId;

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let mode = params.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("design");

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .unwrap_or_else(|| WorkspaceId::from("default".to_string()));

        // Create architect agent with Cortex
        let agent = ArchitectAgent::with_cortex("mcp-architect".to_string(), cortex, ws_id.clone());

        match mode {
            "design" => {
                // Design system architecture
                let system_type = params.get("system_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or(task)
                    .to_string();
                let users = params.get("users")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10_000);
                let requests_per_second = params.get("requests_per_second")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);
                let data_volume_gb = params.get("data_volume_gb")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100);

                let requirements = SystemRequirements {
                    system_type,
                    scale: ScaleRequirements {
                        users,
                        requests_per_second,
                        data_volume_gb,
                    },
                    quality_attributes: vec![
                        QualityAttribute::Performance,
                        QualityAttribute::Scalability,
                        QualityAttribute::Maintainability,
                    ],
                    constraints: vec![],
                    integrations: vec![],
                };

                let architecture = agent.design_system(requirements)?;

                Ok(serde_json::json!({
                    "status": "completed",
                    "mode": "design",
                    "architecture": {
                        "summary": architecture.summary,
                        "style": format!("{:?}", architecture.style),
                        "patterns_used": architecture.patterns_used,
                        "components": architecture.components.iter().map(|c| {
                            serde_json::json!({
                                "name": c.name,
                                "description": c.description,
                                "responsibilities": c.responsibilities,
                                "dependencies": c.dependencies,
                            })
                        }).collect::<Vec<_>>(),
                        "decisions": architecture.decisions.iter().map(|d| {
                            serde_json::json!({
                                "decision": d.decision,
                                "rationale": d.rationale,
                                "alternatives": d.alternatives_considered,
                                "trade_offs": d.trade_offs,
                            })
                        }).collect::<Vec<_>>(),
                    }
                }))
            }
            "analyze" => {
                // Analyze dependencies
                let modules = params.get("modules")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| vec!["module_a".to_string(), "module_b".to_string()]);

                let analysis = agent.analyze_dependencies_async(modules).await?;

                Ok(serde_json::json!({
                    "status": "completed",
                    "mode": "analyze",
                    "analysis": {
                        "total_dependencies": analysis.total_dependencies,
                        "max_depth": analysis.max_depth,
                        "circular_dependencies": analysis.circular_dependencies.iter().map(|cd| {
                            serde_json::json!({
                                "cycle": cd.cycle,
                                "severity": format!("{:?}", cd.severity),
                            })
                        }).collect::<Vec<_>>(),
                        "recommendations": analysis.recommendations,
                    }
                }))
            }
            _ => {
                anyhow::bail!("Unknown architect mode: {}", mode);
            }
        }
    }

    async fn execute_researcher(
        task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::researcher::{ResearcherAgent, ResearchQuery, QueryType, ResearchScope};
        use cortex_types::WorkspaceId;

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let query_type = params.get("query_type")
            .and_then(|v| v.as_str())
            .map(|t| match t {
                "comparison" => QueryType::TechnologyComparison,
                "best_practices" => QueryType::BestPractices,
                "trend" => QueryType::TrendAnalysis,
                "fact_check" => QueryType::FactChecking,
                "problem_solving" => QueryType::ProblemSolving,
                _ => QueryType::General,
            })
            .unwrap_or(QueryType::General);
        let scope = params.get("scope")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "local" => ResearchScope::Local,
                "organization" => ResearchScope::Organization,
                "public" => ResearchScope::Public,
                _ => ResearchScope::Combined,
            })
            .unwrap_or(ResearchScope::Combined);
        let max_results = params.get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let quality_threshold = params.get("quality_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7) as f32;

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .unwrap_or_else(|| WorkspaceId::from("default".to_string()));

        // Create researcher agent with Cortex
        let agent = ResearcherAgent::with_cortex("mcp-researcher".to_string(), cortex, ws_id);

        // Create research query
        let query = ResearchQuery {
            query: task.to_string(),
            query_type,
            scope,
            max_results,
            time_range: None,
            quality_threshold,
        };

        // Conduct research
        let report = agent.research_async(query).await?;

        // Return result as JSON
        Ok(serde_json::json!({
            "status": "completed",
            "research": {
                "query": report.query,
                "summary": report.summary,
                "confidence": report.confidence,
                "findings": report.key_findings.iter().map(|f| {
                    serde_json::json!({
                        "title": f.title,
                        "description": f.description,
                        "relevance": f.relevance,
                        "confidence": f.confidence,
                        "sources": f.sources,
                        "tags": f.tags,
                    })
                }).collect::<Vec<_>>(),
                "sources": report.sources.iter().map(|s| {
                    serde_json::json!({
                        "title": s.title,
                        "url": s.url,
                        "source_type": format!("{:?}", s.source_type),
                        "quality_score": s.quality_score,
                    })
                }).collect::<Vec<_>>(),
                "recommendations": report.recommendations,
                "related_topics": report.related_topics,
            }
        }))
    }

    async fn execute_optimizer(
        task: &str,
        _workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        _cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::optimizer::{OptimizerAgent, OptimizationTarget, OptimizationType, PerformanceMetrics};

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let target_name = params.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or(task)
            .to_string();
        let optimization_type = params.get("optimization_type")
            .and_then(|v| v.as_str())
            .map(|t| match t {
                "cost" => OptimizationType::Cost,
                "resources" => OptimizationType::Resources,
                "throughput" => OptimizationType::Throughput,
                "latency" => OptimizationType::Latency,
                "balanced" => OptimizationType::Balanced,
                _ => OptimizationType::Performance,
            })
            .unwrap_or(OptimizationType::Performance);

        // Parse current metrics
        let current_metrics = PerformanceMetrics {
            execution_time_ms: params.get("current_execution_time_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(1000.0),
            memory_usage_mb: params.get("current_memory_mb")
                .and_then(|v| v.as_f64())
                .unwrap_or(512.0),
            cpu_usage_percent: params.get("current_cpu_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(75.0),
            throughput_ops: params.get("current_throughput")
                .and_then(|v| v.as_f64())
                .unwrap_or(100.0),
            cost_per_op_cents: params.get("current_cost_cents")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.01),
            io_operations: params.get("current_io_ops")
                .and_then(|v| v.as_u64())
                .unwrap_or(1000),
        };

        // Parse target metrics
        let target_metrics = PerformanceMetrics {
            execution_time_ms: params.get("target_execution_time_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(current_metrics.execution_time_ms * 0.5),
            memory_usage_mb: params.get("target_memory_mb")
                .and_then(|v| v.as_f64())
                .unwrap_or(current_metrics.memory_usage_mb * 0.7),
            cpu_usage_percent: params.get("target_cpu_percent")
                .and_then(|v| v.as_f64())
                .unwrap_or(current_metrics.cpu_usage_percent * 0.7),
            throughput_ops: params.get("target_throughput")
                .and_then(|v| v.as_f64())
                .unwrap_or(current_metrics.throughput_ops * 1.5),
            cost_per_op_cents: params.get("target_cost_cents")
                .and_then(|v| v.as_f64())
                .unwrap_or(current_metrics.cost_per_op_cents * 0.7),
            io_operations: params.get("target_io_ops")
                .and_then(|v| v.as_u64())
                .unwrap_or((current_metrics.io_operations as f64 * 0.7) as u64),
        };

        // Create optimizer agent
        let agent = OptimizerAgent::new("mcp-optimizer".to_string());

        // Create optimization target
        let target = OptimizationTarget {
            target: target_name,
            optimization_type,
            current_metrics,
            target_metrics,
            constraints: vec![],
        };

        // Optimize
        let report = agent.optimize(target)?;

        // Return result as JSON
        Ok(serde_json::json!({
            "status": "completed",
            "optimization": {
                "target": report.target,
                "improvement": {
                    "speed_improvement": report.improvement.speed_improvement,
                    "memory_reduction": report.improvement.memory_reduction,
                    "cost_reduction": report.improvement.cost_reduction,
                    "overall_score": report.improvement.overall_score,
                },
                "optimizations": report.optimizations.iter().map(|opt| {
                    serde_json::json!({
                        "description": opt.description,
                        "strategy": format!("{:?}", opt.strategy),
                        "improvement_percent": opt.improvement_percent,
                        "before": {
                            "execution_time_ms": opt.before.execution_time_ms,
                            "memory_usage_mb": opt.before.memory_usage_mb,
                            "cpu_usage_percent": opt.before.cpu_usage_percent,
                        },
                        "after": {
                            "execution_time_ms": opt.after.execution_time_ms,
                            "memory_usage_mb": opt.after.memory_usage_mb,
                            "cpu_usage_percent": opt.after.cpu_usage_percent,
                        },
                        "changes_required": opt.changes_required,
                    })
                }).collect::<Vec<_>>(),
                "bottlenecks": report.bottlenecks.iter().map(|b| {
                    serde_json::json!({
                        "location": b.location,
                        "type": format!("{:?}", b.bottleneck_type),
                        "severity": b.severity,
                        "impact": b.impact,
                        "suggested_fixes": b.suggested_fixes,
                    })
                }).collect::<Vec<_>>(),
                "recommendations": report.recommendations,
                "validation": {
                    "valid": report.validation.valid,
                    "tests_passed": report.validation.tests_passed,
                    "tests_failed": report.validation.tests_failed,
                    "regression_detected": report.validation.regression_detected,
                },
            }
        }))
    }

    async fn execute_documenter(
        _task: &str,
        workspace_id: Option<&str>,
        params: Option<serde_json::Value>,
        cortex: Arc<CortexBridge>,
    ) -> Result<serde_json::Value> {
        use cortex_agents::documenter::{DocumenterAgent, DocType};
        use cortex_types::WorkspaceId;

        // Parse parameters
        let params = params.unwrap_or(serde_json::json!({}));
        let file_path = params.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("file_path parameter is required for documenter"))?;
        let doc_types = params.get("doc_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| match s {
                        "readme" => DocType::ReadMe,
                        "api" => DocType::ApiDoc,
                        "architecture" => DocType::ArchitectureDiagram,
                        "module" => DocType::ModuleDoc,
                        _ => DocType::Rustdoc,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![DocType::Rustdoc]);

        // Get workspace ID
        let ws_id = workspace_id
            .map(|id| WorkspaceId::from(id.to_string()))
            .ok_or_else(|| anyhow::anyhow!("workspace_id is required for documenter"))?;

        // Create documenter agent with Cortex
        let mut agent = DocumenterAgent::new("mcp-documenter".to_string(), cortex);
        agent.set_workspace(ws_id.clone());

        // Generate documentation
        let result = agent.generate_documentation(file_path, &ws_id, doc_types).await?;

        // Return result as JSON
        Ok(serde_json::json!({
            "status": "completed",
            "documentation": {
                "file_path": result.file_path,
                "documents": result.documentation.iter().map(|doc| {
                    serde_json::json!({
                        "doc_type": format!("{:?}", doc.doc_type),
                        "output_path": doc.output_path,
                        "content_length": doc.content.len(),
                        "content_preview": if doc.content.len() > 200 {
                            format!("{}...", &doc.content[..200])
                        } else {
                            doc.content.clone()
                        },
                    })
                }).collect::<Vec<_>>(),
                "metadata": {
                    "generated_at": result.metadata.generated_at.to_rfc3339(),
                    "agent_id": result.metadata.agent_id,
                    "doc_types": result.metadata.doc_types.iter()
                        .map(|t| format!("{:?}", t))
                        .collect::<Vec<_>>(),
                },
            }
        }))
    }
}

#[async_trait]
impl Tool for AgentLaunchTool {
    fn name(&self) -> &str {
        "axon.agent.launch"
    }

    fn description(&self) -> Option<&str> {
        Some("Launch a specialized agent for a specific task")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(AgentLaunchInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentLaunchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.launch(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: Some(false),
        })
    }
}
