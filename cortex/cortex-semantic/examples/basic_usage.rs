//! Basic usage example for cortex-semantic.

use cortex_semantic::prelude::*;
use cortex_semantic::config::SemanticConfig;
use cortex_semantic::{EntityType, SearchFilter};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Cortex Semantic Search Example ===\n");

    // Create configuration
    let mut config = SemanticConfig::default();

    // Use mock provider for demo (in production, use OpenAI or ONNX)
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];

    // Enable caching
    config.cache.enable_embedding_cache = true;
    config.cache.enable_query_cache = true;

    println!("Creating semantic search engine...");
    let engine = SemanticSearchEngine::new(config).await?;

    // Example 1: Index code snippets
    println!("\n--- Example 1: Indexing Code Snippets ---");

    let mut rust_metadata = HashMap::new();
    rust_metadata.insert("language".to_string(), "rust".to_string());
    rust_metadata.insert("file".to_string(), "main.rs".to_string());

    engine.index_document(
        "rust_fn1".to_string(),
        "fn calculate_sum(numbers: Vec<i32>) -> i32 { numbers.iter().sum() }".to_string(),
        EntityType::Code,
        rust_metadata.clone(),
    ).await?;

    rust_metadata.insert("file".to_string(), "lib.rs".to_string());
    engine.index_document(
        "rust_fn2".to_string(),
        "fn find_max(numbers: &[i32]) -> Option<i32> { numbers.iter().max().copied() }".to_string(),
        EntityType::Code,
        rust_metadata,
    ).await?;

    let mut python_metadata = HashMap::new();
    python_metadata.insert("language".to_string(), "python".to_string());
    python_metadata.insert("file".to_string(), "utils.py".to_string());

    engine.index_document(
        "python_fn1".to_string(),
        "def calculate_average(numbers): return sum(numbers) / len(numbers)".to_string(),
        EntityType::Code,
        python_metadata,
    ).await?;

    println!("Indexed 3 code snippets");

    // Example 2: Index documentation
    println!("\n--- Example 2: Indexing Documentation ---");

    engine.index_document(
        "doc1".to_string(),
        "Rust is a systems programming language focused on safety, speed, and concurrency".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    engine.index_document(
        "doc2".to_string(),
        "Python is an interpreted high-level programming language known for its simplicity".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    engine.index_document(
        "doc3".to_string(),
        "Machine learning is a subset of artificial intelligence that enables systems to learn from data".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    println!("Indexed 3 documentation pages");

    // Example 3: Batch indexing
    println!("\n--- Example 3: Batch Indexing ---");

    let episodes = vec![
        ("episode1".to_string(), "User asked about error handling in async Rust".to_string(), EntityType::Episode, HashMap::new()),
        ("episode2".to_string(), "Implemented authentication using JWT tokens".to_string(), EntityType::Episode, HashMap::new()),
        ("episode3".to_string(), "Debugged performance issue in database queries".to_string(), EntityType::Episode, HashMap::new()),
    ];

    engine.index_batch(episodes).await?;
    println!("Batch indexed 3 episodes");

    println!("\nTotal documents indexed: {}", engine.document_count().await);

    // Example 4: Basic search
    println!("\n--- Example 4: Basic Search ---");

    let results = engine.search("calculate sum of numbers", 5).await?;
    println!("Query: 'calculate sum of numbers'");
    println!("Found {} results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, result.id, result.score);
        println!("     Content: {}", result.content);
    }

    // Example 5: Search with entity type filter
    println!("\n--- Example 5: Filtered Search (Code Only) ---");

    let filter = SearchFilter {
        entity_type: Some(EntityType::Code),
        ..Default::default()
    };

    let results = engine.search_with_filter("find maximum value", 5, filter).await?;
    println!("Query: 'find maximum value' (Entity: Code)");
    println!("Found {} code results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, result.id, result.score);
    }

    // Example 6: Search with metadata filter
    println!("\n--- Example 6: Metadata Filter (Rust Only) ---");

    let mut metadata_filters = HashMap::new();
    metadata_filters.insert("language".to_string(), "rust".to_string());

    let filter = SearchFilter {
        metadata_filters,
        ..Default::default()
    };

    let results = engine.search_with_filter("function", 5, filter).await?;
    println!("Query: 'function' (Language: Rust)");
    println!("Found {} Rust results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, result.id, result.score);
    }

    // Example 7: Search documentation
    println!("\n--- Example 7: Documentation Search ---");

    let filter = SearchFilter {
        entity_type: Some(EntityType::Document),
        ..Default::default()
    };

    let results = engine.search_with_filter("programming language", 5, filter).await?;
    println!("Query: 'programming language' (Entity: Document)");
    println!("Found {} documentation results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, result.id, result.score);
        println!("     {}", result.content);
    }

    // Example 8: Search episodes
    println!("\n--- Example 8: Episode Search ---");

    let filter = SearchFilter {
        entity_type: Some(EntityType::Episode),
        min_score: Some(0.1),
        ..Default::default()
    };

    let results = engine.search_with_filter("authentication", 5, filter).await?;
    println!("Query: 'authentication' (Entity: Episode)");
    println!("Found {} episode results:", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.3})", i + 1, result.id, result.score);
        println!("     {}", result.content);
    }

    // Example 9: Multiple searches (demonstrates caching)
    println!("\n--- Example 9: Cache Performance ---");

    let query = "programming";

    let start = std::time::Instant::now();
    let _ = engine.search(query, 5).await?;
    let first_duration = start.elapsed();

    let start = std::time::Instant::now();
    let _ = engine.search(query, 5).await?;
    let second_duration = start.elapsed();

    println!("First search: {:?}", first_duration);
    println!("Second search (cached): {:?}", second_duration);
    println!("Speedup: {:.2}x", first_duration.as_secs_f64() / second_duration.as_secs_f64());

    // Example 10: Index statistics
    println!("\n--- Example 10: Index Statistics ---");

    let stats = engine.stats().await;
    println!("Index Statistics:");
    println!("  Total vectors: {}", stats.total_vectors);
    println!("  Dimension: {}", stats.dimension);
    println!("  Similarity metric: {:?}", stats.metric);
    println!("  HNSW M: {}", stats.hnsw_m);
    println!("  HNSW ef_construction: {}", stats.hnsw_ef_construction);

    println!("\n=== Example Complete ===");

    Ok(())
}
