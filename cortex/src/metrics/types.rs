use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Histogram for latency tracking (Prometheus-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    /// Configurable buckets in milliseconds
    /// Default: [1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000]
    pub buckets: Vec<f64>,
    /// Counts for each bucket
    pub counts: Vec<u64>,
    /// Sum of all observed values
    pub sum: f64,
    /// Total count of observations
    pub count: u64,
}

impl Histogram {
    /// Create a new histogram with default buckets
    pub fn new() -> Self {
        Self::with_buckets(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
        ])
    }

    /// Create a new histogram with custom buckets
    pub fn with_buckets(buckets: Vec<f64>) -> Self {
        let counts = vec![0; buckets.len()];
        Self {
            buckets,
            counts,
            sum: 0.0,
            count: 0,
        }
    }

    /// Record a new observation
    pub fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;

        // Find the appropriate bucket
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                self.counts[i] += 1;
                break;
            }
        }
    }

    /// Calculate the mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Calculate percentile (approximate based on buckets)
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        let target_count = (self.count as f64 * p / 100.0) as u64;
        let mut cumulative = 0u64;

        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target_count {
                return self.buckets[i];
            }
        }

        // Return the last bucket if we didn't find it
        *self.buckets.last().unwrap_or(&0.0)
    }

    /// Get the p50 (median) latency
    pub fn p50(&self) -> f64 {
        self.percentile(50.0)
    }

    /// Get the p95 latency
    pub fn p95(&self) -> f64 {
        self.percentile(95.0)
    }

    /// Get the p99 latency
    pub fn p99(&self) -> f64 {
        self.percentile(99.0)
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe histogram wrapper for concurrent updates
#[derive(Debug)]
pub struct ConcurrentHistogram {
    inner: Arc<RwLock<Histogram>>,
}

impl ConcurrentHistogram {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Histogram::new())),
        }
    }

    pub fn with_buckets(buckets: Vec<f64>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Histogram::with_buckets(buckets))),
        }
    }

    pub fn observe(&self, value: f64) {
        self.inner.write().observe(value);
    }

    pub fn snapshot(&self) -> Histogram {
        self.inner.read().clone()
    }
}

impl Default for ConcurrentHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ConcurrentHistogram {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Per-tool metrics
#[derive(Debug)]
pub struct ToolMetrics {
    pub total_calls: AtomicU64,
    pub success_count: AtomicU64,
    pub error_count: AtomicU64,
    pub latency_histogram: ConcurrentHistogram,
    pub total_input_tokens: AtomicU64,
    pub total_output_tokens: AtomicU64,
    pub error_breakdown: DashMap<String, AtomicU64>,
    pub last_24h_calls: AtomicU64,
}

impl ToolMetrics {
    pub fn new() -> Self {
        Self {
            total_calls: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            latency_histogram: ConcurrentHistogram::new(),
            total_input_tokens: AtomicU64::new(0),
            total_output_tokens: AtomicU64::new(0),
            error_breakdown: DashMap::new(),
            last_24h_calls: AtomicU64::new(0),
        }
    }

    /// Record a successful call
    pub fn record_success(&self, latency_ms: f64) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.last_24h_calls.fetch_add(1, Ordering::Relaxed);
        self.latency_histogram.observe(latency_ms);
    }

    /// Record a failed call
    pub fn record_error(&self, latency_ms: f64, error_type: &str) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.error_count.fetch_add(1, Ordering::Relaxed);
        self.last_24h_calls.fetch_add(1, Ordering::Relaxed);
        self.latency_histogram.observe(latency_ms);

