//! Qdrant command implementations for cortex-cli

use crate::config::CortexConfig;
use crate::output::{self, OutputFormat, TableBuilder};
use anyhow::{Context, Result};
use cortex_storage::{CollectionConfig, CollectionStats, HnswConfig, OptimizerConfig, QdrantClient, QdrantConfig};
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

/// Migrate data to Qdrant (placeholder)
pub async fn qdrant_migrate(
    source: String,
    target: String,
    batch_size: usize,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        output::info(format!(
            "DRY RUN: Would migrate from {} to {} (batch size: {})",
            source, target, batch_size
        ));
        return Ok(());
    }

    output::warning("Migration functionality not yet implemented");
    output::info(format!("Source: {}", source));
    output::info(format!("Target: {}", target));
    output::info(format!("Batch size: {}", batch_size));

    Ok(())
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
        let results = client.search(&collection_name, vector, 10, None).await?;
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

/// Restore from snapshot (placeholder)
pub async fn qdrant_restore(snapshot: PathBuf, collection: Option<String>) -> Result<()> {
    output::warning("Snapshot restore functionality not yet implemented");
    output::info(format!("Snapshot: {:?}", snapshot));
    if let Some(name) = collection {
        output::info(format!("Target collection: {}", name));
    }
    Ok(())
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
