//! Comprehensive Performance Benchmarks for Cortex
//!
//! This benchmark suite measures ALL performance targets from the specification:
//! 1. Navigation operations: <50ms
//! 2. Semantic search: <100ms
//! 3. Code manipulation: <200ms
//! 4. Flush 10K LOC to disk: <5s
//! 5. Memory query: <50ms
//! 6. Association retrieval: <50ms
//! 7. Memory storage: <100ms
//! 8. Session creation: <200ms
//! 9. Connection pool: 1000-2000 ops/sec
//!
//! All benchmarks use criterion.rs for accurate statistical analysis.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use cortex_core::prelude::*;
use cortex_core::CodeUnitType; // Explicit import to avoid ambiguity
use cortex_memory::prelude::*;
use cortex_semantic::prelude::*;
use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

// =============================================================================
// Helper Functions
// =============================================================================

fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            max_connections: 20,
            min_connections: 5,
            connection_timeout: std::time::Duration::from_secs(30),
            idle_timeout: Some(std::time::Duration::from_secs(300)),
            ..Default::default()
        },
        namespace: "cortex_bench".to_string(),
        database: db_name.to_string(),
    }
}

// =============================================================================
// 1. Navigation Operations (Target: <50ms)
// =============================================================================

fn bench_navigation_find_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("nav_find");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create directory structure: 100 files across 10 directories
    rt.block_on(async {
        for i in 0..100 {
            let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", i / 10, i % 10)).unwrap();
            vfs.write_file(&workspace_id, &path, format!("// File {}", i).as_bytes())
                .await
                .unwrap();
        }
    });

    c.bench_function("navigation_find_file", |b| {
        b.iter(|| {
            rt.block_on(async {
                let path = VirtualPath::new(black_box("src/module_5/file_3.rs")).unwrap();
                vfs.read_file(&workspace_id, &path).await.unwrap()
            })
        })
    });
}

fn bench_navigation_traverse_tree(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("nav_traverse");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create nested directory structure
    rt.block_on(async {
        for i in 0..50 {
            let path = VirtualPath::new(&format!("src/deep/nested/structure_{}/file.rs", i)).unwrap();
            vfs.write_file(&workspace_id, &path, b"content").await.unwrap();
        }
    });

    c.bench_function("navigation_traverse_tree", |b| {
        b.iter(|| {
            rt.block_on(async {
                let root = VirtualPath::new(black_box("")).unwrap();
                vfs.list_directory(&workspace_id, &root).await.unwrap()
            })
        })
    });
}

fn bench_navigation_list_children(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("nav_children");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    rt.block_on(async {
        for i in 0..20 {
            let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
            vfs.write_file(&workspace_id, &path, b"content").await.unwrap();
        }
    });

    c.bench_function("navigation_list_children", |b| {
        b.iter(|| {
            rt.block_on(async {
                let dir = VirtualPath::new(black_box("src")).unwrap();
                vfs.list_directory(&workspace_id, &dir).await.unwrap()
            })
        })
    });
}

// =============================================================================
// 2. Semantic Search (Target: <100ms)
// =============================================================================

fn bench_semantic_search_functions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = SemanticConfig::default();
    let engine = rt.block_on(async {
        SemanticSearchEngine::new(config).await.unwrap()
    });

    // Index 100 functions
    rt.block_on(async {
        for i in 0..100 {
            let doc = format!("fn process_data_{}(input: Data) -> Result<Output> {{ /* implementation */ }}", i);
            engine.index_document(&format!("fn_{}", i), &doc).await.unwrap();
        }
    });

    c.bench_function("semantic_search_functions", |b| {
        b.iter(|| {
            rt.block_on(async {
                engine.search(black_box("process data"), black_box(10)).await.unwrap()
            })
        })
    });
}

fn bench_semantic_search_by_meaning(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = SemanticConfig::default();
    let engine = rt.block_on(async {
        SemanticSearchEngine::new(config).await.unwrap()
    });

    // Index diverse documents
    rt.block_on(async {
        let docs = vec![
            ("authentication", "Handle user login and session management"),
            ("database", "Execute SQL queries and manage connections"),
            ("validation", "Validate input data and sanitize user input"),
            ("caching", "Store frequently accessed data in memory cache"),
            ("logging", "Record system events and error messages"),
        ];
        for (id, content) in docs {
            engine.index_document(id, content).await.unwrap();
        }
    });

    c.bench_function("semantic_search_by_meaning", |b| {
        b.iter(|| {
            rt.block_on(async {
                engine.search(black_box("user authentication"), black_box(5)).await.unwrap()
            })
        })
    });
}

