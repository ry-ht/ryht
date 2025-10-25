//! End-to-End Workflow Performance Benchmarks
//!
//! Comprehensive benchmarks for complete agent workflows:
//! - "Find all callers" workflow (<5s for 1,000 files)
//! - "Refactor rename across project" (<10s for 1,000 files)
//! - "Implement new feature" (<2s without LLM)
//! - Token efficiency (75%+ reduction vs traditional)

use cortex_vfs::{
    virtual_filesystem::VirtualFileSystem,
    path::VirtualPath,
};
use cortex_code_analysis::{
    parser::{Parser, Language},
    ast_editor::AstEditor,
    dependency_extractor::DependencyExtractor,
};
use cortex_semantic::{
    search::{SemanticSearch, SearchConfig},
    providers::MockEmbeddingProvider,
    cache::EmbeddingCache,
};
use cortex_memory::{
    working::WorkingMemory,
    episodic::{EpisodicMemory, Episode, EpisodeType},
    semantic::SemanticMemory,
};
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, PoolConfig,
    RetryPolicy, Credentials,
};
use cortex_core::types::CodeUnit;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;
use serde_json::json;

// ==============================================================================
// Test Infrastructure Setup
// ==============================================================================

struct TestProject {
    manager: Arc<ConnectionManager>,
    vfs: VirtualFileSystem,
    semantic_search: SemanticSearch,
    working_memory: WorkingMemory,
    episodic_memory: EpisodicMemory,
    semantic_memory: SemanticMemory,
    workspace_id: Uuid,
}

fn create_test_config() -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "memory".to_string(),
        },
        credentials: Credentials {
            username: None,
            password: None,
        },
        pool_config: PoolConfig {
            min_connections: 10,
            max_connections: 100,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(3600)),
            retry_policy: RetryPolicy {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
                backoff_multiplier: 2.0,
            },
            warm_connections: true,
            health_check_interval: Duration::from_secs(30),
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
        },
        namespace: "e2e_bench_ns".to_string(),
        database: "e2e_bench_db".to_string(),
    }
}

async fn setup_test_project(file_count: usize) -> TestProject {
    let config = create_test_config();
    let manager = ConnectionManager::new(config)
        .await
        .expect("Failed to create connection manager");
    let manager = Arc::new(manager);

    let vfs = VirtualFileSystem::new(manager.clone());
    let workspace_id = Uuid::new_v4();

    vfs.create_workspace(&workspace_id, "benchmark_workspace", None)
        .await
        .expect("Failed to create workspace");

    // Setup semantic search
    let provider = Arc::new(MockEmbeddingProvider::new(384));
    let cache = EmbeddingCache::new(1000);
    let search_config = SearchConfig {
        max_results: 10,
        min_similarity: 0.7,
        use_hybrid_search: false,
        keyword_weight: 0.3,
        semantic_weight: 0.7,
    };
    let semantic_search = SemanticSearch::new(
        manager.clone(),
        provider,
        cache,
        search_config,
    );

    // Setup memory systems
    let working_memory = WorkingMemory::new(manager.clone(), 1000);
    let episodic_memory = EpisodicMemory::new(manager.clone());
    let semantic_memory = SemanticMemory::new(manager.clone());

    // Create realistic project structure
    create_project_structure(&vfs, &workspace_id, file_count).await;

    TestProject {
        manager,
        vfs,
        semantic_search,
        working_memory,
        episodic_memory,
        semantic_memory,
        workspace_id,
    }
}

