# Cortex Ingestion Framework - Quick Start Guide

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cortex-ingestion = { path = "../cortex-ingestion" }
```

## Basic Usage

### 1. Simple File Ingestion

```rust
use cortex_ingestion::prelude::*;
use std::sync::Arc;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize storage (your implementation)
    let storage = Arc::new(your_storage_implementation);

    // Create ingester
    let ingester = DocumentIngester::new(storage);

    // Ingest a file
    let project_id = CortexId::new();
    let document = ingester
        .ingest_file(project_id, Path::new("document.pdf"))
        .await?;

    println!("Ingested document: {}", document.id);
    Ok(())
}
```

### 2. Ingestion with Automatic Chunking

```rust
use cortex_ingestion::prelude::*;

let ingester = DocumentIngester::new(storage)
    .with_auto_chunk(true)  // Enable automatic chunking
    .with_embeddings(true); // Generate embeddings

let document = ingester
    .ingest_file(project_id, Path::new("article.md"))
    .await?;
```

### 3. Process Specific File Types

```rust
use cortex_ingestion::processors::*;

// Create processor factory
let factory = ProcessorFactory::new();

// Get processor for a file
if let Some(processor) = factory.get_for_path(Path::new("data.json")) {
    let content = tokio::fs::read("data.json").await?;
    let processed = processor.process(&content).await?;

    println!("Content type: {:?}", processed.content_type);
    println!("Chunks: {}", processed.chunks.len());
    println!("Metadata: {:?}", processed.metadata);
}
```

### 4. Import External Projects

```rust
use cortex_ingestion::prelude::*;

let loader = ProjectLoader::new();

// Configure import options
let options = ProjectImportOptions {
    respect_gitignore: true,
    process_code: true,
    generate_embeddings: false,
    max_depth: Some(10),
    ..Default::default()
};

// Import a project
let (files, report) = loader
    .import_project(Path::new("/path/to/codebase"), options)
    .await?;

println!("Imported {} files", report.files_imported);
println!("Skipped {} files", report.files_skipped);
println!("Processed {} bytes in {:.2}s",
    report.bytes_processed,
    report.duration_secs);

// Access imported files
for file in files {
    println!("File: {} ({})", file.relative_path, file.content_type);
}
```

### 5. Custom Chunking Strategies

```rust
use cortex_ingestion::chunker::*;
use cortex_core::traits::Chunker;

// Semantic chunking (paragraph-based)
let chunker = SemanticChunker::new(1000, 100);
let chunks = chunker.chunk("Your long text here...");

// Code-aware chunking
let code_chunker = CodeChunker::new(1500, 150);
let code_chunks = code_chunker.chunk("fn main() { ... }");

// Hierarchical chunking
let hierarchical = HierarchicalChunker::new(5000, 1000, 100);
let (parent_chunks, child_chunks) = hierarchical.chunk_hierarchical("Document text");
```

### 6. Metadata Extraction

```rust
use cortex_ingestion::extractor::*;

let content = "Your document text...";

// Detect language
if let Some(lang) = detect_language(content) {
    println!("Language: {}", lang);
}

// Extract keywords
let keywords = extract_keywords(content, 10);
println!("Keywords: {:?}", keywords);

// Comprehensive metadata
let metadata = extract_comprehensive_metadata(
    Path::new("document.md"),
    content
);
println!("Metadata: {:?}", metadata);
```

### 7. Embedding Generation

```rust
use cortex_ingestion::embeddings::*;
use std::sync::Arc;

// Create an embedding provider (use your own implementation)
let provider = Arc::new(MockEmbeddingProvider::new(384));
let service = EmbeddingService::with_provider(provider);

// Generate single embedding
let embedding = service.embed("Some text to embed").await?;
println!("Embedding dimension: {}", embedding.len());

// Batch processing
let texts = vec![
    "First text".to_string(),
    "Second text".to_string(),
    "Third text".to_string(),
];
let embeddings = service.embed_batch(&texts).await?;
println!("Generated {} embeddings", embeddings.len());
```

### 8. Analyze Project Before Import

```rust
use cortex_ingestion::prelude::*;

let loader = ProjectLoader::new();
let options = ProjectImportOptions::default();

// Get statistics without importing
let stats = loader.analyze_project(
    Path::new("/path/to/project"),
    options
).await?;

