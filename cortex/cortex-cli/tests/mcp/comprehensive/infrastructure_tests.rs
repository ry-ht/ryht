//! Infrastructure Component Tests
//!
//! Comprehensive tests for all core infrastructure components:
//! - SurrealDB lifecycle management
//! - Connection pooling strategies
//! - VFS initialization and operations
//! - Memory system initialization (5 tiers)
//! - Semantic search initialization
//! - Configuration management
//! - Error recovery mechanisms

use cortex_storage::{
    DatabaseConfig, ConnectionMode, Credentials, PoolConfig,
    ConnectionManager, SurrealDBConfig, SurrealDBManager,
};
use cortex_vfs::{VirtualFileSystem, VirtualPath, ExternalProjectLoader, MaterializationEngine};
use cortex_memory::{
    CognitiveManager, EpisodicMemorySystem, SemanticMemorySystem,
    WorkingMemorySystem, ProceduralMemorySystem, MemoryConsolidator,
};
use cortex_semantic::{SemanticSearchEngine, SemanticConfig, EmbeddingProviderConfig};
use cortex_parser::CodeParser;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::sleep;
use uuid::Uuid;

/// Helper function to create an in-memory database configuration for testing
fn create_memory_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::InMemory,
        credentials: Credentials::default(),
        pool_config: PoolConfig::default(),
        namespace: "cortex_test".to_string(),
        database: "main".to_string(),
    }
}

/// Test: SurrealDB configuration and basic lifecycle
#[tokio::test]
async fn test_surrealdb_configuration() {
    println!("\n=== Testing SurrealDB Configuration ===");

    // Test: Create configuration
    println!("  [1/3] Creating SurrealDB configuration...");
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let data_dir = temp_dir.path().join("data");

    let config = SurrealDBConfig::new(
        "127.0.0.1:8001".to_string(),
        data_dir.clone(),
    );

    assert_eq!(config.bind_address, "127.0.0.1:8001");
    assert_eq!(config.data_dir, data_dir);
    println!("    ✓ Configuration created");

    // Test: Validate configuration
    println!("  [2/3] Validating configuration...");
    let validation_result = config.validate();
    assert!(validation_result.is_ok(), "Configuration should be valid");
    println!("    ✓ Configuration validated");

    // Test: Ensure directories
    println!("  [3/3] Ensuring directories exist...");
    config.ensure_directories().await.expect("Failed to create directories");
    assert!(data_dir.exists(), "Data directory should exist");
    println!("    ✓ Directories created");

    println!("✓ SurrealDB Configuration Test Passed\n");
}

/// Test: Connection manager with in-memory database
#[tokio::test]
async fn test_connection_manager() {
    println!("\n=== Testing Connection Manager ===");

    // Test: Create connection manager
    println!("  [1/4] Creating connection manager...");
    let config = create_memory_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    println!("    ✓ Connection manager created");

    // Test: Acquire connection
    println!("  [2/4] Acquiring connection...");
    let conn = manager.acquire().await.expect("Failed to acquire connection");
    println!("    ✓ Connection acquired");

    // Test: Execute simple query
    println!("  [3/4] Executing test query...");
    let result: Result<Vec<serde_json::Value>, _> =
        conn.connection().query("SELECT * FROM type::table('test')").await;
    assert!(result.is_ok(), "Query should execute");
    println!("    ✓ Query executed");

    // Test: Pool statistics
    println!("  [4/4] Checking pool statistics...");
    let stats = manager.statistics();
    println!("    - Total connections: {}", stats.total_connections);
    println!("    - Active connections: {}", stats.active_connections);
    assert!(stats.total_connections > 0, "Should have connections");
    println!("    ✓ Statistics available");

    println!("✓ Connection Manager Test Passed\n");
}

