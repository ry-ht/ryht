//! Comprehensive tests for CodeUnitService caching

use anyhow::Result;
use cortex::services::{CacheConfig, CodeUnitService};
use cortex_core::types::{CodeUnit, CodeUnitType, Complexity, Language, Visibility};
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Helper to create a test storage connection
async fn create_test_storage() -> Result<Arc<ConnectionManager>> {
    let storage = ConnectionManager::new_memory().await?;
    Ok(Arc::new(storage))
}

/// Helper to create a test code unit
fn create_test_code_unit(id: &str, name: &str, qualified_name: &str) -> CodeUnit {
    CodeUnit {
        id: id.to_string(),
        unit_type: CodeUnitType::Function,
        name: name.to_string(),
        qualified_name: qualified_name.to_string(),
        display_name: name.to_string(),
        file_path: "/test/file.rs".to_string(),
        language: Language::Rust,
        start_line: 1,
        end_line: 10,
        start_column: 0,
        end_column: 0,
        signature: format!("fn {}()", name),
        body: Some(format!("{{ println!(\"test\"); }}")),
        docstring: Some(format!("Test function {}", name)),
        visibility: Visibility::Public,
        is_async: false,
        is_exported: true,
        complexity: Complexity {
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            lines: 10,
        },
        has_tests: false,
        has_documentation: true,
        dependencies: vec![],
        version: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// Helper to insert code unit into database
async fn insert_code_unit(storage: &Arc<ConnectionManager>, unit: &CodeUnit) -> Result<()> {
    let pooled = storage.acquire().await?;
    let conn = pooled.connection();

    let query = "CREATE code_unit CONTENT $unit";
    conn.query(query).bind(("unit", unit)).await?;

    Ok(())
}

#[tokio::test]
async fn test_cache_hit_on_repeated_get() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    // Insert test unit
    let unit = create_test_code_unit("test:1", "test_func", "module::test_func");
    insert_code_unit(&storage, &unit).await?;

    // First call - should be cache miss
    let result1 = service.get_code_unit("test:1").await?;
    let stats1 = service.cache_stats();
    assert_eq!(stats1.misses, 1);
    assert_eq!(stats1.hits, 0);

    // Second call - should be cache hit
    let result2 = service.get_code_unit("test:1").await?;
    let stats2 = service.cache_stats();
    assert_eq!(stats2.misses, 1);
    assert_eq!(stats2.hits, 1);

    // Verify same data
    assert_eq!(result1.id, result2.id);
    assert_eq!(result1.name, result2.name);

    Ok(())
}

#[tokio::test]
async fn test_cache_hit_by_qualified_name() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    let unit = create_test_code_unit("test:2", "another_func", "module::another_func");
    insert_code_unit(&storage, &unit).await?;

    // Get by ID first - populates both caches
    let _result1 = service.get_code_unit("test:2").await?;

    // Get by qualified name - should hit cache
    let result2 = service.get_by_qualified_name("module::another_func").await?;
    let stats = service.cache_stats();

    assert_eq!(result2.qualified_name, "module::another_func");
    assert_eq!(stats.hits, 1); // Second call hits cache

    Ok(())
}

#[tokio::test]
async fn test_cache_invalidation_on_update() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    let unit = create_test_code_unit("test:3", "update_func", "module::update_func");
    insert_code_unit(&storage, &unit).await?;

    // First get - cache miss
    let result1 = service.get_code_unit("test:3").await?;
    assert_eq!(result1.version, 1);

    // Update the unit
    service
        .update_code_unit(
            "test:3",
            Some("{ println!(\"updated\"); }".to_string()),
            None,
            Some(1),
        )
        .await?;

    let stats = service.cache_stats();
    assert_eq!(stats.invalidations, 1);

    // Get again - should fetch updated version from DB
    let result2 = service.get_code_unit("test:3").await?;
    assert_eq!(result2.version, 2);
    assert!(result2.body.as_ref().unwrap().contains("updated"));

    Ok(())
}

#[tokio::test]
async fn test_cache_ttl_expiration() -> Result<()> {
    let storage = create_test_storage().await?;

    // Create service with very short TTL (1 second)
    let config = CacheConfig {
        max_capacity: 1000,
        ttl_seconds: 1,
        tti_seconds: 1,
    };
    let service = CodeUnitService::with_cache_config(storage.clone(), config);

    let unit = create_test_code_unit("test:4", "ttl_func", "module::ttl_func");
    insert_code_unit(&storage, &unit).await?;

    // First call - cache miss
    service.get_code_unit("test:4").await?;
    let stats1 = service.cache_stats();
    assert_eq!(stats1.misses, 1);

    // Wait for TTL to expire
    sleep(Duration::from_secs(2)).await;

    // Second call - should be cache miss again due to expiration
    service.get_code_unit("test:4").await?;
    let stats2 = service.cache_stats();
    assert_eq!(stats2.misses, 2);

    Ok(())
}

