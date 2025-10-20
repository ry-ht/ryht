# Cortex Semantic Search - Quick Start Guide

Get up and running with semantic search in 5 minutes.

## Installation

Add to your workspace:

```toml
# cortex/Cargo.toml
[workspace]
members = [
    # ... other members
    "cortex-semantic",
]
```

Add dependency:

```toml
# your-crate/Cargo.toml
[dependencies]
cortex-semantic = { path = "../cortex-semantic" }
```

## Basic Usage

### 1. Create Search Engine

```rust
use cortex_semantic::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use default configuration with OpenAI
    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config).await?;

    Ok(())
}
```

### 2. Index Documents

```rust
use cortex_semantic::EntityType;
use std::collections::HashMap;

// Index a single document
engine.index_document(
    "doc1".to_string(),
    "Rust is a systems programming language".to_string(),
    EntityType::Document,
    HashMap::new(),
).await?;

// Index multiple documents at once (faster)
let documents = vec![
    ("doc2".to_string(), "Python for data science".to_string(), EntityType::Document, HashMap::new()),
    ("doc3".to_string(), "JavaScript for web".to_string(), EntityType::Document, HashMap::new()),
];

engine.index_batch(documents).await?;
```

### 3. Search

```rust
// Simple search
let results = engine.search("programming languages", 10).await?;

for result in results {
    println!("{}: {} (score: {:.3})",
        result.id,
        result.content,
        result.score
    );
}
```

## Configuration Options

### Using OpenAI (Default)

```rust
let mut config = SemanticConfig::default();

// Set API key (or use OPENAI_API_KEY env var)
config.embedding.openai.api_key = Some("sk-...".to_string());

// Choose model
config.embedding.openai.model = "text-embedding-3-small".to_string(); // 1536 dims
// or
config.embedding.openai.model = "text-embedding-3-large".to_string(); // 3072 dims

let engine = SemanticSearchEngine::new(config).await?;
```

### Using Local ONNX Models

```rust
let mut config = SemanticConfig::default();

config.embedding.primary_provider = "onnx".to_string();
config.embedding.onnx.model_path = Some("/path/to/model.onnx".into());
config.embedding.onnx.dimension = 384;

let engine = SemanticSearchEngine::new(config).await?;
```

### Using Ollama

```rust
let mut config = SemanticConfig::default();

config.embedding.primary_provider = "ollama".to_string();
config.embedding.ollama.endpoint = "http://localhost:11434".to_string();
config.embedding.ollama.model = "nomic-embed-text".to_string();

let engine = SemanticSearchEngine::new(config).await?;
```

### Testing / Development (Mock Provider)

```rust
let mut config = SemanticConfig::default();

config.embedding.primary_provider = "mock".to_string();
config.embedding.fallback_providers = vec![];

let engine = SemanticSearchEngine::new(config).await?;
```

## Advanced Features

### Search with Filters

```rust
use cortex_semantic::SearchFilter;

// Filter by entity type
let filter = SearchFilter {
    entity_type: Some(EntityType::Code),
    ..Default::default()
};

let results = engine.search_with_filter("function", 10, filter).await?;
```

### Metadata Filtering

```rust
// Add metadata when indexing
let mut metadata = HashMap::new();
metadata.insert("language".to_string(), "rust".to_string());
metadata.insert("file".to_string(), "main.rs".to_string());

engine.index_document(
    "rust_fn".to_string(),
    "fn main() { println!(\"Hello\"); }".to_string(),
    EntityType::Code,
    metadata,
).await?;

// Filter by metadata
let mut metadata_filters = HashMap::new();
metadata_filters.insert("language".to_string(), "rust".to_string());

let filter = SearchFilter {
    metadata_filters,
    ..Default::default()
};

let results = engine.search_with_filter("hello world", 10, filter).await?;
```

### Score Threshold

```rust
// Only return high-confidence results
let filter = SearchFilter {
    min_score: Some(0.8),
    ..Default::default()
};

let results = engine.search_with_filter("query", 10, filter).await?;
```

### Hybrid Search (Keyword + Semantic)

```rust
let mut config = SemanticConfig::default();

// Enable hybrid search
config.search.enable_hybrid_search = true;
config.search.hybrid_keyword_weight = 0.3; // 30% keyword, 70% semantic

let engine = SemanticSearchEngine::new(config).await?;
```

### Index Persistence

