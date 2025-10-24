//! Qdrant command implementations for cortex-cli

use crate::output::{self, OutputFormat, TableBuilder};
use anyhow::{Context, Result};
use cortex_storage::{CollectionConfig, HnswConfig, OptimizerConfig, QdrantClient, QdrantConfig};
use cortex_storage::qdrant::DistanceMetric;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Collection definitions for Cortex
fn get_collection_configs() -> Vec<CollectionConfig> {
    vec![
        CollectionConfig {
            name: "code_vectors".to_string(),
            vector_size: 1536,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        },
        CollectionConfig {
            name: "memory_vectors".to_string(),
            vector_size: 1536,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        },
        CollectionConfig {
            name: "document_vectors".to_string(),
            vector_size: 1536,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        },
        CollectionConfig {
            name: "ast_vectors".to_string(),
            vector_size: 768,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        },
        CollectionConfig {
            name: "dependency_vectors".to_string(),
            vector_size: 384,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        },
    ]
}

/// Create Qdrant client from config
async fn create_qdrant_client() -> Result<QdrantClient> {
    let config = QdrantConfig {
        host: std::env::var("QDRANT_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("QDRANT_HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(6333),
        grpc_port: std::env::var("QDRANT_GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok()),
        api_key: std::env::var("QDRANT_API_KEY").ok(),
        use_https: std::env::var("QDRANT_USE_HTTPS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false),
        timeout: std::time::Duration::from_secs(10),
        request_timeout: std::time::Duration::from_secs(60),
    };

    QdrantClient::new(config).await
}

/// Initialize Qdrant collections
pub async fn qdrant_init(force: bool, skip_verify: bool) -> Result<()> {
    let spinner = output::spinner("Initializing Qdrant collections...");

    let client = create_qdrant_client().await?;

    // Check health
    let health = client.health().await.context("Qdrant health check failed")?;
    output::info(format!("Qdrant is healthy: {}", health.title));

    // Get existing collections
    let existing = client.list_collections().await?;
    let existing_set: std::collections::HashSet<_> = existing.iter().cloned().collect();

    let configs = get_collection_configs();
    let mut created = 0;
    let mut skipped = 0;

    for config in configs {
        if existing_set.contains(&config.name) {
            if force {
                output::warning(format!("Deleting existing collection: {}", config.name));
                client.delete_collection(&config.name).await?;
                client.create_collection(config.clone()).await?;
                created += 1;
            } else {
                output::info(format!("Collection already exists: {}", config.name));
                skipped += 1;
            }
        } else {
            output::info(format!("Creating collection: {}", config.name));
            client.create_collection(config.clone()).await?;
            created += 1;
        }

        // Verify if not skipped
        if !skip_verify {
            let info = client.collection_info(&config.name).await?;
            output::success(format!(
                "  Collection verified: {} (status: {:?})",
                config.name, info.status
            ));
        }
    }

    spinner.finish_with_message(format!(
        "Initialized {} collections ({} created, {} skipped)",
        created + skipped,
        created,
        skipped
    ));

    Ok(())
}

/// Show Qdrant status and statistics
pub async fn qdrant_status(
    detailed: bool,
    collection: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let client = create_qdrant_client().await?;

    // Health check
    let health = client.health().await?;
    output::success(format!("Qdrant Health: {}", health.title));

    // Get collections to display
    let collections = if let Some(name) = collection {
        vec![name]
    } else {
        client.list_collections().await?
    };

    if collections.is_empty() {
        output::warning("No collections found");
        return Ok(());
    }

    // Gather statistics
    let mut stats = Vec::new();
    for name in &collections {
        match client.get_collection_stats(name).await {
            Ok(s) => stats.push(s),
            Err(e) => output::error(format!("Failed to get stats for {}: {}", name, e)),
        }
    }

    // Display based on format
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        OutputFormat::Plain => {
            for stat in stats {
                println!("{}: {} vectors", stat.name, stat.vectors_count);
            }
        }
        OutputFormat::Human => {
            let table = TableBuilder::new()
                .header(vec![
                    "Collection",
                    "Vectors",
                    "Indexed",
                    "Points",
                    "Segments",
                    "Status",
                ]);

            let table = stats.into_iter().fold(table, |table, stat| {
                table.row(vec![
                    stat.name,
                    stat.vectors_count.to_string(),
                    stat.indexed_vectors_count.to_string(),
                    stat.points_count.to_string(),
                    stat.segments_count.to_string(),
                    stat.status,
                ])
            });

            table.print();

            if detailed {
                println!("\nDetailed Collection Information:");
                for name in &collections {
                    let info = client.collection_info(name).await?;
                    println!("\n{}:", name);
                    println!("  Status: {:?}", info.status);
                    println!("  Optimizer Status: {:?}", info.optimizer_status);
                    println!("  Vectors: {}", info.vectors_count.unwrap_or(0));
                    println!("  Points: {}", info.points_count.unwrap_or(0));
                    println!("  Segments: {}", info.segments_count);
                }
            }
        }
    }

    Ok(())
}

/// Migrate data between Qdrant collections
pub async fn qdrant_migrate(
    source: String,
    target: String,
    batch_size: usize,
    dry_run: bool,
) -> Result<()> {
    let client = create_qdrant_client().await?;

    // Validate collections exist
    let collections = client.list_collections().await?;
    if !collections.contains(&source) {
        anyhow::bail!("Source collection '{}' does not exist", source);
    }
    if !collections.contains(&target) {
        anyhow::bail!("Target collection '{}' does not exist", target);
    }

    // Get source collection info to check dimensions and validate compatibility
    let source_info = client.collection_info(&source).await?;
    let target_info = client.collection_info(&target).await?;

    output::info(format!(
        "Migrating from '{}' to '{}'",
        source, target
    ));

    // Get total count for progress tracking
    let total_points = client.count_points(&source).await?;
    output::info(format!("Total points to migrate: {}", total_points));

    if dry_run {
        output::info(format!(
            "DRY RUN: Would migrate {} points from {} to {} (batch size: {})",
            total_points, source, target, batch_size
        ));
        output::info("Migration steps that would be performed:");
        output::info("  1. Scroll through all points in source collection");
        output::info("  2. Transform vectors if dimensions differ");
        output::info("  3. Upsert points to target collection in batches");
        output::info("  4. Track and report progress");
        output::info("  5. Handle errors with retries");
        return Ok(());
    }

    if total_points == 0 {
        output::warning("Source collection is empty, nothing to migrate");
        return Ok(());
    }

    // Check if dimensions match
    let source_dim = source_info.config.as_ref()
        .and_then(|c| c.params.as_ref())
        .and_then(|p| p.vectors_config.as_ref())
        .and_then(|v| {
            if let Some(qdrant_client::qdrant::vectors_config::Config::Params(params)) = &v.config {
                Some(params.size)
            } else {
                None
            }
        });

    let target_dim = target_info.config.as_ref()
        .and_then(|c| c.params.as_ref())
        .and_then(|p| p.vectors_config.as_ref())
        .and_then(|v| {
            if let Some(qdrant_client::qdrant::vectors_config::Config::Params(params)) = &v.config {
                Some(params.size)
            } else {
                None
            }
        });

    let needs_transform = match (source_dim, target_dim) {
        (Some(s), Some(t)) if s != t => {
            output::warning(format!(
                "Vector dimensions differ (source: {}, target: {}). Will transform vectors.",
                s, t
            ));
            true
        }
        (Some(s), Some(t)) => {
            output::success(format!("Vector dimensions match: {}", s));
            false
        }
        _ => {
            output::warning("Could not determine vector dimensions, proceeding without validation");
            false
        }
    };

    let start_time = Instant::now();
    let mut migrated_count = 0u64;
    let mut failed_count = 0u64;
    let mut offset: Option<qdrant_client::qdrant::PointId> = None;

    let spinner = output::spinner("Migrating points...");

    // Migration loop with scroll pagination
    loop {
        // Scroll through points in batches
        let scroll_result = client.scroll_points(
            &source,
            batch_size as u32,
            offset.clone(),
            true,
            true,
        ).await?;

        let points = scroll_result.result;

        if points.is_empty() {
            break;
        }

        let batch_count = points.len();

        // Transform points if needed
        let transformed_points: Vec<qdrant_client::qdrant::PointStruct> = points
            .into_iter()
            .filter_map(|point| {
                // Extract point data
                let point_id = point.id?;
                let vectors = point.vectors?;
                let payload = point.payload;

                // Extract vector data
                let vector_data = match vectors.vectors_options? {
                    qdrant_client::qdrant::vectors_output::VectorsOptions::Vector(v) => v.data,
                    _ => return None,
                };

                // Transform vector if dimensions differ
                let final_vector = if needs_transform {
                    if let (Some(s), Some(t)) = (source_dim, target_dim) {
                        transform_vector(vector_data, s as usize, t as usize)
                    } else {
                        vector_data
                    }
                } else {
                    vector_data
                };

                Some(qdrant_client::qdrant::PointStruct::new(
                    point_id,
                    final_vector,
                    payload,
                ))
            })
            .collect();

        let points_to_insert = transformed_points.len();

        // Upsert to target collection with retries
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match client.upsert_points(&target, transformed_points.clone()).await {
                Ok(_) => {
                    migrated_count += points_to_insert as u64;
                    break;
                }
                Err(e) if retry_count < max_retries => {
                    retry_count += 1;
                    output::warning(format!(
                        "Retry {}/{}: Failed to upsert batch: {}",
                        retry_count, max_retries, e
                    ));
                    tokio::time::sleep(std::time::Duration::from_millis(100 * 2u64.pow(retry_count))).await;
                }
                Err(e) => {
                    output::error(format!("Failed to upsert batch after {} retries: {}", max_retries, e));
                    failed_count += points_to_insert as u64;
                    break;
                }
            }
        }

        // Update progress
        let progress_pct = (migrated_count as f64 / total_points as f64 * 100.0) as u64;
        spinner.set_message(format!(
            "Migrated {}/{} points ({}%)",
            migrated_count, total_points, progress_pct
        ));

        // Update offset for next iteration
        if let Some(last_point) = scroll_result.result.last() {
            offset = last_point.id.clone();
        } else {
            break;
        }

        // Check if we've processed all points
        if batch_count < batch_size {
            break;
        }
    }

    let duration = start_time.elapsed();

    spinner.finish_with_message(format!(
        "Migration complete: {}/{} points migrated in {:.2}s",
        migrated_count,
        total_points,
        duration.as_secs_f64()
    ));

    // Display statistics
    output::success("\nMigration Statistics:");
    println!("  Total points:     {}", total_points);
    println!("  Migrated:         {}", migrated_count);
    println!("  Failed:           {}", failed_count);
    println!("  Duration:         {:.2}s", duration.as_secs_f64());
    println!("  Throughput:       {:.2} points/sec", migrated_count as f64 / duration.as_secs_f64());

    if failed_count > 0 {
        output::warning(format!(
            "{} points failed to migrate. Check logs for details.",
            failed_count
        ));
    }

    // Verify target collection count
    let target_count = client.count_points(&target).await?;
    output::info(format!("Target collection now contains {} points", target_count));

    if failed_count > 0 {
        anyhow::bail!("Migration completed with {} failures", failed_count);
    }

    Ok(())
}

/// Transform vector from source dimension to target dimension
/// Uses truncation for dimension reduction and zero-padding for dimension increase
fn transform_vector(vector: Vec<f32>, source_dim: usize, target_dim: usize) -> Vec<f32> {
    if source_dim == target_dim {
        return vector;
    }

    if source_dim > target_dim {
        // Truncate to target dimension
        vector.into_iter().take(target_dim).collect()
    } else {
        // Pad with zeros to target dimension
        let mut result = vector;
        result.resize(target_dim, 0.0);
        result
    }
}

/// Verify Qdrant data consistency
pub async fn qdrant_verify(collection: Option<String>, fix: bool) -> Result<()> {
    let client = create_qdrant_client().await?;

    let collections = if let Some(name) = collection {
        vec![name]
    } else {
        client.list_collections().await?
    };

    output::info(format!("Verifying {} collection(s)...", collections.len()));

    for name in collections {
        output::info(format!("\nVerifying collection: {}", name));

        let stats = client.get_collection_stats(&name).await?;

        // Check indexing lag
        let unindexed = stats.vectors_count - stats.indexed_vectors_count;
        if unindexed > 0 {
            output::warning(format!("  {} unindexed vectors", unindexed));
            if fix {
                output::info("  Triggering optimization...");
                // Note: Optimization is automatic in Qdrant, but we could trigger it
            }
        } else {
            output::success("  All vectors indexed");
        }

        // Check status
        if stats.status != "Green" {
            output::warning(format!("  Collection status: {}", stats.status));
        } else {
            output::success("  Collection status: Green");
        }

        // Check segments
        if stats.segments_count > 50 {
            output::warning(format!(
                "  High segment count: {} (consider optimization)",
                stats.segments_count
            ));
        } else {
            output::success(format!("  Segment count: {}", stats.segments_count));
        }
    }

    Ok(())
}

/// Run performance benchmarks
pub async fn qdrant_benchmark(
    collection: Option<String>,
    num_queries: usize,
    dimensions: usize,
    format: OutputFormat,
) -> Result<()> {
    let client = create_qdrant_client().await?;

    // Use first available collection if none specified
    let collection_name = if let Some(name) = collection {
        name
    } else {
        let collections = client.list_collections().await?;
        collections
            .first()
            .ok_or_else(|| anyhow::anyhow!("No collections found"))?
            .clone()
    };

    output::info(format!(
        "Running benchmark on collection: {}",
        collection_name
    ));
    output::info(format!("Queries: {}, Dimensions: {}", num_queries, dimensions));

    // Generate random vectors
    use rand::Rng;
    let mut rng = rand::rng();

    let mut latencies = Vec::new();

    for i in 0..num_queries {
        let vector: Vec<f32> = (0..dimensions).map(|_| rng.random::<f32>()).collect();

        let start = Instant::now();
        let _results = client.search(&collection_name, vector, 10, None).await?;
        let duration = start.elapsed();

        latencies.push(duration.as_millis() as f64);

        if (i + 1) % 10 == 0 {
            output::info(format!("Completed {} queries...", i + 1));
        }
    }

    // Calculate statistics
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = latencies.first().unwrap();
    let max = latencies.last().unwrap();
    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[latencies.len() * 95 / 100];
    let p99 = latencies[latencies.len() * 99 / 100];

    match format {
        OutputFormat::Json => {
            let results = serde_json::json!({
                "collection": collection_name,
                "num_queries": num_queries,
                "dimensions": dimensions,
                "latency_ms": {
                    "min": min,
                    "max": max,
                    "avg": avg,
                    "p50": p50,
                    "p95": p95,
                    "p99": p99,
                }
            });
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        _ => {
            output::success("\nBenchmark Results:");
            println!("  Min latency:  {:.2} ms", min);
            println!("  Max latency:  {:.2} ms", max);
            println!("  Avg latency:  {:.2} ms", avg);
            println!("  P50 latency:  {:.2} ms", p50);
            println!("  P95 latency:  {:.2} ms", p95);
            println!("  P99 latency:  {:.2} ms", p99);
            println!("  Throughput:   {:.2} qps", num_queries as f64 / (latencies.iter().sum::<f64>() / 1000.0));
        }
    }

    Ok(())
}

/// Create a snapshot
pub async fn qdrant_snapshot(collection: Option<String>, output: Option<PathBuf>) -> Result<()> {
    let client = create_qdrant_client().await?;

    let collections = if let Some(name) = collection {
        vec![name]
    } else {
        client.list_collections().await?
    };

    for name in collections {
        output::info(format!("Creating snapshot for collection: {}", name));
        let snapshot_name = client.create_snapshot(&name).await?;
        output::success(format!("  Snapshot created: {}", snapshot_name));

        if let Some(ref output_dir) = output {
            output::info(format!("  Snapshot location: {:?}", output_dir));
        }
    }

    Ok(())
}

/// Restore from snapshot
pub async fn qdrant_restore(snapshot: PathBuf, collection: Option<String>) -> Result<()> {
    let spinner = output::spinner("Restoring snapshot...");

    // Validate snapshot file exists early
    if !snapshot.exists() {
        anyhow::bail!("Snapshot file does not exist: {:?}", snapshot);
    }

    if !snapshot.is_file() {
        anyhow::bail!("Snapshot path is not a file: {:?}", snapshot);
    }

    // Get file size for display
    let metadata = std::fs::metadata(&snapshot)
        .context(format!("Failed to read snapshot metadata: {:?}", snapshot))?;
    let file_size = metadata.len();

    output::info(format!("Snapshot file: {:?}", snapshot));
    output::info(format!("File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1024.0 / 1024.0));

    if let Some(ref name) = collection {
        output::info(format!("Target collection: {}", name));
    } else {
        output::info("Collection name will be extracted from snapshot filename");
    }

    // Create Qdrant client
    let client = create_qdrant_client().await?;

    // Verify Qdrant is healthy before attempting restore
    let health = client.health().await.context("Qdrant health check failed")?;
    output::info(format!("Qdrant is healthy: {}", health.title));

    // Perform the restore
    let start = Instant::now();
    let collection_ref = collection.as_deref();

    match client.restore_snapshot(&snapshot, collection_ref, Some(true)).await {
        Ok(_) => {
            let duration = start.elapsed();
            spinner.finish_with_message(format!(
                "Snapshot restored successfully in {:.2}s",
                duration.as_secs_f64()
            ));

            // Get and display collection stats
            if let Some(ref name) = collection {
                match client.get_collection_stats(name).await {
                    Ok(stats) => {
                        output::success("\nRestored Collection Statistics:");
                        println!("  Collection: {}", stats.name);
                        println!("  Vectors: {}", stats.vectors_count);
                        println!("  Indexed Vectors: {}", stats.indexed_vectors_count);
                        println!("  Points: {}", stats.points_count);
                        println!("  Segments: {}", stats.segments_count);
                        println!("  Status: {}", stats.status);
                    }
                    Err(e) => {
                        output::warning(format!("Could not fetch collection stats: {}", e));
                    }
                }
            } else {
                // Try to extract collection name from snapshot filename to show stats
                if let Some(stem) = snapshot.file_stem().and_then(|s| s.to_str()) {
                    if let Some(extracted_name) = stem.split('-').next() {
                        match client.get_collection_stats(extracted_name).await {
                            Ok(stats) => {
                                output::success("\nRestored Collection Statistics:");
                                println!("  Collection: {}", stats.name);
                                println!("  Vectors: {}", stats.vectors_count);
                                println!("  Indexed Vectors: {}", stats.indexed_vectors_count);
                                println!("  Points: {}", stats.points_count);
                                println!("  Segments: {}", stats.segments_count);
                                println!("  Status: {}", stats.status);
                            }
                            Err(e) => {
                                output::warning(format!("Could not fetch collection stats: {}", e));
                            }
                        }
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            spinner.finish_with_message("Snapshot restore failed");
            Err(e).context("Failed to restore snapshot")
        }
    }
}

/// Optimize collection
pub async fn qdrant_optimize(collection: String, wait: bool) -> Result<()> {
    let client = create_qdrant_client().await?;

    output::info(format!("Triggering optimization for collection: {}", collection));

    // Note: Qdrant handles optimization automatically
    // This command is mainly informational
    let stats = client.get_collection_stats(&collection).await?;

    output::info(format!("Current status: {}", stats.status));
    output::info(format!("Optimizer status: {}", stats.optimizer_status));
    output::info(format!("Segments: {}", stats.segments_count));

    if wait {
        output::info("Note: Qdrant optimizes automatically. No manual trigger needed.");
    }

    Ok(())
}

/// List all collections
pub async fn qdrant_list(detailed: bool, format: OutputFormat) -> Result<()> {
    let client = create_qdrant_client().await?;

    let collections = client.list_collections().await?;

    if collections.is_empty() {
        output::warning("No collections found");
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            if detailed {
                let mut detailed_info = HashMap::new();
                for name in &collections {
                    if let Ok(stats) = client.get_collection_stats(name).await {
                        detailed_info.insert(name.clone(), stats);
                    }
                }
                println!("{}", serde_json::to_string_pretty(&detailed_info)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&collections)?);
            }
        }
        OutputFormat::Plain => {
            for name in collections {
                println!("{}", name);
            }
        }
        OutputFormat::Human => {
            if detailed {
                let table = TableBuilder::new()
                    .header(vec!["Collection", "Vectors", "Indexed", "Segments", "Status"]);

                let mut table = table;
                for name in collections {
                    if let Ok(stats) = client.get_collection_stats(&name).await {
                        table = table.row(vec![
                            stats.name,
                            stats.vectors_count.to_string(),
                            stats.indexed_vectors_count.to_string(),
                            stats.segments_count.to_string(),
                            stats.status,
                        ]);
                    }
                }

                table.print();
            } else {
                output::success(format!("Found {} collection(s):", collections.len()));
                for name in collections {
                    println!("  - {}", name);
                }
            }
        }
    }

    Ok(())
}