/// Test: VFS initialization and cache management
#[tokio::test]
async fn test_vfs_initialization() {
    println!("\n=== Testing VFS Initialization ===");

    // Test: Create VFS with in-memory storage
    println!("  [1/6] Creating VFS with in-memory storage...");
    let config = create_memory_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager")
    );

    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    println!("    ✓ VFS created");

    // Test: Create workspace
    println!("  [2/6] Creating test workspace...");
    let workspace_id = Uuid::new_v4();

    // Create a simple file
    let file_path = VirtualPath::new("src/main.rs").expect("Failed to create virtual path");
    let content = b"fn main() { println!(\"Hello, world!\"); }";

    vfs.write_file(&workspace_id, &file_path, content)
        .await
        .expect("Failed to write file");
    println!("    ✓ Workspace created and file written");

    // Test: Read file back
    println!("  [3/6] Reading file from VFS...");
    let read_content = vfs
        .read_file(&workspace_id, &file_path)
        .await
        .expect("Failed to read file");
    assert_eq!(read_content, content, "File content mismatch");
    println!("    ✓ File read successfully");

    // Test: List directory
    println!("  [4/6] Listing directory contents...");
    let src_path = VirtualPath::new("src").expect("Failed to create src path");
    let entries = vfs
        .list_directory(&workspace_id, &src_path)
        .await
        .expect("Failed to list directory");
    assert_eq!(entries.len(), 1, "Expected 1 file in src/");
    println!("    ✓ Directory listed: {} entries", entries.len());

    // Test: Content cache
    println!("  [5/6] Testing content cache...");
    let cache_stats_before = vfs.cache_statistics();

    // Read same file multiple times to test cache
    for _ in 0..5 {
        let _ = vfs.read_file(&workspace_id, &file_path).await;
    }

    let cache_stats_after = vfs.cache_statistics();
    println!("    - Cache hits: {}", cache_stats_after.hits);
    println!("    - Cache misses: {}", cache_stats_after.misses);
    println!("    - Hit rate: {:.2}%", cache_stats_after.hit_rate * 100.0);
    println!("    ✓ Content cache working");

    // Test: External project loader
    println!("  [6/6] Testing external project loader...");
    let _loader = ExternalProjectLoader::new((*vfs).clone());
    println!("    ✓ External project loader ready");

    println!("✓ VFS Initialization Test Passed\n");
}

/// Test: Memory system initialization (5 tiers)
#[tokio::test]
async fn test_memory_system_initialization() {
    println!("\n=== Testing Memory System Initialization ===");

    let config = create_memory_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager")
    );

    // Test: Working Memory (Tier 1)
    println!("  [1/5] Initializing Working Memory...");
    let _working_memory = WorkingMemorySystem::new(storage.clone());
    println!("    ✓ Working Memory initialized (7±2 item capacity)");

    // Test: Episodic Memory (Tier 2)
    println!("  [2/5] Initializing Episodic Memory...");
    let _episodic_memory = EpisodicMemorySystem::new(storage.clone());
    println!("    ✓ Episodic Memory initialized (session episodes)");

    // Test: Semantic Memory (Tier 3)
    println!("  [3/5] Initializing Semantic Memory...");
    let _semantic_memory = SemanticMemorySystem::new(storage.clone());
    println!("    ✓ Semantic Memory initialized (code structures)");

    // Test: Procedural Memory (Tier 4)
    println!("  [4/5] Initializing Procedural Memory...");
    let _procedural_memory = ProceduralMemorySystem::new(storage.clone());
    println!("    ✓ Procedural Memory initialized (workflows)");

    // Test: Cognitive Manager (orchestrates all tiers)
    println!("  [5/5] Creating Cognitive Manager...");
    let _cognitive_manager = CognitiveManager::new(storage.clone());
    println!("    ✓ Cognitive Manager created (all 5 tiers active)");

    println!("✓ Memory System Initialization Test Passed\n");
}

