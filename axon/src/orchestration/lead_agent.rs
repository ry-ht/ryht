//! Lead Agent - Orchestrator-Worker Pattern Implementation
//!
//! This module implements the Lead Agent (Orchestrator) as described in Anthropic's
//! multi-agent research system architecture. The Lead Agent is responsible for:
//!
//! - Analyzing query complexity
//! - Developing execution strategies
//! - Spawning and delegating to worker agents
//! - Synthesizing results from multiple workers
//! - Resource allocation based on complexity
//!
//! # Architecture
//!
//! The Lead Agent follows the pattern:
//! 1. Analyze query complexity (Simple/Medium/Complex)
//! 2. Select strategy from library
//! 3. Create execution plan with resource allocation
//! 4. Spawn workers in parallel with explicit delegation
//! 5. Monitor progress and adapt
//! 6. Synthesize final result
//!
//! # Performance Goals
//!
//! - Simple queries: 1 worker, 3-10 tool calls
//! - Medium queries: 2-4 workers, 10-15 calls each
//! - Complex queries: 10+ workers with divided responsibilities
//! - Target: 90% time reduction through parallelization

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::agents::{AgentId, AgentMetrics};
use crate::cortex_bridge::{CortexBridge, SessionId, WorkspaceId, Episode, EpisodeType, EpisodeOutcome};
use crate::coordination::{UnifiedMessageBus, MessageCoordinator};

use super::{
    strategy_library::{StrategyLibrary, ExecutionStrategy},
    worker_registry::{WorkerRegistry, WorkerHandle},
    task_delegation::{TaskDelegation, TaskBoundaries},
    result_synthesizer::{ResultSynthesizer, SynthesizedResult},
    execution_plan::{ExecutionPlan, ResourceAllocation},
    OrchestrationError, Result,
};

// ============================================================================
// Query Complexity Analysis
// ============================================================================

/// Query complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryComplexity {
    /// Simple fact-finding: 1 agent, 3-10 tool calls
    Simple,
    /// Direct comparisons: 2-4 subagents, 10-15 calls each
    Medium,
    /// Complex research: 10+ subagents with divided responsibilities
    Complex,
}

impl QueryComplexity {
    /// Get recommended resource allocation for this complexity
    pub fn recommended_allocation(&self) -> ResourceAllocation {
        match self {
            QueryComplexity::Simple => ResourceAllocation {
                num_workers: 1,
                max_tool_calls_per_worker: 10,
                max_parallel_workers: 1,
                timeout: Duration::from_secs(30),
                max_tokens_budget: 10_000,
                max_cost_cents: 10,
            },
            QueryComplexity::Medium => ResourceAllocation {
                num_workers: 4,
                max_tool_calls_per_worker: 15,
                max_parallel_workers: 4,
                timeout: Duration::from_secs(120),
                max_tokens_budget: 50_000,
                max_cost_cents: 50,
            },
            QueryComplexity::Complex => ResourceAllocation {
                num_workers: 10,
                max_tool_calls_per_worker: 20,
                max_parallel_workers: 10,
                timeout: Duration::from_secs(300),
                max_tokens_budget: 150_000,
                max_cost_cents: 200,
            },
        }
    }
}

/// Query analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysis {
    /// Original query
    pub query: String,
    /// Detected complexity
    pub complexity: QueryComplexity,
    /// Key aspects to address
    pub key_aspects: Vec<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Estimated subtasks
    pub estimated_subtasks: usize,
    /// Parallelization opportunities
    pub parallelization_score: f32,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

// ============================================================================
// Lead Agent Implementation
// ============================================================================

/// Lead Agent for orchestrating multi-agent workflows
///
/// Implements the Orchestrator-Worker pattern from Anthropic's research.
pub struct LeadAgent {
    /// Unique agent ID
    id: AgentId,

    /// Agent name
    name: String,

    /// Cortex bridge for memory operations
    cortex: Arc<CortexBridge>,

    /// Strategy library for execution patterns
    strategy_library: Arc<StrategyLibrary>,

    /// Worker registry for agent pool
    worker_registry: Arc<RwLock<WorkerRegistry>>,

