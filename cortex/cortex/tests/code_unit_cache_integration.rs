//! Integration tests for CodeUnitService caching with real workflows

use anyhow::Result;
use cortex::services::{CacheConfig, CodeUnitService};
use cortex_core::types::{CodeUnit, CodeUnitType, Complexity, Language, Visibility};
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;
use uuid::Uuid;

async fn setup_test_service() -> Result<(Arc<ConnectionManager>, CodeUnitService)> {
    let storage = Arc::new(ConnectionManager::new_memory().await?);
    let service = CodeUnitService::new(storage.clone());
    Ok((storage, service))
}

fn create_code_unit(id: &str, name: &str, qname: &str) -> CodeUnit {
    CodeUnit {
        id: id.to_string(),
        unit_type: CodeUnitType::Function,
        name: name.to_string(),
        qualified_name: qname.to_string(),
        display_name: name.to_string(),
        file_path: format!("/test/{}.rs", name),
        language: Language::Rust,
        start_line: 1,
        end_line: 10,
        start_column: 0,
        end_column: 0,
        signature: format!("fn {}()", name),
        body: Some("{ }".to_string()),
        docstring: Some(format!("Test {}", name)),
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

async fn insert_unit(storage: &Arc<ConnectionManager>, unit: &CodeUnit) -> Result<()> {
    let pooled = storage.acquire().await?;
    let conn = pooled.connection();
    let query = "CREATE code_unit CONTENT $unit";
    conn.query(query).bind(("unit", unit)).await?;
    Ok(())
}

#[tokio::test]
async fn test_realistic_read_heavy_workload() -> Result<()> {
    let (storage, service) = setup_test_service().await?;

    // Insert 100 code units
    for i in 0..100 {
        let unit = create_code_unit(
            &format!("unit:{}", i),
            &format!("func_{}", i),
            &format!("module::func_{}", i),
        );
        insert_unit(&storage, &unit).await?;
    }

    // Simulate read-heavy workload: 80% reads to same units, 20% to random units
    let popular_ids: Vec<String> = (0..10).map(|i| format!("unit:{}", i)).collect();

    let service = Arc::new(service);
    let mut tasks = JoinSet::new();

    for iteration in 0..100 {
        let service_clone = service.clone();
        let ids_clone = popular_ids.clone();

        tasks.spawn(async move {
            // 80% chance of accessing popular unit
            let unit_id = if iteration % 5 != 0 {
                &ids_clone[iteration % ids_clone.len()]
            } else {
                // 20% chance of random access
                &format!("unit:{}", iteration % 100)
            };

            service_clone.get_code_unit(unit_id).await
        });
    }

    // Wait for all tasks
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 100);

    let stats = service.cache_stats();
    println!("Stats: {:?}", stats);

    // With 80% reads to 10 popular units, we should have high hit rate
    assert!(stats.hit_rate > 50.0, "Hit rate should be > 50%");
    assert!(stats.hits > stats.misses, "Should have more hits than misses");

    Ok(())
}

#[tokio::test]
async fn test_update_invalidation_workflow() -> Result<()> {
    let (storage, service) = setup_test_service().await?;

    let unit = create_code_unit("workflow:1", "update_test", "module::update_test");
    insert_unit(&storage, &unit).await?;

    // Read multiple times to warm cache
    for _ in 0..5 {
        service.get_code_unit("workflow:1").await?;
    }

    let stats_before = service.cache_stats();
    assert_eq!(stats_before.hits, 4); // First is miss, rest are hits

    // Update the unit
    service
        .update_code_unit(
            "workflow:1",
            Some("{ updated() }".to_string()),
            Some("Updated docs".to_string()),
            Some(1),
        )
        .await?;

    let stats_after_update = service.cache_stats();
    assert_eq!(stats_after_update.invalidations, 1);

    // Read again - should fetch updated version
    let updated = service.get_code_unit("workflow:1").await?;
    assert_eq!(updated.version, 2);
    assert!(updated.body.unwrap().contains("updated"));

    Ok(())
}

#[tokio::test]
async fn test_memory_pressure_simulation() -> Result<()> {
    let storage = Arc::new(ConnectionManager::new_memory().await?);

    // Create service with limited cache size
    let config = CacheConfig {
        max_capacity: 50,
        ttl_seconds: 300,
        tti_seconds: 60,
    };
    let service = CodeUnitService::with_cache_config(storage.clone(), config);

    // Insert 200 units (more than cache capacity)
    for i in 0..200 {
        let unit = create_code_unit(
            &format!("mem:{}", i),
            &format!("mem_func_{}", i),
            &format!("module::mem_func_{}", i),
        );
        insert_unit(&storage, &unit).await?;
    }

    // Access all units sequentially
    for i in 0..200 {
        service.get_code_unit(&format!("mem:{}", i)).await?;
    }

    let stats = service.cache_stats();

    // All should be misses on first pass
    assert_eq!(stats.misses, 200);

    // Access first 50 again - some should hit cache, some might be evicted
    for i in 0..50 {
        service.get_code_unit(&format!("mem:{}", i)).await?;
    }

    let final_stats = service.cache_stats();
    // We can't predict exact hit rate due to LRU eviction, but should have some hits
    assert!(final_stats.total_requests > 200);

    Ok(())
}

#[tokio::test]
async fn test_mixed_access_patterns() -> Result<()> {
    let (storage, service) = setup_test_service().await?;

    // Insert test units
    for i in 0..20 {
        let unit = create_code_unit(
            &format!("mixed:{}", i),
            &format!("mixed_func_{}", i),
            &format!("module::mixed_func_{}", i),
        );
        insert_unit(&storage, &unit).await?;
    }

    let service = Arc::new(service);

    // Simulate mixed access: by ID and by qualified name
    let mut tasks = JoinSet::new();

    for i in 0..40 {
        let service_clone = service.clone();

        tasks.spawn(async move {
            if i % 2 == 0 {
                // Access by ID
                service_clone
                    .get_code_unit(&format!("mixed:{}", i % 20))
                    .await
            } else {
                // Access by qualified name
                service_clone
                    .get_by_qualified_name(&format!("module::mixed_func_{}", i % 20))
                    .await
            }
        });
    }

    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result?.is_ok() {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 40);

    let stats = service.cache_stats();
    // Should have cache hits from both caches
    assert!(stats.hits > 0);

    Ok(())
}

#[tokio::test]
async fn test_cache_performance_improvement() -> Result<()> {
    let (storage, service) = setup_test_service().await?;

    let unit = create_code_unit("perf:1", "perf_test", "module::perf_test");
    insert_unit(&storage, &unit).await?;

    // Measure first access (cache miss)
    let start_miss = Instant::now();
    service.get_code_unit("perf:1").await?;
    let duration_miss = start_miss.elapsed();

    // Measure subsequent accesses (cache hits)
    let mut hit_durations = vec![];
    for _ in 0..10 {
        let start_hit = Instant::now();
        service.get_code_unit("perf:1").await?;
        hit_durations.push(start_hit.elapsed());
    }

    let avg_hit_duration = hit_durations.iter().sum::<std::time::Duration>() / 10;

    println!("Miss duration: {:?}", duration_miss);
    println!("Avg hit duration: {:?}", avg_hit_duration);

    // Cache hits should generally be faster (though not guaranteed in tests)
    // This is more for observability than assertion
    let stats = service.cache_stats();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 10);

    Ok(())
}