async fn create_project_structure(
    vfs: &VirtualFileSystem,
    workspace_id: &Uuid,
    file_count: usize,
) {
    // Create directory structure
    let modules = ["core", "api", "utils", "models", "services"];

    for module in &modules {
        let module_dir = VirtualPath::parse(&format!("/src/{}", module)).unwrap();
        vfs.create_directory(workspace_id, &module_dir)
            .await
            .expect("Failed to create module directory");
    }

    // Create files distributed across modules
    let files_per_module = file_count / modules.len();

    for (i, module) in modules.iter().enumerate() {
        for j in 0..files_per_module {
            let file_idx = i * files_per_module + j;
            let file_path = VirtualPath::parse(&format!("/src/{}/file_{}.rs", module, j)).unwrap();

            let content = generate_realistic_rust_file(file_idx, file_count);
            vfs.write_file(workspace_id, &file_path, content.into_bytes())
                .await
                .expect("Failed to write file");
        }
    }
}

fn generate_realistic_rust_file(index: usize, total: usize) -> String {
    let mut code = String::new();

    code.push_str(&format!(
        "//! Module file_{}\n\n\
         use std::collections::HashMap;\n\
         use crate::common::{{Result, Error}};\n\n",
        index
    ));

    // Add imports from other files to create dependencies
    if index > 0 {
        let dep_idx = (index - 1) % total;
        code.push_str(&format!("use crate::file_{}::process_data;\n\n", dep_idx));
    }

    // Add struct
    code.push_str(&format!(
        "#[derive(Debug, Clone)]\n\
         pub struct Data{} {{\n\
         \tpub id: u64,\n\
         \tpub value: String,\n\
         }}\n\n",
        index
    ));

    // Add implementation
    code.push_str(&format!(
        "impl Data{} {{\n\
         \tpub fn new(id: u64, value: String) -> Self {{\n\
         \t\tSelf {{ id, value }}\n\
         \t}}\n\
         \n\
         \tpub fn process(&self) -> Result<String> {{\n\
         \t\tOk(format!(\"Processed: {{}}\", self.value))\n\
         \t}}\n\
         \n\
         \tpub fn get_id(&self) -> u64 {{\n\
         \t\tself.id\n\
         \t}}\n\
         }}\n\n",
        index
    ));

    // Add standalone function that calls other functions
    code.push_str(&format!(
        "pub fn handle_data_{}(data: &Data{}) -> Result<()> {{\n\
         \tlet processed = data.process()?;\n\
         \tprintln!(\"Result: {{}}\", processed);\n\
         \tOk(())\n\
         }}\n",
        index, index
    ));

    code
}

// ==============================================================================
// Workflow 1: Find All Callers
// ==============================================================================

