use super::{MetricsCollector, MetricsStorage};
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

/// End-to-end test to verify metrics collection and storage works correctly
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_metrics_flow() -> Result<()> {
        // Setup
        let temp_dir = TempDir::new()?;
        let collector = Arc::new(MetricsCollector::new());
        let storage = Arc::new(MetricsStorage::new(temp_dir.path(), Some(30)).await?);

        // Verify directory was created
        assert!(temp_dir.path().exists(), "Metrics directory should be created");

        // Simulate some tool calls
        collector.record_tool_call("code.search_symbols", 15.5, true);
        collector.record_tokens("code.search_symbols", 100, 50);

        collector.record_tool_call("memory.find_similar_episodes", 25.3, true);
        collector.record_tokens("memory.find_similar_episodes", 150, 75);

        collector.record_tool_call("task.create_task", 5.2, true);
        collector.record_tokens("task.create_task", 50, 25);

        // Take a snapshot
        let snapshot1 = collector.take_snapshot();
        assert_eq!(snapshot1.tools.len(), 3, "Should have 3 tools recorded");

        // Save snapshot to storage
        storage.save_snapshot(&snapshot1).await?;

        // Verify snapshot was saved
        let count = storage.count_snapshots().await?;
        assert_eq!(count, 1, "Should have 1 snapshot saved");

        // Load the snapshot back
        let loaded = storage.load_snapshot(&snapshot1.timestamp).await?;
        assert!(loaded.is_some(), "Snapshot should be loadable");

        let loaded_snapshot = loaded.unwrap();
        assert_eq!(loaded_snapshot.tools.len(), 3, "Loaded snapshot should have 3 tools");

        // Verify tool metrics
        let search_metrics = loaded_snapshot.tools.get("code.search_symbols").unwrap();
        assert_eq!(search_metrics.total_calls, 1);
        assert_eq!(search_metrics.total_input_tokens, 100);
        assert_eq!(search_metrics.total_output_tokens, 50);

        // Simulate more activity
        collector.record_tool_call("code.search_symbols", 12.0, true);
        collector.record_tokens("code.search_symbols", 80, 40);

        // Take another snapshot
        let snapshot2 = collector.take_snapshot();
        storage.save_snapshot(&snapshot2).await?;

        // Verify we have 2 snapshots now
        let count = storage.count_snapshots().await?;
        assert_eq!(count, 2, "Should have 2 snapshots saved");

        // Verify cumulative metrics in second snapshot
        let search_metrics_2 = snapshot2.tools.get("code.search_symbols").unwrap();
        assert_eq!(search_metrics_2.total_calls, 2, "Should have 2 total calls");
        assert_eq!(search_metrics_2.total_input_tokens, 180, "Should have cumulative tokens");

        Ok(())
    }

    #[tokio::test]
    async fn test_background_snapshot_simulation() -> Result<()> {
        // This simulates what happens in production with the background task
        let temp_dir = TempDir::new()?;
        let collector = Arc::new(MetricsCollector::new());
        let storage = Arc::new(MetricsStorage::new(temp_dir.path(), Some(30)).await?);

        // Start a background task (similar to the server)
        let collector_clone = collector.clone();
        let storage_clone = storage.clone();
        let snapshot_task = tokio::spawn(async move {
            // Simulate 3 snapshots taken at intervals
            for _ in 0..3 {
                sleep(Duration::from_millis(100)).await;
                let snapshot = collector_clone.take_snapshot();
                storage_clone.save_snapshot(&snapshot).await.ok();
            }
        });

        // Simulate tool calls happening concurrently
        for i in 0..10 {
            collector.record_tool_call("test_tool", (i * 5) as f64, true);
            collector.record_tokens("test_tool", 50, 25);
            sleep(Duration::from_millis(50)).await;
        }

        // Wait for snapshot task to complete
        snapshot_task.await?;

        // Verify snapshots were saved
        let count = storage.count_snapshots().await?;
        assert!(count >= 3, "Should have at least 3 snapshots saved");

        // Verify we can query the time range
        let time_range = storage.get_time_range().await?;
        assert!(time_range.is_some(), "Should have a time range");

        Ok(())
    }

    #[tokio::test]
    async fn test_metrics_persistence_across_restarts() -> Result<()> {
        let temp_dir = TempDir::new()?;

        // First "session" - create collector, record data, save snapshot
        {
            let collector = Arc::new(MetricsCollector::new());
            let storage = Arc::new(MetricsStorage::new(temp_dir.path(), Some(30)).await?);

            collector.record_tool_call("session1_tool", 10.0, true);
            collector.record_tokens("session1_tool", 100, 50);

            let snapshot = collector.take_snapshot();
            storage.save_snapshot(&snapshot).await?;
        }

        // Simulate server restart - create new instances
        {
            let new_storage = Arc::new(MetricsStorage::new(temp_dir.path(), Some(30)).await?);

            // Verify previous snapshot is still there
            let count = new_storage.count_snapshots().await?;
            assert_eq!(count, 1, "Previous snapshot should persist across restart");

            // New collector starts fresh
            let new_collector = Arc::new(MetricsCollector::new());

            // Record new data
            new_collector.record_tool_call("session2_tool", 15.0, true);
            new_collector.record_tokens("session2_tool", 150, 75);

            // Save new snapshot
            let snapshot = new_collector.take_snapshot();
            new_storage.save_snapshot(&snapshot).await?;

            // Verify we now have 2 snapshots
            let count = new_storage.count_snapshots().await?;
            assert_eq!(count, 2, "Should have snapshots from both sessions");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_error_metrics_recording() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let collector = Arc::new(MetricsCollector::new());
        let storage = Arc::new(MetricsStorage::new(temp_dir.path(), Some(30)).await?);

        // Record successful and failed calls
        collector.record_tool_call("test_tool", 10.0, true);
        collector.record_tool_call("test_tool", 15.0, true);
        collector.record_tool_call("test_tool", 20.0, false);
        collector.record_tool_error("test_tool", 25.0, "timeout");
        collector.record_tool_error("test_tool", 30.0, "not_found");

        // Take snapshot and save
        let snapshot = collector.take_snapshot();
        storage.save_snapshot(&snapshot).await?;

        // Load and verify error tracking
        let loaded = storage.load_snapshot(&snapshot.timestamp).await?.unwrap();
        let metrics = loaded.tools.get("test_tool").unwrap();

        assert_eq!(metrics.total_calls, 5, "Should have 5 total calls");
        assert_eq!(metrics.success_count, 2, "Should have 2 successful calls");
        assert_eq!(metrics.error_count, 3, "Should have 3 failed calls");
        assert!((metrics.success_rate - 0.4).abs() < 0.01, "Success rate should be 40%");

        // Verify error breakdown
        assert_eq!(*metrics.error_breakdown.get("timeout").unwrap(), 1);
        assert_eq!(*metrics.error_breakdown.get("not_found").unwrap(), 1);
        assert_eq!(*metrics.error_breakdown.get("unknown").unwrap(), 1);

        Ok(())
    }
}
