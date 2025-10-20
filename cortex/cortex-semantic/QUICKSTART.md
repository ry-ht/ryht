# Cortex Semantic Search - Quick Start Guide

Get started with semantic search in 5 minutes!

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cortex-semantic = { path = "../cortex-semantic" }
tokio = { version = "1.48", features = ["full"] }
```

## Basic Example

```rust
use cortex_semantic::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Create engine with default config
    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config).await?;

    // 2. Index some documents
    engine.index_document(
        "doc1".into(),
        "Rust is a systems programming language".into(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    // 3. Search!
    let results = engine.search("programming languages", 10).await?;

    // 4. Use results
    for result in results {
        println!("{}: {} (score: {:.2})",
            result.id,
            result.content,
            result.score
        );
    }

    Ok(())
}
```

## Configuration

### Use OpenAI Embeddings (Recommended for Production)

```rust
use cortex_semantic::config::SemanticConfig;

let mut config = SemanticConfig::default();

// Set OpenAI as primary provider
config.embedding.primary_provider = "openai".to_string();
config.embedding.openai.api_key = Some(std::env::var("OPENAI_API_KEY")?);
config.embedding.openai.model = "text-embedding-3-small".to_string();

let engine = SemanticSearchEngine::new(config).await?;
```

### Use Local ONNX Embeddings (Privacy-First)

```rust
let mut config = SemanticConfig::default();

config.embedding.primary_provider = "onnx".to_string();
config.embedding.onnx.model_name = "all-MiniLM-L6-v2".to_string();
config.embedding.onnx.dimension = 384;

let engine = SemanticSearchEngine::new(config).await?;
```

### Use Mock Provider (Testing)

```rust
let mut config = SemanticConfig::default();
config.embedding.primary_provider = "mock".to_string();

let engine = SemanticSearchEngine::new(config).await?;
```

## Common Use Cases

### 1. Search Code Snippets

```rust
use cortex_semantic::{EntityType, SearchFilter};

// Index code
let mut metadata = HashMap::new();
metadata.insert("language".into(), "rust".into());
metadata.insert("file".into(), "auth.rs".into());

engine.index_document(
    "fn_authenticate".into(),
    "fn authenticate(token: &str) -> Result<User> { /* ... */ }".into(),
    EntityType::Code,
    metadata,
).await?;

// Search code only
let filter = SearchFilter {
    entity_type: Some(EntityType::Code),
    ..Default::default()
};

let results = engine.search_with_filter(
    "user authentication",
    10,
    filter
).await?;
```

### 2. Search Documentation

```rust
// Index docs
engine.index_document(
    "async_guide".into(),
    "Async programming in Rust allows concurrent execution...".into(),
    EntityType::Document,
    HashMap::new(),
).await?;

// Search
let results = engine.search("concurrency in rust", 5).await?;
```

### 3. Batch Indexing

```rust
let documents = vec![
    ("doc1".into(), "Content 1".into(), EntityType::Document, HashMap::new()),
    ("doc2".into(), "Content 2".into(), EntityType::Document, HashMap::new()),
    ("doc3".into(), "Content 3".into(), EntityType::Document, HashMap::new()),
];

engine.index_batch(documents).await?;
```

### 4. Filter by Metadata

```rust
// Index with metadata
let mut metadata = HashMap::new();
metadata.insert("language".into(), "rust".into());
metadata.insert("complexity".into(), "beginner".into());

engine.index_document(
    "tutorial_1".into(),
    "Getting started with Rust...".into(),
    EntityType::Document,
    metadata,
).await?;

// Search with metadata filter
let mut metadata_filters = HashMap::new();
metadata_filters.insert("language".into(), "rust".into());
metadata_filters.insert("complexity".into(), "beginner".into());

let filter = SearchFilter {
    metadata_filters,
    ..Default::default()
};

let results = engine.search_with_filter("rust basics", 10, filter).await?;
```

### 5. Persist Index to Disk

```rust
let mut config = SemanticConfig::default();
config.index.persist_path = Some("./data/semantic_index.bin".into());
config.index.auto_save_interval_seconds = 300; // Auto-save every 5 min

let engine = SemanticSearchEngine::new(config).await?;

// Index documents...
// ...

// Manual save
engine.save_index().await?;

// Later, load from disk
let engine2 = SemanticSearchEngine::new(config).await?;
// Index automatically loaded from persist_path
```

## Advanced Features

### Enable Hybrid Search (Semantic + Keyword)

```rust
let mut config = SemanticConfig::default();
config.search.enable_hybrid_search = true;
config.search.hybrid_keyword_weight = 0.3; // 70% semantic, 30% keyword

let engine = SemanticSearchEngine::new(config).await?;
```

### Custom Ranking Weights

```rust
use cortex_semantic::ranking::{Ranker, RankingStrategy, ScoringWeights};

let weights = ScoringWeights {
    semantic: 0.5,     // 50% semantic similarity
    keyword: 0.3,      // 30% keyword match
    recency: 0.15,     // 15% recency bias
    popularity: 0.05,  // 5% popularity signals
};

// Use in custom ranker
let ranker = Ranker::with_weights(RankingStrategy::Weighted, weights);
```

### Enable Caching for Performance

```rust
let mut config = SemanticConfig::default();

// Embedding cache (saves API calls)
config.cache.enable_embedding_cache = true;
config.cache.embedding_cache_size = 10_000;
config.cache.embedding_cache_ttl_seconds = 3600; // 1 hour

// Query cache (saves search time)
config.cache.enable_query_cache = true;
config.cache.query_cache_size = 1_000;
config.cache.query_cache_ttl_seconds = 300; // 5 minutes

let engine = SemanticSearchEngine::new(config).await?;
```

## Performance Tips

### 1. Use Batch Indexing for Large Datasets

```rust
// ❌ Slow: Individual inserts
for doc in documents {
    engine.index_document(...).await?;
}

// ✅ Fast: Batch insert
engine.index_batch(documents).await?;
```

### 2. Enable Caching

```rust
config.cache.enable_embedding_cache = true;
config.cache.enable_query_cache = true;
```

### 3. Filter Early

```rust
// Use filters to reduce search space
let filter = SearchFilter {
    entity_type: Some(EntityType::Code),
    min_score: Some(0.7),
    ..Default::default()
};
```

### 4. Tune HNSW Parameters

```rust
// For accuracy
config.index.hnsw_ef_search = 200;  // Higher = more accurate, slower

// For speed
config.index.hnsw_ef_search = 50;   // Lower = faster, less accurate
```

## TOML Configuration

Create `config.toml`:

```toml
[embedding]
primary_provider = "openai"
fallback_providers = ["onnx"]
batch_size = 32

[embedding.openai]
model = "text-embedding-3-small"
dimension = 384

[index]
hnsw_m = 32
hnsw_ef_construction = 100
hnsw_ef_search = 100
similarity_metric = "cosine"
persist_path = "./data/index.bin"
auto_save_interval_seconds = 300

[search]
default_limit = 10
max_limit = 100
default_threshold = 0.5
enable_hybrid_search = true
enable_reranking = true
timeout_ms = 1000

[cache]
enable_embedding_cache = true
embedding_cache_size = 10000
embedding_cache_ttl_seconds = 3600
enable_query_cache = true
query_cache_size = 1000
query_cache_ttl_seconds = 300
```

Load configuration:

```rust
use std::fs;

let config_str = fs::read_to_string("config.toml")?;
let config: SemanticConfig = toml::from_str(&config_str)?;

let engine = SemanticSearchEngine::new(config).await?;
```

## Environment Variables

Set your OpenAI API key:

```bash
export OPENAI_API_KEY="sk-..."
```

Or in `.env` file:

```
OPENAI_API_KEY=sk-...
RUST_LOG=info
```

## Error Handling

```rust
use cortex_semantic::error::{SemanticError, Result};

match engine.search("query", 10).await {
    Ok(results) => {
        println!("Found {} results", results.len());
    }
    Err(SemanticError::Provider(msg)) => {
        eprintln!("Provider error: {}", msg);
        // Fallback to alternative provider
    }
    Err(SemanticError::DimensionMismatch { expected, got }) => {
        eprintln!("Dimension mismatch: expected {}, got {}", expected, got);
    }
    Err(e) => {
        eprintln!("Search error: {}", e);
    }
}
```

## Testing

Run the examples:

```bash
# Basic usage example
cargo run --example basic_usage

# Run tests
cargo test

# Run benchmarks
cargo bench
```

## Monitoring

### Get Index Statistics

```rust
let stats = engine.stats().await;
println!("Total vectors: {}", stats.total_vectors);
println!("Dimension: {}", stats.dimension);
println!("Metric: {:?}", stats.metric);
```

### Check Document Count

```rust
let count = engine.document_count().await;
println!("Indexed documents: {}", count);
```

### Enable Logging

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

Or via environment:

```bash
RUST_LOG=cortex_semantic=debug cargo run
```

## Common Issues

### Issue: "Provider error: All providers failed"

**Solution:** Check your API key is set:

```bash
echo $OPENAI_API_KEY
```

Or configure a local provider:

```rust
config.embedding.primary_provider = "onnx".to_string();
```

### Issue: Slow queries

**Solution:** Enable caching:

```rust
config.cache.enable_query_cache = true;
```

### Issue: High memory usage

**Solution:** Reduce cache sizes:

```rust
config.cache.embedding_cache_size = 1_000;
config.cache.query_cache_size = 100;
```

## Next Steps

- Read the [Implementation Report](./IMPLEMENTATION_REPORT.md) for detailed architecture
- Check [Integration Tests](./tests/integration_tests.rs) for more examples
- Review [Benchmarks](./benches/) for performance insights
- Explore [Advanced Configuration](./IMPLEMENTATION_REPORT.md#27-configuration-system)

## API Reference

Full API documentation:

```bash
cargo doc --open
```

## Support

- GitHub Issues: [Report bugs](https://github.com/your-org/cortex/issues)
- Documentation: [Full docs](./IMPLEMENTATION_REPORT.md)
- Examples: [See examples/](./examples/)

---

**Happy Searching! 🔍**
