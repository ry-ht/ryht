//! Dashboard and visualization support

use super::*;

pub struct Dashboard {
    metrics_collector: Arc<MetricsCollector>,
}

impl Dashboard {
    pub fn new(metrics_collector: Arc<MetricsCollector>) -> Self {
        Self { metrics_collector }
    }

    pub fn generate_report(&self) -> String {
        let snapshot = self.metrics_collector.snapshot();

        format!(
            "=== Axon Multi-Agent System Dashboard ===\n\
             Total Tasks: {}\n\
             Successful: {}\n\
             Failed: {}\n\
             Success Rate: {:.2}%\n\
             Avg Duration: {}ms\n\
             Total Tokens: {}\n\
             Total Cost: ${:.2}\n\
             Timestamp: {}\n",
            snapshot.total_tasks,
            snapshot.successful_tasks,
            snapshot.failed_tasks,
            snapshot.success_rate,
            snapshot.avg_duration_ms,
            snapshot.total_tokens,
            snapshot.total_cost_dollars,
            snapshot.timestamp
        )
    }
}
