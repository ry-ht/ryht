// RocksDB storage layer for task tracking

use super::types::{Priority, Task, TaskId, TaskStatus};
use crate::storage::Storage;
use anyhow::{Context, Result};
use std::sync::Arc;

/// Storage operations for task tracking
pub struct TaskStorage {
    storage: Arc<dyn Storage>,
}

impl TaskStorage {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Save a task to storage
    pub async fn save_task(&self, task: &Task) -> Result<()> {
        // Load old task if exists to clean up old indices
        let old_task = self.load_task(&task.id).await?;

        // Serialize task
        let task_bytes = serde_json::to_vec(task)
            .context("Failed to serialize task")?;

        // Main task key
        let task_key = Self::task_key(&task.id);

        // Save task
        self.storage.put(&task_key, &task_bytes).await?;

        // Clean up old indices if task existed before
        if let Some(ref old_task) = old_task {
            self.remove_from_indices(old_task).await?;
        }

        // Update indices
        self.update_indices(task).await?;

        Ok(())
    }

    /// Load a task from storage
    pub async fn load_task(&self, task_id: &TaskId) -> Result<Option<Task>> {
        let task_key = Self::task_key(task_id);

        if let Some(task_bytes) = self.storage.get(&task_key).await? {
            let task: Task = serde_json::from_slice(&task_bytes)
                .context("Failed to deserialize task")?;
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Delete a task from storage
    pub async fn delete_task(&self, task_id: &TaskId) -> Result<()> {
        // Load task to get status and other info for index cleanup
        if let Some(task) = self.load_task(task_id).await? {
            // Delete main task
            let task_key = Self::task_key(task_id);
            self.storage.delete(&task_key).await?;

            // Clean up indices
            self.remove_from_indices(&task).await?;
        }

        Ok(())
    }

    /// List all tasks by status
    pub async fn list_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let prefix = Self::status_index_prefix(status);
        let keys = self.storage.get_keys_with_prefix(&prefix).await?;

        let mut tasks = Vec::new();
        for key in keys {
            // Extract task ID from index key
            if let Some(task_id) = Self::extract_task_id_from_index_key(&key) {
                if let Some(task) = self.load_task(&task_id).await? {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    /// List all tasks for a specific spec
    pub async fn list_by_spec(&self, spec_name: &str) -> Result<Vec<Task>> {
        let prefix = Self::spec_index_prefix(spec_name);
        let keys = self.storage.get_keys_with_prefix(&prefix).await?;

        let mut tasks = Vec::new();
        for key in keys {
            if let Some(task_id) = Self::extract_task_id_from_index_key(&key) {
                if let Some(task) = self.load_task(&task_id).await? {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    /// List all tasks
    pub async fn list_all(&self) -> Result<Vec<Task>> {
        let prefix = b"task:";
        let keys = self.storage.get_keys_with_prefix(prefix).await?;

        let mut tasks = Vec::new();
        for key in keys {
            if let Some(task_bytes) = self.storage.get(&key).await? {
                if let Ok(task) = serde_json::from_slice::<Task>(&task_bytes) {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    // === Key Format Helpers ===

    fn task_key(task_id: &TaskId) -> Vec<u8> {
        format!("task:{}", task_id.0).into_bytes()
    }

    fn status_index_key(status: TaskStatus, task_id: &TaskId) -> Vec<u8> {
        format!("idx_status:{}:{}", status, task_id.0).into_bytes()
    }

    fn status_index_prefix(status: TaskStatus) -> Vec<u8> {
        format!("idx_status:{}:", status).into_bytes()
    }

    fn spec_index_key(spec_name: &str, task_id: &TaskId) -> Vec<u8> {
        format!("idx_spec:{}:{}", spec_name, task_id.0).into_bytes()
    }

    fn spec_index_prefix(spec_name: &str) -> Vec<u8> {
        format!("idx_spec:{}:", spec_name).into_bytes()
    }

    fn session_index_key(session_id: &str, task_id: &TaskId) -> Vec<u8> {
        format!("idx_session:{}:{}", session_id, task_id.0).into_bytes()
    }

    fn priority_index_key(priority: Priority, task_id: &TaskId) -> Vec<u8> {
        format!("idx_priority:{}:{}", priority, task_id.0).into_bytes()
    }

    /// Extract task ID from index key (format: "idx_*:value:task_id")
    fn extract_task_id_from_index_key(key: &[u8]) -> Option<TaskId> {
        let key_str = String::from_utf8_lossy(key);
        let parts: Vec<&str> = key_str.split(':').collect();
        if parts.len() >= 3 {
            Some(TaskId::from_str(parts[2]))
        } else {
            None
        }
    }

    // === Index Management ===

    async fn update_indices(&self, task: &Task) -> Result<()> {
        let marker = b"1";

        // Status index
        let status_key = Self::status_index_key(task.status, &task.id);
        self.storage.put(&status_key, marker).await?;

        // Spec index (if spec_ref exists)
        if let Some(ref spec_ref) = task.spec_ref {
            let spec_key = Self::spec_index_key(&spec_ref.spec_name, &task.id);
            self.storage.put(&spec_key, marker).await?;
        }

        // Session index (if session_id exists)
        if let Some(ref session_id) = task.session_id {
            let session_key = Self::session_index_key(session_id, &task.id);
            self.storage.put(&session_key, marker).await?;
        }

        // Priority index
        let priority_key = Self::priority_index_key(task.priority, &task.id);
        self.storage.put(&priority_key, marker).await?;

        Ok(())
    }

    async fn remove_from_indices(&self, task: &Task) -> Result<()> {
        // Status index
        let status_key = Self::status_index_key(task.status, &task.id);
        self.storage.delete(&status_key).await?;

        // Spec index (if spec_ref exists)
        if let Some(ref spec_ref) = task.spec_ref {
            let spec_key = Self::spec_index_key(&spec_ref.spec_name, &task.id);
            self.storage.delete(&spec_key).await?;
        }

        // Session index (if session_id exists)
        if let Some(ref session_id) = task.session_id {
            let session_key = Self::session_index_key(session_id, &task.id);
            self.storage.delete(&session_key).await?;
        }

        // Priority index
        let priority_key = Self::priority_index_key(task.priority, &task.id);
        self.storage.delete(&priority_key).await?;

        Ok(())
    }

    /// Update indices when status changes
    pub async fn update_status_index(&self, task_id: &TaskId, old_status: TaskStatus, new_status: TaskStatus) -> Result<()> {
        // Remove old status index
        let old_key = Self::status_index_key(old_status, task_id);
        self.storage.delete(&old_key).await?;

        // Add new status index
        let new_key = Self::status_index_key(new_status, task_id);
        self.storage.put(&new_key, b"1").await?;

        Ok(())
    }

    /// Count all tasks
    pub async fn count_all_tasks(&self) -> Result<usize> {
        let prefix = b"task:";
        let keys = self.storage.get_keys_with_prefix(prefix).await?;
        Ok(keys.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;

    async fn create_test_storage() -> (TaskStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = MemoryStorage::new();
        let storage = TaskStorage::new(Arc::new(db));
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_save_and_load_task() {
        let (storage, _temp_dir) = create_test_storage().await;

        let task = Task::new("Test task".to_string());
        let task_id = task.id.clone();

        // Save task
        storage.save_task(&task).await.unwrap();

        // Load task
        let loaded = storage.load_task(&task_id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, "Test task");
    }

    #[tokio::test]
    async fn test_delete_task() {
        let (storage, _temp_dir) = create_test_storage().await;

        let task = Task::new("Test task".to_string());
        let task_id = task.id.clone();

        storage.save_task(&task).await.unwrap();
        storage.delete_task(&task_id).await.unwrap();

        let loaded = storage.load_task(&task_id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create tasks with different statuses
        let mut task1 = Task::new("Task 1".to_string());
        task1.update_status(TaskStatus::InProgress, None).unwrap();

        let task2 = Task::new("Task 2".to_string()); // Pending

        storage.save_task(&task1).await.unwrap();
        storage.save_task(&task2).await.unwrap();

        // List in-progress tasks
        let in_progress = storage.list_by_status(TaskStatus::InProgress).await.unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].title, "Task 1");

        // List pending tasks
        let pending = storage.list_by_status(TaskStatus::Pending).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Task 2");
    }
}