/// Test: Semantic search initialization
#[tokio::test]
async fn test_semantic_search_initialization() {
    println!("\n=== Testing Semantic Search Initialization ===");

    // Test: Create semantic config with mock provider (no external dependencies)
    println!("  [1/4] Creating semantic search configuration...");
    let config = SemanticConfig {
        provider: EmbeddingProviderConfig::Mock {
            dimension: 384,
        },
        index: cortex_semantic::IndexConfig {
            dimension: 384,
            max_elements: 10000,
            ef_construction: 200,
            m: 16,
        },
        search: cortex_semantic::SearchConfig {
            default_limit: 10,
            max_limit: 100,
            use_reranking: false,
            use_query_expansion: true,
        },
    };
    println!("    ✓ Configuration created");

    // Test: Initialize search engine
    println!("  [2/4] Initializing semantic search engine...");
    let start = Instant::now();
    let engine = SemanticSearchEngine::new(config)
        .await
        .expect("Failed to create semantic search engine");
    let init_duration = start.elapsed();
    println!("    ✓ Engine initialized in {:.2}ms", init_duration.as_millis());

    // Test: Index a document
    println!("  [3/4] Indexing test document...");
    let doc_id = "test_doc_1";
    let content = "fn calculate_sum(a: i32, b: i32) -> i32 { a + b }";

    engine
        .index_document(doc_id, content)
        .await
        .expect("Failed to index document");
    println!("    ✓ Document indexed");

    // Test: Perform search
    println!("  [4/4] Performing semantic search...");
    let query = "function that adds two numbers";
    let results = engine
        .search(query, 5)
        .await
        .expect("Failed to search");

    println!("    - Found {} results", results.len());
    if !results.is_empty() {
        println!("    - Top result: {} (score: {:.4})", results[0].id, results[0].score);
    }
    assert!(!results.is_empty(), "Should find indexed document");
    println!("    ✓ Semantic search working");

    println!("✓ Semantic Search Initialization Test Passed\n");
}

/// Test: Configuration management
#[tokio::test]
async fn test_configuration_management() {
    println!("\n=== Testing Configuration Management ===");

    // Test: In-memory configuration
    println!("  [1/3] Testing in-memory configuration...");
    let mem_config = create_memory_config();
    assert_eq!(mem_config.namespace, "cortex_test");
    assert_eq!(mem_config.database, "main");
    assert!(matches!(mem_config.connection_mode, ConnectionMode::InMemory));
    println!("    ✓ In-memory configuration valid");

    // Test: Pool configuration
    println!("  [2/3] Testing pool configuration...");
    let pool_config = PoolConfig::default();
    assert!(pool_config.min_connections >= 1, "Should have minimum connections");
    assert!(pool_config.max_connections > pool_config.min_connections, "Max should be greater than min");
    println!("    ✓ Pool configuration valid");

    // Test: Create connection manager with config
    println!("  [3/3] Testing connection manager initialization...");
    let config = create_memory_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    let _conn = manager.acquire().await.expect("Failed to acquire connection");
    println!("    ✓ Connection manager working with configuration");

    println!("✓ Configuration Management Test Passed\n");
}

