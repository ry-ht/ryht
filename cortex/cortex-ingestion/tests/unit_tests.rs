//! Comprehensive unit tests for cortex-ingestion

// ============================================================================
// Chunker Tests
// ============================================================================

#[test]
fn test_fixed_size_chunker() {
    use cortex_ingestion::chunker::Chunker;
    use cortex_core::traits::Chunker as ChunkerTrait;

    let chunker = Chunker::new(100, 10);
    let content = "a".repeat(250);

    let chunks = chunker.chunk(&content);

    assert!(chunks.len() >= 2);
    assert!(chunks[0].len() <= 100);
}

#[test]
fn test_semantic_chunker_paragraphs() {
    use cortex_ingestion::chunker::{SemanticChunker, ChunkStrategy};
    use cortex_core::traits::Chunker;

    let chunker = SemanticChunker::with_strategy(500, 50, ChunkStrategy::Paragraph);
    let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";

    let chunks = chunker.chunk(content);

    assert!(chunks.len() >= 1);
}

#[test]
fn test_chunker_empty_content() {
    use cortex_ingestion::chunker::Chunker;
    use cortex_core::traits::Chunker as ChunkerTrait;

    let chunker = Chunker::new(100, 10);
    let chunks = chunker.chunk("");

    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_chunker_overlap() {
    use cortex_ingestion::chunker::Chunker;
    use cortex_core::traits::Chunker as ChunkerTrait;

    let chunker = Chunker::new(50, 10);
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
fn test_content_filter() {
    use cortex_ingestion::filters::ContentFilter;

    let mut filter = ContentFilter::new().with_min_quality(0.3);

    let good_content = "This is high quality content with good variety and sufficient length.";
    let result = filter.should_accept(good_content, "hash1");
    assert!(result.accepted);

    // Test duplicate detection
    let result = filter.should_accept(good_content, "hash1");
    assert!(!result.accepted);
}

#[test]
fn test_quality_scoring() {
    use cortex_ingestion::filters::calculate_quality_score;

    let good_text = "This is a well-written piece with good variety and structure.";
    let metrics = calculate_quality_score(good_text);
    assert!(metrics.score > 0.5);
    assert!(metrics.readability > 0.0);
    assert!(metrics.density > 0.0);

    let poor_text = "a a a a a";
    let metrics = calculate_quality_score(poor_text);
    assert!(metrics.score < 0.5);
}

// ============================================================================
// Processor Tests - Plain Text
// ============================================================================

#[tokio::test]
async fn test_txt_processor() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = TxtProcessor::new();
    let content = b"Line 1\nLine 2\nLine 3";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.text_content.contains("Line 1"));
    assert!(processed.text_content.contains("Line 3"));
}

#[tokio::test]
async fn test_txt_processor_empty() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = TxtProcessor::new();
    let result = processor.process(b"").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().text_content, "");
}

// ============================================================================
// Processor Tests - Markdown
// ============================================================================

#[tokio::test]
async fn test_markdown_processor_headers() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = MarkdownProcessor::new();
    let content = b"# Header 1\n\nParagraph\n\n## Header 2";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("heading_count"));
}

#[tokio::test]
async fn test_markdown_processor_code_blocks() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = MarkdownProcessor::new();
    let content = b"```rust\nfn main() {}\n```";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("code_block_count"));
}

#[tokio::test]
async fn test_markdown_processor_links() {
    use cortex_ingestion::processors::markdown::MarkdownProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = MarkdownProcessor::new();
    let content = b"[Link text](https://example.com)";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("link_count"));
}

// ============================================================================
// Processor Tests - JSON
// ============================================================================

#[tokio::test]
async fn test_json_processor_valid() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = JsonProcessor::new();
    let content = br#"{"name": "test", "value": 123}"#;

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.text_content.contains("name"));
    assert!(processed.text_content.contains("test"));
}