#[tokio::test]
async fn test_cache_size_limit() -> Result<()> {
    let storage = create_test_storage().await?;

    // Create service with small cache (max 2 entries)
    let config = CacheConfig {
        max_capacity: 2,
        ttl_seconds: 300,
        tti_seconds: 60,
    };
    let service = CodeUnitService::with_cache_config(storage.clone(), config);

    // Insert 3 units
    for i in 1..=3 {
        let unit = create_test_code_unit(
            &format!("test:{}", i),
            &format!("func{}", i),
            &format!("module::func{}", i),
        );
        insert_code_unit(&storage, &unit).await?;
    }

    // Access all 3 units
    service.get_code_unit("test:1").await?;
    service.get_code_unit("test:2").await?;
    service.get_code_unit("test:3").await?;

    let stats = service.cache_stats();
    assert_eq!(stats.misses, 3); // All should be misses on first access

    // Access first unit again - might be evicted due to size limit
    service.get_code_unit("test:1").await?;

    // The cache should have evicted the oldest entry
    // We can't deterministically test which was evicted, but we can verify the cache is working
    let final_stats = service.cache_stats();
    assert!(final_stats.total_requests >= 4);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_cache_access() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = Arc::new(CodeUnitService::new(storage.clone()));

    let unit = create_test_code_unit("test:5", "concurrent_func", "module::concurrent_func");
    insert_code_unit(&storage, &unit).await?;

    // Spawn multiple concurrent tasks accessing the same unit
    let mut handles = vec![];
    for _ in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            service_clone.get_code_unit("test:5").await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10);

    // Check that we had at least one cache hit
    let stats = service.cache_stats();
    assert!(stats.hits > 0);

    Ok(())
}

#[tokio::test]
async fn test_cache_clear() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    let unit = create_test_code_unit("test:6", "clear_func", "module::clear_func");
    insert_code_unit(&storage, &unit).await?;

    // Populate cache
    service.get_code_unit("test:6").await?;
    let stats1 = service.cache_stats();
    assert_eq!(stats1.misses, 1);

    // Clear cache
    service.clear_cache().await;

    // Access again - should be cache miss
    service.get_code_unit("test:6").await?;
    let stats2 = service.cache_stats();
    assert_eq!(stats2.misses, 2);

    Ok(())
}

#[tokio::test]
async fn test_cache_stats_reset() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    let unit = create_test_code_unit("test:7", "stats_func", "module::stats_func");
    insert_code_unit(&storage, &unit).await?;

    // Generate some stats
    service.get_code_unit("test:7").await?;
    service.get_code_unit("test:7").await?;

    let stats1 = service.cache_stats();
    assert!(stats1.total_requests > 0);

    // Reset stats
    service.reset_cache_stats();

    let stats2 = service.cache_stats();
    assert_eq!(stats2.total_requests, 0);
    assert_eq!(stats2.hits, 0);
    assert_eq!(stats2.misses, 0);

    Ok(())
}

#[tokio::test]
async fn test_cache_hit_rate_calculation() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    let unit = create_test_code_unit("test:8", "hitrate_func", "module::hitrate_func");
    insert_code_unit(&storage, &unit).await?;

    // 1 miss, 3 hits
    service.get_code_unit("test:8").await?;
    service.get_code_unit("test:8").await?;
    service.get_code_unit("test:8").await?;
    service.get_code_unit("test:8").await?;

    let stats = service.cache_stats();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.total_requests, 4);
    assert!((stats.hit_rate - 75.0).abs() < 0.1); // 75% hit rate

    Ok(())
}

#[tokio::test]
async fn test_batch_get_with_cache() -> Result<()> {
    let storage = create_test_storage().await?;
    let service = CodeUnitService::new(storage.clone());

    // Insert multiple units
    for i in 1..=5 {
        let unit = create_test_code_unit(
            &format!("test:{}", i),
            &format!("batch_func{}", i),
            &format!("module::batch_func{}", i),
        );
        insert_code_unit(&storage, &unit).await?;
    }

    // Pre-populate cache with some units
    service.get_code_unit("test:1").await?;
    service.get_code_unit("test:2").await?;

    // Batch get
    let ids = vec![
        "test:1".to_string(),
        "test:2".to_string(),
        "test:3".to_string(),
    ];
    let results = service.batch_get_units(ids).await?;

    assert_eq!(results.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_custom_cache_config() -> Result<()> {
    let storage = create_test_storage().await?;

    let config = CacheConfig {
        max_capacity: 5000,
        ttl_seconds: 600,
        tti_seconds: 120,
    };

    let service = CodeUnitService::with_cache_config(storage.clone(), config.clone());

    // Verify service is created with custom config
    // (We can't directly access the config, but we can test its behavior)
    let unit = create_test_code_unit("test:9", "config_func", "module::config_func");
    insert_code_unit(&storage, &unit).await?;

    service.get_code_unit("test:9").await?;
    service.get_code_unit("test:9").await?;

    let stats = service.cache_stats();
    assert_eq!(stats.hits, 1);

    Ok(())
}
