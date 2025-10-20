// Progress Manager - high-level task management operations

use super::storage::TaskStorage;
use super::types::{Priority, TaskStats, SpecReference, Task, TaskId, TaskStatus, TaskSummary};
use anyhow::{anyhow, Result};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;


/// Default cache size for task LRU cache
const TASK_CACHE_SIZE: usize = 100;
/// Progress Manager - manages tasks with caching and filtering
pub struct TaskManager {
    storage: Arc<TaskStorage>,
    cache: Arc<RwLock<LruCache<TaskId, Task>>>,
}

impl TaskManager {
    /// Create a new TaskManager
    pub fn new(storage: Arc<TaskStorage>) -> Self {
        let cache_size = NonZeroUsize::new(TASK_CACHE_SIZE)
            .expect("TASK_CACHE_SIZE constant must be non-zero");
        Self {
            storage,
            cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
        }
    }

    /// Create a new task
    pub async fn create_task(
        &self,
        title: String,
        description: Option<String>,
        priority: Option<Priority>,
        spec_ref: Option<SpecReference>,
        tags: Vec<String>,
        estimated_hours: Option<f32>,
        timeout_hours: Option<u32>,
    ) -> Result<TaskId> {
        let mut task = Task::new(title);

        task.description = description;
        task.priority = priority.unwrap_or(Priority::Medium);
        task.spec_ref = spec_ref;
        task.tags = tags;
        task.estimated_hours = estimated_hours;
        task.timeout_hours = timeout_hours;

        // Save to storage
        self.storage.save_task(&task).await?;

        // Cache the task
        self.cache.write().await.put(task.id.clone(), task.clone());

        Ok(task.id)
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &TaskId) -> Result<Task> {
        // Check cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(task) = cache.get(task_id) {
                return Ok(task.clone());
            }
        }

        // Load from storage
        let task = self.storage.load_task(task_id).await?
            .ok_or_else(|| anyhow!("Task not found: {}", task_id))?;

        // Cache for next time
        self.cache.write().await.put(task_id.clone(), task.clone());

        Ok(task)
    }

    /// Update a task
    pub async fn update_task(
        &self,
        task_id: &TaskId,
        title: Option<String>,
        description: Option<String>,
        priority: Option<Priority>,
        status: Option<TaskStatus>,
        status_note: Option<String>,
        tags: Option<Vec<String>>,
        estimated_hours: Option<f32>,
        actual_hours: Option<f32>,
        commit_hash: Option<String>,
    ) -> Result<()> {
        let mut task = self.get_task(task_id).await?;
        let old_status = task.status;
        let now = chrono::Utc::now();

        // Update fields
        if let Some(t) = title {
            task.title = t;
        }
        if let Some(d) = description {
            task.description = Some(d);
        }
        if let Some(p) = priority {
            task.priority = p;
        }
        if let Some(s) = status {
            task.update_status(s, status_note)
                .map_err(|e| anyhow!(e))?;
        }
        if let Some(t) = tags {
            task.tags = t;
        }
        if let Some(e) = estimated_hours {
            task.estimated_hours = Some(e);
        }
        if let Some(a) = actual_hours {
            task.actual_hours = Some(a);
        }
        if let Some(c) = commit_hash {
            task.commit_hash = Some(c);
        }

        // Update timestamps - ANY update counts as activity
        task.updated_at = now;
        task.last_activity = now;

        // Save to storage
        self.storage.save_task(&task).await?;

        // Update status index if status changed
        if let Some(new_status) = status {
            if new_status != old_status {
                self.storage.update_status_index(task_id, old_status, new_status).await?;
            }
        }

        // Update cache
        self.cache.write().await.put(task_id.clone(), task);

        Ok(())
    }

    /// Find tasks by commit hash
    pub async fn find_tasks_by_commit(&self, commit_hash: &str) -> Result<Vec<Task>> {
        let all_tasks = self.storage.list_all().await?;
        let matching_tasks: Vec<Task> = all_tasks
            .into_iter()
            .filter(|t| t.commit_hash.as_deref() == Some(commit_hash))
            .collect();

        tracing::info!("Found {} tasks for commit {}", matching_tasks.len(), commit_hash);
        Ok(matching_tasks)
    }

    /// Delete a task
    pub async fn delete_task(&self, task_id: &TaskId) -> Result<()> {
        // Remove from storage
        self.storage.delete_task(task_id).await?;

        // Remove from cache
        self.cache.write().await.pop(task_id);

        Ok(())
    }

    /// List tasks with optional filters
    pub async fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        spec_name: Option<String>,
        limit: Option<usize>,
    ) -> Result<Vec<TaskSummary>> {
        let mut tasks = if let Some(s) = status {
            self.storage.list_by_status(s).await?
        } else if let Some(spec) = spec_name {
            self.storage.list_by_spec(&spec).await?
        } else {
            self.storage.list_all().await?
        };

        // Sort by updated_at (most recent first)
        tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply limit
        if let Some(limit) = limit {
            tasks.truncate(limit);
        }

        // Convert to summaries (token efficiency)
        Ok(tasks.iter().map(TaskSummary::from).collect())
    }

    /// Get progress statistics
    pub async fn get_progress(&self, spec_name: Option<String>) -> Result<TaskStats> {
        use std::collections::HashMap;
        use super::types::{SpecProgress, PriorityProgress};

        let tasks = if let Some(spec) = spec_name {
            self.storage.list_by_spec(&spec).await?
        } else {
            self.storage.list_all().await?
        };

        let total = tasks.len();
        let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let blocked = tasks.iter().filter(|t| t.status == TaskStatus::Blocked).count();
        let done = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
        let cancelled = tasks.iter().filter(|t| t.status == TaskStatus::Cancelled).count();

        let completion_percentage = if total > 0 {
            (done as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        // Group by spec
        let mut spec_map: HashMap<String, (usize, usize)> = HashMap::new();
        for task in &tasks {
            if let Some(ref spec_ref) = task.spec_ref {
                let entry = spec_map.entry(spec_ref.spec_name.clone()).or_insert((0, 0));
                entry.0 += 1; // total
                if task.status == TaskStatus::Done {
                    entry.1 += 1; // done
                }
            }
        }

        let by_spec: Vec<SpecProgress> = spec_map
            .into_iter()
            .map(|(spec_name, (total, done))| {
                let percentage = if total > 0 {
                    (done as f32 / total as f32) * 100.0
                } else {
                    0.0
                };
                SpecProgress {
                    spec_name,
                    total,
                    done,
                    percentage,
                }
            })
            .collect();

        // Group by priority
        let mut priority_map: HashMap<Priority, (usize, usize)> = HashMap::new();
        for task in &tasks {
            let entry = priority_map.entry(task.priority).or_insert((0, 0));
            entry.0 += 1; // total
            if task.status == TaskStatus::Done {
                entry.1 += 1; // done
            }
        }

        let by_priority: Vec<PriorityProgress> = priority_map
            .into_iter()
            .map(|(priority, (total, done))| PriorityProgress {
                priority,
                total,
                done,
            })
            .collect();

        Ok(TaskStats {
            total_tasks: total,
            pending,
            in_progress,
            blocked,
            done,
            cancelled,
            completion_percentage,
            by_spec,
            by_priority,
        })
    }

    /// Mark task as complete with memory integration (auto-episode recording)
    pub async fn mark_complete(
        &self,
        task_id: &TaskId,
        actual_hours: Option<f32>,
        commit_hash: Option<String>,
        solution_summary: Option<String>,
        files_touched: Vec<String>,
        queries_made: Vec<String>,
        memory_system: Arc<tokio::sync::RwLock<crate::memory::MemorySystem>>,
    ) -> Result<Option<String>> {
        use crate::types::{EpisodeId, Outcome, TaskEpisode, TokenCount, ContextSnapshot};

        // 1. Load task
        let mut task = self.get_task(task_id).await?;

        // 2. Update to Done status
        let now = chrono::Utc::now();
        task.update_status(TaskStatus::Done, Some("Task completed".to_string()))
            .map_err(|e| anyhow!(e))?;
        task.actual_hours = actual_hours;
        task.commit_hash = commit_hash.clone();
        task.updated_at = now;
        task.last_activity = now;

        // 3. Build episode data
        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: chrono::Utc::now(),
            task_description: format!("{}: {}", task.title, task.description.clone().unwrap_or_default()),
            initial_context: ContextSnapshot::default(),
            queries_made,
            files_touched,
            solution_path: solution_summary.unwrap_or_else(|| task.title.clone()),
            outcome: Outcome::Success,
            tokens_used: TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.0,
        };

        let episode_id = episode.id.0.clone();

        // 4. Record episode in memory system
        let mut mem_system = memory_system.write().await;
        mem_system.episodic.record_episode(episode).await?;
        drop(mem_system);

        tracing::info!("Recorded episode {} for task {}", episode_id, task_id);

        // 5. Store episode_id in task
        task.episode_id = Some(episode_id.clone());

        // 6. Save task
        self.storage.save_task(&task).await?;

        // Update status index
        self.storage.update_status_index(task_id, TaskStatus::InProgress, TaskStatus::Done).await?;

        // Update cache
        self.cache.write().await.put(task_id.clone(), task);

        tracing::info!("Marked task {} as complete with episode {}", task_id, episode_id);

        // 7. Return episode_id
        Ok(Some(episode_id))
    }

    /// Search tasks by title or description (full-text search)
    pub async fn search_tasks(&self, query: &str, limit: Option<usize>) -> Result<Vec<TaskSummary>> {
        let all_tasks = self.storage.list_all().await?;

        let query_lower = query.to_lowercase();
        let mut matching_tasks: Vec<Task> = all_tasks
            .into_iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&query_lower)
                    || t.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || t.id.to_string().to_lowercase().contains(&query_lower)
            })
            .collect();

        // Sort by relevance (title matches first, then updated_at)
        matching_tasks.sort_by(|a, b| {
            let a_title_match = a.title.to_lowercase().contains(&query_lower);
            let b_title_match = b.title.to_lowercase().contains(&query_lower);

            match (a_title_match, b_title_match) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.updated_at.cmp(&a.updated_at),
            }
        });

        // Apply limit
        if let Some(limit) = limit {
            matching_tasks.truncate(limit);
        }

        tracing::info!("Found {} tasks matching query: {}", matching_tasks.len(), query);

        Ok(matching_tasks.iter().map(TaskSummary::from).collect())
    }

    /// Link task to a specification section with validation
    pub async fn link_to_spec(
        &self,
        task_id: &TaskId,
        spec_name: String,
        section: String,
        validate: bool,
        spec_manager: Arc<tokio::sync::RwLock<crate::specs::SpecificationManager>>,
    ) -> Result<()> {
        // 1. Load task
        let mut task = self.get_task(task_id).await?;

        // 2. Validate spec and section if requested
        if validate {
            let mut spec_mgr = spec_manager.write().await;

            // Check if spec exists
            spec_mgr.get_spec(&spec_name)
                .map_err(|e| anyhow!("Spec '{}' not found: {}", spec_name, e))?;

            // Check if section exists
            let sections = spec_mgr.list_sections(&spec_name)?;
            let section_exists = sections.iter().any(|s| {
                s.to_lowercase().contains(&section.to_lowercase())
                    || section.to_lowercase().contains(&s.to_lowercase())
            });

            if !section_exists {
                return Err(anyhow!(
                    "Section '{}' not found in spec '{}'. Available sections: {}",
                    section,
                    spec_name,
                    sections.join(", ")
                ));
            }

            tracing::info!("Validated spec reference: {} - {}", spec_name, section);
        }

        // 3. Set task.spec_ref
        task.spec_ref = Some(SpecReference {
            spec_name: spec_name.clone(),
            section: section.clone(),
        });
        task.updated_at = chrono::Utc::now();

        // 4. Save task
        self.storage.save_task(&task).await?;

        // Update cache
        self.cache.write().await.put(task_id.clone(), task);

        tracing::info!("Linked task {} to spec {} - {}", task_id, spec_name, section);

        Ok(())
    }

    /// Get status transition history for a task
    pub async fn get_history(&self, task_id: &TaskId) -> Result<Vec<crate::tasks::types::StatusTransition>> {
        let task = self.get_task(task_id).await?;
        Ok(task.history)
    }

    /// Clear cache (useful for testing)
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    // ========== Timeout and Recovery Methods ==========

    /// Check for timed-out tasks and revert them to pending
    /// Returns the list of recovered task IDs
    pub async fn check_timeouts(&self) -> Result<Vec<TaskId>> {
        let in_progress = self.storage.list_by_status(TaskStatus::InProgress).await?;
        let mut recovered = Vec::new();

        for task in in_progress {
            if task.is_timed_out() {
                let timeout_hours = task.timeout_hours
                    .expect("is_timed_out() guarantees timeout_hours is Some");
                tracing::warn!(
                    "Task {} timed out after {} hours, reverting to pending",
                    task.id, timeout_hours
                );

                let mut updated = task.clone();
                updated.update_status(
                    TaskStatus::Pending,
                    Some(format!(
                        "Auto-reverted: timeout after {} hours of inactivity",
                        timeout_hours
                    ))
                ).map_err(|e| anyhow!(e))?;
                updated.last_activity = chrono::Utc::now();

                // Save updated task
                self.storage.save_task(&updated).await?;

                // Update status index
                self.storage.update_status_index(&updated.id, TaskStatus::InProgress, TaskStatus::Pending).await?;

                // Update cache
                self.cache.write().await.put(updated.id.clone(), updated.clone());

                recovered.push(updated.id);
            }
        }

        if !recovered.is_empty() {
            tracing::info!("Recovered {} timed-out tasks", recovered.len());
        }

        Ok(recovered)
    }

    /// Manually recover all orphaned tasks (tasks stuck in InProgress)
    /// If force=true, ignores timeout settings and recovers all InProgress tasks
    /// Returns the list of recovered task IDs
    pub async fn recover_orphaned_tasks(&self, force: bool) -> Result<Vec<TaskId>> {
        let in_progress = self.storage.list_by_status(TaskStatus::InProgress).await?;
        let mut recovered = Vec::new();

        for task in in_progress {
            let should_recover = if force {
                true
            } else {
                task.is_timed_out()
            };

            if should_recover {
                tracing::warn!(
                    "Recovering orphaned task {} (force={})",
                    task.id, force
                );

                let mut updated = task.clone();
                let reason = if force {
                    "Manual recovery: forced revert to pending".to_string()
                } else {
                    format!(
                        "Manual recovery: timeout after {} hours",
                        task.timeout_hours.unwrap_or(0)
                    )
                };

                updated.update_status(TaskStatus::Pending, Some(reason))
                    .map_err(|e| anyhow!(e))?;
                updated.last_activity = chrono::Utc::now();

                // Save updated task
                self.storage.save_task(&updated).await?;

                // Update status index
                self.storage.update_status_index(&updated.id, TaskStatus::InProgress, TaskStatus::Pending).await?;

                // Update cache
                self.cache.write().await.put(updated.id.clone(), updated.clone());

                recovered.push(updated.id);
            }
        }

        tracing::info!("Recovered {} orphaned tasks (force={})", recovered.len(), force);

        Ok(recovered)
    }

    /// Count all tasks
    pub async fn count_all(&self) -> Result<usize> {
        self.storage.count_all_tasks().await
    }

    // ========== Dependency Management Methods ==========

    /// Add a dependency relationship between tasks
    /// Returns error if this would create a circular dependency
    pub async fn add_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()> {
        // Validate both tasks exist
        let mut task = self.get_task(task_id).await?;
        self.get_task(depends_on).await?; // Ensure dependency exists

        // Check for circular dependency
        if self.would_create_cycle(task_id, depends_on).await? {
            return Err(anyhow!(
                "Cannot add dependency: would create circular dependency"
            ));
        }

        // Add dependency
        task.add_dependency(depends_on.clone());
        task.last_activity = chrono::Utc::now();

        // Save
        self.storage.save_task(&task).await?;
        self.cache.write().await.put(task_id.clone(), task);

        Ok(())
    }

    /// Remove a dependency relationship
    pub async fn remove_dependency(&self, task_id: &TaskId, depends_on: &TaskId) -> Result<()> {
        let mut task = self.get_task(task_id).await?;

        if task.remove_dependency(depends_on) {
            task.last_activity = chrono::Utc::now();
            self.storage.save_task(&task).await?;
            self.cache.write().await.put(task_id.clone(), task);
        }

        Ok(())
    }

    /// Get all tasks that this task depends on
    pub async fn get_dependencies(&self, task_id: &TaskId) -> Result<Vec<Task>> {
        let task = self.get_task(task_id).await?;
        let mut dependencies = Vec::new();

        for dep_id in &task.depends_on {
            if let Ok(dep_task) = self.get_task(dep_id).await {
                dependencies.push(dep_task);
            }
        }

        Ok(dependencies)
    }

    /// Get all tasks that depend on this task (blocked by this task)
    pub async fn get_dependents(&self, task_id: &TaskId) -> Result<Vec<Task>> {
        let all_tasks = self.storage.list_all().await?;
        let dependents: Vec<Task> = all_tasks
            .into_iter()
            .filter(|t| t.depends_on.contains(task_id))
            .collect();

        Ok(dependents)
    }

    /// Check if task can be started (all dependencies are Done)
    pub async fn can_start_task(&self, task_id: &TaskId) -> Result<bool> {
        let task = self.get_task(task_id).await?;

        // If not in a startable state, return false
        if !task.can_start() {
            return Ok(false);
        }

        // Check all dependencies
        for dep_id in &task.depends_on {
            if let Ok(dep_task) = self.get_task(dep_id).await {
                if dep_task.status != TaskStatus::Done {
                    return Ok(false);
                }
            } else {
                // Dependency doesn't exist - treat as unmet
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Get unmet dependencies for a task
    pub async fn get_unmet_dependencies(&self, task_id: &TaskId) -> Result<Vec<Task>> {
        let task = self.get_task(task_id).await?;
        let mut unmet = Vec::new();

        for dep_id in &task.depends_on {
            if let Ok(dep_task) = self.get_task(dep_id).await {
                if dep_task.status != TaskStatus::Done {
                    unmet.push(dep_task);
                }
            }
        }

        Ok(unmet)
    }

    /// Check if adding a dependency would create a cycle
    async fn would_create_cycle(&self, task_id: &TaskId, new_dep: &TaskId) -> Result<bool> {
        // If task depends on itself, that's a cycle
        if task_id == new_dep {
            return Ok(true);
        }

        // Check if new_dep transitively depends on task_id
        // This would create a cycle: task_id -> new_dep -> ... -> task_id
        self.has_transitive_dependency(new_dep, task_id, &mut std::collections::HashSet::new()).await
    }

    /// Recursively check if 'from' task transitively depends on 'target' task
    #[async_recursion::async_recursion]
    async fn has_transitive_dependency(
        &self,
        from: &TaskId,
        target: &TaskId,
        visited: &mut std::collections::HashSet<TaskId>,
    ) -> Result<bool> {
        // Avoid infinite loops
        if visited.contains(from) {
            return Ok(false);
        }
        visited.insert(from.clone());

        // Get dependencies of 'from' task
        let task = self.get_task(from).await?;

        for dep_id in &task.depends_on {
            // If we find the target, there's a path
            if dep_id == target {
                return Ok(true);
            }

            // Recursively check this dependency
            if self.has_transitive_dependency(dep_id, target, visited).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Auto-update blocked tasks when a dependency completes
    /// Call this after marking a task as done
    pub async fn update_dependent_tasks(&self, completed_task_id: &TaskId) -> Result<Vec<TaskId>> {
        let dependents = self.get_dependents(completed_task_id).await?;
        let mut unblocked = Vec::new();

        for dep_task in dependents {
            // Check if this task can now start
            if dep_task.status == TaskStatus::Blocked {
                // Check if ALL dependencies are met
                let can_start = self.can_start_task(&dep_task.id).await?;

                if can_start {
                    // Update status from Blocked to Pending
                    self.update_task(
                        &dep_task.id,
                        None,
                        None,
                        None,
                        Some(TaskStatus::Pending),
                        Some(format!("Unblocked: dependency {} completed", completed_task_id)),
                        None,
                        None,
                        None,
                        None,
                    ).await?;

                    unblocked.push(dep_task.id);
                }
            }
        }

        Ok(unblocked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use tempfile::TempDir;

    async fn create_test_manager() -> (TaskManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = MemoryStorage::new();
        let storage = Arc::new(TaskStorage::new(Arc::new(db)));
        let manager = TaskManager::new(storage);
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task(
            "Test task".to_string(),
            Some("Description".to_string()),
            Some(Priority::High),
            None,
            vec!["test".to_string()],
            Some(2.0),
            None,
        ).await.unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.title, "Test task");
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags, vec!["test"]);
    }

    #[tokio::test]
    async fn test_update_task_status() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task(
            "Test".to_string(),
            None,
            None,
            None,
            vec![],
            None,
            None,
        ).await.unwrap();

        // Update status
        manager.update_task(
            &task_id,
            None,
            None,
            None,
            Some(TaskStatus::InProgress),
            Some("Starting work".to_string()),
            None,
            None,
            None,
            None,
        ).await.unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.history.len(), 2); // Created + Updated
    }

    #[tokio::test]
    async fn test_list_tasks_by_status() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create tasks
        let id1 = manager.create_task("Task 1".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let id2 = manager.create_task("Task 2".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Update one to InProgress
        manager.update_task(&id1, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // List in-progress
        let in_progress = manager.list_tasks(Some(TaskStatus::InProgress), None, None).await.unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].id, id1);

        // List pending
        let pending = manager.list_tasks(Some(TaskStatus::Pending), None, None).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id2);
    }

    #[tokio::test]
    async fn test_get_progress() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create 5 tasks
        for i in 0..5 {
            let id = manager.create_task(format!("Task {}", i), None, None, None, vec![], None, None).await.unwrap();
            if i < 2 {
                // Mark first 2 as done
                manager.update_task(&id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
            }
        }

        let stats = manager.get_progress(None).await.unwrap();
        assert_eq!(stats.total_tasks, 5);
        assert_eq!(stats.done, 2);
        assert_eq!(stats.pending, 3);
        assert_eq!(stats.completion_percentage, 40.0);
    }

    #[tokio::test]
    async fn test_get_progress_with_spec_grouping() {
        use crate::tasks::SpecReference;

        let (manager, _temp_dir) = create_test_manager().await;

        // Create tasks for different specs
        let spec1_ref = Some(SpecReference {
            spec_name: "spec1".to_string(),
            section: "Phase 1".to_string(),
        });
        let spec2_ref = Some(SpecReference {
            spec_name: "spec2".to_string(),
            section: "Phase 1".to_string(),
        });

        // Spec1: 3 tasks (2 done)
        for i in 0..3 {
            let id = manager.create_task(format!("Spec1 Task {}", i), None, None, spec1_ref.clone(), vec![], None, None).await.unwrap();
            if i < 2 {
                manager.update_task(&id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
            }
        }

        // Spec2: 2 tasks (1 done)
        for i in 0..2 {
            let id = manager.create_task(format!("Spec2 Task {}", i), None, None, spec2_ref.clone(), vec![], None, None).await.unwrap();
            if i == 0 {
                manager.update_task(&id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
            }
        }

        let stats = manager.get_progress(None).await.unwrap();
        assert_eq!(stats.total_tasks, 5);
        assert_eq!(stats.by_spec.len(), 2);

        // Find spec1 stats
        let spec1_stats = stats.by_spec.iter().find(|s| s.spec_name == "spec1").unwrap();
        assert_eq!(spec1_stats.total, 3);
        assert_eq!(spec1_stats.done, 2);
        assert!((spec1_stats.percentage - 66.666).abs() < 0.1);

        // Find spec2 stats
        let spec2_stats = stats.by_spec.iter().find(|s| s.spec_name == "spec2").unwrap();
        assert_eq!(spec2_stats.total, 2);
        assert_eq!(spec2_stats.done, 1);
        assert_eq!(spec2_stats.percentage, 50.0);
    }

    #[tokio::test]
    async fn test_get_progress_with_priority_grouping() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create tasks with different priorities
        for i in 0..6 {
            let priority = match i % 3 {
                0 => Priority::High,
                1 => Priority::Medium,
                _ => Priority::Low,
            };
            let id = manager.create_task(format!("Task {}", i), None, Some(priority), None, vec![], None, None).await.unwrap();
            if i < 3 {
                manager.update_task(&id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
            }
        }

        let stats = manager.get_progress(None).await.unwrap();
        assert_eq!(stats.total_tasks, 6);
        assert_eq!(stats.by_priority.len(), 3);

        // Check that all priorities are represented
        let high_count = stats.by_priority.iter().filter(|p| p.priority == Priority::High).count();
        let medium_count = stats.by_priority.iter().filter(|p| p.priority == Priority::Medium).count();
        let low_count = stats.by_priority.iter().filter(|p| p.priority == Priority::Low).count();

        assert_eq!(high_count, 1);
        assert_eq!(medium_count, 1);
        assert_eq!(low_count, 1);
    }

    #[tokio::test]
    async fn test_cache() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Cached".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // First get (from storage)
        let _task1 = manager.get_task(&task_id).await.unwrap();

        // Clear storage to test cache
        manager.clear_cache().await;

        // Second get (should still work from cache before clear)
        let task_id2 = manager.create_task("Test2".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let _task2 = manager.get_task(&task_id2).await.unwrap();
    }

    #[tokio::test]
    async fn test_search_tasks() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create tasks with different titles and descriptions
        manager.create_task(
            "Implement search feature".to_string(),
            Some("Add full-text search to the API".to_string()),
            None,
            None,
            vec![],
            None,
            None,
        ).await.unwrap();

        manager.create_task(
            "Fix bug in search".to_string(),
            Some("The search is returning wrong results".to_string()),
            None,
            None,
            vec![],
            None,
            None,
        ).await.unwrap();

        manager.create_task(
            "Add pagination".to_string(),
            Some("Implement pagination for lists".to_string()),
            None,
            None,
            vec![],
            None,
            None,
        ).await.unwrap();

        // Search by title
        let results = manager.search_tasks("search", None).await.unwrap();
        assert_eq!(results.len(), 2);

        // Search by description
        let results = manager.search_tasks("pagination", None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Add pagination");

        // Search with limit
        let results = manager.search_tasks("search", Some(1)).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_get_history() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Test task".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Update status a few times
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), Some("Starting".to_string()), None, None, None, None).await.unwrap();
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::Blocked), Some("Waiting for review".to_string()), None, None, None, None).await.unwrap();
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), Some("Resuming".to_string()), None, None, None, None).await.unwrap();

        let history = manager.get_history(&task_id).await.unwrap();
        assert_eq!(history.len(), 4); // Created + 3 updates
        assert_eq!(history[0].to, TaskStatus::Pending);
        assert_eq!(history[1].to, TaskStatus::InProgress);
        assert_eq!(history[2].to, TaskStatus::Blocked);
        assert_eq!(history[3].to, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_mark_complete_with_memory() {
        use crate::config::MemoryConfig;
        use crate::memory::MemorySystem;
        use crate::storage::MemoryStorage;

        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(MemoryStorage::new());

        // Create memory system
        let memory_config = MemoryConfig {
            working_memory_size: "10MB".to_string(),
            episodic_retention_days: 90,
            consolidation_interval: "1h".to_string(),
        };
        let mut memory_system = MemorySystem::new(db.clone(), memory_config).unwrap();
        memory_system.init().await.unwrap();
        let memory_system = Arc::new(tokio::sync::RwLock::new(memory_system));

        // Create progress manager
        let storage = Arc::new(TaskStorage::new(db));
        let manager = TaskManager::new(storage);

        // Create and complete a task
        let task_id = manager.create_task(
            "Implement feature X".to_string(),
            Some("Add feature X to the system".to_string()),
            None,
            None,
            vec![],
            None,
            None,
        ).await.unwrap();

        // Mark as in progress first
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Mark complete with episode data
        let episode_id = manager.mark_complete(
            &task_id,
            Some(2.5),
            Some("abc123".to_string()),
            Some("Implemented feature X using approach Y".to_string()),
            vec!["src/feature.rs".to_string(), "src/tests.rs".to_string()],
            vec!["code.search feature".to_string(), "code.get_definition FeatureX".to_string()],
            memory_system.clone(),
        ).await.unwrap();

        // Verify task is marked as done
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Done);
        assert_eq!(task.actual_hours, Some(2.5));
        assert_eq!(task.commit_hash, Some("abc123".to_string()));
        assert!(task.completed_at.is_some());
        assert!(task.episode_id.is_some());
        assert_eq!(task.episode_id, episode_id);

        // Verify episode was recorded in memory system
        let mem = memory_system.read().await;
        let similar = mem.episodic.find_similar("feature X", 5).await;
        assert_eq!(similar.len(), 1);
        assert_eq!(similar[0].id.0, episode_id.unwrap());
    }

    #[tokio::test]
    async fn test_find_tasks_by_commit() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create multiple tasks with different commits
        let id1 = manager.create_task("Task 1".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let id2 = manager.create_task("Task 2".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let id3 = manager.create_task("Task 3".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Mark tasks as done with commit hashes
        manager.update_task(&id1, None, None, None, Some(TaskStatus::Done), None, None, None, None, Some("abc123".to_string())).await.unwrap();
        manager.update_task(&id2, None, None, None, Some(TaskStatus::Done), None, None, None, None, Some("abc123".to_string())).await.unwrap();
        manager.update_task(&id3, None, None, None, Some(TaskStatus::Done), None, None, None, None, Some("def456".to_string())).await.unwrap();

        // Find tasks by commit hash
        let tasks_abc = manager.find_tasks_by_commit("abc123").await.unwrap();
        assert_eq!(tasks_abc.len(), 2);
        assert!(tasks_abc.iter().any(|t| t.id == id1));
        assert!(tasks_abc.iter().any(|t| t.id == id2));

        let tasks_def = manager.find_tasks_by_commit("def456").await.unwrap();
        assert_eq!(tasks_def.len(), 1);
        assert_eq!(tasks_def[0].id, id3);

        // Query non-existent commit
        let tasks_none = manager.find_tasks_by_commit("xyz789").await.unwrap();
        assert_eq!(tasks_none.len(), 0);
    }

    // === Timeout and Recovery Tests ===

    #[tokio::test]
    async fn test_task_timeout_detection() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create task with 1 hour timeout
        let task_id = manager.create_task(
            "Task with timeout".to_string(),
            None,
            None,
            None,
            vec![],
            None,
            Some(1), // 1 hour timeout
        ).await.unwrap();

        // Mark as in progress
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Verify task is not timed out yet
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.started_at.is_some());
        assert!(!task.is_timed_out());

        // Manually set last_activity to past time to simulate timeout
        let mut task = manager.get_task(&task_id).await.unwrap();
        task.last_activity = chrono::Utc::now() - chrono::Duration::hours(2);
        manager.storage.save_task(&task).await.unwrap();
        manager.cache.write().await.put(task_id.clone(), task.clone());

        // Now task should be timed out
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(task.is_timed_out());
    }

    #[tokio::test]
    async fn test_timeout_recovery() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create task with 1 hour timeout
        let task_id = manager.create_task(
            "Task to recover".to_string(),
            None,
            None,
            None,
            vec![],
            None,
            Some(1),
        ).await.unwrap();

        // Mark as in progress
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Simulate timeout by setting last_activity to past
        let mut task = manager.get_task(&task_id).await.unwrap();
        task.last_activity = chrono::Utc::now() - chrono::Duration::hours(2);
        manager.storage.save_task(&task).await.unwrap();
        manager.cache.write().await.put(task_id.clone(), task.clone());

        // Check timeouts - should recover the task
        let recovered = manager.check_timeouts().await.unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0], task_id);

        // Verify task is now pending
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Pending);

        // Verify history records the auto-revert
        assert!(task.history.iter().any(|h|
            h.to == TaskStatus::Pending &&
            h.note.as_ref().map(|n| n.contains("Auto-reverted")).unwrap_or(false)
        ));
    }

    #[tokio::test]
    async fn test_no_timeout_when_active() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create task with timeout
        let task_id = manager.create_task(
            "Active task".to_string(),
            None,
            None,
            None,
            vec![],
            None,
            Some(2), // 2 hour timeout
        ).await.unwrap();

        // Mark as in progress
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Update task (simulates activity)
        manager.update_task(&task_id, None, Some("Still working on it".to_string()), None, None, None, None, None, None, None).await.unwrap();

        // Check timeouts - should not recover anything
        let recovered = manager.check_timeouts().await.unwrap();
        assert_eq!(recovered.len(), 0);

        // Task should still be in progress
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_no_timeout_without_config() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create task WITHOUT timeout
        let task_id = manager.create_task(
            "Task without timeout".to_string(),
            None,
            None,
            None,
            vec![],
            None,
            None, // No timeout
        ).await.unwrap();

        // Mark as in progress
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Simulate long inactivity
        let mut task = manager.get_task(&task_id).await.unwrap();
        task.last_activity = chrono::Utc::now() - chrono::Duration::hours(100);
        manager.storage.save_task(&task).await.unwrap();
        manager.cache.write().await.put(task_id.clone(), task.clone());

        // Check timeouts - should not recover (no timeout configured)
        let recovered = manager.check_timeouts().await.unwrap();
        assert_eq!(recovered.len(), 0);

        // Task should still be in progress
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(!task.is_timed_out());
    }

    #[tokio::test]
    async fn test_manual_recovery_with_force() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create multiple in-progress tasks without timeouts
        let id1 = manager.create_task("Task 1".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let id2 = manager.create_task("Task 2".to_string(), None, None, None, vec![], None, None).await.unwrap();

        manager.update_task(&id1, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();
        manager.update_task(&id2, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Force recovery should recover all in-progress tasks
        let recovered = manager.recover_orphaned_tasks(true).await.unwrap();
        assert_eq!(recovered.len(), 2);
        assert!(recovered.contains(&id1));
        assert!(recovered.contains(&id2));

        // Both should be pending now
        let task1 = manager.get_task(&id1).await.unwrap();
        let task2 = manager.get_task(&id2).await.unwrap();
        assert_eq!(task1.status, TaskStatus::Pending);
        assert_eq!(task2.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_manual_recovery_without_force() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create tasks: one with timeout expired, one without
        let id_timeout = manager.create_task("With timeout".to_string(), None, None, None, vec![], None, Some(1)).await.unwrap();
        let id_no_timeout = manager.create_task("No timeout".to_string(), None, None, None, vec![], None, None).await.unwrap();

        manager.update_task(&id_timeout, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();
        manager.update_task(&id_no_timeout, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Simulate timeout for first task
        let mut task = manager.get_task(&id_timeout).await.unwrap();
        task.last_activity = chrono::Utc::now() - chrono::Duration::hours(2);
        manager.storage.save_task(&task).await.unwrap();
        manager.cache.write().await.put(id_timeout.clone(), task.clone());

        // Non-force recovery should only recover timed-out task
        let recovered = manager.recover_orphaned_tasks(false).await.unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0], id_timeout);

        // Only timed-out task should be pending
        let task1 = manager.get_task(&id_timeout).await.unwrap();
        let task2 = manager.get_task(&id_no_timeout).await.unwrap();
        assert_eq!(task1.status, TaskStatus::Pending);
        assert_eq!(task2.status, TaskStatus::InProgress);
    }

    #[tokio::test]
    async fn test_started_at_tracking() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Test".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Initially no started_at
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(task.started_at.is_none());

        // Transition to InProgress sets started_at
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(task.started_at.is_some());
        let first_started = task.started_at.unwrap();

        // Transition to Blocked keeps started_at
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::Blocked), None, None, None, None, None).await.unwrap();
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.started_at, Some(first_started));

        // Back to InProgress keeps original started_at
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.started_at, Some(first_started));

        // Transition to Done clears started_at
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
        let task = manager.get_task(&task_id).await.unwrap();
        assert!(task.started_at.is_none());
    }

    #[tokio::test]
    async fn test_last_activity_updates() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Test".to_string(), None, None, None, vec![], None, None).await.unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        let initial_activity = task.last_activity;

        // Wait a bit to ensure timestamp difference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Any update should update last_activity
        manager.update_task(&task_id, None, Some("Updated description".to_string()), None, None, None, None, None, None, None).await.unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert!(task.last_activity > initial_activity);
    }

    // === Dependency Management Tests ===

    #[tokio::test]
    async fn test_add_dependency() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create two tasks
        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Add dependency: A depends on B
        manager.add_dependency(&task_a_id, &task_b_id).await.unwrap();

        // Verify dependency was added
        let task_a = manager.get_task(&task_a_id).await.unwrap();
        assert_eq!(task_a.depends_on.len(), 1);
        assert_eq!(task_a.depends_on[0], task_b_id);

        // Verify dependencies can be retrieved
        let dependencies = manager.get_dependencies(&task_a_id).await.unwrap();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].id, task_b_id);
    }

    #[tokio::test]
    async fn test_remove_dependency() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Add then remove dependency
        manager.add_dependency(&task_a_id, &task_b_id).await.unwrap();
        manager.remove_dependency(&task_a_id, &task_b_id).await.unwrap();

        // Verify dependency was removed
        let task_a = manager.get_task(&task_a_id).await.unwrap();
        assert_eq!(task_a.depends_on.len(), 0);
    }

    #[tokio::test]
    async fn test_circular_dependency_direct() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // A depends on B
        manager.add_dependency(&task_a_id, &task_b_id).await.unwrap();

        // Try to make B depend on A (should fail - circular)
        let result = manager.add_dependency(&task_b_id, &task_a_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[tokio::test]
    async fn test_circular_dependency_transitive() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create chain: A -> B -> C
        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_c_id = manager.create_task("Task C".to_string(), None, None, None, vec![], None, None).await.unwrap();

        manager.add_dependency(&task_a_id, &task_b_id).await.unwrap();
        manager.add_dependency(&task_b_id, &task_c_id).await.unwrap();

        // Try to make C depend on A (should fail - creates cycle)
        let result = manager.add_dependency(&task_c_id, &task_a_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[tokio::test]
    async fn test_self_dependency() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Task".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Task cannot depend on itself
        let result = manager.add_dependency(&task_id, &task_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("circular"));
    }

    #[tokio::test]
    async fn test_get_dependents() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_c_id = manager.create_task("Task C".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Both B and C depend on A
        manager.add_dependency(&task_b_id, &task_a_id).await.unwrap();
        manager.add_dependency(&task_c_id, &task_a_id).await.unwrap();

        // Get dependents of A
        let dependents = manager.get_dependents(&task_a_id).await.unwrap();
        assert_eq!(dependents.len(), 2);

        let dependent_ids: Vec<TaskId> = dependents.iter().map(|t| t.id.clone()).collect();
        assert!(dependent_ids.contains(&task_b_id));
        assert!(dependent_ids.contains(&task_c_id));
    }

    #[tokio::test]
    async fn test_can_start_task_no_dependencies() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Task".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Task with no dependencies can start
        let can_start = manager.can_start_task(&task_id).await.unwrap();
        assert!(can_start);
    }

    #[tokio::test]
    async fn test_can_start_task_with_met_dependencies() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // B depends on A
        manager.add_dependency(&task_b_id, &task_a_id).await.unwrap();

        // Initially B cannot start (A not done)
        let can_start = manager.can_start_task(&task_b_id).await.unwrap();
        assert!(!can_start);

        // Mark A as done
        manager.update_task(&task_a_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // Now B can start
        let can_start = manager.can_start_task(&task_b_id).await.unwrap();
        assert!(can_start);
    }

    #[tokio::test]
    async fn test_can_start_task_with_unmet_dependencies() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_c_id = manager.create_task("Task C".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // C depends on both A and B
        manager.add_dependency(&task_c_id, &task_a_id).await.unwrap();
        manager.add_dependency(&task_c_id, &task_b_id).await.unwrap();

        // Mark only A as done
        manager.update_task(&task_a_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // C still cannot start (B not done)
        let can_start = manager.can_start_task(&task_c_id).await.unwrap();
        assert!(!can_start);

        // Mark B as done
        manager.update_task(&task_b_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // Now C can start
        let can_start = manager.can_start_task(&task_c_id).await.unwrap();
        assert!(can_start);
    }

    #[tokio::test]
    async fn test_get_unmet_dependencies() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_c_id = manager.create_task("Task C".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // C depends on A and B
        manager.add_dependency(&task_c_id, &task_a_id).await.unwrap();
        manager.add_dependency(&task_c_id, &task_b_id).await.unwrap();

        // All dependencies are unmet
        let unmet = manager.get_unmet_dependencies(&task_c_id).await.unwrap();
        assert_eq!(unmet.len(), 2);

        // Mark A as done
        manager.update_task(&task_a_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // Only B is unmet now
        let unmet = manager.get_unmet_dependencies(&task_c_id).await.unwrap();
        assert_eq!(unmet.len(), 1);
        assert_eq!(unmet[0].id, task_b_id);

        // Mark B as done
        manager.update_task(&task_b_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // No unmet dependencies
        let unmet = manager.get_unmet_dependencies(&task_c_id).await.unwrap();
        assert_eq!(unmet.len(), 0);
    }

    #[tokio::test]
    async fn test_update_dependent_tasks() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // B depends on A
        manager.add_dependency(&task_b_id, &task_a_id).await.unwrap();

        // Mark B as in-progress first, then blocked (pending->blocked is invalid)
        manager.update_task(&task_b_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();
        manager.update_task(&task_b_id, None, None, None, Some(TaskStatus::Blocked), None, None, None, None, None).await.unwrap();

        // Complete A
        manager.update_task(&task_a_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // Update dependent tasks
        let unblocked = manager.update_dependent_tasks(&task_a_id).await.unwrap();

        // B should be unblocked
        assert_eq!(unblocked.len(), 1);
        assert_eq!(unblocked[0], task_b_id);

        // Verify B is now pending
        let task_b = manager.get_task(&task_b_id).await.unwrap();
        assert_eq!(task_b.status, TaskStatus::Pending);
    }

    #[tokio::test]
    async fn test_multiple_dependencies_chain() {
        let (manager, _temp_dir) = create_test_manager().await;

        // Create dependency chain: D -> C -> B -> A
        let task_a_id = manager.create_task("Task A".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_b_id = manager.create_task("Task B".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_c_id = manager.create_task("Task C".to_string(), None, None, None, vec![], None, None).await.unwrap();
        let task_d_id = manager.create_task("Task D".to_string(), None, None, None, vec![], None, None).await.unwrap();

        manager.add_dependency(&task_b_id, &task_a_id).await.unwrap();
        manager.add_dependency(&task_c_id, &task_b_id).await.unwrap();
        manager.add_dependency(&task_d_id, &task_c_id).await.unwrap();

        // D cannot start (depends on C which depends on B which depends on A)
        assert!(!manager.can_start_task(&task_d_id).await.unwrap());

        // Complete A, B, C in order
        manager.update_task(&task_a_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
        assert!(!manager.can_start_task(&task_d_id).await.unwrap()); // Still can't start

        manager.update_task(&task_b_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
        assert!(!manager.can_start_task(&task_d_id).await.unwrap()); // Still can't start

        manager.update_task(&task_c_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();
        assert!(manager.can_start_task(&task_d_id).await.unwrap()); // Now can start!
    }

    #[tokio::test]
    async fn test_cannot_start_task_in_progress() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Task".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Mark as in progress
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None).await.unwrap();

        // Cannot start a task that's already in progress
        assert!(!manager.can_start_task(&task_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_cannot_start_completed_task() {
        let (manager, _temp_dir) = create_test_manager().await;

        let task_id = manager.create_task("Task".to_string(), None, None, None, vec![], None, None).await.unwrap();

        // Mark as done
        manager.update_task(&task_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None).await.unwrap();

        // Cannot start a completed task
        assert!(!manager.can_start_task(&task_id).await.unwrap());
    }
}
