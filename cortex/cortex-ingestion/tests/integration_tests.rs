//! Integration tests for the cortex-ingestion system

use cortex_core::id::CortexId;
use cortex_core::traits::{Ingester, Storage};
use cortex_core::types::{Document, Embedding, Episode, Project, SystemStats, AgentSession};
use cortex_core::error::Result;
use cortex_ingestion::prelude::*;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// Mock storage implementation for testing
struct MockStorage;

#[async_trait]
impl Storage for MockStorage {
    async fn store_project(&self, _project: &Project) -> Result<()> {
        Ok(())
    }

    async fn get_project(&self, _id: CortexId) -> Result<Option<Project>> {
        Ok(None)
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        Ok(Vec::new())
    }

    async fn delete_project(&self, _id: CortexId) -> Result<()> {
        Ok(())
    }

    async fn store_document(&self, _document: &Document) -> Result<()> {
        Ok(())
    }

    async fn get_document(&self, _id: CortexId) -> Result<Option<Document>> {
        Ok(None)
    }

    async fn list_documents(&self, _project_id: CortexId) -> Result<Vec<Document>> {
        Ok(Vec::new())
    }

    async fn delete_document(&self, _id: CortexId) -> Result<()> {
        Ok(())
    }

    async fn store_embedding(&self, _embedding: &Embedding) -> Result<()> {
        Ok(())
    }

    async fn get_embeddings(&self, _entity_id: CortexId) -> Result<Vec<Embedding>> {
        Ok(Vec::new())
    }

    async fn store_episode(&self, _episode: &Episode) -> Result<()> {
        Ok(())
    }

    async fn get_episode(&self, _id: CortexId) -> Result<Option<Episode>> {
        Ok(None)
    }

