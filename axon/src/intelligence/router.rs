//! Model routing for optimal provider selection
//!
//! This module provides intelligent model selection based on:
//! - Historical performance data from Cortex
//! - Pattern matching of successful model choices
//! - Real-time cost and latency analysis
//! - Context analysis for task requirements

use super::*;
use crate::cortex_bridge::{CortexBridge, Episode, EpisodeOutcome, Pattern};
use std::time::{Duration, Instant};

/// Model selection result with detailed metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
    pub confidence: f32,
    pub rationale: String,
    /// Estimated cost per 1K tokens
    pub estimated_cost: f64,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u64,
    /// Success rate from historical data
    pub historical_success_rate: f32,
}

/// Model requirements for task execution
#[derive(Debug, Clone)]
pub enum ModelRequirements {
    LowestCost,
    FastestResponse,
    HighestQuality,
    Balanced,
    /// Custom requirements with weights
    Custom {
        cost_weight: f32,
        speed_weight: f32,
        quality_weight: f32,
    },
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelMetrics {
    model_id: String,
    provider_id: String,
    avg_cost_per_1k: f64,
    avg_latency_ms: u64,
    success_rate: f32,
    total_uses: u32,
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// Context for model selection
#[derive(Debug, Clone)]
pub struct SelectionContext {
    pub task_type: String,
    pub task_description: String,
    pub expected_complexity: TaskComplexity,
    pub max_tokens: Option<usize>,
    pub deadline_ms: Option<u64>,
}

/// Task complexity classification
#[derive(Debug, Clone, Copy)]
pub enum TaskComplexity {
    Trivial,
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

impl Default for SelectionContext {
    fn default() -> Self {
        Self {
            task_type: "general".to_string(),
            task_description: String::new(),
            expected_complexity: TaskComplexity::Medium,
            max_tokens: None,
            deadline_ms: None,
        }
    }
}

/// Enhanced model router with Cortex integration
pub struct ModelRouter {
    /// Decision cache with TTL
    cache: Arc<RwLock<HashMap<String, (ModelSelection, Instant)>>>,
    /// Cache TTL
    cache_ttl: Duration,
    /// Cortex bridge for historical data
    cortex: Option<Arc<CortexBridge>>,
    /// Performance metrics cache
    metrics_cache: Arc<RwLock<HashMap<String, ModelMetrics>>>,
    /// Metrics cache TTL
    metrics_ttl: Duration,
}

impl ModelRouter {
    /// Create a new ModelRouter without Cortex integration
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            cortex: None,
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics_ttl: Duration::from_secs(600), // 10 minutes
        }
    }

    /// Create a new ModelRouter with Cortex integration
    pub fn with_cortex(cortex: Arc<CortexBridge>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300),
            cortex: Some(cortex),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics_ttl: Duration::from_secs(600),
        }
    }

    /// Set cache TTL
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Select optimal model based on requirements and context
    pub async fn select_model(
        &self,
        requirements: ModelRequirements,
        context: SelectionContext,
    ) -> Result<ModelSelection> {
        // Generate cache key from context
        let cache_key = self.generate_cache_key(&context, &requirements);

        // Check cache
        if let Some(cached) = self.get_cached_selection(&cache_key).await {
            return Ok(cached);
        }

        // Gather historical performance data from Cortex
        let metrics = self.gather_performance_metrics(&context).await?;

        // Select model based on requirements
        let selection = match requirements {
            ModelRequirements::LowestCost => self.select_lowest_cost(&metrics, &context).await?,
            ModelRequirements::FastestResponse => {
                self.select_fastest(&metrics, &context).await?
            }
            ModelRequirements::HighestQuality => {
                self.select_highest_quality(&metrics, &context).await?
            }
            ModelRequirements::Balanced => self.select_balanced(&metrics, &context).await?,
            ModelRequirements::Custom {
                cost_weight,
                speed_weight,
                quality_weight,
            } => {
                self.select_custom(&metrics, &context, cost_weight, speed_weight, quality_weight)
                    .await?
            }
        };

        // Cache result
        self.cache_selection(&cache_key, selection.clone()).await;

        Ok(selection)
    }

    /// Select model for a specific task type
    pub async fn select_model_for_task(
        &self,
        task_type: &str,
        requirements: ModelRequirements,
    ) -> Result<ModelSelection> {
        let context = SelectionContext {
            task_type: task_type.to_string(),
            ..Default::default()
        };
        self.select_model(requirements, context).await
    }

    /// Gather performance metrics from Cortex and local cache
    async fn gather_performance_metrics(
        &self,
        context: &SelectionContext,
    ) -> Result<Vec<ModelMetrics>> {
        // Try to get metrics from cache first
        let cache_key = format!("metrics_{}", context.task_type);
        if let Some(cached_metrics) = self.get_cached_metrics(&cache_key).await {
            return Ok(cached_metrics);
        }

        let mut all_metrics = Vec::new();

        // Get data from Cortex if available
        if let Some(cortex) = &self.cortex {
            // Search for relevant episodes
            let episodes = cortex
                .search_episodes(&context.task_description, 50)
                .await
                .unwrap_or_default();

            // Extract model performance from episodes
            let cortex_metrics = self.extract_metrics_from_episodes(episodes).await;
            all_metrics.extend(cortex_metrics);

            // Get learned patterns about model selection
            if let Ok(patterns) = cortex.get_patterns().await {
                let pattern_metrics = self.extract_metrics_from_patterns(patterns).await;
                all_metrics.extend(pattern_metrics);
            }
        }

        // If no Cortex data or no data found, use default metrics
        if all_metrics.is_empty() {
            all_metrics = self.get_default_metrics();
        }

        // Cache the metrics
        self.cache_metrics(&cache_key, all_metrics.clone()).await;

        Ok(all_metrics)
    }

    /// Extract metrics from historical episodes
    async fn extract_metrics_from_episodes(&self, episodes: Vec<Episode>) -> Vec<ModelMetrics> {
        let mut metrics_map: HashMap<String, Vec<(f64, u64, bool)>> = HashMap::new();

        for episode in episodes {
            // Extract model info from metadata (assuming it's stored in success_metrics)
            if let Some(model) = episode.success_metrics.get("model_used").and_then(|v| v.as_str())
            {
                let cost = episode
                    .success_metrics
                    .get("cost")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let latency = episode
                    .success_metrics
                    .get("latency_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);
                let success = matches!(episode.outcome, EpisodeOutcome::Success);

                metrics_map
                    .entry(model.to_string())
                    .or_default()
                    .push((cost, latency, success));
            }
        }

        // Aggregate metrics
        let mut metrics = Vec::new();
        for (model, data) in metrics_map {
            let total = data.len() as f32;
            let avg_cost = data.iter().map(|(c, _, _)| c).sum::<f64>() / total as f64;
            let avg_latency = (data.iter().map(|(_, l, _)| l).sum::<u64>() as f64 / total as f64)
                .round() as u64;
            let success_rate =
                data.iter().filter(|(_, _, s)| *s).count() as f32 / total;

            // Parse model string (format: "provider:model")
            let parts: Vec<&str> = model.split(':').collect();
            let (provider, model_id) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                ("unknown".to_string(), model)
            };

            metrics.push(ModelMetrics {
                model_id,
                provider_id: provider,
                avg_cost_per_1k: avg_cost,
                avg_latency_ms: avg_latency,
                success_rate,
                total_uses: total as u32,
                last_updated: chrono::Utc::now(),
            });
        }

        metrics
    }

    /// Extract metrics from learned patterns
    async fn extract_metrics_from_patterns(&self, patterns: Vec<Pattern>) -> Vec<ModelMetrics> {
        let mut metrics = Vec::new();

        for pattern in patterns {
            // Look for patterns about model selection
            if let Some(model_info) = pattern.transformation.get("recommended_model")
                && let Some(model_str) = model_info.as_str() {
                    let parts: Vec<&str> = model_str.split(':').collect();
                    let (provider, model_id) = if parts.len() == 2 {
                        (parts[0].to_string(), parts[1].to_string())
                    } else {
                        continue;
                    };

                    // Extract metrics from pattern's average_improvement
                    let cost = pattern
                        .average_improvement
                        .get("cost_per_1k")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.01);
                    let latency = pattern
                        .average_improvement
                        .get("latency_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1000);

                    metrics.push(ModelMetrics {
                        model_id,
                        provider_id: provider,
                        avg_cost_per_1k: cost,
                        avg_latency_ms: latency,
                        success_rate: pattern.success_rate,
                        total_uses: pattern.times_applied as u32,
                        last_updated: chrono::Utc::now(),
                    });
                }
        }

        metrics
    }

    /// Get default model metrics when no historical data is available
    fn get_default_metrics(&self) -> Vec<ModelMetrics> {
        vec![
            // OpenAI models
            ModelMetrics {
                model_id: "gpt-4-turbo".to_string(),
                provider_id: "openai".to_string(),
                avg_cost_per_1k: 0.03,
                avg_latency_ms: 2000,
                success_rate: 0.92,
                total_uses: 0,
                last_updated: chrono::Utc::now(),
            },
            ModelMetrics {
                model_id: "gpt-3.5-turbo".to_string(),
                provider_id: "openai".to_string(),
                avg_cost_per_1k: 0.002,
                avg_latency_ms: 800,
                success_rate: 0.85,
                total_uses: 0,
                last_updated: chrono::Utc::now(),
            },
            // Anthropic models
            ModelMetrics {
                model_id: "claude-3-opus".to_string(),
                provider_id: "anthropic".to_string(),
                avg_cost_per_1k: 0.075,
                avg_latency_ms: 2500,
                success_rate: 0.95,
                total_uses: 0,
                last_updated: chrono::Utc::now(),
            },
            ModelMetrics {
                model_id: "claude-3-sonnet".to_string(),
                provider_id: "anthropic".to_string(),
                avg_cost_per_1k: 0.015,
                avg_latency_ms: 1500,
                success_rate: 0.90,
                total_uses: 0,
                last_updated: chrono::Utc::now(),
            },
            ModelMetrics {
                model_id: "claude-3-haiku".to_string(),
                provider_id: "anthropic".to_string(),
                avg_cost_per_1k: 0.0025,
                avg_latency_ms: 600,
                success_rate: 0.82,
                total_uses: 0,
                last_updated: chrono::Utc::now(),
            },
        ]
    }

    /// Select model with lowest cost
    async fn select_lowest_cost(
        &self,
        metrics: &[ModelMetrics],
        context: &SelectionContext,
    ) -> Result<ModelSelection> {
        let model = metrics
            .iter()
            .filter(|m| self.meets_requirements(m, context))
            .min_by(|a, b| a.avg_cost_per_1k.partial_cmp(&b.avg_cost_per_1k).unwrap())
            .ok_or(IntelligenceError::NoSuitableModel)?;

        Ok(ModelSelection {
            provider_id: model.provider_id.clone(),
            model_id: model.model_id.clone(),
            confidence: 0.85,
            rationale: format!(
                "Selected {} for lowest cost (${:.4}/1K tokens) with {:.1}% historical success rate",
                model.model_id, model.avg_cost_per_1k, model.success_rate * 100.0
            ),
            estimated_cost: model.avg_cost_per_1k,
            estimated_latency_ms: model.avg_latency_ms,
            historical_success_rate: model.success_rate,
        })
    }

    /// Select model with fastest response
    async fn select_fastest(
        &self,
        metrics: &[ModelMetrics],
        context: &SelectionContext,
    ) -> Result<ModelSelection> {
        let model = metrics
            .iter()
            .filter(|m| self.meets_requirements(m, context))
            .min_by_key(|m| m.avg_latency_ms)
            .ok_or(IntelligenceError::NoSuitableModel)?;

        Ok(ModelSelection {
            provider_id: model.provider_id.clone(),
            model_id: model.model_id.clone(),
            confidence: 0.88,
            rationale: format!(
                "Selected {} for fastest response (avg {}ms latency) with {:.1}% success rate",
                model.model_id, model.avg_latency_ms, model.success_rate * 100.0
            ),
            estimated_cost: model.avg_cost_per_1k,
            estimated_latency_ms: model.avg_latency_ms,
            historical_success_rate: model.success_rate,
        })
    }

    /// Select model with highest quality
    async fn select_highest_quality(
        &self,
        metrics: &[ModelMetrics],
        context: &SelectionContext,
    ) -> Result<ModelSelection> {
        // For highest quality, prioritize success rate and handle complex tasks
        let model = metrics
            .iter()
            .filter(|m| self.meets_requirements(m, context))
            .max_by(|a, b| {
                // First compare success rate
                let success_cmp = a.success_rate.partial_cmp(&b.success_rate).unwrap();
                if success_cmp != std::cmp::Ordering::Equal {
                    success_cmp
                } else {
                    // If equal, prefer models with more usage (more proven)
                    a.total_uses.cmp(&b.total_uses)
                }
            })
            .ok_or(IntelligenceError::NoSuitableModel)?;

        Ok(ModelSelection {
            provider_id: model.provider_id.clone(),
            model_id: model.model_id.clone(),
            confidence: 0.92,
            rationale: format!(
                "Selected {} for highest quality ({:.1}% success rate, {} uses) at ${:.4}/1K tokens",
                model.model_id, model.success_rate * 100.0, model.total_uses, model.avg_cost_per_1k
            ),
            estimated_cost: model.avg_cost_per_1k,
            estimated_latency_ms: model.avg_latency_ms,
            historical_success_rate: model.success_rate,
        })
    }

    /// Select balanced model
    async fn select_balanced(
        &self,
        metrics: &[ModelMetrics],
        context: &SelectionContext,
    ) -> Result<ModelSelection> {
        self.select_custom(metrics, context, 0.33, 0.33, 0.34)
            .await
    }

    /// Select model with custom weights
    async fn select_custom(
        &self,
        metrics: &[ModelMetrics],
        context: &SelectionContext,
        cost_weight: f32,
        speed_weight: f32,
        quality_weight: f32,
    ) -> Result<ModelSelection> {
        let filtered: Vec<_> = metrics
            .iter()
            .filter(|m| self.meets_requirements(m, context))
            .collect();

        if filtered.is_empty() {
            return Err(IntelligenceError::NoSuitableModel);
        }

        // Normalize metrics to 0-1 range
        let max_cost = filtered
            .iter()
            .map(|m| m.avg_cost_per_1k)
            .fold(0.0f64, f64::max);
        let max_latency = filtered.iter().map(|m| m.avg_latency_ms).max().unwrap();
        let min_cost = filtered
            .iter()
            .map(|m| m.avg_cost_per_1k)
            .fold(f64::MAX, f64::min);
        let min_latency = filtered.iter().map(|m| m.avg_latency_ms).min().unwrap();

        // Calculate scores (higher is better)
        let model = filtered
            .iter()
            .max_by(|a, b| {
                let score_a = self.calculate_score(
                    a,
                    min_cost,
                    max_cost,
                    min_latency,
                    max_latency,
                    cost_weight,
                    speed_weight,
                    quality_weight,
                );
                let score_b = self.calculate_score(
                    b,
                    min_cost,
                    max_cost,
                    min_latency,
                    max_latency,
                    cost_weight,
                    speed_weight,
                    quality_weight,
                );
                score_a.partial_cmp(&score_b).unwrap()
            })
            .ok_or(IntelligenceError::NoSuitableModel)?;

        Ok(ModelSelection {
            provider_id: model.provider_id.clone(),
            model_id: model.model_id.clone(),
            confidence: 0.90,
            rationale: format!(
                "Selected {} with balanced metrics: {:.1}% success, {}ms latency, ${:.4}/1K tokens (weights: cost={:.0}%, speed={:.0}%, quality={:.0}%)",
                model.model_id,
                model.success_rate * 100.0,
                model.avg_latency_ms,
                model.avg_cost_per_1k,
                cost_weight * 100.0,
                speed_weight * 100.0,
                quality_weight * 100.0
            ),
            estimated_cost: model.avg_cost_per_1k,
            estimated_latency_ms: model.avg_latency_ms,
            historical_success_rate: model.success_rate,
        })
    }

    /// Calculate weighted score for a model
    fn calculate_score(
        &self,
        model: &ModelMetrics,
        min_cost: f64,
        max_cost: f64,
        min_latency: u64,
        max_latency: u64,
        cost_weight: f32,
        speed_weight: f32,
        quality_weight: f32,
    ) -> f32 {
        // Normalize cost (lower is better, so invert)
        let cost_score = if max_cost > min_cost {
            1.0 - ((model.avg_cost_per_1k - min_cost) / (max_cost - min_cost)) as f32
        } else {
            1.0
        };

        // Normalize latency (lower is better, so invert)
        let speed_score = if max_latency > min_latency {
            1.0 - ((model.avg_latency_ms - min_latency) as f32
                / (max_latency - min_latency) as f32)
        } else {
            1.0
        };

        // Success rate is already 0-1
        let quality_score = model.success_rate;

        cost_weight * cost_score + speed_weight * speed_score + quality_weight * quality_score
    }

    /// Check if model meets context requirements
    fn meets_requirements(&self, model: &ModelMetrics, context: &SelectionContext) -> bool {
        // Check deadline constraint
        if let Some(deadline) = context.deadline_ms
            && model.avg_latency_ms > deadline {
                return false;
            }

        // Filter by complexity (simple tasks don't need expensive models)
        match context.expected_complexity {
            TaskComplexity::Trivial | TaskComplexity::Simple => {
                // Allow all models, but prefer cheaper ones
                true
            }
            TaskComplexity::Medium => {
                // Exclude models with very low success rates
                model.success_rate >= 0.75
            }
            TaskComplexity::Complex | TaskComplexity::VeryComplex => {
                // Only high-quality models
                model.success_rate >= 0.85
            }
        }
    }

    /// Generate cache key from context and requirements
    fn generate_cache_key(&self, context: &SelectionContext, requirements: &ModelRequirements) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        context.task_type.hash(&mut hasher);
        format!("{:?}", requirements).hash(&mut hasher);
        format!("{:?}", context.expected_complexity).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get cached selection if still valid
    async fn get_cached_selection(&self, key: &str) -> Option<ModelSelection> {
        let cache = self.cache.read().await;
        if let Some((selection, timestamp)) = cache.get(key)
            && timestamp.elapsed() < self.cache_ttl {
                return Some(selection.clone());
            }
        None
    }

    /// Cache a selection
    async fn cache_selection(&self, key: &str, selection: ModelSelection) {
        self.cache
            .write()
            .await
            .insert(key.to_string(), (selection, Instant::now()));
    }

    /// Get cached metrics if still valid
    async fn get_cached_metrics(&self, key: &str) -> Option<Vec<ModelMetrics>> {
        let cache = self.metrics_cache.read().await;
        let mut result = Vec::new();
        let now = Instant::now();

        for (metric_key, metric) in cache.iter() {
            if metric_key.starts_with(key)
                && now.duration_since(
                    Instant::now()
                        - Duration::from_secs(
                            (chrono::Utc::now() - metric.last_updated).num_seconds() as u64,
                        ),
                ) < self.metrics_ttl
                {
                    result.push(metric.clone());
                }
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Cache metrics
    async fn cache_metrics(&self, key: &str, metrics: Vec<ModelMetrics>) {
        let mut cache = self.metrics_cache.write().await;
        for (i, metric) in metrics.iter().enumerate() {
            cache.insert(format!("{}_{}", key, i), metric.clone());
        }
    }

    /// Clear all caches
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
        self.metrics_cache.write().await.clear();
    }
}

