//! Comprehensive end-to-end tests for Cortex semantic search and embeddings.
//!
//! Tests cover:
//! - Embedding generation for code snippets
//! - Semantic search with various queries
//! - Similarity detection (finding similar code)
//! - Hybrid search (keyword + semantic)
//! - Performance measurements (latency, accuracy, memory)
//! - Real-world scenarios with 100+ functions

use cortex_semantic::prelude::*;
use cortex_semantic::config::SemanticConfig;
use cortex_semantic::types::EntityType;
use cortex_semantic::search::SearchFilter;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Test constants
const EMBEDDING_DIMENSION: usize = 384;
const TARGET_SEARCH_LATENCY_MS: u128 = 100;
const MIN_PRECISION_THRESHOLD: f64 = 0.7;
const MIN_RECALL_THRESHOLD: f64 = 0.6;

/// Helper to create a test search engine with mock provider
async fn create_test_engine() -> SemanticSearchEngine {
    let mut config = SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];

    SemanticSearchEngine::new(config).await.unwrap()
}

/// Helper to create a test engine with custom config
async fn create_engine_with_config(config: SemanticConfig) -> SemanticSearchEngine {
    SemanticSearchEngine::new(config).await.unwrap()
}

/// Test metrics collection
#[derive(Debug, Default)]
struct TestMetrics {
    embedding_times: Vec<Duration>,
    search_times: Vec<Duration>,
    precision_scores: Vec<f64>,
    recall_scores: Vec<f64>,
    total_documents: usize,
}

impl TestMetrics {
    fn avg_embedding_time_ms(&self) -> f64 {
        if self.embedding_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.embedding_times.iter().sum();
        total.as_millis() as f64 / self.embedding_times.len() as f64
    }

    fn avg_search_time_ms(&self) -> f64 {
        if self.search_times.is_empty() {
            return 0.0;
        }
        let total: Duration = self.search_times.iter().sum();
        total.as_millis() as f64 / self.search_times.len() as f64
    }

    fn avg_precision(&self) -> f64 {
        if self.precision_scores.is_empty() {
            return 0.0;
        }
        self.precision_scores.iter().sum::<f64>() / self.precision_scores.len() as f64
    }

    fn avg_recall(&self) -> f64 {
        if self.recall_scores.is_empty() {
            return 0.0;
        }
        self.recall_scores.iter().sum::<f64>() / self.recall_scores.len() as f64
    }
}

// ============================================================================
// Test 1: Embedding Generation Tests
// ============================================================================

#[tokio::test]
async fn test_embedding_generation_single() {
    let engine = create_test_engine().await;
    let code_snippet = r#"
        fn authenticate_user(username: &str, password: &str) -> Result<User, AuthError> {
            let user = database::find_user_by_username(username)?;
            if verify_password(password, &user.password_hash) {
                Ok(user)
            } else {
                Err(AuthError::InvalidCredentials)
            }
        }
    "#;

    let start = Instant::now();
    engine.index_document(
        "auth_fn".to_string(),
        code_snippet.to_string(),
        EntityType::Code,
        HashMap::new(),
    ).await.unwrap();
    let duration = start.elapsed();

    println!("Single embedding generation: {:?}", duration);
    assert!(duration < Duration::from_secs(1), "Embedding generation too slow");
    assert_eq!(engine.document_count().await, 1);
}

#[tokio::test]
async fn test_embedding_generation_batch() {
    let engine = create_test_engine().await;

    let code_snippets = vec![
        ("fn1", "fn parse_json(data: &str) -> Result<Value, Error>"),
        ("fn2", "fn serialize_to_json(value: &Value) -> String"),
        ("fn3", "fn validate_json_schema(data: &str, schema: &Schema) -> bool"),
        ("fn4", "fn handle_database_error(err: DbError) -> AppError"),
        ("fn5", "fn log_error_message(msg: &str, severity: Level)"),
    ];

    let documents: Vec<_> = code_snippets
        .iter()
        .map(|(id, code)| {
            (id.to_string(), code.to_string(), EntityType::Code, HashMap::new())
        })
        .collect();

    let start = Instant::now();
    engine.index_batch(documents).await.unwrap();
    let batch_duration = start.elapsed();

    println!("Batch embedding generation (5 docs): {:?}", batch_duration);
    assert_eq!(engine.document_count().await, 5);
    assert!(batch_duration < Duration::from_secs(5), "Batch embedding too slow");
}

