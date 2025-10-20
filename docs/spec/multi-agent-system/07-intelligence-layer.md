# Axon: Intelligence Layer

## Overview

Intelligence Layer in Axon is a layer for optimizing task execution through integration with Cortex. Axon does NOT implement learning, knowledge graphs or memory — it coordinates agents and uses Cortex cognitive capabilities via REST API.

## Responsibility Separation

```
┌─────────────────────────────────────────────┐
│                    Axon                      │
│    (Orchestration & Execution Optimization)  │
│                                              │
│  ✓ Model Router (выбор LLM provider)        │
│  ✓ Context Optimizer (token optimization)   │
│  ✓ Task Scheduling                           │
│  ✓ Agent Coordination                        │
│  ✓ Performance Monitoring                    │
│                                              │
│  ✗ Learning                                  │
│  ✗ Knowledge Graph                           │
│  ✗ Memory Storage                            │
│  ✗ Pattern Extraction                        │
└──────────────────┬──────────────────────────┘
                   │ REST API
                   ▼
┌─────────────────────────────────────────────┐
│                   Cortex                     │
│        (Intelligence & Memory)               │
│                                              │
│  ✓ Knowledge Graph                           │
│  ✓ Learning System                           │
│  ✓ Pattern Recognition                       │
│  ✓ Context Optimization (Agentwise 3.0)     │
│  ✓ Semantic Search                           │
│  ✓ Episodic Memory                           │
└─────────────────────────────────────────────┘
```

## Model Router

Model Router выбирает оптимального LLM provider для задачи на основе данных из Cortex.

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Model Router selects optimal provider for task
pub struct ModelRouter {
    /// Available providers
    providers: Vec<Box<dyn ModelProvider>>,

    /// Client for Cortex requests
    cortex_bridge: Arc<CortexBridge>,

    /// Local cache for routing decisions
    routing_cache: Arc<RwLock<RoutingCache>>,

    /// Routing rules
    routing_rules: RoutingRules,

    /// Cost optimizer
    cost_optimizer: CostOptimizer,

    /// Metrics
    metrics: RouterMetrics,
}

/// Base trait for model providers
pub trait ModelProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn supported_models(&self) -> Vec<ModelInfo>;
    async fn execute(&self, request: ModelRequest) -> Result<ModelResponse>;
    fn cost_per_token(&self, model: &str) -> f64;
    fn max_tokens(&self, model: &str) -> usize;
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_window: usize,
    pub supports_streaming: bool,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone)]
pub enum Capability {
    CodeGeneration,
    Reasoning,
    FunctionCalling,
    Vision,
    LongContext,
}

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub task_type: TaskType,
    pub context: String,
    pub requirements: Requirements,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    CodeGeneration,
    CodeReview,
    Testing,
    Documentation,
    Analysis,
    Planning,
}

#[derive(Debug, Clone)]
pub enum Requirements {
    LowestCost,
    FastestResponse,
    HighestQuality,
    Balanced,
    Custom {
        cost_weight: f32,
        speed_weight: f32,
        quality_weight: f32,
    },
}

#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub content: String,
    pub model_used: String,
    pub provider: String,
    pub tokens_used: usize,
    pub cost: f64,
    pub duration: std::time::Duration,
}

