use super::types::MetricsSnapshot;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::sql::Datetime as SurrealDatetime;
use surrealdb::Surreal;

/// Default metrics database path relative to Meridian home (~/.meridian/db/current/metrics/)
pub const DEFAULT_METRICS_DB_PATH: &str = "db/current/metrics";

/// SurrealDB record for metrics snapshot
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotRecord {
    timestamp: SurrealDatetime,
    // Store snapshot as serde_json::Value to avoid nested datetime issues
    data: serde_json::Value,
}

/// SurrealDB record for metrics aggregation
#[derive(Debug, Serialize, Deserialize)]
struct AggregationRecord {
    granularity: String,
    timestamp: SurrealDatetime,
    data: serde_json::Value,
}

/// Count result for SurrealDB queries
#[derive(Debug, Deserialize)]
struct CountResult {
    count: u64,
}

/// Get default metrics database path
pub fn get_default_metrics_path() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".meridian").join(DEFAULT_METRICS_DB_PATH))
}

/// Metrics storage using SurrealDB with time-series optimization
///
/// This storage is completely separate from the main Meridian database to:
/// - Prevent metrics bloat from affecting main DB performance
/// - Allow independent tuning for time-series workloads
/// - Enable separate backup/restore strategies
/// - Isolate high-write metrics I/O from main database operations
///
/// Storage schema:
/// - Table: metrics_snapshots (timestamp, data)
/// - Table: metrics_aggregations (granularity, timestamp, data)
/// - Indexes: timestamp-based for efficient range queries
///
/// Optimizations:
/// - SurrealDB with RocksDB backend for LSM-tree benefits
/// - DateTime indexes for fast time-range queries
/// - SQL-like queries for flexible data retrieval
///
/// Retention policy:
/// - Automatically deletes snapshots older than retention period
/// - Default retention: 30 days
/// - Aggregated data has separate retention (90 days)
#[derive(Clone)]
pub struct MetricsStorage {
    db: Arc<Surreal<Db>>,
    retention_days: u32,
    aggregation_retention_days: u32,
}

impl MetricsStorage {
    /// Create a new metrics storage at default path
    ///
    /// # Arguments
    /// * `retention_days` - Number of days to retain raw snapshots (default: 30)
    pub async fn new_default(retention_days: Option<u32>) -> Result<Self> {
        let path = get_default_metrics_path()?;
        Self::new(&path, retention_days).await
    }

    /// Create a new metrics storage at custom path
    ///
    /// # Arguments
    /// * `path` - Path to SurrealDB directory (separate from main DB)
    /// * `retention_days` - Number of days to retain raw snapshots (default: 30)
    pub async fn new(path: &Path, retention_days: Option<u32>) -> Result<Self> {
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create metrics DB directory: {:?}", parent))?;
        }

        // Connect to embedded SurrealDB with RocksDB backend
        let db = Surreal::new::<RocksDb>(path)
            .await
            .context("Failed to initialize SurrealDB for metrics")?;

        // Use dedicated namespace and database for metrics
        db.use_ns("meridian")
            .use_db("metrics")
            .await
            .context("Failed to set namespace and database")?;

        let storage = Self {
            db: Arc::new(db),
            retention_days: retention_days.unwrap_or(30),
            aggregation_retention_days: 90, // Keep aggregations longer
        };

        // Initialize schema
        storage.initialize_schema().await?;

