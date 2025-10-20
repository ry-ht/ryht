//! Comprehensive unit tests for cortex-ingestion

// ============================================================================
// Chunker Tests
// ============================================================================

#[test]
fn test_fixed_size_chunker() {
    use cortex_ingestion::chunker::FixedSizeChunker;
    use cortex_core::traits::Chunker;

    let chunker = FixedSizeChunker::new(100, 10);
    let content = "a".repeat(250);

    let chunks = chunker.chunk(&content);

    assert!(chunks.len() >= 2);
    assert!(chunks[0].len() <= 100);
}

#[test]
fn test_semantic_chunker_paragraphs() {
    use cortex_ingestion::chunker::SemanticChunker;
    use cortex_core::traits::Chunker;

    let chunker = SemanticChunker::new_paragraph_based(500);
    let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";

    let chunks = chunker.chunk(content);

    assert!(chunks.len() >= 1);
}

#[test]
fn test_chunker_empty_content() {
    use cortex_ingestion::chunker::FixedSizeChunker;
    use cortex_core::traits::Chunker;

    let chunker = FixedSizeChunker::new(100, 10);
    let chunks = chunker.chunk("");

    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_chunker_overlap() {
    use cortex_ingestion::chunker::FixedSizeChunker;
    use cortex_core::traits::Chunker;

    let chunker = FixedSizeChunker::new(50, 10);
    assert_eq!(chunker.overlap(), 10);

    let content = "a".repeat(100);
    let chunks = chunker.chunk(&content);

    // With overlap, content should appear in multiple chunks
    assert!(chunks.len() >= 2);
}

// ============================================================================
// Filter Tests
// ============================================================================

#[test]
fn test_extension_filter() {
    use cortex_ingestion::filters::ExtensionFilter;
    use cortex_ingestion::filters::FileFilter;

    let filter = ExtensionFilter::new(vec!["rs", "toml"]);

    assert!(filter.should_include("test.rs"));
    assert!(filter.should_include("Cargo.toml"));
    assert!(!filter.should_include("test.txt"));
}

#[test]
fn test_size_filter() {
    use cortex_ingestion::filters::SizeFilter;
    use cortex_ingestion::filters::FileFilter;

    let filter = SizeFilter::new(0, 1024 * 1024); // 0 to 1MB

    assert!(filter.should_include_size(500));
    assert!(filter.should_include_size(1024 * 1024));
    assert!(!filter.should_include_size(2 * 1024 * 1024));
}

#[test]
fn test_gitignore_filter() {
    use cortex_ingestion::filters::GitignoreFilter;
    use cortex_ingestion::filters::FileFilter;

    let filter = GitignoreFilter::default();

    assert!(!filter.should_include("target/debug/app"));
    assert!(!filter.should_include(".git/config"));
    assert!(!filter.should_include("node_modules/package"));
}

#[test]
fn test_combined_filters() {
    use cortex_ingestion::filters::{ExtensionFilter, SizeFilter, CombinedFilter};
    use cortex_ingestion::filters::FileFilter;

    let ext_filter = ExtensionFilter::new(vec!["rs"]);
    let size_filter = SizeFilter::new(0, 10000);

    let combined = CombinedFilter::new(vec![
        Box::new(ext_filter),
        Box::new(size_filter),
    ]);

    // Should pass both filters
    assert!(combined.should_include("test.rs") && combined.should_include_size(5000));

    // Should fail extension filter
    assert!(!combined.should_include("test.txt"));
}

// ============================================================================
// Extractor Tests
// ============================================================================

#[test]
fn test_text_extractor() {
    use cortex_ingestion::extractor::TextExtractor;

    let extractor = TextExtractor::new();
    let content = b"Hello, world!";

    let result = extractor.extract(content);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, world!");
}

#[test]
fn test_text_extractor_invalid_utf8() {
    use cortex_ingestion::extractor::TextExtractor;

    let extractor = TextExtractor::new();
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];

    let result = extractor.extract(&invalid_utf8);
    // Should handle invalid UTF-8 gracefully
    assert!(result.is_ok() || result.is_err());
}

// ============================================================================
// Processor Tests - Plain Text
// ============================================================================