    /// Result synthesizer
    result_synthesizer: Arc<ResultSynthesizer>,

    /// Message bus for communication
    message_bus: Arc<UnifiedMessageBus>,

    /// Message coordinator
    coordinator: Arc<MessageCoordinator>,

    /// Active executions tracking
    active_executions: Arc<RwLock<HashMap<String, ExecutionState>>>,

    /// Agent metrics
    metrics: AgentMetrics,

    /// Configuration
    config: LeadAgentConfig,
}

/// Lead agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadAgentConfig {
    /// Enable adaptive resource allocation
    pub adaptive_allocation: bool,

    /// Enable early termination optimization
    pub early_termination: bool,

    /// Enable dynamic worker spawning
    pub dynamic_spawning: bool,

    /// Maximum concurrent executions
    pub max_concurrent_executions: usize,

    /// Default timeout for executions
    pub default_timeout: Duration,

    /// Enable progress tracking
    pub enable_progress_tracking: bool,
}

impl Default for LeadAgentConfig {
    fn default() -> Self {
        Self {
            adaptive_allocation: true,
            early_termination: true,
            dynamic_spawning: true,
            max_concurrent_executions: 5,
            default_timeout: Duration::from_secs(300),
            enable_progress_tracking: true,
        }
    }
}

/// Execution state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// Execution ID
    pub execution_id: String,

    /// Query being processed
    pub query: String,

    /// Current phase
    pub phase: ExecutionPhase,

    /// Start time
    pub started_at: DateTime<Utc>,

    /// Worker handles
    pub workers: Vec<String>,

    /// Progress (0.0 - 1.0)
    pub progress: f32,

    /// Intermediate results
    pub intermediate_results: Vec<WorkerResult>,

    /// Errors encountered
    pub errors: Vec<String>,
}

/// Execution phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionPhase {
    /// Analyzing query
    Analyzing,
    /// Planning execution
    Planning,
    /// Spawning workers
    SpawningWorkers,
    /// Executing tasks
    Executing,
    /// Synthesizing results
    Synthesizing,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// Worker execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// Worker ID
    pub worker_id: AgentId,

    /// Task delegation
    pub task: TaskDelegation,

    /// Result data
    pub result: serde_json::Value,

    /// Success flag
    pub success: bool,

    /// Execution duration
    pub duration: Duration,

    /// Tokens used
    pub tokens_used: u64,

    /// Cost in cents
    pub cost_cents: u64,

    /// Completed timestamp
    pub completed_at: DateTime<Utc>,
}

impl LeadAgent {
    /// Create a new Lead Agent
    pub fn new(
        name: String,
        cortex: Arc<CortexBridge>,
        strategy_library: Arc<StrategyLibrary>,
        worker_registry: Arc<RwLock<WorkerRegistry>>,
        result_synthesizer: Arc<ResultSynthesizer>,
        message_bus: Arc<UnifiedMessageBus>,
        coordinator: Arc<MessageCoordinator>,
        config: LeadAgentConfig,
    ) -> Self {
        let id = AgentId::new();

        info!("Initializing Lead Agent '{}' with ID: {}", name, id);

        Self {
            id,
            name,
            cortex,
            strategy_library,
            worker_registry,
            result_synthesizer,
            message_bus,
            coordinator,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            metrics: AgentMetrics::new(),
            config,
        }
    }

    // ========================================================================
    // Main Orchestration Flow
    // ========================================================================

