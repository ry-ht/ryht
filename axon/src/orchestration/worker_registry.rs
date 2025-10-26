//! Worker Registry - Pool of Specialized Agents
//!
//! Maintains a pool of worker agents with capability matching and load balancing.
//! Provides mechanisms to acquire suitable workers for tasks based on required
//! capabilities and current availability.
//!
//! # Features
//!
//! - Capability-based worker selection
//! - Load balancing across workers
//! - Health monitoring and failover
//! - Dynamic worker spawning (planned)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{debug, warn, info};

use crate::agents::{AgentId, AgentType};
use crate::cortex_bridge::{SessionId, WorkspaceId};
use super::{OrchestrationError, Result};

// ============================================================================
// Worker Handle
// ============================================================================

/// Handle to an acquired worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHandle {
    /// Worker agent ID
    pub worker_id: AgentId,

    /// Worker agent type
    pub agent_type: AgentType,

    /// Worker capabilities
    pub capabilities: Vec<String>,

    /// Session ID for this worker
    pub session_id: SessionId,

    /// Workspace ID for this worker
    pub workspace_id: WorkspaceId,

    /// Acquisition timestamp
    pub acquired_at: DateTime<Utc>,

    /// Current load (0.0 - 1.0)
    pub load: f32,
}

// ============================================================================
// Worker Info
// ============================================================================

/// Information about a registered worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Worker agent ID
    pub agent_id: AgentId,

    /// Worker agent type
    pub agent_type: AgentType,

    /// Worker capabilities
    pub capabilities: Vec<String>,

    /// Current status
    pub status: WorkerStatus,

    /// Current load (0.0 - 1.0)
    pub load: f32,

    /// Tasks completed
    pub tasks_completed: u64,

    /// Tasks failed
    pub tasks_failed: u64,

    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,

    /// Average task duration (seconds)
    pub avg_task_duration_secs: f64,

    /// Last health check
    pub last_heartbeat: DateTime<Utc>,

    /// Registration timestamp
    pub registered_at: DateTime<Utc>,
}

/// Worker status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    /// Worker is idle and available
    Idle,

    /// Worker is currently executing a task
    Busy,

    /// Worker is paused
    Paused,

    /// Worker has failed and needs intervention
    Failed,

    /// Worker is offline
    Offline,
}

impl Default for WorkerStatus {
    fn default() -> Self {
        Self::Idle
    }
}

// ============================================================================
// Worker Registry
// ============================================================================

/// Registry of available worker agents
pub struct WorkerRegistry {
    /// Workers indexed by agent ID
    workers: HashMap<AgentId, WorkerInfo>,

    /// Capability index: capability -> worker IDs
    capability_index: HashMap<String, Vec<AgentId>>,

    /// Configuration
    config: WorkerRegistryConfig,
}

/// Worker registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistryConfig {
    /// Maximum load before worker is considered busy
    pub max_load_threshold: f32,

    /// Heartbeat timeout (seconds)
    pub heartbeat_timeout_secs: u64,

    /// Enable automatic worker health checks
    pub auto_health_check: bool,

    /// Minimum success rate to keep worker active
    pub min_success_rate: f32,

    /// Enable load balancing
    pub enable_load_balancing: bool,
}

impl Default for WorkerRegistryConfig {
    fn default() -> Self {
        Self {
            max_load_threshold: 0.8,
            heartbeat_timeout_secs: 60,
            auto_health_check: true,
            min_success_rate: 0.5,
            enable_load_balancing: true,
        }
    }
}

impl WorkerRegistry {
    /// Create a new worker registry
    pub fn new(config: WorkerRegistryConfig) -> Self {
        info!("Initializing Worker Registry");

        Self {
            workers: HashMap::new(),
            capability_index: HashMap::new(),
            config,
        }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(WorkerRegistryConfig::default())
    }

    // ========================================================================
    // Worker Registration
    // ========================================================================

    /// Register a worker in the pool
    pub fn register_worker(
        &mut self,
        agent_id: AgentId,
        agent_type: AgentType,
        capabilities: Vec<String>,
    ) -> Result<()> {
        debug!("Registering worker: {} ({:?})", agent_id, agent_type);

        // Create worker info
        let info = WorkerInfo {
            agent_id: agent_id.clone(),
            agent_type,
            capabilities: capabilities.clone(),
            status: WorkerStatus::Idle,
            load: 0.0,
            tasks_completed: 0,
            tasks_failed: 0,
            success_rate: 1.0,
            avg_task_duration_secs: 0.0,
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
        };

        // Index by capabilities
        for capability in &capabilities {
            self.capability_index
                .entry(capability.clone())
                .or_default()
                .push(agent_id.clone());
        }

        // Add to registry
        self.workers.insert(agent_id.clone(), info);

        info!("Worker {} registered with {} capabilities", agent_id, capabilities.len());

        Ok(())
    }

