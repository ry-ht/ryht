//! Integration tests for HNSW vector index implementation

use cortex_semantic::{
    config::IndexConfig,
    index::{HNSWIndex, VectorIndex},
    types::SimilarityMetric,
};

fn create_test_vector(dimension: usize, seed: u64) -> Vec<f32> {
    let mut vec = Vec::with_capacity(dimension);
    for i in 0..dimension {
        vec.push(((seed + i as u64) % 100) as f32 / 100.0);
    }
    // Normalize
    let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag > 0.0 {
        for val in &mut vec {
            *val /= mag;
        }
    }
    vec
}

#[tokio::test]
async fn test_hnsw_basic_functionality() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    // Insert vectors
    let vec1 = create_test_vector(128, 1);
    let vec2 = create_test_vector(128, 100);
    let vec3 = create_test_vector(128, 200);

    index.insert("doc1".to_string(), vec1.clone()).await.unwrap();
    index.insert("doc2".to_string(), vec2.clone()).await.unwrap();
    index.insert("doc3".to_string(), vec3.clone()).await.unwrap();

    assert_eq!(index.len().await, 3);

    // Search - should find the exact match first
    let results = index.search(&vec1, 3).await.unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].doc_id, "doc1");
    assert!(results[0].score > 0.9); // Should be very similar to itself
}

#[tokio::test]
async fn test_hnsw_large_index() {
    let config = IndexConfig {
        hnsw_m: 16,
        hnsw_ef_construction: 200,
        hnsw_ef_search: 50,
        similarity_metric: SimilarityMetric::Cosine,
        persist_path: None,
        auto_save_interval_seconds: 0,
        max_index_size: 100_000,
    };

    let index = HNSWIndex::new(config, 384).unwrap();

    // Insert 1000 vectors
    for i in 0..1000 {
        let vec = create_test_vector(384, i);
        let doc_id = format!("doc_{}", i);
        index.insert(doc_id, vec).await.unwrap();
    }

    assert_eq!(index.len().await, 1000);

    // Search should work efficiently
    let query = create_test_vector(384, 500);
    let results = index.search(&query, 10).await.unwrap();

    assert_eq!(results.len(), 10);
    // The exact match should be in top results
    assert!(results.iter().any(|r| r.doc_id == "doc_500"));
}

#[tokio::test]
async fn test_hnsw_removal() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    // Insert multiple vectors
    for i in 0..10 {
        let vec = create_test_vector(128, i);
        index.insert(format!("doc{}", i), vec).await.unwrap();
    }

    assert_eq!(index.len().await, 10);

    // Remove some vectors
    index.remove(&"doc5".to_string()).await.unwrap();
    index.remove(&"doc7".to_string()).await.unwrap();

    assert_eq!(index.len().await, 8);

    // Search should still work
    let query = create_test_vector(128, 3);
    let results = index.search(&query, 5).await.unwrap();

    assert!(results.len() <= 5);
    // Removed documents should not appear
    assert!(!results.iter().any(|r| r.doc_id == "doc5"));
    assert!(!results.iter().any(|r| r.doc_id == "doc7"));
}

#[tokio::test]
async fn test_hnsw_batch_insert() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    let mut items = Vec::new();
    for i in 0..50 {
        let vec = create_test_vector(128, i);
        let doc_id = format!("doc_{}", i);
        items.push((doc_id, vec));
    }

    index.insert_batch(items).await.unwrap();
    assert_eq!(index.len().await, 50);

    // Verify search works after batch insert
    let query = create_test_vector(128, 25);
    let results = index.search(&query, 10).await.unwrap();
    assert_eq!(results.len(), 10);
}

#[tokio::test]
async fn test_hnsw_persistence() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let index_path = temp_dir.path().join("hnsw_test.bin");

    // Create and populate index
    {
        let config = IndexConfig::default();
        let index = HNSWIndex::new(config, 128).unwrap();

        for i in 0..20 {
            let vec = create_test_vector(128, i);
            index.insert(format!("doc{}", i), vec).await.unwrap();
        }

        index.save(&index_path).await.unwrap();
    }

    // Load index and verify
    {
        let config = IndexConfig::default();
        let mut index = HNSWIndex::new(config, 128).unwrap();
        index.load(&index_path).await.unwrap();

        assert_eq!(index.len().await, 20);

        // Verify search works after loading
        let query = create_test_vector(128, 10);
        let results = index.search(&query, 5).await.unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].doc_id, "doc10");
    }
}

#[tokio::test]
async fn test_hnsw_rebuild_threshold() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    // Insert vectors to trigger rebuild (threshold is 1000)
    for i in 0..1100 {
        let vec = create_test_vector(128, i);
        index.insert(format!("doc_{}", i), vec).await.unwrap();
    }

    // Search should still work efficiently after rebuild
    let query = create_test_vector(128, 500);
    let results = index.search(&query, 10).await.unwrap();

    assert_eq!(results.len(), 10);
    assert!(results.iter().any(|r| r.doc_id == "doc_500"));
}

#[tokio::test]
async fn test_hnsw_different_k_values() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    // Insert 100 vectors
    for i in 0..100 {
        let vec = create_test_vector(128, i);
        index.insert(format!("doc_{}", i), vec).await.unwrap();
    }

    let query = create_test_vector(128, 50);

    // Test different k values
    for k in [1, 5, 10, 20, 50, 100] {
        let results = index.search(&query, k).await.unwrap();
        assert_eq!(results.len(), k.min(100));
    }
}

#[tokio::test]
async fn test_hnsw_empty_index() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    let query = create_test_vector(128, 1);
    let results = index.search(&query, 10).await.unwrap();

    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_hnsw_dimension_validation() {
    let config = IndexConfig::default();
    let index = HNSWIndex::new(config, 128).unwrap();

    // Try to insert vector with wrong dimension
    let wrong_vec = vec![0.0; 64]; // Wrong dimension
    let result = index.insert("doc1".to_string(), wrong_vec).await;

    assert!(result.is_err());
}