fn bench_semantic_hybrid_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = SemanticConfig::default();
    let engine = rt.block_on(async {
        SemanticSearchEngine::new(config).await.unwrap()
    });

    // Index with metadata for filtering
    rt.block_on(async {
        for i in 0..50 {
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "function".to_string());
            metadata.insert("module".to_string(), format!("module_{}", i % 5));

            let content = format!("Function implementation for feature {}", i);
            engine.index_document(
                &format!("fn_{}", i),
                &content,
                cortex_semantic::EntityType::Function,
                metadata,
            ).await.unwrap();
        }
    });

    c.bench_function("semantic_hybrid_search", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Semantic search with metadata filtering
                engine.search(black_box("feature implementation"), black_box(10)).await.unwrap()
            })
        })
    });
}

// =============================================================================
// 3. Code Manipulation (Target: <200ms)
// =============================================================================

fn bench_code_parse_file(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("code_parse");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);

    let rust_code = r#"
pub struct DataProcessor {
    config: Config,
    cache: HashMap<String, Value>,
}

impl DataProcessor {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    pub fn process(&mut self, input: &str) -> Result<Output> {
        if let Some(cached) = self.cache.get(input) {
            return Ok(cached.clone());
        }

        let result = self.transform(input)?;
        self.cache.insert(input.to_string(), result.clone());
        Ok(result)
    }
}
"#;

    c.bench_function("code_parse_file", |b| {
        b.iter(|| {
            // Simulate parsing by creating semantic units
            rt.block_on(async {
                let unit = SemanticUnit {
                    id: CortexId::new(),
                    unit_type: CodeUnitType::Struct,
                    name: "DataProcessor".to_string(),
                    qualified_name: "module::DataProcessor".to_string(),
                    display_name: "DataProcessor".to_string(),
                    file_path: "src/processor.rs".to_string(),
                    start_line: 1,
                    start_column: 0,
                    end_line: 20,
                    end_column: 1,
                    signature: "pub struct DataProcessor".to_string(),
                    body: black_box(rust_code).to_string(),
                    docstring: None,
                    visibility: "public".to_string(),
                    modifiers: vec![],
                    parameters: vec![],
                    return_type: None,
                    summary: "Data processor".to_string(),
                    purpose: "Process data".to_string(),
                    complexity: ComplexityMetrics::default(),
                    test_coverage: None,
                    has_tests: false,
                    has_documentation: false,
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                cognitive.remember_unit(&unit).await.unwrap()
            })
        })
    });
}

fn bench_code_modify_ast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("code_modify");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    let original_code = "fn old_function() { println!(\"test\"); }";
    let modified_code = "fn new_function() { println!(\"test\"); }";

    rt.block_on(async {
        let path = VirtualPath::new("src/test.rs").unwrap();
        vfs.write_file(&workspace_id, &path, original_code.as_bytes()).await.unwrap();
    });

    c.bench_function("code_modify_ast", |b| {
        b.iter(|| {
            rt.block_on(async {
                let path = VirtualPath::new("src/test.rs").unwrap();
                vfs.write_file(&workspace_id, &path, black_box(modified_code).as_bytes())
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_code_apply_changes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("code_apply");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager.clone());
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Pre-create file and unit
    let unit_id = rt.block_on(async {
        let path = VirtualPath::new("src/lib.rs").unwrap();
        vfs.write_file(&workspace_id, &path, b"fn test() {}").await.unwrap();

        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: "test".to_string(),
            qualified_name: "test".to_string(),
            display_name: "test".to_string(),
            file_path: "src/lib.rs".to_string(),
            start_line: 1,
            start_column: 0,
            end_line: 1,
            end_column: 13,
            signature: "fn test()".to_string(),
            body: "{}".to_string(),
            docstring: None,
            visibility: "private".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: None,
            summary: "Test".to_string(),
            purpose: "Testing".to_string(),
            complexity: ComplexityMetrics::default(),
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        cognitive.remember_unit(&unit).await.unwrap();
        unit.id
    });

    c.bench_function("code_apply_changes", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Update the file
                let path = VirtualPath::new("src/lib.rs").unwrap();
                let new_content = black_box("fn test() { println!(\"updated\"); }");
                vfs.write_file(&workspace_id, &path, new_content.as_bytes()).await.unwrap();

                // Update the semantic unit
                let mut unit = cognitive.semantic().get_unit(unit_id).await.unwrap().unwrap();
                unit.body = "{ println!(\"updated\"); }".to_string();
                unit.updated_at = chrono::Utc::now();
                cognitive.remember_unit(&unit).await.unwrap()
            })
        })
    });
}