        // Update error breakdown
        self.error_breakdown
            .entry(error_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record token usage
    pub fn record_tokens(&self, input: u64, output: u64) {
        self.total_input_tokens.fetch_add(input, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_calls.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        let success = self.success_count.load(Ordering::Relaxed);
        success as f64 / total as f64
    }

    /// Create a serializable snapshot
    pub fn snapshot(&self) -> ToolMetricsSnapshot {
        let error_breakdown: HashMap<String, u64> = self
            .error_breakdown
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .collect();

        ToolMetricsSnapshot {
            total_calls: self.total_calls.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            latency_histogram: self.latency_histogram.snapshot(),
            total_input_tokens: self.total_input_tokens.load(Ordering::Relaxed),
            total_output_tokens: self.total_output_tokens.load(Ordering::Relaxed),
            error_breakdown,
            last_24h_calls: self.last_24h_calls.load(Ordering::Relaxed),
            success_rate: self.success_rate(),
        }
    }
}

impl Default for ToolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable snapshot of tool metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetricsSnapshot {
    pub total_calls: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub latency_histogram: Histogram,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub error_breakdown: HashMap<String, u64>,
    pub last_24h_calls: u64,
    pub success_rate: f64,
}

/// Memory system metrics
#[derive(Debug)]
pub struct MemoryMetrics {
    // Episodic
    pub total_episodes: AtomicU64,
    pub episodes_last_24h: AtomicU64,
    pub avg_episode_usefulness: parking_lot::RwLock<f64>,

    // Working
    pub cache_hit_rate: parking_lot::RwLock<f64>,
    pub cache_size_mb: parking_lot::RwLock<f64>,
    pub prefetch_accuracy: parking_lot::RwLock<f64>,

    // Semantic
    pub total_patterns: AtomicU64,
    pub knowledge_graph_nodes: AtomicU64,

    // Procedural
    pub total_procedures: AtomicU64,
    pub procedure_success_rate: parking_lot::RwLock<f64>,
}

impl MemoryMetrics {
    pub fn new() -> Self {
        Self {
            total_episodes: AtomicU64::new(0),
            episodes_last_24h: AtomicU64::new(0),
            avg_episode_usefulness: parking_lot::RwLock::new(0.0),
            cache_hit_rate: parking_lot::RwLock::new(0.0),
            cache_size_mb: parking_lot::RwLock::new(0.0),
            prefetch_accuracy: parking_lot::RwLock::new(0.0),
            total_patterns: AtomicU64::new(0),
            knowledge_graph_nodes: AtomicU64::new(0),
            total_procedures: AtomicU64::new(0),
            procedure_success_rate: parking_lot::RwLock::new(0.0),
        }
    }

    pub fn snapshot(&self) -> MemoryMetricsSnapshot {
        MemoryMetricsSnapshot {
            total_episodes: self.total_episodes.load(Ordering::Relaxed),
            episodes_last_24h: self.episodes_last_24h.load(Ordering::Relaxed),
            avg_episode_usefulness: *self.avg_episode_usefulness.read(),
            cache_hit_rate: *self.cache_hit_rate.read(),
            cache_size_mb: *self.cache_size_mb.read(),
            prefetch_accuracy: *self.prefetch_accuracy.read(),
            total_patterns: self.total_patterns.load(Ordering::Relaxed),
            knowledge_graph_nodes: self.knowledge_graph_nodes.load(Ordering::Relaxed),
            total_procedures: self.total_procedures.load(Ordering::Relaxed),
            procedure_success_rate: *self.procedure_success_rate.read(),
        }
    }
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetricsSnapshot {
    pub total_episodes: u64,
    pub episodes_last_24h: u64,
    pub avg_episode_usefulness: f64,
    pub cache_hit_rate: f64,
    pub cache_size_mb: f64,
    pub prefetch_accuracy: f64,
    pub total_patterns: u64,
    pub knowledge_graph_nodes: u64,
    pub total_procedures: u64,
    pub procedure_success_rate: f64,
}

/// Search metrics
#[derive(Debug)]
pub struct SearchMetrics {
    pub total_queries: AtomicU64,
    pub semantic_queries: AtomicU64,
    pub text_queries: AtomicU64,
    pub avg_query_latency_ms: parking_lot::RwLock<f64>,
    pub avg_results_returned: parking_lot::RwLock<f64>,
    pub rerank_calls: AtomicU64,
    pub avg_rerank_latency_ms: parking_lot::RwLock<f64>,
}

impl SearchMetrics {
    pub fn new() -> Self {
        Self {
            total_queries: AtomicU64::new(0),
            semantic_queries: AtomicU64::new(0),
            text_queries: AtomicU64::new(0),
            avg_query_latency_ms: parking_lot::RwLock::new(0.0),
            avg_results_returned: parking_lot::RwLock::new(0.0),
            rerank_calls: AtomicU64::new(0),
            avg_rerank_latency_ms: parking_lot::RwLock::new(0.0),
        }
    }

    pub fn snapshot(&self) -> SearchMetricsSnapshot {
        SearchMetricsSnapshot {
            total_queries: self.total_queries.load(Ordering::Relaxed),
            semantic_queries: self.semantic_queries.load(Ordering::Relaxed),
            text_queries: self.text_queries.load(Ordering::Relaxed),
            avg_query_latency_ms: *self.avg_query_latency_ms.read(),
            avg_results_returned: *self.avg_results_returned.read(),
            rerank_calls: self.rerank_calls.load(Ordering::Relaxed),
            avg_rerank_latency_ms: *self.avg_rerank_latency_ms.read(),
        }
    }
}

impl Default for SearchMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetricsSnapshot {
    pub total_queries: u64,
    pub semantic_queries: u64,
    pub text_queries: u64,
    pub avg_query_latency_ms: f64,
    pub avg_results_returned: f64,
    pub rerank_calls: u64,
    pub avg_rerank_latency_ms: f64,
}

/// Session metrics
#[derive(Debug)]
pub struct SessionMetrics {
    pub total_sessions: AtomicU64,
    pub active_sessions: AtomicU64,
    pub avg_session_duration_minutes: parking_lot::RwLock<f64>,
    pub queries_per_session: parking_lot::RwLock<f64>,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self {
            total_sessions: AtomicU64::new(0),
            active_sessions: AtomicU64::new(0),
            avg_session_duration_minutes: parking_lot::RwLock::new(0.0),
            queries_per_session: parking_lot::RwLock::new(0.0),
        }
    }

    pub fn snapshot(&self) -> SessionMetricsSnapshot {
        SessionMetricsSnapshot {
            total_sessions: self.total_sessions.load(Ordering::Relaxed),
            active_sessions: self.active_sessions.load(Ordering::Relaxed),
            avg_session_duration_minutes: *self.avg_session_duration_minutes.read(),
            queries_per_session: *self.queries_per_session.read(),
        }
    }
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetricsSnapshot {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub avg_session_duration_minutes: f64,
    pub queries_per_session: f64,
}

/// Token efficiency metrics
#[derive(Debug)]
pub struct TokenEfficiencyMetrics {
    pub total_input_tokens: AtomicU64,
    pub total_output_tokens: AtomicU64,
    pub tokens_saved_compression: AtomicU64,
    pub tokens_saved_deduplication: AtomicU64,
    pub avg_compression_ratio: parking_lot::RwLock<f64>,
}

impl TokenEfficiencyMetrics {
    pub fn new() -> Self {
        Self {
            total_input_tokens: AtomicU64::new(0),
            total_output_tokens: AtomicU64::new(0),
            tokens_saved_compression: AtomicU64::new(0),
            tokens_saved_deduplication: AtomicU64::new(0),
            avg_compression_ratio: parking_lot::RwLock::new(0.0),
        }
    }

    pub fn snapshot(&self) -> TokenEfficiencyMetricsSnapshot {
        TokenEfficiencyMetricsSnapshot {
            total_input_tokens: self.total_input_tokens.load(Ordering::Relaxed),
            total_output_tokens: self.total_output_tokens.load(Ordering::Relaxed),
            tokens_saved_compression: self.tokens_saved_compression.load(Ordering::Relaxed),
            tokens_saved_deduplication: self.tokens_saved_deduplication.load(Ordering::Relaxed),
            avg_compression_ratio: *self.avg_compression_ratio.read(),
        }
    }
}

impl Default for TokenEfficiencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEfficiencyMetricsSnapshot {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub tokens_saved_compression: u64,
    pub tokens_saved_deduplication: u64,
    pub avg_compression_ratio: f64,
}

/// System metrics
#[derive(Debug)]
pub struct SystemMetrics {
    pub cpu_usage_percent: parking_lot::RwLock<f64>,
    pub memory_usage_mb: parking_lot::RwLock<f64>,
    pub disk_usage_mb: parking_lot::RwLock<f64>,
    pub uptime_seconds: AtomicU64,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            cpu_usage_percent: parking_lot::RwLock::new(0.0),
            memory_usage_mb: parking_lot::RwLock::new(0.0),
            disk_usage_mb: parking_lot::RwLock::new(0.0),
            uptime_seconds: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> SystemMetricsSnapshot {
        SystemMetricsSnapshot {
            cpu_usage_percent: *self.cpu_usage_percent.read(),
            memory_usage_mb: *self.memory_usage_mb.read(),
            disk_usage_mb: *self.disk_usage_mb.read(),
            uptime_seconds: self.uptime_seconds.load(Ordering::Relaxed),
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsSnapshot {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_usage_mb: f64,
    pub uptime_seconds: u64,
}

/// Complete metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub tools: HashMap<String, ToolMetricsSnapshot>,
    pub memory: MemoryMetricsSnapshot,
    pub search: SearchMetricsSnapshot,
    pub sessions: SessionMetricsSnapshot,
    pub tokens: TokenEfficiencyMetricsSnapshot,
    pub system: SystemMetricsSnapshot,
}