    /// Handle a complex query with orchestration
    ///
    /// This is the main entry point implementing the Orchestrator-Worker pattern.
    pub async fn handle_query(
        &self,
        query: &str,
        workspace_id: WorkspaceId,
        session_id: SessionId,
    ) -> Result<SynthesizedResult> {
        let start_time = Instant::now();
        let execution_id = uuid::Uuid::new_v4().to_string();

        info!(
            "Lead Agent {} handling query: {} [execution: {}]",
            self.id, query, execution_id
        );

        // Track execution
        let mut state = ExecutionState {
            execution_id: execution_id.clone(),
            query: query.to_string(),
            phase: ExecutionPhase::Analyzing,
            started_at: Utc::now(),
            workers: Vec::new(),
            progress: 0.0,
            intermediate_results: Vec::new(),
            errors: Vec::new(),
        };

        self.active_executions.write().await.insert(execution_id.clone(), state.clone());

        // Step 1: Analyze query and determine complexity
        debug!("Step 1: Analyzing query complexity");
        state.phase = ExecutionPhase::Analyzing;
        self.update_execution_state(&execution_id, state.clone()).await;

        let analysis = self.analyze_query(query, &workspace_id).await?;
        info!("Query complexity: {:?}, estimated subtasks: {}",
              analysis.complexity, analysis.estimated_subtasks);

        state.progress = 0.2;
        self.update_execution_state(&execution_id, state.clone()).await;

        // Step 2: Select strategy from library
        debug!("Step 2: Selecting execution strategy");
        state.phase = ExecutionPhase::Planning;
        self.update_execution_state(&execution_id, state.clone()).await;

        let strategy = self.strategy_library
            .find_best_strategy(&analysis)
            .await?;

        debug!("Selected strategy: {}", strategy.name);

        // Step 3: Create execution plan with resource allocation
        debug!("Step 3: Creating execution plan");
        let plan = self.create_execution_plan(&analysis, &strategy, &workspace_id).await?;

        info!("Execution plan: {} workers, {} max parallel, timeout: {:?}",
              plan.resource_allocation.num_workers,
              plan.resource_allocation.max_parallel_workers,
              plan.resource_allocation.timeout);

        state.progress = 0.4;
        self.update_execution_state(&execution_id, state.clone()).await;

        // Step 4: Spawn workers with explicit delegation
        debug!("Step 4: Spawning workers");
        state.phase = ExecutionPhase::SpawningWorkers;
        self.update_execution_state(&execution_id, state.clone()).await;

        let worker_handles = self.spawn_workers(&plan, &session_id, &workspace_id).await?;

        state.workers = worker_handles.iter()
            .map(|h| h.worker_id.to_string())
            .collect();

        info!("Spawned {} workers", worker_handles.len());

        state.progress = 0.5;
        state.phase = ExecutionPhase::Executing;
        self.update_execution_state(&execution_id, state.clone()).await;

        // Step 5: Execute workers in parallel and collect results
        debug!("Step 5: Executing workers in parallel");
        let worker_results = self.execute_workers_parallel(
            worker_handles,
            &plan,
            &execution_id,
        ).await?;

        state.intermediate_results = worker_results.clone();
        state.progress = 0.8;
        self.update_execution_state(&execution_id, state.clone()).await;

        // Step 6: Synthesize results
        debug!("Step 6: Synthesizing results");
        state.phase = ExecutionPhase::Synthesizing;
        self.update_execution_state(&execution_id, state.clone()).await;

        let synthesized = self.result_synthesizer
            .synthesize(query, &analysis, &strategy, worker_results)
            .await?;

        let duration = start_time.elapsed();

        info!(
            "Query completed successfully in {:.2}s with {} workers",
            duration.as_secs_f64(),
            synthesized.worker_count
        );

        // Step 7: Store episode for learning
        self.store_orchestration_episode(
            query,
            &workspace_id,
            &session_id,
            &analysis,
            &plan,
            &synthesized,
            duration,
        ).await?;

        // Update metrics
        self.metrics.record_success(
            duration.as_millis() as u64,
            synthesized.total_tokens_used,
            synthesized.total_cost_cents,
        );

        state.phase = ExecutionPhase::Completed;
        state.progress = 1.0;
        self.update_execution_state(&execution_id, state).await;

        Ok(synthesized)
    }

    // ========================================================================
    // Query Analysis
    // ========================================================================