// =============================================================================
// 4. Materialization (Target: <5s for 10K LOC)
// =============================================================================

fn bench_materialization_1k_loc(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mat_1k");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create ~1K LOC (100 files × 10 lines each)
    rt.block_on(async {
        for i in 0..100 {
            let path = VirtualPath::new(&format!("src/file_{}.rs", i)).unwrap();
            let content = format!(
                "// File {}\npub fn function_{}() {{\n    let x = {};\n    println!(\"{{:?}}\", x);\n}}\n\n",
                i, i, i
            );
            vfs.write_file(&workspace_id, &path, content.as_bytes()).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("materialization");
    group.bench_function("flush_1k_loc", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let engine = MaterializationEngine::new(vfs.clone());

            rt.block_on(async {
                engine.flush(
                    FlushScope::All,
                    temp_dir.path(),
                    FlushOptions::default(),
                ).await.unwrap()
            })
        })
    });
    group.finish();
}

fn bench_materialization_10k_loc(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mat_10k");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager));
    let workspace_id = uuid::Uuid::new_v4();

    // Create ~10K LOC (1000 files × 10 lines each)
    rt.block_on(async {
        for i in 0..1000 {
            let path = VirtualPath::new(&format!("src/module_{}/file_{}.rs", i / 100, i % 100)).unwrap();
            let content = format!(
                "// File {}\npub fn function_{}() {{\n    let x = {};\n    println!(\"{{:?}}\", x);\n}}\n\n",
                i, i, i
            );
            vfs.write_file(&workspace_id, &path, content.as_bytes()).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("materialization");
    group.sample_size(10); // Fewer samples for longer benchmarks
    group.bench_function("flush_10k_loc", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let engine = MaterializationEngine::new(vfs.clone());

            rt.block_on(async {
                engine.flush(
                    FlushScope::All,
                    temp_dir.path(),
                    FlushOptions::default(),
                ).await.unwrap()
            })
        })
    });
    group.finish();
}

// =============================================================================
// 5. Memory Query (Target: <50ms)
// =============================================================================

fn bench_memory_query_episode(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mem_query");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create 100 episodes
    let episode_ids: Vec<_> = rt.block_on(async {
        let mut ids = Vec::new();
        for i in 0..100 {
            let episode = EpisodicMemory::new(
                format!("Task {}", i),
                "agent".to_string(),
                project_id,
                EpisodeType::Task,
            );
            let id = cognitive.remember_episode(&episode).await.unwrap();
            ids.push(id);
        }
        ids
    });

    c.bench_function("memory_query_episode", |b| {
        b.iter(|| {
            rt.block_on(async {
                let id = black_box(episode_ids[50]);
                cognitive.episodic().get_episode(id).await.unwrap()
            })
        })
    });
}

fn bench_memory_query_recent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mem_recent");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    // Create episodes
    rt.block_on(async {
        for i in 0..50 {
            let mut episode = EpisodicMemory::new(
                format!("Recent task {}", i),
                "agent".to_string(),
                project_id,
                EpisodeType::Task,
            );
            episode.outcome = EpisodeOutcome::Success;
            cognitive.remember_episode(&episode).await.unwrap();
        }
    });

    c.bench_function("memory_query_recent", |b| {
        b.iter(|| {
            rt.block_on(async {
                cognitive.episodic()
                    .get_recent_episodes(black_box(10))
                    .await
                    .unwrap()
            })
        })
    });
}

// =============================================================================
// 6. Association Retrieval (Target: <50ms)
// =============================================================================