fn bench_find_all_callers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("workflow_find_callers");
    group.significance_level(0.05).sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // Test with different project sizes
    for file_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("find_callers", file_count),
            file_count,
            |b, &count| {
                let project = rt.block_on(setup_test_project(count));

                b.to_async(&rt).iter(|| async {
                    let parser = Parser::new(Language::Rust);
                    let extractor = DependencyExtractor::new();

                    let target_function = "process";
                    let mut callers = Vec::new();

                    // Step 1: List all files
                    let root = VirtualPath::parse("/src").unwrap();
                    let all_files = project.vfs
                        .list_directory(&project.workspace_id, &root, true)
                        .await
                        .unwrap();

                    // Step 2: Parse each file and find callers
                    for file_entry in all_files.iter() {
                        if file_entry.path.ends_with(".rs") {
                            let path = VirtualPath::parse(&file_entry.path).unwrap();
                            let content = project.vfs
                                .read_file(&project.workspace_id, &path)
                                .await
                                .unwrap();

                            let code = String::from_utf8_lossy(&content);
                            if let Ok(ast) = parser.parse(&code) {
                                let calls = extractor.extract_function_calls(&ast);

                                for call in calls {
                                    if call.contains(target_function) {
                                        callers.push(file_entry.path.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    black_box(callers);
                });
            },
        );
    }

    group.finish();
}

// ==============================================================================
// Workflow 2: Refactor Rename Across Project
// ==============================================================================

fn bench_refactor_rename(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("workflow_refactor_rename");
    group.significance_level(0.05).sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    // Test with different project sizes - Target: <10s for 1000 files
    for file_count in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("rename_identifier", file_count),
            file_count,
            |b, &count| {
                let project = rt.block_on(setup_test_project(count));

                b.to_async(&rt).iter(|| async {
                    let editor = AstEditor::new(Language::Rust);
                    let old_name = "process";
                    let new_name = "execute";

                    // Step 1: Find all files containing the identifier
                    let root = VirtualPath::parse("/src").unwrap();
                    let all_files = project.vfs
                        .list_directory(&project.workspace_id, &root, true)
                        .await
                        .unwrap();

                    let mut modified_count = 0;

                    // Step 2: Rename in each file
                    for file_entry in all_files.iter() {
                        if file_entry.path.ends_with(".rs") {
                            let path = VirtualPath::parse(&file_entry.path).unwrap();
                            let content = project.vfs
                                .read_file(&project.workspace_id, &path)
                                .await
                                .unwrap();

                            let code = String::from_utf8_lossy(&content);

                            if code.contains(old_name) {
                                if let Ok(new_code) = editor.rename_identifier(&code, old_name, new_name) {
                                    project.vfs
                                        .write_file(&project.workspace_id, &path, new_code.into_bytes())
                                        .await
                                        .unwrap();
                                    modified_count += 1;
                                }
                            }
                        }
                    }

                    black_box(modified_count);
                });
            },
        );
    }

    group.finish();
}

// ==============================================================================
// Workflow 3: Implement New Feature
// ==============================================================================

fn bench_implement_feature(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let project = rt.block_on(setup_test_project(500));

    let mut group = c.benchmark_group("workflow_implement_feature");
    group.significance_level(0.05).sample_size(20);

    // Complete feature implementation - Target: <2s (without LLM)
    group.bench_function("implement_feature_complete", |b| {
        let mut feature_counter = 0;

        b.to_async(&rt).iter(|| async {
            feature_counter += 1;

            // Step 1: Create new module directory
            let feature_name = format!("feature_{}", feature_counter);
            let feature_dir = VirtualPath::parse(&format!("/src/{}", feature_name)).unwrap();
            project.vfs
                .create_directory(&project.workspace_id, &feature_dir)
                .await
                .unwrap();

            // Step 2: Create 5 new files
            for i in 0..5 {
                let file_path = VirtualPath::parse(&format!(
                    "/src/{}/component_{}.rs",
                    feature_name, i
                )).unwrap();

                let content = format!(
                    "//! Feature {} component {}\n\n\
                     pub struct Component{} {{}}\n\n\
                     impl Component{} {{\n\
                     \tpub fn new() -> Self {{ Self {{}} }}\n\
                     \tpub fn execute(&self) -> String {{ \"executed\".to_string() }}\n\
                     }}",
                    feature_name, i, i, i
                );

                project.vfs
                    .write_file(&project.workspace_id, &file_path, content.into_bytes())
                    .await
                    .unwrap();
            }

            // Step 3: Modify 3 existing files to integrate feature
            for i in 0..3 {
                let existing_path = VirtualPath::parse(&format!("/src/core/file_{}.rs", i)).unwrap();

                if let Ok(content) = project.vfs.read_file(&project.workspace_id, &existing_path).await {
                    let mut code = String::from_utf8_lossy(&content).to_string();

                    // Add import
                    code.insert_str(0, &format!("use crate::{}::component_0::Component0;\n", feature_name));

                    project.vfs
                        .write_file(&project.workspace_id, &existing_path, code.into_bytes())
                        .await
                        .unwrap();
                }
            }

            // Step 4: Create test file
            let test_path = VirtualPath::parse(&format!("/src/{}/tests.rs", feature_name)).unwrap();
            let test_content = format!(
                "#[cfg(test)]\n\
                 mod tests {{\n\
                 \tuse super::*;\n\
                 \n\
                 \t#[test]\n\
                 \tfn test_component() {{\n\
                 \t\tlet comp = Component0::new();\n\
                 \t\tassert_eq!(comp.execute(), \"executed\");\n\
                 \t}}\n\
                 }}"
            );

            project.vfs
                .write_file(&project.workspace_id, &test_path, test_content.into_bytes())
                .await
                .unwrap();

            // Step 5: Update documentation
            let docs_path = VirtualPath::parse(&format!("/src/{}/README.md", feature_name)).unwrap();
            let docs_content = format!("# {}\n\nFeature documentation", feature_name);

            project.vfs
                .write_file(&project.workspace_id, &docs_path, docs_content.into_bytes())
                .await
                .unwrap();

            black_box(feature_name);
        });
    });

    group.finish();
}

// ==============================================================================
// Workflow 4: Code Search and Navigation
// ==============================================================================

fn bench_code_search_navigation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let project = rt.block_on(setup_test_project(1000));

    let mut group = c.benchmark_group("workflow_search_navigation");
    group.significance_level(0.05).sample_size(50);

    // Semantic search + file navigation - Target: <200ms
    group.bench_function("semantic_search_and_open", |b| {
        b.to_async(&rt).iter(|| async {
            // Step 1: Semantic search
            let query_embedding = vec![0.1; 384]; // Mock embedding
            let search_results = project.semantic_search
                .search_by_embedding(&query_embedding, 10, 0.7)
                .await
                .unwrap();

            // Step 2: Open top result
            if let Some(result) = search_results.first() {
                let path = VirtualPath::parse(&result.path).unwrap();
                let content = project.vfs
                    .read_file(&project.workspace_id, &path)
                    .await
                    .unwrap();

                black_box(content);
            }
        });
    });

    // Text search across project - Target: <500ms for 1000 files
    group.bench_function("text_search_1000_files", |b| {
        b.to_async(&rt).iter(|| async {
            let search_term = "process";
            let mut matches = Vec::new();

            let root = VirtualPath::parse("/src").unwrap();
            let all_files = project.vfs
                .list_directory(&project.workspace_id, &root, true)
                .await
                .unwrap();

            for file_entry in all_files.iter() {
                if file_entry.path.ends_with(".rs") {
                    let path = VirtualPath::parse(&file_entry.path).unwrap();
                    let content = project.vfs
                        .read_file(&project.workspace_id, &path)
                        .await
                        .unwrap();

                    let code = String::from_utf8_lossy(&content);
                    if code.contains(search_term) {
                        matches.push(file_entry.path.clone());
                    }
                }
            }

            black_box(matches);
        });
    });

    // Go to definition - Target: <100ms
    group.bench_function("go_to_definition", |b| {
        b.to_async(&rt).iter(|| async {
            let symbol = "Data0";
            let parser = Parser::new(Language::Rust);
            let extractor = DependencyExtractor::new();

            // Search all files for definition
            let root = VirtualPath::parse("/src").unwrap();
            let all_files = project.vfs
                .list_directory(&project.workspace_id, &root, true)
                .await
                .unwrap();

            for file_entry in all_files.iter() {
                if file_entry.path.ends_with(".rs") {
                    let path = VirtualPath::parse(&file_entry.path).unwrap();
                    let content = project.vfs
                        .read_file(&project.workspace_id, &path)
                        .await
                        .unwrap();

                    let code = String::from_utf8_lossy(&content);
                    if let Ok(ast) = parser.parse(&code) {
                        let structs = parser.find_structs(&ast);
                        if structs.iter().any(|s| s.contains(symbol)) {
                            black_box(file_entry.path.clone());
                            break;
                        }
                    }
                }
            }
        });
    });

    group.finish();
}

