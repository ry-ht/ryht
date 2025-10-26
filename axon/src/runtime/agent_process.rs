//! Agent Process Management
//!
//! This module handles spawning, monitoring, and managing individual agent processes.
//! Each agent runs in its own process for isolation and resource management.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{error, info, warn};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::agents::AgentId;
use super::runtime_config::{ProcessConfig, ResourceLimits};

/// Result type for agent process operations
pub type Result<T> = std::result::Result<T, AgentProcessError>;

/// Agent process errors
#[derive(Debug, thiserror::Error)]
pub enum AgentProcessError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Process timeout: {0}")]
    Timeout(String),

    #[error("Process crashed: {0}")]
    Crashed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process not found: {0}")]
    NotFound(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Agent process handle
pub struct AgentProcess {
    /// Process ID
    pub pid: u32,

    /// Agent ID
    pub agent_id: AgentId,

    /// Process handle
    process: Option<Child>,

    /// Process state
    state: ProcessState,

    /// Spawn time
    spawn_time: Instant,

    /// Resource usage
    resources: ResourceUsage,

    /// Output channel
    stdout_tx: mpsc::UnboundedSender<String>,
    stderr_tx: mpsc::UnboundedSender<String>,

    /// Process metadata
    metadata: ProcessMetadata,
}

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// Process is starting
    Starting,

    /// Process is running
    Running,

    /// Process is being terminated
    Terminating,

    /// Process has exited normally
    Exited,

    /// Process has crashed
    Crashed,

    /// Process was killed
    Killed,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current memory usage (bytes)
    pub memory_bytes: u64,

    /// Peak memory usage (bytes)
    pub peak_memory_bytes: u64,

    /// CPU time (milliseconds)
    pub cpu_time_ms: u64,

    /// Tool calls made
    pub tool_calls: u64,

    /// Tasks executed
    pub tasks_executed: u64,

    /// Last updated
    pub last_updated: DateTime<Utc>,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_bytes: 0,
            peak_memory_bytes: 0,
            cpu_time_ms: 0,
            tool_calls: 0,
            tasks_executed: 0,
            last_updated: Utc::now(),
        }
    }
}

/// Process metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetadata {
    /// Agent name
    pub agent_name: String,

    /// Working directory
    pub working_directory: String,

    /// Command line
    pub command_line: String,

    /// Environment variables
    pub environment: Vec<(String, String)>,

    /// Spawn timestamp
    pub spawned_at: DateTime<Utc>,

    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
}