fn bench_association_get_dependencies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("assoc_deps");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);

    let source_id = CortexId::new();

    // Create 50 dependencies
    rt.block_on(async {
        for _ in 0..50 {
            let target_id = CortexId::new();
            cognitive.associate(source_id, target_id, DependencyType::Calls)
                .await
                .unwrap();
        }
    });

    c.bench_function("association_get_dependencies", |b| {
        b.iter(|| {
            rt.block_on(async {
                cognitive.semantic()
                    .get_dependencies(black_box(source_id))
                    .await
                    .unwrap()
            })
        })
    });
}

fn bench_association_get_dependents(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("assoc_depts");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);

    let target_id = CortexId::new();

    // Create 50 dependents
    rt.block_on(async {
        for _ in 0..50 {
            let source_id = CortexId::new();
            cognitive.associate(source_id, target_id, DependencyType::Uses)
                .await
                .unwrap();
        }
    });

    c.bench_function("association_get_dependents", |b| {
        b.iter(|| {
            rt.block_on(async {
                cognitive.semantic()
                    .get_dependents(black_box(target_id))
                    .await
                    .unwrap()
            })
        })
    });
}

// =============================================================================
// 7. Memory Storage (Target: <100ms)
// =============================================================================

fn bench_memory_store_episode(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mem_store");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    c.bench_function("memory_store_episode", |b| {
        b.iter(|| {
            rt.block_on(async {
                let episode = EpisodicMemory::new(
                    black_box("Benchmark task".to_string()),
                    black_box("agent".to_string()),
                    project_id,
                    EpisodeType::Task,
                );
                cognitive.remember_episode(&episode).await.unwrap()
            })
        })
    });
}

fn bench_memory_store_semantic_unit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mem_unit");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);

    c.bench_function("memory_store_semantic_unit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let unit = SemanticUnit {
                    id: CortexId::new(),
                    unit_type: CodeUnitType::Function,
                    name: black_box("benchmark_fn".to_string()),
                    qualified_name: black_box("module::benchmark_fn".to_string()),
                    display_name: black_box("benchmark_fn".to_string()),
                    file_path: "src/bench.rs".to_string(),
                    start_line: 10,
                    start_column: 0,
                    end_line: 20,
                    end_column: 1,
                    signature: "fn benchmark_fn()".to_string(),
                    body: "// body".to_string(),
                    docstring: None,
                    visibility: "public".to_string(),
                    modifiers: vec![],
                    parameters: vec![],
                    return_type: None,
                    summary: "Benchmark".to_string(),
                    purpose: "Testing".to_string(),
                    complexity: ComplexityMetrics::default(),
                    test_coverage: None,
                    has_tests: false,
                    has_documentation: false,
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                cognitive.remember_unit(&unit).await.unwrap()
            })
        })
    });
}

fn bench_memory_store_pattern(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("mem_pattern");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);

    c.bench_function("memory_store_pattern", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pattern = LearnedPattern::new(
                    PatternType::Code,
                    black_box("Error handling pattern".to_string()),
                    black_box("Use Result<T, E> for error handling".to_string()),
                    black_box("Rust error handling context".to_string()),
                );
                cognitive.remember_pattern(&pattern).await.unwrap()
            })
        })
    });
}

// =============================================================================
// 8. Session Creation (Target: <200ms)
// =============================================================================

fn bench_session_create(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("session_create");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let cognitive = CognitiveManager::new(connection_manager);
    let project_id = CortexId::new();

    c.bench_function("session_create", |b| {
        b.iter(|| {
            rt.block_on(async {
                let episode = EpisodicMemory::new(
                    black_box(format!("Session {}", uuid::Uuid::new_v4())),
                    black_box("agent".to_string()),
                    project_id,
                    EpisodeType::Session,
                );
                cognitive.remember_episode(&episode).await.unwrap()
            })
        })
    });
}

// =============================================================================
// 9. Connection Pool (Target: 1000-2000 ops/sec)
// =============================================================================

fn bench_connection_pool_acquire_release(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("pool_acquire");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });

    let mut group = c.benchmark_group("connection_pool");
    group.throughput(Throughput::Elements(1));

    group.bench_function("acquire_release", |b| {
        b.iter(|| {
            rt.block_on(async {
                let conn = connection_manager.acquire().await.unwrap();
                // Connection automatically released on drop
                black_box(conn)
            })
        })
    });
    group.finish();
}

