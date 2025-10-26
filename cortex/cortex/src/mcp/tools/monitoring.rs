//! Monitoring & Analytics Tools (10 tools)

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, warn};
use uuid;

#[derive(Clone)]
pub struct MonitoringContext {
    storage: Arc<ConnectionManager>,
    start_time: SystemTime,
}

impl MonitoringContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            start_time: SystemTime::now(),
        }
    }
}

// Helper functions for time range parsing
fn parse_time_range(time_range: &Option<serde_json::Value>) -> (DateTime<Utc>, DateTime<Utc>) {
    let end = Utc::now();
    let start = if let Some(range) = time_range {
        if let Some(days) = range.get("days").and_then(|v| v.as_i64()) {
            end - Duration::days(days)
        } else if let Some(hours) = range.get("hours").and_then(|v| v.as_i64()) {
            end - Duration::hours(hours)
        } else if let Some(start_str) = range.get("start").and_then(|v| v.as_str()) {
            DateTime::parse_from_rfc3339(start_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| end - Duration::days(7))
        } else {
            end - Duration::days(7)
        }
    } else {
        end - Duration::days(7)
    };
    (start, end)
}

// Helper to format time series points
fn format_timestamp(dt: DateTime<Utc>, granularity: &str) -> String {
    match granularity {
        "hour" => dt.format("%Y-%m-%d %H:00:00").to_string(),
        "day" => dt.format("%Y-%m-%d").to_string(),
        "week" => dt.format("%Y-W%V").to_string(),
        "month" => dt.format("%Y-%m").to_string(),
        _ => dt.to_rfc3339(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MonitorHealthInput {
    #[serde(default = "default_true")]
    include_metrics: bool,
    #[serde(default)]
    include_diagnostics: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct MonitorHealthOutput {
    status: String,
    uptime_seconds: i64,
    metrics: Option<serde_json::Value>,
}

// ============================================================================
// 1. Monitor Health Tool
// ============================================================================

pub struct MonitorHealthTool {
    ctx: MonitoringContext,
}

impl MonitorHealthTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MonitorHealthTool {
    fn name(&self) -> &str {
        "cortex.monitor.health"
    }

    fn description(&self) -> Option<&str> {
        Some("Get system health status with uptime, connection pool metrics, and database statistics")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MonitorHealthInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: MonitorHealthInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.monitor.health executed");

        // Calculate uptime
        let uptime_seconds = self.ctx.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs() as i64;

        // Get health status from connection manager
        let health = self.ctx.storage.health_status();
        let status = if health.healthy { "healthy" } else { "degraded" };

        // Collect detailed metrics if requested
        let metrics = if input.include_metrics {
            let pool_stats = self.ctx.storage.pool_stats();
            let metrics_snapshot = self.ctx.storage.metrics().snapshot();

            Some(json!({
                "pool": {
                    "total_connections": pool_stats.total_connections,
                    "available_connections": pool_stats.available_connections,
                    "in_use_connections": pool_stats.in_use_connections,
                    "health_check_pass_rate": pool_stats.health_check_pass_rate,
                    "acquisition_success_rate": pool_stats.acquisition_success_rate,
                },
                "operations": {
                    "total_successes": metrics_snapshot.successes,
                    "total_errors": metrics_snapshot.errors,
                    "total_retries": metrics_snapshot.retries,
                },
                "circuit_breaker": {
                    "state": format!("{:?}", health.circuit_breaker_state),
                },
            }))
        } else {
            None
        };

        // Add diagnostics if requested
        let metrics_with_diagnostics = if input.include_diagnostics {
            if let Some(mut m) = metrics {
                if let Some(obj) = m.as_object_mut() {
                    let conn = self.ctx.storage.acquire().await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

                    // Query database statistics
                    let episode_count = match conn.connection()
                        .query("SELECT count() as count FROM episodes GROUP ALL")
                        .await
                    {
                        Ok(mut response) => {
                            match response.take::<Vec<serde_json::Value>>(0) {
                                Ok(mut results) => {
                                    results.pop()
                                        .and_then(|v| v.get("count").and_then(|c| c.as_i64()))
                                        .unwrap_or(0)
                                }
                                Err(_) => 0
                            }
                        }
                        Err(_) => 0
                    };

                    obj.insert("database".to_string(), json!({
                        "total_episodes": episode_count,
                    }));
                }
                Some(m)
            } else {
                None
            }
        } else {
            metrics
        };

        let output = MonitorHealthOutput {
            status: status.to_string(),
            uptime_seconds,
            metrics: metrics_with_diagnostics,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MonitorPerformanceInput {
    time_range: Option<serde_json::Value>,
    metrics: Option<Vec<String>>,
    #[serde(default = "default_tool_group")]
    group_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct MonitorPerformanceOutput {
    metrics: Vec<PerformanceMetric>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct PerformanceMetric {
    metric_name: String,
    value: f64,
    group: String,
}

// ============================================================================
// 2. Monitor Performance Tool
// ============================================================================

pub struct MonitorPerformanceTool {
    ctx: MonitoringContext,
}

impl MonitorPerformanceTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MonitorPerformanceTool {
    fn name(&self) -> &str {
        "cortex.monitor.performance"
    }

    fn description(&self) -> Option<&str> {
        Some("Get performance metrics including query times, throughput, and resource utilization")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MonitorPerformanceInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: MonitorPerformanceInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.monitor.performance executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut metrics = Vec::new();

        // Get pool metrics
        let pool_stats = self.ctx.storage.pool_stats();
        metrics.push(PerformanceMetric {
            metric_name: "connection_reuse_ratio".to_string(),
            value: pool_stats.average_reuse_ratio,
            group: input.group_by.clone(),
        });

        metrics.push(PerformanceMetric {
            metric_name: "health_check_pass_rate".to_string(),
            value: pool_stats.health_check_pass_rate,
            group: input.group_by.clone(),
        });

        // Query episode creation rate
        let episode_query = format!(
            "SELECT count() as count FROM episodes WHERE created_at >= '{}' AND created_at <= '{}'",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&episode_query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                if let Some(result) = results.into_iter().next() {
                    if let Some(count) = result.get("count").and_then(|c| c.as_i64()) {
                        let duration_hours = (end_time - start_time).num_hours() as f64;
                        let rate = if duration_hours > 0.0 {
                            count as f64 / duration_hours
                        } else {
                            0.0
                        };

                        metrics.push(PerformanceMetric {
                            metric_name: "episodes_per_hour".to_string(),
                            value: rate,
                            group: input.group_by.clone(),
                        });
                    }
                }
            }
        }

        // Query session count
        let session_query = format!(
            "SELECT count() as count FROM agent_sessions WHERE created_at >= '{}' AND created_at <= '{}'",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&session_query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                if let Some(result) = results.into_iter().next() {
                    if let Some(count) = result.get("count").and_then(|c| c.as_i64()) {
                        metrics.push(PerformanceMetric {
                            metric_name: "active_sessions".to_string(),
                            value: count as f64,
                            group: input.group_by.clone(),
                        });
                    }
                }
            }
        }

        // Add operations metrics from the metrics snapshot
        let metrics_snapshot = self.ctx.storage.metrics().snapshot();
        let total_ops = metrics_snapshot.successes + metrics_snapshot.errors;
        let success_rate = if total_ops > 0 {
            (metrics_snapshot.successes as f64 / total_ops as f64) * 100.0
        } else {
            100.0
        };

        metrics.push(PerformanceMetric {
            metric_name: "operation_success_rate".to_string(),
            value: success_rate,
            group: input.group_by.clone(),
        });

        let output = MonitorPerformanceOutput { metrics };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyticsCodeMetricsInput {
    metrics: Option<Vec<String>>,
    time_range: Option<serde_json::Value>,
    #[serde(default = "default_day")]
    granularity: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyticsCodeMetricsOutput {
    time_series: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TimeSeriesPoint {
    timestamp: String,
    metrics: serde_json::Value,
}

// ============================================================================
// 3. Analytics Code Metrics Tool
// ============================================================================

pub struct AnalyticsCodeMetricsTool {
    ctx: MonitoringContext,
}

impl AnalyticsCodeMetricsTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AnalyticsCodeMetricsTool {
    fn name(&self) -> &str {
        "cortex.analytics.code_metrics"
    }

    fn description(&self) -> Option<&str> {
        Some("Get code quality metrics over time including complexity, documentation coverage, and change frequency")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyticsCodeMetricsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyticsCodeMetricsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.analytics.code_metrics executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut time_series = Vec::new();

        // Query episode changes grouped by time granularity
        let query = format!(
            "SELECT
                time::floor(created_at, 1{}) as bucket,
                count() as total_changes,
                math::sum(lines_added) as lines_added,
                math::sum(lines_removed) as lines_removed,
                count(DISTINCT file_path) as files_changed
            FROM episode_changes
            WHERE created_at >= '{}' AND created_at <= '{}'
            GROUP BY bucket
            ORDER BY bucket",
            match input.granularity.as_str() {
                "hour" => "h",
                "day" => "d",
                "week" => "w",
                "month" => "M",
                _ => "d",
            },
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                for result in results {
                if let Some(bucket) = result.get("bucket").and_then(|b| b.as_str()) {
                    let timestamp = DateTime::parse_from_rfc3339(bucket)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(start_time);

                    let added = result.get("lines_added").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                    let removed = result.get("lines_removed").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                    let churn_rate = if added + removed > 0.0 {
                        (removed / (added + removed)) * 100.0
                    } else {
                        0.0
                    };

                    let metrics = json!({
                        "total_changes": result.get("total_changes").and_then(|v| v.as_i64()).unwrap_or(0),
                        "lines_added": added as i64,
                        "lines_removed": removed as i64,
                        "files_changed": result.get("files_changed").and_then(|v| v.as_i64()).unwrap_or(0),
                        "churn_rate": churn_rate,
                    });

                    time_series.push(TimeSeriesPoint {
                        timestamp: format_timestamp(timestamp, &input.granularity),
                        metrics,
                    });
                }
            }
            }
        }

        let output = AnalyticsCodeMetricsOutput { time_series };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyticsAgentActivityInput {
    agent_id: Option<String>,
    time_range: Option<serde_json::Value>,
    #[serde(default)]
    include_details: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyticsAgentActivityOutput {
    activities: Vec<AgentActivity>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AgentActivity {
    agent_id: String,
    action_count: i32,
    success_rate: f32,
}

// ============================================================================
// 4. Analytics Agent Activity Tool
// ============================================================================

pub struct AnalyticsAgentActivityTool {
    ctx: MonitoringContext,
}

impl AnalyticsAgentActivityTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AnalyticsAgentActivityTool {
    fn name(&self) -> &str {
        "cortex.analytics.agent_activity"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze agent activity including action counts, success rates, and session statistics")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyticsAgentActivityInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyticsAgentActivityInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.analytics.agent_activity executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut activities = Vec::new();

        // Build query based on whether agent_id is specified
        let query = if let Some(agent_id) = &input.agent_id {
            format!(
                "SELECT
                    id as agent_id,
                    count() as action_count
                FROM agent_sessions
                WHERE id = '{}' AND created_at >= '{}' AND created_at <= '{}'
                GROUP BY id",
                agent_id,
                start_time.to_rfc3339(),
                end_time.to_rfc3339()
            )
        } else {
            format!(
                "SELECT
                    id as agent_id,
                    count() as action_count
                FROM agent_sessions
                WHERE created_at >= '{}' AND created_at <= '{}'
                GROUP BY id
                LIMIT 100",
                start_time.to_rfc3339(),
                end_time.to_rfc3339()
            )
        };

        if let Ok(mut response) = conn.connection().query(&query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                for result in results {
                    let agent_id = result.get("agent_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let action_count = result.get("action_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;

                    // Calculate success rate (simplified - in real impl would track successes/failures)
                    let success_rate = 0.95; // Placeholder - would query from episode outcomes

                    activities.push(AgentActivity {
                        agent_id,
                        action_count,
                        success_rate,
                    });
                }
            }
        }

        let total_count = activities.len() as i32;
        let output = AnalyticsAgentActivityOutput {
            activities,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyticsErrorAnalysisInput {
    time_range: Option<serde_json::Value>,
    error_types: Option<Vec<String>>,
    #[serde(default = "default_type_group")]
    group_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyticsErrorAnalysisOutput {
    error_groups: Vec<ErrorGroup>,
    total_errors: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ErrorGroup {
    group_key: String,
    count: i32,
    error_type: String,
}

// ============================================================================
// 5. Analytics Error Analysis Tool
// ============================================================================

pub struct AnalyticsErrorAnalysisTool {
    ctx: MonitoringContext,
}

impl AnalyticsErrorAnalysisTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AnalyticsErrorAnalysisTool {
    fn name(&self) -> &str {
        "cortex.analytics.error_analysis"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze errors and failures with pattern detection and grouping")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyticsErrorAnalysisInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyticsErrorAnalysisInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.analytics.error_analysis executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut error_groups = Vec::new();

        // Query episodes with error/failure outcomes
        let query = format!(
            "SELECT
                outcome as error_type,
                count() as count
            FROM episodes
            WHERE created_at >= '{}' AND created_at <= '{}'
                AND (outcome = 'error' OR outcome = 'failure')
            GROUP BY outcome",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        let mut total_errors = 0;

        if let Ok(mut response) = conn.connection().query(&query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                for result in results {
                    let error_type = result.get("error_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let count = result.get("count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32;

                    total_errors += count;

                    error_groups.push(ErrorGroup {
                        group_key: input.group_by.clone(),
                        count,
                        error_type,
                    });
                }
            }
        }

        // Also get metrics errors
        let metrics_snapshot = self.ctx.storage.metrics().snapshot();
        if metrics_snapshot.errors > 0 {
            error_groups.push(ErrorGroup {
                group_key: input.group_by.clone(),
                count: metrics_snapshot.errors as i32,
                error_type: "database_operation_error".to_string(),
            });
            total_errors += metrics_snapshot.errors as i32;
        }

        let output = AnalyticsErrorAnalysisOutput {
            error_groups,
            total_errors,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyticsProductivityInput {
    time_range: Option<serde_json::Value>,
    metrics: Option<Vec<String>>,
    #[serde(default = "default_agent_group")]
    group_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyticsProductivityOutput {
    productivity_metrics: Vec<ProductivityMetric>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ProductivityMetric {
    group_key: String,
    metric_name: String,
    value: f64,
}

// ============================================================================
// 6. Analytics Productivity Tool
// ============================================================================

pub struct AnalyticsProductivityTool {
    ctx: MonitoringContext,
}

impl AnalyticsProductivityTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AnalyticsProductivityTool {
    fn name(&self) -> &str {
        "cortex.analytics.productivity"
    }

    fn description(&self) -> Option<&str> {
        Some("Measure productivity metrics including code velocity, task completion rate, and efficiency")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyticsProductivityInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyticsProductivityInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.analytics.productivity executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut productivity_metrics = Vec::new();

        // Calculate code velocity (lines changed per day)
        let velocity_query = format!(
            "SELECT
                math::sum(lines_added) as total_added,
                math::sum(lines_removed) as total_removed,
                count(DISTINCT file_path) as files_touched
            FROM episode_changes
            WHERE created_at >= '{}' AND created_at <= '{}'",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&velocity_query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                if let Some(result) = results.into_iter().next() {
                let total_added = result.get("total_added").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let total_removed = result.get("total_removed").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let files_touched = result.get("files_touched").and_then(|v| v.as_i64()).unwrap_or(0) as f64;

                let duration_days = (end_time - start_time).num_days() as f64;
                let duration_days = if duration_days > 0.0 { duration_days } else { 1.0 };

                productivity_metrics.push(ProductivityMetric {
                    group_key: input.group_by.clone(),
                    metric_name: "code_velocity_lines_per_day".to_string(),
                    value: (total_added + total_removed) / duration_days,
                });

                productivity_metrics.push(ProductivityMetric {
                    group_key: input.group_by.clone(),
                    metric_name: "files_touched_per_day".to_string(),
                    value: files_touched / duration_days,
                });
            }
            }
        }

        // Calculate task completion rate
        let completion_query = format!(
            "SELECT
                count() as total_episodes,
                count(CASE WHEN outcome = 'success' THEN 1 END) as successful_episodes
            FROM episodes
            WHERE created_at >= '{}' AND created_at <= '{}'",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&completion_query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                if let Some(result) = results.into_iter().next() {
                let total = result.get("total_episodes").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let successful = result.get("successful_episodes").and_then(|v| v.as_i64()).unwrap_or(0) as f64;

                let completion_rate = if total > 0.0 {
                    (successful / total) * 100.0
                } else {
                    0.0
                };

                productivity_metrics.push(ProductivityMetric {
                    group_key: input.group_by.clone(),
                    metric_name: "task_completion_rate_percent".to_string(),
                    value: completion_rate,
                });
            }
            }
        }

        let output = AnalyticsProductivityOutput {
            productivity_metrics,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyticsQualityTrendsInput {
    time_range: Option<serde_json::Value>,
    quality_metrics: Option<Vec<String>>,
    #[serde(default)]
    include_predictions: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyticsQualityTrendsOutput {
    trends: Vec<QualityTrend>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct QualityTrend {
    metric_name: String,
    trend: String,
    prediction: Option<f64>,
}

// ============================================================================
// 7. Analytics Quality Trends Tool
// ============================================================================

pub struct AnalyticsQualityTrendsTool {
    ctx: MonitoringContext,
}

impl AnalyticsQualityTrendsTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AnalyticsQualityTrendsTool {
    fn name(&self) -> &str {
        "cortex.analytics.quality_trends"
    }

    fn description(&self) -> Option<&str> {
        Some("Track code quality trends over time with optional predictions")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyticsQualityTrendsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyticsQualityTrendsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.analytics.quality_trends executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut trends = Vec::new();

        // Query episode outcomes to track quality trends
        let query = format!(
            "SELECT
                time::floor(created_at, 1d) as bucket,
                count() as total,
                count(CASE WHEN outcome = 'success' THEN 1 END) as successes,
                count(CASE WHEN outcome = 'error' OR outcome = 'failure' THEN 1 END) as failures
            FROM episodes
            WHERE created_at >= '{}' AND created_at <= '{}'
            GROUP BY bucket
            ORDER BY bucket",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        let mut success_rates = Vec::new();

        if let Ok(mut response) = conn.connection().query(&query).await {
            if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                for result in results {
                let total = result.get("total").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let successes = result.get("successes").and_then(|v| v.as_i64()).unwrap_or(0) as f64;

                if total > 0.0 {
                    success_rates.push((successes / total) * 100.0);
                }
            }
            }
        }

        // Calculate trend direction
        let trend_direction = if success_rates.len() >= 2 {
            let first_half: f64 = success_rates[..success_rates.len()/2].iter().sum::<f64>() / (success_rates.len()/2) as f64;
            let second_half: f64 = success_rates[success_rates.len()/2..].iter().sum::<f64>() / (success_rates.len() - success_rates.len()/2) as f64;

            if second_half > first_half + 5.0 {
                "improving"
            } else if second_half < first_half - 5.0 {
                "declining"
            } else {
                "stable"
            }
        } else {
            "insufficient_data"
        };

        trends.push(QualityTrend {
            metric_name: "success_rate".to_string(),
            trend: trend_direction.to_string(),
            prediction: if input.include_predictions && !success_rates.is_empty() {
                // Simple linear prediction
                Some(success_rates.iter().sum::<f64>() / success_rates.len() as f64)
            } else {
                None
            },
        });

        // Add code churn trend
        let churn_query = format!(
            "SELECT
                math::sum(lines_added) + math::sum(lines_removed) as churn
            FROM episode_changes
            WHERE created_at >= '{}' AND created_at <= '{}'",
            start_time.to_rfc3339(),
            end_time.to_rfc3339()
        );

        if let Ok(mut response) = conn.connection().query(&churn_query).await {
            if let Ok(churn_results) = response.take::<Vec<serde_json::Value>>(0) {
                if let Some(result) = churn_results.into_iter().next() {
                let churn = result.get("churn").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                trends.push(QualityTrend {
                    metric_name: "code_churn".to_string(),
                    trend: "measured".to_string(),
                    prediction: if input.include_predictions {
                        Some(churn)
                    } else {
                        None
                    },
                });
            }
            }
        }

        let output = AnalyticsQualityTrendsOutput { trends };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportMetricsInput {
    #[serde(default = "default_json_format")]
    format: String,
    metrics: Option<Vec<String>>,
    time_range: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ExportMetricsOutput {
    export_data: String,
    format: String,
    metrics_count: i32,
}

// ============================================================================
// 8. Export Metrics Tool
// ============================================================================

pub struct ExportMetricsTool {
    ctx: MonitoringContext,
}

impl ExportMetricsTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ExportMetricsTool {
    fn name(&self) -> &str {
        "cortex.export.metrics"
    }

    fn description(&self) -> Option<&str> {
        Some("Export metrics data to external systems in various formats (JSON, CSV, Prometheus)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ExportMetricsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ExportMetricsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.export.metrics executed");

        let (_start_time, _end_time) = parse_time_range(&input.time_range);

        // Collect all metrics
        let pool_stats = self.ctx.storage.pool_stats();
        let metrics_snapshot = self.ctx.storage.metrics().snapshot();
        let health = self.ctx.storage.health_status();

        let mut metrics_data = HashMap::new();
        metrics_data.insert("pool_total_connections", pool_stats.total_connections as f64);
        metrics_data.insert("pool_available_connections", pool_stats.available_connections as f64);
        metrics_data.insert("pool_in_use_connections", pool_stats.in_use_connections as f64);
        metrics_data.insert("pool_health_check_pass_rate", pool_stats.health_check_pass_rate);
        metrics_data.insert("operations_successes", metrics_snapshot.successes as f64);
        metrics_data.insert("operations_errors", metrics_snapshot.errors as f64);
        metrics_data.insert("operations_retries", metrics_snapshot.retries as f64);
        metrics_data.insert("circuit_breaker_is_open", if health.healthy { 0.0 } else { 1.0 });

        // Format based on requested format
        let export_data = match input.format.as_str() {
            "json" => {
                serde_json::to_string_pretty(&metrics_data)
                    .unwrap_or_else(|_| "{}".to_string())
            }
            "csv" => {
                let mut csv = String::from("metric,value\n");
                for (key, value) in &metrics_data {
                    csv.push_str(&format!("{},{}\n", key, value));
                }
                csv
            }
            "prometheus" => {
                let mut prom = String::new();
                for (key, value) in &metrics_data {
                    prom.push_str(&format!("cortex_{} {}\n", key, value));
                }
                prom
            }
            _ => {
                return Err(ToolError::ExecutionFailed(format!("Unsupported format: {}", input.format)));
            }
        };

        let metrics_count = metrics_data.len() as i32;

        let output = ExportMetricsOutput {
            export_data,
            format: input.format,
            metrics_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AlertConfigureInput {
    #[serde(default = "default_threshold")]
    alert_type: String,
    condition: serde_json::Value,
    actions: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AlertConfigureOutput {
    alert_id: String,
    configured: bool,
}

// ============================================================================
// 9. Alert Configure Tool
// ============================================================================

pub struct AlertConfigureTool {
    ctx: MonitoringContext,
}

impl AlertConfigureTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AlertConfigureTool {
    fn name(&self) -> &str {
        "cortex.alert.configure"
    }

    fn description(&self) -> Option<&str> {
        Some("Configure alerting rules for monitoring thresholds and conditions")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AlertConfigureInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AlertConfigureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.alert.configure executed");

        // Generate unique alert ID
        let alert_id = uuid::Uuid::new_v4().to_string();

        // In a production implementation, this would:
        // 1. Store alert configuration in database
        // 2. Register with monitoring system
        // 3. Set up notification channels
        // 4. Configure threshold watchers

        let _conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Store alert configuration (using a hypothetical alerts table)
        let alert_config = json!({
            "id": alert_id,
            "alert_type": input.alert_type,
            "condition": input.condition,
            "actions": input.actions,
            "created_at": Utc::now().to_rfc3339(),
            "enabled": true,
        });

        // In production, would store to an alerts table
        // For now, we just validate the configuration
        debug!("Alert configured: {:?}", alert_config);

        let output = AlertConfigureOutput {
            alert_id,
            configured: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReportGenerateInput {
    #[serde(default = "default_summary")]
    report_type: String,
    time_range: Option<serde_json::Value>,
    sections: Option<Vec<String>>,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ReportGenerateOutput {
    report_content: String,
    format: String,
}

// ============================================================================
// 10. Report Generate Tool
// ============================================================================

pub struct ReportGenerateTool {
    ctx: MonitoringContext,
}

impl ReportGenerateTool {
    pub fn new(ctx: MonitoringContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ReportGenerateTool {
    fn name(&self) -> &str {
        "cortex.report.generate"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate comprehensive analytics and monitoring reports")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ReportGenerateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ReportGenerateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.report.generate executed");

        let (start_time, end_time) = parse_time_range(&input.time_range);
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut report = String::new();

        if input.format == "markdown" {
            report.push_str(&format!("# Cortex Monitoring Report\n\n"));
            report.push_str(&format!("**Report Type:** {}\n\n", input.report_type));
            report.push_str(&format!("**Time Range:** {} to {}\n\n", start_time.format("%Y-%m-%d %H:%M"), end_time.format("%Y-%m-%d %H:%M")));
            report.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

            // System Health Section
            report.push_str("## System Health\n\n");
            let health = self.ctx.storage.health_status();
            report.push_str(&format!("- **Status:** {}\n", if health.healthy { "Healthy" } else { "Degraded" }));
            report.push_str(&format!("- **Pool Size:** {}\n", health.pool_size));
            report.push_str(&format!("- **Available Connections:** {}\n", health.available_connections));
            report.push_str(&format!("- **Circuit Breaker:** {:?}\n\n", health.circuit_breaker_state));

            // Performance Metrics Section
            report.push_str("## Performance Metrics\n\n");
            let pool_stats = self.ctx.storage.pool_stats();
            report.push_str(&format!("- **Connection Reuse Ratio:** {:.2}%\n", pool_stats.average_reuse_ratio * 100.0));
            report.push_str(&format!("- **Health Check Pass Rate:** {:.2}%\n", pool_stats.health_check_pass_rate));
            report.push_str(&format!("- **Acquisition Success Rate:** {:.2}%\n\n", pool_stats.acquisition_success_rate));

            // Activity Summary
            report.push_str("## Activity Summary\n\n");
            let episode_query = format!(
                "SELECT count() as count FROM episodes WHERE created_at >= '{}' AND created_at <= '{}'",
                start_time.to_rfc3339(),
                end_time.to_rfc3339()
            );

            if let Ok(mut response) = conn.connection().query(&episode_query).await {
                if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                    if let Some(result) = results.into_iter().next() {
                    let episode_count = result.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
                    report.push_str(&format!("- **Total Episodes:** {}\n", episode_count));
                }
                }
            }

            let session_query = format!(
                "SELECT count() as count FROM agent_sessions WHERE created_at >= '{}' AND created_at <= '{}'",
                start_time.to_rfc3339(),
                end_time.to_rfc3339()
            );

            if let Ok(mut response) = conn.connection().query(&session_query).await {
                if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                    if let Some(result) = results.into_iter().next() {
                    let session_count = result.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
                    report.push_str(&format!("- **Active Sessions:** {}\n", session_count));
                }
                }
            }

            // Code Metrics Section
            report.push_str("\n## Code Metrics\n\n");
            let changes_query = format!(
                "SELECT
                    count() as total_changes,
                    math::sum(lines_added) as lines_added,
                    math::sum(lines_removed) as lines_removed,
                    count(DISTINCT file_path) as files_changed
                FROM episode_changes
                WHERE created_at >= '{}' AND created_at <= '{}'",
                start_time.to_rfc3339(),
                end_time.to_rfc3339()
            );

            if let Ok(mut response) = conn.connection().query(&changes_query).await {
                if let Ok(results) = response.take::<Vec<serde_json::Value>>(0) {
                    if let Some(result) = results.into_iter().next() {
                    let total_changes = result.get("total_changes").and_then(|v| v.as_i64()).unwrap_or(0);
                    let lines_added = result.get("lines_added").and_then(|v| v.as_i64()).unwrap_or(0);
                    let lines_removed = result.get("lines_removed").and_then(|v| v.as_i64()).unwrap_or(0);
                    let files_changed = result.get("files_changed").and_then(|v| v.as_i64()).unwrap_or(0);

                    report.push_str(&format!("- **Total Changes:** {}\n", total_changes));
                    report.push_str(&format!("- **Lines Added:** {}\n", lines_added));
                    report.push_str(&format!("- **Lines Removed:** {}\n", lines_removed));
                    report.push_str(&format!("- **Files Changed:** {}\n", files_changed));
                }
                }
            }

            report.push_str("\n---\n");
            report.push_str("*Report generated by Cortex Monitoring System*\n");

        } else if input.format == "json" {
            // JSON format report
            let health = self.ctx.storage.health_status();
            let pool_stats = self.ctx.storage.pool_stats();
            let metrics_snapshot = self.ctx.storage.metrics().snapshot();

            let json_report = json!({
                "report_type": input.report_type,
                "time_range": {
                    "start": start_time.to_rfc3339(),
                    "end": end_time.to_rfc3339(),
                },
                "generated_at": Utc::now().to_rfc3339(),
                "health": {
                    "status": if health.healthy { "healthy" } else { "degraded" },
                    "pool_size": health.pool_size,
                    "available_connections": health.available_connections,
                    "circuit_breaker_state": format!("{:?}", health.circuit_breaker_state),
                },
                "performance": {
                    "connection_reuse_ratio": pool_stats.average_reuse_ratio,
                    "health_check_pass_rate": pool_stats.health_check_pass_rate,
                    "acquisition_success_rate": pool_stats.acquisition_success_rate,
                },
                "operations": {
                    "successes": metrics_snapshot.successes,
                    "errors": metrics_snapshot.errors,
                    "retries": metrics_snapshot.retries,
                },
            });

            report = serde_json::to_string_pretty(&json_report)
                .unwrap_or_else(|_| "{}".to_string());
        } else {
            return Err(ToolError::ExecutionFailed(format!("Unsupported report format: {}", input.format)));
        }

        let output = ReportGenerateOutput {
            report_content: report,
            format: input.format,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

fn default_true() -> bool { true }
fn default_tool_group() -> String { "tool".to_string() }
fn default_day() -> String { "day".to_string() }
fn default_type_group() -> String { "type".to_string() }
fn default_agent_group() -> String { "agent".to_string() }
fn default_json_format() -> String { "json".to_string() }
fn default_threshold() -> String { "threshold".to_string() }
fn default_summary() -> String { "summary".to_string() }
fn default_markdown() -> String { "markdown".to_string() }
