use super::types::*;
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

/// Lock-free metrics collector
///
/// This collector uses atomic operations and lock-free data structures (DashMap)
/// to minimize overhead during metric collection. Target overhead: <1μs per metric.
#[derive(Debug)]
pub struct MetricsCollector {
    tool_metrics: Arc<DashMap<String, Arc<ToolMetrics>>>,
    memory_metrics: Arc<MemoryMetrics>,
    search_metrics: Arc<SearchMetrics>,
    session_metrics: Arc<SessionMetrics>,
    token_metrics: Arc<TokenEfficiencyMetrics>,
    system_metrics: Arc<SystemMetrics>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            tool_metrics: Arc::new(DashMap::new()),
            memory_metrics: Arc::new(MemoryMetrics::new()),
            search_metrics: Arc::new(SearchMetrics::new()),
            session_metrics: Arc::new(SessionMetrics::new()),
            token_metrics: Arc::new(TokenEfficiencyMetrics::new()),
            system_metrics: Arc::new(SystemMetrics::new()),
        }
    }

    /// Record a tool call with latency and success status
    ///
    /// This is a lock-free operation using atomic counters and DashMap.
    /// Expected overhead: <1μs
    pub fn record_tool_call(&self, tool_name: &str, latency_ms: f64, success: bool) {
        let metrics = self
            .tool_metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| Arc::new(ToolMetrics::new()));

        if success {
            metrics.record_success(latency_ms);
        } else {
            // Default error type if not specified
            metrics.record_error(latency_ms, "unknown");
        }
    }

    /// Record a tool call with error details
    pub fn record_tool_error(&self, tool_name: &str, latency_ms: f64, error_type: &str) {
        let metrics = self
            .tool_metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| Arc::new(ToolMetrics::new()));

        metrics.record_error(latency_ms, error_type);
    }

    /// Record token usage for a tool
    pub fn record_tokens(&self, tool_name: &str, input: u64, output: u64) {
        let metrics = self
            .tool_metrics
            .entry(tool_name.to_string())
            .or_insert_with(|| Arc::new(ToolMetrics::new()));

        metrics.record_tokens(input, output);

        // Also update global token metrics
        use std::sync::atomic::Ordering;
        self.token_metrics
            .total_input_tokens
            .fetch_add(input, Ordering::Relaxed);
        self.token_metrics
            .total_output_tokens
            .fetch_add(output, Ordering::Relaxed);
    }

    /// Record memory metrics
    pub fn update_memory_metrics<F>(&self, updater: F)
    where
        F: FnOnce(&MemoryMetrics),
    {
        updater(&self.memory_metrics);
    }

    /// Record search metrics
    pub fn update_search_metrics<F>(&self, updater: F)
    where
        F: FnOnce(&SearchMetrics),
    {
        updater(&self.search_metrics);
    }

    /// Record session metrics
    pub fn update_session_metrics<F>(&self, updater: F)
    where
        F: FnOnce(&SessionMetrics),
    {
        updater(&self.session_metrics);
    }

    /// Record token efficiency metrics
    pub fn update_token_efficiency<F>(&self, updater: F)
    where
        F: FnOnce(&TokenEfficiencyMetrics),
    {
        updater(&self.token_metrics);
    }

    /// Record system metrics
    pub fn update_system_metrics<F>(&self, updater: F)
    where
        F: FnOnce(&SystemMetrics),
    {
        updater(&self.system_metrics);
    }

    /// Take a snapshot of all current metrics
    ///
    /// This creates a serializable snapshot of all metrics at the current point in time.
    /// The snapshot is consistent (all metrics are captured at approximately the same time).
    pub fn take_snapshot(&self) -> MetricsSnapshot {
        // Capture tool metrics
        let tools: HashMap<String, ToolMetricsSnapshot> = self
            .tool_metrics
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().snapshot()))
            .collect();

        MetricsSnapshot {
            timestamp: Utc::now(),
            tools,
            memory: self.memory_metrics.snapshot(),
            search: self.search_metrics.snapshot(),
            sessions: self.session_metrics.snapshot(),
            tokens: self.token_metrics.snapshot(),
            system: self.system_metrics.snapshot(),
        }
    }

    /// Reset all metrics (useful for testing)
    #[cfg(test)]
    pub fn reset(&self) {
        self.tool_metrics.clear();
        // Note: Cannot reset Arc-wrapped metrics in place
        // Users should create a new MetricsCollector instead
    }

    /// Get metrics for a specific tool
    pub fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolMetricsSnapshot> {
        self.tool_metrics.get(tool_name).map(|m| m.snapshot())
    }

    /// Get all tool names
    pub fn get_tool_names(&self) -> Vec<String> {
        self.tool_metrics.iter().map(|e| e.key().clone()).collect()
    }

    /// Get memory metrics reference for direct updates
    pub fn memory_metrics(&self) -> Arc<MemoryMetrics> {
        Arc::clone(&self.memory_metrics)
    }

    /// Get search metrics reference for direct updates
    pub fn search_metrics(&self) -> Arc<SearchMetrics> {
        Arc::clone(&self.search_metrics)
    }

    /// Get session metrics reference for direct updates
    pub fn session_metrics(&self) -> Arc<SessionMetrics> {
        Arc::clone(&self.session_metrics)
    }

    /// Get token metrics reference for direct updates
    pub fn token_metrics(&self) -> Arc<TokenEfficiencyMetrics> {
        Arc::clone(&self.token_metrics)
    }

    /// Get system metrics reference for direct updates
    pub fn system_metrics(&self) -> Arc<SystemMetrics> {
        Arc::clone(&self.system_metrics)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            tool_metrics: Arc::clone(&self.tool_metrics),
            memory_metrics: Arc::clone(&self.memory_metrics),
            search_metrics: Arc::clone(&self.search_metrics),
            session_metrics: Arc::clone(&self.session_metrics),
            token_metrics: Arc::clone(&self.token_metrics),
            system_metrics: Arc::clone(&self.system_metrics),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_tool_call() {
        let collector = MetricsCollector::new();

        // Record successful calls
        collector.record_tool_call("test_tool", 10.5, true);
        collector.record_tool_call("test_tool", 15.2, true);
        collector.record_tool_call("test_tool", 20.1, false);

        let metrics = collector.get_tool_metrics("test_tool").unwrap();
        assert_eq!(metrics.total_calls, 3);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.error_count, 1);
        assert!((metrics.success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_record_tokens() {
        let collector = MetricsCollector::new();

        collector.record_tokens("test_tool", 100, 50);
        collector.record_tokens("test_tool", 200, 75);

        let metrics = collector.get_tool_metrics("test_tool").unwrap();
        assert_eq!(metrics.total_input_tokens, 300);
        assert_eq!(metrics.total_output_tokens, 125);

        // Check global token metrics
        let snapshot = collector.take_snapshot();
        assert_eq!(snapshot.tokens.total_input_tokens, 300);
        assert_eq!(snapshot.tokens.total_output_tokens, 125);
    }

    #[test]
    fn test_histogram_recording() {
        let collector = MetricsCollector::new();

        // Record various latencies
        for latency in [5.0, 10.0, 15.0, 25.0, 50.0, 100.0, 200.0] {
            collector.record_tool_call("test_tool", latency, true);
        }

        let metrics = collector.get_tool_metrics("test_tool").unwrap();
        let hist = &metrics.latency_histogram;

        assert_eq!(hist.count, 7);
        assert!(hist.mean() > 0.0);
        assert!(hist.p50() > 0.0);
        assert!(hist.p95() > 0.0);
        assert!(hist.p99() > 0.0);
    }

    #[test]
    fn test_error_breakdown() {
        let collector = MetricsCollector::new();

        collector.record_tool_error("test_tool", 10.0, "timeout");
        collector.record_tool_error("test_tool", 15.0, "timeout");
        collector.record_tool_error("test_tool", 20.0, "not_found");

        let metrics = collector.get_tool_metrics("test_tool").unwrap();
        assert_eq!(metrics.error_count, 3);
        assert_eq!(*metrics.error_breakdown.get("timeout").unwrap(), 2);
        assert_eq!(*metrics.error_breakdown.get("not_found").unwrap(), 1);
    }

    #[test]
    fn test_snapshot() {
        let collector = MetricsCollector::new();

        collector.record_tool_call("tool1", 10.0, true);
        collector.record_tool_call("tool2", 20.0, true);

        let snapshot = collector.take_snapshot();
        assert!(snapshot.tools.contains_key("tool1"));
        assert!(snapshot.tools.contains_key("tool2"));
        assert!(snapshot.timestamp <= Utc::now());
    }

    #[test]
    fn test_concurrent_updates() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // Spawn multiple threads updating metrics concurrently
        for i in 0..10 {
            let collector = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    collector.record_tool_call(
                        &format!("tool_{}", i % 3),
                        (j as f64) * 1.5,
                        j % 2 == 0,
                    );
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify total calls
        let snapshot = collector.take_snapshot();
        let total_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
        assert_eq!(total_calls, 1000); // 10 threads * 100 calls each
    }
}
