// Core types for task tracking system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Default schema version for new tasks
fn default_schema_version() -> u32 {
    crate::storage::CURRENT_SCHEMA_VERSION
}

/// Default last_activity for tasks being migrated (set to current time)
fn default_last_activity() -> DateTime<Utc> {
    Utc::now()
}

/// Unique identifier for tasks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task status with clear transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Not started yet
    Pending,
    /// Currently being worked on
    InProgress,
    /// Blocked by external dependency
    Blocked,
    /// Successfully completed
    Done,
    /// Cancelled or abandoned
    Cancelled,
}

impl TaskStatus {
    /// Can transition to the given status
    pub fn can_transition_to(&self, target: TaskStatus) -> bool {
        use TaskStatus::*;
        match (self, target) {
            // From Pending
            (Pending, InProgress | Cancelled | Done) => true,
            // From InProgress
            (InProgress, Blocked | Done | Cancelled | Pending) => true,
            // From Blocked
            (Blocked, InProgress | Cancelled | Pending) => true,
            // Terminal states
            (Done, _) => false,
            (Cancelled, _) => false,
            // No self-transitions
            (a, b) if a == &b => false,
            _ => false,
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Done => write!(f, "done"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

/// Reference to a specification section
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecReference {
    /// Spec name (e.g., "spec", "documentation-tools-spec")
    pub spec_name: String,
    /// Section name or path (e.g., "Phase 1", "MCP Tools")
    pub section: String,
}

/// Status transition history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    /// When the transition occurred
    pub timestamp: DateTime<Utc>,
    /// Previous status (None for initial creation)
    pub from: Option<TaskStatus>,
    /// New status
    pub to: TaskStatus,
    /// Optional note about why
    pub note: Option<String>,
}

/// Task metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Schema version for migration support
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Unique identifier
    pub id: TaskId,

    /// Human-readable title
    pub title: String,

    /// Detailed description (optional)
    pub description: Option<String>,

    /// Current status
    pub status: TaskStatus,

    /// Priority level
    pub priority: Priority,

    /// Reference to specification section (if applicable)
    pub spec_ref: Option<SpecReference>,

    /// Session ID when created
    pub session_id: Option<String>,

    /// Session ID when last worked on
    pub active_session_id: Option<String>,

    /// When task was created
    pub created_at: DateTime<Utc>,

    /// When task was last updated
    pub updated_at: DateTime<Utc>,

    /// When task was started (set when transitioning to InProgress)
    pub started_at: Option<DateTime<Utc>>,

    /// Last activity timestamp (updated on any task modification)
    #[serde(default = "default_last_activity")]
    pub last_activity: DateTime<Utc>,

    /// Timeout in hours for InProgress tasks (None = no timeout)
    pub timeout_hours: Option<u32>,

    /// When task was completed (if done)
    pub completed_at: Option<DateTime<Utc>>,

    /// History of status changes
    #[serde(default)]
    pub history: Vec<StatusTransition>,

    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,

    /// Estimated effort (in hours, optional)
    pub estimated_hours: Option<f32>,

    /// Actual effort (in hours, tracked when done)
    pub actual_hours: Option<f32>,

    /// Task IDs that must be completed before this task can start
    #[serde(default)]
    pub depends_on: Vec<TaskId>,

    /// Related task IDs (for general relationships)
    #[serde(default)]
    pub related_tasks: Vec<TaskId>,

    /// Git commit hash (if committed)
    pub commit_hash: Option<String>,

    /// Episode ID (if completed and recorded)
    pub episode_id: Option<String>,
}

impl Task {
    /// Create a new task
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            schema_version: crate::storage::CURRENT_SCHEMA_VERSION,
            id: TaskId::new(),
            title,
            description: None,
            status: TaskStatus::Pending,
            priority: Priority::Medium,
            spec_ref: None,
            session_id: None,
            active_session_id: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            last_activity: now,
            timeout_hours: None,
            completed_at: None,
            history: vec![StatusTransition {
                timestamp: now,
                from: None,
                to: TaskStatus::Pending,
                note: Some("Task created".to_string()),
            }],
            tags: Vec::new(),
            estimated_hours: None,
            actual_hours: None,
            depends_on: Vec::new(),
            related_tasks: Vec::new(),
            commit_hash: None,
            episode_id: None,
        }
    }

