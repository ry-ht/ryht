//! Performance Measurement Framework

use std::time::{Duration, Instant};

/// Performance measurement result
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation: String,
    pub duration: Duration,
    pub iterations: usize,
    pub avg_duration_ms: f64,
    pub ops_per_sec: f64,
}

impl PerformanceMetrics {
    pub fn print(&self) {
        println!("\n📊 Performance Metrics: {}", self.operation);
        println!("  Iterations:      {}", self.iterations);
        println!("  Total time:      {:?}", self.duration);
        println!("  Avg per op:      {:.2}ms", self.avg_duration_ms);
        println!("  Throughput:      {:.2} ops/sec", self.ops_per_sec);
    }
}

/// Performance benchmark runner
pub struct BenchmarkRunner {
    name: String,
    warmup_iterations: usize,
    measurement_iterations: usize,
}

impl BenchmarkRunner {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            warmup_iterations: 10,
            measurement_iterations: 100,
        }
    }

    pub fn with_iterations(mut self, warmup: usize, measurement: usize) -> Self {
        self.warmup_iterations = warmup;
        self.measurement_iterations = measurement;
        self
    }

    /// Run benchmark and return metrics
    pub fn run<F>(&self, mut operation: F) -> PerformanceMetrics
    where
        F: FnMut(),
    {
        // Warmup
        for _ in 0..self.warmup_iterations {
            operation();
        }

        // Measurement
        let start = Instant::now();
        for _ in 0..self.measurement_iterations {
            operation();
        }
        let duration = start.elapsed();

        let avg_duration_ms = duration.as_secs_f64 * 1000.0 / self.measurement_iterations as f64;
        let ops_per_sec = self.measurement_iterations as f64 / duration.as_secs_f64();

        PerformanceMetrics {
            operation: self.name.clone(),
            duration,
            iterations: self.measurement_iterations,
            avg_duration_ms,
            ops_per_sec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new("test_op").with_iterations(1, 10);
        let metrics = runner.run(|| {
            // Simulate work
            std::thread::sleep(Duration::from_millis(1));
        });

        assert_eq!(metrics.iterations, 10);
        assert!(metrics.avg_duration_ms >= 1.0);
        metrics.print();
    }
}
