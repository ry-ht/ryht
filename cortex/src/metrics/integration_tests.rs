/// Integration tests for metrics system with MCP server
#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::mcp::server::MeridianServer;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::SystemTime;
    use tempfile::TempDir;

    async fn create_test_server() -> (MeridianServer, TempDir) {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let temp_dir = TempDir::new().unwrap();
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = temp_dir
            .path()
            .join(format!("metrics_test_{}_{}", timestamp, counter));
        std::fs::create_dir_all(&db_path).unwrap();

        let config = Config {
            index: crate::config::IndexConfig {
                languages: vec!["rust".to_string()],
                ignore: vec![],
                max_file_size: "1MB".to_string(),
            },
            storage: crate::config::StorageConfig {
                path: db_path,
                cache_size: "256MB".to_string(),
                hnsw_index_path: None,
            },
            memory: crate::config::MemoryConfig {
                episodic_retention_days: 30,
                working_memory_size: "10MB".to_string(),
                consolidation_interval: "1h".to_string(),
            },
            session: crate::config::SessionConfig {
                max_sessions: 10,
                session_timeout: "1h".to_string(),
            },
            monorepo: crate::config::MonorepoConfig::default(),
            learning: crate::config::LearningConfig::default(),
            mcp: crate::config::McpConfig::default(),
        };

        let server = MeridianServer::new(config).await.unwrap();
        (server, temp_dir)
    }

    #[tokio::test]
    async fn test_metrics_collector_integration() {
        let (server, _temp) = create_test_server().await;

        // Get metrics collector
        let collector = server.get_metrics_collector().expect("Should have metrics collector");

        // Record some tool calls
        collector.record_tool_call("code.search_symbols", 15.5, true);
        collector.record_tool_call("code.search_symbols", 20.3, true);
        collector.record_tool_call("code.search_symbols", 100.0, false);

        collector.record_tokens("code.search_symbols", 100, 50);
        collector.record_tokens("code.search_symbols", 200, 75);

        // Take snapshot
        let snapshot = collector.take_snapshot();

        // Verify metrics were recorded
        assert!(snapshot.tools.contains_key("code.search_symbols"));
        let metrics = &snapshot.tools["code.search_symbols"];
        assert_eq!(metrics.total_calls, 3);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.error_count, 1);
        assert_eq!(metrics.total_input_tokens, 300);
        assert_eq!(metrics.total_output_tokens, 125);

        // Verify success rate
        assert!((metrics.success_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_metrics_storage_integration() {
        let (server, _temp) = create_test_server().await;

        // Get metrics collector
        let collector = server.get_metrics_collector().expect("Should have metrics collector");

        // Record multiple tool calls
        for i in 0..10 {
            collector.record_tool_call("test_tool", (i as f64) * 10.0, i % 2 == 0);
            collector.record_tokens("test_tool", 100, 50);
        }

        // Take snapshot
        let snapshot = collector.take_snapshot();

        // Verify metrics
        let metrics = snapshot.tools.get("test_tool").unwrap();
        assert_eq!(metrics.total_calls, 10);
        assert_eq!(metrics.success_count, 5);
        assert_eq!(metrics.error_count, 5);

        // Note: Storage is tested separately in storage module tests
        // Here we just verify the collector is properly integrated
    }

    #[tokio::test]
    async fn test_multiple_tools_tracking() {
        let (server, _temp) = create_test_server().await;

        let collector = server.get_metrics_collector().expect("Should have metrics collector");

        // Record calls to multiple different tools
        let tools = vec![
            "code.search_symbols",
            "code.get_definition",
            "task.create_task",
            "memory.record_episode",
            "specs.search",
        ];

        for tool in &tools {
            collector.record_tool_call(tool, 25.0, true);
            collector.record_tokens(tool, 100, 200);
        }

        let snapshot = collector.take_snapshot();

        // Verify all tools were tracked
        assert_eq!(snapshot.tools.len(), tools.len());
        for tool in &tools {
            assert!(snapshot.tools.contains_key(*tool));
            let metrics = &snapshot.tools[*tool];
            assert_eq!(metrics.total_calls, 1);
            assert_eq!(metrics.success_count, 1);
        }
    }

    #[tokio::test]
    async fn test_error_tracking() {
        let (server, _temp) = create_test_server().await;

        let collector = server.get_metrics_collector().expect("Should have metrics collector");

        // Record different types of errors
        collector.record_tool_error("test_tool", 10.0, "timeout");
        collector.record_tool_error("test_tool", 15.0, "timeout");
        collector.record_tool_error("test_tool", 20.0, "not_found");
        collector.record_tool_error("test_tool", 25.0, "permission_denied");

        let snapshot = collector.take_snapshot();
        let metrics = snapshot.tools.get("test_tool").unwrap();

        // Verify error breakdown
        assert_eq!(metrics.error_count, 4);
        assert_eq!(*metrics.error_breakdown.get("timeout").unwrap(), 2);
        assert_eq!(*metrics.error_breakdown.get("not_found").unwrap(), 1);
        assert_eq!(*metrics.error_breakdown.get("permission_denied").unwrap(), 1);
    }

    #[tokio::test]
    async fn test_latency_histogram() {
        let (server, _temp) = create_test_server().await;

        let collector = server.get_metrics_collector().expect("Should have metrics collector");

        // Record various latencies
        let latencies = vec![5.0, 10.0, 15.0, 25.0, 50.0, 100.0, 200.0, 500.0];
        for latency in latencies {
            collector.record_tool_call("test_tool", latency, true);
        }

        let snapshot = collector.take_snapshot();
        let metrics = snapshot.tools.get("test_tool").unwrap();
        let hist = &metrics.latency_histogram;

        // Verify histogram captured all values
        assert_eq!(hist.count, 8);
        assert!(hist.mean() > 0.0);
        assert!(hist.p50() > 0.0);
        assert!(hist.p95() > 0.0);
        assert!(hist.p99() > 0.0);

        // Verify percentiles are in expected ranges
        assert!(hist.p50() >= 15.0 && hist.p50() <= 50.0);
        assert!(hist.p95() >= 200.0);
    }
}