        Ok(storage)
    }

    /// Initialize the SurrealDB schema for metrics
    async fn initialize_schema(&self) -> Result<()> {
        self.db
            .query(
                r#"
            DEFINE TABLE IF NOT EXISTS metrics_snapshots SCHEMALESS;
            DEFINE FIELD IF NOT EXISTS timestamp ON TABLE metrics_snapshots TYPE datetime;
            DEFINE INDEX IF NOT EXISTS idx_timestamp ON TABLE metrics_snapshots COLUMNS timestamp;

            DEFINE TABLE IF NOT EXISTS metrics_aggregations SCHEMALESS;
            DEFINE FIELD IF NOT EXISTS granularity ON TABLE metrics_aggregations TYPE string;
            DEFINE FIELD IF NOT EXISTS timestamp ON TABLE metrics_aggregations TYPE datetime;
            DEFINE INDEX IF NOT EXISTS idx_granularity_timestamp
                ON TABLE metrics_aggregations COLUMNS granularity, timestamp;
        "#,
            )
            .await
            .context("Failed to initialize metrics schema")?;

        Ok(())
    }

    /// Get retention days for snapshots
    pub fn retention_days(&self) -> u32 {
        self.retention_days
    }

    /// Get retention days for aggregations
    pub fn aggregation_retention_days(&self) -> u32 {
        self.aggregation_retention_days
    }

    /// Save a metrics snapshot to storage
    pub async fn save_snapshot(&self, snapshot: &MetricsSnapshot) -> Result<()> {
        // Generate ID from timestamp (milliseconds since epoch)
        let id = format!("snapshot_{}", snapshot.timestamp.timestamp_millis());

        // Convert DateTime<Utc> to SurrealDB Datetime
        let timestamp = SurrealDatetime::from(snapshot.timestamp);

        // Serialize snapshot to JSON to avoid nested datetime issues
        let snapshot_json = serde_json::to_value(snapshot)
            .context("Failed to serialize snapshot to JSON")?;

        let record = SnapshotRecord {
            timestamp,
            data: snapshot_json,
        };

        let _: Option<SnapshotRecord> = self
            .db
            .create(("metrics_snapshots", id))
            .content(record)
            .await
            .context("Failed to save metrics snapshot")?;

        Ok(())
    }

    /// Load a single snapshot by timestamp
    pub async fn load_snapshot(&self, timestamp: &DateTime<Utc>) -> Result<Option<MetricsSnapshot>> {
        let timestamp = SurrealDatetime::from(*timestamp);
        let mut result = self
            .db
            .query(
                r#"
            SELECT * FROM metrics_snapshots
            WHERE timestamp = $timestamp
            LIMIT 1
        "#,
            )
            .bind(("timestamp", timestamp))
            .await
            .context("Failed to load metrics snapshot")?;

        let record: Option<SnapshotRecord> = result.take(0)?;
        match record {
            Some(r) => {
                let snapshot: MetricsSnapshot = serde_json::from_value(r.data)
                    .context("Failed to deserialize snapshot from JSON")?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }

    /// Load all snapshots within a time range
    ///
    /// # Arguments
    /// * `start` - Start of time range (inclusive)
    /// * `end` - End of time range (inclusive)
    pub async fn load_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MetricsSnapshot>> {
        let start = SurrealDatetime::from(start);
        let end = SurrealDatetime::from(end);
        let mut result = self
            .db
            .query(
                r#"
            SELECT * FROM metrics_snapshots
            WHERE timestamp >= $start AND timestamp <= $end
            ORDER BY timestamp ASC
        "#,
            )
            .bind(("start", start))
            .bind(("end", end))
            .await
            .context("Failed to load metrics range")?;

        let records: Vec<SnapshotRecord> = result.take(0)?;
        records
            .into_iter()
            .map(|r| {
                serde_json::from_value(r.data)
                    .context("Failed to deserialize snapshot from JSON")
            })
            .collect()
    }

    /// Delete snapshots older than retention period
    ///
    /// Returns the number of snapshots deleted
    pub async fn cleanup_old_snapshots(&self, custom_retention_days: Option<u32>) -> Result<u64> {
        let retention = custom_retention_days.unwrap_or(self.retention_days);
        let cutoff = Utc::now() - Duration::days(retention as i64);
        let cutoff = SurrealDatetime::from(cutoff);

        let mut result = self
            .db
            .query(
                r#"
            DELETE metrics_snapshots
            WHERE timestamp < $cutoff
            RETURN BEFORE
        "#,
            )
            .bind(("cutoff", cutoff))
            .await
            .context("Failed to cleanup old snapshots")?;

        let deleted: Vec<SnapshotRecord> = result.take(0)?;
        Ok(deleted.len() as u64)
    }

    /// Delete aggregations older than their retention period
    ///
    /// Returns the number of aggregations deleted
    pub async fn cleanup_old_aggregations(&self) -> Result<u64> {
        let cutoff = Utc::now() - Duration::days(self.aggregation_retention_days as i64);
        let cutoff = SurrealDatetime::from(cutoff);

        let mut result = self
            .db
            .query(
                r#"
            DELETE metrics_aggregations
            WHERE timestamp < $cutoff
            RETURN BEFORE
        "#,
            )
            .bind(("cutoff", cutoff))
            .await
            .context("Failed to cleanup old aggregations")?;

        let deleted: Vec<AggregationRecord> = result.take(0)?;
        Ok(deleted.len() as u64)
    }

    /// Cleanup all old data (snapshots and aggregations)
    ///
    /// Returns tuple of (snapshots_deleted, aggregations_deleted)
    pub async fn cleanup_all(&self) -> Result<(u64, u64)> {
        let snapshots_deleted = self.cleanup_old_snapshots(None).await?;
        let aggregations_deleted = self.cleanup_old_aggregations().await?;
        Ok((snapshots_deleted, aggregations_deleted))
    }

    /// Get the total number of snapshots stored
    pub async fn count_snapshots(&self) -> Result<u64> {
        let mut result = self
            .db
            .query(
                r#"
            SELECT count() AS count FROM metrics_snapshots GROUP ALL
        "#,
            )
            .await
            .context("Failed to count snapshots")?;

        let count: Option<CountResult> = result.take(0)?;
        Ok(count.map(|c| c.count).unwrap_or(0))
    }

    /// Get the total number of aggregations stored
    pub async fn count_aggregations(&self) -> Result<u64> {
        let mut result = self
            .db
            .query(
                r#"
            SELECT count() AS count FROM metrics_aggregations GROUP ALL
        "#,
            )
            .await
            .context("Failed to count aggregations")?;

        let count: Option<CountResult> = result.take(0)?;
        Ok(count.map(|c| c.count).unwrap_or(0))
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<MetricsStorageStats> {
        let snapshot_count = self.count_snapshots().await?;
        let aggregation_count = self.count_aggregations().await?;
        let time_range = self.get_time_range().await?;

        Ok(MetricsStorageStats {
            snapshot_count,
            aggregation_count,
            retention_days: self.retention_days,
            aggregation_retention_days: self.aggregation_retention_days,
            oldest_snapshot: time_range.map(|(first, _)| first),
            newest_snapshot: time_range.map(|(_, last)| last),
        })
    }

    /// Get the oldest and newest snapshot timestamps
    pub async fn get_time_range(&self) -> Result<Option<(DateTime<Utc>, DateTime<Utc>)>> {
        // Get oldest timestamp
        let mut oldest_result = self
            .db
            .query(
                r#"
            SELECT * FROM metrics_snapshots
            ORDER BY timestamp ASC
            LIMIT 1
        "#,
            )
            .await
            .context("Failed to get oldest snapshot")?;

        let oldest: Option<SnapshotRecord> = oldest_result.take(0)?;

        // Get newest timestamp
        let mut newest_result = self
            .db
            .query(
                r#"
            SELECT * FROM metrics_snapshots
            ORDER BY timestamp DESC
            LIMIT 1
        "#,
            )
            .await
            .context("Failed to get newest snapshot")?;

        let newest: Option<SnapshotRecord> = newest_result.take(0)?;

        Ok(match (oldest, newest) {
            (Some(o), Some(n)) => {
                // Convert SurrealDatetime to DateTime<Utc>
                let oldest_time = DateTime::<Utc>::from(o.timestamp);
                let newest_time = DateTime::<Utc>::from(n.timestamp);
                Some((oldest_time, newest_time))
            }
            _ => None,
        })
    }

    /// Delete all snapshots (useful for testing)
    #[cfg(test)]
    pub async fn clear_all(&self) -> Result<()> {
        self.db
            .query(
                r#"
            DELETE metrics_snapshots;
            DELETE metrics_aggregations;
        "#,
            )
            .await
            .context("Failed to clear all metrics")?;

        Ok(())
    }
}

/// Statistics about the metrics storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsStorageStats {
    pub snapshot_count: u64,
    pub aggregation_count: u64,
    pub retention_days: u32,
    pub aggregation_retention_days: u32,
    pub oldest_snapshot: Option<DateTime<Utc>>,
    pub newest_snapshot: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::types::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_snapshot(timestamp: DateTime<Utc>) -> MetricsSnapshot {
        MetricsSnapshot {
            timestamp,
            tools: HashMap::new(),
            memory: MemoryMetricsSnapshot {
                total_episodes: 10,
                episodes_last_24h: 5,
                avg_episode_usefulness: 0.8,
                cache_hit_rate: 0.75,
                cache_size_mb: 100.0,
                prefetch_accuracy: 0.9,
                total_patterns: 20,
                knowledge_graph_nodes: 50,
                total_procedures: 15,
                procedure_success_rate: 0.95,
            },
            search: SearchMetricsSnapshot {
                total_queries: 100,
                semantic_queries: 60,
                text_queries: 40,
                avg_query_latency_ms: 15.5,
                avg_results_returned: 10.0,
                rerank_calls: 30,
                avg_rerank_latency_ms: 5.2,
            },
            sessions: SessionMetricsSnapshot {
                total_sessions: 5,
                active_sessions: 2,
                avg_session_duration_minutes: 45.0,
                queries_per_session: 20.0,
            },
            tokens: TokenEfficiencyMetricsSnapshot {
                total_input_tokens: 10000,
                total_output_tokens: 5000,
                tokens_saved_compression: 2000,
                tokens_saved_deduplication: 500,
                avg_compression_ratio: 0.7,
            },
            system: SystemMetricsSnapshot {
                cpu_usage_percent: 45.0,
                memory_usage_mb: 512.0,
                disk_usage_mb: 1024.0,
                uptime_seconds: 3600,
            },
        }
    }

    #[tokio::test]
    async fn test_save_and_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();

        let snapshot = create_test_snapshot(Utc::now());
        storage.save_snapshot(&snapshot).await.unwrap();

        let loaded = storage
            .load_snapshot(&snapshot.timestamp)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.memory.total_episodes, 10);
        assert_eq!(loaded.search.total_queries, 100);
    }

    #[tokio::test]
    async fn test_load_range() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();

        let now = Utc::now();
        let mut timestamps = Vec::new();

        // Create snapshots at different times
        for i in 0..5 {
            let ts = now - Duration::hours(i);
            timestamps.push(ts);
            let snapshot = create_test_snapshot(ts);
            storage.save_snapshot(&snapshot).await.unwrap();
        }

        // Load range covering middle 3 snapshots
        let start = now - Duration::hours(3);
        let end = now - Duration::hours(1);
        let range = storage.load_range(start, end).await.unwrap();

        assert_eq!(range.len(), 3);
    }

    #[tokio::test]
    async fn test_cleanup_old() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(7)).await.unwrap();

        let now = Utc::now();

        // Create old snapshots
        for i in 0..5 {
            let ts = now - Duration::days(10 + i);
            let snapshot = create_test_snapshot(ts);
            storage.save_snapshot(&snapshot).await.unwrap();
        }

        // Create recent snapshots
        for i in 0..3 {
            let ts = now - Duration::days(i);
            let snapshot = create_test_snapshot(ts);
            storage.save_snapshot(&snapshot).await.unwrap();
        }

        // Should have 8 total
        let count_before = storage.count_snapshots().await.unwrap();
        assert_eq!(count_before, 8);

        // Cleanup with 7 day retention
        let deleted = storage.cleanup_old_snapshots(Some(7)).await.unwrap();
        assert_eq!(deleted, 5);

        // Should have 3 left
        let count_after = storage.count_snapshots().await.unwrap();
        assert_eq!(count_after, 3);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();

        let now = Utc::now();
        for i in 0..5 {
            let ts = now - Duration::hours(i);
            storage.save_snapshot(&create_test_snapshot(ts)).await.unwrap();
        }

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.snapshot_count, 5);
        assert_eq!(stats.retention_days, 30);
        assert_eq!(stats.aggregation_retention_days, 90);
        assert!(stats.oldest_snapshot.is_some());
        assert!(stats.newest_snapshot.is_some());
    }

    #[tokio::test]
    async fn test_get_time_range() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();

        let now = Utc::now();
        let oldest = now - Duration::hours(10);
        let newest = now;

        // Create snapshots
        storage.save_snapshot(&create_test_snapshot(oldest)).await.unwrap();
        storage.save_snapshot(&create_test_snapshot(now - Duration::hours(5))).await.unwrap();
        storage.save_snapshot(&create_test_snapshot(newest)).await.unwrap();

        let (first, last) = storage.get_time_range().await.unwrap().unwrap();

        // Allow small timing differences (within 1 second)
        assert!((first.timestamp() - oldest.timestamp()).abs() < 1);
        assert!((last.timestamp() - newest.timestamp()).abs() < 1);
    }

    #[tokio::test]
    async fn test_count_snapshots() {
        let temp_dir = TempDir::new().unwrap();
        let storage = MetricsStorage::new(temp_dir.path(), Some(30)).await.unwrap();

        assert_eq!(storage.count_snapshots().await.unwrap(), 0);

        for i in 0..10 {
            let ts = Utc::now() - Duration::hours(i);
            storage.save_snapshot(&create_test_snapshot(ts)).await.unwrap();
        }

        assert_eq!(storage.count_snapshots().await.unwrap(), 10);
    }
}