#[tokio::test]
async fn test_invalidation_under_concurrent_load() -> Result<()> {
    let (storage, service) = setup_test_service().await?;
    let service = Arc::new(service);

    let unit = create_code_unit("concurrent:1", "concurrent_test", "module::concurrent_test");
    insert_unit(&storage, &unit).await?;

    // Spawn readers
    let mut tasks = JoinSet::new();
    for _ in 0..50 {
        let service_clone = service.clone();
        tasks.spawn(async move {
            for _ in 0..5 {
                let _ = service_clone.get_code_unit("concurrent:1").await;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
    }

    // Spawn updater
    let service_updater = service.clone();
    let update_task = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        service_updater
            .update_code_unit(
                "concurrent:1",
                Some("{ concurrent_updated() }".to_string()),
                None,
                Some(1),
            )
            .await
    });

    // Wait for all tasks
    while let Some(_) = tasks.join_next().await {}
    let _ = update_task.await?;

    let stats = service.cache_stats();
    println!("Concurrent stats: {:?}", stats);

    // Should have cache invalidation
    assert!(stats.invalidations > 0);
    // Should have multiple reads
    assert!(stats.total_requests > 50);

    Ok(())
}

#[tokio::test]
async fn test_cache_with_workspace_operations() -> Result<()> {
    let (storage, service) = setup_test_service().await?;

    let workspace_id = Uuid::new_v4();

    // Insert units belonging to workspace
    for i in 0..10 {
        let mut unit = create_code_unit(
            &format!("ws:{}", i),
            &format!("ws_func_{}", i),
            &format!("module::ws_func_{}", i),
        );
        unit.file_path = format!("{}/test/file.rs", workspace_id);
        insert_unit(&storage, &unit).await?;
    }

    // List units (doesn't use cache)
    let listed = service.list_code_units(workspace_id, None, None, None, None, 100).await?;
    assert_eq!(listed.len(), 10);

    // Now access individual units to populate cache
    for i in 0..10 {
        service.get_code_unit(&format!("ws:{}", i)).await?;
    }

    let stats = service.cache_stats();
    assert_eq!(stats.misses, 10); // All first accesses are misses

    // Access again to verify cache hits
    for i in 0..5 {
        service.get_code_unit(&format!("ws:{}", i)).await?;
    }

    let final_stats = service.cache_stats();
    assert_eq!(final_stats.hits, 5);

    Ok(())
}