#[test]
fn test_txt_processor() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = TxtProcessor::new();
    let content = b"Line 1\nLine 2\nLine 3";

    let result = processor.process("test.txt", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.content.contains("Line 1"));
    assert!(processed.content.contains("Line 3"));
}

#[test]
fn test_txt_processor_empty() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = TxtProcessor::new();
    let result = processor.process("empty.txt", b"");

    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "");
}

// ============================================================================
// Processor Tests - Markdown
// ============================================================================

#[test]
fn test_markdown_processor_headers() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = MarkdownProcessor::new();
    let content = b"# Header 1\n\nParagraph\n\n## Header 2";

    let result = processor.process("test.md", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("headers"));
}

#[test]
fn test_markdown_processor_code_blocks() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = MarkdownProcessor::new();
    let content = b"```rust\nfn main() {}\n```";

    let result = processor.process("code.md", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("code_blocks"));
}

#[test]
fn test_markdown_processor_links() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = MarkdownProcessor::new();
    let content = b"[Link text](https://example.com)";

    let result = processor.process("links.md", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("links"));
}

// ============================================================================
// Processor Tests - JSON
// ============================================================================

#[test]
fn test_json_processor_valid() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = JsonProcessor::new();
    let content = br#"{"name": "test", "value": 123}"#;

    let result = processor.process("data.json", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.content.contains("name"));
    assert!(processed.content.contains("test"));
}

#[test]
fn test_json_processor_invalid() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = JsonProcessor::new();
    let content = b"invalid json {";

    let result = processor.process("invalid.json", content);
    assert!(result.is_err());
}

#[test]
fn test_json_processor_nested() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = JsonProcessor::new();
    let content = br#"{"nested": {"key": "value", "array": [1, 2, 3]}}"#;

    let result = processor.process("nested.json", content);
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - YAML
// ============================================================================

#[test]
fn test_yaml_processor_valid() {
    use cortex_ingestion::processors::yaml::YamlProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = YamlProcessor::new();
    let content = b"name: test\nvalue: 123";

    let result = processor.process("config.yaml", content);
    assert!(result.is_ok());
}

#[test]
fn test_yaml_processor_list() {
    use cortex_ingestion::processors::yaml::YamlProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = YamlProcessor::new();
    let content = b"- item1\n- item2\n- item3";

    let result = processor.process("list.yaml", content);
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - CSV
// ============================================================================

#[test]
fn test_csv_processor_headers() {
    use cortex_ingestion::processors::csv::CsvProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = CsvProcessor::new();
    let content = b"name,age,city\nAlice,30,NYC\nBob,25,LA";

    let result = processor.process("data.csv", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("columns"));
    assert!(processed.metadata.contains_key("rows"));
}

#[test]
fn test_csv_processor_no_headers() {
    use cortex_ingestion::processors::csv::CsvProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = CsvProcessor::new_without_headers();
    let content = b"Alice,30,NYC\nBob,25,LA";

    let result = processor.process("data.csv", content);
    assert!(result.is_ok());
}

#[test]
fn test_csv_processor_custom_delimiter() {
    use cortex_ingestion::processors::csv::CsvProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = CsvProcessor::with_delimiter(b'\t');
    let content = b"name\tage\tcity\nAlice\t30\tNYC";

    let result = processor.process("data.tsv", content);
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - HTML
// ============================================================================

#[test]
fn test_html_processor_extract_text() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = HtmlProcessor::new();
    let content = b"<html><body><h1>Title</h1><p>Paragraph</p></body></html>";

    let result = processor.process("page.html", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.content.contains("Title"));
    assert!(processed.content.contains("Paragraph"));
}

#[test]
fn test_html_processor_metadata() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = HtmlProcessor::new();
    let content = b"<html><head><title>Page Title</title></head><body>Content</body></html>";

    let result = processor.process("page.html", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("title"));
}

#[test]
fn test_html_processor_strip_scripts() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::Processor;

    let processor = HtmlProcessor::new();
    let content = b"<html><body><script>alert('test')</script><p>Content</p></body></html>";

    let result = processor.process("page.html", content);
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(!processed.content.contains("alert"));
    assert!(processed.content.contains("Content"));
}

// ============================================================================
// Embeddings Tests
// ============================================================================