println!("Files: {}", stats.file_count);
println!("Directories: {}", stats.directory_count);
println!("Total size: {} bytes", stats.total_size_bytes);
println!("Languages: {:?}", stats.languages);
println!("File types: {:?}", stats.file_types);
```

## Supported File Formats

| Format | Extension | Features |
|--------|-----------|----------|
| PDF | `.pdf` | Text extraction, metadata, page-level chunks |
| Markdown | `.md`, `.markdown` | Frontmatter, sections, code blocks |
| HTML | `.html`, `.htm` | Structure preservation, metadata extraction |
| JSON | `.json` | Structure preservation, searchable text |
| YAML | `.yaml`, `.yml` | Configuration parsing, nested structures |
| CSV | `.csv` | Tabular data, row-level chunks |
| Text | `.txt` | Paragraph-based chunking |
| Code | Various | Syntax-aware chunking (via CodeChunker) |

## Chunking Strategies

### Semantic Chunking
```rust
let chunker = SemanticChunker::with_strategy(
    1000,  // max chunk size
    100,   // overlap
    ChunkStrategy::Hybrid  // paragraph + sentence
);
```

**Strategies:**
- `ChunkStrategy::Sentence` - Split by sentences
- `ChunkStrategy::Paragraph` - Split by paragraphs
- `ChunkStrategy::FixedSize` - Fixed character count
- `ChunkStrategy::Hybrid` - Smart combination (default)

### Code Chunking
```rust
let chunker = CodeChunker::new(1500, 150);
```

- Respects function/class boundaries
- Tracks brace depth
- Handles string literals correctly

### Hierarchical Chunking
```rust
let chunker = HierarchicalChunker::new(
    5000,  // parent chunk size
    1000,  // child chunk size
    100    // overlap
);
```

- Creates parent-child relationships
- Enables multi-level retrieval

## Configuration

### Import Options

```rust
let options = ProjectImportOptions {
    read_only: false,              // Make files read-only
    create_fork: false,            // Create editable fork
    include_patterns: vec![],      // Include globs
    exclude_patterns: vec![],      // Exclude globs
    max_depth: Some(10),           // Max directory depth
    process_code: true,            // Parse code files
    generate_embeddings: false,    // Generate embeddings
    follow_links: false,           // Follow symlinks
    respect_gitignore: true,       // Honor .gitignore
};
```

### Embedding Config

```rust
let config = EmbeddingConfig {
    batch_size: 32,           // Batch size for processing
    cache_enabled: true,      // Enable caching
    max_text_length: 8000,    // Max text before truncation
};

let service = EmbeddingService::new(provider, config);
```

## Error Handling

All operations return `Result<T, CortexError>`:

```rust
match ingester.ingest_file(project_id, path).await {
    Ok(document) => println!("Success: {}", document.id),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Performance Tips

1. **Batch Processing**: Use `embed_batch()` instead of multiple `embed()` calls
2. **Parallel Import**: Process multiple files concurrently with `tokio::spawn`
3. **Streaming**: For very large files, consider streaming chunks
4. **Caching**: Enable embedding cache for repeated content
5. **Filtering**: Use include/exclude patterns to skip unnecessary files

## Common Patterns

### Process Directory of Documents

```rust
let documents = ingester
    .ingest_directory(project_id, Path::new("./documents"))
    .await?;

for doc in documents {
    println!("Processed: {}", doc.path);
}
```

### Custom Processor Pipeline

```rust
// Read file
let content = tokio::fs::read(path).await?;

// Process with appropriate processor
let factory = ProcessorFactory::new();
let processor = factory.get_for_path(path).unwrap();
let processed = processor.process(&content).await?;

// Extract metadata
let metadata = extract_comprehensive_metadata(path, &processed.text_content);

// Generate embeddings for chunks
let texts: Vec<String> = processed.chunks.iter()
    .map(|c| c.content.clone())
    .collect();
let embeddings = embedding_service.embed_batch(&texts).await?;
```

### Filter by Programming Language

```rust
use cortex_ingestion::extractor::detect_programming_language;

if let Some(lang) = detect_programming_language(path) {
    if lang == "Rust" {
        // Process Rust files specially
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_ingestion() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        tokio::fs::write(&file_path, b"# Test\n\nContent")
            .await
            .unwrap();

        let storage = Arc::new(MockStorage);
        let ingester = DocumentIngester::new(storage);

        let doc = ingester
            .ingest_file(CortexId::new(), &file_path)
            .await
            .unwrap();

        assert_eq!(doc.mime_type, "text/markdown");
    }
}
```

## Troubleshooting

### "No processor found for file"
- Check file extension is supported
- Use `detect_content_type()` to verify detection
- Provide custom processor via factory

### "Failed to parse document"
- Check file encoding (UTF-8 preferred)
- Verify file is not corrupted
- Check logs for specific error

### "Embedding generation slow"
- Reduce batch size
- Enable caching
- Use async/parallel processing

### "Import skipped many files"
- Check .gitignore patterns
- Review exclude_patterns
- Verify file permissions

## Further Reading

- See `IMPLEMENTATION_SUMMARY.md` for architecture details
- Check module documentation: `cargo doc --open`
- Review tests for usage examples
- See spec: `docs/spec/cortex-system/12-scalable-memory-architecture.md`