impl ModelRouter {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        Self {
            providers: vec![
                Box::new(OpenAIProvider::new()),
                Box::new(AnthropicProvider::new()),
                Box::new(GoogleProvider::new()),
                Box::new(LocalProvider::new()),
            ],
            cortex_bridge,
            routing_cache: Arc::new(RwLock::new(RoutingCache::new())),
            routing_rules: RoutingRules::default(),
            cost_optimizer: CostOptimizer::new(),
            metrics: RouterMetrics::new(),
        }
    }

    /// Select optimal model for task
    pub async fn select_model(&self, task: &Task) -> Result<ModelSelection> {
        // Check cache
        if let Some(cached) = self.routing_cache.read().await.get(&task.id) {
            if !cached.is_expired() {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.selection.clone());
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);

        // Query Cortex for historical performance data
        let history = self.query_cortex_performance(task).await?;

        // Query cost optimization patterns from Cortex
        let cost_patterns = self.query_cortex_cost_patterns(task).await?;

        // Make decision based on Cortex intelligence
        let selection = self.route_based_on_intelligence(
            task,
            &history,
            &cost_patterns,
        ).await?;

        // Cache result
        self.routing_cache.write().await.insert(
            task.id.clone(),
            CachedSelection {
                selection: selection.clone(),
                cached_at: std::time::Instant::now(),
                ttl: std::time::Duration::from_secs(300),
            },
        );

        self.metrics.selections_made.fetch_add(1, Ordering::Relaxed);

        Ok(selection)
    }

    /// Query performance data from Cortex
    async fn query_cortex_performance(&self, task: &Task) -> Result<PerformanceHistory> {
        // GET /intelligence/model-performance?task_type=...
        let query = PerformanceQuery {
            task_type: task.task_type.clone(),
            time_window: chrono::Duration::days(30),
            limit: 100,
        };

        let history = self.cortex_bridge
            .query_model_performance(query)
            .await?;

        Ok(history)
    }

    /// Query cost optimization patterns from Cortex
    async fn query_cortex_cost_patterns(&self, task: &Task) -> Result<Vec<CostPattern>> {
        // POST /intelligence/patterns/cost-optimization
        let patterns = self.cortex_bridge
            .find_patterns(PatternQuery::CostOptimization {
                task_type: task.task_type.clone(),
                context_size: task.estimated_context_size(),
            })
            .await?;

        Ok(patterns)
    }

    /// Make decision based on Cortex intelligence
    async fn route_based_on_intelligence(
        &self,
        task: &Task,
        history: &PerformanceHistory,
        patterns: &[CostPattern],
    ) -> Result<ModelSelection> {
        let mut scores: Vec<(String, String, f32)> = Vec::new();

        for provider in &self.providers {
            for model in provider.supported_models() {
                // Check capabilities
                if !self.matches_requirements(task, &model) {
                    continue;
                }

                // Calculate score based on historical data
                let historical_score = history.get_score(provider.id(), &model.id);

                // Calculate score based on patterns
                let pattern_score = patterns.iter()
                    .filter_map(|p| p.get_score(provider.id(), &model.id))
                    .max_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.5);

                // Consider cost
                let cost_score = self.calculate_cost_score(
                    provider.as_ref(),
                    &model.id,
                    task.estimated_tokens(),
                );

                // Combine scores
                let final_score = match &task.requirements {
                    Requirements::LowestCost => cost_score * 0.8 + historical_score * 0.2,
                    Requirements::FastestResponse => historical_score * 0.7 + pattern_score * 0.3,
                    Requirements::HighestQuality => pattern_score * 0.6 + historical_score * 0.4,
                    Requirements::Balanced => {
                        historical_score * 0.4 + pattern_score * 0.3 + cost_score * 0.3
                    }
                    Requirements::Custom { cost_weight, speed_weight, quality_weight } => {
                        cost_score * cost_weight +
                        historical_score * speed_weight +
                        pattern_score * quality_weight
                    }
                };

                scores.push((provider.id().to_string(), model.id.clone(), final_score));
            }
        }

        // Select best option
        let (provider_id, model_id, confidence) = scores
            .into_iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .ok_or(Error::NoSuitableModel)?;

        Ok(ModelSelection {
            provider_id,
            model_id,
            confidence,
            rationale: format!(
                "Selected based on Cortex intelligence: {} historical datapoints, {} cost patterns",
                history.datapoints_count(),
                patterns.len()
            ),
        })
    }

    fn matches_requirements(&self, task: &Task, model: &ModelInfo) -> bool {
        // Check context window
        if task.estimated_context_size() > model.context_window {
            return false;
        }

        // Check capabilities
        task.required_capabilities()
            .iter()
            .all(|req| model.capabilities.contains(req))
    }

    fn calculate_cost_score(&self, provider: &dyn ModelProvider, model: &str, tokens: usize) -> f32 {
        let cost = provider.cost_per_token(model) * tokens as f64;
        let max_cost = 1.0; // $1.00 max

        // Normalize: lower cost = higher score
        1.0 - (cost / max_cost).min(1.0) as f32
    }

    /// Execute request through selected provider
    pub async fn execute(
        &self,
        selection: &ModelSelection,
        request: ModelRequest,
    ) -> Result<ModelResponse> {
        let provider = self.providers
            .iter()
            .find(|p| p.id() == selection.provider_id)
            .ok_or(Error::ProviderNotFound)?;

        let start = std::time::Instant::now();

        let response = provider.execute(request).await?;

        let duration = start.elapsed();

        // Record metrics in Cortex for learning
        self.record_execution(selection, &response, duration).await?;

        Ok(response)
    }

    /// Record execution in Cortex for learning
    async fn record_execution(
        &self,
        selection: &ModelSelection,
        response: &ModelResponse,
        duration: std::time::Duration,
    ) -> Result<()> {
        let execution = ModelExecution {
            provider: selection.provider_id.clone(),
            model: selection.model_id.clone(),
            tokens_used: response.tokens_used,
            cost: response.cost,
            duration,
            success: true,
            timestamp: chrono::Utc::now(),
        };

        // POST /intelligence/executions
        self.cortex_bridge
            .record_model_execution(execution)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
    pub confidence: f32,
    pub rationale: String,
}

