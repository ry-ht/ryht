//! Performance Monitoring and Metrics
//!
//! Comprehensive monitoring for agents, workflows, and system performance.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

pub mod metrics;
pub mod telemetry;
pub mod dashboard;

pub use metrics::*;
pub use telemetry::*;
pub use dashboard::*;

/// Main monitoring coordinator
pub struct MonitoringCoordinator {
    metrics_collector: Arc<MetricsCollector>,
    telemetry_exporter: Arc<TelemetryExporter>,
}

impl MonitoringCoordinator {
    pub fn new() -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
            telemetry_exporter: Arc::new(TelemetryExporter::new()),
        }
    }

    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    pub fn telemetry(&self) -> &TelemetryExporter {
        &self.telemetry_exporter
    }
}

impl Default for MonitoringCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for monitoring operations
pub type Result<T> = std::result::Result<T, MonitoringError>;

/// Monitoring errors
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Metrics collection failed: {0}")]
    CollectionFailed(String),

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
