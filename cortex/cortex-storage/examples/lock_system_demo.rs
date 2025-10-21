//! Lock System Demonstration
//!
//! This example demonstrates the entity lock system with deadlock detection.

use cortex_storage::locks::*;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== Entity Lock System Demo ===\n");

    // Create lock manager
    let manager = Arc::new(LockManager::new(
        Duration::from_secs(300),
        Duration::from_millis(100),
    ));

    println!("1. Basic Lock Acquisition");
    println!("--------------------------");

    let session1 = "session1".to_string();
    let request = LockRequest {
        entity_id: "entity1".to_string(),
        entity_type: EntityType::CodeUnit,
        lock_type: LockType::Write,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    match manager.acquire_lock(&session1, request).await {
        Ok(lock) => {
            println!("✓ Acquired write lock: {}", lock.lock_id);
            println!("  Entity: {}", lock.entity_id);
            println!("  Session: {}", lock.holder_session);
            println!("  Type: {:?}", lock.lock_type);
        }
        Err(e) => println!("✗ Failed to acquire lock: {}", e),
    }

    println!("\n2. Multiple Read Locks");
    println!("----------------------");

    let session2 = "session2".to_string();
    let session3 = "session3".to_string();

    let request2 = LockRequest {
        entity_id: "entity2".to_string(),
        entity_type: EntityType::VNode,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    let request3 = LockRequest {
        entity_id: "entity2".to_string(),
        entity_type: EntityType::VNode,
        lock_type: LockType::Read,
        timeout: Duration::from_secs(1),
        metadata: None,
    };

    manager.acquire_lock(&session2, request2).await.unwrap();
    manager.acquire_lock(&session3, request3).await.unwrap();

    println!("✓ Acquired 2 read locks on same entity (compatible)");
    let locks = manager.list_entity_locks("entity2").unwrap();
    println!("  Total locks on entity2: {}", locks.len());

    println!("\n3. Lock Conflict Detection");
    println!("--------------------------");

    let request4 = LockRequest {
        entity_id: "entity2".to_string(),
        entity_type: EntityType::VNode,
        lock_type: LockType::Write,
        timeout: Duration::from_millis(100),
        metadata: None,
    };

    match manager.acquire_lock(&"session4".to_string(), request4).await {
        Ok(_) => println!("✗ Unexpected: write lock should conflict"),
        Err(e) => println!("✓ Lock conflict detected: {}", e),
    }

    println!("\n4. Lock Statistics");
    println!("------------------");

    let stats = manager.statistics();
    println!("  Total acquired: {}", stats.total_acquired);
    println!("  Total released: {}", stats.total_released);
    println!("  Active locks: {}", stats.active_locks);

    println!("\n5. Wait-For Graph");
    println!("-----------------");

    let mut graph = WaitForGraph::new();
    graph.add_wait_edge("A".to_string(), "B".to_string());
    graph.add_wait_edge("B".to_string(), "C".to_string());

    println!("  Created wait chain: A -> B -> C");
    println!("  Has path A to C: {}", graph.has_path(&"A".to_string(), &"C".to_string()));
    println!("  Has cycle: {}", graph.detect_cycle().is_some());

    // Create cycle
    graph.add_wait_edge("C".to_string(), "A".to_string());
    println!("\n  Added edge: C -> A (creates cycle)");

    if let Some(cycle) = graph.detect_cycle() {
        println!("  ✓ Cycle detected: {:?}", cycle);
    }

    println!("\n6. Deadlock Detector");
    println!("--------------------");

    let detector = DeadlockDetector::new(Duration::from_millis(100));

    detector.add_wait("session1".to_string(), "session2".to_string());
    detector.add_wait("session2".to_string(), "session3".to_string());
    detector.add_wait("session3".to_string(), "session1".to_string());

    if let Some(deadlock) = detector.check_deadlock() {
        println!("  ✓ Deadlock detected!");
        println!("  Cycle: {:?}", deadlock.cycle);

        let victim = detector.select_victim(&deadlock);
        println!("  Selected victim: {}", victim);
    }

    println!("\n7. Session Cleanup");
    println!("------------------");

    let released = manager.release_session_locks(&session2).unwrap();
    println!("  Released {} locks for session2", released);

    let stats = manager.statistics();
    println!("  Active locks remaining: {}", stats.active_locks);

    println!("\n8. Lock Compatibility Matrix");
    println!("-----------------------------");
    println!("  Read + Read:   {}", is_compatible(&LockType::Read, &LockType::Read));
    println!("  Read + Write:  {}", is_compatible(&LockType::Read, &LockType::Write));
    println!("  Write + Write: {}", is_compatible(&LockType::Write, &LockType::Write));
    println!("  Read + Intent: {}", is_compatible(&LockType::Read, &LockType::Intent));
    println!("  Intent + Intent: {}", is_compatible(&LockType::Intent, &LockType::Intent));
    println!("  Write + Intent: {}", is_compatible(&LockType::Write, &LockType::Intent));

    println!("\n✓ Demo completed successfully!");
}