#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    datapoints: Vec<PerformanceDatapoint>,
}

impl PerformanceHistory {
    fn get_score(&self, provider_id: &str, model_id: &str) -> f32 {
        let relevant: Vec<_> = self.datapoints
            .iter()
            .filter(|d| d.provider == provider_id && d.model == model_id)
            .collect();

        if relevant.is_empty() {
            return 0.5; // Default score
        }

        // Calculate average success rate
        let success_rate = relevant.iter()
            .filter(|d| d.success)
            .count() as f32 / relevant.len() as f32;

        // Consider recency (newer data is more important)
        let recency_weighted = relevant.iter()
            .map(|d| {
                let age_days = (chrono::Utc::now() - d.timestamp).num_days() as f32;
                let recency_factor = (-age_days / 30.0).exp(); // Exponential decay
                if d.success { recency_factor } else { 0.0 }
            })
            .sum::<f32>() / relevant.len() as f32;

        // Combine
        success_rate * 0.6 + recency_weighted * 0.4
    }

    fn datapoints_count(&self) -> usize {
        self.datapoints.len()
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceDatapoint {
    provider: String,
    model: String,
    success: bool,
    duration: std::time::Duration,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CostPattern {
    pub pattern_type: String,
    pub recommendations: HashMap<String, f32>,
}

impl CostPattern {
    fn get_score(&self, provider_id: &str, model_id: &str) -> Option<f32> {
        let key = format!("{}:{}", provider_id, model_id);
        self.recommendations.get(&key).copied()
    }
}
```

## Context Optimizer

Context Optimizer uses Agentwise Context 3.0 from Cortex for token optimization.

```rust
/// Context Optimizer optimizes context before sending to LLM
pub struct ContextOptimizer {
    /// Cortex bridge for accessing Context 3.0
    cortex_bridge: Arc<CortexBridge>,

    /// Local cache for optimized contexts
    cache: Arc<RwLock<LruCache<ContextHash, OptimizedContext>>>,

    /// Compression engine for preprocessing
    compression_engine: CompressionEngine,

    /// Metrics
    metrics: OptimizerMetrics,
}

impl ContextOptimizer {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        Self {
            cortex_bridge,
            cache: Arc::new(RwLock::new(LruCache::new(1000))),
            compression_engine: CompressionEngine::new(),
            metrics: OptimizerMetrics::new(),
        }
    }

    /// Optimize context for task
    pub async fn optimize(&self, context: RawContext, task: &Task) -> Result<OptimizedContext> {
        let start = std::time::Instant::now();

        // Calculate hash for caching
        let hash = self.calculate_hash(&context);

        // Check cache
        if let Some(cached) = self.cache.read().await.get(&hash) {
            if !cached.is_expired() {
                self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.clone());
            }
        }

        self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);

        // Pre-compression locally
        let pre_compressed = self.compression_engine.compress(&context);

        // Send to Cortex for Context 3.0 optimization
        let optimized = self.cortex_optimize(pre_compressed, task).await?;

        let duration = start.elapsed();

        // Calculate token savings
        let original_tokens = self.estimate_tokens(&context.content);
        let optimized_tokens = self.estimate_tokens(&optimized.content);
        let savings = original_tokens.saturating_sub(optimized_tokens);

        info!(
            "Context optimized: {} -> {} tokens (saved {})",
            original_tokens, optimized_tokens, savings
        );

        self.metrics.total_tokens_saved.fetch_add(savings as u64, Ordering::Relaxed);

        // Cache result
        self.cache.write().await.put(hash, optimized.clone());

        Ok(optimized)
    }

