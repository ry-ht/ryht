# Cortex Semantic Search

A production-ready semantic search system for Cortex, providing vector embeddings, HNSW indexing, and intelligent search capabilities.

## Features

- **Multiple Embedding Providers**
  - OpenAI (text-embedding-3-small/large)
  - ONNX Runtime (local models like all-MiniLM-L6-v2)
  - Ollama (local LLM embeddings)
  - Mock provider for testing
  - Automatic fallback chain

- **HNSW Vector Index**
  - Fast approximate nearest neighbor search
  - Configurable parameters (M, ef_construction, ef_search)
  - Index persistence to disk
  - Incremental updates
  - Support for up to 1M+ vectors

- **Advanced Search**
  - Pure semantic search
  - Hybrid keyword + semantic search
  - Query expansion and intent detection
  - Result re-ranking with multiple scoring algorithms
  - Faceted search with filters

- **Query Processing**
  - Natural language query parsing
  - Intent detection (code, documentation, examples, etc.)
  - Keyword extraction
  - Query expansion with synonyms
  - Filter extraction (language:rust, type:function, etc.)

- **Performance Optimizations**
  - Embedding caching (in-memory with TTL)
  - Query result caching
  - Batch embedding generation
  - Parallel indexing
  - < 100ms search latency target

- **Production Ready**
  - Comprehensive error handling
  - Full async/await support
  - Thread-safe operations
  - Extensive testing
  - Performance benchmarks

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cortex-semantic = { path = "../cortex-semantic" }
```

## Quick Start

```rust
use cortex_semantic::prelude::*;
use cortex_semantic::config::SemanticConfig;
use cortex_semantic::EntityType;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create search engine
    let config = SemanticConfig::default();
    let engine = SemanticSearchEngine::new(config).await?;

    // Index documents
    engine.index_document(
        "doc1".to_string(),
        "Rust is a systems programming language".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    engine.index_document(
        "doc2".to_string(),
        "Python is great for data science".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await?;

    // Search
    let results = engine.search("programming languages", 10).await?;

    for result in results {
        println!("Found: {} (score: {:.3})", result.id, result.score);
        println!("Content: {}\n", result.content);
    }

    Ok(())
}
```

## Configuration

### Basic Configuration

```rust
use cortex_semantic::config::*;

let mut config = SemanticConfig::default();

// Configure embedding provider
config.embedding.primary_provider = "openai".to_string();
config.embedding.fallback_providers = vec!["onnx".to_string()];

// Set OpenAI API key
config.embedding.openai.api_key = Some("sk-...".to_string());
config.embedding.openai.model = "text-embedding-3-small".to_string();

// Configure index
config.index.hnsw_m = 32;
config.index.hnsw_ef_construction = 100;
config.index.persist_path = Some("/path/to/index.bin".into());

// Configure search
config.search.enable_hybrid_search = true;
config.search.enable_reranking = true;
```

### Environment Variables

```bash
# OpenAI API Key
export OPENAI_API_KEY="sk-..."

# Use local ONNX models
export CORTEX_EMBEDDING_PROVIDER="onnx"

# Configure Ollama endpoint
export CORTEX_OLLAMA_ENDPOINT="http://localhost:11434"
```

## Usage Examples

### Batch Indexing

```rust
let documents = vec![
    ("doc1".to_string(), "Content 1".to_string(), EntityType::Document, HashMap::new()),
    ("doc2".to_string(), "Content 2".to_string(), EntityType::Document, HashMap::new()),
    ("doc3".to_string(), "Content 3".to_string(), EntityType::Document, HashMap::new()),
];

engine.index_batch(documents).await?;
```

### Search with Filters

```rust
use cortex_semantic::SearchFilter;

let mut metadata_filters = HashMap::new();
metadata_filters.insert("language".to_string(), "rust".to_string());

let filter = SearchFilter {
    entity_type: Some(EntityType::Code),
    language: Some("rust".to_string()),
    min_score: Some(0.7),
    metadata_filters,
};

let results = engine.search_with_filter("error handling", 10, filter).await?;
```

### Query Intent Detection

```rust
use cortex_semantic::query::QueryProcessor;

let processor = QueryProcessor::new();
let processed = processor.process("how to implement authentication?")?;

println!("Intent: {:?}", processed.intent);  // Intent::Examples
println!("Keywords: {:?}", processed.keywords);
println!("Expanded queries: {:?}", processed.expanded);
```

### Custom Ranking

```rust
use cortex_semantic::ranking::{Ranker, RankingStrategy, ScoringWeights};

let weights = ScoringWeights {
    semantic: 0.5,
    keyword: 0.3,
    recency: 0.1,
    popularity: 0.1,
};

let ranker = Ranker::with_weights(RankingStrategy::Weighted, weights);
```

### Index Persistence

```rust
// Save index
engine.save_index().await?;

// Load index on startup
let config = SemanticConfig {
    index: IndexConfig {
        persist_path: Some("/path/to/index.bin".into()),
        ..Default::default()
    },
    ..Default::default()
};

let engine = SemanticSearchEngine::new(config).await?;
// Index is automatically loaded if file exists
```

## Embedding Providers

### OpenAI

```rust
config.embedding.openai.api_key = Some(std::env::var("OPENAI_API_KEY")?);
config.embedding.openai.model = "text-embedding-3-small".to_string(); // 1536 dims
// or
config.embedding.openai.model = "text-embedding-3-large".to_string(); // 3072 dims
```

### ONNX Runtime (Local)

```rust
config.embedding.onnx.model_path = Some("/path/to/model.onnx".into());
config.embedding.onnx.model_name = "all-MiniLM-L6-v2".to_string();
config.embedding.onnx.dimension = 384;
config.embedding.onnx.use_gpu = true;
```

### Ollama

```rust
config.embedding.ollama.endpoint = "http://localhost:11434".to_string();
config.embedding.ollama.model = "nomic-embed-text".to_string();
config.embedding.ollama.dimension = 768;
```

## Performance

Target performance metrics:

- **Indexing**: 1000+ documents/second (batch mode)
- **Search Latency**: < 100ms for 100k vectors
- **Throughput**: 100+ queries/second
- **Memory**: ~4GB for 1M 384-dim vectors

### Benchmarks

Run benchmarks:

```bash
cargo bench --package cortex-semantic
```

## Testing

Run tests:

```bash
# Unit tests
cargo test --package cortex-semantic

# Integration tests
cargo test --package cortex-semantic --test integration_tests

# All tests
cargo test --package cortex-semantic --all
```

## Architecture

```
cortex-semantic/
├── src/
│   ├── lib.rs              # Public API
│   ├── config.rs           # Configuration types
│   ├── types.rs            # Core types (Vector, DocumentId, etc.)
│   ├── error.rs            # Error types
│   ├── providers.rs        # Embedding providers
│   ├── index.rs            # HNSW vector index
│   ├── query.rs            # Query processing
│   ├── ranking.rs          # Result ranking
│   ├── search.rs           # Main search engine
│   └── cache.rs            # Caching layer
├── tests/
│   └── integration_tests.rs
├── benches/
│   ├── search_bench.rs
│   └── embedding_bench.rs
└── examples/
    └── basic_usage.rs
```

## Integration with Cortex

The semantic search system integrates with other Cortex components:

- **cortex-storage**: Persist embeddings and metadata
- **cortex-memory**: Search episodic memories
- **cortex-ingestion**: Index processed documents
- **cortex-mcp**: Expose search through MCP tools
- **cortex-vfs**: Index virtual filesystem content

## Roadmap

- [ ] Support for multi-modal embeddings (image + text)
- [ ] Dynamic index updates without rebuilding
- [ ] Distributed index sharding
- [ ] GPU acceleration for ONNX models
- [ ] Advanced query understanding with LLMs
- [ ] Federated search across multiple indexes
- [ ] Real-time index updates via streaming

## Contributing

See main Cortex documentation for contribution guidelines.

## License

MIT