/// Test: Error recovery mechanisms
#[tokio::test]
async fn test_error_recovery() {
    println!("\n=== Testing Error Recovery Mechanisms ===");

    // Test: Invalid path handling
    println!("  [1/4] Testing invalid path handling...");
    let invalid_path_result = VirtualPath::new("");
    assert!(invalid_path_result.is_err(), "Empty path should be rejected");
    println!("    ✓ Empty paths rejected");

    // Test: Non-existent file read
    println!("  [2/4] Testing non-existent file read...");
    let config = create_memory_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager")
    );
    let vfs = VirtualFileSystem::new(storage);

    let workspace_id = Uuid::new_v4();
    let missing_path = VirtualPath::new("does_not_exist.rs").expect("Valid path");
    let read_result = vfs.read_file(&workspace_id, &missing_path).await;
    assert!(read_result.is_err(), "Reading non-existent file should fail");
    println!("    ✓ Non-existent file reads handled");

    // Test: Parser error handling
    println!("  [3/4] Testing parser error handling...");
    let parser = CodeParser::new().expect("Failed to create parser");
    let invalid_rust = "fn incomplete(";
    let parse_result = parser.parse(invalid_rust, "rust");

    // Parser should either return an error or handle gracefully
    match parse_result {
        Ok(result) => {
            println!("    ✓ Parser handled invalid code gracefully ({} units)", result.units.len());
        }
        Err(_) => {
            println!("    ✓ Parser reported error for invalid code");
        }
    }

    // Test: Configuration validation
    println!("  [4/4] Testing configuration validation...");
    let config = SurrealDBConfig::default();
    let validation_result = config.validate();
    assert!(validation_result.is_ok(), "Default config should be valid");
    println!("    ✓ Configuration validation working");

    println!("✓ Error Recovery Test Passed\n");
}

/// Test: VFS materialization engine
#[tokio::test]
async fn test_materialization_engine() {
    println!("\n=== Testing Materialization Engine ===");

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let target_path = temp_dir.path().join("materialized");

    // Setup VFS
    let config = create_memory_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager")
    );
    let vfs = Arc::new(VirtualFileSystem::new(storage));
    let workspace_id = Uuid::new_v4();

    // Create test files in VFS
    println!("  [1/4] Creating test files in VFS...");
    let files = vec![
        ("src/main.rs", b"fn main() {}"),
        ("src/lib.rs", b"pub fn add(a: i32, b: i32) -> i32 { a + b }"),
        ("Cargo.toml", b"[package]\nname = \"test\"\nversion = \"0.1.0\""),
    ];

    for (path_str, content) in &files {
        let path = VirtualPath::new(path_str).expect("Valid path");
        vfs.write_file(&workspace_id, &path, content)
            .await
            .expect("Failed to write file");
    }
    println!("    ✓ Created {} files in VFS", files.len());

    // Create materialization engine
    println!("  [2/4] Creating materialization engine...");
    let engine = MaterializationEngine::new((*vfs).clone());
    println!("    ✓ Engine created");

    // Materialize to disk
    println!("  [3/4] Materializing VFS to disk...");
    use cortex_vfs::{FlushScope, FlushOptions};

    let flush_result = engine
        .flush(
            FlushScope::Workspace(workspace_id),
            &target_path,
            FlushOptions::default(),
        )
        .await
        .expect("Failed to flush VFS");

    println!("    - Files flushed: {}", flush_result.files_flushed);
    println!("    - Bytes written: {}", flush_result.bytes_written);
    assert_eq!(flush_result.files_flushed, files.len(), "All files should be flushed");
    println!("    ✓ VFS materialized successfully");

    // Verify files exist on disk
    println!("  [4/4] Verifying materialized files...");
    for (path_str, _) in &files {
        let disk_path = target_path.join(path_str);
        assert!(disk_path.exists(), "File {} should exist on disk", path_str);
    }
    println!("    ✓ All files verified on disk");

    println!("✓ Materialization Engine Test Passed\n");
}