impl AgentProcess {
    /// Spawn a new agent process
    pub fn spawn(
        agent_id: AgentId,
        agent_name: String,
        command: &str,
        args: &[String],
        config: &ProcessConfig,
    ) -> Result<Self> {
        info!("Spawning agent process: {} ({})", agent_name, agent_id);

        // Build command
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set working directory
        if let Some(ref working_dir) = config.working_directory {
            cmd.current_dir(working_dir);
        }

        // Set environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Spawn process
        let mut child = cmd.spawn()
            .map_err(|e| AgentProcessError::SpawnFailed(e.to_string()))?;

        let pid = child.id();

        info!("Agent process spawned: {} with PID {}", agent_name, pid);

        // Create output channels
        let (stdout_tx, _) = mpsc::unbounded_channel();
        let (stderr_tx, _) = mpsc::unbounded_channel();

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = stdout_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let _ = tx.send(line);
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = stderr_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let _ = tx.send(line);
                }
            });
        }

        let metadata = ProcessMetadata {
            agent_name: agent_name.clone(),
            working_directory: config.working_directory
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| ".".to_string()),
            command_line: format!("{} {}", command, args.join(" ")),
            environment: config.environment.clone(),
            spawned_at: Utc::now(),
            last_heartbeat: Utc::now(),
        };

        Ok(Self {
            pid,
            agent_id,
            process: Some(child),
            state: ProcessState::Starting,
            spawn_time: Instant::now(),
            resources: ResourceUsage::default(),
            stdout_tx,
            stderr_tx,
            metadata,
        })
    }

    /// Check if process is alive
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.state = ProcessState::Exited;
                    false
                }
                Ok(None) => {
                    self.state = ProcessState::Running;
                    true
                }
                Err(_) => {
                    self.state = ProcessState::Crashed;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get process uptime
    pub fn uptime(&self) -> Duration {
        self.spawn_time.elapsed()
    }

    /// Update resource usage
    pub fn update_resources(&mut self, usage: ResourceUsage) {
        self.resources = usage;
    }

    /// Update heartbeat
    pub fn update_heartbeat(&mut self) {
        self.metadata.last_heartbeat = Utc::now();
    }

    /// Check resource limits
    pub fn check_limits(&self, limits: &ResourceLimits) -> Result<()> {
        // Check memory limit
        if let Some(max_memory) = limits.max_memory_bytes {
            if self.resources.memory_bytes > max_memory {
                return Err(AgentProcessError::ResourceLimitExceeded(
                    format!("Memory limit exceeded: {} > {}",
                            self.resources.memory_bytes, max_memory)
                ));
            }
        }

        // Check task duration
        if self.uptime() > limits.max_task_duration {
            return Err(AgentProcessError::ResourceLimitExceeded(
                format!("Task duration exceeded: {:?} > {:?}",
                        self.uptime(), limits.max_task_duration)
            ));
        }

        Ok(())
    }

    /// Gracefully terminate the process
    pub fn terminate(&mut self, grace_period: Duration) -> Result<()> {
        info!("Terminating agent process: {} (PID {})", self.agent_id, self.pid);

        self.state = ProcessState::Terminating;

        if let Some(mut child) = self.process.take() {
            // Try graceful shutdown first
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(self.pid as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }

            // Wait for grace period
            std::thread::sleep(grace_period);

            // Force kill if still alive
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.state = ProcessState::Exited;
                    info!("Process {} exited gracefully", self.pid);
                }
                Ok(None) => {
                    warn!("Process {} did not exit gracefully, killing", self.pid);
                    let _ = child.kill();
                    self.state = ProcessState::Killed;
                }
                Err(e) => {
                    error!("Error checking process status: {}", e);
                    return Err(AgentProcessError::Io(e));
                }
            }
        }

        Ok(())
    }

    /// Force kill the process
    pub fn kill(&mut self) -> Result<()> {
        info!("Killing agent process: {} (PID {})", self.agent_id, self.pid);

        if let Some(mut child) = self.process.take() {
            child.kill()
                .map_err(AgentProcessError::Io)?;
            self.state = ProcessState::Killed;
        }

        Ok(())
    }

    /// Get process state
    pub fn state(&self) -> ProcessState {
        self.state
    }

    /// Get resource usage
    pub fn resources(&self) -> &ResourceUsage {
        &self.resources
    }

    /// Get metadata
    pub fn metadata(&self) -> &ProcessMetadata {
        &self.metadata
    }

    /// Get stdout receiver
    pub fn subscribe_stdout(&self) -> mpsc::UnboundedReceiver<String> {
        let (_tx, rx) = mpsc::unbounded_channel();
        rx
    }

    /// Get stderr receiver
    pub fn subscribe_stderr(&self) -> mpsc::UnboundedReceiver<String> {
        let (_tx, rx) = mpsc::unbounded_channel();
        rx
    }
}

impl Drop for AgentProcess {
    fn drop(&mut self) {
        // Ensure process is cleaned up
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
    }
}

/// Process manager for tracking all agent processes
pub struct ProcessManager {
    /// Active processes
    processes: Arc<RwLock<HashMap<AgentId, AgentProcess>>>,

    /// Configuration
    config: ProcessConfig,