    async fn get_stats(&self) -> Result<SystemStats> {
        Ok(SystemStats {
            total_projects: 0,
            total_documents: 0,
            total_chunks: 0,
            total_embeddings: 0,
            total_episodes: 0,
            storage_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn create_agent_session(
        &self,
        session_id: String,
        name: String,
        agent_type: String,
    ) -> Result<AgentSession> {
        Ok(AgentSession {
            id: session_id,
            name,
            agent_type,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn delete_agent_session(&self, _session_id: &str) -> Result<()> {
        Ok(())
    }

    async fn get_agent_session(&self, _session_id: &str) -> Result<Option<AgentSession>> {
        Ok(None)
    }

    async fn list_agent_sessions(&self) -> Result<Vec<AgentSession>> {
        Ok(Vec::new())
    }
}

// Helper to create test files
async fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create directory structure
    fs::create_dir(base_path.join("src")).await.unwrap();
    fs::create_dir(base_path.join("docs")).await.unwrap();

    // Create README with markdown
    let mut file = fs::File::create(base_path.join("README.md")).await.unwrap();
    file.write_all(b"# Test Project\n\nA comprehensive test project for ingestion.\n\n## Features\n\n- Feature 1\n- Feature 2\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n").await.unwrap();

    // Create Rust source file
    let mut file = fs::File::create(base_path.join("src/main.rs")).await.unwrap();
    file.write_all(
        b"fn main() {\n    println!(\"Hello, world!\");\n}\n\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .await
    .unwrap();

    // Create JSON config
    let mut file = fs::File::create(base_path.join("config.json")).await.unwrap();
    file.write_all(br#"{"name": "test", "version": "1.0", "settings": {"debug": true}}"#)
        .await
        .unwrap();

    // Create YAML config
    let mut file = fs::File::create(base_path.join("config.yaml")).await.unwrap();
    file.write_all(b"name: test\nversion: 1.0\nsettings:\n  debug: true\n")
        .await
        .unwrap();

    // Create CSV data
    let mut file = fs::File::create(base_path.join("data.csv")).await.unwrap();
    file.write_all(b"name,age,city\nAlice,30,NYC\nBob,25,LA\nCharlie,35,SF\n")
        .await
        .unwrap();

    // Create HTML file
    let mut file = fs::File::create(base_path.join("docs/index.html")).await.unwrap();
    file.write_all(
        b"<!DOCTYPE html>\n<html>\n<head><title>Test</title></head>\n<body>\n<h1>Welcome</h1>\n<p>Test content</p>\n</body>\n</html>\n",
    )
    .await
    .unwrap();

    temp_dir
}

#[tokio::test]
async fn test_pdf_processor_basic() {
    // Note: This test requires a valid PDF file
    // For now, we test that the processor can be created
    let processor = cortex_ingestion::processors::PdfProcessor::new();
    assert_eq!(processor.supported_extensions(), vec!["pdf"]);
}

#[tokio::test]
async fn test_markdown_processor() {
    let processor = cortex_ingestion::processors::MarkdownProcessor::new();
    let content = b"# Test\n\nThis is a test.\n\n## Section\n\nMore content.\n\n```rust\nfn main() {}\n```\n";

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Markdown);
    assert!(!result.chunks.is_empty());
    assert!(result.text_content.contains("Test"));
}

#[tokio::test]
async fn test_html_processor() {
    let processor = cortex_ingestion::processors::HtmlProcessor::new();
    let content = b"<html><head><title>Test</title></head><body><h1>Header</h1><p>Paragraph</p></body></html>";

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Html);
    assert!(result.metadata.contains_key("title"));
}

#[tokio::test]
async fn test_json_processor() {
    let processor = cortex_ingestion::processors::JsonProcessor::new();
    let content = br#"{"name": "test", "nested": {"key": "value"}}"#;

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Json);
    assert!(result.structured_data.is_some());
    assert!(!result.chunks.is_empty());
}

#[tokio::test]
async fn test_yaml_processor() {
    let processor = cortex_ingestion::processors::YamlProcessor::new();
    let content = b"name: test\nversion: 1.0\nconfig:\n  debug: true";

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Yaml);
    assert!(result.structured_data.is_some());
}

#[tokio::test]
async fn test_csv_processor() {
    let processor = cortex_ingestion::processors::CsvProcessor::new();
    let content = b"name,age,city\nAlice,30,NYC\nBob,25,LA";

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Csv);
    assert!(result.structured_data.is_some());
    assert!(result.metadata.contains_key("row_count"));
}

#[tokio::test]
async fn test_txt_processor() {
    let processor = cortex_ingestion::processors::TxtProcessor::new();
    let content = b"This is a test.\n\nSecond paragraph with more text.\n\nThird paragraph.";

    let result = processor.process(content).await.unwrap();
    assert_eq!(result.content_type, cortex_ingestion::processors::ContentType::Text);
    assert!(!result.chunks.is_empty());
}

#[tokio::test]
async fn test_semantic_chunker() {
    use cortex_core::traits::Chunker;

    let chunker = SemanticChunker::new(100, 10);
    let text = "First sentence. Second sentence. Third sentence. Fourth sentence with more text.";
    let chunks = chunker.chunk(text);

    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(chunk.len() <= 120); // Allow for some overlap
    }
}

#[tokio::test]
async fn test_code_chunker() {
    use cortex_core::traits::Chunker;

    let chunker = CodeChunker::new(200, 20);
    let code = r#"
fn function1() {
    println!("test");
}

fn function2() {
    let x = 42;
    return x;
}
"#;

    let chunks = chunker.chunk(code);
    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_hierarchical_chunker() {
    use cortex_ingestion::HierarchicalChunker;

    let chunker = HierarchicalChunker::new(500, 100, 20);
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(10);

    let (parent_chunks, child_chunks) = chunker.chunk_hierarchical(&text);
    assert!(!parent_chunks.is_empty());
    assert!(!child_chunks.is_empty());
}

#[tokio::test]
async fn test_project_loader() {
    let temp_dir = create_test_project().await;
    let loader = ProjectLoader::new();

    let options = ProjectImportOptions {
        respect_gitignore: true,
        process_code: true,
        ..Default::default()
    };

    let (files, report) = loader.import_project(temp_dir.path(), options).await.unwrap();

    assert!(files.len() >= 5); // At least 5 test files
    assert!(report.files_imported >= 5);
    assert_eq!(report.errors, 0);
    assert!(report.bytes_processed > 0);
}

#[tokio::test]
async fn test_project_analysis() {
    let temp_dir = create_test_project().await;
    let loader = ProjectLoader::new();

    let options = ProjectImportOptions::default();
    let stats = loader.analyze_project(temp_dir.path(), options).await.unwrap();

    assert!(stats.file_count >= 5);
    assert!(stats.total_size_bytes > 0);
    assert!(!stats.file_types.is_empty());
}

#[tokio::test]
async fn test_embedding_service() {
    use cortex_ingestion::embeddings::MockEmbeddingProvider;

    let provider = Arc::new(MockEmbeddingProvider::new(384));
    let service = EmbeddingService::with_provider(provider);

    let embedding = service.embed("test text").await.unwrap();
    assert_eq!(embedding.len(), 384);

    let texts = vec!["text1".to_string(), "text2".to_string(), "text3".to_string()];
    let embeddings = service.embed_batch(&texts).await.unwrap();
    assert_eq!(embeddings.len(), 3);
}

#[tokio::test]
async fn test_embedding_with_progress() {
    use cortex_ingestion::embeddings::MockEmbeddingProvider;

    let provider = Arc::new(MockEmbeddingProvider::new(384));
    let service = EmbeddingService::with_provider(provider);

    let texts = vec!["text1".to_string(), "text2".to_string()];
    let (embeddings, progress) = service.embed_batch_with_progress(&texts).await.unwrap();

    assert_eq!(embeddings.len(), 2);
    assert_eq!(progress.total, 2);
    assert_eq!(progress.completed, 2);
    assert!(progress.duration_secs > 0.0);
}

#[tokio::test]
async fn test_content_filter() {
    use cortex_ingestion::filters::ContentFilter;

    let mut filter = ContentFilter::new().with_min_quality(0.3);

    let good_content = "This is high quality content with good variety and sufficient length.";
    let result = filter.should_accept(good_content, "hash1");
    assert!(result.accepted);

    // Test duplicate detection
    let result = filter.should_accept(good_content, "hash1");
    assert!(!result.accepted);

    // Test quality filtering
    let bad_content = "a a a a a";
    let result = filter.should_accept(bad_content, "hash2");
    assert!(!result.accepted);
}

#[tokio::test]
async fn test_quality_scoring() {
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

#[tokio::test]
async fn test_metadata_extraction() {
    use cortex_ingestion::extractor::{
        extract_links, extract_headings,
    };

    let content = r#"
# Main Heading

Some text with a [link](https://example.com).

## Sub Heading

More content.
"#;

    let links = extract_links(content);
    assert!(!links.is_empty());

    let headings = extract_headings(content);
    assert_eq!(headings.len(), 2);
}

#[tokio::test]
async fn test_document_ingester() {
    let temp_dir = create_test_project().await;
    let storage = Arc::new(MockStorage);
    let ingester = cortex_ingestion::DocumentIngester::new(storage)
        .with_auto_chunk(true)
        .with_embeddings(true);

    let project_id = CortexId::new();
    let readme_path = temp_dir.path().join("README.md");

    let doc = ingester.ingest_file(project_id, &readme_path).await.unwrap();

    assert_eq!(doc.project_id, project_id);
    assert!(doc.size > 0);
    assert!(!doc.content_hash.is_empty());
}

#[tokio::test]
async fn test_directory_ingestion() {
    let temp_dir = create_test_project().await;
    let storage = Arc::new(MockStorage);
    let ingester = cortex_ingestion::DocumentIngester::new(storage).with_auto_chunk(true);

    let project_id = CortexId::new();
    let docs = ingester
        .ingest_directory(project_id, temp_dir.path())
        .await
        .unwrap();

    assert!(docs.len() >= 5);
}

#[tokio::test]
async fn test_processor_factory() {
    let factory = ProcessorFactory::new();

    // Test PDF processor
    let processor = factory.get(cortex_ingestion::processors::ContentType::Pdf);
    assert!(processor.is_some());

    // Test Markdown processor
    let processor = factory.get_for_path(Path::new("test.md"));
    assert!(processor.is_some());

    // Test JSON processor
    let processor = factory.get_for_mime("application/json");
    assert!(processor.is_some());
}

#[tokio::test]
async fn test_large_file_chunking() {
    use cortex_core::traits::Chunker;

    let chunker = SemanticChunker::new(512, 50);
    let large_text = "This is a sentence. ".repeat(1000);

    let chunks = chunker.chunk(&large_text);
    assert!(!chunks.is_empty());

    // Verify chunk sizes are reasonable
    for chunk in &chunks {
        assert!(chunk.len() <= 600); // Max size + overlap
    }
}

#[tokio::test]
async fn test_malformed_json() {
    let processor = cortex_ingestion::processors::JsonProcessor::new();
    let malformed = b"{ invalid json content }";

    let result = processor.process(malformed).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_file() {
    let processor = cortex_ingestion::processors::TxtProcessor::new();
    let empty = b"";

    let result = processor.process(empty).await.unwrap();
    assert_eq!(result.text_content, "");
}

#[tokio::test]
async fn test_unicode_handling() {
    let processor = cortex_ingestion::processors::TxtProcessor::new();
    let unicode = "Hello 世界 🌍 Здравствуй мир".as_bytes();

    let result = processor.process(unicode).await.unwrap();
    assert!(result.text_content.contains("世界"));
    assert!(result.text_content.contains("🌍"));
    assert!(result.text_content.contains("мир"));
}