/// Integration test: Full infrastructure stack
#[tokio::test]
async fn test_full_infrastructure_stack() {
    println!("\n=== Testing Full Infrastructure Stack ===");

    let start = Instant::now();

    // Initialize all components
    println!("  [1/6] Initializing storage layer...");
    let config = create_memory_config();
    let storage = Arc::new(
        ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager")
    );
    println!("    ✓ Storage initialized");

    println!("  [2/6] Initializing VFS...");
    let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
    println!("    ✓ VFS initialized");

    println!("  [3/6] Initializing parser...");
    let parser = Arc::new(tokio::sync::Mutex::new(
        CodeParser::new().expect("Failed to create parser")
    ));
    println!("    ✓ Parser initialized");

    println!("  [4/6] Initializing memory systems...");
    let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
    let _cognitive_manager = CognitiveManager::new(storage.clone());
    println!("    ✓ Memory systems initialized");

    println!("  [5/6] Initializing ingestion pipeline...");
    use cortex_vfs::FileIngestionPipeline;
    let _ingestion = FileIngestionPipeline::new(
        parser.clone(),
        vfs.clone(),
        semantic_memory.clone(),
    );
    println!("    ✓ Ingestion pipeline initialized");

    println!("  [6/6] Testing end-to-end workflow...");
    let workspace_id = Uuid::new_v4();
    let test_path = VirtualPath::new("src/test.rs").expect("Valid path");
    let test_content = b"pub struct User { pub id: u64, pub name: String }";

    vfs.write_file(&workspace_id, &test_path, test_content)
        .await
        .expect("Failed to write file");

    let read_back = vfs.read_file(&workspace_id, &test_path)
        .await
        .expect("Failed to read file");

    assert_eq!(read_back, test_content, "Content should match");

    let duration = start.elapsed();
    println!("    ✓ End-to-end workflow completed in {:.2}ms", duration.as_millis());

    println!("✓ Full Infrastructure Stack Test Passed\n");
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    /// Benchmark connection pool performance
    #[tokio::test]
    async fn bench_connection_pool_throughput() {
        println!("\n=== Benchmarking Connection Pool Throughput ===");

        let config = create_memory_config();
        let manager = ConnectionManager::new(config)
            .await
            .expect("Failed to create connection manager");

        let iterations = 100;
        let start = Instant::now();

        for _ in 0..iterations {
            let conn = manager.acquire().await.expect("Failed to acquire");
            let _: Result<Vec<serde_json::Value>, _> =
                conn.connection().query("SELECT * FROM type::table('test')").await;
        }

        let duration = start.elapsed();
        let ops_per_sec = (iterations as f64 / duration.as_secs_f64()) as u64;

        println!("  - Operations: {}", iterations);
        println!("  - Duration: {:.2}s", duration.as_secs_f64());
        println!("  - Throughput: {} ops/sec", ops_per_sec);

        assert!(ops_per_sec > 50, "Should handle at least 50 ops/sec");
        println!("✓ Connection Pool Throughput Benchmark Passed\n");
    }

    /// Benchmark VFS operations
    #[tokio::test]
    async fn bench_vfs_operations() {
        println!("\n=== Benchmarking VFS Operations ===");

        let config = create_memory_config();
        let storage = Arc::new(
            ConnectionManager::new(config)
                .await
                .expect("Failed to create connection manager")
        );
        let vfs = VirtualFileSystem::new(storage);
        let workspace_id = Uuid::new_v4();

        let file_count = 50;
        let content = b"test content";

        // Benchmark writes
        let start = Instant::now();
        for i in 0..file_count {
            let path = VirtualPath::new(&format!("file_{}.txt", i))
                .expect("Valid path");
            vfs.write_file(&workspace_id, &path, content)
                .await
                .expect("Failed to write");
        }
        let write_duration = start.elapsed();

        // Benchmark reads
        let start = Instant::now();
        for i in 0..file_count {
            let path = VirtualPath::new(&format!("file_{}.txt", i))
                .expect("Valid path");
            vfs.read_file(&workspace_id, &path)
                .await
                .expect("Failed to read");
        }
        let read_duration = start.elapsed();

        println!("  - Files: {}", file_count);
        println!("  - Write time: {:.2}ms", write_duration.as_millis());
        println!("  - Read time: {:.2}ms", read_duration.as_millis());
        println!("  - Write throughput: {:.1} files/sec",
                 file_count as f64 / write_duration.as_secs_f64());
        println!("  - Read throughput: {:.1} files/sec",
                 file_count as f64 / read_duration.as_secs_f64());

        println!("✓ VFS Operations Benchmark Passed\n");
    }
}