// ==============================================================================
// Workflow 5: Token Efficiency Measurement
// ==============================================================================

fn bench_token_efficiency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let project = rt.block_on(setup_test_project(100));

    let mut group = c.benchmark_group("workflow_token_efficiency");
    group.significance_level(0.05).sample_size(50);

    // Measure token usage with Cortex vs traditional approach
    group.bench_function("cortex_context_generation", |b| {
        b.to_async(&rt).iter(|| async {
            // Cortex approach: Only relevant code units
            let workspace_id = &project.workspace_id;

            // Get relevant code units from semantic memory
            let relevant_units = project.semantic_memory
                .search_by_kind(workspace_id, "function", 20)
                .await
                .unwrap();

            // Get recent episodes from episodic memory
            let agent_id = Uuid::new_v4();
            let recent_context = project.episodic_memory
                .get_recent(&agent_id, 5)
                .await
                .unwrap();

            // Calculate approximate token count (1 token ≈ 4 chars)
            let mut total_chars = 0;
            for unit in &relevant_units {
                total_chars += unit.content.len();
            }
            for episode in &recent_context {
                total_chars += serde_json::to_string(&episode).unwrap().len();
            }

            let cortex_tokens = total_chars / 4;
            black_box(cortex_tokens);
        });
    });

    // Traditional approach: Load all files
    group.bench_function("traditional_full_context", |b| {
        b.to_async(&rt).iter(|| async {
            let root = VirtualPath::parse("/src").unwrap();
            let all_files = project.vfs
                .list_directory(&project.workspace_id, &root, true)
                .await
                .unwrap();

            let mut total_chars = 0;
            for file_entry in all_files.iter() {
                if file_entry.path.ends_with(".rs") {
                    let path = VirtualPath::parse(&file_entry.path).unwrap();
                    let content = project.vfs
                        .read_file(&project.workspace_id, &path)
                        .await
                        .unwrap();
                    total_chars += content.len();
                }
            }

            let traditional_tokens = total_chars / 4;
            black_box(traditional_tokens);
        });
    });

    group.finish();
}

