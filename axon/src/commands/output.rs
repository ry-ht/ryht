//! Output formatting utilities for CLI

use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use console::{style, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", "::");
pub static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "::");
pub static CHECK: Emoji<'_, '_> = Emoji("✓ ", "OK ");
pub static CROSS: Emoji<'_, '_> = Emoji("✗ ", "ERR");
pub static INFO: Emoji<'_, '_> = Emoji("ℹ ", "i ");

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OutputFormat {
    Human,
    Json,
    Plain,
}

/// Output format argument (for CLI compatibility)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OutputFormatArg {
    Human,
    Json,
    Plain,
}

impl From<OutputFormatArg> for OutputFormat {
    fn from(arg: OutputFormatArg) -> Self {
        match arg {
            OutputFormatArg::Human => OutputFormat::Human,
            OutputFormatArg::Json => OutputFormat::Json,
            OutputFormatArg::Plain => OutputFormat::Plain,
        }
    }
}

/// Create a spinner with message
pub fn spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid template"),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print success message
pub fn success(msg: impl Into<String>) {
    println!("{} {}", style(CHECK).green(), style(msg.into()).green());
}

/// Print error message
pub fn error(msg: impl Into<String>) {
    eprintln!("{} {}", style(CROSS).red(), style(msg.into()).red());
}

/// Print info message
pub fn info(msg: impl Into<String>) {
    println!("{} {}", style(INFO).cyan(), msg.into());
}

/// Print warning message
pub fn warn(msg: impl Into<String>) {
    println!("{} {}", style("⚠").yellow(), style(msg.into()).yellow());
}