#[tokio::test]
async fn test_json_processor_invalid() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = JsonProcessor::new();
    let content = b"invalid json {";

    let result = processor.process(content).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_json_processor_nested() {
    use cortex_ingestion::processors::json::JsonProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = JsonProcessor::new();
    let content = br#"{"nested": {"key": "value", "array": [1, 2, 3]}}"#;

    let result = processor.process(content).await;
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - YAML
// ============================================================================

#[tokio::test]
async fn test_yaml_processor_valid() {
    use cortex_ingestion::processors::yaml::YamlProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = YamlProcessor::new();
    let content = b"name: test\nvalue: 123";

    let result = processor.process(content).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_yaml_processor_list() {
    use cortex_ingestion::processors::yaml::YamlProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = YamlProcessor::new();
    let content = b"- item1\n- item2\n- item3";

    let result = processor.process(content).await;
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - CSV
// ============================================================================

#[tokio::test]
async fn test_csv_processor_headers() {
    use cortex_ingestion::processors::csv::CsvProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = CsvProcessor::new();
    let content = b"name,age,city\nAlice,30,NYC\nBob,25,LA";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("column_count"));
    assert!(processed.metadata.contains_key("row_count"));
}

#[tokio::test]
async fn test_csv_processor_row_chunks() {
    use cortex_ingestion::processors::csv::CsvProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = CsvProcessor::with_row_chunks();
    let content = b"name,age,city\nAlice,30,NYC\nBob,25,LA";

    let result = processor.process(content).await;
    assert!(result.is_ok());
}

// ============================================================================
// Processor Tests - HTML
// ============================================================================

#[tokio::test]
async fn test_html_processor_extract_text() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = HtmlProcessor::new();
    let content = b"<html><body><h1>Title</h1><p>Paragraph</p></body></html>";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.text_content.contains("Title"));
    assert!(processed.text_content.contains("Paragraph"));
}

#[tokio::test]
async fn test_html_processor_metadata() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = HtmlProcessor::new();
    let content = b"<html><head><title>Page Title</title></head><body>Content</body></html>";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(processed.metadata.contains_key("title"));
}

#[tokio::test]
async fn test_html_processor_strip_scripts() {
    use cortex_ingestion::processors::html::HtmlProcessor;
    use cortex_ingestion::processors::ContentProcessor;

    let processor = HtmlProcessor::new();
    let content = b"<html><body><script>alert('test')</script><p>Content</p></body></html>";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();
    assert!(!processed.text_content.contains("alert"));
    assert!(processed.text_content.contains("Content"));
}

// ============================================================================
// Embeddings Tests
// ============================================================================

#[test]
fn test_embedding_config() {
    use cortex_ingestion::embeddings::EmbeddingConfig;

    let config = EmbeddingConfig {
        batch_size: 32,
        cache_enabled: true,
        max_text_length: 8000,
    };

    assert_eq!(config.batch_size, 32);
    assert!(config.cache_enabled);
}

// ============================================================================
// Project Loader Tests
// ============================================================================

#[test]
fn test_project_import_options() {
    use cortex_ingestion::project_loader::ProjectImportOptions;

    let options = ProjectImportOptions {
        read_only: false,
        create_fork: false,
        include_patterns: vec![],
        exclude_patterns: vec![],
        max_depth: Some(5),
        process_code: true,
        generate_embeddings: false,
        follow_links: false,
        respect_gitignore: true,
    };

    assert_eq!(options.max_depth, Some(5));
    assert!(options.respect_gitignore);
}

#[test]
fn test_project_stats() {
    use cortex_ingestion::project_loader::ProjectStats;

    let mut stats = ProjectStats::default();

    stats.file_count += 10;
    stats.total_size_bytes += 1024 * 1024;
    stats.directory_count += 1;

    assert_eq!(stats.file_count, 10);
    assert_eq!(stats.total_size_bytes, 1024 * 1024);
    assert_eq!(stats.directory_count, 1);
}

// ============================================================================
// Integration-style Tests
// ============================================================================

#[tokio::test]
async fn test_end_to_end_text_processing() {
    use cortex_ingestion::processors::txt::TxtProcessor;
    use cortex_ingestion::processors::ContentProcessor;
    use cortex_ingestion::chunker::Chunker;
    use cortex_core::traits::Chunker as ChunkerTrait;

    let processor = TxtProcessor::new();
    let content = b"This is a test document with multiple sentences. It should be processed and chunked correctly.";

    let result = processor.process(content).await;
    assert!(result.is_ok());

    let processed = result.unwrap();

    let chunker = Chunker::new(50, 10);
    let chunks = chunker.chunk(&processed.text_content);

    assert!(chunks.len() > 0);
}

#[test]
fn test_filter_then_process_workflow() {
    use cortex_ingestion::filters::{should_ignore_file, is_text_file};
    use std::path::Path;

    // Test text file detection
    assert!(is_text_file(Path::new("doc.txt")));
    assert!(is_text_file(Path::new("README.md")));

    // Test ignore file detection
    assert!(!should_ignore_file(Path::new("doc.txt")));
    assert!(should_ignore_file(Path::new("image.png")));
}
