// Integration tests for task tracking

#[cfg(test)]
mod integration_tests {
    use crate::tasks::{TaskManager, TaskStorage, Priority, SpecReference, TaskStatus};
    use crate::storage::MemoryStorage;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_setup() -> (TaskManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = MemoryStorage::new();
        let storage = Arc::new(TaskStorage::new(Arc::new(db)));
        let manager = TaskManager::new(storage);
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_full_task_lifecycle() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create task
        let task_id = manager
            .create_task(
                "Implement feature X".to_string(),
                Some("Add new API endpoint".to_string()),
                Some(Priority::High),
                Some(SpecReference {
                    spec_name: "api-spec".to_string(),
                    section: "Phase 1".to_string(),
                }),
                vec!["backend".to_string(), "api".to_string()],
                Some(4.0),
                None,
            )
            .await
            .unwrap();

        // Verify creation
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.title, "Implement feature X");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.tags.len(), 2);

        // Start work
        manager
            .update_task(
                &task_id,
                None,
                None,
                None,
                Some(TaskStatus::InProgress),
                Some("Starting implementation".to_string()),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.history.len(), 2);

        // Mark as blocked
        manager
            .update_task(
                &task_id,
                None,
                None,
                None,
                Some(TaskStatus::Blocked),
                Some("Waiting for review".to_string()),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Resume work
        manager
            .update_task(
                &task_id,
                None,
                None,
                None,
                Some(TaskStatus::InProgress),
                Some("Review complete".to_string()),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Complete task
        manager
            .update_task(
                &task_id,
                None,
                None,
                None,
                Some(TaskStatus::Done),
                Some("Feature implemented".to_string()),
                None,
                None,
                Some(3.5),
                Some("abc123".to_string()),
            )
            .await
            .unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
        assert_eq!(task.actual_hours, Some(3.5));
        assert_eq!(task.commit_hash, Some("abc123".to_string()));
        assert_eq!(task.history.len(), 5); // Created + 4 transitions
    }

    #[tokio::test]
    async fn test_multiple_tasks_filtering() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create tasks with different statuses
        let id1 = manager
            .create_task("Task 1".to_string(), None, Some(Priority::High), None, vec![], None, None)
            .await
            .unwrap();

        let id2 = manager
            .create_task("Task 2".to_string(), None, Some(Priority::Medium), None, vec![], None, None)
            .await
            .unwrap();

        let id3 = manager
            .create_task("Task 3".to_string(), None, Some(Priority::Low), None, vec![], None, None)
            .await
            .unwrap();

        // Update statuses
        manager
            .update_task(&id1, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None)
            .await
            .unwrap();

        manager
            .update_task(&id2, None, None, None, Some(TaskStatus::Done), None, None, None, None, None)
            .await
            .unwrap();

        // List all tasks
        let all_tasks = manager.list_tasks(None, None, None).await.unwrap();
        assert_eq!(all_tasks.len(), 3);

        // List in-progress tasks
        let in_progress = manager.list_tasks(Some(TaskStatus::InProgress), None, None).await.unwrap();
        assert_eq!(in_progress.len(), 1);
        assert_eq!(in_progress[0].id, id1);

        // List pending tasks
        let pending = manager.list_tasks(Some(TaskStatus::Pending), None, None).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id3);

        // List done tasks
        let done = manager.list_tasks(Some(TaskStatus::Done), None, None).await.unwrap();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, id2);
    }

    #[tokio::test]
    async fn test_spec_filtering() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create tasks with different specs
        let _id1 = manager
            .create_task(
                "Task 1".to_string(),
                None,
                None,
                Some(SpecReference {
                    spec_name: "spec-a".to_string(),
                    section: "Phase 1".to_string(),
                }),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let _id2 = manager
            .create_task(
                "Task 2".to_string(),
                None,
                None,
                Some(SpecReference {
                    spec_name: "spec-a".to_string(),
                    section: "Phase 2".to_string(),
                }),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        let _id3 = manager
            .create_task(
                "Task 3".to_string(),
                None,
                None,
                Some(SpecReference {
                    spec_name: "spec-b".to_string(),
                    section: "Phase 1".to_string(),
                }),
                vec![],
                None,
                None,
            )
            .await
            .unwrap();

        // List tasks by spec
        let spec_a_tasks = manager.list_tasks(None, Some("spec-a".to_string()), None).await.unwrap();
        assert_eq!(spec_a_tasks.len(), 2);

        let spec_b_tasks = manager.list_tasks(None, Some("spec-b".to_string()), None).await.unwrap();
        assert_eq!(spec_b_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_progress_stats() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create 10 tasks with various statuses
        for i in 0..10 {
            let id = manager
                .create_task(format!("Task {}", i), None, None, None, vec![], None, None)
                .await
                .unwrap();

            if i < 4 {
                // 4 done
                manager
                    .update_task(&id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None)
                    .await
                    .unwrap();
            } else if i < 6 {
                // 2 in progress
                manager
                    .update_task(&id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None)
                    .await
                    .unwrap();
            } else if i < 7 {
                // 1 blocked (must go through InProgress first)
                manager
                    .update_task(&id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None)
                    .await
                    .unwrap();
                manager
                    .update_task(&id, None, None, None, Some(TaskStatus::Blocked), None, None, None, None, None)
                    .await
                    .unwrap();
            }
            // Remaining 3 are pending
        }

        let stats = manager.get_progress(None).await.unwrap();
        assert_eq!(stats.total_tasks, 10);
        assert_eq!(stats.done, 4);
        assert_eq!(stats.in_progress, 2);
        assert_eq!(stats.blocked, 1);
        assert_eq!(stats.pending, 3);
        assert_eq!(stats.completion_percentage, 40.0);
    }

    #[tokio::test]
    async fn test_invalid_status_transitions() {
        let (manager, _temp_dir) = create_test_setup().await;

        let task_id = manager
            .create_task("Test".to_string(), None, None, None, vec![], None, None)
            .await
            .unwrap();

        // Mark as done
        manager
            .update_task(&task_id, None, None, None, Some(TaskStatus::Done), None, None, None, None, None)
            .await
            .unwrap();

        // Try to transition from done (should fail)
        let result = manager
            .update_task(&task_id, None, None, None, Some(TaskStatus::InProgress), None, None, None, None, None)
            .await;

        assert!(result.is_err());

        // Verify status unchanged
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let (manager, _temp_dir) = create_test_setup().await;

        let task_id = manager
            .create_task("To be deleted".to_string(), None, None, None, vec![], None, None)
            .await
            .unwrap();

        // Verify exists
        assert!(manager.get_task(&task_id).await.is_ok());

        // Delete
        manager.delete_task(&task_id).await.unwrap();

        // Verify deleted
        let result = manager.get_task(&task_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let (manager, _temp_dir) = create_test_setup().await;

        let task_id = manager
            .create_task("Cached task".to_string(), None, None, None, vec![], None, None)
            .await
            .unwrap();

        // First get (loads from storage, caches)
        let task1 = manager.get_task(&task_id).await.unwrap();

        // Second get (should come from cache)
        let task2 = manager.get_task(&task_id).await.unwrap();

        assert_eq!(task1.id, task2.id);
        assert_eq!(task1.title, task2.title);

        // Update task
        manager
            .update_task(
                &task_id,
                Some("Updated title".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Get again (cache should be updated)
        let task3 = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task3.title, "Updated title");
    }

    #[tokio::test]
    async fn test_limit_functionality() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create 20 tasks
        for i in 0..20 {
            manager
                .create_task(format!("Task {}", i), None, None, None, vec![], None, None)
                .await
                .unwrap();
        }

        // List with limit
        let limited = manager.list_tasks(None, None, Some(10)).await.unwrap();
        assert_eq!(limited.len(), 10);

        // List all
        let all = manager.list_tasks(None, None, None).await.unwrap();
        assert_eq!(all.len(), 20);
    }

    #[tokio::test]
    async fn test_task_history_tracking() {
        let (manager, _temp_dir) = create_test_setup().await;

        let task_id = manager
            .create_task("History test".to_string(), None, None, None, vec![], None, None)
            .await
            .unwrap();

        // Initial history (just creation)
        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.history.len(), 1);
        assert_eq!(task.history[0].to, TaskStatus::Pending);
        assert!(task.history[0].from.is_none());

        // Transition 1: Pending -> InProgress
        manager
            .update_task(
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
            )
            .await
            .unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.history.len(), 2);
        assert_eq!(task.history[1].from, Some(TaskStatus::Pending));
        assert_eq!(task.history[1].to, TaskStatus::InProgress);
        assert_eq!(task.history[1].note, Some("Starting work".to_string()));

        // Transition 2: InProgress -> Blocked
        manager
            .update_task(
                &task_id,
                None,
                None,
                None,
                Some(TaskStatus::Blocked),
                Some("Waiting on dependency".to_string()),
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let task = manager.get_task(&task_id).await.unwrap();
        assert_eq!(task.history.len(), 3);
        assert_eq!(task.history[2].from, Some(TaskStatus::InProgress));
        assert_eq!(task.history[2].to, TaskStatus::Blocked);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let (manager, _temp_dir) = create_test_setup().await;
        let manager = Arc::new(manager);

        // Create tasks concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let mgr = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                mgr.create_task(
                    format!("Concurrent task {}", i),
                    None,
                    None,
                    None,
                    vec![],
                    None,
                    None,
                )
                .await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all tasks created
        let all_tasks = manager.list_tasks(None, None, None).await.unwrap();
        assert_eq!(all_tasks.len(), 10);
    }

    #[tokio::test]
    async fn test_persistence_across_manager_instances() {
        let temp_dir = TempDir::new().unwrap();

        // Create first manager and add task
        let task_id = {
            let db = MemoryStorage::new();
            let storage = Arc::new(TaskStorage::new(Arc::new(db)));
            let manager = TaskManager::new(storage);

            manager
                .create_task("Persistent task".to_string(), None, None, None, vec![], None, None)
                .await
                .unwrap()
        };

        // Create second manager with same DB
        {
            let db = MemoryStorage::new();
            let storage = Arc::new(TaskStorage::new(Arc::new(db)));
            let manager = TaskManager::new(storage);

            // Should be able to load task
            let task = manager.get_task(&task_id).await.unwrap();
            assert_eq!(task.title, "Persistent task");
        }
    }

    #[tokio::test]
    async fn test_performance_create_1000_tasks() {
        let (manager, _temp_dir) = create_test_setup().await;

        let start = std::time::Instant::now();

        // Create 1000 tasks
        for i in 0..1000 {
            manager
                .create_task(
                    format!("Task {}", i),
                    Some(format!("Description for task {}", i)),
                    Some(Priority::Medium),
                    None,
                    vec![format!("tag{}", i % 10)],
                    None,
                    None,
                )
                .await
                .unwrap();
        }

        let duration = start.elapsed();
        println!("Created 1000 tasks in {:?}", duration);

        // Should complete in reasonable time (< 5 seconds)
        assert!(duration.as_secs() < 5);

        // Verify all created
        let all_tasks = manager.list_tasks(None, None, None).await.unwrap();
        assert_eq!(all_tasks.len(), 1000);
    }

    #[tokio::test]
    async fn test_performance_list_100_tasks() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create 1000 tasks with different statuses
        for i in 0..1000 {
            let task_id = manager
                .create_task(format!("Task {}", i), None, None, None, vec![], None, None)
                .await
                .unwrap();

            // Mark some as in-progress
            if i % 5 == 0 {
                manager
                    .update_task(
                        &task_id,
                        None,
                        None,
                        None,
                        Some(TaskStatus::InProgress),
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    .unwrap();
            }
        }

        let start = std::time::Instant::now();

        // List 100 tasks
        let tasks = manager.list_tasks(None, None, Some(100)).await.unwrap();

        let duration = start.elapsed();
        println!("Listed 100 tasks (from 1000) in {:?}", duration);

        // Should complete very fast (< 100ms)
        assert!(duration.as_millis() < 100);
        assert_eq!(tasks.len(), 100);
    }

    #[tokio::test]
    async fn test_performance_get_task_cached() {
        let (manager, _temp_dir) = create_test_setup().await;

        let task_id = manager
            .create_task("Test task".to_string(), None, None, None, vec![], None, None)
            .await
            .unwrap();

        // First get (from storage)
        let _ = manager.get_task(&task_id).await.unwrap();

        // Subsequent gets (from cache)
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = manager.get_task(&task_id).await.unwrap();
        }
        let duration = start.elapsed();

        println!("100 cached gets in {:?}", duration);

        // Should be very fast with cache (< 10ms)
        assert!(duration.as_millis() < 10);
    }

    #[tokio::test]
    async fn test_performance_progress_calculation() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create 1000 tasks with various specs and statuses
        for i in 0..1000 {
            let spec_ref = if i % 2 == 0 {
                Some(SpecReference {
                    spec_name: format!("spec{}", i % 5),
                    section: "Phase 1".to_string(),
                })
            } else {
                None
            };

            let task_id = manager
                .create_task(format!("Task {}", i), None, None, spec_ref, vec![], None, None)
                .await
                .unwrap();

            // Mark some as done
            if i % 3 == 0 {
                manager
                    .update_task(
                        &task_id,
                        None,
                        None,
                        None,
                        Some(TaskStatus::Done),
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .await
                    .unwrap();
            }
        }

        let start = std::time::Instant::now();
        let stats = manager.get_progress(None).await.unwrap();
        let duration = start.elapsed();

        println!("Progress calculation for 1000 tasks in {:?}", duration);

        // Should complete in reasonable time (< 200ms)
        assert!(duration.as_millis() < 200);
        assert_eq!(stats.total_tasks, 1000);
        assert!(stats.by_spec.len() > 0);
        assert!(stats.by_priority.len() > 0);
    }

    #[tokio::test]
    async fn test_performance_search_in_large_dataset() {
        let (manager, _temp_dir) = create_test_setup().await;

        // Create 1000 tasks with various titles
        for i in 0..1000 {
            manager
                .create_task(
                    format!("Task {} - {}", i, if i % 10 == 0 { "special" } else { "normal" }),
                    Some(format!("Description for task {}", i)),
                    None,
                    None,
                    vec![],
                    None,
                    None,
                )
                .await
                .unwrap();
        }

        let start = std::time::Instant::now();
        let results = manager.search_tasks("special", None).await.unwrap();
        let duration = start.elapsed();

        println!("Searched 1000 tasks for 'special' in {:?}", duration);

        // Should complete in reasonable time (< 100ms)
        assert!(duration.as_millis() < 100);
        assert_eq!(results.len(), 100); // Should find 100 "special" tasks
    }
}
