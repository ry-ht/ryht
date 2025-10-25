//! Telemetry export and reporting

use super::*;

pub struct TelemetryExporter {
    enabled: bool,
}

impl TelemetryExporter {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn export(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // Export to logging/monitoring system
        log::info!(
            "Metrics: tasks={}, success_rate={:.2}%, cost=${:.2}",
            snapshot.total_tasks,
            snapshot.success_rate,
            snapshot.total_cost_dollars
        );

        Ok(())
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

impl Default for TelemetryExporter {
    fn default() -> Self {
        Self::new()
    }
}
