use super::*;
use std::fs;
use tempfile::TempDir;

fn create_test_markdown() -> String {
    r#"# Test Document

This is a test document for testing markdown parsing.

## Section 1

Some content in section 1.

```rust
fn example() {
    println!("Hello, world!");
}
```

### Subsection 1.1

More detailed content here.

## Section 2

Content for section 2 with a [link](https://example.com).

```typescript
function test() {
    console.log("TypeScript example");
}
```
"#.to_string()
}

fn create_test_rust_doc() -> String {
    r#"
/// Main entry point for the application
///
/// This function initializes the system and starts processing.
pub fn main() {
    println!("Starting application");
}

/// Calculate the sum of two numbers
///
/// # Arguments
/// * `a` - First number
/// * `b` - Second number
///
/// # Returns
/// The sum of `a` and `b`
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#.to_string()
}

fn create_test_jsdoc() -> String {
    r#"
/**
 * Calculates the factorial of a number
 * @param {number} n - The number to calculate factorial for
 * @returns {number} The factorial result
 */
function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

/**
 * User class for managing user data
 * @class
 */
class User {
    constructor(name, email) {
        this.name = name;
        this.email = email;
    }
}
"#.to_string()
}

#[tokio::test]
async fn test_parse_markdown() {
    let content = create_test_markdown();
    let result = parser::parse_markdown(&content, "test.md");

    assert!(result.is_ok());
    let doc = result.unwrap();

    // Should have sections
    assert!(doc.sections.len() >= 3);

    // Should have code blocks
    assert_eq!(doc.code_blocks.len(), 2);
    assert_eq!(doc.code_blocks[0].language, Some("rust".to_string()));
    assert_eq!(doc.code_blocks[1].language, Some("typescript".to_string()));

    // Should have links
    assert_eq!(doc.links.len(), 1);
    assert_eq!(doc.links[0], "https://example.com");

    // Check section hierarchy
    let section1 = &doc.sections[0];
    assert_eq!(section1.title, "Test Document");
    assert_eq!(section1.level, 1);
}

#[tokio::test]
async fn test_parse_rust_doc_comments() {
    let content = create_test_rust_doc();
    let result = parser::parse_doc_comments(&content, "test.rs", parser::DocFormat::RustDoc);

    assert!(result.is_ok());
    let entries = result.unwrap();

    // Should extract both doc comments
    assert_eq!(entries.len(), 2);

    // Check first entry
    assert!(entries[0].content.contains("Main entry point"));
    assert_eq!(entries[0].doc_type, DocType::DocComment);

    // Check second entry
    assert!(entries[1].content.contains("Calculate the sum"));
}

#[tokio::test]
async fn test_parse_jsdoc_comments() {
    let content = create_test_jsdoc();
    let result = parser::parse_doc_comments(&content, "test.js", parser::DocFormat::JSDoc);

    assert!(result.is_ok());
    let entries = result.unwrap();

    // Should extract JSDoc comments
    assert!(entries.len() >= 2);

    // Check factorial documentation
    assert!(entries[0].content.contains("factorial"));
}

#[tokio::test]
async fn test_doc_indexer_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.md");
    fs::write(&test_file, create_test_markdown()).unwrap();

    let indexer = DocIndexer::new();
    let result = indexer.index_markdown_file(&test_file).await;

    assert!(result.is_ok());
    let count = result.unwrap();
    assert!(count > 0);

    // Test search
    let search_results = indexer.search_docs("section", 10).await;
    assert!(search_results.is_ok());
    let results = search_results.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_doc_indexer_source_files() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, create_test_rust_doc()).unwrap();

    let indexer = DocIndexer::new();
    let result = indexer.index_source_file(&test_file).await;

    assert!(result.is_ok());
    let count = result.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_search_relevance() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.md");
    fs::write(&test_file, create_test_markdown()).unwrap();

    let indexer = DocIndexer::new();
    indexer.index_markdown_file(&test_file).await.unwrap();

    // Search for specific term
    let results = indexer.search_docs("section 1", 10).await.unwrap();

    // Should find results
    assert!(!results.is_empty());

    // First result should have highest relevance
    if results.len() > 1 {
        assert!(results[0].relevance >= results[1].relevance);
    }
}