    /// Optimization through Cortex Context 3.0
    async fn cortex_optimize(
        &self,
        context: PreCompressedContext,
        task: &Task,
    ) -> Result<OptimizedContext> {
        // POST /context/optimize
        let request = OptimizationRequest {
            content: context.content,
            task_type: task.task_type.clone(),
            target_tokens: task.max_context_tokens(),
            preserve_entities: true,
            extract_key_patterns: true,
        };

        let response = self.cortex_bridge
            .optimize_context(request)
            .await?;

        Ok(OptimizedContext {
            content: response.optimized_content,
            entities_preserved: response.entities,
            key_patterns: response.patterns,
            token_count: response.estimated_tokens,
            optimization_strategy: response.strategy_used,
            created_at: std::time::Instant::now(),
            ttl: std::time::Duration::from_secs(600),
        })
    }

    fn calculate_hash(&self, context: &RawContext) -> ContextHash {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        context.content.hash(&mut hasher);
        ContextHash(hasher.finish())
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        // Simple estimation: ~4 characters per token
        text.len() / 4
    }
}

#[derive(Debug, Clone)]
pub struct RawContext {
    pub content: String,
    pub metadata: ContextMetadata,
}

#[derive(Debug, Clone)]
pub struct ContextMetadata {
    pub source: String,
    pub language: Option<String>,
    pub importance: f32,
}