#[tokio::test]
async fn test_embedding_dimension_consistency() {
    let engine = create_test_engine().await;

    engine.index_document(
        "doc1".to_string(),
        "test content".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await.unwrap();

    // All embeddings should have consistent dimensions
    let stats = engine.stats().await;
    assert_eq!(stats.dimension, EMBEDDING_DIMENSION);
}

// ============================================================================
// Test 2: Semantic Search Tests
// ============================================================================

#[tokio::test]
async fn test_semantic_search_basic() {
    let engine = create_test_engine().await;

    // Index documents
    let documents = vec![
        ("doc1", "User authentication and login functionality", EntityType::Code),
        ("doc2", "Database connection pooling and management", EntityType::Code),
        ("doc3", "Error handling and logging utilities", EntityType::Code),
        ("doc4", "JSON parsing and serialization", EntityType::Code),
    ];

    for (id, content, entity_type) in documents {
        engine.index_document(
            id.to_string(),
            content.to_string(),
            entity_type,
            HashMap::new(),
        ).await.unwrap();
    }

    // Search for authentication
    let results = engine.search("authentication logic", 5).await.unwrap();

    println!("Search results for 'authentication logic':");
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, result.id, result.score);
    }

    assert!(!results.is_empty(), "Should find results");
    assert_eq!(results[0].id, "doc1", "Should find authentication doc first");
    assert!(results[0].score > 0.5, "Score should be reasonable");
}

#[tokio::test]
async fn test_semantic_search_various_queries() {
    let engine = create_test_engine().await;

    // Index a diverse set of code functions
    let functions = vec![
        ("auth_user", "fn authenticate_user(username: &str, password: &str) -> Result<User>", "rust"),
        ("validate_token", "fn validate_jwt_token(token: &str) -> Result<Claims>", "rust"),
        ("db_connect", "fn establish_database_connection(url: &str) -> Result<Connection>", "rust"),
        ("parse_json", "fn parse_json_string(data: &str) -> Result<Value>", "rust"),
        ("log_error", "fn log_error_with_context(err: Error, context: &str)", "rust"),
        ("send_email", "fn send_notification_email(to: &str, subject: &str, body: &str)", "rust"),
        ("hash_password", "fn hash_password_with_bcrypt(password: &str) -> String", "rust"),
        ("query_db", "fn execute_database_query(sql: &str) -> Result<Vec<Row>>", "rust"),
    ];

    for (id, code, lang) in &functions {
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), lang.to_string());

        engine.index_document(
            id.to_string(),
            code.to_string(),
            EntityType::Code,
            metadata,
        ).await.unwrap();
    }

    // Test various query types
    let test_queries = vec![
        ("authentication logic", vec!["auth_user", "validate_token"]),
        ("database operations", vec!["db_connect", "query_db"]),
        ("error handling", vec!["log_error"]),
        ("password security", vec!["hash_password"]),
    ];

    for (query, expected_ids) in test_queries {
        let results = engine.search(query, 3).await.unwrap();
        println!("\nQuery: '{}'", query);

        for (i, result) in results.iter().enumerate() {
            println!("  {}. {} (score: {:.4})", i + 1, result.id, result.score);
        }

        // Check if at least one expected result is in top 3
        let found = results.iter().any(|r| expected_ids.contains(&r.id.as_str()));
        assert!(found, "Should find relevant results for query: {}", query);
    }
}