    /// Analyze query to determine complexity and requirements
    async fn analyze_query(
        &self,
        query: &str,
        _workspace_id: &WorkspaceId,
    ) -> Result<QueryAnalysis> {
        debug!("Analyzing query: {}", query);

        // Use heuristics and learned patterns to determine complexity
        let word_count = query.split_whitespace().count();
        let has_multiple_questions = query.matches('?').count() > 1;
        let has_comparison = query.to_lowercase().contains("compare")
            || query.to_lowercase().contains("versus")
            || query.to_lowercase().contains("vs");
        let _has_research_keywords = query.to_lowercase().contains("research")
            || query.to_lowercase().contains("investigate")
            || query.to_lowercase().contains("analyze")
            || query.to_lowercase().contains("comprehensive");

        // Load similar past queries from episodic memory
        let _similar_episodes = self.cortex
            .search_episodes(query, 5)
            .await
            .unwrap_or_default();

        // Determine complexity based on heuristics
        let complexity = if word_count < 20 && !has_multiple_questions && !has_comparison {
            QueryComplexity::Simple
        } else if has_multiple_questions || has_comparison || word_count < 50 {
            QueryComplexity::Medium
        } else {
            QueryComplexity::Complex
        };

        // Extract key aspects from query
        let key_aspects = self.extract_key_aspects(query);

        // Determine required capabilities
        let required_capabilities = self.determine_required_capabilities(query);

        // Estimate subtasks
        let estimated_subtasks = match complexity {
            QueryComplexity::Simple => 1,
            QueryComplexity::Medium => key_aspects.len().max(2),
            QueryComplexity::Complex => key_aspects.len().max(3),
        };

        // Calculate parallelization score (0.0 - 1.0)
        let parallelization_score = if key_aspects.len() > 1 {
            (key_aspects.len() as f32 / 10.0).min(1.0)
        } else {
            0.2
        };

        Ok(QueryAnalysis {
            query: query.to_string(),
            complexity,
            key_aspects,
            required_capabilities,
            estimated_subtasks,
            parallelization_score,
            analyzed_at: Utc::now(),
        })
    }

    /// Extract key aspects from query
    fn extract_key_aspects(&self, query: &str) -> Vec<String> {
        let mut aspects = Vec::new();

        // Simple heuristic: split on conjunctions and extract distinct topics
        let parts: Vec<&str> = query.split(&[',', ';', '&'][..]).collect();

        for part in parts {
            let trimmed = part.trim();
            if !trimmed.is_empty() && trimmed.len() > 5 {
                aspects.push(trimmed.to_string());
            }
        }

        if aspects.is_empty() {
            aspects.push(query.to_string());
        }

        aspects
    }

    /// Determine required capabilities from query
    fn determine_required_capabilities(&self, query: &str) -> Vec<String> {
        let mut capabilities = Vec::new();
        let lower = query.to_lowercase();

        // Code-related capabilities
        if lower.contains("code") || lower.contains("implement") || lower.contains("write") {
            capabilities.push("CodeGeneration".to_string());
        }

        if lower.contains("review") || lower.contains("check") {
            capabilities.push("CodeReview".to_string());
        }

        if lower.contains("test") {
            capabilities.push("Testing".to_string());
        }

        if lower.contains("refactor") || lower.contains("improve") {
            capabilities.push("CodeRefactoring".to_string());
        }

        // Research capabilities
        if lower.contains("research") || lower.contains("find") || lower.contains("search") {
            capabilities.push("InformationRetrieval".to_string());
        }

        // Analysis capabilities
        if lower.contains("analyze") || lower.contains("analysis") {
            capabilities.push("CodeAnalysis".to_string());
        }

        if capabilities.is_empty() {
            capabilities.push("General".to_string());
        }

        capabilities
    }

    // ========================================================================
    // Execution Planning
    // ========================================================================

    /// Create execution plan based on analysis and strategy
    async fn create_execution_plan(
        &self,
        analysis: &QueryAnalysis,
        strategy: &ExecutionStrategy,
        workspace_id: &WorkspaceId,
    ) -> Result<ExecutionPlan> {
        debug!("Creating execution plan for strategy: {}", strategy.name);

        // Get recommended allocation for complexity
        let mut allocation = analysis.complexity.recommended_allocation();

        // Apply adaptive allocation if enabled
        if self.config.adaptive_allocation {
            allocation = self.adapt_resource_allocation(allocation, analysis, workspace_id).await?;
        }

        // Create task delegations
        let delegations = self.create_task_delegations(analysis, strategy, &allocation).await?;

        let estimated_duration = allocation.timeout;

        Ok(ExecutionPlan {
            plan_id: uuid::Uuid::new_v4().to_string(),
            strategy: strategy.clone(),
            resource_allocation: allocation,
            task_delegations: delegations,
            parallelizable: analysis.parallelization_score > 0.5,
            estimated_duration,
            created_at: Utc::now(),
        })
    }