#[derive(Debug, Clone)]
pub struct PreCompressedContext {
    pub content: String,
    pub compression_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct OptimizedContext {
    pub content: String,
    pub entities_preserved: Vec<String>,
    pub key_patterns: Vec<String>,
    pub token_count: usize,
    pub optimization_strategy: String,
    pub created_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

impl OptimizedContext {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextHash(u64);

pub struct CompressionEngine {
    // Simple compression strategies before sending to Cortex
}

impl CompressionEngine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn compress(&self, context: &RawContext) -> PreCompressedContext {
        let mut content = context.content.clone();

        // Remove redundant whitespace
        content = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // Remove duplicate lines
        content = self.deduplicate_lines(&content);

        let original_len = context.content.len();
        let compressed_len = content.len();
        let ratio = compressed_len as f32 / original_len as f32;

        PreCompressedContext {
            content,
            compression_ratio: ratio,
        }
    }

    fn deduplicate_lines(&self, text: &str) -> String {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        text.lines()
            .filter(|line| seen.insert(line.to_string()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

## Token Optimization

```rust
/// Token Budget Manager manages token allocation
pub struct TokenBudgetManager {
    /// Maximum budget for task
    max_budget: usize,

    /// Priorities for different context types
    priorities: HashMap<ContextType, f32>,

    /// Cortex bridge for optimization
    cortex_bridge: Arc<CortexBridge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextType {
    TaskDescription,
    CodeContext,
    HistoricalData,
    Examples,
    Documentation,
}

impl TokenBudgetManager {
    pub fn new(max_budget: usize, cortex_bridge: Arc<CortexBridge>) -> Self {
        let mut priorities = HashMap::new();
        priorities.insert(ContextType::TaskDescription, 1.0);
        priorities.insert(ContextType::CodeContext, 0.8);
        priorities.insert(ContextType::HistoricalData, 0.6);
        priorities.insert(ContextType::Examples, 0.5);
        priorities.insert(ContextType::Documentation, 0.4);

        Self {
            max_budget,
            priorities,
            cortex_bridge,
        }
    }

    /// Allocate budget between contexts
    pub async fn allocate_budget(
        &self,
        contexts: Vec<(ContextType, RawContext)>,
    ) -> Result<Vec<(ContextType, OptimizedContext)>> {
        // Calculate total priority weight
        let total_weight: f32 = contexts.iter()
            .map(|(ctx_type, _)| self.priorities.get(ctx_type).unwrap_or(&0.5))
            .sum();

        let mut allocated = Vec::new();

        for (ctx_type, raw_context) in contexts {
            let priority = self.priorities.get(&ctx_type).unwrap_or(&0.5);
            let allocated_tokens = ((self.max_budget as f32 * priority) / total_weight) as usize;

            // Optimize through Cortex with target tokens
            let optimized = self.cortex_bridge
                .optimize_context(OptimizationRequest {
                    content: raw_context.content,
                    task_type: TaskType::CodeGeneration, // Would be provided
                    target_tokens: allocated_tokens,
                    preserve_entities: true,
                    extract_key_patterns: true,
                })
                .await?;

            allocated.push((ctx_type, OptimizedContext {
                content: optimized.optimized_content,
                entities_preserved: optimized.entities,
                key_patterns: optimized.patterns,
                token_count: optimized.estimated_tokens,
                optimization_strategy: optimized.strategy_used,
                created_at: std::time::Instant::now(),
                ttl: std::time::Duration::from_secs(600),
            }));
        }

        Ok(allocated)
    }
}
```

## Knowledge Graph Integration

Axon queries data from Cortex Knowledge Graph via API.

```rust
/// Knowledge Graph Client for Cortex queries
pub struct KnowledgeGraphClient {
    cortex_bridge: Arc<CortexBridge>,
    cache: Arc<RwLock<LruCache<QueryHash, KnowledgeResult>>>,
}

impl KnowledgeGraphClient {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        Self {
            cortex_bridge,
            cache: Arc::new(RwLock::new(LruCache::new(500))),
        }
    }

    /// Query related entities for task
    pub async fn query_related_entities(&self, task: &Task) -> Result<Vec<Entity>> {
        // GET /knowledge/entities?related_to=...
        let entities = self.cortex_bridge
            .query_entities(EntityQuery {
                related_to: task.description.clone(),
                entity_types: vec!["code_unit", "pattern", "concept"],
                max_depth: 2,
                limit: 20,
            })
            .await?;

        Ok(entities)
    }

    /// Get dependency graph for code units
    pub async fn get_dependency_graph(&self, unit_ids: Vec<String>) -> Result<DependencyGraph> {
        // GET /knowledge/dependencies
        let graph = self.cortex_bridge
            .query_dependencies(DependencyQuery {
                unit_ids,
                include_transitive: true,
                max_depth: 3,
            })
            .await?;

        Ok(graph)
    }

    /// Find patterns relevant for task
    pub async fn find_relevant_patterns(&self, task: &Task) -> Result<Vec<Pattern>> {
        // POST /knowledge/patterns/search
        let patterns = self.cortex_bridge
            .search_patterns(PatternSearchQuery {
                task_type: task.task_type.clone(),
                context: task.description.clone(),
                min_confidence: 0.7,
                limit: 10,
            })
            .await?;

        Ok(patterns)
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub target_id: String,
    pub relationship_type: String,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: String,
    pub description: String,
    pub confidence: f32,
    pub examples: Vec<String>,
}
```

## Learning через Episodes

Axon records execution episodes in Cortex for learning.

```rust
/// Episode Recorder records execution episodes in Cortex
pub struct EpisodeRecorder {
    cortex_bridge: Arc<CortexBridge>,
    buffer: Arc<Mutex<Vec<ExecutionEvent>>>,
    flush_interval: std::time::Duration,
}

impl EpisodeRecorder {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        let recorder = Self {
            cortex_bridge,
            buffer: Arc::new(Mutex::new(Vec::new())),
            flush_interval: std::time::Duration::from_secs(60),
        };

        // Start background task for flush
        recorder.start_flush_task();

        recorder
    }

    /// Record execution workflow
    pub async fn record_workflow_execution(&self, execution: WorkflowExecution) -> Result<()> {
        let episode = ExecutionEpisode {
            episode_type: EpisodeType::Workflow,
            workflow_id: Some(execution.workflow_id),
            agent_id: None,
            task_id: None,
            actions: execution.actions,
            decisions: execution.decisions,
            metrics: execution.metrics,
            outcome: execution.outcome,
            duration: execution.duration,
            timestamp: chrono::Utc::now(),
        };

        // POST /memory/episodes
        self.cortex_bridge.store_episode(episode).await?;

        info!("Recorded workflow execution in Cortex");

        Ok(())
    }

    /// Record agent performance
    pub async fn record_agent_performance(
        &self,
        agent_id: AgentId,
        performance: AgentPerformance,
    ) -> Result<()> {
        let episode = ExecutionEpisode {
            episode_type: EpisodeType::Agent,
            workflow_id: None,
            agent_id: Some(agent_id),
            task_id: Some(performance.task_id),
            actions: performance.actions_taken,
            decisions: vec![],
            metrics: performance.metrics,
            outcome: performance.outcome,
            duration: performance.duration,
            timestamp: chrono::Utc::now(),
        };

        self.cortex_bridge.store_episode(episode).await?;

        Ok(())
    }

    /// Record model execution
    pub async fn record_model_execution(&self, execution: ModelExecution) -> Result<()> {
        let event = ExecutionEvent::ModelExecution(execution);
        self.buffer.lock().await.push(event);

        Ok(())
    }

    fn start_flush_task(&self) {
        let buffer = self.buffer.clone();
        let cortex_bridge = self.cortex_bridge.clone();
        let interval = self.flush_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let events = {
                    let mut buf = buffer.lock().await;
                    std::mem::take(&mut *buf)
                };

                if !events.is_empty() {
                    info!("Flushing {} execution events to Cortex", events.len());

                    for event in events {
                        if let Err(e) = Self::flush_event(&cortex_bridge, event).await {
                            error!("Failed to flush event: {}", e);
                        }
                    }
                }
            }
        });
    }

    async fn flush_event(cortex_bridge: &CortexBridge, event: ExecutionEvent) -> Result<()> {
        match event {
            ExecutionEvent::ModelExecution(exec) => {
                cortex_bridge.record_model_execution(exec).await?;
            }
            ExecutionEvent::TaskCompletion(task) => {
                cortex_bridge.record_task_completion(task).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum EpisodeType {
    Workflow,
    Agent,
    Consensus,
    Model,
}

#[derive(Debug, Clone)]
pub struct ExecutionEpisode {
    pub episode_type: EpisodeType,
    pub workflow_id: Option<String>,
    pub agent_id: Option<AgentId>,
    pub task_id: Option<String>,
    pub actions: Vec<Action>,
    pub decisions: Vec<Decision>,
    pub metrics: ExecutionMetrics,
    pub outcome: Outcome,
    pub duration: std::time::Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub actions: Vec<Action>,
    pub decisions: Vec<Decision>,
    pub metrics: ExecutionMetrics,
    pub outcome: Outcome,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct AgentPerformance {
    pub task_id: String,
    pub actions_taken: Vec<Action>,
    pub metrics: ExecutionMetrics,
    pub outcome: Outcome,
    pub duration: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub action_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    pub tokens_used: usize,
    pub api_calls: usize,
    pub cost: f64,
    pub success_rate: f32,
    pub custom_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
    pub quality_score: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    ModelExecution(ModelExecution),
    TaskCompletion(TaskCompletion),
}

#[derive(Debug, Clone)]
pub struct ModelExecution {
    pub provider: String,
    pub model: String,
    pub tokens_used: usize,
    pub cost: f64,
    pub duration: std::time::Duration,
    pub success: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct TaskCompletion {
    pub task_id: String,
    pub success: bool,
    pub duration: std::time::Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

## Pattern Extraction из Cortex

```rust
/// Pattern Analyzer gets patterns from Cortex
pub struct PatternAnalyzer {
    cortex_bridge: Arc<CortexBridge>,
}

impl PatternAnalyzer {
    pub fn new(cortex_bridge: Arc<CortexBridge>) -> Self {
        Self { cortex_bridge }
    }

    /// Analyze workflow for optimization
    pub async fn analyze_workflow(&self, workflow: &Workflow) -> Result<WorkflowAnalysis> {
        // POST /intelligence/patterns/analyze-workflow
        let patterns = self.cortex_bridge
            .find_workflow_patterns(WorkflowPatternQuery {
                tasks: workflow.tasks.clone(),
                dependencies: workflow.dependencies.clone(),
                historical_context: true,
            })
            .await?;

        // Convert patterns to optimization opportunities
        let optimizations = patterns.iter()
            .filter_map(|p| self.pattern_to_optimization(p))
            .collect();

        let estimated_improvement = self.calculate_improvement(&patterns);

        Ok(WorkflowAnalysis {
            detected_patterns: patterns,
            optimization_opportunities: optimizations,
            estimated_improvement,
        })
    }

    fn pattern_to_optimization(&self, pattern: &Pattern) -> Option<Optimization> {
        // Convert pattern to specific optimization
        match pattern.pattern_type.as_str() {
            "bottleneck" => Some(Optimization::Parallelize {
                tasks: pattern.affected_tasks(),
            }),
            "redundancy" => Some(Optimization::Deduplicate {
                tasks: pattern.affected_tasks(),
            }),
            "inefficient_sequence" => Some(Optimization::Reorder {
                new_order: pattern.suggested_order(),
            }),
            _ => None,
        }
    }

    fn calculate_improvement(&self, patterns: &[Pattern]) -> f32 {
        patterns.iter()
            .map(|p| p.expected_improvement())
            .sum::<f32>() / patterns.len() as f32
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowAnalysis {
    pub detected_patterns: Vec<Pattern>,
    pub optimization_opportunities: Vec<Optimization>,
    pub estimated_improvement: f32,
}

#[derive(Debug, Clone)]
pub enum Optimization {
    Parallelize { tasks: Vec<String> },
    Deduplicate { tasks: Vec<String> },
    Reorder { new_order: Vec<String> },
    CacheResult { task: String },
}

impl Pattern {
    fn affected_tasks(&self) -> Vec<String> {
        // Extract from pattern data
        vec![]
    }

    fn suggested_order(&self) -> Vec<String> {
        vec![]
    }

    fn expected_improvement(&self) -> f32 {
        self.confidence * 0.5 // Simple heuristic
    }
}
```

## Supporting Types and Utilities

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use lru::LruCache;

// Model Providers

pub struct OpenAIProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

impl ModelProvider for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "gpt-4-turbo".to_string(),
                name: "GPT-4 Turbo".to_string(),
                context_window: 128000,
                supports_streaming: true,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::Reasoning,
                    Capability::FunctionCalling,
                    Capability::LongContext,
                ],
            },
            ModelInfo {
                id: "gpt-3.5-turbo".to_string(),
                name: "GPT-3.5 Turbo".to_string(),
                context_window: 16385,
                supports_streaming: true,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::FunctionCalling,
                ],
            },
        ]
    }

    async fn execute(&self, request: ModelRequest) -> Result<ModelResponse> {
        // Implementation
        todo!()
    }

    fn cost_per_token(&self, model: &str) -> f64 {
        match model {
            "gpt-4-turbo" => 0.00003,
            "gpt-3.5-turbo" => 0.000002,
            _ => 0.00001,
        }
    }

    fn max_tokens(&self, model: &str) -> usize {
        match model {
            "gpt-4-turbo" => 4096,
            "gpt-3.5-turbo" => 4096,
            _ => 2048,
        }
    }
}

pub struct AnthropicProvider {
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            client: reqwest::Client::new(),
        }
    }
}

impl ModelProvider for AnthropicProvider {
    fn id(&self) -> &str {
        "anthropic"
    }

    fn name(&self) -> &str {
        "Anthropic"
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "claude-3-opus".to_string(),
                name: "Claude 3 Opus".to_string(),
                context_window: 200000,
                supports_streaming: true,
                capabilities: vec![
                    Capability::CodeGeneration,
                    Capability::Reasoning,
                    Capability::LongContext,
                    Capability::Vision,
                ],
            },
        ]
    }

    async fn execute(&self, request: ModelRequest) -> Result<ModelResponse> {
        todo!()
    }

    fn cost_per_token(&self, model: &str) -> f64 {
        match model {
            "claude-3-opus" => 0.000015,
            _ => 0.00001,
        }
    }

    fn max_tokens(&self, model: &str) -> usize {
        4096
    }
}

pub struct GoogleProvider;
pub struct LocalProvider;

impl GoogleProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ModelProvider for GoogleProvider {
    fn id(&self) -> &str {
        "google"
    }

    fn name(&self) -> &str {
        "Google"
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![]
    }

    async fn execute(&self, request: ModelRequest) -> Result<ModelResponse> {
        todo!()
    }

    fn cost_per_token(&self, model: &str) -> f64 {
        0.00001
    }

    fn max_tokens(&self, model: &str) -> usize {
        2048
    }
}

impl LocalProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ModelProvider for LocalProvider {
    fn id(&self) -> &str {
        "local"
    }

    fn name(&self) -> &str {
        "Local"
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![]
    }

    async fn execute(&self, request: ModelRequest) -> Result<ModelResponse> {
        todo!()
    }

    fn cost_per_token(&self, model: &str) -> f64 {
        0.0 // Free
    }

    fn max_tokens(&self, model: &str) -> usize {
        4096
    }
}

// Metrics

pub struct RouterMetrics {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub selections_made: AtomicU64,
}

impl RouterMetrics {
    pub fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            selections_made: AtomicU64::new(0),
        }
    }
}

pub struct OptimizerMetrics {
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_tokens_saved: AtomicU64,
}

impl OptimizerMetrics {
    pub fn new() -> Self {
        Self {
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_tokens_saved: AtomicU64::new(0),
        }
    }
}

// Cache types

pub struct RoutingCache {
    cache: HashMap<String, CachedSelection>,
}

impl RoutingCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get(&self, task_id: &str) -> Option<&CachedSelection> {
        self.cache.get(task_id)
    }

    pub fn insert(&mut self, task_id: String, selection: CachedSelection) {
        self.cache.insert(task_id, selection);
    }
}

pub struct CachedSelection {
    pub selection: ModelSelection,
    pub cached_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

impl CachedSelection {
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

// Routing and cost types

pub struct RoutingRules {
    // Custom routing rules
}

impl Default for RoutingRules {
    fn default() -> Self {
        Self {}
    }
}

pub struct CostOptimizer {
    // Cost optimization logic
}

impl CostOptimizer {
    pub fn new() -> Self {
        Self {}
    }
}

// Query types

#[derive(Debug, Clone)]
pub struct PerformanceQuery {
    pub task_type: TaskType,
    pub time_window: chrono::Duration,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub enum PatternQuery {
    CostOptimization {
        task_type: TaskType,
        context_size: usize,
    },
}

#[derive(Debug, Clone)]
pub struct OptimizationRequest {
    pub content: String,
    pub task_type: TaskType,
    pub target_tokens: usize,
    pub preserve_entities: bool,
    pub extract_key_patterns: bool,
}

#[derive(Debug, Clone)]
pub struct EntityQuery {
    pub related_to: String,
    pub entity_types: Vec<&'static str>,
    pub max_depth: usize,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct DependencyQuery {
    pub unit_ids: Vec<String>,
    pub include_transitive: bool,
    pub max_depth: usize,
}

#[derive(Debug, Clone)]
pub struct PatternSearchQuery {
    pub task_type: TaskType,
    pub context: String,
    pub min_confidence: f32,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct WorkflowPatternQuery {
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
    pub historical_context: bool,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub from: String,
    pub to: String,
}

// Task types

#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub description: String,
    pub requirements: Requirements,
}

impl Task {
    fn estimated_context_size(&self) -> usize {
        self.description.len() * 2
    }

    fn max_context_tokens(&self) -> usize {
        8000
    }

    fn estimated_tokens(&self) -> usize {
        self.description.len() / 4
    }

    fn required_capabilities(&self) -> Vec<Capability> {
        match self.task_type {
            TaskType::CodeGeneration => vec![Capability::CodeGeneration],
            TaskType::CodeReview => vec![Capability::Reasoning],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub id: String,
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No suitable model found")]
    NoSuitableModel,

    #[error("Provider not found")]
    ProviderNotFound,

    #[error("Cortex error: {0}")]
    Cortex(String),

    #[error("Other error: {0}")]
    Other(String),
}

type QueryHash = u64;
type KnowledgeResult = Vec<Entity>;
```

---

Intelligence Layer in Axon provides task execution optimization through integration with Cortex, which provides all cognitive capabilities: learning, knowledge graphs, pattern recognition and context optimization.