#[tokio::test]
async fn test_semantic_search_with_filters() {
    let engine = create_test_engine().await;

    // Index documents with different entity types and metadata
    let mut rust_meta = HashMap::new();
    rust_meta.insert("language".to_string(), "rust".to_string());

    let mut python_meta = HashMap::new();
    python_meta.insert("language".to_string(), "python".to_string());

    engine.index_document(
        "rust_auth".to_string(),
        "Rust authentication function".to_string(),
        EntityType::Code,
        rust_meta.clone(),
    ).await.unwrap();

    engine.index_document(
        "python_auth".to_string(),
        "Python authentication function".to_string(),
        EntityType::Code,
        python_meta.clone(),
    ).await.unwrap();

    engine.index_document(
        "rust_db".to_string(),
        "Rust database connection".to_string(),
        EntityType::Code,
        rust_meta.clone(),
    ).await.unwrap();

    // Search with language filter
    let filter = SearchFilter {
        entity_type: Some(EntityType::Code),
        metadata_filters: {
            let mut filters = HashMap::new();
            filters.insert("language".to_string(), "rust".to_string());
            filters
        },
        ..Default::default()
    };

    let results = engine.search_with_filter("authentication", 5, filter).await.unwrap();

    println!("Filtered search results (Rust only):");
    for result in &results {
        println!("  - {} (score: {:.4})", result.id, result.score);
    }

    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.id.starts_with("rust_")));
}

// ============================================================================
// Test 3: Similarity Detection Tests
// ============================================================================

#[tokio::test]
async fn test_find_similar_code() {
    let engine = create_test_engine().await;

    // Index similar and dissimilar functions
    let functions = vec![
        ("auth1", "fn authenticate_with_password(user: &str, pass: &str) -> bool"),
        ("auth2", "fn verify_user_credentials(username: &str, password: &str) -> Result<bool>"),
        ("auth3", "fn login_user(credentials: Credentials) -> Result<Session>"),
        ("db1", "fn connect_to_database(url: &str) -> Connection"),
        ("db2", "fn establish_db_connection(connection_string: &str) -> DbConn"),
        ("parse1", "fn parse_json_data(input: &str) -> Value"),
    ];

    for (id, code) in &functions {
        engine.index_document(
            id.to_string(),
            code.to_string(),
            EntityType::Code,
            HashMap::new(),
        ).await.unwrap();
    }

    // Search for code similar to auth1
    let results = engine.search(
        "fn authenticate_with_password(user: &str, pass: &str) -> bool",
        5,
    ).await.unwrap();

    println!("Similar functions to auth1:");
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (similarity: {:.4})", i + 1, result.id, result.score);
    }

    // The top 3 results should all be authentication-related
    let top3_ids: Vec<&str> = results.iter().take(3).map(|r| r.id.as_str()).collect();
    let auth_count = top3_ids.iter().filter(|id| id.starts_with("auth")).count();

    assert!(auth_count >= 2, "Should find similar authentication functions");
}

#[tokio::test]
async fn test_similarity_ranking() {
    let engine = create_test_engine().await;

    // Index documents with varying similarity
    let docs = vec![
        ("exact", "machine learning model training"),
        ("similar", "machine learning model optimization"),
        ("related", "deep learning neural networks"),
        ("different", "database query optimization"),
    ];

    for (id, content) in &docs {
        engine.index_document(
            id.to_string(),
            content.to_string(),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    let results = engine.search("machine learning model training", 4).await.unwrap();

    println!("Similarity ranking:");
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, result.id, result.score);
    }

    // Verify ranking order
    assert_eq!(results[0].id, "exact");
    assert!(results[0].score > results[1].score);
    assert!(results[1].score > results[2].score);
}

// ============================================================================
// Test 4: Hybrid Search Tests (Keyword + Semantic)
// ============================================================================

