//! Monitoring & Analytics Tools (10 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct MonitoringContext {
    storage: Arc<ConnectionManager>,
}

impl MonitoringContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_monitor_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: MonitoringContext,
        }

        impl $name {
            pub fn new(ctx: MonitoringContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
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

impl_monitor_tool!(MonitorHealthTool, "cortex.monitor.health", "Get system health status", MonitorHealthInput, MonitorHealthOutput);

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

impl_monitor_tool!(MonitorPerformanceTool, "cortex.monitor.performance", "Get performance metrics", MonitorPerformanceInput, MonitorPerformanceOutput);

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

impl_monitor_tool!(AnalyticsCodeMetricsTool, "cortex.analytics.code_metrics", "Get code metrics over time", AnalyticsCodeMetricsInput, AnalyticsCodeMetricsOutput);

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

impl_monitor_tool!(AnalyticsAgentActivityTool, "cortex.analytics.agent_activity", "Analyze agent activity", AnalyticsAgentActivityInput, AnalyticsAgentActivityOutput);

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

impl_monitor_tool!(AnalyticsErrorAnalysisTool, "cortex.analytics.error_analysis", "Analyze errors and failures", AnalyticsErrorAnalysisInput, AnalyticsErrorAnalysisOutput);

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

impl_monitor_tool!(AnalyticsProductivityTool, "cortex.analytics.productivity", "Measure productivity metrics", AnalyticsProductivityInput, AnalyticsProductivityOutput);

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

impl_monitor_tool!(AnalyticsQualityTrendsTool, "cortex.analytics.quality_trends", "Track quality trends", AnalyticsQualityTrendsInput, AnalyticsQualityTrendsOutput);

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

impl_monitor_tool!(ExportMetricsTool, "cortex.export.metrics", "Export metrics data", ExportMetricsInput, ExportMetricsOutput);

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

impl_monitor_tool!(AlertConfigureTool, "cortex.alert.configure", "Configure alerts", AlertConfigureInput, AlertConfigureOutput);

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

impl_monitor_tool!(ReportGenerateTool, "cortex.report.generate", "Generate analytics report", ReportGenerateInput, ReportGenerateOutput);

fn default_true() -> bool { true }
fn default_tool_group() -> String { "tool".to_string() }
fn default_day() -> String { "day".to_string() }
fn default_type_group() -> String { "type".to_string() }
fn default_agent_group() -> String { "agent".to_string() }
fn default_json_format() -> String { "json".to_string() }
fn default_threshold() -> String { "threshold".to_string() }
fn default_summary() -> String { "summary".to_string() }
fn default_markdown() -> String { "markdown".to_string() }