#[tokio::test]
async fn test_symbol_to_docs_linking() {
    let indexer = DocIndexer::new();

    // Create and index test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");
    fs::write(&test_file, create_test_rust_doc()).unwrap();

    let count = indexer.index_source_file(&test_file).await.unwrap();
    assert_eq!(count, 2, "Should index 2 doc comments");

    // Test 1: get_docs_for_symbol with search fallback (no explicit link)
    // The word "calculate" appears in the doc comment title
    let docs = indexer.get_docs_for_symbol("calculate").await.unwrap();
    assert!(!docs.is_empty(), "Should find docs via search fallback for 'calculate'");
    assert!(docs[0].title.to_lowercase().contains("calculate") ||
            docs[0].content.to_lowercase().contains("calculate"));

    // Test 2: Verify get_docs_for_file works
    let file_docs = indexer.get_docs_for_file(&test_file);
    assert_eq!(file_docs.len(), 2, "Should have 2 doc entries for the file");

    // Test 3: Test fallback with different search terms
    let sum_docs = indexer.get_docs_for_symbol("sum").await.unwrap();
    assert!(!sum_docs.is_empty(), "Should find docs containing 'sum'");

    let main_docs = indexer.get_docs_for_symbol("main").await.unwrap();
    assert!(!main_docs.is_empty(), "Should find docs containing 'main'");
}

#[tokio::test]
async fn test_doc_indexer_stats() {
    let temp_dir = TempDir::new().unwrap();

    let md_file = temp_dir.path().join("test.md");
    fs::write(&md_file, create_test_markdown()).unwrap();

    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, create_test_rust_doc()).unwrap();

    let indexer = DocIndexer::new();
    indexer.index_markdown_file(&md_file).await.unwrap();
    indexer.index_source_file(&rs_file).await.unwrap();

    let stats = indexer.stats();
    assert!(stats.total_entries > 0);
    assert!(stats.total_files > 0);
    assert!(stats.total_markdown > 0);
    assert!(stats.total_doc_comments > 0);
}

#[tokio::test]
async fn test_search_performance() {
    use std::time::Instant;

    let temp_dir = TempDir::new().unwrap();

    // Create multiple markdown files
    for i in 0..10 {
        let file = temp_dir.path().join(format!("test_{}.md", i));
        fs::write(&file, create_test_markdown()).unwrap();
    }

    let indexer = DocIndexer::new();

    // Index all files
    for i in 0..10 {
        let file = temp_dir.path().join(format!("test_{}.md", i));
        indexer.index_markdown_file(&file).await.unwrap();
    }

    // Measure search performance
    let start = Instant::now();
    let _results = indexer.search_docs("section", 10).await.unwrap();
    let elapsed = start.elapsed();

    // Should be fast (< 50ms)
    assert!(elapsed.as_millis() < 50, "Search took {}ms, expected < 50ms", elapsed.as_millis());
}

#[tokio::test]
async fn test_empty_markdown() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("empty.md");
    fs::write(&test_file, "").unwrap();

    let indexer = DocIndexer::new();
    let result = indexer.index_markdown_file(&test_file).await;

    // Should handle empty files gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_malformed_markdown() {
    let malformed = r#"
# Incomplete heading
Some text without proper structure

```rust
// Unclosed code block
fn test() {
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("malformed.md");
    fs::write(&test_file, malformed).unwrap();

    let indexer = DocIndexer::new();
    let result = indexer.index_markdown_file(&test_file).await;

    // Should handle malformed markdown gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_context_extraction() {
    // This is tested internally by search_docs
    // Just verify that search results include relevant context
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.md");
    let content = "This is a very long piece of content that should be truncated when extracting context around a search term. The search term appears here in the middle.";
    fs::write(&test_file, content).unwrap();

    let indexer = DocIndexer::new();
    indexer.index_markdown_file(&test_file).await.unwrap();

    let results = indexer.search_docs("search term", 10).await.unwrap();
    if !results.is_empty() {
        assert!(results[0].content.contains("search"));
    }
}

#[tokio::test]
async fn test_detect_format() {
    use std::path::Path;

    assert_eq!(parser::detect_format(Path::new("test.md")), Some(parser::DocFormat::Markdown));
    assert_eq!(parser::detect_format(Path::new("test.rs")), Some(parser::DocFormat::RustDoc));
    assert_eq!(parser::detect_format(Path::new("test.js")), Some(parser::DocFormat::JSDoc));
    assert_eq!(parser::detect_format(Path::new("test.py")), Some(parser::DocFormat::PyDoc));
    assert_eq!(parser::detect_format(Path::new("test.go")), Some(parser::DocFormat::GoDoc));
    assert_eq!(parser::detect_format(Path::new("test.txt")), None);
}

#[tokio::test]
async fn test_multiple_languages() {
    let temp_dir = TempDir::new().unwrap();

    let indexer = DocIndexer::new();

    // Index Rust file
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, create_test_rust_doc()).unwrap();
    indexer.index_source_file(&rs_file).await.unwrap();

    // Index JavaScript file
    let js_file = temp_dir.path().join("test.js");
    fs::write(&js_file, create_test_jsdoc()).unwrap();
    indexer.index_source_file(&js_file).await.unwrap();

    let stats = indexer.stats();
    assert!(stats.total_doc_comments >= 3); // At least 2 from Rust + 2 from JS
}