    /// Update status with validation
    pub fn update_status(&mut self, new_status: TaskStatus, note: Option<String>) -> Result<(), String> {
        if !self.status.can_transition_to(new_status) {
            return Err(format!(
                "Invalid transition: {} -> {}",
                self.status, new_status
            ));
        }

        let now = Utc::now();
        let old_status = self.status;

        // Record transition
        self.history.push(StatusTransition {
            timestamp: now,
            from: Some(old_status),
            to: new_status,
            note,
        });

        self.status = new_status;
        self.updated_at = now;
        self.last_activity = now;

        // Set started_at when transitioning to InProgress for the first time
        if new_status == TaskStatus::InProgress && old_status != TaskStatus::InProgress
            && self.started_at.is_none() {
                self.started_at = Some(now);
            }

        // Clear started_at when leaving InProgress to a terminal state
        if old_status == TaskStatus::InProgress
            && (new_status == TaskStatus::Done || new_status == TaskStatus::Cancelled) {
            self.started_at = None;
        }

        // Set completed_at if transitioning to Done
        if new_status == TaskStatus::Done {
            self.completed_at = Some(now);
        }

        Ok(())
    }

    /// Check if this task has timed out
    pub fn is_timed_out(&self) -> bool {
        // Only check tasks that are in progress
        if self.status != TaskStatus::InProgress {
            return false;
        }

        // No timeout configured
        let Some(timeout_hours) = self.timeout_hours else {
            return false;
        };

        // Calculate elapsed time since last activity
        let timeout_duration = chrono::Duration::hours(timeout_hours as i64);
        let elapsed = Utc::now() - self.last_activity;

        elapsed > timeout_duration
    }

    /// Check if task can be started based on dependencies
    pub fn can_start(&self) -> bool {
        matches!(self.status, TaskStatus::Pending | TaskStatus::Blocked)
    }

    /// Check if this task has unmet dependencies
    pub fn has_unmet_dependencies(&self) -> bool {
        !self.depends_on.is_empty()
    }

    /// Add a dependency (does not check for cycles - use manager for that)
    pub fn add_dependency(&mut self, task_id: TaskId) {
        if !self.depends_on.contains(&task_id) {
            self.depends_on.push(task_id);
            self.updated_at = Utc::now();
            self.last_activity = Utc::now();
        }
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, task_id: &TaskId) -> bool {
        if let Some(pos) = self.depends_on.iter().position(|id| id == task_id) {
            self.depends_on.remove(pos);
            self.updated_at = Utc::now();
            self.last_activity = Utc::now();
            true
        } else {
            false
        }
    }
}

/// Task summary (for lists) - minimal token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: TaskId,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<SpecReference>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Task> for TaskSummary {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            status: task.status,
            priority: task.priority,
            spec_ref: task.spec_ref.clone(),
            updated_at: task.updated_at,
        }
    }
}

/// Progress statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub total_tasks: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub done: usize,
    pub cancelled: usize,
    pub completion_percentage: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_spec: Vec<SpecProgress>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_priority: Vec<PriorityProgress>,
}

/// Progress for a specific spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecProgress {
    pub spec_name: String,
    pub total: usize,
    pub done: usize,
    pub percentage: f32,
}

/// Progress by priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityProgress {
    pub priority: Priority,
    pub total: usize,
    pub done: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task".to_string());
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.history.len(), 1);
    }

    #[test]
    fn test_valid_status_transition() {
        let mut task = Task::new("Test".to_string());

        // Pending -> InProgress (valid)
        assert!(task.update_status(TaskStatus::InProgress, None).is_ok());
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.history.len(), 2);

        // InProgress -> Done (valid)
        assert!(task.update_status(TaskStatus::Done, Some("Finished".to_string())).is_ok());
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_invalid_status_transition() {
        let mut task = Task::new("Test".to_string());

        // Transition to InProgress first
        task.update_status(TaskStatus::InProgress, None).unwrap();

        // Mark as done
        task.update_status(TaskStatus::Done, None).unwrap();

        // Try to change from done (should fail)
        let result = task.update_status(TaskStatus::InProgress, None);
        assert!(result.is_err());
        assert_eq!(task.status, TaskStatus::Done); // Status unchanged
    }

    #[test]
    fn test_task_summary_conversion() {
        let task = Task::new("Test".to_string());
        let summary = TaskSummary::from(&task);

        assert_eq!(summary.id, task.id);
        assert_eq!(summary.title, task.title);
        assert_eq!(summary.status, task.status);
    }
}
