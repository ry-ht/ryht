use super::*;
use chrono::Utc;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

#[test]
fn test_histogram_basic() {
    let mut hist = Histogram::new();

    // Record some values
    hist.observe(5.0);
    hist.observe(10.0);
    hist.observe(15.0);
    hist.observe(25.0);
    hist.observe(50.0);

    assert_eq!(hist.count, 5);
    assert_eq!(hist.sum, 105.0);
    assert_eq!(hist.mean(), 21.0);
}

#[test]
fn test_histogram_percentiles() {
    let mut hist = Histogram::new();

    // Record 100 values: 1ms to 100ms
    for i in 1..=100 {
        hist.observe(i as f64);
    }

    assert_eq!(hist.count, 100);

    // Test percentiles (approximate due to bucketing)
    let p50 = hist.p50();
    let p95 = hist.p95();
    let p99 = hist.p99();

    // p50 should be around 50ms
    assert!(p50 >= 25.0 && p50 <= 100.0);

    // p95 should be higher than p50
    assert!(p95 > p50);

    // p99 should be higher than p95
    assert!(p99 >= p95);
}

#[test]
fn test_concurrent_histogram() {
    let hist = ConcurrentHistogram::new();
    let hist_clone = hist.clone();

    // Record from multiple threads
    let handle = thread::spawn(move || {
        for i in 0..100 {
            hist_clone.observe(i as f64);
        }
    });

    for i in 100..200 {
        hist.observe(i as f64);
    }

    handle.join().unwrap();

    let snapshot = hist.snapshot();
    assert_eq!(snapshot.count, 200);
}

#[test]
fn test_tool_metrics_record_success() {
    let metrics = ToolMetrics::new();

    metrics.record_success(10.5);
    metrics.record_success(15.2);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_calls, 2);
    assert_eq!(snapshot.success_count, 2);
    assert_eq!(snapshot.error_count, 0);
    assert_eq!(snapshot.success_rate, 1.0);
}

#[test]
fn test_tool_metrics_record_error() {
    let metrics = ToolMetrics::new();

    metrics.record_error(10.0, "timeout");
    metrics.record_error(15.0, "timeout");
    metrics.record_error(20.0, "not_found");

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_calls, 3);
    assert_eq!(snapshot.success_count, 0);
    assert_eq!(snapshot.error_count, 3);
    assert_eq!(snapshot.success_rate, 0.0);
    assert_eq!(*snapshot.error_breakdown.get("timeout").unwrap(), 2);
    assert_eq!(*snapshot.error_breakdown.get("not_found").unwrap(), 1);
}

#[test]
fn test_tool_metrics_mixed() {
    let metrics = ToolMetrics::new();

    metrics.record_success(10.0);
    metrics.record_success(15.0);
    metrics.record_error(20.0, "timeout");

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_calls, 3);
    assert_eq!(snapshot.success_count, 2);
    assert_eq!(snapshot.error_count, 1);
    assert!((snapshot.success_rate - 0.666).abs() < 0.01);
}

#[test]
fn test_tool_metrics_tokens() {
    let metrics = ToolMetrics::new();

    metrics.record_tokens(100, 50);
    metrics.record_tokens(200, 75);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_input_tokens, 300);
    assert_eq!(snapshot.total_output_tokens, 125);
}

#[test]
fn test_metrics_collector_basic() {
    let collector = MetricsCollector::new();

    collector.record_tool_call("test_tool", 10.5, true);
    collector.record_tool_call("test_tool", 15.2, true);

    let metrics = collector.get_tool_metrics("test_tool").unwrap();
    assert_eq!(metrics.total_calls, 2);
    assert_eq!(metrics.success_count, 2);
}

#[test]
fn test_metrics_collector_multiple_tools() {
    let collector = MetricsCollector::new();

    collector.record_tool_call("tool1", 10.0, true);
    collector.record_tool_call("tool2", 20.0, true);
    collector.record_tool_call("tool1", 15.0, false);

    let tool1 = collector.get_tool_metrics("tool1").unwrap();
    let tool2 = collector.get_tool_metrics("tool2").unwrap();

    assert_eq!(tool1.total_calls, 2);
    assert_eq!(tool2.total_calls, 1);
}

#[test]
fn test_metrics_collector_snapshot() {
    let collector = MetricsCollector::new();

    collector.record_tool_call("tool1", 10.0, true);
    collector.record_tool_call("tool2", 20.0, true);

    let snapshot = collector.take_snapshot();

    assert!(snapshot.tools.contains_key("tool1"));
    assert!(snapshot.tools.contains_key("tool2"));
    assert!(snapshot.timestamp <= Utc::now());
}