    /// Adapt resource allocation based on available resources and history
    async fn adapt_resource_allocation(
        &self,
        mut allocation: ResourceAllocation,
        analysis: &QueryAnalysis,
        _workspace_id: &WorkspaceId,
    ) -> Result<ResourceAllocation> {
        debug!("Adapting resource allocation");

        // Check worker availability
        let available_workers = self.worker_registry.read().await.available_worker_count();

        if available_workers < allocation.num_workers {
            warn!(
                "Only {} workers available, requested {}. Adjusting allocation.",
                available_workers, allocation.num_workers
            );
            allocation.num_workers = available_workers.max(1);
            allocation.max_parallel_workers = allocation.max_parallel_workers.min(available_workers);
        }

        // Adjust based on parallelization score
        if analysis.parallelization_score < 0.3 {
            debug!("Low parallelization score, reducing workers");
            allocation.num_workers = (allocation.num_workers / 2).max(1);
            allocation.max_parallel_workers = (allocation.max_parallel_workers / 2).max(1);
        }

        Ok(allocation)
    }

    /// Create task delegations for workers
    async fn create_task_delegations(
        &self,
        analysis: &QueryAnalysis,
        strategy: &ExecutionStrategy,
        allocation: &ResourceAllocation,
    ) -> Result<Vec<TaskDelegation>> {
        debug!("Creating {} task delegations", allocation.num_workers);

        let mut delegations = Vec::new();

        // Divide work based on key aspects
        for (idx, aspect) in analysis.key_aspects.iter().enumerate() {
            if idx >= allocation.num_workers {
                break;
            }

            let delegation = TaskDelegation {
                task_id: uuid::Uuid::new_v4().to_string(),
                objective: format!("Address aspect: {}", aspect),
                output_format: strategy.output_format.clone(),
                allowed_tools: strategy.allowed_tools.clone(),
                boundaries: TaskBoundaries {
                    scope: vec![aspect.clone()],
                    constraints: vec![
                        format!("Focus only on: {}", aspect),
                        "Provide concise findings".to_string(),
                        "Include citations if applicable".to_string(),
                    ],
                    max_tool_calls: allocation.max_tool_calls_per_worker,
                    timeout: allocation.timeout,
                },
                priority: if idx == 0 { 9 } else { 5 },
                required_capabilities: analysis.required_capabilities.clone(),
                context: serde_json::json!({
                    "query": analysis.query,
                    "aspect": aspect,
                    "complexity": format!("{:?}", analysis.complexity),
                }),
            };

            delegations.push(delegation);
        }

        // If we have more workers than aspects, create supporting tasks
        while delegations.len() < allocation.num_workers {
            let delegation = TaskDelegation {
                task_id: uuid::Uuid::new_v4().to_string(),
                objective: "Provide supporting research and validation".to_string(),
                output_format: strategy.output_format.clone(),
                allowed_tools: strategy.allowed_tools.clone(),
                boundaries: TaskBoundaries {
                    scope: vec!["Supporting research".to_string()],
                    constraints: vec![
                        "Validate findings from other workers".to_string(),
                        "Fill in gaps".to_string(),
                    ],
                    max_tool_calls: allocation.max_tool_calls_per_worker / 2,
                    timeout: allocation.timeout,
                },
                priority: 3,
                required_capabilities: vec!["InformationRetrieval".to_string()],
                context: serde_json::json!({
                    "query": analysis.query,
                    "role": "supporting",
                }),
            };

            delegations.push(delegation);
        }

        Ok(delegations)
    }

    // ========================================================================
    // Worker Management
    // ========================================================================