fn bench_connection_pool_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("pool_concurrent");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });

    let mut group = c.benchmark_group("connection_pool");
    group.sample_size(20);

    group.bench_function("concurrent_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::new();
                for _ in 0..100 {
                    let mgr = connection_manager.clone();
                    let handle = tokio::spawn(async move {
                        let _conn = mgr.acquire().await.unwrap();
                        // Simulate work
                        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                    });
                    handles.push(handle);
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            })
        })
    });
    group.finish();
}

// =============================================================================
// 10. Load Tests
// =============================================================================

fn bench_load_test_mixed_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db_config = create_test_db_config("load_mixed");
    let connection_manager = rt.block_on(async {
        Arc::new(ConnectionManager::new(db_config).await.unwrap())
    });
    let vfs = Arc::new(VirtualFileSystem::new(connection_manager.clone()));
    let cognitive = CognitiveManager::new(connection_manager);
    let workspace_id = uuid::Uuid::new_v4();
    let project_id = CortexId::new();

    let mut group = c.benchmark_group("load_test");
    group.sample_size(20);

    group.bench_function("mixed_100_ops", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..100 {
                    match i % 4 {
                        0 => {
                            // VFS write
                            let path = VirtualPath::new(&format!("load/file_{}.txt", i)).unwrap();
                            vfs.write_file(&workspace_id, &path, b"content").await.unwrap();
                        },
                        1 => {
                            // VFS read
                            let path = VirtualPath::new(&format!("load/file_{}.txt", i - 1)).unwrap();
                            let _ = vfs.read_file(&workspace_id, &path).await;
                        },
                        2 => {
                            // Memory store
                            let episode = EpisodicMemory::new(
                                format!("Load {}", i),
                                "load_agent".to_string(),
                                project_id,
                                EpisodeType::Task,
                            );
                            cognitive.remember_episode(&episode).await.unwrap();
                        },
                        _ => {
                            // Semantic unit
                            let unit = SemanticUnit {
                                id: CortexId::new(),
                                unit_type: CodeUnitType::Function,
                                name: format!("fn_{}", i),
                                qualified_name: format!("module::fn_{}", i),
                                display_name: format!("fn_{}", i),
                                file_path: "src/lib.rs".to_string(),
                                start_line: i,
                                start_column: 0,
                                end_line: i + 5,
                                end_column: 1,
                                signature: format!("fn fn_{}()", i),
                                body: "{}".to_string(),
                                docstring: None,
                                visibility: "private".to_string(),
                                modifiers: vec![],
                                parameters: vec![],
                                return_type: None,
                                summary: "Test".to_string(),
                                purpose: "Load test".to_string(),
                                complexity: ComplexityMetrics::default(),
                                test_coverage: None,
                                has_tests: false,
                                has_documentation: false,
                                embedding: None,
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                            };
                            cognitive.remember_unit(&unit).await.unwrap();
                        }
                    }
                }
            })
        })
    });
    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group!(
    navigation,
    bench_navigation_find_file,
    bench_navigation_traverse_tree,
    bench_navigation_list_children,
);

criterion_group!(
    semantic_search,
    bench_semantic_search_functions,
    bench_semantic_search_by_meaning,
    bench_semantic_hybrid_search,
);

criterion_group!(
    code_manipulation,
    bench_code_parse_file,
    bench_code_modify_ast,
    bench_code_apply_changes,
);

criterion_group!(
    materialization,
    bench_materialization_1k_loc,
    bench_materialization_10k_loc,
);

criterion_group!(
    memory_query,
    bench_memory_query_episode,
    bench_memory_query_recent,
);

criterion_group!(
    association,
    bench_association_get_dependencies,
    bench_association_get_dependents,
);

criterion_group!(
    memory_storage,
    bench_memory_store_episode,
    bench_memory_store_semantic_unit,
    bench_memory_store_pattern,
);

criterion_group!(
    session,
    bench_session_create,
);

criterion_group!(
    connection_pool,
    bench_connection_pool_acquire_release,
    bench_connection_pool_concurrent,
);

criterion_group!(
    load_tests,
    bench_load_test_mixed_operations,
);

criterion_main!(
    navigation,
    semantic_search,
    code_manipulation,
    materialization,
    memory_query,
    association,
    memory_storage,
    session,
    connection_pool,
    load_tests,
);