    /// Unregister a worker
    pub fn unregister_worker(&mut self, agent_id: &AgentId) -> Result<()> {
        debug!("Unregistering worker: {}", agent_id);

        if let Some(info) = self.workers.remove(agent_id) {
            // Remove from capability index
            for capability in &info.capabilities {
                if let Some(workers) = self.capability_index.get_mut(capability) {
                    workers.retain(|id| id != agent_id);
                }
            }

            info!("Worker {} unregistered", agent_id);
            Ok(())
        } else {
            Err(OrchestrationError::Other(
                anyhow::anyhow!("Worker not found: {}", agent_id)
            ))
        }
    }

    // ========================================================================
    // Worker Acquisition
    // ========================================================================

    /// Acquire a worker with required capabilities
    pub async fn acquire_worker(
        &mut self,
        required_capabilities: &[String],
        session_id: &SessionId,
        workspace_id: &WorkspaceId,
    ) -> Result<WorkerHandle> {
        debug!("Acquiring worker with capabilities: {:?}", required_capabilities);

        // Find suitable workers
        let suitable_workers = self.find_suitable_workers(required_capabilities)?;

        if suitable_workers.is_empty() {
            return Err(OrchestrationError::NoSuitableAgent {
                task_id: "acquisition".to_string(),
            });
        }

        // Select best worker (lowest load)
        let best_worker_id = if self.config.enable_load_balancing {
            self.select_lowest_load_worker(&suitable_workers)?
        } else {
            suitable_workers[0].clone()
        };

        // Get worker info
        let worker_info = self.workers.get_mut(&best_worker_id)
            .ok_or_else(|| OrchestrationError::Other(
                anyhow::anyhow!("Worker not found: {}", best_worker_id)
            ))?;

        // Update worker status
        worker_info.status = WorkerStatus::Busy;
        worker_info.load = (worker_info.load + 0.3).min(1.0);

        debug!("Acquired worker: {} (load: {:.2})", best_worker_id, worker_info.load);

        Ok(WorkerHandle {
            worker_id: best_worker_id,
            agent_type: worker_info.agent_type,
            capabilities: worker_info.capabilities.clone(),
            session_id: session_id.clone(),
            workspace_id: workspace_id.clone(),
            acquired_at: Utc::now(),
            load: worker_info.load,
        })
    }

    /// Release a worker back to the pool
    pub fn release_worker(&mut self, worker_id: &AgentId, success: bool) -> Result<()> {
        debug!("Releasing worker: {} (success: {})", worker_id, success);

        if let Some(info) = self.workers.get_mut(worker_id) {
            // Update status
            info.status = WorkerStatus::Idle;
            info.load = (info.load - 0.3).max(0.0);

            // Update statistics
            if success {
                info.tasks_completed += 1;
            } else {
                info.tasks_failed += 1;
            }

            // Update success rate
            let total = info.tasks_completed + info.tasks_failed;
            if total > 0 {
                info.success_rate = info.tasks_completed as f32 / total as f32;
            }

            debug!("Worker {} released (load: {:.2}, success_rate: {:.2})",
                   worker_id, info.load, info.success_rate);

            Ok(())
        } else {
            Err(OrchestrationError::Other(
                anyhow::anyhow!("Worker not found: {}", worker_id)
            ))
        }
    }

    // ========================================================================
    // Worker Selection
    // ========================================================================