    /// Spawn workers for execution plan
    async fn spawn_workers(
        &self,
        plan: &ExecutionPlan,
        session_id: &SessionId,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<WorkerHandle>> {
        debug!("Spawning {} workers", plan.task_delegations.len());

        let mut handles = Vec::new();
        let mut registry = self.worker_registry.write().await;

        for delegation in &plan.task_delegations {
            let handle = registry
                .acquire_worker(&delegation.required_capabilities, session_id, workspace_id)
                .await?;

            debug!(
                "Assigned worker {} to task {} ({})",
                handle.worker_id, delegation.task_id, delegation.objective
            );

            handles.push(handle);
        }

        Ok(handles)
    }

    /// Execute workers in parallel
    async fn execute_workers_parallel(
        &self,
        worker_handles: Vec<WorkerHandle>,
        plan: &ExecutionPlan,
        _execution_id: &str,
    ) -> Result<Vec<WorkerResult>> {
        info!(
            "Executing {} workers in parallel (max: {} concurrent)",
            worker_handles.len(),
            plan.resource_allocation.max_parallel_workers
        );

        let mut results = Vec::new();
        let handles_with_tasks: Vec<_> = worker_handles
            .into_iter()
            .zip(plan.task_delegations.iter())
            .collect();

        // Execute in batches to respect max_parallel_workers
        let batch_size = plan.resource_allocation.max_parallel_workers;

        for batch in handles_with_tasks.chunks(batch_size) {
            let batch_futures: Vec<_> = batch
                .iter()
                .map(|(handle, delegation)| {
                    self.execute_worker_task(handle.clone(), (*delegation).clone())
                })
                .collect();

            let batch_results = futures::future::join_all(batch_futures).await;

            for result in batch_results {
                match result {
                    Ok(worker_result) => {
                        debug!("Worker {} completed successfully", worker_result.worker_id);
                        results.push(worker_result);
                    }
                    Err(e) => {
                        error!("Worker execution failed: {}", e);
                        // Continue with other workers
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute a single worker task
    async fn execute_worker_task(
        &self,
        handle: WorkerHandle,
        delegation: TaskDelegation,
    ) -> Result<WorkerResult> {
        let start_time = Instant::now();

        debug!(
            "Worker {} executing task: {}",
            handle.worker_id, delegation.objective
        );

        // Delegate task to worker through message bus
        let result = self.send_task_to_worker(&handle, &delegation).await?;

        let duration = start_time.elapsed();

        Ok(WorkerResult {
            worker_id: handle.worker_id.clone(),
            task: delegation.clone(),
            result,
            success: true,
            duration,
            tokens_used: 1000, // Would come from actual execution
            cost_cents: 5,     // Would come from actual execution
            completed_at: Utc::now(),
        })
    }

    /// Send task to worker via message bus
    async fn send_task_to_worker(
        &self,
        handle: &WorkerHandle,
        delegation: &TaskDelegation,
    ) -> Result<serde_json::Value> {
        use crate::coordination::{MessageEnvelope, Message};

        debug!("Sending task {} to worker {} via message bus", delegation.task_id, handle.worker_id);

        // Create task assignment message
        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: Some(delegation.task_id.clone()),
            causation_id: None,
            from: self.id.clone(),
            to: Some(handle.worker_id.clone()),
            topic: None,
            session_id: handle.session_id.clone(),
            workspace_id: handle.workspace_id.clone(),
            payload: Message::TaskAssignment {
                task_id: delegation.task_id.clone(),
                task_description: delegation.objective.clone(),
                context: delegation.context.clone(),
            },
            timestamp: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::from_std(delegation.boundaries.timeout).unwrap()),
            priority: delegation.priority,
            attempt_count: 0,
            max_attempts: 3,
            metadata: std::collections::HashMap::new(),
        };

        // Send message
        self.message_bus.send(envelope).await
            .map_err(|e| OrchestrationError::ExecutionFailed {
                reason: format!("Failed to send task to worker: {}", e)
            })?;

        // Create message envelope for the request
        let envelope = MessageEnvelope {
            message_id: uuid::Uuid::new_v4().to_string(),
            correlation_id: Some(delegation.task_id.clone()),
            causation_id: None,
            from: self.id.clone(),
            to: Some(handle.worker_id.clone()),
            topic: None,
            session_id: handle.session_id.clone(),
            workspace_id: handle.workspace_id.clone(),
            payload: Message::Query {
                query_id: delegation.task_id.clone(),
                query_text: format!("get_task_result:{}", delegation.task_id),
                filters: serde_json::json!({
                    "action": "get_task_result",
                    "task_id": delegation.task_id,
                }),
            },
            timestamp: chrono::Utc::now(),
            expires_at: None,
            priority: 5,
        };

        // Wait for result via coordinator
        let response = self.coordinator
            .request_response(
                envelope,
                delegation.boundaries.timeout,
            )
            .await
            .map_err(|e| OrchestrationError::ExecutionFailed {
                reason: format!("Failed to receive task result: {}", e)
            })?;

        Ok(response)
    }

    // ========================================================================
    // Episode Storage
    // ========================================================================

    /// Store orchestration episode for learning
    async fn store_orchestration_episode(
        &self,
        query: &str,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
        analysis: &QueryAnalysis,
        _plan: &ExecutionPlan,
        result: &SynthesizedResult,
        duration: Duration,
    ) -> Result<()> {
        debug!("Storing orchestration episode");

        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Task,
            task_description: query.to_string(),
            agent_id: self.id.to_string(),
            session_id: Some(session_id.to_string()),
            workspace_id: workspace_id.to_string(),
            entities_created: Vec::new(),
            entities_modified: Vec::new(),
            entities_deleted: Vec::new(),
            files_touched: Vec::new(),
            queries_made: vec![query.to_string()],
            tools_used: Vec::new(),
            solution_summary: result.summary.clone(),
            outcome: if result.success {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Partial
            },
            success_metrics: serde_json::json!({
                "complexity": format!("{:?}", analysis.complexity),
                "worker_count": result.worker_count,
                "parallel_efficiency": result.parallel_efficiency,
                "time_reduction": result.time_reduction_percent,
            }),
            errors_encountered: Vec::new(),
            lessons_learned: vec![
                format!("Query complexity: {:?}", analysis.complexity),
                format!("Workers used: {}", result.worker_count),
                format!("Parallel efficiency: {:.2}%", result.parallel_efficiency * 100.0),
            ],
            duration_seconds: duration.as_secs() as i32,
            tokens_used: crate::cortex_bridge::models::TokenUsage {
                input: result.total_tokens_used / 2,
                output: result.total_tokens_used / 2,
                total: result.total_tokens_used,
            },
            embedding: Vec::new(),
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };

        self.cortex.store_episode(episode).await
            .map_err(|e| OrchestrationError::Other(e.into()))?;

        Ok(())
    }

    // ========================================================================
    // State Management
    // ========================================================================

    /// Update execution state
    async fn update_execution_state(&self, execution_id: &str, state: ExecutionState) {
        self.active_executions
            .write()
            .await
            .insert(execution_id.to_string(), state);
    }

    /// Get execution state
    pub async fn get_execution_state(&self, execution_id: &str) -> Option<ExecutionState> {
        self.active_executions.read().await.get(execution_id).cloned()
    }

    /// Get all active executions
    pub async fn get_active_executions(&self) -> Vec<ExecutionState> {
        self.active_executions
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get agent ID
    pub fn id(&self) -> &AgentId {
        &self.id
    }

    /// Get agent name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get metrics
    pub fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }

    /// Get configuration
    pub fn config(&self) -> &LeadAgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_complexity_allocation() {
        let simple = QueryComplexity::Simple.recommended_allocation();
        assert_eq!(simple.num_workers, 1);
        assert!(simple.max_tool_calls_per_worker <= 10);

        let medium = QueryComplexity::Medium.recommended_allocation();
        assert_eq!(medium.num_workers, 4);
        assert!(medium.max_tool_calls_per_worker <= 15);

        let complex = QueryComplexity::Complex.recommended_allocation();
        assert_eq!(complex.num_workers, 10);
        assert!(complex.max_tool_calls_per_worker <= 20);
    }
}