#[test]
fn test_metrics_collector_concurrent() {
    let collector = Arc::new(MetricsCollector::new());
    let mut handles = vec![];

    // Spawn 10 threads, each making 100 calls
    for i in 0..10 {
        let collector = Arc::clone(&collector);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                collector.record_tool_call(
                    &format!("tool_{}", i % 3),
                    (j as f64) * 1.5,
                    j % 2 == 0,
                );
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify total calls
    let snapshot = collector.take_snapshot();
    let total_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
    assert_eq!(total_calls, 1000);
}

#[tokio::test]
async fn test_storage_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();
    let collector = MetricsCollector::new();

    collector.record_tool_call("test_tool", 10.0, true);

    let snapshot = collector.take_snapshot();
    storage.save_snapshot(&snapshot).await.unwrap();

    let loaded = storage.load_snapshot(&snapshot.timestamp).await.unwrap();
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert!(loaded.tools.contains_key("test_tool"));
}

#[tokio::test]
async fn test_storage_retention() {
    use chrono::Duration;

    let temp_dir = TempDir::new().unwrap();
    let storage = MetricsStorage::new(temp_dir.path(), Some(7)).await.unwrap();
    let collector = MetricsCollector::new();

    let now = Utc::now();

    // Create old snapshots
    for i in 0..5 {
        let mut snapshot = collector.take_snapshot();
        snapshot.timestamp = now - Duration::days(10 + i);
        storage.save_snapshot(&snapshot).await.unwrap();
    }

    // Create recent snapshots
    for i in 0..3 {
        let mut snapshot = collector.take_snapshot();
        snapshot.timestamp = now - Duration::days(i);
        storage.save_snapshot(&snapshot).await.unwrap();
    }

    // Cleanup old snapshots
    let deleted = storage.cleanup_old_snapshots(Some(7)).await.unwrap();
    assert_eq!(deleted, 5);

    let count = storage.count_snapshots().await.unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_memory_metrics() {
    let metrics = MemoryMetrics::new();

    use std::sync::atomic::Ordering;

    metrics.total_episodes.fetch_add(10, Ordering::Relaxed);
    metrics.episodes_last_24h.fetch_add(5, Ordering::Relaxed);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_episodes, 10);
    assert_eq!(snapshot.episodes_last_24h, 5);
}

#[test]
fn test_search_metrics() {
    let metrics = SearchMetrics::new();

    use std::sync::atomic::Ordering;

    metrics.total_queries.fetch_add(100, Ordering::Relaxed);
    metrics.semantic_queries.fetch_add(60, Ordering::Relaxed);
    metrics.text_queries.fetch_add(40, Ordering::Relaxed);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_queries, 100);
    assert_eq!(snapshot.semantic_queries, 60);
    assert_eq!(snapshot.text_queries, 40);
}

#[test]
fn test_session_metrics() {
    let metrics = SessionMetrics::new();

    use std::sync::atomic::Ordering;

    metrics.total_sessions.fetch_add(5, Ordering::Relaxed);
    metrics.active_sessions.fetch_add(2, Ordering::Relaxed);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_sessions, 5);
    assert_eq!(snapshot.active_sessions, 2);
}

#[test]
fn test_token_efficiency_metrics() {
    let metrics = TokenEfficiencyMetrics::new();

    use std::sync::atomic::Ordering;

    metrics.total_input_tokens.fetch_add(10000, Ordering::Relaxed);
    metrics.total_output_tokens.fetch_add(5000, Ordering::Relaxed);
    metrics.tokens_saved_compression.fetch_add(2000, Ordering::Relaxed);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_input_tokens, 10000);
    assert_eq!(snapshot.total_output_tokens, 5000);
    assert_eq!(snapshot.tokens_saved_compression, 2000);
}

#[test]
fn test_system_metrics() {
    let metrics = SystemMetrics::new();

    *metrics.cpu_usage_percent.write() = 45.5;
    *metrics.memory_usage_mb.write() = 512.0;

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.cpu_usage_percent, 45.5);
    assert_eq!(snapshot.memory_usage_mb, 512.0);
}

// ============================================================================
// Self-Improvement Metrics Tests
// ============================================================================

#[test]
fn test_self_improvement_metrics_new() {
    use super::self_improvement::SelfImprovementMetrics;

    let metrics = SelfImprovementMetrics::new();

    assert_eq!(metrics.health_score, 0.0);
    assert_eq!(metrics.code_quality_score, 0.0);
    assert_eq!(metrics.test_coverage_percent, 0.0);
    assert_eq!(metrics.circular_dependencies_count, 0);
    assert!(metrics.language_breakdown.is_empty());
}

#[test]
fn test_health_score_calculation() {
    use super::self_improvement::SelfImprovementMetrics;

    let mut metrics = SelfImprovementMetrics::new();

    // Perfect scores should give health score of 1.0
    metrics.code_quality_score = 1.0;
    metrics.test_coverage_percent = 100.0;
    metrics.technical_debt_score = 0.0;
    metrics.avg_cyclomatic_complexity = 3.0;

    metrics.calculate_health_score();
    assert!((metrics.health_score - 1.0).abs() < 0.01, "Health score should be ~1.0, got {}", metrics.health_score);
}

#[test]
fn test_health_score_poor_quality() {
    use super::self_improvement::SelfImprovementMetrics;

    let mut metrics = SelfImprovementMetrics::new();

    // Poor scores
    metrics.code_quality_score = 0.3;
    metrics.test_coverage_percent = 20.0;
    metrics.technical_debt_score = 0.8;
    metrics.avg_cyclomatic_complexity = 15.0;

    metrics.calculate_health_score();
    assert!(metrics.health_score < 0.5, "Health score should be low, got {}", metrics.health_score);
}