/// Print agent table
pub fn print_agent_table(agents: &[crate::agents::AgentInfo], detailed: bool) {
    if agents.is_empty() {
        info("No agents found");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    if detailed {
        table.set_header(vec![
            "ID",
            "Name",
            "Type",
            "Status",
            "Tasks",
            "Success Rate",
            "Uptime",
        ]);

        for agent in agents {
            let uptime = format_duration(
                chrono::Utc::now()
                    .signed_duration_since(agent.last_heartbeat)
                    .to_std()
                    .unwrap_or_default(),
            );

            table.add_row(vec![
                Cell::new(&agent.id.to_string()[..8]),
                Cell::new(&agent.name),
                Cell::new(format!("{:?}", agent.agent_type)),
                Cell::new(format_status(agent.status)),
                Cell::new(format!(
                    "{}",
                    agent
                        .metadata
                        .max_concurrent_tasks
                )),
                Cell::new(format!("{:.1}%", agent.metadata.performance_score * 100.0)),
                Cell::new(uptime),
            ]);
        }
    } else {
        table.set_header(vec!["ID", "Name", "Type", "Status"]);

        for agent in agents {
            table.add_row(vec![
                Cell::new(&agent.id.to_string()[..8]),
                Cell::new(&agent.name),
                Cell::new(format!("{:?}", agent.agent_type)),
                Cell::new(format_status(agent.status)),
            ]);
        }
    }

    println!("{table}");
}

/// Print agent info
pub fn print_agent_info(agent: &crate::agents::AgentInfo) {
    println!("\n{} Agent Information\n", ROCKET);
    println!("  ID:           {}", agent.id);
    println!("  Name:         {}", agent.name);
    println!("  Type:         {:?}", agent.agent_type);
    println!("  Status:       {}", format_status(agent.status));
    println!("  Version:      {}", agent.metadata.version);
    println!("  Max Tasks:    {}", agent.metadata.max_concurrent_tasks);
    println!("  Performance:  {:.1}%", agent.metadata.performance_score * 100.0);

    if !agent.metadata.specialization.is_empty() {
        println!("  Specialization: {}", agent.metadata.specialization.join(", "));
    }

    if !agent.capabilities.is_empty() {
        println!("\n  Capabilities:");
        for cap in &agent.capabilities {
            println!("    - {:?}", cap);
        }
    }

    println!();
}

/// Print workflow table
pub fn print_workflow_table(workflows: &[WorkflowInfo]) {
    if workflows.is_empty() {
        info("No workflows found");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Name", "Status", "Progress", "Started"]);

    for workflow in workflows {
        table.add_row(vec![
            Cell::new(&workflow.id[..8]),
            Cell::new(&workflow.name),
            Cell::new(&workflow.status),
            Cell::new(format!("{}%", workflow.progress)),
            Cell::new(workflow.started_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]);
    }

    println!("{table}");
}

/// Print workflow status
pub fn print_workflow_status(status: &WorkflowStatus) {
    println!("\n{} Workflow Status\n", ROCKET);
    println!("  ID:         {}", status.id);
    println!("  Name:       {}", status.name);
    println!("  Status:     {}", status.status);
    println!("  Progress:   {}%", status.progress);
    println!("  Started:    {}", status.started_at.format("%Y-%m-%d %H:%M:%S"));

    if let Some(completed) = status.completed_at {
        println!("  Completed:  {}", completed.format("%Y-%m-%d %H:%M:%S"));
    }

    println!();
}

/// Print system status
pub fn print_system_status(status: &SystemStatus, detailed: bool) {
    println!("\n{} System Status\n", SPARKLE);
    println!("  Active Agents:      {}", status.active_agents);
    println!("  Running Workflows:  {}", status.running_workflows);
    println!("  Total Tasks:        {}", status.total_tasks);

    if detailed {
        println!("\n  Resource Usage:");
        println!("    CPU:     {:.1}%", status.cpu_usage);
        println!("    Memory:  {:.1}%", status.memory_usage);
        println!("    Threads: {}", status.thread_count);
    }

    println!();
}

/// Print configuration
pub fn print_config(config: &super::config::AxonConfig) {
    println!("\n{} Configuration\n", INFO);
    println!("  Workspace:  {}", config.workspace_name);
    println!("  Path:       {}", config.workspace_path.display());
    println!();
}

/// Print metrics table
pub fn print_metrics_table(metrics: &std::collections::HashMap<String, MetricsData>) {
    if metrics.is_empty() {
        info("No metrics found");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Agent", "Tasks", "Failed", "Success Rate", "Tokens", "Cost"]);

    for (id, metric) in metrics {
        table.add_row(vec![
            Cell::new(&id[..8]),
            Cell::new(metric.tasks_completed.to_string()),
            Cell::new(metric.tasks_failed.to_string()),
            Cell::new(format!("{}%", metric.success_rate)),
            Cell::new(metric.tokens_used.to_string()),
            Cell::new(format!("${:.2}", metric.total_cost_cents as f64 / 100.0)),
        ]);
    }

    println!("{table}");
}

/// Print telemetry
pub fn print_telemetry(telemetry: &TelemetryData) {
    println!("\n{} Telemetry (last {} minutes)\n", INFO, telemetry.range_minutes);
    println!("  Total Requests:   {}", telemetry.total_requests);
    println!("  Successful:       {}", telemetry.successful_requests);
    println!("  Failed:           {}", telemetry.failed_requests);
    println!("  Avg Response:     {}ms", telemetry.avg_response_time_ms);
    println!();
}

fn format_status(status: crate::agents::AgentStatus) -> String {
    use crate::agents::AgentStatus;

    match status {
        AgentStatus::Idle => style("Idle").cyan().to_string(),
        AgentStatus::Working => style("Working").green().to_string(),
        AgentStatus::Paused => style("Paused").yellow().to_string(),
        AgentStatus::Failed => style("Failed").red().to_string(),
        AgentStatus::ShuttingDown => style("Shutting Down").dim().to_string(),
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

// Helper types for output
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub progress: u8,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub progress: u8,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub active_agents: usize,
    pub running_workflows: usize,
    pub total_tasks: usize,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub thread_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsData {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub success_rate: u64,
    pub tokens_used: u64,
    pub total_cost_cents: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetryData {
    pub range_minutes: u64,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: u64,
}