    /// Resource limits
    limits: ResourceLimits,
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new(config: ProcessConfig, limits: ResourceLimits) -> Self {
        info!("Initializing Process Manager");

        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            config,
            limits,
        }
    }

    /// Spawn a new agent process
    pub async fn spawn(
        &self,
        agent_id: AgentId,
        agent_name: String,
        command: &str,
        args: &[String],
    ) -> Result<()> {
        // Check concurrent process limit
        let process_count = self.processes.read().await.len();
        if process_count >= self.config.max_concurrent_processes {
            return Err(AgentProcessError::ResourceLimitExceeded(
                format!("Max concurrent processes reached: {}",
                        self.config.max_concurrent_processes)
            ));
        }

        // Spawn process with timeout
        let process = match timeout(
            self.config.spawn_timeout,
            tokio::task::spawn_blocking({
                let agent_id = agent_id.clone();
                let agent_name = agent_name.clone();
                let command = command.to_string();
                let args = args.to_vec();
                let config = self.config.clone();
                move || {
                    AgentProcess::spawn(
                        agent_id,
                        agent_name,
                        &command,
                        &args,
                        &config,
                    )
                }
            })
        ).await {
            Ok(Ok(Ok(process))) => process,
            Ok(Ok(Err(e))) => return Err(e),
            Ok(Err(e)) => return Err(AgentProcessError::SpawnFailed(e.to_string())),
            Err(_) => return Err(AgentProcessError::Timeout("Spawn timeout".to_string())),
        };

        // Store process
        self.processes.write().await.insert(agent_id, process);

        Ok(())
    }

    /// Get process handle
    pub async fn get(&self, agent_id: &AgentId) -> Option<AgentId> {
        self.processes.read().await.get(agent_id).map(|p| p.agent_id.clone())
    }

    /// Check if process is alive
    pub async fn is_alive(&self, agent_id: &AgentId) -> bool {
        if let Some(process) = self.processes.write().await.get_mut(agent_id) {
            process.is_alive()
        } else {
            false
        }
    }

    /// Update heartbeat
    pub async fn update_heartbeat(&self, agent_id: &AgentId) -> Result<()> {
        if let Some(process) = self.processes.write().await.get_mut(agent_id) {
            process.update_heartbeat();
            Ok(())
        } else {
            Err(AgentProcessError::NotFound(agent_id.to_string()))
        }
    }

    /// Terminate process
    pub async fn terminate(&self, agent_id: &AgentId) -> Result<()> {
        if let Some(mut process) = self.processes.write().await.remove(agent_id) {
            process.terminate(self.config.shutdown_grace_period)
        } else {
            Err(AgentProcessError::NotFound(agent_id.to_string()))
        }
    }

    /// Kill process
    pub async fn kill(&self, agent_id: &AgentId) -> Result<()> {
        if let Some(mut process) = self.processes.write().await.remove(agent_id) {
            process.kill()
        } else {
            Err(AgentProcessError::NotFound(agent_id.to_string()))
        }
    }

    /// Get all active process IDs
    pub async fn active_processes(&self) -> Vec<AgentId> {
        self.processes.read().await.keys().cloned().collect()
    }

    /// Cleanup dead processes
    pub async fn cleanup_dead_processes(&self) {
        let mut processes = self.processes.write().await;
        let mut dead_processes = Vec::new();

        for (agent_id, process) in processes.iter_mut() {
            if !process.is_alive() {
                dead_processes.push(agent_id.clone());
            }
        }

        for agent_id in dead_processes {
            info!("Removing dead process: {}", agent_id);
            processes.remove(&agent_id);
        }
    }

    /// Get process statistics
    pub async fn get_statistics(&self) -> ProcessManagerStatistics {
        let processes = self.processes.read().await;

        let total_memory: u64 = processes.values().map(|p| p.resources.memory_bytes).sum();
        let total_cpu: u64 = processes.values().map(|p| p.resources.cpu_time_ms).sum();
        let total_tool_calls: u64 = processes.values().map(|p| p.resources.tool_calls).sum();

        ProcessManagerStatistics {
            active_processes: processes.len(),
            total_memory_bytes: total_memory,
            total_cpu_time_ms: total_cpu,
            total_tool_calls,
        }
    }
}

/// Process manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessManagerStatistics {
    pub active_processes: usize,
    pub total_memory_bytes: u64,
    pub total_cpu_time_ms: u64,
    pub total_tool_calls: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_state_transitions() {
        let state = ProcessState::Starting;
        assert_eq!(state, ProcessState::Starting);
    }

    #[test]
    fn test_resource_usage_default() {
        let usage = ResourceUsage::default();
        assert_eq!(usage.memory_bytes, 0);
        assert_eq!(usage.tool_calls, 0);
    }
}
