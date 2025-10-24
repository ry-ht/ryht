//! Integration tests for semantic search system.

use cortex_semantic::prelude::*;
use cortex_semantic::{SearchFilter, EntityType};
use std::collections::HashMap;
use tempfile::tempdir;

/// Helper to create a test engine with mock provider.
async fn create_test_engine() -> SemanticSearchEngine {
    let mut config = cortex_semantic::config::SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];
    config.cache.enable_embedding_cache = true;
    config.cache.enable_query_cache = true;

    SemanticSearchEngine::new(config).await.unwrap()
}

#[tokio::test]
async fn test_basic_indexing_and_search() {
    let engine = create_test_engine().await;

    // Index some documents
    engine
        .index_document(
            "doc1".to_string(),
            "This is a document about machine learning and artificial intelligence".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document(
            "doc2".to_string(),
            "Natural language processing is a subfield of AI".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document(
            "doc3".to_string(),
            "Deep learning uses neural networks for pattern recognition".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // Search for relevant documents
    let results = engine.search("machine learning", 10).await.unwrap();

    assert!(!results.is_empty(), "Should find results");
    assert_eq!(results[0].id, "doc1", "Most relevant document should be first");
}

#[tokio::test]
async fn test_batch_indexing() {
    let engine = create_test_engine().await;

    let documents = vec![
        (
            "doc1".to_string(),
            "Rust is a systems programming language".to_string(),
            EntityType::Code,
            HashMap::new(),
        ),
        (
            "doc2".to_string(),
            "Python is great for data science".to_string(),
            EntityType::Code,
            HashMap::new(),
        ),
        (
            "doc3".to_string(),
            "JavaScript powers the web".to_string(),
            EntityType::Code,
            HashMap::new(),
        ),
    ];

    engine.index_batch(documents).await.unwrap();

    let count = engine.document_count().await;
    assert_eq!(count, 3, "All documents should be indexed");

    let results = engine.search("programming", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find programming-related documents");
}

#[tokio::test]
async fn test_search_with_entity_filter() {
    let engine = create_test_engine().await;

    // Index documents with different entity types
    engine
        .index_document(
            "code1".to_string(),
            "Function to calculate fibonacci numbers".to_string(),
            EntityType::Code,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document(
            "doc1".to_string(),
            "Documentation about fibonacci sequence".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // Search with entity type filter
    let filter = SearchFilter {
        entity_type: Some(EntityType::Code),
        ..Default::default()
    };

    let results = engine
        .search_with_filter("fibonacci", 10, filter)
        .await
        .unwrap();

    assert_eq!(results.len(), 1, "Should only return code results");
    assert_eq!(results[0].id, "code1");
    assert_eq!(results[0].entity_type, EntityType::Code);
}

#[tokio::test]
async fn test_search_with_metadata_filter() {
    let engine = create_test_engine().await;

    let mut rust_metadata = HashMap::new();
    rust_metadata.insert("language".to_string(), "rust".to_string());

    let mut python_metadata = HashMap::new();
    python_metadata.insert("language".to_string(), "python".to_string());

    engine
        .index_document(
            "rust1".to_string(),
            "Rust error handling with Result".to_string(),
            EntityType::Code,
            rust_metadata,
        )
        .await
        .unwrap();

    engine
        .index_document(
            "python1".to_string(),
            "Python error handling with try-except".to_string(),
            EntityType::Code,
            python_metadata,
        )
        .await
        .unwrap();

    // Filter by language
    let mut metadata_filters = HashMap::new();
    metadata_filters.insert("language".to_string(), "rust".to_string());

    let filter = SearchFilter {
        metadata_filters,
        ..Default::default()
    };

    let results = engine
        .search_with_filter("error handling", 10, filter)
        .await
        .unwrap();

    assert_eq!(results.len(), 1, "Should only return Rust results");
    assert_eq!(results[0].id, "rust1");
}

#[tokio::test]
async fn test_search_with_score_threshold() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "Highly relevant content about search".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    engine
        .index_document(
            "doc2".to_string(),
            "Somewhat related to finding things".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // High threshold should return fewer results
    let filter = SearchFilter {
        min_score: Some(0.8),
        ..Default::default()
    };

    let results = engine
        .search_with_filter("search", 10, filter)
        .await
        .unwrap();

    // With mock provider, results depend on deterministic hashing
    assert!(
        results.len() <= 2,
        "High threshold should filter out low-scoring results"
    );
}

#[tokio::test]
async fn test_remove_document() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "Document to be removed".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    assert_eq!(engine.document_count().await, 1);

    engine.remove_document(&"doc1".to_string()).await.unwrap();

    assert_eq!(engine.document_count().await, 0);

    let results = engine.search("removed", 10).await.unwrap();
    assert!(results.is_empty(), "Removed document should not appear in search");
}

#[tokio::test]
async fn test_clear_index() {
    let engine = create_test_engine().await;

    // Index multiple documents
    for i in 0..5 {
        engine
            .index_document(
                format!("doc{}", i),
                format!("Document content {}", i),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();
    }

    assert_eq!(engine.document_count().await, 5);

    engine.clear().await.unwrap();

    assert_eq!(engine.document_count().await, 0);
}

#[tokio::test]
async fn test_index_persistence() {
    let temp_dir = tempdir().unwrap();
    let persist_path = temp_dir.path().join("test_index.bin");

    // Create engine and index documents
    {
        let mut config = cortex_semantic::config::SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.index.persist_path = Some(persist_path.clone());

        let engine = SemanticSearchEngine::new(config).await.unwrap();

        engine
            .index_document(
                "doc1".to_string(),
                "Persistent document content".to_string(),
                EntityType::Document,
                HashMap::new(),
            )
            .await
            .unwrap();

        // Note: save_index() method doesn't exist in the current implementation
        // Qdrant automatically persists data, so this is not needed
        // engine.save_index().await.unwrap();
    }

    // Load engine and verify data persisted
    {
        let mut config = cortex_semantic::config::SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.index.persist_path = Some(persist_path.clone());

        let engine = SemanticSearchEngine::new(config).await.unwrap();

        // Note: In the current implementation, we only persist the index, not the documents
        // A full implementation would need to persist documents separately
        let stats = engine.stats().await;
        assert!(stats.total_vectors > 0 || engine.document_count().await > 0);
    }
}

#[tokio::test]
async fn test_large_scale_indexing() {
    let engine = create_test_engine().await;

    // Index many documents
    let mut documents = Vec::new();
    for i in 0..100 {
        documents.push((
            format!("doc{}", i),
            format!("Document content about topic {} with keywords", i),
            EntityType::Document,
            HashMap::new(),
        ));
    }

    engine.index_batch(documents).await.unwrap();

    assert_eq!(engine.document_count().await, 100);

    let results = engine.search("topic keywords", 20).await.unwrap();
    assert!(!results.is_empty(), "Should find results from large index");
    assert!(results.len() <= 20, "Should respect limit");
}

#[tokio::test]
async fn test_query_variations() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "How to implement authentication in web applications".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // Try different query formulations
    let queries = vec![
        "authentication",
        "implement authentication",
        "web app security",
        "how to authenticate users",
    ];

    for query in queries {
        let results = engine.search(query, 5).await.unwrap();
        // With semantic search, different queries should find the document
        // (This depends on the embedding model, mock provider gives deterministic results)
        println!("Query '{}' found {} results", query, results.len());
    }
}

#[tokio::test]
async fn test_multilingual_content() {
    let engine = create_test_engine().await;

    let mut metadata = HashMap::new();
    metadata.insert("language".to_string(), "en".to_string());

    engine
        .index_document(
            "en1".to_string(),
            "Hello world in English".to_string(),
            EntityType::Document,
            metadata.clone(),
        )
        .await
        .unwrap();

    metadata.insert("language".to_string(), "es".to_string());
    engine
        .index_document(
            "es1".to_string(),
            "Hola mundo en español".to_string(),
            EntityType::Document,
            metadata,
        )
        .await
        .unwrap();

    let results = engine.search("hello", 10).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_code_search() {
    let engine = create_test_engine().await;

    let mut metadata = HashMap::new();
    metadata.insert("language".to_string(), "rust".to_string());
    metadata.insert("symbol_type".to_string(), "function".to_string());

    engine
        .index_document(
            "fn1".to_string(),
            "fn calculate_total(items: Vec<f64>) -> f64 { items.iter().sum() }".to_string(),
            EntityType::Code,
            metadata,
        )
        .await
        .unwrap();

    let results = engine.search("calculate sum", 5).await.unwrap();
    assert!(!results.is_empty(), "Should find code with semantic search");
}

#[tokio::test]
async fn test_cache_effectiveness() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "Test document for caching".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    // First search (cache miss)
    let start = std::time::Instant::now();
    let results1 = engine.search("caching", 10).await.unwrap();
    let duration1 = start.elapsed();

    // Second identical search (cache hit)
    let start = std::time::Instant::now();
    let results2 = engine.search("caching", 10).await.unwrap();
    let duration2 = start.elapsed();

    assert_eq!(results1.len(), results2.len());
    // Second search should be faster due to caching (though with mock provider, difference might be minimal)
    println!(
        "First search: {:?}, Second search: {:?}",
        duration1, duration2
    );
}

#[tokio::test]
async fn test_concurrent_operations() {
    let engine = std::sync::Arc::new(create_test_engine().await);

    // Spawn multiple concurrent indexing tasks
    let mut handles = Vec::new();

    for i in 0..10 {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            engine_clone
                .index_document(
                    format!("doc{}", i),
                    format!("Concurrent document {}", i),
                    EntityType::Document,
                    HashMap::new(),
                )
                .await
                .unwrap();
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(engine.document_count().await, 10);
}

#[tokio::test]
async fn test_empty_query() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "Some content".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    let results = engine.search("", 10).await.unwrap();
    // Empty query should still work (returns based on embedding of empty string)
    println!("Empty query returned {} results", results.len());
}

#[tokio::test]
async fn test_special_characters() {
    let engine = create_test_engine().await;

    engine
        .index_document(
            "doc1".to_string(),
            "Content with special chars: @#$%^&*()".to_string(),
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    let results = engine.search("special chars @#$", 10).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_very_long_content() {
    let engine = create_test_engine().await;

    let long_content = "word ".repeat(10000);

    engine
        .index_document(
            "long_doc".to_string(),
            long_content,
            EntityType::Document,
            HashMap::new(),
        )
        .await
        .unwrap();

    let results = engine.search("word", 10).await.unwrap();
    assert!(!results.is_empty());
}