```rust
use std::path::PathBuf;

let mut config = SemanticConfig::default();

// Enable persistence
config.index.persist_path = Some(PathBuf::from("/data/semantic_index.bin"));
config.index.auto_save_interval_seconds = 300; // Auto-save every 5 minutes

let engine = SemanticSearchEngine::new(config).await?;

// Manual save
engine.save_index().await?;
```

## Common Patterns

### Code Search

```rust
let mut metadata = HashMap::new();
metadata.insert("language".to_string(), "rust".to_string());
metadata.insert("symbol_type".to_string(), "function".to_string());

engine.index_document(
    "fn_calculate".to_string(),
    "fn calculate_sum(numbers: Vec<i32>) -> i32 { numbers.iter().sum() }".to_string(),
    EntityType::Code,
    metadata,
).await?;

// Search with language filter
let mut filters = HashMap::new();
filters.insert("language".to_string(), "rust".to_string());

let filter = SearchFilter {
    entity_type: Some(EntityType::Code),
    metadata_filters: filters,
    ..Default::default()
};

let results = engine.search_with_filter("calculate sum", 5, filter).await?;
```

### Episode Search

```rust
engine.index_document(
    "episode_123".to_string(),
    "User implemented JWT authentication for the API".to_string(),
    EntityType::Episode,
    HashMap::new(),
).await?;

let filter = SearchFilter {
    entity_type: Some(EntityType::Episode),
    ..Default::default()
};

let results = engine.search_with_filter("authentication", 10, filter).await?;
```

### Document Chunking + Indexing

```rust
// If you have long documents, chunk them first
let document = "Very long document content...";
let chunks = chunk_document(document, 500); // 500 tokens per chunk

let docs: Vec<_> = chunks.iter().enumerate().map(|(i, chunk)| {
    (
        format!("doc_chunk_{}", i),
        chunk.to_string(),
        EntityType::Chunk,
        HashMap::new(),
    )
}).collect();

engine.index_batch(docs).await?;
```

## Monitoring & Stats

```rust
// Get document count
let count = engine.document_count().await;
println!("Indexed documents: {}", count);

// Get index statistics
let stats = engine.stats().await;
println!("Total vectors: {}", stats.total_vectors);
println!("Dimension: {}", stats.dimension);
println!("Similarity metric: {:?}", stats.metric);
```

## Running the Example

```bash
# Set OpenAI API key (if using OpenAI)
export OPENAI_API_KEY="sk-..."

# Run the example
cd cortex/cortex-semantic
cargo run --example basic_usage
```

## Running Tests

```bash
# All tests
cargo test --package cortex-semantic

# Just unit tests
cargo test --package cortex-semantic --lib

# Just integration tests
cargo test --package cortex-semantic --test integration_tests

# With output
cargo test --package cortex-semantic -- --nocapture
```

## Running Benchmarks

```bash
# All benchmarks
cargo bench --package cortex-semantic

# Specific benchmark
cargo bench --package cortex-semantic --bench search_bench

# With baseline comparison
cargo bench --package cortex-semantic -- --save-baseline my-baseline
```

## Troubleshooting

### OpenAI API Key Not Found

```bash
export OPENAI_API_KEY="sk-your-key-here"
```

Or set in config:

```rust
config.embedding.openai.api_key = Some("sk-...".to_string());
```

### Dimension Mismatch

Ensure all documents use the same embedding model:

```rust
// Don't mix models!
config.embedding.primary_provider = "openai".to_string(); // 1536 dims
// ... index some documents ...

// Later changing to ONNX (384 dims) will cause errors
// Solution: Clear index or use same model
engine.clear().await?;
```

### Slow Search

1. Check index size - HNSW is O(log n) but large indexes need more memory
2. Adjust HNSW parameters:

```rust
config.index.hnsw_ef_search = 50; // Lower = faster but less accurate
```

3. Enable caching:

```rust
config.cache.enable_query_cache = true;
config.cache.query_cache_size = 1000;
```

### Out of Memory

1. Reduce index size limit:

```rust
config.index.max_index_size = 100_000; // Smaller limit
```

2. Use smaller embedding dimension:

```rust
// Use ONNX with 384 dims instead of OpenAI 1536 dims
config.embedding.primary_provider = "onnx".to_string();
```

## Next Steps

- Read the full [README.md](./README.md) for detailed documentation
- Check out [IMPLEMENTATION.md](./IMPLEMENTATION.md) for architecture details
- Browse the [examples](./examples/) directory
- Review the [tests](./tests/) for more usage patterns
- Run [benchmarks](./benches/) to understand performance

## Support

For issues and questions:
- Check existing tests and examples
- Review the documentation
- Open an issue in the main Cortex repository

## License

MIT