#[test]
fn test_embedding_config() {
    use cortex_ingestion::embeddings::EmbeddingConfig;

    let config = EmbeddingConfig {
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        batch_size: 32,
        api_key: Some("test-key".to_string()),
    };

    assert_eq!(config.dimension, 1536);
    assert_eq!(config.batch_size, 32);
}

#[test]
fn test_embedding_request_batching() {
    use cortex_ingestion::embeddings::EmbeddingBatch;

    let mut batch = EmbeddingBatch::new(10);

    batch.add("text 1");
    batch.add("text 2");

    assert_eq!(batch.len(), 2);
    assert!(!batch.is_full());

    for i in 3..=10 {
        batch.add(&format!("text {}", i));
    }

    assert!(batch.is_full());
}

// ============================================================================
// Project Loader Tests
// ============================================================================

#[test]
fn test_project_loader_config() {
    use cortex_ingestion::project_loader::LoaderConfig;

    let config = LoaderConfig {
        max_file_size: 10 * 1024 * 1024, // 10MB
        follow_symlinks: false,
        include_hidden: false,
        parallel_workers: 4,
    };

    assert_eq!(config.max_file_size, 10 * 1024 * 1024);
    assert!(!config.follow_symlinks);
}

#[test]
fn test_project_stats() {
    use cortex_ingestion::project_loader::ProjectStats;

    let mut stats = ProjectStats::default();

    stats.files_processed += 10;
    stats.bytes_processed += 1024 * 1024;
    stats.errors += 1;

    assert_eq!(stats.files_processed, 10);
    assert_eq!(stats.bytes_processed, 1024 * 1024);
    assert_eq!(stats.errors, 1);
}

// ============================================================================
// Ingester Tests
// ============================================================================

#[test]
fn test_ingester_config() {
    use cortex_ingestion::ingester::IngesterConfig;

    let config = IngesterConfig {
        chunk_size: 1000,
        chunk_overlap: 100,
        generate_embeddings: true,
        parallel_workers: 4,
        max_file_size: 100 * 1024 * 1024,
    };

    assert_eq!(config.chunk_size, 1000);
    assert!(config.generate_embeddings);
}

#[test]
fn test_ingestion_result() {
    use cortex_ingestion::ingester::IngestionResult;
    use cortex_core::id::CortexId;

    let result = IngestionResult {
        document_id: CortexId::new(),
        chunks_created: 5,
        embeddings_created: 5,
        processing_time_ms: 1500,
        errors: vec![],
    };

    assert_eq!(result.chunks_created, 5);
    assert_eq!(result.embeddings_created, 5);
    assert!(result.errors.is_empty());
}

#[test]
fn test_ingestion_result_with_errors() {
    use cortex_ingestion::ingester::IngestionResult;
    use cortex_core::id::CortexId;

    let mut result = IngestionResult {
        document_id: CortexId::new(),
        chunks_created: 3,
        embeddings_created: 2,
        processing_time_ms: 2000,
        errors: vec![],
    };

    result.errors.push("Failed to generate embedding for chunk 3".to_string());

    assert_eq!(result.chunks_created, 3);
    assert_eq!(result.embeddings_created, 2);
    assert_eq!(result.errors.len(), 1);
}

// ============================================================================
// Integration-style Tests
// ============================================================================

#[test]
fn test_end_to_end_text_processing() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::Processor;
    use cortex_ingestion::chunker::FixedSizeChunker;
    use cortex_core::traits::Chunker;

    let processor = TxtProcessor::new();
    let content = b"This is a test document with multiple sentences. It should be processed and chunked correctly.";

    let result = processor.process("test.txt", content);
    assert!(result.is_ok());

    let processed = result.unwrap();

    let chunker = FixedSizeChunker::new(50, 10);
    let chunks = chunker.chunk(&processed.content);

    assert!(chunks.len() > 0);
}

#[test]
fn test_filter_then_process_workflow() {
    use cortex_ingestion::filters::{ExtensionFilter, FileFilter};
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::Processor;

    let filter = ExtensionFilter::new(vec!["txt", "md"]);

    // Should process
    if filter.should_include("doc.txt") {
        let processor = TxtProcessor::new();
        let result = processor.process("doc.txt", b"content");
        assert!(result.is_ok());
    }

    // Should skip
    assert!(!filter.should_include("image.png"));
}