// ==============================================================================
// Workflow 6: Multi-Agent Collaboration
// ==============================================================================

fn bench_multi_agent_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let project = rt.block_on(setup_test_project(500));

    let mut group = c.benchmark_group("workflow_multi_agent");
    group.significance_level(0.05).sample_size(20);

    // Simulate 5 agents working concurrently - Target: <3s
    group.bench_function("5_agents_concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

            let mut tasks = Vec::new();

            for (i, agent_id) in agent_ids.iter().enumerate() {
                let workspace_id = project.workspace_id;
                let vfs = project.vfs.clone();
                let working = project.working_memory.clone();
                let episodic = project.episodic_memory.clone();

                let task = async move {
                    // Each agent reads different files
                    let file_path = VirtualPath::parse(&format!("/src/core/file_{}.rs", i)).unwrap();
                    let content = vfs.read_file(&workspace_id, &file_path).await.unwrap();

                    // Store in working memory
                    let item = cortex_memory::working::WorkingMemoryItem {
                        key: format!("agent_{}_context", i),
                        value: json!({"file": file_path.to_string(), "size": content.len()}),
                        priority: cortex_memory::working::ItemPriority::Normal,
                        ttl: Some(Duration::from_secs(300)),
                        access_count: 0,
                        last_accessed: chrono::Utc::now(),
                    };
                    working.store(item).await.unwrap();

                    // Record episode
                    let episode = Episode {
                        id: Uuid::new_v4(),
                        agent_id: *agent_id,
                        episode_type: EpisodeType::CodeModification,
                        timestamp: chrono::Utc::now(),
                        actions: vec![json!({"action": "read_file", "path": file_path.to_string()})],
                        outcome: json!({"success": true}),
                        context: json!({"workspace": workspace_id}),
                        duration: Duration::from_millis(100),
                    };
                    episodic.store(episode).await.unwrap();
                };

                tasks.push(task);
            }

            // Execute all agent tasks concurrently
            futures::future::join_all(tasks).await;
        });
    });

    group.finish();
}

// ==============================================================================
// Main Benchmark Configuration
// ==============================================================================

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5));
    targets =
        bench_find_all_callers,
        bench_refactor_rename,
        bench_implement_feature,
        bench_code_search_navigation,
        bench_token_efficiency,
        bench_multi_agent_workflow,
);

criterion_main!(benches);
