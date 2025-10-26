//! Agent Lifecycle Management
//!
//! Compile-time state machine for agent lifecycle with type-safe state transitions.

use super::{AgentId, AgentMetrics};

/// Sealed trait to prevent external state implementations
mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Idle {}
    impl Sealed for super::Assigned {}
    impl Sealed for super::Working {}
    impl Sealed for super::Completed {}
    impl Sealed for super::Failed {}
}

/// Base trait for agent states
pub trait AgentState: sealed::Sealed + Send + Sync {}

/// Agent is idle and available
pub struct Idle;
impl AgentState for Idle {}

/// Agent has been assigned a task
pub struct Assigned {
    pub task_id: String,
}
impl AgentState for Assigned {}

/// Agent is actively working on a task
pub struct Working {
    pub task_id: String,
    pub progress: f32,
}
impl AgentState for Working {}

/// Agent has completed a task
pub struct Completed {
    pub result: String,
}
impl AgentState for Completed {}

/// Agent task has failed
pub struct Failed {
    pub error: String,
}
impl AgentState for Failed {}

/// Core agent structure with compile-time state
pub struct AgentInstance<S: AgentState> {
    pub id: AgentId,
    pub name: String,
    pub state: S,
    pub metrics: AgentMetrics,
}

// State transitions - enforced at compile time

impl AgentInstance<Idle> {
    /// Create a new idle agent
    pub fn new(name: String) -> Self {
        Self {
            id: AgentId::new(),
            name,
            state: Idle,
            metrics: AgentMetrics::new(),
        }
    }

    /// Assign a task to an idle agent
    pub fn assign(self, task_id: String) -> AgentInstance<Assigned> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Assigned { task_id },
            metrics: self.metrics,
        }
    }
}

impl AgentInstance<Assigned> {
    /// Start working on the assigned task
    pub fn start(self) -> AgentInstance<Working> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Working {
                task_id: self.state.task_id,
                progress: 0.0,
            },
            metrics: self.metrics,
        }
    }

    /// Cancel the assignment and return to idle
    pub fn cancel(self) -> AgentInstance<Idle> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Idle,
            metrics: self.metrics,
        }
    }
}

impl AgentInstance<Working> {
    /// Update progress
    pub fn update_progress(mut self, progress: f32) -> Self {
        self.state.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Complete the task successfully
    pub fn complete(self, result: String, duration_ms: u64, tokens: u64, cost_cents: u64) -> AgentInstance<Completed> {
        self.metrics.record_success(duration_ms, tokens, cost_cents);

        AgentInstance {
            id: self.id,
            name: self.name,
            state: Completed { result },
            metrics: self.metrics,
        }
    }

    /// Mark the task as failed
    pub fn fail(self, error: String) -> AgentInstance<Failed> {
        self.metrics.record_failure();

        AgentInstance {
            id: self.id,
            name: self.name,
            state: Failed { error },
            metrics: self.metrics,
        }
    }
}

impl AgentInstance<Completed> {
    /// Return to idle state after completion
    pub fn reset(self) -> AgentInstance<Idle> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Idle,
            metrics: self.metrics,
        }
    }
}

impl AgentInstance<Failed> {
    /// Return to idle state after failure
    pub fn reset(self) -> AgentInstance<Idle> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Idle,
            metrics: self.metrics,
        }
    }

    /// Retry the failed task
    pub fn retry(self, task_id: String) -> AgentInstance<Assigned> {
        AgentInstance {
            id: self.id,
            name: self.name,
            state: Assigned { task_id },
            metrics: self.metrics,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_transitions() {
        // Create idle agent
        let agent = AgentInstance::<Idle>::new("test-agent".to_string());
        assert_eq!(agent.name, "test-agent");

        // Assign task
        let agent = agent.assign("task-1".to_string());
        assert_eq!(agent.state.task_id, "task-1");

        // Start working
        let agent = agent.start();
        assert_eq!(agent.state.progress, 0.0);

        // Update progress
        let agent = agent.update_progress(0.5);
        assert_eq!(agent.state.progress, 0.5);

        // Complete
        let agent = agent.complete("success".to_string(), 1000, 500, 10);
        assert_eq!(agent.state.result, "success");

        // Reset to idle
        let agent = agent.reset();
        // Agent is back to idle state
    }

    #[test]
    fn test_failure_and_retry() {
        let agent = AgentInstance::<Idle>::new("test-agent".to_string());
        let agent = agent.assign("task-1".to_string());
        let agent = agent.start();
        let agent = agent.fail("error occurred".to_string());

        assert_eq!(agent.state.error, "error occurred");

        // Retry
        let agent = agent.retry("task-1-retry".to_string());
        assert_eq!(agent.state.task_id, "task-1-retry");
    }
}
