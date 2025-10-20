pub mod types;
pub mod collector;
pub mod storage;
pub mod self_improvement;

pub use types::*;
pub use collector::MetricsCollector;
pub use storage::{MetricsStorage, MetricsStorageStats, get_default_metrics_path, DEFAULT_METRICS_DB_PATH};
pub use self_improvement::{SelfImprovementMetrics, SelfImprovementCollector, TrendDirection, LanguageMetrics};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod end_to_end_test;