#[test]
fn test_technical_debt_score_calculation() {
    use super::self_improvement::SelfImprovementMetrics;

    let mut metrics = SelfImprovementMetrics::new();

    metrics.untested_symbols_count = 50;
    metrics.undocumented_symbols_count = 30;
    metrics.high_complexity_symbols_count = 20;
    metrics.circular_dependencies_count = 5;

    let total_symbols = 100;
    metrics.calculate_technical_debt(total_symbols);
    assert!(metrics.technical_debt_score > 0.0 && metrics.technical_debt_score <= 1.0,
            "Technical debt should be in range [0,1], got {}", metrics.technical_debt_score);
}

#[test]
fn test_trend_analysis() {
    use super::self_improvement::{SelfImprovementMetrics, TrendDirection};

    let current = SelfImprovementMetrics::new();

    // Improving case
    let mut previous = SelfImprovementMetrics::new();
    previous.health_score = 0.5;
    let mut improving = current.clone();
    improving.health_score = 0.7;
    improving.calculate_trend(Some(&previous));
    assert_eq!(improving.trend_direction, TrendDirection::Improving);

    // Degrading case
    let mut degrading = current.clone();
    degrading.health_score = 0.3;
    degrading.calculate_trend(Some(&previous));
    assert_eq!(degrading.trend_direction, TrendDirection::Degrading);

    // Stable case
    let mut stable = previous.clone();
    stable.calculate_trend(Some(&previous));
    assert_eq!(stable.trend_direction, TrendDirection::Stable);
}

#[test]
fn test_language_metrics() {
    use super::self_improvement::{LanguageMetrics, SelfImprovementMetrics};
    use std::collections::HashMap;

    let mut metrics = SelfImprovementMetrics::new();

    let rust_metrics = LanguageMetrics {
        language: "rust".to_string(),
        symbol_count: 1000,
        avg_complexity: 4.5,
        test_coverage_percent: 75.0,
        health_score: 0.85,
    };

    let ts_metrics = LanguageMetrics {
        language: "typescript".to_string(),
        symbol_count: 500,
        avg_complexity: 6.2,
        test_coverage_percent: 60.0,
        health_score: 0.70,
    };

    let mut breakdown = HashMap::new();
    breakdown.insert("rust".to_string(), rust_metrics);
    breakdown.insert("typescript".to_string(), ts_metrics);

    metrics.language_breakdown = breakdown;

    assert_eq!(metrics.language_breakdown.len(), 2);
    assert_eq!(metrics.language_breakdown.get("rust").unwrap().symbol_count, 1000);
    assert_eq!(metrics.language_breakdown.get("typescript").unwrap().symbol_count, 500);
}

#[test]
fn test_code_quality_score_components() {
    use super::self_improvement::SelfImprovementMetrics;

    let mut metrics = SelfImprovementMetrics::new();
    let total_symbols = 100;

    // Low complexity, good documentation, no circular deps
    metrics.avg_cyclomatic_complexity = 3.0;
    metrics.undocumented_symbols_count = 5;
    metrics.high_complexity_symbols_count = 2;
    metrics.circular_dependencies_count = 0;

    metrics.calculate_code_quality(total_symbols);
    assert!(metrics.code_quality_score > 0.7, "Quality should be high with good metrics, got {}", metrics.code_quality_score);

    // High complexity, poor documentation, circular deps
    metrics.avg_cyclomatic_complexity = 15.0;
    metrics.undocumented_symbols_count = 80;
    metrics.high_complexity_symbols_count = 60;
    metrics.circular_dependencies_count = 10;

    metrics.calculate_code_quality(total_symbols);
    assert!(metrics.code_quality_score < 0.5, "Quality should be low with poor metrics, got {}", metrics.code_quality_score);
}

#[test]
fn test_improvement_velocity() {
    use super::self_improvement::SelfImprovementMetrics;

    let mut metrics = SelfImprovementMetrics::new();

    // Test improvements tracking
    metrics.improvements_per_week = 10;
    metrics.avg_improvement_time_hours = 4.0;

    // Velocity formula: improvements / time
    let velocity = metrics.improvements_per_week as f64 / metrics.avg_improvement_time_hours;
    assert!(velocity > 0.0, "Velocity should be positive");

    // More improvements with less time = higher velocity
    metrics.improvements_per_week = 20;
    metrics.avg_improvement_time_hours = 2.0;

    let higher_velocity = metrics.improvements_per_week as f64 / metrics.avg_improvement_time_hours;
    assert!(higher_velocity > velocity, "More improvements faster should increase velocity");
}

#[test]
fn test_metrics_serialization() {
    use super::self_improvement::SelfImprovementMetrics;

    let metrics = SelfImprovementMetrics::new();

    // Serialize to JSON
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());

    // Deserialize back
    let deserialized: SelfImprovementMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.health_score, metrics.health_score);
    assert_eq!(deserialized.test_coverage_percent, metrics.test_coverage_percent);
}
