//! Metrics collection and aggregation

use super::*;

pub struct MetricsCollector {
    total_tasks: AtomicU64,
    successful_tasks: AtomicU64,
    failed_tasks: AtomicU64,
    total_duration_ms: AtomicU64,
    total_tokens: AtomicU64,
    total_cost_cents: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_tasks: AtomicU64::new(0),
            successful_tasks: AtomicU64::new(0),
            failed_tasks: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_cost_cents: AtomicU64::new(0),
        }
    }

    pub fn record_task_completion(&self, duration_ms: u64, tokens: u64, cost_cents: u64, success: bool) {
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
        self.total_duration_ms.fetch_add(duration_ms, Ordering::Relaxed);
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_cost_cents.fetch_add(cost_cents, Ordering::Relaxed);

        if success {
            self.successful_tasks.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_tasks.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_tasks: self.total_tasks.load(Ordering::Relaxed),
            successful_tasks: self.successful_tasks.load(Ordering::Relaxed),
            failed_tasks: self.failed_tasks.load(Ordering::Relaxed),
            avg_duration_ms: self.calculate_average_duration(),
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_cost_dollars: self.total_cost_cents.load(Ordering::Relaxed) as f64 / 100.0,
            success_rate: self.calculate_success_rate(),
            timestamp: Utc::now(),
        }
    }

    fn calculate_average_duration(&self) -> u64 {
        let total = self.total_tasks.load(Ordering::Relaxed);
        if total > 0 {
            self.total_duration_ms.load(Ordering::Relaxed) / total
        } else {
            0
        }
    }

    fn calculate_success_rate(&self) -> f64 {
        let total = self.total_tasks.load(Ordering::Relaxed);
        if total > 0 {
            let successful = self.successful_tasks.load(Ordering::Relaxed);
            (successful as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub avg_duration_ms: u64,
    pub total_tokens: u64,
    pub total_cost_dollars: f64,
    pub success_rate: f64,
    pub timestamp: DateTime<Utc>,
}