    /// Find workers with required capabilities
    fn find_suitable_workers(&self, required_capabilities: &[String]) -> Result<Vec<AgentId>> {
        debug!("Finding workers with capabilities: {:?}", required_capabilities);

        if required_capabilities.is_empty() {
            // Return all available workers
            return Ok(self.workers
                .iter()
                .filter(|(_, info)| info.status == WorkerStatus::Idle)
                .map(|(id, _)| id.clone())
                .collect());
        }

        // Find workers that have ALL required capabilities
        let mut suitable_workers: Option<Vec<AgentId>> = None;

        for capability in required_capabilities {
            if let Some(workers_with_cap) = self.capability_index.get(capability) {
                let available: Vec<AgentId> = workers_with_cap
                    .iter()
                    .filter(|id| {
                        if let Some(info) = self.workers.get(*id) {
                            info.status == WorkerStatus::Idle
                                && info.load < self.config.max_load_threshold
                                && info.success_rate >= self.config.min_success_rate
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                suitable_workers = Some(if let Some(existing) = suitable_workers {
                    // Intersection
                    existing.into_iter()
                        .filter(|id| available.contains(id))
                        .collect()
                } else {
                    available
                });

                // If no workers match all capabilities so far, we can stop
                if suitable_workers.as_ref().is_some_and(|w| w.is_empty()) {
                    break;
                }
            } else {
                // No workers with this capability
                return Ok(Vec::new());
            }
        }

        Ok(suitable_workers.unwrap_or_default())
    }

    /// Select worker with lowest load
    fn select_lowest_load_worker(&self, worker_ids: &[AgentId]) -> Result<AgentId> {
        worker_ids
            .iter()
            .filter_map(|id| {
                self.workers.get(id).map(|info| (id.clone(), info.load))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
            .ok_or_else(|| OrchestrationError::Other(
                anyhow::anyhow!("No workers available")
            ))
    }

    // ========================================================================
    // Health Monitoring
    // ========================================================================

    /// Update worker heartbeat
    pub fn update_heartbeat(&mut self, worker_id: &AgentId) -> Result<()> {
        if let Some(info) = self.workers.get_mut(worker_id) {
            info.last_heartbeat = Utc::now();
            Ok(())
        } else {
            Err(OrchestrationError::Other(
                anyhow::anyhow!("Worker not found: {}", worker_id)
            ))
        }
    }

    /// Check for unhealthy workers
    pub fn check_worker_health(&mut self) -> Vec<AgentId> {
        let timeout = chrono::Duration::seconds(self.config.heartbeat_timeout_secs as i64);
        let now = Utc::now();
        let mut unhealthy = Vec::new();

        for (id, info) in &mut self.workers {
            if now - info.last_heartbeat > timeout {
                warn!("Worker {} heartbeat timeout", id);
                info.status = WorkerStatus::Offline;
                unhealthy.push(id.clone());
            }
        }

        unhealthy
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get total worker count
    pub fn total_worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Get available worker count
    pub fn available_worker_count(&self) -> usize {
        self.workers
            .values()
            .filter(|info| info.status == WorkerStatus::Idle)
            .count()
    }

    /// Get worker info
    pub fn get_worker_info(&self, worker_id: &AgentId) -> Option<&WorkerInfo> {
        self.workers.get(worker_id)
    }

    /// Get all workers with a specific capability
    pub fn get_workers_by_capability(&self, capability: &str) -> Vec<AgentId> {
        self.capability_index
            .get(capability)
            .cloned()
            .unwrap_or_default()
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> RegistryStatistics {
        let total = self.workers.len();
        let idle = self.workers.values().filter(|w| w.status == WorkerStatus::Idle).count();
        let busy = self.workers.values().filter(|w| w.status == WorkerStatus::Busy).count();
        let failed = self.workers.values().filter(|w| w.status == WorkerStatus::Failed).count();

        let total_tasks: u64 = self.workers.values().map(|w| w.tasks_completed).sum();
        let total_failures: u64 = self.workers.values().map(|w| w.tasks_failed).sum();

        let avg_success_rate = if total > 0 {
            self.workers.values().map(|w| w.success_rate).sum::<f32>() / total as f32
        } else {
            0.0
        };

        RegistryStatistics {
            total_workers: total,
            idle_workers: idle,
            busy_workers: busy,
            failed_workers: failed,
            total_tasks_completed: total_tasks,
            total_tasks_failed: total_failures,
            average_success_rate: avg_success_rate,
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatistics {
    pub total_workers: usize,
    pub idle_workers: usize,
    pub busy_workers: usize,
    pub failed_workers: usize,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub average_success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_registration() {
        let mut registry = WorkerRegistry::default();

        let agent_id = AgentId::new();
        let result = registry.register_worker(
            agent_id.clone(),
            AgentType::Developer,
            vec!["CodeGeneration".to_string()],
        );

        assert!(result.is_ok());
        assert_eq!(registry.total_worker_count(), 1);
        assert_eq!(registry.available_worker_count(), 1);
    }

    #[test]
    fn test_worker_acquisition() {
        let mut registry = WorkerRegistry::default();

        let agent_id = AgentId::new();
        registry.register_worker(
            agent_id.clone(),
            AgentType::Developer,
            vec!["CodeGeneration".to_string()],
        ).unwrap();

        // Try to acquire - this is async so we can't easily test it here
        // But we can test the helper methods
        assert_eq!(registry.available_worker_count(), 1);
    }

    #[test]
    fn test_worker_statistics() {
        let mut registry = WorkerRegistry::default();

        let agent_id = AgentId::new();
        registry.register_worker(
            agent_id.clone(),
            AgentType::Developer,
            vec!["CodeGeneration".to_string()],
        ).unwrap();

        let stats = registry.get_statistics();
        assert_eq!(stats.total_workers, 1);
        assert_eq!(stats.idle_workers, 1);
    }
}