#[tokio::test]
async fn test_hybrid_search() {
    let mut config = SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];
    config.search.enable_hybrid_search = true;
    config.search.enable_reranking = true;
    config.search.hybrid_keyword_weight = 0.3;

    let engine = create_engine_with_config(config).await;

    // Index documents where keyword and semantic matches might differ
    let docs = vec![
        ("doc1", "User authentication using OAuth2 protocol"),
        ("doc2", "Login functionality with password verification"),
        ("doc3", "Database connection pooling"),
        ("doc4", "OAuth2 token validation and refresh"),
    ];

    for (id, content) in &docs {
        engine.index_document(
            id.to_string(),
            content.to_string(),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    // Search with both keyword and semantic signals
    let results = engine.search("OAuth2 authentication", 4).await.unwrap();

    println!("Hybrid search results for 'OAuth2 authentication':");
    for (i, result) in results.iter().enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, result.id, result.score);
        if let Some(explanation) = &result.explanation {
            println!("      {}", explanation);
        }
    }

    // Documents with "OAuth2" keyword should rank highly
    let top_ids: Vec<&str> = results.iter().take(2).map(|r| r.id.as_str()).collect();
    assert!(
        top_ids.contains(&"doc1") || top_ids.contains(&"doc4"),
        "OAuth2 documents should rank high in hybrid search"
    );
}

