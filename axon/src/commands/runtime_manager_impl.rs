//! Runtime Manager Implementation - Manages agent and workflow lifecycles
//!
//! This is a simplified runtime manager for the CLI commands.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use chrono::Utc;

use crate::agents::AgentType;

/// Runtime Manager for CLI commands
pub struct RuntimeManager {
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    workflows: Arc<RwLock<HashMap<String, WorkflowInfo>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub agent_type: AgentType,
    pub status: String,
    pub model: Option<String>,
    pub capabilities: Vec<String>,
    pub started_at: String,
    pub metrics: Option<AgentMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub errors: u64,
    pub avg_response_time_ms: f64,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub tasks_completed: usize,
    pub total_tasks: usize,
    pub current_tasks: Vec<TaskInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub agent: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryData {
    pub request_rate: f64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub active_agents: usize,
    pub active_workflows: usize,
    pub top_errors: Vec<(String, u64)>,
}

impl RuntimeManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_agent(
        &self,
        name: String,
        agent_type: AgentType,
        capabilities: Vec<String>,
        model: Option<String>,
        _max_tasks: usize,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let agent_info = AgentInfo {
            id: id.clone(),
            name,
            agent_type,
            status: "running".to_string(),
            model,
            capabilities,
            started_at: Utc::now().to_rfc3339(),
            metrics: Some(AgentMetrics {
                tasks_completed: 0,
                tasks_failed: 0,
                errors: 0,
                avg_response_time_ms: 0.0,
                memory_usage_mb: 0,
                cpu_usage_percent: 0.0,
            }),
        };

        let mut agents = self.agents.write().await;
        agents.insert(id.clone(), agent_info);

        Ok(id)
    }