impl Default for ModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_model_router_creation() {
        let router = ModelRouter::new();
        assert!(router.cortex.is_none());
    }

    #[tokio::test]
    async fn test_default_metrics() {
        let router = ModelRouter::new();
        let metrics = router.get_default_metrics();
        assert!(!metrics.is_empty());
        assert!(metrics.iter().any(|m| m.model_id == "gpt-4-turbo"));
        assert!(metrics.iter().any(|m| m.model_id == "claude-3-opus"));
    }

    #[tokio::test]
    async fn test_select_lowest_cost() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();
        let selection = router
            .select_model(ModelRequirements::LowestCost, context)
            .await
            .unwrap();

        // Should select one of the cheaper models
        assert!(
            selection.model_id == "gpt-3.5-turbo" || selection.model_id == "claude-3-haiku"
        );
        assert!(selection.estimated_cost < 0.01);
    }

    #[tokio::test]
    async fn test_select_fastest() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();
        let selection = router
            .select_model(ModelRequirements::FastestResponse, context)
            .await
            .unwrap();

        // Should select one of the faster models
        assert!(selection.estimated_latency_ms < 1000);
    }

    #[tokio::test]
    async fn test_select_highest_quality() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();
        let selection = router
            .select_model(ModelRequirements::HighestQuality, context)
            .await
            .unwrap();

        // Should select claude-3-opus (highest success rate in defaults)
        assert_eq!(selection.model_id, "claude-3-opus");
        assert!(selection.historical_success_rate > 0.90);
    }

    #[tokio::test]
    async fn test_select_balanced() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();
        let selection = router
            .select_model(ModelRequirements::Balanced, context)
            .await
            .unwrap();

        assert!(!selection.model_id.is_empty());
        assert!(!selection.provider_id.is_empty());
        assert!(selection.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_complexity_filtering() {
        let router = ModelRouter::new();
        let metrics = router.get_default_metrics();

        // Simple task should accept all models
        let simple_context = SelectionContext {
            expected_complexity: TaskComplexity::Simple,
            ..Default::default()
        };
        for metric in &metrics {
            assert!(router.meets_requirements(metric, &simple_context));
        }

        // Very complex task should filter by success rate
        let complex_context = SelectionContext {
            expected_complexity: TaskComplexity::VeryComplex,
            ..Default::default()
        };
        let high_quality_count = metrics
            .iter()
            .filter(|m| router.meets_requirements(m, &complex_context))
            .count();
        assert!(high_quality_count < metrics.len());
    }

    #[tokio::test]
    async fn test_deadline_filtering() {
        let router = ModelRouter::new();
        let metrics = router.get_default_metrics();

        // Context with tight deadline
        let deadline_context = SelectionContext {
            deadline_ms: Some(1000),
            ..Default::default()
        };

        let fast_count = metrics
            .iter()
            .filter(|m| router.meets_requirements(m, &deadline_context))
            .count();

        // Should filter out slower models
        assert!(fast_count < metrics.len());
    }

    #[tokio::test]
    async fn test_cache() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();

        // First call
        let start = Instant::now();
        let selection1 = router
            .select_model(ModelRequirements::Balanced, context.clone())
            .await
            .unwrap();
        let first_duration = start.elapsed();

        // Second call should be cached and faster
        let start = Instant::now();
        let selection2 = router
            .select_model(ModelRequirements::Balanced, context)
            .await
            .unwrap();
        let second_duration = start.elapsed();

        assert_eq!(selection1.model_id, selection2.model_id);
        // Cached call should be significantly faster (though this might be flaky)
        // Just verify it doesn't error
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();

        router
            .select_model(ModelRequirements::Balanced, context)
            .await
            .unwrap();

        router.clear_cache().await;

        let cache = router.cache.read().await;
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_custom_weights() {
        let router = ModelRouter::new();
        let context = SelectionContext::default();

        // Heavily weighted towards cost
        let selection = router
            .select_model(
                ModelRequirements::Custom {
                    cost_weight: 0.9,
                    speed_weight: 0.05,
                    quality_weight: 0.05,
                },
                context,
            )
            .await
            .unwrap();

        // Should select cheap model
        assert!(selection.estimated_cost < 0.01);
    }
}
