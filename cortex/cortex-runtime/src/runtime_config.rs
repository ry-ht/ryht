//! Runtime Configuration for Agent Execution
//!
//! This module defines configuration structures for the agent runtime system,
//! including process limits, resource constraints, and execution parameters.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Runtime configuration for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct RuntimeConfig {
    /// Process configuration
    pub process: ProcessConfig,

    /// Resource limits
    pub resources: ResourceLimits,

    /// MCP integration settings
    pub mcp: McpConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,

    /// Recovery settings
    pub recovery: RecoveryConfig,
}


/// Process configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    /// Maximum number of concurrent agent processes
    pub max_concurrent_processes: usize,

    /// Process spawn timeout
    pub spawn_timeout: Duration,

    /// Process shutdown grace period
    pub shutdown_grace_period: Duration,

    /// Enable process isolation
    pub enable_isolation: bool,

    /// Working directory for agent processes
    pub working_directory: Option<PathBuf>,

    /// Environment variables to pass to processes
    pub environment: Vec<(String, String)>,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            max_concurrent_processes: 10,
            spawn_timeout: Duration::from_secs(30),
            shutdown_grace_period: Duration::from_secs(10),
            enable_isolation: true,
            working_directory: None,
            environment: Vec::new(),
        }
    }
}

/// Resource limits for agent processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory per process (bytes)
    pub max_memory_bytes: Option<u64>,

    /// CPU limit (percentage, 0.0 - 100.0)
    pub cpu_limit_percent: Option<f32>,

    /// Maximum file descriptors
    pub max_file_descriptors: Option<u32>,

    /// Maximum execution time per task
    pub max_task_duration: Duration,

    /// Maximum tool calls per task
    pub max_tool_calls_per_task: usize,

    /// Maximum output size (bytes)
    pub max_output_size_bytes: usize,

    /// Enable resource tracking
    pub enable_resource_tracking: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
            cpu_limit_percent: Some(80.0),
            max_file_descriptors: Some(1024),
            max_task_duration: Duration::from_secs(300),
            max_tool_calls_per_task: 50,
            max_output_size_bytes: 10 * 1024 * 1024, // 10MB
            enable_resource_tracking: true,
        }
    }
}

/// MCP integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Path to cortex binary
    pub cortex_binary_path: Option<PathBuf>,

    /// Cortex arguments
    pub cortex_args: Vec<String>,

    /// MCP protocol version
    pub protocol_version: String,

    /// Request timeout
    pub request_timeout: Duration,

    /// Maximum retries for failed requests
    pub max_retries: u32,

    /// Enable MCP logging
    pub enable_mcp_logging: bool,

    /// MCP server port range (for stdio mode)
    pub stdio_buffer_size: usize,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            cortex_binary_path: None, // Will auto-discover
            cortex_args: vec!["mcp".to_string(), "stdio".to_string()],
            protocol_version: "2024-11-05".to_string(),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_mcp_logging: true,
            stdio_buffer_size: 8192,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Health check interval
    pub health_check_interval: Duration,

    /// Heartbeat timeout
    pub heartbeat_timeout: Duration,

    /// Enable performance metrics
    pub enable_metrics: bool,

    /// Metrics collection interval
    pub metrics_interval: Duration,

    /// Enable log aggregation
    pub enable_log_aggregation: bool,

    /// Log level for agent processes
    pub agent_log_level: String,

    /// Maximum log buffer size
    pub max_log_buffer_size: usize,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(10),
            heartbeat_timeout: Duration::from_secs(30),
            enable_metrics: true,
            metrics_interval: Duration::from_secs(5),
            enable_log_aggregation: true,
            agent_log_level: "info".to_string(),
            max_log_buffer_size: 1024 * 1024, // 1MB
        }
    }
}

/// Recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Enable automatic restart on failure
    pub enable_auto_restart: bool,

    /// Maximum restart attempts
    pub max_restart_attempts: u32,

    /// Restart backoff (exponential)
    pub restart_backoff_base: Duration,

    /// Enable checkpoint/resume
    pub enable_checkpointing: bool,

    /// Checkpoint interval
    pub checkpoint_interval: Duration,

    /// Enable graceful degradation
    pub enable_graceful_degradation: bool,

    /// Failure threshold before degradation
    pub failure_threshold: u32,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enable_auto_restart: true,
            max_restart_attempts: 3,
            restart_backoff_base: Duration::from_secs(5),
            enable_checkpointing: false, // Disabled by default
            checkpoint_interval: Duration::from_secs(60),
            enable_graceful_degradation: true,
            failure_threshold: 3,
        }
    }
}

/// Agent process execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Local process execution
    Local,

    /// Containerized execution (future)
    Container,

    /// Remote execution (future)
    Remote,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Local
    }
}

/// Agent runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    /// Total processes spawned
    pub total_processes_spawned: u64,

    /// Currently active processes
    pub active_processes: usize,

    /// Total tasks executed
    pub total_tasks_executed: u64,

    /// Total tasks failed
    pub total_tasks_failed: u64,

    /// Average task duration (milliseconds)
    pub avg_task_duration_ms: u64,

    /// Total memory used (bytes)
    pub total_memory_bytes: u64,

    /// Total CPU time (seconds)
    pub total_cpu_seconds: u64,

    /// Total tool calls made
    pub total_tool_calls: u64,

    /// Average response time (milliseconds)
    pub avg_response_time_ms: u64,

    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,
}

impl Default for RuntimeStatistics {
    fn default() -> Self {
        Self {
            total_processes_spawned: 0,
            active_processes: 0,
            total_tasks_executed: 0,
            total_tasks_failed: 0,
            avg_task_duration_ms: 0,
            total_memory_bytes: 0,
            total_cpu_seconds: 0,
            total_tool_calls: 0,
            avg_response_time_ms: 0,
            success_rate: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_runtime_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.process.max_concurrent_processes, 10);
        assert_eq!(config.resources.max_tool_calls_per_task, 50);
        assert!(config.monitoring.enable_metrics);
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits::default();
        assert!(limits.max_memory_bytes.is_some());
        assert_eq!(limits.max_tool_calls_per_task, 50);
    }

    #[test]
    fn test_mcp_config() {
        let mcp = McpConfig::default();
        assert_eq!(mcp.protocol_version, "2024-11-05");
        assert_eq!(mcp.max_retries, 3);
    }
}