    pub async fn stop_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;
        Ok(())
    }

    pub async fn force_stop_agent(&self, agent_id: &str) -> Result<()> {
        self.stop_agent(agent_id).await
    }

    pub async fn list_agents(&self, agent_type: Option<AgentType>) -> Result<Vec<AgentInfo>> {
        let agents = self.agents.read().await;
        let mut result: Vec<AgentInfo> = agents.values().cloned().collect();

        if let Some(filter_type) = agent_type {
            result.retain(|a| a.agent_type == filter_type);
        }

        Ok(result)
    }

    pub async fn get_agent_info(&self, agent_id: &str) -> Result<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(agent_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))
    }

    pub async fn pause_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = "paused".to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found: {}", agent_id))
        }
    }

    pub async fn resume_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = "running".to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Agent not found: {}", agent_id))
        }
    }

    pub async fn get_agent_logs(&self, agent_id: &str, lines: usize) -> Result<Vec<String>> {
        // Verify agent exists
        let agents = self.agents.read().await;
        if !agents.contains_key(agent_id) {
            return Err(anyhow::anyhow!("Agent not found: {}", agent_id));
        }
        drop(agents);

        // Get the log file path from the config
        let config = crate::commands::config::AxonConfig::load()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        let log_file = config.logs_dir().join(format!("{}.log", agent_id));

        // Check if log file exists
        if !log_file.exists() {
            return Ok(vec!["No logs available yet".to_string()]);
        }

        // Read the log file
        let content = std::fs::read_to_string(&log_file)
            .map_err(|e| anyhow::anyhow!("Failed to read log file: {}", e))?;

        // Get the last N lines
        let log_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let start = log_lines.len().saturating_sub(lines);
        Ok(log_lines[start..].to_vec())
    }

    pub async fn stream_agent_logs(&self, agent_id: &str) -> Result<mpsc::Receiver<String>> {
        // Verify agent exists
        let agents = self.agents.read().await;
        if !agents.contains_key(agent_id) {
            return Err(anyhow::anyhow!("Agent not found: {}", agent_id));
        }
        drop(agents);

        // Get the log file path from the config
        let config = crate::commands::config::AxonConfig::load()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        let log_file = config.logs_dir().join(format!("{}.log", agent_id));

        let (tx, rx) = mpsc::channel(100);

        // Spawn a task to watch the log file and stream new lines
        tokio::spawn(async move {
            use std::fs::File;
            use std::io::{BufRead, BufReader, Seek, SeekFrom};
            use tokio::time::{sleep, Duration};

            // Wait for log file to exist if it doesn't
            while !log_file.exists() {
                if tx.is_closed() {
                    return;
                }
                sleep(Duration::from_millis(500)).await;
            }

            // Open the file and seek to the end
            let mut file = match File::open(&log_file) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(format!("Error opening log file: {}", e)).await;
                    return;
                }
            };

            // Seek to end of file to start streaming from now
            if let Err(e) = file.seek(SeekFrom::End(0)) {
                let _ = tx.send(format!("Error seeking log file: {}", e)).await;
                return;
            }

            let mut reader = BufReader::new(file);
            let mut line = String::new();

            loop {
                // Check if the receiver is still active
                if tx.is_closed() {
                    break;
                }

                // Try to read a line
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        // No new data, wait a bit and try again
                        sleep(Duration::from_millis(100)).await;
                    }
                    Ok(_) => {
                        // Send the line (trim the trailing newline)
                        if let Err(_) = tx.send(line.trim_end().to_string()).await {
                            break;
                        }
                        line.clear();
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Error reading log file: {}", e)).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    pub async fn run_workflow(
        &self,
        _workflow_content: String,
        _input_data: serde_json::Value,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let workflow_info = WorkflowInfo {
            id: id.clone(),
            name: "Workflow".to_string(),
            status: "running".to_string(),
            started_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };

        let mut workflows = self.workflows.write().await;
        workflows.insert(id.clone(), workflow_info);

        Ok(id)
    }

    pub async fn list_workflows(&self, status: Option<String>) -> Result<Vec<WorkflowInfo>> {
        let workflows = self.workflows.read().await;
        let mut result: Vec<WorkflowInfo> = workflows.values().cloned().collect();

        if let Some(filter_status) = status {
            result.retain(|w| w.status == filter_status);
        }

        Ok(result)
    }

    pub async fn get_workflow_status(&self, workflow_id: &str) -> Result<WorkflowStatus> {
        let workflows = self.workflows.read().await;
        let workflow = workflows.get(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;

        Ok(WorkflowStatus {
            id: workflow.id.clone(),
            name: workflow.name.clone(),
            status: workflow.status.clone(),
            started_at: workflow.started_at.clone(),
            completed_at: workflow.completed_at.clone(),
            tasks_completed: 0,
            total_tasks: 1,
            current_tasks: Vec::new(),
            error: None,
        })
    }

    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        if let Some(workflow) = workflows.get_mut(workflow_id) {
            workflow.status = "cancelled".to_string();
            workflow.completed_at = Some(Utc::now().to_rfc3339());
            Ok(())
        } else {
            Err(anyhow::anyhow!("Workflow not found: {}", workflow_id))
        }
    }

    pub async fn get_agent_metrics(&self, agent_id: &str) -> Result<HashMap<String, AgentMetrics>> {
        let agents = self.agents.read().await;
        let agent = agents.get(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent not found: {}", agent_id))?;

        let mut result = HashMap::new();
        if let Some(metrics) = &agent.metrics {
            result.insert(agent_id.to_string(), metrics.clone());
        }

        Ok(result)
    }

    pub async fn get_all_metrics(&self) -> Result<HashMap<String, AgentMetrics>> {
        let agents = self.agents.read().await;
        let mut result = HashMap::new();

        for (id, agent) in agents.iter() {
            if let Some(metrics) = &agent.metrics {
                result.insert(id.clone(), metrics.clone());
            }
        }

        Ok(result)
    }

    pub async fn get_telemetry(&self, _range: u64) -> Result<TelemetryData> {
        let agents = self.agents.read().await;
        let workflows = self.workflows.read().await;

        Ok(TelemetryData {
            request_rate: 0.0,
            error_rate: 0.0,
            avg_latency_ms: 0.0,
            active_agents: agents.len(),
            active_workflows: workflows.len(),
            top_errors: Vec::new(),
        })
    }
}