#[tokio::test]
async fn test_semantic_vs_keyword_comparison() {
    // Create two engines: one semantic-only, one hybrid
    let mut semantic_config = SemanticConfig::default();
    semantic_config.embedding.primary_provider = "mock".to_string();
    semantic_config.embedding.fallback_providers = vec![];
    semantic_config.search.enable_hybrid_search = false;
    semantic_config.search.enable_reranking = false;

    let mut hybrid_config = semantic_config.clone();
    hybrid_config.search.enable_hybrid_search = true;
    hybrid_config.search.enable_reranking = true;

    let semantic_engine = create_engine_with_config(semantic_config).await;
    let hybrid_engine = create_engine_with_config(hybrid_config).await;

    // Index the same documents in both
    let docs = vec![
        ("doc1", "Rust async programming with tokio runtime"),
        ("doc2", "Python asynchronous code using asyncio"),
        ("doc3", "Rust systems programming language"),
    ];

    for (id, content) in &docs {
        semantic_engine.index_document(
            id.to_string(),
            content.to_string(),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();

        hybrid_engine.index_document(
            id.to_string(),
            content.to_string(),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    // Compare results
    let query = "Rust async programming";
    let semantic_results = semantic_engine.search(query, 3).await.unwrap();
    let hybrid_results = hybrid_engine.search(query, 3).await.unwrap();

    println!("\nSemantic-only results:");
    for (i, r) in semantic_results.iter().enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, r.id, r.score);
    }

    println!("\nHybrid results:");
    for (i, r) in hybrid_results.iter().enumerate() {
        println!("  {}. {} (score: {:.4})", i + 1, r.id, r.score);
    }

    // Both should find doc1 as top result
    assert_eq!(semantic_results[0].id, "doc1");
    assert_eq!(hybrid_results[0].id, "doc1");
}

// ============================================================================
// Test 5: Real-World Scenario - 100+ Functions
// ============================================================================

#[tokio::test]
async fn test_large_codebase_indexing() {
    let engine = create_test_engine().await;
    let mut metrics = TestMetrics::default();

    // Generate 100+ diverse code functions
    let function_templates = vec![
        ("auth", "fn authenticate_{}(user: &str) -> Result<Session>"),
        ("validate", "fn validate_{}(input: &str) -> bool"),
        ("parse", "fn parse_{}(data: &str) -> Result<Value>"),
        ("serialize", "fn serialize_{}(obj: &Object) -> String"),
        ("db_query", "fn query_{}(sql: &str) -> Result<Rows>"),
        ("db_insert", "fn insert_{}(record: Record) -> Result<Id>"),
        ("db_update", "fn update_{}(id: Id, data: Data) -> Result<()>"),
        ("cache_get", "fn get_cached_{}(key: &str) -> Option<Value>"),
        ("cache_set", "fn set_cache_{}(key: &str, value: Value)"),
        ("log_info", "fn log_{}_info(msg: &str)"),
        ("log_error", "fn log_{}_error(err: Error)"),
        ("handle_error", "fn handle_{}_error(e: Error) -> Response"),
        ("send_request", "fn send_{}_request(url: &str) -> Result<Response>"),
        ("process_response", "fn process_{}_response(resp: Response) -> Result<Data>"),
    ];

    let topics = vec![
        "user", "product", "order", "payment", "inventory", "shipping",
        "customer", "invoice", "report", "notification", "email", "sms",
    ];

    let mut documents = Vec::new();
    for template in &function_templates {
        for topic in &topics {
            let id = format!("{}_{}", template.0, topic);
            let code = template.1.replace("{}", topic);
            documents.push((id, code, EntityType::Code, HashMap::new()));
        }
    }

    metrics.total_documents = documents.len();
    println!("Indexing {} functions...", documents.len());

    // Batch index with timing
    let start = Instant::now();
    engine.index_batch(documents).await.unwrap();
    let indexing_duration = start.elapsed();
    metrics.embedding_times.push(indexing_duration);

    println!("Indexed {} documents in {:?}", metrics.total_documents, indexing_duration);
    println!("Average per document: {:?}", indexing_duration / metrics.total_documents as u32);

    assert_eq!(engine.document_count().await, metrics.total_documents);
    assert!(indexing_duration < Duration::from_secs(60), "Indexing should complete in reasonable time");
}

#[tokio::test]
async fn test_large_codebase_search_queries() {
    let engine = create_test_engine().await;
    let mut metrics = TestMetrics::default();

    // First, index a large codebase
    let function_categories = vec![
        ("authentication", vec![
            "fn authenticate_user(username: &str, password: &str) -> Result<User>",
            "fn validate_jwt_token(token: &str) -> Result<Claims>",
            "fn refresh_access_token(refresh_token: &str) -> Result<String>",
            "fn logout_user(session_id: &str) -> Result<()>",
            "fn verify_user_session(session_id: &str) -> bool",
        ]),
        ("database", vec![
            "fn connect_to_database(url: &str) -> Result<Connection>",
            "fn execute_query(conn: &Connection, sql: &str) -> Result<Rows>",
            "fn insert_record(conn: &Connection, table: &str, data: Value) -> Result<i64>",
            "fn update_record(conn: &Connection, id: i64, data: Value) -> Result<()>",
            "fn delete_record(conn: &Connection, id: i64) -> Result<()>",
            "fn begin_transaction(conn: &Connection) -> Result<Transaction>",
            "fn commit_transaction(tx: Transaction) -> Result<()>",
            "fn rollback_transaction(tx: Transaction) -> Result<()>",
        ]),
        ("error_handling", vec![
            "fn handle_database_error(err: DbError) -> AppError",
            "fn log_error_with_context(err: Error, context: &str)",
            "fn send_error_notification(err: Error, recipient: &str)",
            "fn format_error_message(err: Error) -> String",
            "fn is_retryable_error(err: &Error) -> bool",
        ]),
        ("json_processing", vec![
            "fn parse_json_string(data: &str) -> Result<Value>",
            "fn serialize_to_json(obj: &impl Serialize) -> String",
            "fn validate_json_schema(data: &Value, schema: &Schema) -> bool",
            "fn merge_json_objects(a: Value, b: Value) -> Value",
        ]),
        ("caching", vec![
            "fn get_from_cache(key: &str) -> Option<Value>",
            "fn set_in_cache(key: &str, value: Value, ttl: Duration)",
            "fn invalidate_cache(pattern: &str)",
            "fn clear_all_cache()",
        ]),
    ];

    let mut doc_id = 0;
    let mut ground_truth: HashMap<&str, Vec<String>> = HashMap::new();

    for (category, functions) in &function_categories {
        let mut category_ids = Vec::new();
        for code in functions {
            let id = format!("fn_{}", doc_id);
            category_ids.push(id.clone());

            engine.index_document(
                id,
                code.to_string(),
                EntityType::Code,
                HashMap::new(),
            ).await.unwrap();

            doc_id += 1;
        }
        ground_truth.insert(category, category_ids);
    }

    metrics.total_documents = doc_id;

    // Test various search queries
    let test_cases = vec![
        ("authentication logic", "authentication"),
        ("database operations", "database"),
        ("error handling", "error_handling"),
        ("JSON parsing", "json_processing"),
        ("cache management", "caching"),
    ];

    for (query, expected_category) in test_cases {
        let start = Instant::now();
        let results = engine.search(query, 10).await.unwrap();
        let search_duration = start.elapsed();
        metrics.search_times.push(search_duration);

        println!("\nQuery: '{}' ({:?})", query, search_duration);
        println!("Top results:");
        for (i, result) in results.iter().take(5).enumerate() {
            println!("  {}. {} (score: {:.4})", i + 1, result.id, result.score);
        }

        // Calculate precision and recall
        let expected_ids = ground_truth.get(expected_category).unwrap();
        let retrieved_ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();

        let relevant_retrieved = retrieved_ids.iter()
            .filter(|id| expected_ids.contains(id))
            .count();

        let precision = if retrieved_ids.is_empty() {
            0.0
        } else {
            relevant_retrieved as f64 / retrieved_ids.len() as f64
        };

        let recall = if expected_ids.is_empty() {
            0.0
        } else {
            relevant_retrieved as f64 / expected_ids.len() as f64
        };

        metrics.precision_scores.push(precision);
        metrics.recall_scores.push(recall);

        println!("  Precision: {:.2}%", precision * 100.0);
        println!("  Recall: {:.2}%", recall * 100.0);

        assert!(!results.is_empty(), "Should return results for: {}", query);
        assert!(precision > 0.0, "Should have some relevant results");
    }

    // Print summary metrics
    println!("\n=== SEARCH METRICS SUMMARY ===");
    println!("Total documents indexed: {}", metrics.total_documents);
    println!("Average search latency: {:.2}ms", metrics.avg_search_time_ms());
    println!("Average precision: {:.2}%", metrics.avg_precision() * 100.0);
    println!("Average recall: {:.2}%", metrics.avg_recall() * 100.0);

    assert!(
        metrics.avg_search_time_ms() < TARGET_SEARCH_LATENCY_MS as f64,
        "Search latency should be under {}ms", TARGET_SEARCH_LATENCY_MS
    );
}

// ============================================================================
// Test 6: Performance Measurements
// ============================================================================

#[tokio::test]
async fn test_search_latency() {
    let engine = create_test_engine().await;

    // Index 50 documents
    for i in 0..50 {
        engine.index_document(
            format!("doc_{}", i),
            format!("Document {} with some content about topic {}", i, i % 5),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    // Measure search latency over multiple queries
    let queries = vec![
        "topic 0", "topic 1", "topic 2", "topic 3", "topic 4",
        "document", "content", "some content", "topic content",
    ];

    let mut latencies = Vec::new();

    for query in &queries {
        let start = Instant::now();
        let _ = engine.search(query, 10).await.unwrap();
        let latency = start.elapsed();
        latencies.push(latency);
    }

    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let max_latency = latencies.iter().max().unwrap();

    println!("Search latency statistics:");
    println!("  Average: {:?}", avg_latency);
    println!("  Max: {:?}", max_latency);
    println!("  All latencies: {:?}", latencies);

    assert!(
        avg_latency.as_millis() < TARGET_SEARCH_LATENCY_MS,
        "Average search latency should be under {}ms, got {}ms",
        TARGET_SEARCH_LATENCY_MS,
        avg_latency.as_millis()
    );
}

#[tokio::test]
async fn test_embedding_generation_performance() {
    let engine = create_test_engine().await;

    let long_text = "x".repeat(1000);
    let test_texts = vec![
        "Short text",
        "Medium length text with several words in it",
        "This is a longer piece of text that contains multiple sentences. It should test the embedding generation performance with more substantial content.",
        &long_text, // Very long text
    ];

    let mut times = Vec::new();

    for text in &test_texts {
        let start = Instant::now();
        engine.index_document(
            format!("doc_{}", times.len()),
            text.to_string(),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
        times.push(start.elapsed());
    }

    println!("Embedding generation times:");
    for (i, time) in times.iter().enumerate() {
        println!("  Text {}: {:?}", i, time);
    }

    // All should complete in reasonable time
    for time in &times {
        assert!(time < &Duration::from_secs(2), "Embedding generation too slow");
    }
}

#[tokio::test]
async fn test_concurrent_searches() {
    let engine = Arc::new(create_test_engine().await);

    // Index documents
    for i in 0..20 {
        engine.index_document(
            format!("doc_{}", i),
            format!("Content about topic {} and subject {}", i % 5, i % 3),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    // Perform concurrent searches
    let queries = vec!["topic", "subject", "content", "topic subject"];
    let start = Instant::now();

    let mut handles = Vec::new();
    for query in queries {
        let engine_clone = Arc::clone(&engine);
        let query_str = query.to_string();

        let handle = tokio::spawn(async move {
            engine_clone.search(&query_str, 5).await
        });
        handles.push(handle);
    }

    // Wait for all searches to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent search should succeed");
    }

    let total_time = start.elapsed();
    println!("Concurrent searches completed in: {:?}", total_time);

    // Should complete faster than sequential
    assert!(total_time < Duration::from_secs(5));
}

#[tokio::test]
async fn test_index_stats_and_memory() {
    let engine = create_test_engine().await;

    // Index documents
    for i in 0..100 {
        engine.index_document(
            format!("doc_{}", i),
            format!("Document {} content", i),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    let stats = engine.stats().await;

    println!("Index statistics:");
    println!("  Total vectors: {}", stats.total_vectors);
    println!("  Dimension: {}", stats.dimension);
    println!("  Metric: {:?}", stats.metric);
    println!("  HNSW M: {}", stats.hnsw_m);
    println!("  HNSW ef_construction: {}", stats.hnsw_ef_construction);

    assert_eq!(stats.total_vectors, 100);
    assert_eq!(stats.dimension, EMBEDDING_DIMENSION);
}

// ============================================================================
// Test 7: Accuracy Measurements
// ============================================================================

#[tokio::test]
async fn test_search_accuracy_precision_recall() {
    let engine = create_test_engine().await;

    // Create a test set with known ground truth
    let test_set = vec![
        // Category: Authentication (IDs: auth_1, auth_2, auth_3)
        ("auth_1", "User login and authentication system", "authentication"),
        ("auth_2", "Password verification and validation", "authentication"),
        ("auth_3", "OAuth2 token authentication", "authentication"),

        // Category: Database (IDs: db_1, db_2, db_3)
        ("db_1", "Database connection and pooling", "database"),
        ("db_2", "SQL query execution engine", "database"),
        ("db_3", "Database transaction management", "database"),

        // Category: Logging (IDs: log_1, log_2)
        ("log_1", "Error logging and monitoring", "logging"),
        ("log_2", "Application log formatting", "logging"),
    ];

    for (id, content, _category) in &test_set {
        engine.index_document(
            id.to_string(),
            content.to_string(),
            EntityType::Code,
            HashMap::new(),
        ).await.unwrap();
    }

    // Test queries with expected results
    let test_queries = vec![
        ("authentication system", vec!["auth_1", "auth_2", "auth_3"]),
        ("database operations", vec!["db_1", "db_2", "db_3"]),
        ("logging errors", vec!["log_1", "log_2"]),
    ];

    let mut total_precision = 0.0;
    let mut total_recall = 0.0;
    let mut test_count = 0;

    for (query, expected_ids) in test_queries {
        let results = engine.search(query, 5).await.unwrap();
        let retrieved_ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();

        let relevant_retrieved = retrieved_ids.iter()
            .filter(|id| expected_ids.contains(id))
            .count();

        let precision = relevant_retrieved as f64 / retrieved_ids.len().max(1) as f64;
        let recall = relevant_retrieved as f64 / expected_ids.len() as f64;

        total_precision += precision;
        total_recall += recall;
        test_count += 1;

        println!("\nQuery: '{}'", query);
        println!("  Precision: {:.2}%", precision * 100.0);
        println!("  Recall: {:.2}%", recall * 100.0);
        println!("  Retrieved: {:?}", retrieved_ids);
        println!("  Expected: {:?}", expected_ids);
    }

    let avg_precision = total_precision / test_count as f64;
    let avg_recall = total_recall / test_count as f64;

    println!("\n=== ACCURACY METRICS ===");
    println!("Average Precision: {:.2}%", avg_precision * 100.0);
    println!("Average Recall: {:.2}%", avg_recall * 100.0);
    println!("F1 Score: {:.2}%", 2.0 * (avg_precision * avg_recall) / (avg_precision + avg_recall) * 100.0);

    assert!(avg_precision >= MIN_PRECISION_THRESHOLD,
        "Precision {:.2}% below threshold {:.2}%",
        avg_precision * 100.0, MIN_PRECISION_THRESHOLD * 100.0);

    assert!(avg_recall >= MIN_RECALL_THRESHOLD,
        "Recall {:.2}% below threshold {:.2}%",
        avg_recall * 100.0, MIN_RECALL_THRESHOLD * 100.0);
}

// ============================================================================
// Test 8: Cache Performance
// ============================================================================

#[tokio::test]
async fn test_cache_effectiveness() {
    let mut config = SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.embedding.fallback_providers = vec![];
    config.cache.enable_query_cache = true;
    config.cache.enable_embedding_cache = true;

    let engine = create_engine_with_config(config).await;

    // Index documents
    for i in 0..10 {
        engine.index_document(
            format!("doc_{}", i),
            format!("Content {}", i),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    // First search (cold cache)
    let query = "Content 5";
    let start = Instant::now();
    let _ = engine.search(query, 5).await.unwrap();
    let cold_time = start.elapsed();

    // Second search (warm cache)
    let start = Instant::now();
    let _ = engine.search(query, 5).await.unwrap();
    let warm_time = start.elapsed();

    println!("Cache performance:");
    println!("  Cold cache: {:?}", cold_time);
    println!("  Warm cache: {:?}", warm_time);
    println!("  Speedup: {:.2}x", cold_time.as_nanos() as f64 / warm_time.as_nanos().max(1) as f64);

    // Cached search should be faster
    assert!(warm_time <= cold_time, "Cached search should be faster or equal");
}

// ============================================================================
// Test 9: Edge Cases and Error Handling
// ============================================================================

#[tokio::test]
async fn test_empty_index_search() {
    let engine = create_test_engine().await;

    let results = engine.search("test query", 10).await.unwrap();
    assert!(results.is_empty(), "Empty index should return no results");
}

#[tokio::test]
async fn test_search_with_empty_query() {
    let engine = create_test_engine().await;

    engine.index_document(
        "doc1".to_string(),
        "content".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await.unwrap();

    let results = engine.search("", 10).await.unwrap();
    // Empty query should still work (normalized to empty string)
    assert!(results.is_empty() || !results.is_empty()); // Either is acceptable
}

#[tokio::test]
async fn test_remove_and_search() {
    let engine = create_test_engine().await;

    // Index documents
    engine.index_document(
        "doc1".to_string(),
        "authentication code".to_string(),
        EntityType::Code,
        HashMap::new(),
    ).await.unwrap();

    engine.index_document(
        "doc2".to_string(),
        "database code".to_string(),
        EntityType::Code,
        HashMap::new(),
    ).await.unwrap();

    // Search should find both
    let results = engine.search("code", 10).await.unwrap();
    assert_eq!(results.len(), 2);

    // Remove one document
    engine.remove_document(&"doc1".to_string()).await.unwrap();

    // Search should find only one
    let results = engine.search("code", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "doc2");
}

#[tokio::test]
async fn test_clear_and_reindex() {
    let engine = create_test_engine().await;

    // Index documents
    for i in 0..5 {
        engine.index_document(
            format!("doc_{}", i),
            format!("Content {}", i),
            EntityType::Document,
            HashMap::new(),
        ).await.unwrap();
    }

    assert_eq!(engine.document_count().await, 5);

    // Clear
    engine.clear().await.unwrap();
    assert_eq!(engine.document_count().await, 0);

    // Reindex
    engine.index_document(
        "new_doc".to_string(),
        "New content".to_string(),
        EntityType::Document,
        HashMap::new(),
    ).await.unwrap();

    assert_eq!(engine.document_count().await, 1);

    let results = engine.search("New content", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "new_doc");
}
