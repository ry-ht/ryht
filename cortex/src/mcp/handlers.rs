use crate::context::ContextManager;
use crate::docs::DocIndexer;
use crate::git::GitHistory;
use crate::indexer::{CodeIndexer, DeltaIndexer, Indexer};
use crate::links::LinksStorage;
use crate::memory::MemorySystem;
use crate::metrics::MetricsCollector;
use crate::tasks::TaskManager;
use crate::session::{SessionAction, SessionManager};
use crate::specs::SpecificationManager;
use crate::types::*;
use anyhow::{Context as _, Result, anyhow};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, trace};

/// Handler for all MCP tool calls
pub struct ToolHandlers {
    memory_system: Arc<tokio::sync::RwLock<MemorySystem>>,
    context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
    indexer: Arc<tokio::sync::RwLock<CodeIndexer>>,
    delta_indexer: Option<Arc<DeltaIndexer>>,
    session_manager: Arc<SessionManager>,
    doc_indexer: Arc<DocIndexer>,
    spec_manager: Arc<tokio::sync::RwLock<SpecificationManager>>,
    project_registry: Option<Arc<crate::global::registry::ProjectRegistryManager>>,
    progress_manager: Arc<tokio::sync::RwLock<TaskManager>>,
    links_storage: Arc<tokio::sync::RwLock<dyn LinksStorage>>,
    pattern_engine: Arc<crate::indexer::PatternSearchEngine>,
    metrics_collector: Option<Arc<MetricsCollector>>,
    start_time: std::time::Instant,
    backup_manager: Option<Arc<tokio::sync::RwLock<crate::storage::BackupManager>>>,
    graph_analyzer: Option<Arc<crate::graph::CodeGraphAnalyzer>>,
}

impl ToolHandlers {
    pub fn new(
        memory_system: Arc<tokio::sync::RwLock<MemorySystem>>,
        context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
        indexer: Arc<tokio::sync::RwLock<CodeIndexer>>,
        session_manager: Arc<SessionManager>,
        doc_indexer: Arc<DocIndexer>,
        spec_manager: Arc<tokio::sync::RwLock<SpecificationManager>>,
        progress_manager: Arc<tokio::sync::RwLock<TaskManager>>,
        links_storage: Arc<tokio::sync::RwLock<dyn LinksStorage>>,
        pattern_engine: Arc<crate::indexer::PatternSearchEngine>,
    ) -> Self {
        Self {
            memory_system,
            context_manager,
            indexer,
            delta_indexer: None,
            session_manager,
            doc_indexer,
            spec_manager,
            project_registry: None,
            progress_manager,
            links_storage,
            pattern_engine,
            metrics_collector: None,
            start_time: std::time::Instant::now(),
            backup_manager: None,
            graph_analyzer: None,
        }
    }

    /// Set the metrics collector
    pub fn set_metrics_collector(&mut self, collector: Arc<MetricsCollector>) {
        self.metrics_collector = Some(collector);
    }


    /// Set the delta indexer for real-time file watching
    pub fn set_delta_indexer(&mut self, delta_indexer: Arc<DeltaIndexer>) {
        self.delta_indexer = Some(delta_indexer);
    }

    /// Set the backup manager
    pub fn set_backup_manager(&mut self, backup_manager: Arc<tokio::sync::RwLock<crate::storage::BackupManager>>) {
        self.backup_manager = Some(backup_manager);
    }

    /// Set the graph analyzer
    pub fn set_graph_analyzer(&mut self, graph_analyzer: Arc<crate::graph::CodeGraphAnalyzer>) {
        self.graph_analyzer = Some(graph_analyzer);
    }

    /// Create handlers with project registry for cross-monorepo support
    pub fn new_with_registry(
        memory_system: Arc<tokio::sync::RwLock<MemorySystem>>,
        context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
        indexer: Arc<tokio::sync::RwLock<CodeIndexer>>,
        session_manager: Arc<SessionManager>,
        doc_indexer: Arc<DocIndexer>,
        spec_manager: Arc<tokio::sync::RwLock<SpecificationManager>>,
        project_registry: Arc<crate::global::registry::ProjectRegistryManager>,
        progress_manager: Arc<tokio::sync::RwLock<TaskManager>>,
        links_storage: Arc<tokio::sync::RwLock<dyn LinksStorage>>,
        pattern_engine: Arc<crate::indexer::PatternSearchEngine>,
    ) -> Self {
        Self {
            memory_system,
            context_manager,
            indexer,
            delta_indexer: None,
            session_manager,
            doc_indexer,
            spec_manager,
            project_registry: Some(project_registry),
            progress_manager,
            links_storage,
            pattern_engine,
            metrics_collector: None,
            start_time: std::time::Instant::now(),
            backup_manager: None,
            graph_analyzer: None,
        }
    }

    /// Estimate token count from JSON value (simple heuristic: ~4 chars per token)
    fn estimate_tokens(value: &Value) -> u64 {
        (serde_json::to_string(value).unwrap_or_default().len() / 4) as u64
    }

    /// Route tool call to appropriate handler with metrics instrumentation
    pub async fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        trace!("Handling tool call: {}", name);

        // Start timing
        let start = Instant::now();

        // Count input tokens
        let input_tokens = Self::estimate_tokens(&arguments);

        // Call the actual handler
        let result = self.handle_tool_call_inner(name, arguments).await;

        // Record metrics if collector is available
        if let Some(ref collector) = self.metrics_collector {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let success = result.is_ok();

            trace!("Tool call metrics: {} (latency: {:.2}ms, success: {})", name, latency_ms, success);
            collector.record_tool_call(name, latency_ms, success);

            if let Ok(ref value) = result {
                let output_tokens = Self::estimate_tokens(value);
                trace!("Token count for {}: input={}, output={}", name, input_tokens, output_tokens);
                collector.record_tokens(name, input_tokens, output_tokens);
            } else if let Err(ref e) = result {
                let error_type = format!("{:?}", e);
                let error_category = error_type.split_whitespace().next().unwrap_or("unknown");
                trace!("Error recorded for {}: category={}", name, error_category);
                collector.record_tool_error(name, latency_ms, error_category);
            }
        }

        result
    }

    /// Internal handler without metrics (to avoid double-instrumentation)
    async fn handle_tool_call_inner(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            // Memory Management Tools
            "memory.record_episode" => self.handle_record_episode(arguments).await,
            "memory.find_similar_episodes" => self.handle_find_similar_episodes(arguments).await,
            "memory.update_working_set" => self.handle_update_working_set(arguments).await,
            "memory.get_statistics" => self.handle_get_memory_statistics(arguments).await,

            // Context Management Tools
            "context.prepare_adaptive" => self.handle_prepare_adaptive_context(arguments).await,
            "context.defragment" => self.handle_defragment_context(arguments).await,
            "context.compress" => self.handle_compress_context(arguments).await,

            // Code Navigation Tools
            "code.search_symbols" => self.handle_search_symbols(arguments).await,
            "code.search_patterns" => self.handle_search_patterns(arguments).await,
            "code.get_definition" => self.handle_get_definition(arguments).await,
            "code.find_references" => self.handle_find_references(arguments).await,
            "code.get_dependencies" => self.handle_get_dependencies(arguments).await,

            // Session Management Tools
            "session.begin" => self.handle_begin_session(arguments).await,
            "session.update" => self.handle_update_session(arguments).await,
            "session.query" => self.handle_session_query(arguments).await,
            "session.complete" => self.handle_complete_session(arguments).await,

            // Feedback and Learning Tools
            "feedback.mark_useful" => self.handle_mark_useful(arguments).await,
            "learning.train_on_success" => self.handle_train_on_success(arguments).await,
            "predict.next_action" => self.handle_predict_next_action(arguments).await,

            // Attention-based Retrieval
            "attention.retrieve" => self.handle_attention_retrieve(arguments).await,
            "attention.analyze_patterns" => self.handle_analyze_attention_patterns(arguments).await,

            // Documentation Tools
            "docs.search" => self.handle_docs_search(arguments).await,
            "docs.get_for_symbol" => self.handle_docs_get_for_symbol(arguments).await,

            // History Tools
            "history.get_evolution" => self.handle_history_get_evolution(arguments).await,
            "history.blame" => self.handle_history_blame(arguments).await,

            // Analysis Tools
            "analyze.complexity" => self.handle_analyze_complexity(arguments).await,
            "analyze.token_cost" => self.handle_analyze_token_cost(arguments).await,

            // Monorepo Tools
            "monorepo.list_projects" => self.handle_monorepo_list_projects(arguments).await,
            "monorepo.set_context" => self.handle_monorepo_set_context(arguments).await,
            "monorepo.find_cross_references" => self.handle_monorepo_find_cross_references(arguments).await,

            // Specification Tools
            "specs.list" => self.handle_specs_list(arguments).await,
            "specs.get_structure" => self.handle_specs_get_structure(arguments).await,
            "specs.get_section" => self.handle_specs_get_section(arguments).await,
            "specs.search" => self.handle_specs_search(arguments).await,
            "specs.validate" => self.handle_specs_validate(arguments).await,

            // Catalog Tools (Phase 3)
            "catalog.list_projects" => self.handle_catalog_list_projects(arguments).await,
            "catalog.get_project" => self.handle_catalog_get_project(arguments).await,
            "catalog.search_documentation" => self.handle_catalog_search_documentation(arguments).await,

            // Documentation Generation Tools (Phase 3)
            "docs.generate" => self.handle_docs_generate(arguments).await,
            "docs.validate" => self.handle_docs_validate(arguments).await,
            "docs.transform" => self.handle_docs_transform(arguments).await,

            // Example Tools (Phase 4)
            "examples.generate" => self.handle_examples_generate(arguments).await,
            "examples.validate" => self.handle_examples_validate(arguments).await,

            // Test Tools (Phase 4)
            "tests.generate" => self.handle_tests_generate(arguments).await,
            "tests.validate" => self.handle_tests_validate(arguments).await,

            // Global Tools (Phase 5)
            "global.list_monorepos" => self.handle_global_list_monorepos(arguments).await,
            "global.search_all_projects" => self.handle_global_search_all_projects(arguments).await,
            "global.get_dependency_graph" => self.handle_global_get_dependency_graph(arguments).await,

            // External Tools (Phase 5)
            "external.get_documentation" => self.handle_external_get_documentation(arguments).await,
            "external.find_usages" => self.handle_external_find_usages(arguments).await,

            // Task Management Tools (Phase 2)
            "task.create_task" => self.handle_task_create_task(arguments).await,
            "task.update_task" => self.handle_task_update_task(arguments).await,
            "task.list_tasks" => self.handle_task_list_tasks(arguments).await,
            "task.get_task" => self.handle_task_get_task(arguments).await,
            "task.delete_task" => self.handle_task_delete_task(arguments).await,
            "task.get_progress" => self.handle_task_get_progress(arguments).await,
            "task.search_tasks" => self.handle_task_search_tasks(arguments).await,
            "task.link_to_spec" => self.handle_task_link_to_spec(arguments).await,
            "task.get_history" => self.handle_task_get_history(arguments).await,
            "task.mark_complete" => self.handle_task_mark_complete(arguments).await,
            "task.check_timeouts" => self.handle_task_check_timeouts(arguments).await,
            "task.recover_orphaned" => self.handle_task_recover_orphaned(arguments).await,
            "task.add_dependency" => self.handle_task_add_dependency(arguments).await,
            "task.remove_dependency" => self.handle_task_remove_dependency(arguments).await,
            "task.get_dependencies" => self.handle_task_get_dependencies(arguments).await,
            "task.get_dependents" => self.handle_task_get_dependents(arguments).await,
            "task.can_start_task" => self.handle_task_can_start_task(arguments).await,

            // Semantic Links Tools (Phase 2)
            "links.find_implementation" => self.handle_links_find_implementation(arguments).await,
            "links.find_documentation" => self.handle_links_find_documentation(arguments).await,
            "links.find_examples" => self.handle_links_find_examples(arguments).await,
            "links.find_tests" => self.handle_links_find_tests(arguments).await,
            "links.add_link" => self.handle_links_add_link(arguments).await,
            "links.remove_link" => self.handle_links_remove_link(arguments).await,
            "links.get_links" => self.handle_links_get_links(arguments).await,
            "links.validate" => self.handle_links_validate(arguments).await,
            "links.trace_path" => self.handle_links_trace_path(arguments).await,
            "links.get_health" => self.handle_links_get_health(arguments).await,
            "links.find_orphans" => self.handle_links_find_orphans(arguments).await,
            "links.extract_from_file" => self.handle_links_extract_from_file(arguments).await,

            // Indexer Watch Control Tools
            "indexer.enable_watching" => self.handle_indexer_enable_watching(arguments).await,
            "indexer.disable_watching" => self.handle_indexer_disable_watching(arguments).await,
            "indexer.get_watch_status" => self.handle_indexer_get_watch_status(arguments).await,
            "indexer.poll_changes" => self.handle_indexer_poll_changes(arguments).await,
            "indexer.index_project" => self.handle_indexer_index_project(arguments).await,

            // System Health Tools
            "system.health" => self.handle_system_health(arguments).await,

            // Metrics Tools
            "metrics.get_summary" => self.handle_metrics_get_summary(arguments).await,
            "metrics.get_tool_stats" => self.handle_metrics_get_tool_stats(arguments).await,
            "metrics.get_time_range" => self.handle_metrics_get_time_range(arguments).await,
            "metrics.list_slow_tools" => self.handle_metrics_list_slow_tools(arguments).await,
            "metrics.get_token_efficiency" => self.handle_metrics_get_token_efficiency(arguments).await,
            "metrics.export_prometheus" => self.handle_metrics_export_prometheus(arguments).await,
            "metrics.analyze_trends" => self.handle_metrics_analyze_trends(arguments).await,
            "metrics.get_health" => self.handle_metrics_get_health(arguments).await,

            // Backup Tools
            "backup.create" => self.handle_backup_create(arguments).await,
            "backup.list" => self.handle_backup_list(arguments).await,
            "backup.restore" => self.handle_backup_restore(arguments).await,
            "backup.verify" => self.handle_backup_verify(arguments).await,
            "backup.delete" => self.handle_backup_delete(arguments).await,
            "backup.get_stats" => self.handle_backup_get_stats(arguments).await,
            "backup.create_scheduled" => self.handle_backup_create_scheduled(arguments).await,
            "backup.create_pre_migration" => self.handle_backup_create_pre_migration(arguments).await,

            // Graph Analysis Tools
            "graph.find_dependencies" => self.handle_graph_find_dependencies(arguments).await,
            "graph.find_dependents" => self.handle_graph_find_dependents(arguments).await,
            "graph.semantic_search" => self.handle_graph_semantic_search(arguments).await,
            "graph.find_similar_patterns" => self.handle_graph_find_similar_patterns(arguments).await,
            "graph.impact_analysis" => self.handle_graph_impact_analysis(arguments).await,
            "graph.code_lineage" => self.handle_graph_code_lineage(arguments).await,
            "graph.get_call_graph" => self.handle_graph_get_call_graph(arguments).await,
            "graph.get_callers" => self.handle_graph_get_callers(arguments).await,
            "graph.get_stats" => self.handle_graph_get_stats(arguments).await,
            "graph.find_hubs" => self.handle_graph_find_hubs(arguments).await,
            "graph.find_circular_dependencies" => self.handle_graph_find_circular_dependencies(arguments).await,
            "graph.get_symbol_full" => self.handle_graph_get_symbol_full(arguments).await,

            _ => Err(anyhow!("Unknown tool: {}", name)),
        }
    }

    // === Memory Management Handlers ===

    async fn handle_record_episode(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct RecordEpisodeParams {
            task: String,
            queries_made: Option<Vec<String>>,
            files_accessed: Option<Vec<String>>,
            solution: Option<String>,
            outcome: String,
        }

        let params: RecordEpisodeParams = serde_json::from_value(args)
            .context("Invalid parameters for memory.record_episode")?;

        let outcome = match params.outcome.as_str() {
            "success" => Outcome::Success,
            "failure" => Outcome::Failure,
            "partial" => Outcome::Partial,
            _ => return Err(anyhow!("Invalid outcome value")),
        };

        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: chrono::Utc::now(),
            task_description: params.task,
            initial_context: ContextSnapshot::default(),
            queries_made: params.queries_made.unwrap_or_default(),
            files_touched: params.files_accessed.unwrap_or_default(),
            solution_path: params.solution.unwrap_or_default(),
            outcome,
            tokens_used: TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.0,
        };

        let mut memory = self.memory_system.write().await;
        memory.episodic.record_episode(episode.clone()).await?;

        info!("Recorded episode: {}", episode.id.0);

        Ok(json!({
            "episode_id": episode.id.0,
            "patterns_extracted": [],
            "suggestions": ["Episode recorded for future learning"]
        }))
    }

    async fn handle_find_similar_episodes(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindSimilarParams {
            task_description: String,
            limit: Option<usize>,
        }

        let params: FindSimilarParams = serde_json::from_value(args)
            .context("Invalid parameters for memory.find_similar_episodes")?;

        let memory = self.memory_system.read().await;
        let episodes = memory.episodic
            .find_similar(&params.task_description, params.limit.unwrap_or(5))
            .await;

        let episodes_json: Vec<Value> = episodes
            .iter()
            .map(|e| {
                json!({
                    "episode_id": e.id.0,
                    "task": e.task_description,
                    "outcome": e.outcome.to_string(),
                    "tokens_used": e.tokens_used.0,
                    "timestamp": e.timestamp.to_rfc3339(),
                })
            })
            .collect();

        Ok(json!({
            "episodes": episodes_json,
            "recommended_approach": "Review similar successful episodes",
            "predicted_files": []
        }))
    }

    async fn handle_update_working_set(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct UpdateWorkingSetParams {
            focused_symbols: Vec<FocusedSymbol>,
            #[allow(dead_code)]
            accessed_files: Vec<String>,
            #[allow(dead_code)]
            session_id: String,
        }

        #[derive(Deserialize)]
        struct FocusedSymbol {
            symbol: String,
            weight: f32,
        }

        let params: UpdateWorkingSetParams = serde_json::from_value(args)
            .context("Invalid parameters for memory.update_working_set")?;

        let mut memory = self.memory_system.write().await;

        // Update attention weights in working memory
        for focused in params.focused_symbols {
            let symbol_id = SymbolId::new(focused.symbol);
            memory.working.update_attention_weight(&symbol_id, focused.weight);
        }

        // Evict if needed
        memory.working.evict_if_needed()?;

        Ok(json!({
            "updated_context": {
                "active_symbols": memory.working.get_active_count(),
                "total_tokens": memory.working.estimate_tokens()
            },
            "evicted_symbols": [],
            "prefetched_symbols": []
        }))
    }

    // === Context Management Handlers ===

    async fn handle_prepare_adaptive_context(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct PrepareContextParams {
            #[allow(dead_code)]
            request: Value,
            model: String,
            available_tokens: usize,
        }

        let params: PrepareContextParams = serde_json::from_value(args)
            .context("Invalid parameters for context.prepare_adaptive")?;

        let _adapter = match params.model.as_str() {
            "claude-3" => LLMAdapter::claude3(),
            "gpt-4" => LLMAdapter::gpt4(),
            "gemini" => LLMAdapter::gemini(),
            _ => LLMAdapter::custom(params.available_tokens),
        };

        let manager = self.context_manager.read().await;
        let context_request = ContextRequest {
            files: vec![],
            symbols: vec![],
            max_tokens: Some(TokenCount::new(params.available_tokens as u32)),
        };

        let optimized = manager.prepare_context(&context_request)?;

        Ok(json!({
            "context": optimized.content,
            "compression_ratio": optimized.compression_ratio,
            "strategy_used": optimized.strategy,
            "quality_score": optimized.quality_score(),
            "tokens_used": optimized.token_count.0
        }))
    }

    async fn handle_defragment_context(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct DefragmentParams {
            fragments: Vec<String>,
            target_tokens: usize,
        }

        let params: DefragmentParams = serde_json::from_value(args)
            .context("Invalid parameters for context.defragment")?;

        let manager = self.context_manager.read().await;
        let defragmented = manager.defragment_fragments(params.fragments, params.target_tokens)?;

        Ok(json!({
            "unified": defragmented.content,
            "bridges": defragmented.bridges,
            "narrative_flow": defragmented.narrative
        }))
    }

    // === Code Navigation Handlers ===

    async fn handle_search_symbols(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct SearchSymbolsParams {
            query: String,
            #[serde(rename = "type")]
            symbol_types: Option<Vec<String>>,
            scope: Option<String>,
            detail_level: Option<String>,
            max_results: Option<usize>,
            max_tokens: Option<usize>,
            /// Offset for pagination (default: 0)
            offset: Option<usize>,
            /// Enable cross-encoder reranking (default: true)
            #[serde(default = "default_rerank")]
            rerank: bool,
            /// Number of results to return after reranking (default: 3)
            #[serde(default = "default_rerank_top_k")]
            rerank_top_k: usize,
        }

        fn default_rerank() -> bool { true }
        fn default_rerank_top_k() -> usize { 3 }

        let params: SearchSymbolsParams = serde_json::from_value(args)
            .context("Invalid parameters for code.search_symbols")?;

        let types = params.symbol_types.map(|types| {
            types
                .iter()
                .filter_map(|t| SymbolKind::from_string(t))
                .collect()
        });

        // Parse detail_level parameter
        let detail_level = params.detail_level.as_ref().and_then(|level_str| {
            match level_str.to_lowercase().as_str() {
                "skeleton" => Some(DetailLevel::Skeleton),
                "interface" => Some(DetailLevel::Interface),
                "implementation" => Some(DetailLevel::Implementation),
                "full" => Some(DetailLevel::Full),
                _ => None,
            }
        }).unwrap_or(DetailLevel::Interface); // Default to Interface if not specified or invalid

        // Adjust max_results for reranking:
        // If reranking is enabled, fetch more results (20) for reranking, then return top-k
        let initial_max_results = if params.rerank {
            Some(20) // Fetch 20 candidates for reranking
        } else {
            params.max_results
        };

        let query = Query {
            text: params.query.clone(),
            symbol_types: types,
            scope: params.scope,
            detail_level,
            max_results: initial_max_results,
            max_tokens: params.max_tokens.map(|t| TokenCount::new(t as u32)),
            offset: params.offset,
        };

        use crate::indexer::Indexer;
        let indexer = self.indexer.read().await;
        let results = indexer.search_symbols(&query).await?;

        // Reranking is temporarily disabled (ml module removed)
        // TODO: Re-implement reranking using SurrealDB vector similarity
        let (reranked, original_count) = (false, results.symbols.len());

        let symbols_json: Vec<Value> = results
            .symbols
            .iter()
            .map(|s| {
                let mut symbol_obj = json!({
                    "id": s.id.0,
                    "name": s.name,
                    "kind": s.kind.as_str(),
                    "signature": s.signature,
                    "location": {
                        "file": s.location.file,
                        "line_start": s.location.line_start,
                        "line_end": s.location.line_end
                    },
                    "token_cost": s.metadata.token_cost.0
                });

                // Add detail_level field to show what level was applied
                symbol_obj["detail_level"] = json!(match detail_level {
                    DetailLevel::Skeleton => "skeleton",
                    DetailLevel::Interface => "interface",
                    DetailLevel::Implementation => "implementation",
                    DetailLevel::Full => "full",
                });

                // For Interface and Full levels, include doc_comment if available
                if matches!(detail_level, DetailLevel::Interface | DetailLevel::Full) {
                    if let Some(ref doc) = s.metadata.doc_comment {
                        symbol_obj["doc_comment"] = json!(doc);
                    }
                }

                // For Full level, include dependencies and references info
                if matches!(detail_level, DetailLevel::Full) {
                    symbol_obj["dependencies_count"] = json!(s.dependencies.len());
                    symbol_obj["references_count"] = json!(s.references.len());
                    symbol_obj["complexity"] = json!(s.metadata.complexity);
                }

                symbol_obj
            })
            .collect();

        let mut response = json!({
            "symbols": symbols_json,
            "total_tokens": results.total_tokens.0,
            "truncated": results.truncated,
            "detail_level": match detail_level {
                DetailLevel::Skeleton => "skeleton",
                DetailLevel::Interface => "interface",
                DetailLevel::Implementation => "implementation",
                DetailLevel::Full => "full",
            }
        });

        // Add pagination metadata
        if let Some(total_matches) = results.total_matches {
            response["total_matches"] = json!(total_matches);
        }
        if let Some(offset) = results.offset {
            response["offset"] = json!(offset);
        }
        if let Some(has_more) = results.has_more {
            response["has_more"] = json!(has_more);
        }

        // Add reranking metadata
        if reranked {
            response["reranked"] = json!(true);
            response["original_count"] = json!(original_count);
            response["token_savings_percent"] = json!(
                ((original_count - results.symbols.len()) as f32 / original_count as f32 * 100.0) as u32
            );
        } else {
            response["reranked"] = json!(false);
        }

        Ok(response)
    }

    async fn handle_search_patterns(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize, serde::Serialize)]
        struct SearchPatternsParams {
            pattern: String,
            language: Option<String>,
            scope: Option<String>,
            max_results: Option<usize>,  // Legacy, for backwards compatibility
            page_size: Option<usize>,     // Preferred pagination parameter
            offset: Option<usize>,
        }

        let params: SearchPatternsParams = serde_json::from_value(args)
            .context("Invalid parameters for code.search_patterns")?;

        // Build search configuration
        let config = Self::build_pattern_search_config(&params);
        
        // Collect files to search
        let files_to_search = Self::collect_pattern_search_files(
            &config.scope_path,
            params.language.as_deref(),
        )?;

        // Execute pattern searches with early-exit optimization
        let search_result = self.execute_pattern_search(
            &files_to_search,
            &params.pattern,
            params.language.as_deref(),
            config.target_matches,
        )?;

        // Apply pagination
        let paginated = Self::apply_pattern_pagination(
            search_result.matches,
            search_result.searched_all_files,
            config.offset,
            config.page_size,
        );

        // Build response
        Ok(Self::build_pattern_search_response(
            paginated,
            files_to_search.len(),
            &params.pattern,
            params.language,
            config.offset,
            self.pattern_engine.cache_stats(),
        ))
    }

    async fn handle_get_definition(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetDefinitionParams {
            symbol_id: String,
            #[allow(dead_code)]
            include_body: Option<bool>,
            #[allow(dead_code)]
            include_references: Option<bool>,
            #[allow(dead_code)]
            include_dependencies: Option<bool>,
        }

        let params: GetDefinitionParams = serde_json::from_value(args)
            .context("Invalid parameters for code.get_definition")?;

        use crate::indexer::Indexer;
        let symbol_id = params.symbol_id;
        let indexer = self.indexer.read().await;

        let symbol = indexer
            .get_symbol(&symbol_id)
            .await?
            .ok_or_else(|| anyhow!("Symbol not found"))?;

        let tokens_used = symbol.metadata.token_cost;

        let definition_json = json!({
            "id": symbol.id.0,
            "name": symbol.name,
            "kind": symbol.kind.as_str(),
            "signature": symbol.signature,
            "location": {
                "file": symbol.location.file,
                "line_start": symbol.location.line_start,
                "line_end": symbol.location.line_end
            },
            "doc_comment": symbol.metadata.doc_comment,
            "complexity": symbol.metadata.complexity,
            "token_cost": symbol.metadata.token_cost.0
        });

        Ok(json!({
            "definition": definition_json,
            "tokens_used": tokens_used.0
        }))
    }

    async fn handle_find_references(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindReferencesParams {
            symbol_id: String,
            #[allow(dead_code)]
            include_context: Option<bool>,
            #[allow(dead_code)]
            group_by_file: Option<bool>,
        }

        
        let params: FindReferencesParams = serde_json::from_value(args)
            .context("Invalid parameters for code.find_references")?;

        let symbol_id = SymbolId::new(params.symbol_id);
        let indexer = self.indexer.read().await;

        let references = indexer.find_references(&symbol_id).await?;

        let references_json: Vec<Value> = references
            .iter()
            .map(|r| {
                json!({
                    "symbol_id": r.symbol_id.0,
                    "location": {
                        "file": r.location.file,
                        "line_start": r.location.line_start,
                        "line_end": r.location.line_end
                    },
                    "kind": format!("{:?}", r.kind)
                })
            })
            .collect();

        Ok(json!({
            "references": references_json,
            "summary": {
                "total": references.len(),
                "by_file": {}
            }
        }))
    }

    async fn handle_get_dependencies(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetDependenciesParams {
            entry_point: String,
            depth: Option<usize>,
            direction: Option<String>,
        }

        use crate::indexer::DependencyDirection;
        let params: GetDependenciesParams = serde_json::from_value(args)
            .context("Invalid parameters for code.get_dependencies")?;

        let symbol_id = SymbolId::new(params.entry_point);
        let indexer = self.indexer.read().await;

        let direction = match params.direction.as_deref() {
            Some("imports") => DependencyDirection::Imports,
            Some("exports") => DependencyDirection::Exports,
            _ => DependencyDirection::Both,
        };

        let dependencies = indexer
            .get_dependencies(&symbol_id, params.depth, direction)
            .await?;

        let deps_json: Vec<Value> = dependencies
            .nodes
            .iter()
            .map(|dep_id| json!({ "symbol_id": dep_id.0 }))
            .collect();

        Ok(json!({
            "graph": {
                "nodes": deps_json,
                "edges": []
            },
            "cycles": []
        }))
    }

    // === Session Management Handlers ===

    async fn handle_begin_session(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BeginSessionParams {
            task_description: String,
            scope: Option<Vec<String>>,
            base_commit: Option<String>,
        }

        let params: BeginSessionParams = serde_json::from_value(args)
            .context("Invalid parameters for session.begin")?;

        let scope = params
            .scope
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect();

        let session_id = self
            .session_manager
            .begin(params.task_description, scope, params.base_commit)
            .await?;

        info!("Started session: {}", session_id.0);

        Ok(json!({
            "session_id": session_id.0,
            "workspace": {
                "active": true,
                "base_commit": null
            }
        }))
    }

    async fn handle_update_session(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct UpdateSessionParams {
            session_id: String,
            path: String,
            content: String,
            reindex: Option<bool>,
        }

        let params: UpdateSessionParams = serde_json::from_value(args)
            .context("Invalid parameters for session.update")?;

        let session_id = SessionId(params.session_id);
        let path = PathBuf::from(params.path);

        let status = self
            .session_manager
            .update(&session_id, path, params.content, params.reindex.unwrap_or(true))
            .await?;

        Ok(json!({
            "status": "updated",
            "affected_symbols": status.affected_symbols.len()
        }))
    }

    async fn handle_session_query(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct SessionQueryParams {
            session_id: String,
            query: String,
            prefer_session: Option<bool>,
        }

        let params: SessionQueryParams = serde_json::from_value(args)
            .context("Invalid parameters for session.query")?;

        let session_id = SessionId(params.session_id);
        let query = Query::new(params.query);

        let results = self
            .session_manager
            .query(&session_id, query, params.prefer_session.unwrap_or(true))
            .await?;

        Ok(json!({
            "results": [],
            "from_session": results.from_session,
            "from_base": results.from_base
        }))
    }

    async fn handle_complete_session(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CompleteSessionParams {
            session_id: String,
            action: String,
            #[allow(dead_code)]
            commit_message: Option<String>,
        }

        let params: CompleteSessionParams = serde_json::from_value(args)
            .context("Invalid parameters for session.complete")?;

        let session_id = SessionId(params.session_id);
        let action = match params.action.as_str() {
            "commit" => SessionAction::Commit,
            "discard" => SessionAction::Discard,
            "stash" => SessionAction::Stash,
            _ => return Err(anyhow!("Invalid action")),
        };

        let result = self.session_manager.complete(&session_id, action).await?;

        info!("Completed session: {} with action: {:?}", result.session_id.0, result.action);

        Ok(json!({
            "result": format!("{:?}", result.action),
            "changes_summary": {
                "total_deltas": result.changes_summary.total_deltas,
                "affected_symbols": result.changes_summary.affected_symbols,
                "files_modified": result.changes_summary.files_modified
            }
        }))
    }

    // === Feedback and Learning Handlers ===

    async fn handle_mark_useful(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct MarkUsefulParams {
            session_id: String,
            useful_symbols: Option<Vec<String>>,
            unnecessary_symbols: Option<Vec<String>>,
            #[allow(dead_code)]
            missing_context: Option<String>,
        }

        let params: MarkUsefulParams = serde_json::from_value(args)
            .context("Invalid parameters for feedback.mark_useful")?;

        let mut memory = self.memory_system.write().await;
        let feedback_id = uuid::Uuid::new_v4().to_string();

        // Update attention weights based on feedback
        if let Some(useful) = params.useful_symbols {
            for symbol_name in useful {
                let symbol_id = SymbolId::new(symbol_name);
                memory.working.update_attention_weight(&symbol_id, 2.0);
            }
        }

        if let Some(unnecessary) = params.unnecessary_symbols {
            for symbol_name in unnecessary {
                let symbol_id = SymbolId::new(symbol_name);
                memory.working.update_attention_weight(&symbol_id, 0.1);
            }
        }

        info!("Processed feedback for session: {}", params.session_id);

        Ok(json!({
            "feedback_id": feedback_id,
            "model_updated": true
        }))
    }

    async fn handle_train_on_success(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct TrainParams {
            task: Value,
            solution: Value,
            key_insights: Option<Vec<String>>,
        }

        let params: TrainParams = serde_json::from_value(args)
            .context("Invalid parameters for learning.train_on_success")?;

        let mut memory = self.memory_system.write().await;

        // Extract task description and metadata from task object
        let task_desc = params.task.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown task")
            .to_string();

        // Extract queries made if available
        let queries_made: Vec<String> = params.task.get("queries_made")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        // Extract files accessed if available
        let files_touched: Vec<String> = params.task.get("files_accessed")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        // Extract solution path
        let solution_path = if let Some(sol_str) = params.solution.as_str() {
            sol_str.to_string()
        } else {
            params.solution.to_string()
        };

        // Extract tokens used if available
        let tokens_used = params.task.get("tokens_used")
            .and_then(|v| v.as_u64())
            .map(|t| TokenCount::new(t as u32))
            .unwrap_or_else(TokenCount::zero);

        // Create a rich episode from this successful task
        let episode = TaskEpisode {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: chrono::Utc::now(),
            task_description: task_desc.clone(),
            initial_context: ContextSnapshot {
                active_files: files_touched.clone(),
                active_symbols: vec![],
                working_directory: None,
            },
            queries_made: queries_made.clone(),
            files_touched: files_touched.clone(),
            solution_path: solution_path.clone(),
            outcome: Outcome::Success,
            tokens_used,
            access_count: 0,
            pattern_value: 0.9,
        };

        info!("Recording successful episode: {} - {}", episode.id.0, task_desc);

        // Record the episode in episodic memory
        memory.episodic.record_episode(episode.clone()).await?;

        // Find similar successful episodes for pattern extraction
        let similar_episodes = memory.episodic.find_similar(&task_desc, 10).await;
        debug!("Found {} similar episodes for learning", similar_episodes.len());

        let mut patterns_learned = 0;
        let mut procedure_updated = false;

        // Learn patterns from semantic memory
        if !similar_episodes.is_empty() {
            memory.semantic.learn_patterns(&similar_episodes).await?;
            patterns_learned = memory.semantic.patterns().len();
            debug!("Learned {} semantic patterns", patterns_learned);
        }

        // Learn or update procedural knowledge
        let all_similar = {
            let mut all = similar_episodes;
            all.push(episode.clone());
            all
        };

        if all_similar.len() >= 2 {
            memory.procedural.learn_from_episodes(&all_similar).await?;
            procedure_updated = true;
            info!("Updated procedural memory with {} episodes", all_similar.len());
        }

        // Extract patterns from key insights if provided
        if let Some(insights) = params.key_insights {
            for insight in &insights {
                debug!("Processing insight: {}", insight);
            }
        }

        // Calculate confidence based on number of similar episodes
        let confidence = (all_similar.len() as f32 / 10.0).min(1.0) * 0.9 + 0.1;

        info!(
            "Training complete: {} patterns learned, procedure_updated={}, confidence={:.2}",
            patterns_learned, procedure_updated, confidence
        );

        Ok(json!({
            "patterns_learned": patterns_learned,
            "procedure_updated": procedure_updated,
            "confidence": confidence,
            "similar_episodes_count": all_similar.len() - 1,
            "episode_id": episode.id.0
        }))
    }

    async fn handle_predict_next_action(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct PredictParams {
            current_context: Value,
            task_type: Option<String>,
        }

        let params: PredictParams = serde_json::from_value(args)
            .context("Invalid parameters for predict.next_action")?;

        let memory = self.memory_system.read().await;

        // Extract task description and completed steps
        let task_desc = params.current_context.get("task")
            .and_then(|v| v.as_str())
            .or(params.task_type.as_deref())
            .unwrap_or("Unknown task");

        let completed_steps: Vec<String> = params.current_context.get("completed_steps")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        debug!("Predicting next actions for task: '{}', completed steps: {:?}", task_desc, completed_steps);

        let procedure = memory.procedural.get_procedure_for_task(task_desc);

        let (predicted_actions, suggested_queries, confidence_scores, predicted_files) =
            if let Some(proc) = &procedure {
                Self::predict_from_procedure(proc, &completed_steps)
            } else {
                Self::predict_from_episodes(&memory.episodic, task_desc).await
            };

        Ok(json!({
            "predicted_actions": predicted_actions,
            "suggested_queries": suggested_queries,
            "confidence_scores": confidence_scores,
            "predicted_files": predicted_files,
            "has_procedure": procedure.is_some()
        }))
    }

    fn predict_from_procedure(
        proc: &crate::memory::procedural::Procedure,
        completed_steps: &[String],
    ) -> (Vec<Value>, Vec<String>, Vec<f32>, Vec<String>) {
        info!(
            "Found procedure for task with {} steps, success_rate: {:.2}",
            proc.steps.len(),
            proc.success_rate
        );

        let (next_steps, step_confidences) = Self::extract_next_steps(proc, completed_steps);
        let queries: Vec<String> = proc.typical_queries.iter().take(5).cloned().collect();
        let files: Vec<String> = proc.required_context.iter().take(5).cloned().collect();

        (next_steps, queries, step_confidences, files)
    }

    fn extract_next_steps(
        proc: &crate::memory::procedural::Procedure,
        completed_steps: &[String],
    ) -> (Vec<Value>, Vec<f32>) {
        let mut next_steps = Vec::new();
        let mut step_confidences = Vec::new();

        // Extract pending required steps
        for step in &proc.steps {
            if Self::is_step_completed(step, completed_steps) {
                continue;
            }

            let step_confidence = if step.optional {
                proc.success_rate * 0.7
            } else {
                proc.success_rate
            };

            next_steps.push(Self::step_to_json(step));
            step_confidences.push(step_confidence);

            if next_steps.len() >= 3 {
                break;
            }
        }

        // If all required steps completed, suggest optional steps
        if next_steps.is_empty() {
            for step in proc.steps.iter().filter(|s| s.optional) {
                next_steps.push(Self::step_to_json(step));
                step_confidences.push(proc.success_rate * 0.5);

                if next_steps.len() >= 3 {
                    break;
                }
            }
        }

        (next_steps, step_confidences)
    }

    fn is_step_completed(
        step: &crate::memory::procedural::ProcedureStep,
        completed_steps: &[String],
    ) -> bool {
        completed_steps.iter().any(|completed| {
            step.description.to_lowercase().contains(&completed.to_lowercase())
                || completed.to_lowercase().contains(&step.description.to_lowercase())
        })
    }

    fn step_to_json(step: &crate::memory::procedural::ProcedureStep) -> Value {
        json!({
            "description": step.description,
            "typical_actions": step.typical_actions,
            "expected_files": step.expected_files,
            "optional": step.optional,
            "order": step.order
        })
    }

    async fn predict_from_episodes(
        episodic: &crate::memory::episodic::EpisodicMemory,
        task_desc: &str,
    ) -> (Vec<Value>, Vec<String>, Vec<f32>, Vec<String>) {
        debug!("No procedure found, searching for similar episodes");

        let similar_episodes = episodic.find_similar(task_desc, 5).await;

        if similar_episodes.is_empty() {
            info!("No similar episodes found for task: '{}'", task_desc);
            return (vec![], vec![], vec![], vec![]);
        }

        let (action_freq, query_freq, file_freq) = Self::build_frequency_maps(&similar_episodes);

        let (predicted_actions, confidence) = Self::extract_top_actions(action_freq, similar_episodes.len());
        let suggested_queries = Self::extract_top_items(query_freq, 5);
        let predicted_files = Self::extract_top_items(file_freq, 5);
        let confidences = vec![confidence; predicted_actions.len()];

        info!(
            "Predicted {} actions from {} similar episodes (confidence: {:.2})",
            predicted_actions.len(),
            similar_episodes.len(),
            confidence
        );

        (predicted_actions, suggested_queries, confidences, predicted_files)
    }

    fn build_frequency_maps(
        episodes: &[crate::types::episode::TaskEpisode],
    ) -> (
        std::collections::HashMap<String, usize>,
        std::collections::HashMap<String, usize>,
        std::collections::HashMap<String, usize>,
    ) {
        let mut action_frequency = std::collections::HashMap::new();
        let mut query_frequency = std::collections::HashMap::new();
        let mut file_frequency = std::collections::HashMap::new();

        for episode in episodes {
            for step in episode.solution_path.split(['.', ',', ';']) {
                let trimmed = step.trim();
                if !trimmed.is_empty() {
                    *action_frequency.entry(trimmed.to_string()).or_insert(0) += 1;
                }
            }

            for query in &episode.queries_made {
                *query_frequency.entry(query.clone()).or_insert(0) += 1;
            }

            for file in &episode.files_touched {
                *file_frequency.entry(file.clone()).or_insert(0) += 1;
            }
        }

        (action_frequency, query_frequency, file_frequency)
    }

    fn extract_top_actions(
        action_frequency: std::collections::HashMap<String, usize>,
        episode_count: usize,
    ) -> (Vec<Value>, f32) {
        let mut actions: Vec<(String, usize)> = action_frequency.into_iter().collect();
        actions.sort_by(|a, b| b.1.cmp(&a.1));

        let predicted_actions: Vec<Value> = actions
            .iter()
            .take(3)
            .map(|(action, freq)| {
                json!({
                    "description": action,
                    "frequency": freq,
                    "source": "similar_episodes"
                })
            })
            .collect();

        let avg_frequency = if !actions.is_empty() {
            actions.iter().map(|(_, f)| *f as f32).sum::<f32>() / actions.len() as f32
        } else {
            0.0
        };
        let confidence = (avg_frequency / episode_count as f32).min(1.0);

        (predicted_actions, confidence)
    }

    fn extract_top_items(
        frequency_map: std::collections::HashMap<String, usize>,
        limit: usize,
    ) -> Vec<String> {
        let mut items: Vec<(String, usize)> = frequency_map.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.iter().take(limit).map(|(item, _)| item.clone()).collect()
    }

    async fn handle_attention_retrieve(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct AttentionRetrieveParams {
            attention_pattern: Value,
            token_budget: usize,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: AttentionRetrieveParams = serde_json::from_value(args)
            .context("Invalid parameters for attention.retrieve")?;

        let focused_symbols = extract_focused_symbols(&params.attention_pattern);
        debug!("Retrieving {} focused symbols, budget {} tokens", focused_symbols.len(), params.token_budget);

        let memory = self.memory_system.read().await;
        let indexer = self.indexer.read().await;
        let active_ids = memory.working.active_symbols().clone();

        let mut symbols = load_symbols_with_weights(&memory, &indexer, &focused_symbols).await?;
        apply_attention_boost(&mut symbols);

        let main_budget = (params.token_budget as f32 * 0.8) as usize;
        let prefetch_budget = params.token_budget - main_budget;

        let category = categorize_symbols_by_attention(&symbols, main_budget);
        let (prefetched, prefetch_tokens) = 
            prefetch_related_symbols(&symbols, &memory, &indexer, &active_ids, prefetch_budget).await?;

        let recently_evicted: Vec<String> = memory.working.eviction_history()
            .iter().rev().take(5).map(|s| s.0.clone()).collect();

        info!("Retrieved {} high, {} med, {} ctx + {} prefetch ({} tokens)",
            category.high_attention.len(), category.medium_attention.len(),
            category.context_symbols.len(), prefetched.len(),
            category.total_tokens + prefetch_tokens);

        Ok(build_attention_response(category, prefetched, prefetch_tokens, recently_evicted, params.token_budget))
    }


    async fn handle_analyze_attention_patterns(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct AnalyzeAttentionParams {
            #[allow(dead_code)]
            session_id: String,
            window: Option<usize>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: AnalyzeAttentionParams = serde_json::from_value(args)
            .context("Invalid parameters for attention.analyze_patterns")?;

        let memory = self.memory_system.read().await;
        let _window_size = params.window.unwrap_or(10);

        // Get recent episodes to analyze patterns
        let all_episodes = memory.episodic.episodes();
        let recent_episodes: Vec<&crate::types::TaskEpisode> = all_episodes.iter().rev().take(20).collect();

        // Analyze which files and symbols are frequently accessed
        let mut file_frequency: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for episode in &recent_episodes {
            for file in &episode.files_touched {
                *file_frequency.entry(file.clone()).or_insert(0) += 1;
            }
        }

        let patterns: Vec<Value> = file_frequency.iter()
            .map(|(file, count)| json!({
                "file": file,
                "access_count": count,
                "pattern_type": "frequent_access"
            }))
            .collect();

        let focus_areas: Vec<String> = file_frequency.keys().take(5).cloned().collect();

        // Calculate attention drift (simplified)
        let attention_drift = if recent_episodes.len() > 1 {
            0.3 // Placeholder calculation
        } else {
            0.0
        };

        Ok(json!({
            "patterns": patterns,
            "focus_areas": focus_areas,
            "attention_drift": attention_drift
        }))
    }

    // === Documentation Tools ===

    async fn handle_docs_search(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct DocsSearchParams {
            query: String,
            scope: Option<String>,
            max_results: Option<usize>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: DocsSearchParams = serde_json::from_value(args)
            .context("Invalid parameters for docs.search")?;

        let max_results = params.max_results.unwrap_or(10);

        // Search using DocIndexer for markdown/doc comments
        let doc_results = self.doc_indexer.search_docs(&params.query, max_results).await?;

        // Also search symbols with doc comments
        use crate::indexer::Indexer;
        let indexer = self.indexer.read().await;
        let query = Query {
            text: params.query.clone(),
            symbol_types: None,
            scope: params.scope,
            detail_level: DetailLevel::default(),
            max_results: Some(max_results),
            max_tokens: None,
            offset: None,
        };
        let symbol_results = indexer.search_symbols(&query).await?;

        // Combine results
        let mut all_results: Vec<Value> = Vec::new();

        // Add documentation results
        for doc in &doc_results {
            all_results.push(json!({
                "title": doc.title,
                "content": doc.content,
                "file": doc.file,
                "line_start": doc.line_start,
                "line_end": doc.line_end,
                "section_path": doc.section_path,
                "relevance": doc.relevance,
                "type": match doc.doc_type {
                    crate::docs::DocType::Markdown => "markdown",
                    crate::docs::DocType::DocComment => "doc_comment",
                    crate::docs::DocType::InlineComment => "inline_comment",
                    crate::docs::DocType::CodeBlock => "code_block",
                }
            }));
        }

        // Add symbol doc comments (if not already included)
        for symbol in symbol_results.symbols.iter().filter(|s| s.metadata.doc_comment.is_some()).take(max_results - doc_results.len()) {
            all_results.push(json!({
                "title": symbol.name.clone(),
                "content": symbol.metadata.doc_comment.clone().unwrap_or_default(),
                "file": symbol.location.file.clone(),
                "line_start": symbol.location.line_start,
                "line_end": symbol.location.line_end,
                "section_path": [],
                "relevance": 0.7,
                "type": "symbol_doc"
            }));
        }

        Ok(json!({
            "results": all_results,
            "total_found": all_results.len(),
            "query": params.query
        }))
    }

    async fn handle_docs_get_for_symbol(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct DocsForSymbolParams {
            symbol_id: String,
            #[allow(dead_code)]
            include_examples: Option<bool>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: DocsForSymbolParams = serde_json::from_value(args)
            .context("Invalid parameters for docs.get_for_symbol")?;

        // First, get symbol information
        use crate::indexer::Indexer;
        let indexer = self.indexer.read().await;
        let symbol = indexer.get_symbol(&params.symbol_id).await?
            .ok_or_else(|| anyhow!("Symbol not found"))?;

        // Get documentation from DocIndexer
        let docs = self.doc_indexer.get_docs_for_symbol(&symbol.name).await?;

        let examples: Vec<Value> = Vec::new();
        let mut related_docs = Vec::new();

        // Extract code examples and related documentation
        for doc in docs {
            related_docs.push(json!({
                "title": doc.title,
                "content": doc.content,
                "file": doc.file,
                "line_start": doc.line_start,
                "type": match doc.doc_type {
                    crate::docs::DocType::Markdown => "markdown",
                    crate::docs::DocType::DocComment => "doc_comment",
                    crate::docs::DocType::InlineComment => "inline_comment",
                    crate::docs::DocType::CodeBlock => "code_block",
                }
            }));
        }

        Ok(json!({
            "symbol_id": symbol.id.0,
            "symbol_name": symbol.name,
            "documentation": symbol.metadata.doc_comment,
            "signature": symbol.signature,
            "file": symbol.location.file,
            "examples": examples,
            "related_docs": related_docs
        }))
    }

    // === History Tools ===

    async fn handle_history_get_evolution(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct HistoryEvolutionParams {
            path: String,
            max_commits: Option<usize>,
            #[allow(dead_code)]
            include_diffs: Option<bool>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: HistoryEvolutionParams = serde_json::from_value(args)
            .context("Invalid parameters for history.get_evolution")?;

        let max_commits = params.max_commits.unwrap_or(10);
        let path = PathBuf::from(&params.path);

        // Try to use GitHistory to get file evolution
        match GitHistory::new(&path) {
            Ok(git_history) => {
                let commits = git_history.get_file_evolution(&path, max_commits)?;

                let commits_json: Vec<Value> = commits
                    .iter()
                    .map(|c| {
                        json!({
                            "sha": c.sha,
                            "author": c.author,
                            "author_email": c.author_email,
                            "date": c.date.to_rfc3339(),
                            "message": c.message,
                            "changes": c.changes,
                            "insertions": c.insertions,
                            "deletions": c.deletions
                        })
                    })
                    .collect();

                Ok(json!({
                    "path": path.display().to_string(),
                    "commits": commits_json,
                    "total_commits": commits.len()
                }))
            }
            Err(e) => {
                // Not a git repository or git error
                debug!("Git error for path {:?}: {}", path, e);
                Ok(json!({
                    "path": path.display().to_string(),
                    "commits": [],
                    "total_commits": 0,
                    "error": format!("Not a git repository or git error: {}", e)
                }))
            }
        }
    }

    async fn handle_history_blame(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct HistoryBlameParams {
            path: String,
            line_start: Option<usize>,
            line_end: Option<usize>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: HistoryBlameParams = serde_json::from_value(args)
            .context("Invalid parameters for history.blame")?;

        let path = PathBuf::from(&params.path);

        // Try to use GitHistory to get blame information
        match GitHistory::new(&path) {
            Ok(git_history) => {
                let blame_info = git_history.get_blame(&path, params.line_start, params.line_end)?;

                let blame_json: Vec<Value> = blame_info
                    .iter()
                    .map(|b| {
                        json!({
                            "line": b.line,
                            "author": b.author,
                            "author_email": b.author_email,
                            "sha": b.sha,
                            "date": b.date.to_rfc3339(),
                            "content": b.content
                        })
                    })
                    .collect();

                Ok(json!({
                    "path": params.path,
                    "blame": blame_json,
                    "total_lines": blame_info.len()
                }))
            }
            Err(e) => {
                // Not a git repository or git error
                debug!("Git error for path {:?}: {}", path, e);
                Ok(json!({
                    "path": params.path,
                    "blame": [],
                    "total_lines": 0,
                    "error": format!("Not a git repository or git error: {}", e)
                }))
            }
        }
    }

    // === Analysis Tools ===

    async fn handle_analyze_complexity(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct ComplexityParams {
            target: String,
            #[allow(dead_code)]
            include_metrics: Option<Vec<String>>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: ComplexityParams = serde_json::from_value(args)
            .context("Invalid parameters for analyze.complexity")?;

        use crate::indexer::Indexer;
        let indexer = self.indexer.read().await;

        // Try to get as symbol first, then as file
        let symbol = indexer.get_symbol(&params.target).await?;

        if let Some(sym) = symbol {
            // Analyze symbol complexity
            Ok(json!({
                "target": params.target,
                "type": "symbol",
                "metrics": {
                    "cyclomatic": sym.metadata.complexity,
                    "cognitive": sym.metadata.complexity + 2,
                    "lines": sym.location.line_end - sym.location.line_start,
                    "dependencies": sym.dependencies.len()
                },
                "rating": if sym.metadata.complexity < 10 { "simple" } else if sym.metadata.complexity < 20 { "moderate" } else { "complex" }
            }))
        } else {
            // Assume it's a file path
            Ok(json!({
                "target": params.target,
                "type": "file",
                "metrics": {
                    "cyclomatic": 0,
                    "cognitive": 0,
                    "lines": 0,
                    "dependencies": 0
                },
                "rating": "unknown"
            }))
        }
    }

    async fn handle_analyze_token_cost(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct TokenCostParams {
            items: Vec<TokenCostItem>,
            #[allow(dead_code)]
            model: Option<String>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        #[derive(Deserialize)]
        struct TokenCostItem {
            #[serde(rename = "type")]
            item_type: String,
            identifier: String,
        }

        let params: TokenCostParams = serde_json::from_value(args)
            .context("Invalid parameters for analyze.token_cost")?;

        use crate::indexer::Indexer;
        let indexer = self.indexer.read().await;
        let context_manager = self.context_manager.read().await;

        let mut total_tokens = 0u32;
        let mut item_costs = Vec::new();

        for item in params.items {
            let cost = match item.item_type.as_str() {
                "symbol" => {
                    if let Ok(Some(symbol)) = indexer.get_symbol(&item.identifier).await {
                        symbol.metadata.token_cost.0
                    } else {
                        0
                    }
                }
                "file" => {
                    // Estimate based on file size
                    match tokio::fs::read_to_string(&item.identifier).await {
                        Ok(content) => context_manager.count_tokens(&content),
                        Err(_) => 0,
                    }
                }
                "text" => {
                    context_manager.count_tokens(&item.identifier)
                }
                _ => 0,
            };

            total_tokens += cost;
            item_costs.push(json!({
                "identifier": item.identifier,
                "type": item.item_type,
                "tokens": cost
            }));
        }

        Ok(json!({
            "items": item_costs,
            "total_tokens": total_tokens,
            "estimated_cost_usd": (total_tokens as f64 * 0.00001)
        }))
    }

    // === Monorepo Tools ===

    async fn handle_monorepo_list_projects(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct ListProjectsParams {
            root_path: Option<String>,
            include_dependencies: Option<bool>,
        }

        let params: ListProjectsParams = serde_json::from_value(args)
            .context("Invalid parameters for monorepo.list_projects")?;

        use crate::indexer::MonorepoParser;
        let parser = MonorepoParser::new();

        let root = if let Some(path) = params.root_path {
            PathBuf::from(path)
        } else {
            std::env::current_dir()?
        };

        let projects = parser.detect_projects(&root).await?;

        let projects_json: Vec<Value> = projects.iter().map(|p| {
            json!({
                "name": p.name,
                "path": p.path.display().to_string(),
                "type": format!("{:?}", p.project_type),
                "dependencies": p.dependencies
            })
        }).collect();

        let dependency_graph = if params.include_dependencies.unwrap_or(false) {
            Some(parser.build_dependency_graph(&projects))
        } else {
            None
        };

        Ok(json!({
            "projects": projects_json,
            "total_projects": projects.len(),
            "dependency_graph": dependency_graph
        }))
    }

    async fn handle_monorepo_set_context(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct SetContextParams {
            project_name: String,
            session_id: String,
        }

        let params: SetContextParams = serde_json::from_value(args)
            .context("Invalid parameters for monorepo.set_context")?;

        // Store context in session metadata
        info!("Setting project context to {} for session {}", params.project_name, params.session_id);

        Ok(json!({
            "session_id": params.session_id,
            "active_project": params.project_name,
            "status": "context_updated"
        }))
    }

    async fn handle_monorepo_find_cross_references(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CrossReferencesParams {
            source_project: String,
            target_project: Option<String>,
            reference_type: Option<String>,
        }

        let params: CrossReferencesParams = serde_json::from_value(args)
            .context("Invalid parameters for monorepo.find_cross_references")?;

        info!("Finding cross-references from project: {}", params.source_project);

        // Use project registry and dependency graph to find cross-references
        let mut cross_references = Vec::new();

        if let Some(registry) = &self.project_registry {
            let projects = registry.list_all().await?;

            // Find source project
            let source_project = projects.iter()
                .find(|p| p.identity.id == params.source_project ||
                          p.identity.full_id == params.source_project);

            if let Some(source) = source_project {
                // Parse dependencies from source project
                let source_path = &source.current_path;

                // Parse package.json or Cargo.toml for dependencies
                let deps = Self::parse_project_dependencies(source_path).await?;

                // Filter by target project if specified
                let filtered_deps: Vec<_> = if let Some(ref target) = params.target_project {
                    deps.into_iter()
                        .filter(|(name, _)| name.contains(target) || name == target)
                        .collect()
                } else {
                    deps
                };

                // Find matching projects in registry for each dependency
                for (dep_name, dep_type) in filtered_deps {
                    if let Some(target_proj) = projects.iter().find(|p|
                        p.identity.id == dep_name ||
                        p.identity.id.ends_with(&format!("/{}", dep_name))
                    ) {
                        // Filter by reference type if specified
                        let type_matches = params.reference_type.as_ref()
                            .map(|t| match (t.as_str(), &dep_type) {
                                ("imports", _) => true,  // All deps are imports
                                ("exports", _) => false, // Would need code analysis
                                ("both", _) => true,
                                _ => true,
                            })
                            .unwrap_or(true);

                        if type_matches {
                            cross_references.push(json!({
                                "from_project": source.identity.id.clone(),
                                "to_project": target_proj.identity.id.clone(),
                                "dependency_type": match dep_type.as_str() {
                                    "runtime" => "runtime",
                                    "dev" => "development",
                                    "peer" => "peer",
                                    _ => "unknown",
                                },
                                "reference_type": "import",
                                "from_version": source.identity.version.clone(),
                                "to_version": target_proj.identity.version.clone(),
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!({
            "cross_references": cross_references,
            "source_project": params.source_project,
            "target_project": params.target_project,
            "total": cross_references.len()
        }))
    }

    /// Helper to parse dependencies from package.json or Cargo.toml
    async fn parse_project_dependencies(path: &std::path::Path) -> Result<Vec<(String, String)>> {
        let mut deps = Vec::new();

        // Try package.json
        let package_json = path.join("package.json");
        if package_json.exists() {
            let content = tokio::fs::read_to_string(&package_json).await?;
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                // Runtime dependencies
                if let Some(dependencies) = pkg["dependencies"].as_object() {
                    for (name, _) in dependencies {
                        deps.push((name.clone(), "runtime".to_string()));
                    }
                }
                // Dev dependencies
                if let Some(dev_deps) = pkg["devDependencies"].as_object() {
                    for (name, _) in dev_deps {
                        deps.push((name.clone(), "dev".to_string()));
                    }
                }
                // Peer dependencies
                if let Some(peer_deps) = pkg["peerDependencies"].as_object() {
                    for (name, _) in peer_deps {
                        deps.push((name.clone(), "peer".to_string()));
                    }
                }
            }
        }

        // Try Cargo.toml
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = tokio::fs::read_to_string(&cargo_toml).await?;
            if let Ok(cargo) = toml::from_str::<toml::Value>(&content) {
                // Runtime dependencies
                if let Some(dependencies) = cargo.get("dependencies").and_then(|v| v.as_table()) {
                    for (name, _) in dependencies {
                        deps.push((name.clone(), "runtime".to_string()));
                    }
                }
                // Dev dependencies
                if let Some(dev_deps) = cargo.get("dev-dependencies").and_then(|v| v.as_table()) {
                    for (name, _) in dev_deps {
                        deps.push((name.clone(), "dev".to_string()));
                    }
                }
            }
        }

        Ok(deps)
    }

    // === Memory Statistics ===

    async fn handle_get_memory_statistics(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetStatsParams {
            #[allow(dead_code)]
            include_details: Option<bool>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let _params: GetStatsParams = serde_json::from_value(args)
            .context("Invalid parameters for memory.get_statistics")?;

        let memory = self.memory_system.read().await;

        let stats = memory.working.stats();

        Ok(json!({
            "episodic": {
                "total_episodes": memory.episodic.episodes().len(),
                "recent_episodes": memory.episodic.episodes().iter().rev().take(10).count()
            },
            "working": {
                "active_symbols": memory.working.get_active_count(),
                "current_usage": stats.current_usage,
                "capacity": stats.capacity,
                "utilization": stats.utilization
            },
            "semantic": {
                "total_patterns": memory.semantic.patterns().len()
            },
            "procedural": {
                "total_procedures": memory.procedural.procedures().len()
            }
        }))
    }

    // === Context Compression ===

    async fn handle_compress_context(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CompressParams {
            content: String,
            strategy: String,
            target_ratio: Option<f32>,
            #[allow(dead_code)]
            project_path: Option<String>,
        }

        let params: CompressParams = serde_json::from_value(args)
            .context("Invalid parameters for context.compress")?;

        use crate::types::CompressionStrategy;

        let strategy = match params.strategy.as_str() {
            "remove_comments" => CompressionStrategy::RemoveComments,
            "remove_whitespace" => CompressionStrategy::RemoveWhitespace,
            "skeleton" => CompressionStrategy::Skeleton,
            "summary" => CompressionStrategy::Summary,
            "extract_key_points" => CompressionStrategy::ExtractKeyPoints,
            "tree_shaking" => CompressionStrategy::TreeShaking,
            "hybrid" => CompressionStrategy::Hybrid,
            "ultra_compact" => CompressionStrategy::UltraCompact,
            _ => return Err(anyhow!("Unknown compression strategy: {}", params.strategy)),
        };

        let manager = self.context_manager.read().await;
        let original_tokens = manager.count_tokens(&params.content);
        let target_tokens = if let Some(ratio) = params.target_ratio {
            (original_tokens as f32 * ratio) as usize
        } else {
            (original_tokens as f32 * 0.5) as usize
        };

        let compressed = manager.compress(&params.content, strategy, target_tokens).await?;

        let compressed_tokens = manager.count_tokens(&compressed.content);
        let actual_ratio = compressed_tokens as f32 / original_tokens as f32;

        Ok(json!({
            "compressed_content": compressed.content,
            "original_tokens": original_tokens,
            "compressed_tokens": compressed_tokens,
            "compression_ratio": actual_ratio,
            "quality_score": compressed.quality_score,
            "strategy_used": params.strategy
        }))
    }

    // === Specification Management Handlers ===

    async fn handle_specs_list(&self, _args: Value) -> Result<Value> {
        info!("Listing all specifications");

        let spec_manager = self.spec_manager.read().await;
        let registry = spec_manager.discover_specs()?;

        Ok(json!({
            "specs": registry.specs,
            "total_specs": registry.specs.len()
        }))
    }

    async fn handle_specs_get_structure(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            spec_name: String,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Getting structure for specification: {}", params.spec_name);

        let mut spec_manager = self.spec_manager.write().await;
        let doc = spec_manager.get_spec(&params.spec_name)?;
        let structure = crate::specs::MarkdownAnalyzer::get_structure_summary(&doc);

        Ok(json!({
            "structure": structure,
            "title": doc.title,
            "sections": doc.sections.iter().map(|s| s.title.clone()).collect::<Vec<_>>(),
            "metadata": {
                "version": doc.metadata.version,
                "status": doc.metadata.status,
                "date": doc.metadata.date,
                "authors": doc.metadata.authors
            }
        }))
    }

    async fn handle_specs_get_section(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            spec_name: String,
            section_name: String,
        }

        let params: Params = serde_json::from_value(args)?;
        info!(
            "Getting section '{}' from specification '{}'",
            params.section_name, params.spec_name
        );

        let mut spec_manager = self.spec_manager.write().await;
        let content = spec_manager.get_section(&params.spec_name, &params.section_name)?;

        Ok(json!({
            "content": content,
            "section_name": params.section_name
        }))
    }

    async fn handle_specs_search(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            query: String,
            #[serde(default)]
            max_results: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Searching specifications for: {}", params.query);

        let mut spec_manager = self.spec_manager.write().await;
        let mut results = spec_manager.search_all(&params.query)?;

        // Limit results if specified
        if let Some(max) = params.max_results {
            results.truncate(max);
        }

        Ok(json!({
            "results": results.iter().map(|r| json!({
                "spec_name": r.spec_name,
                "spec_path": r.spec_path,
                "section_title": r.section_title,
                "snippet": r.snippet,
                "line_start": r.line_start,
                "line_end": r.line_end
            })).collect::<Vec<_>>(),
            "total_results": results.len()
        }))
    }

    async fn handle_specs_validate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            spec_name: String,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Validating specification: {}", params.spec_name);

        let mut spec_manager = self.spec_manager.write().await;
        let validation = spec_manager.validate(&params.spec_name)?;

        Ok(json!({
            "valid": validation.valid,
            "completeness_score": validation.completeness_score,
            "issues": validation.issues.iter().map(|issue| json!({
                "severity": format!("{:?}", issue.severity),
                "message": issue.message,
                "section": issue.section
            })).collect::<Vec<_>>()
        }))
    }

    // === Catalog Handlers (Phase 3) ===

    async fn handle_catalog_list_projects(&self, _args: Value) -> Result<Value> {
        use crate::codegen::GlobalCatalog;

        info!("Listing all projects in global catalog");

        let catalog = GlobalCatalog::new();
        let projects = catalog.list_projects();

        let projects_json: Vec<Value> = projects.iter().map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "path": p.path.display().to_string(),
                "symbolCount": p.symbol_count,
                "coverage": p.coverage,
                "dependencies": p.dependencies,
                "description": p.description,
                "totalModules": p.total_modules,
                "totalFunctions": p.total_functions,
                "totalClasses": p.total_classes,
                "totalInterfaces": p.total_interfaces,
                "totalTypes": p.total_types,
                "documentedSymbols": p.documented_symbols,
                "documentationCoverage": p.documentation_coverage,
                "examplesCount": p.examples_count,
                "testsCount": p.tests_count,
                "lastIndexed": p.last_indexed,
                "lastModified": p.last_modified
            })
        }).collect();

        let total_documented = projects.iter().filter(|p| p.coverage > 0.0).count();
        let avg_coverage = if !projects.is_empty() {
            projects.iter().map(|p| p.coverage).sum::<f32>() / projects.len() as f32
        } else {
            0.0
        };

        Ok(json!({
            "projects": projects_json,
            "totalProjects": projects.len(),
            "totalDocumented": total_documented,
            "averageCoverage": avg_coverage
        }))
    }

    async fn handle_catalog_get_project(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "projectId")]
            project_id: String,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Getting project: {}", params.project_id);

        use crate::codegen::GlobalCatalog;
        let catalog = GlobalCatalog::new();

        let project = catalog.get_project(&params.project_id)
            .or_else(|| catalog.get_project_by_name(&params.project_id))
            .ok_or_else(|| anyhow!("Project not found: {}", params.project_id))?;

        Ok(json!({
            "project": {
                "id": project.id,
                "name": project.name,
                "path": project.path.display().to_string(),
                "symbolCount": project.symbol_count,
                "coverage": project.coverage,
                "dependencies": project.dependencies,
                "description": project.description,
                "totalModules": project.total_modules,
                "totalFunctions": project.total_functions,
                "totalClasses": project.total_classes,
                "totalInterfaces": project.total_interfaces,
                "totalTypes": project.total_types,
                "documentedSymbols": project.documented_symbols,
                "documentationCoverage": project.documentation_coverage,
                "examplesCount": project.examples_count,
                "testsCount": project.tests_count,
                "lastIndexed": project.last_indexed,
                "lastModified": project.last_modified
            }
        }))
    }

    async fn handle_catalog_search_documentation(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            query: String,
            #[serde(default)]
            scope: Option<String>,
            #[serde(default)]
            limit: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Searching documentation: query='{}', scope={:?}", params.query, params.scope);

        use crate::codegen::{GlobalCatalog, SearchScope};

        let scope = match params.scope.as_deref() {
            Some("local") => SearchScope::Local,
            Some("dependencies") => SearchScope::Dependencies,
            _ => SearchScope::Global,
        };

        let catalog = GlobalCatalog::new();
        let mut results = catalog.search(&params.query, scope, None)?;

        if let Some(limit) = params.limit {
            results.truncate(limit);
        }

        let results_json: Vec<Value> = results.iter().map(|r| {
            json!({
                "projectId": r.project_id,
                "symbolName": r.symbol_name,
                "content": r.content,
                "filePath": r.file_path,
                "relevance": r.relevance,
                "symbolType": r.symbol_type,
                "qualityScore": r.quality_score
            })
        }).collect();

        Ok(json!({
            "results": results_json,
            "totalResults": results.len()
        }))
    }

    // === Documentation Generation Handlers (Phase 3) ===

    async fn handle_docs_generate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "targetPath")]
            target_path: String,
            format: Option<String>,
            #[serde(rename = "includeExamples", default)]
            #[allow(dead_code)]
            include_examples: Option<bool>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Generating documentation for: {}", params.target_path);

        use crate::codegen::{DocumentationGenerator, DocFormat};
        use crate::indexer::Indexer;

        let format = match params.format.as_deref() {
            Some("jsdoc") => DocFormat::JSDoc,
            Some("rustdoc") => DocFormat::RustDoc,
            Some("markdown") => DocFormat::Markdown,
            _ => DocFormat::TSDoc,
        };

        let generator = DocumentationGenerator::new(format);
        let indexer = self.indexer.read().await;

        // Get symbol from target path
        let symbol = indexer.get_symbol(&params.target_path).await?
            .ok_or_else(|| anyhow!("Symbol not found: {}", params.target_path))?;

        let doc = generator.generate(&symbol)?;

        Ok(json!({
            "documentation": doc.content,
            "quality": {
                "format": format!("{:?}", doc.format),
                "isEnhanced": doc.is_enhanced,
                "hasParameters": doc.metadata.has_parameters,
                "hasReturn": doc.metadata.has_return,
                "hasExamples": doc.metadata.has_examples
            }
        }))
    }

    async fn handle_docs_validate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "targetPath")]
            target_path: String,
            #[serde(default)]
            #[allow(dead_code)]
            standards: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Validating documentation for: {}", params.target_path);

        use crate::codegen::QualityValidator;
        use crate::indexer::Indexer;

        let indexer = self.indexer.read().await;
        let symbol = indexer.get_symbol(&params.target_path).await?
            .ok_or_else(|| anyhow!("Symbol not found: {}", params.target_path))?;

        let validator = QualityValidator::new();
        let doc_content = symbol.metadata.doc_comment.as_deref().unwrap_or("");
        let score = validator.assess(doc_content, &symbol);

        Ok(json!({
            "overallScore": score.overall,
            "symbolScores": [{
                "symbol": symbol.name,
                "score": score.overall,
                "completeness": score.completeness,
                "clarity": score.clarity,
                "accuracy": score.accuracy,
                "compliance": score.compliance,
                "issues": score.issues.iter().map(|i| json!({
                    "severity": format!("{:?}", i.severity),
                    "category": i.category,
                    "message": i.message,
                    "line": i.line
                })).collect::<Vec<_>>(),
                "suggestions": score.suggestions.iter().map(|s| json!({
                    "type": s.suggestion_type,
                    "description": s.description,
                    "example": s.example
                })).collect::<Vec<_>>()
            }]
        }))
    }

    async fn handle_docs_transform(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "targetPath")]
            target_path: String,
            #[serde(rename = "targetFormat")]
            target_format: String,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Transforming documentation for: {} to {}", params.target_path, params.target_format);

        use crate::codegen::{DocumentationGenerator, DocFormat, DocTransformOptions};
        use crate::indexer::Indexer;

        let target_fmt = match params.target_format.as_str() {
            "jsdoc" => DocFormat::JSDoc,
            "rustdoc" => DocFormat::RustDoc,
            "markdown" => DocFormat::Markdown,
            _ => DocFormat::TSDoc,
        };

        let indexer = self.indexer.read().await;
        let symbol = indexer.get_symbol(&params.target_path).await?
            .ok_or_else(|| anyhow!("Symbol not found: {}", params.target_path))?;

        let generator = DocumentationGenerator::new(target_fmt);
        let existing_doc = symbol.metadata.doc_comment.as_deref().unwrap_or("");

        let options = DocTransformOptions {
            preserve_examples: true,
            preserve_links: true,
            preserve_formatting: true,
        };

        let transformed = generator.transform(existing_doc, target_fmt, &options)?;

        Ok(json!({
            "transformedDocs": [{
                "symbol": symbol.name,
                "original": existing_doc,
                "transformed": transformed,
                "format": params.target_format
            }],
            "totalTransformed": 1
        }))
    }

    // === Example Handlers (Phase 4) ===

    async fn handle_examples_generate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            symbol_id: String,
            complexity: Option<String>,
            language: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Generating examples for symbol: {}", params.symbol_id);

        use crate::codegen::ExampleGenerator;
        use crate::indexer::Indexer;

        let indexer = self.indexer.read().await;
        let symbol = indexer.get_symbol(&params.symbol_id).await?
            .ok_or_else(|| anyhow!("Symbol not found: {}", params.symbol_id))?;

        let language = params.language.unwrap_or_else(|| "typescript".to_string());
        let generator = ExampleGenerator::new(language);

        let examples = match params.complexity.as_deref() {
            Some("advanced") => generator.generate_advanced(&symbol)?,
            Some("intermediate") => vec![generator.generate_basic(&symbol)?],
            _ => vec![generator.generate_basic(&symbol)?],
        };

        let examples_json: Vec<Value> = examples.iter().map(|ex| {
            json!({
                "code": ex.code,
                "description": ex.description,
                "language": ex.language,
                "complexity": format!("{:?}", ex.complexity)
            })
        }).collect();

        Ok(json!({
            "examples": examples_json
        }))
    }

    async fn handle_examples_validate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            example: ExampleInput,
        }

        #[derive(Deserialize)]
        struct ExampleInput {
            code: String,
            language: String,
            #[allow(dead_code)]
            description: Option<String>,
            #[allow(dead_code)]
            complexity: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Validating example in language: {}", params.example.language);

        use crate::codegen::ExampleValidator;

        let validator = ExampleValidator::new(params.example.language.clone());
        let result = validator.validate_syntax(&params.example.code)?;

        Ok(json!({
            "valid": result.valid,
            "errors": result.errors,
            "warnings": result.warnings
        }))
    }

    // === Test Handlers (Phase 4) ===

    async fn handle_tests_generate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            symbol_id: String,
            framework: Option<String>,
            test_type: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Generating tests for symbol: {}", params.symbol_id);

        use crate::codegen::{TestGenerator, TestFramework, TestType};
        use crate::indexer::Indexer;

        let indexer = self.indexer.read().await;
        let symbol = indexer.get_symbol(&params.symbol_id).await?
            .ok_or_else(|| anyhow!("Symbol not found: {}", params.symbol_id))?;

        let framework = match params.framework.as_deref() {
            Some("vitest") => TestFramework::Vitest,
            Some("bun") | Some("bun:test") => TestFramework::BunTest,
            Some("rust") => TestFramework::RustNative,
            _ => TestFramework::Jest,
        };

        let test_type = match params.test_type.as_deref() {
            Some("integration") => TestType::Integration,
            Some("e2e") => TestType::E2E,
            _ => TestType::Unit,
        };

        let generator = TestGenerator::new(framework);
        let tests = generator.generate_unit_tests(&symbol)?;

        let tests_json: Vec<Value> = tests.iter().map(|t| {
            json!({
                "name": t.name,
                "code": t.code,
                "framework": format!("{:?}", t.framework),
                "test_type": format!("{:?}", test_type)
            })
        }).collect();

        Ok(json!({
            "tests": tests_json
        }))
    }

    async fn handle_tests_validate(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            test: TestInput,
        }

        #[derive(Deserialize)]
        struct TestInput {
            code: String,
            framework: String,
            name: Option<String>,
            test_type: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Validating test with framework: {}", params.test.framework);

        use crate::codegen::TestFramework;

        let framework = match params.test.framework.as_str() {
            "vitest" => TestFramework::Vitest,
            "bun" | "bun:test" => TestFramework::BunTest,
            "rust" => TestFramework::RustNative,
            _ => TestFramework::Jest,
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Check if code is not empty
        if params.test.code.trim().is_empty() {
            errors.push("Test code cannot be empty".to_string());
        }

        // 2. Framework-specific validation
        let has_test_structure = match framework {
            TestFramework::Jest | TestFramework::Vitest => {
                let _has_describe = params.test.code.contains("describe(");
                let has_it = params.test.code.contains("it(") || params.test.code.contains("test(");
                let has_expect = params.test.code.contains("expect(");

                if !has_it {
                    errors.push("Missing test case (it() or test())".to_string());
                }
                if !has_expect {
                    warnings.push("No assertions found (expect())".to_string());
                }

                has_it
            }
            TestFramework::BunTest => {
                let has_test = params.test.code.contains("test(") || params.test.code.contains("it(");
                let has_expect = params.test.code.contains("expect(");

                if !has_test {
                    errors.push("Missing test case (test() or it())".to_string());
                }
                if !has_expect {
                    warnings.push("No assertions found (expect())".to_string());
                }

                has_test
            }
            TestFramework::RustNative => {
                let has_test_attr = params.test.code.contains("#[test]") || params.test.code.contains("#[tokio::test]");
                let has_assert = params.test.code.contains("assert") || params.test.code.contains("panic!");

                if !has_test_attr {
                    errors.push("Missing #[test] attribute".to_string());
                }
                if !has_assert {
                    warnings.push("No assertions found (assert/assert_eq/panic!)".to_string());
                }

                has_test_attr
            }
        };

        // 3. Syntax validation (basic checks)
        let has_balanced_braces = {
            let open_count = params.test.code.matches('{').count();
            let close_count = params.test.code.matches('}').count();
            open_count == close_count
        };

        if !has_balanced_braces {
            errors.push("Unbalanced braces - syntax error likely".to_string());
        }

        let has_balanced_parens = {
            let open_count = params.test.code.matches('(').count();
            let close_count = params.test.code.matches(')').count();
            open_count == close_count
        };

        if !has_balanced_parens {
            errors.push("Unbalanced parentheses - syntax error likely".to_string());
        }

        // 4. Test type validation
        if let Some(ref test_type) = params.test.test_type {
            match test_type.as_str() {
                "unit" => {
                    // Unit tests should not have network/filesystem calls
                    if params.test.code.contains("fetch(") || params.test.code.contains("axios") {
                        warnings.push("Unit test appears to make network calls - consider mocking".to_string());
                    }
                }
                "integration" => {
                    // Integration tests typically test multiple components
                    // This is just a heuristic
                }
                "e2e" => {
                    // E2E tests should have browser/API interactions
                    if !params.test.code.contains("page.") && !params.test.code.contains("request.") {
                        warnings.push("E2E test should interact with browser or API".to_string());
                    }
                }
                _ => {}
            }
        }

        // 5. Estimate coverage (heuristic based on test complexity)
        let coverage_estimate = if errors.is_empty() {
            let assertion_count = params.test.code.matches("expect(").count()
                + params.test.code.matches("assert").count();
            let test_count = params.test.code.matches("it(").count()
                + params.test.code.matches("test(").count()
                + params.test.code.matches("#[test]").count();

            // Simple heuristic: more assertions and tests = higher estimated coverage
            let base_coverage = 0.5f32;
            let assertion_bonus = (assertion_count as f32 * 0.05).min(0.3);
            let test_bonus = (test_count as f32 * 0.05).min(0.2);

            (base_coverage + assertion_bonus + test_bonus).min(1.0)
        } else {
            0.0
        };

        let valid = errors.is_empty() && has_test_structure;

        Ok(json!({
            "valid": valid,
            "coverage_estimate": coverage_estimate,
            "errors": errors,
            "warnings": warnings,
            "framework": params.test.framework,
            "test_type": params.test.test_type.unwrap_or_else(|| "unknown".to_string()),
            "metrics": {
                "has_test_structure": has_test_structure,
                "has_balanced_braces": has_balanced_braces,
                "has_balanced_parens": has_balanced_parens,
                "assertion_count": params.test.code.matches("expect(").count()
                    + params.test.code.matches("assert").count(),
                "test_count": params.test.code.matches("it(").count()
                    + params.test.code.matches("test(").count()
                    + params.test.code.matches("#[test]").count(),
            }
        }))
    }

    // === Global Handlers (Phase 5) ===

    async fn handle_global_list_monorepos(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "includeInactive", default)]
            include_inactive: Option<bool>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Listing monorepos, includeInactive={:?}", params.include_inactive);

        if let Some(registry) = &self.project_registry {
            let projects = registry.list_all().await?;

            // Group projects by monorepo
            let mut monorepo_map: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();

            for project in projects {
                if let Some(ref mono) = project.monorepo {
                    monorepo_map.entry(mono.id.clone())
                        .or_insert_with(Vec::new)
                        .push(project);
                }
            }

            let monorepos: Vec<Value> = monorepo_map.into_iter().map(|(id, projects)| {
                let first_proj = &projects[0];
                let monorepo_path = first_proj.monorepo.as_ref().map(|m| m.path.clone()).unwrap_or_default();

                json!({
                    "id": id,
                    "name": id.clone(),
                    "path": monorepo_path,
                    "type": "mixed",
                    "projectCount": projects.len(),
                    "lastIndexed": projects.iter()
                        .filter_map(|p| p.indexing.last_indexed.as_ref())
                        .max()
                        .map(|dt| dt.to_rfc3339())
                })
            }).collect();

            Ok(json!({
                "monorepos": monorepos
            }))
        } else {
            // No registry available
            Ok(json!({
                "monorepos": [],
                "error": "Project registry not available - global architecture not initialized"
            }))
        }
    }

    async fn handle_global_search_all_projects(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            query: String,
            #[serde(rename = "monorepoId")]
            monorepo_id: Option<String>,
            #[serde(rename = "maxResults", default)]
            max_results: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Searching all projects: query='{}', monorepoId={:?}", params.query, params.monorepo_id);

        if let Some(registry) = &self.project_registry {
            use crate::codegen::CrossMonorepoAccess;

            let access = CrossMonorepoAccess::new(registry.clone());
            let mut results = access.search_all_projects(&params.query).await?;

            // Filter by monorepo if specified
            if let Some(ref monorepo_id) = params.monorepo_id {
                results.retain(|r| {
                    r.project_id.contains(monorepo_id)
                });
            }

            // Apply limit
            if let Some(max) = params.max_results {
                results.truncate(max);
            }

            let projects_json: Vec<Value> = results.iter().map(|r| json!({
                "projectId": r.project_id,
                "name": r.project_name,
                "matchType": format!("{:?}", r.match_type),
                "relevance": r.relevance
            })).collect();

            Ok(json!({
                "results": projects_json,
                "totalResults": projects_json.len()
            }))
        } else {
            Ok(json!({
                "results": [],
                "totalResults": 0,
                "error": "Project registry not available"
            }))
        }
    }

    async fn handle_global_get_dependency_graph(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "projectId")]
            project_id: String,
            #[allow(dead_code)]
            depth: Option<usize>,
            #[allow(dead_code)]
            direction: Option<String>,
            #[serde(rename = "includeTypes")]
            #[allow(dead_code)]
            include_types: Option<Vec<String>>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Getting dependency graph for project: {}", params.project_id);

        if let Some(registry) = &self.project_registry {
            use crate::codegen::{DependencyParser, DependencyGraph, DependencyNode, DependencyEdge, ReferenceType};

            // Get the project
            let project = registry.get(&params.project_id).await?
                .ok_or_else(|| anyhow!("Project not found: {}", params.project_id))?;

            // Parse dependencies from manifest
            let manifest_result = DependencyParser::parse_manifest(&project.current_path);

            let mut graph = DependencyGraph::new();

            // Add root node
            graph.add_node(DependencyNode {
                id: project.identity.full_id.clone(),
                project_id: project.identity.full_id.clone(),
                name: project.identity.id.clone(),
            });

            if let Ok(manifest) = manifest_result {
                // Build dependency graph
                for dep in &manifest.dependencies {
                    // Try to find this dependency in our registry
                    let dep_projects = registry.find_by_name(&dep.name).await?;

                    for dep_project in dep_projects {
                        // Add node for dependency
                        graph.add_node(DependencyNode {
                            id: dep_project.identity.full_id.clone(),
                            project_id: dep_project.identity.full_id.clone(),
                            name: dep_project.identity.id.clone(),
                        });

                        // Add edge
                        graph.add_edge(DependencyEdge {
                            from: project.identity.full_id.clone(),
                            to: dep_project.identity.full_id.clone(),
                            ref_type: ReferenceType::Import,
                        });
                    }
                }
            }

            // Convert to JSON
            let nodes_json: Vec<Value> = graph.nodes.values().map(|n| json!({
                "id": n.id,
                "name": n.name,
                "type": if n.id == params.project_id { "project" } else { "external" }
            })).collect();

            let edges_json: Vec<Value> = graph.edges.iter().map(|e| json!({
                "from": e.from,
                "to": e.to,
                "type": "dependency",
                "version": "*"
            })).collect();

            // Generate simple Mermaid diagram
            let mut mermaid = String::from("graph TD\n");
            for edge in &graph.edges {
                mermaid.push_str(&format!("  {}[{}] --> {}[{}]\n",
                    edge.from.replace('@', "").replace('/', "_").replace("-", "_"),
                    edge.from,
                    edge.to.replace('@', "").replace('/', "_").replace("-", "_"),
                    edge.to
                ));
            }

            Ok(json!({
                "graph": {
                    "nodes": nodes_json,
                    "edges": edges_json
                },
                "visualization": mermaid,
                "cycles": []
            }))
        } else {
            Ok(json!({
                "graph": {
                    "nodes": [],
                    "edges": []
                },
                "visualization": "graph TD",
                "error": "Project registry not available"
            }))
        }
    }

    // === External Handlers (Phase 5) ===

    async fn handle_external_get_documentation(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "projectId")]
            project_id: String,
            #[serde(rename = "symbolName")]
            symbol_name: Option<String>,
            #[serde(rename = "includeExamples", default)]
            #[allow(dead_code)]
            include_examples: Option<bool>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Getting external documentation: projectId={}, symbolName={:?}", params.project_id, params.symbol_name);

        if let Some(registry) = &self.project_registry {
            use crate::codegen::CrossMonorepoAccess;

            let access = CrossMonorepoAccess::new(registry.clone());

            match access.get_external_docs(&params.project_id, params.symbol_name.as_deref()).await {
                Ok(docs) => {
                    let symbols_json: Vec<Value> = docs.symbols.iter().map(|s| json!({
                        "name": s.name,
                        "type": s.symbol_type,
                        "documentation": s.documentation,
                        "filePath": s.file_path,
                        "line": s.line
                    })).collect();

                    Ok(json!({
                        "project": {
                            "id": docs.project_id,
                            "version": "latest"
                        },
                        "documentation": {
                            "symbols": symbols_json
                        },
                        "fromCache": docs.from_cache,
                        "fetchedAt": docs.fetched_at.to_rfc3339()
                    }))
                }
                Err(e) => {
                    Ok(json!({
                        "documentation": {
                            "symbols": []
                        },
                        "error": format!("Failed to get documentation: {}", e),
                        "accessGranted": false
                    }))
                }
            }
        } else {
            Ok(json!({
                "documentation": {
                    "symbols": []
                },
                "error": "Project registry not available",
                "accessGranted": false
            }))
        }
    }

    async fn handle_external_find_usages(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(rename = "symbolId")]
            symbol_id: String,
            #[serde(rename = "includeTests", default)]
            include_tests: Option<bool>,
            #[serde(rename = "maxResults", default)]
            max_results: Option<usize>,
            #[serde(rename = "monorepoId")]
            monorepo_id: Option<String>,
        }

        let params: Params = serde_json::from_value(args)?;
        info!("Finding usages: symbolId={}, monorepoId={:?}", params.symbol_id, params.monorepo_id);

        if let Some(registry) = &self.project_registry {
            use crate::codegen::CrossMonorepoAccess;

            let access = CrossMonorepoAccess::new(registry.clone());
            let include_tests = params.include_tests.unwrap_or(false);

            match access.find_usages(&params.symbol_id, include_tests).await {
                Ok(mut usages) => {
                    // Filter by monorepo if specified
                    if let Some(ref monorepo_id) = params.monorepo_id {
                        usages.retain(|u| u.project_id.contains(monorepo_id));
                    }

                    // Apply limit
                    if let Some(max) = params.max_results {
                        usages.truncate(max);
                    }

                    let projects_searched = usages.iter()
                        .map(|u| u.project_id.as_str())
                        .collect::<std::collections::HashSet<_>>()
                        .len();

                    let usages_json: Vec<Value> = usages.iter().map(|u| json!({
                        "projectId": u.project_id,
                        "filePath": u.file_path,
                        "line": u.line,
                        "context": u.context,
                        "usageType": format!("{:?}", u.usage_type).to_lowercase()
                    })).collect();

                    Ok(json!({
                        "usages": usages_json,
                        "totalUsages": usages.len(),
                        "projectsSearched": projects_searched
                    }))
                }
                Err(e) => {
                    Ok(json!({
                        "usages": [],
                        "totalUsages": 0,
                        "projectsSearched": 0,
                        "error": format!("Failed to find usages: {}", e)
                    }))
                }
            }
        } else {
            Ok(json!({
                "usages": [],
                "totalUsages": 0,
                "projectsSearched": 0,
                "error": "Project registry not available"
            }))
        }
    }

    // === Task Management Handlers (Phase 2) ===

    async fn handle_task_create_task(&self, args: Value) -> Result<Value> {
        use crate::tasks::{Priority, SpecReference};

        #[derive(Deserialize)]
        struct Params {
            title: String,
            description: Option<String>,
            priority: Option<String>,
            spec_ref: Option<SpecRef>,
            tags: Option<Vec<String>>,
            estimated_hours: Option<f32>,
            timeout_hours: Option<u32>,
        }

        #[derive(Deserialize)]
        struct SpecRef {
            spec_name: String,
            section: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.create_task")?;

        let priority = if let Some(p) = params.priority {
            match p.as_str() {
                "low" => Priority::Low,
                "medium" => Priority::Medium,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => Priority::Medium,
            }
        } else {
            Priority::Medium
        };

        let spec_ref = params.spec_ref.map(|r| SpecReference {
            spec_name: r.spec_name,
            section: r.section,
        });

        let manager = self.progress_manager.read().await;
        let task_id = manager.create_task(
            params.title,
            params.description,
            Some(priority),
            spec_ref,
            params.tags.unwrap_or_default(),
            params.estimated_hours,
            params.timeout_hours,
        ).await?;

        info!("Created task: {}", task_id);

        Ok(json!({
            "task_id": task_id.to_string(),
            "status": "created"
        }))
    }

    async fn handle_task_update_task(&self, args: Value) -> Result<Value> {
        use crate::tasks::{Priority, TaskId, TaskStatus};

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
            title: Option<String>,
            description: Option<String>,
            priority: Option<String>,
            status: Option<String>,
            status_note: Option<String>,
            tags: Option<Vec<String>>,
            estimated_hours: Option<f32>,
            actual_hours: Option<f32>,
            commit_hash: Option<String>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.update_task")?;

        let task_id = TaskId::from_str(&params.task_id);

        let priority = params.priority.as_ref().and_then(|p| match p.as_str() {
            "low" => Some(Priority::Low),
            "medium" => Some(Priority::Medium),
            "high" => Some(Priority::High),
            "critical" => Some(Priority::Critical),
            _ => None,
        });

        let status = params.status.as_ref().and_then(|s| match s.as_str() {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" => Some(TaskStatus::InProgress),
            "blocked" => Some(TaskStatus::Blocked),
            "done" => Some(TaskStatus::Done),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        });

        let manager = self.progress_manager.read().await;
        manager.update_task(
            &task_id,
            params.title,
            params.description,
            priority,
            status,
            params.status_note,
            params.tags,
            params.estimated_hours,
            params.actual_hours,
            params.commit_hash,
        ).await?;

        info!("Updated task: {}", task_id);

        Ok(json!({
            "task_id": task_id.to_string(),
            "status": "updated"
        }))
    }

    async fn handle_task_list_tasks(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskStatus;

        #[derive(Deserialize)]
        struct Params {
            status: Option<String>,
            spec_name: Option<String>,
            limit: Option<usize>,
        }

        let params: Params = serde_json::from_value(args).unwrap_or(Params {
            status: None,
            spec_name: None,
            limit: None,
        });

        let status = params.status.as_ref().and_then(|s| match s.as_str() {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" => Some(TaskStatus::InProgress),
            "blocked" => Some(TaskStatus::Blocked),
            "done" => Some(TaskStatus::Done),
            "cancelled" => Some(TaskStatus::Cancelled),
            _ => None,
        });

        let manager = self.progress_manager.read().await;
        let tasks = manager.list_tasks(status, params.spec_name, params.limit).await?;

        info!("Listed {} tasks", tasks.len());

        Ok(json!({
            "tasks": tasks,
            "total": tasks.len()
        }))
    }

    async fn handle_task_get_task(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.get_task")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;
        let task = manager.get_task(&task_id).await?;

        Ok(json!(task))
    }

    async fn handle_task_delete_task(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.delete_task")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;
        manager.delete_task(&task_id).await?;

        info!("Deleted task: {}", task_id);

        Ok(json!({
            "task_id": task_id.to_string(),
            "status": "deleted"
        }))
    }

    async fn handle_task_get_progress(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            spec_name: Option<String>,
        }

        let params: Params = serde_json::from_value(args).unwrap_or(Params { spec_name: None });

        let manager = self.progress_manager.read().await;
        let stats = manager.get_progress(params.spec_name).await?;

        Ok(json!(stats))
    }

    async fn handle_task_search_tasks(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            query: String,
            limit: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.search_tasks")?;

        let manager = self.progress_manager.read().await;
        let matching_tasks = manager.search_tasks(&params.query, params.limit).await?;

        info!("Found {} matching tasks for query: {}", matching_tasks.len(), params.query);

        Ok(json!({
            "tasks": matching_tasks,
            "total": matching_tasks.len()
        }))
    }

    async fn handle_task_link_to_spec(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
            spec_name: String,
            section: String,
            #[serde(default = "default_validate")]
            validate: bool,
        }

        fn default_validate() -> bool {
            true
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.link_to_spec")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;

        manager.link_to_spec(
            &task_id,
            params.spec_name.clone(),
            params.section.clone(),
            params.validate,
            self.spec_manager.clone(),
        ).await?;

        info!("Linked task {} to spec {}", task_id, params.spec_name);

        Ok(json!({
            "task_id": task_id.to_string(),
            "spec_name": params.spec_name,
            "section": params.section,
            "status": "linked"
        }))
    }

    async fn handle_task_get_history(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.get_history")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;
        let history = manager.get_history(&task_id).await?;
        let task = manager.get_task(&task_id).await?;

        Ok(json!({
            "task_id": task_id.to_string(),
            "history": history,
            "created_at": task.created_at,
            "updated_at": task.updated_at,
            "completed_at": task.completed_at
        }))
    }

    async fn handle_task_mark_complete(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
            #[allow(dead_code)]
            note: Option<String>,
            actual_hours: Option<f32>,
            commit_hash: Option<String>,
            solution_summary: Option<String>,
            #[serde(default)]
            files_touched: Vec<String>,
            #[serde(default)]
            queries_made: Vec<String>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.mark_complete")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;

        // Use the new mark_complete method with memory integration
        let episode_id = manager.mark_complete(
            &task_id,
            params.actual_hours,
            params.commit_hash,
            params.solution_summary,
            params.files_touched,
            params.queries_made,
            self.memory_system.clone(),
        ).await?;

        let task = manager.get_task(&task_id).await?;

        info!("Marked task {} as complete with episode {:?}", task_id, episode_id);

        Ok(json!({
            "task_id": task_id.to_string(),
            "status": "done",
            "completed_at": task.completed_at,
            "episode_id": episode_id,
            "episode_recorded": episode_id.is_some()
        }))
    }

    async fn handle_task_check_timeouts(&self, _args: Value) -> Result<Value> {
        let manager = self.progress_manager.read().await;
        let recovered = manager.check_timeouts().await?;

        info!("Checked timeouts, recovered {} tasks", recovered.len());

        Ok(json!({
            "recovered_count": recovered.len(),
            "task_ids": recovered.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "status": "checked"
        }))
    }

    async fn handle_task_recover_orphaned(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            #[serde(default)]
            force: bool,
        }

        let params: Params = serde_json::from_value(args).unwrap_or(Params { force: false });

        let manager = self.progress_manager.read().await;
        let recovered = manager.recover_orphaned_tasks(params.force).await?;

        info!("Recovered {} orphaned tasks (force={})", recovered.len(), params.force);

        Ok(json!({
            "recovered_count": recovered.len(),
            "task_ids": recovered.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            "force": params.force,
            "status": "recovered"
        }))
    }


    async fn handle_task_add_dependency(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
            depends_on: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.add_dependency")?;

        let task_id = TaskId::from_str(&params.task_id);
        let depends_on = TaskId::from_str(&params.depends_on);
        let manager = self.progress_manager.read().await;

        manager.add_dependency(&task_id, &depends_on).await?;

        info!("Added dependency: {} depends on {}", task_id, depends_on);

        Ok(json!({
            "task_id": task_id.to_string(),
            "depends_on": depends_on.to_string(),
            "status": "added"
        }))
    }

    async fn handle_task_remove_dependency(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
            depends_on: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.remove_dependency")?;

        let task_id = TaskId::from_str(&params.task_id);
        let depends_on = TaskId::from_str(&params.depends_on);
        let manager = self.progress_manager.read().await;

        manager.remove_dependency(&task_id, &depends_on).await?;

        info!("Removed dependency: {} no longer depends on {}", task_id, depends_on);

        Ok(json!({
            "task_id": task_id.to_string(),
            "depends_on": depends_on.to_string(),
            "status": "removed"
        }))
    }

    async fn handle_task_get_dependencies(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.get_dependencies")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;

        let dependencies = manager.get_dependencies(&task_id).await?;
        let unmet = manager.get_unmet_dependencies(&task_id).await?;

        let dependencies_list: Vec<Value> = dependencies.iter().map(|t| json!({
            "id": t.id.to_string(),
            "title": t.title,
            "status": t.status.to_string(),
            "priority": t.priority.to_string()
        })).collect();

        let unmet_ids: Vec<String> = unmet.iter().map(|t| t.id.to_string()).collect();

        Ok(json!({
            "task_id": task_id.to_string(),
            "dependencies": dependencies_list,
            "total_dependencies": dependencies.len(),
            "unmet_dependencies": unmet_ids,
            "all_met": unmet.is_empty()
        }))
    }

    async fn handle_task_get_dependents(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.get_dependents")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;

        let dependents = manager.get_dependents(&task_id).await?;

        let dependents_list: Vec<Value> = dependents.iter().map(|t| json!({
            "id": t.id.to_string(),
            "title": t.title,
            "status": t.status.to_string(),
            "priority": t.priority.to_string(),
            "blocked": t.status.to_string() == "blocked"
        })).collect();

        Ok(json!({
            "task_id": task_id.to_string(),
            "dependents": dependents_list,
            "total_dependents": dependents.len(),
            "blocks": dependents.iter().map(|t| t.id.to_string()).collect::<Vec<_>>()
        }))
    }

    async fn handle_task_can_start_task(&self, args: Value) -> Result<Value> {
        use crate::tasks::TaskId;

        #[derive(Deserialize)]
        struct Params {
            task_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for task.can_start_task")?;

        let task_id = TaskId::from_str(&params.task_id);
        let manager = self.progress_manager.read().await;

        let can_start = manager.can_start_task(&task_id).await?;
        let task = manager.get_task(&task_id).await?;
        let unmet = if !can_start {
            manager.get_unmet_dependencies(&task_id).await?
        } else {
            Vec::new()
        };

        let blockers: Vec<Value> = unmet.iter().map(|t| json!({
            "id": t.id.to_string(),
            "title": t.title,
            "status": t.status.to_string()
        })).collect();

        Ok(json!({
            "task_id": task_id.to_string(),
            "can_start": can_start,
            "current_status": task.status.to_string(),
            "blockers": blockers,
            "reason": if !can_start {
                if !task.can_start() {
                    format!("Task status is {} (must be pending or blocked)", task.status)
                } else {
                    format!("{} unmet dependencies", unmet.len())
                }
            } else {
                "All dependencies met".to_string()
            }
        }))
    }

    // === Semantic Links Handlers (Phase 2) ===

    async fn handle_links_find_implementation(&self, args: Value) -> Result<Value> {
        use crate::links::{LinkTarget, LinkType};

        #[derive(Deserialize)]
        struct Params {
            spec_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.find_implementation")?;

        let source = LinkTarget::spec(params.spec_id.clone());
        let storage = self.links_storage.read().await;
        let links = storage.find_links_by_type_from_source(LinkType::ImplementedBy, &source).await?;

        info!("Found {} implementations for {}", links.len(), params.spec_id);

        let implementations: Vec<Value> = links.iter().map(|link| json!({
            "target": link.target.id,
            "confidence": link.confidence,
            "context": link.context,
            "created_at": link.created_at,
            "validation_status": link.validation_status.as_str()
        })).collect();

        Ok(json!({
            "spec_id": params.spec_id,
            "implementations": implementations,
            "total": implementations.len()
        }))
    }

    async fn handle_links_find_documentation(&self, args: Value) -> Result<Value> {
        use crate::links::{LinkTarget, LinkType};

        #[derive(Deserialize)]
        struct Params {
            code_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.find_documentation")?;

        let source = LinkTarget::code(params.code_id.clone());
        let storage = self.links_storage.read().await;
        let links = storage.find_links_by_type_from_source(LinkType::DocumentedIn, &source).await?;

        info!("Found {} documentation links for {}", links.len(), params.code_id);

        let documentation: Vec<Value> = links.iter().map(|link| json!({
            "target": link.target.id,
            "confidence": link.confidence,
            "context": link.context,
            "validation_status": link.validation_status.as_str()
        })).collect();

        Ok(json!({
            "code_id": params.code_id,
            "documentation": documentation,
            "total": documentation.len()
        }))
    }

    async fn handle_links_find_examples(&self, args: Value) -> Result<Value> {
        use crate::links::{LinkTarget, LinkType};

        #[derive(Deserialize)]
        struct Params {
            code_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.find_examples")?;

        let source = LinkTarget::code(params.code_id.clone());
        let storage = self.links_storage.read().await;
        let links = storage.find_links_by_type_from_source(LinkType::DemonstratedIn, &source).await?;

        info!("Found {} examples for {}", links.len(), params.code_id);

        let examples: Vec<Value> = links.iter().map(|link| json!({
            "target": link.target.id,
            "confidence": link.confidence,
            "context": link.context,
            "validation_status": link.validation_status.as_str()
        })).collect();

        Ok(json!({
            "code_id": params.code_id,
            "examples": examples,
            "total": examples.len()
        }))
    }

    async fn handle_links_find_tests(&self, args: Value) -> Result<Value> {
        use crate::links::{LinkTarget, LinkType};

        #[derive(Deserialize)]
        struct Params {
            code_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.find_tests")?;

        let source = LinkTarget::code(params.code_id.clone());
        let storage = self.links_storage.read().await;
        let links = storage.find_links_by_type_from_source(LinkType::TestedBy, &source).await?;

        info!("Found {} tests for {}", links.len(), params.code_id);

        let tests: Vec<Value> = links.iter().map(|link| json!({
            "target": link.target.id,
            "confidence": link.confidence,
            "context": link.context,
            "validation_status": link.validation_status.as_str()
        })).collect();

        Ok(json!({
            "code_id": params.code_id,
            "tests": tests,
            "total": tests.len()
        }))
    }

    async fn handle_links_add_link(&self, args: Value) -> Result<Value> {
        use crate::links::{ExtractionMethod, KnowledgeLevel, LinkTarget, LinkType, SemanticLink};

        #[derive(Deserialize)]
        struct Params {
            link_type: String,
            source_level: String,
            source_id: String,
            target_level: String,
            target_id: String,
            confidence: Option<f32>,
            context: Option<String>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.add_link")?;

        let link_type = LinkType::from_str(&params.link_type)
            .ok_or_else(|| anyhow!("Invalid link_type: {}", params.link_type))?;

        let source_level = KnowledgeLevel::from_str(&params.source_level)
            .ok_or_else(|| anyhow!("Invalid source_level: {}", params.source_level))?;

        let target_level = KnowledgeLevel::from_str(&params.target_level)
            .ok_or_else(|| anyhow!("Invalid target_level: {}", params.target_level))?;

        let source = LinkTarget::new(source_level, params.source_id);
        let target = LinkTarget::new(target_level, params.target_id);

        let mut link = SemanticLink::new(
            link_type,
            source,
            target,
            params.confidence.unwrap_or(1.0),
            ExtractionMethod::Manual,
            "mcp".to_string(),
        );

        if let Some(context) = params.context {
            link = link.with_context(context);
        }

        let storage = self.links_storage.write().await;
        storage.add_link(&link).await?;

        info!("Added link: {} -> {}", link.source.key(), link.target.key());

        Ok(json!({
            "link_id": link.id.to_string(),
            "status": "added"
        }))
    }

    async fn handle_links_remove_link(&self, args: Value) -> Result<Value> {
        use crate::links::LinkId;

        #[derive(Deserialize)]
        struct Params {
            link_id: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.remove_link")?;

        let link_id = LinkId::from_string(params.link_id.clone());
        let storage = self.links_storage.write().await;
        storage.remove_link(&link_id).await?;

        info!("Removed link: {}", params.link_id);

        Ok(json!({
            "link_id": params.link_id,
            "status": "removed"
        }))
    }

    async fn handle_links_get_links(&self, args: Value) -> Result<Value> {
        use crate::links::{KnowledgeLevel, LinkTarget};

        #[derive(Deserialize)]
        struct Params {
            entity_level: String,
            entity_id: String,
            direction: Option<String>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.get_links")?;

        let level = KnowledgeLevel::from_str(&params.entity_level)
            .ok_or_else(|| anyhow!("Invalid entity_level: {}", params.entity_level))?;

        let entity = LinkTarget::new(level, params.entity_id.clone());
        let storage = self.links_storage.read().await;
        let bi_links = storage.get_bidirectional_links(&entity).await?;

        let direction = params.direction.as_deref().unwrap_or("both");

        let (outgoing, incoming) = match direction {
            "outgoing" => (bi_links.outgoing, Vec::new()),
            "incoming" => (Vec::new(), bi_links.incoming),
            _ => (bi_links.outgoing, bi_links.incoming),
        };

        let outgoing_json: Vec<Value> = outgoing.iter().map(|link| json!({
            "link_id": link.id.to_string(),
            "link_type": link.link_type.as_str(),
            "target": link.target.key(),
            "confidence": link.confidence,
            "validation_status": link.validation_status.as_str()
        })).collect();

        let incoming_json: Vec<Value> = incoming.iter().map(|link| json!({
            "link_id": link.id.to_string(),
            "link_type": link.link_type.as_str(),
            "source": link.source.key(),
            "confidence": link.confidence,
            "validation_status": link.validation_status.as_str()
        })).collect();

        Ok(json!({
            "entity": entity.key(),
            "outgoing": outgoing_json,
            "incoming": incoming_json,
            "total_outgoing": outgoing_json.len(),
            "total_incoming": incoming_json.len()
        }))
    }

    async fn handle_links_validate(&self, args: Value) -> Result<Value> {
        use crate::links::{LinkId, ValidationStatus};

        #[derive(Deserialize)]
        struct Params {
            link_id: String,
            status: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.validate")?;

        let validation_status = match params.status.as_str() {
            "valid" => ValidationStatus::Valid,
            "broken" => ValidationStatus::Broken,
            "stale" => ValidationStatus::Stale,
            "unchecked" => ValidationStatus::Unchecked,
            _ => return Err(anyhow!("Invalid status: {}", params.status)),
        };

        let link_id = LinkId::from_string(params.link_id.clone());
        let storage = self.links_storage.write().await;
        storage.validate_link(&link_id, validation_status).await?;

        info!("Validated link {} as {}", params.link_id, params.status);

        Ok(json!({
            "link_id": params.link_id,
            "validation_status": params.status,
            "status": "validated"
        }))
    }

    async fn handle_links_trace_path(&self, args: Value) -> Result<Value> {
        use crate::links::{KnowledgeLevel, LinkTarget};

        #[derive(Deserialize)]
        struct Params {
            from_level: String,
            from_id: String,
            to_level: String,
            to_id: String,
            max_depth: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.trace_path")?;

        let from_level = KnowledgeLevel::from_str(&params.from_level)
            .ok_or_else(|| anyhow!("Invalid from_level: {}", params.from_level))?;

        let to_level = KnowledgeLevel::from_str(&params.to_level)
            .ok_or_else(|| anyhow!("Invalid to_level: {}", params.to_level))?;

        let from = LinkTarget::new(from_level, params.from_id.clone());
        let to = LinkTarget::new(to_level, params.to_id.clone());

        // Simple BFS path finding
        let storage = self.links_storage.read().await;
        let _max_depth = params.max_depth.unwrap_or(5);

        // Simplified path finding - would need more sophisticated implementation
        let bi_links = storage.get_bidirectional_links(&from).await?;
        let mut path = Vec::new();

        for link in &bi_links.outgoing {
            if link.target.key() == to.key() {
                path.push(json!({
                    "from": from.key(),
                    "to": to.key(),
                    "link_type": link.link_type.as_str(),
                    "confidence": link.confidence
                }));
                break;
            }
        }

        Ok(json!({
            "from": from.key(),
            "to": to.key(),
            "path": path,
            "found": !path.is_empty(),
            "depth": if path.is_empty() { Value::Null } else { json!(path.len()) }
        }))
    }

    async fn handle_links_get_health(&self, args: Value) -> Result<Value> {
        let _params: Value = args; // No parameters needed

        let storage = self.links_storage.read().await;
        let stats = storage.get_statistics().await?;
        let broken_links = storage.find_broken_links().await?;

        let health_score = if stats.total_links > 0 {
            let broken_ratio = broken_links.len() as f32 / stats.total_links as f32;
            ((1.0 - broken_ratio) * 100.0).max(0.0)
        } else {
            100.0
        };

        Ok(json!({
            "total_links": stats.total_links,
            "broken_links": broken_links.len(),
            "average_confidence": stats.average_confidence,
            "health_score": health_score,
            "by_type": stats.by_type,
            "by_status": stats.by_status
        }))
    }

    async fn handle_links_find_orphans(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            level: Option<String>,
        }

        let params: Params = serde_json::from_value(args).unwrap_or(Params { level: None });

        let storage = self.links_storage.read().await;
        let stats = storage.get_statistics().await?;

        info!("Finding orphan entities with no semantic links (level: {:?})", params.level);

        let mut orphans: Vec<Value> = Vec::new();

        // Parse the level filter if provided
        use crate::links::types::KnowledgeLevel;
        let level_filter = params.level.as_ref().and_then(|l| KnowledgeLevel::from_str(l));

        // Strategy: Find entities that have no incoming or outgoing links
        // We'll check the code indexer for symbols and compare with linked entities

        use crate::links::types::LinkTarget;

        // Get symbols from indexer by querying with semantic search (empty query returns sample)
        let indexer = self.indexer.read().await;

        // Use hybrid search with a wildcard to get symbols
        use crate::types::Query;
        let query = Query {
            text: "*".to_string(),
            symbol_types: None,
            scope: None,
            detail_level: crate::types::DetailLevel::Skeleton,
            max_results: Some(1000),
            max_tokens: None,
            offset: Some(0),
        };

        // Try to get symbols via hybrid search
        let mut checked_symbols = 0;
        if let Ok(search_result) = indexer.hybrid_search(&query).await {
            for symbol in search_result.symbols {
                // Determine the knowledge level for this symbol
                let knowledge_level = if symbol.location.file.contains("/tests/") || symbol.location.file.contains("_test.") {
                    KnowledgeLevel::Tests
                } else if symbol.location.file.contains("/examples/") {
                    KnowledgeLevel::Examples
                } else {
                    KnowledgeLevel::Code
                };

                // Skip if level filter doesn't match
                if let Some(filter) = level_filter {
                    if knowledge_level != filter {
                        continue;
                    }
                }

                // Create LinkTarget for this symbol
                let target = LinkTarget::new(knowledge_level, symbol.name.clone());

                // Check if this entity has any links
                if let Ok(bidirectional_links) = storage.get_bidirectional_links(&target).await {
                    if bidirectional_links.outgoing.is_empty() && bidirectional_links.incoming.is_empty() {
                        orphans.push(json!({
                            "entity_id": symbol.name,
                            "entity_level": knowledge_level.as_str(),
                            "file_path": symbol.location.file,
                            "symbol_kind": format!("{:?}", symbol.kind),
                            "reason": "No semantic links found (neither implements, documents, nor relates to anything)",
                        }));
                    }
                }
                checked_symbols += 1;
            }
        }

        info!("Checked {} symbols for orphan detection", checked_symbols);

        // Also check specs and docs if level filter allows
        if level_filter.is_none() || level_filter == Some(KnowledgeLevel::Spec) || level_filter == Some(KnowledgeLevel::Docs) {
            // Check spec files
            let spec_manager = self.spec_manager.read().await;
            if let Ok(registry) = spec_manager.discover_specs() {
                for spec_info in registry.specs {
                    let spec_level = if spec_info.name.ends_with("-spec") || spec_info.name.contains("specification") {
                        KnowledgeLevel::Spec
                    } else {
                        KnowledgeLevel::Docs
                    };

                    if let Some(filter) = level_filter {
                        if spec_level != filter {
                            continue;
                        }
                    }

                    let target = LinkTarget::new(spec_level, spec_info.name.clone());
                    let bidirectional_links = storage.get_bidirectional_links(&target).await?;

                    if bidirectional_links.outgoing.is_empty() && bidirectional_links.incoming.is_empty() {
                        orphans.push(json!({
                            "entity_id": spec_info.name,
                            "entity_level": spec_level.as_str(),
                            "file_path": spec_info.path.to_string_lossy(),
                            "reason": "No semantic links (not referenced by code or other specs)",
                        }));
                    }
                }
            }
        }

        info!("Found {} orphan entities out of {} total links", orphans.len(), stats.total_links);

        Ok(json!({
            "orphans": orphans,
            "total": orphans.len(),
            "level": params.level,
            "total_links_in_system": stats.total_links,
            "note": "Orphans are entities with no incoming or outgoing semantic links"
        }))
    }

    async fn handle_links_extract_from_file(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            file_path: String,
            method: Option<String>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for links.extract_from_file")?;

        info!("Extracting links from file: {} (method: {:?})", params.file_path, params.method);

        use crate::links::extractor::{CommentExtractor, TreeSitterExtractor, MarkdownExtractor, LinkExtractor};
        use crate::links::types::KnowledgeLevel;
        use std::path::Path;

        let file_path = Path::new(&params.file_path);

        // Check if file exists
        if !file_path.exists() {
            return Err(anyhow!("File not found: {}", params.file_path));
        }

        // Read file content
        let content = tokio::fs::read_to_string(file_path).await
            .context("Failed to read file")?;

        // Determine knowledge level from file path
        let knowledge_level = if file_path.to_string_lossy().contains("/tests/") || file_path.to_string_lossy().contains("_test.") {
            KnowledgeLevel::Tests
        } else if file_path.to_string_lossy().contains("/examples/") {
            KnowledgeLevel::Examples
        } else if file_path.to_string_lossy().contains("/specs/") || file_path.to_string_lossy().contains("/docs/") {
            if file_path.extension().and_then(|e| e.to_str()) == Some("md") {
                if params.file_path.contains("-spec") || params.file_path.contains("specification") {
                    KnowledgeLevel::Spec
                } else {
                    KnowledgeLevel::Docs
                }
            } else {
                KnowledgeLevel::Docs
            }
        } else {
            KnowledgeLevel::Code
        };

        // Determine extraction method
        let method = params.method.as_deref().unwrap_or("auto");

        let links = match method {
            "comment" | "annotation" => {
                // Use comment extractor
                let extractor = CommentExtractor::new()?;
                extractor.extract(file_path, &content, knowledge_level).await?
            }
            "tree-sitter" | "code" => {
                // Use tree-sitter extractor
                let extractor = TreeSitterExtractor::new()?;
                extractor.extract(file_path, &content, knowledge_level).await?
            }
            "markdown" | "md" => {
                // Use markdown extractor
                let extractor = MarkdownExtractor::new()?;
                extractor.extract(file_path, &content, knowledge_level).await?
            }
            "auto" | _ => {
                // Auto-detect based on file extension
                let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

                match extension {
                    "md" | "markdown" => {
                        let extractor = MarkdownExtractor::new()?;
                        extractor.extract(file_path, &content, knowledge_level).await?
                    }
                    "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" => {
                        // Try both comment and tree-sitter extractors
                        let comment_extractor = CommentExtractor::new()?;
                        let mut links = comment_extractor.extract(file_path, &content, knowledge_level).await?;

                        if let Ok(ts_extractor) = TreeSitterExtractor::new() {
                            if let Ok(mut ts_links) = ts_extractor.extract(file_path, &content, knowledge_level).await {
                                links.append(&mut ts_links);
                            }
                        }

                        links
                    }
                    _ => {
                        // Default to comment extractor for unknown types
                        let extractor = CommentExtractor::new()?;
                        extractor.extract(file_path, &content, knowledge_level).await?
                    }
                }
            }
        };

        // Convert links to JSON
        let links_json: Vec<Value> = links.iter().map(|link| {
            json!({
                "link_id": link.id.as_str(),
                "link_type": link.link_type.as_str(),
                "source": {
                    "level": link.source.level.as_str(),
                    "id": link.source.id.clone(),
                },
                "target": {
                    "level": link.target.level.as_str(),
                    "id": link.target.id.clone(),
                },
                "confidence": link.confidence,
                "extraction_method": format!("{:?}", link.extraction_method),
                "context": link.context,
            })
        }).collect();

        info!("Extracted {} links from {}", links.len(), params.file_path);

        // Optionally store the extracted links
        let storage = self.links_storage.write().await;
        let mut stored_count = 0;
        for link in &links {
            if let Ok(()) = storage.add_link(link).await {
                stored_count += 1;
            }
        }

        Ok(json!({
            "file_path": params.file_path,
            "links_found": links.len(),
            "links_stored": stored_count,
            "links": links_json,
            "method": method,
            "knowledge_level": knowledge_level.as_str(),
        }))
    }

    // === Indexer Watch Control Handlers ===

    async fn handle_indexer_enable_watching(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            scope: String,
            debounce_ms: Option<u64>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for indexer.enable_watching")?;

        let delta_indexer = self.delta_indexer.as_ref()
            .ok_or_else(|| anyhow!("Delta indexer not initialized"))?;

        // Determine the path to watch
        let path = if params.scope == "project" {
            // Get project root from config or current directory
            std::env::current_dir()?
        } else {
            PathBuf::from(params.scope)
        };

        // Enable watching
        delta_indexer.enable_watching(&path).await?;

        info!("File watching enabled for: {:?}", path);

        Ok(json!({
            "status": "enabled",
            "path": path.to_string_lossy(),
            "debounce_ms": params.debounce_ms.unwrap_or(50)
        }))
    }

    async fn handle_indexer_disable_watching(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            scope: String,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for indexer.disable_watching")?;

        let delta_indexer = self.delta_indexer.as_ref()
            .ok_or_else(|| anyhow!("Delta indexer not initialized"))?;

        // Determine the path to stop watching
        let path = if params.scope == "project" {
            std::env::current_dir()?
        } else {
            PathBuf::from(params.scope)
        };

        // Disable watching
        delta_indexer.disable_watching(&path).await?;

        info!("File watching disabled for: {:?}", path);

        Ok(json!({
            "status": "disabled",
            "path": path.to_string_lossy()
        }))
    }

    async fn handle_indexer_get_watch_status(&self, _args: Value) -> Result<Value> {
        let delta_indexer = self.delta_indexer.as_ref()
            .ok_or_else(|| anyhow!("Delta indexer not initialized"))?;

        let status = delta_indexer.get_watch_status().await;

        Ok(json!({
            "enabled": status.enabled,
            "watched_paths": status.watched_paths.iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            "pending_changes": status.pending_changes,
            "queue_size": status.queue_size
        }))
    }

    async fn handle_indexer_poll_changes(&self, _args: Value) -> Result<Value> {
        let delta_indexer = self.delta_indexer.as_ref()
            .ok_or_else(|| anyhow!("Delta indexer not initialized"))?;

        let result = delta_indexer.poll_and_apply().await?;

        info!(
            "Polled and applied changes: {} files updated in {}ms",
            result.files_updated, result.duration_ms
        );

        Ok(json!({
            "files_updated": result.files_updated,
            "symbols_updated": result.symbols_updated,
            "symbols_deleted": result.symbols_deleted,
            "duration_ms": result.duration_ms
        }))
    }

    async fn handle_indexer_index_project(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            path: String,
            #[serde(default)]
            force: bool,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for indexer.index_project")?;

        let path = PathBuf::from(params.path);
        if !path.exists() {
            anyhow::bail!("Path does not exist: {:?}", path);
        }

        info!("Starting indexing of project at: {:?}", path);
        let start = std::time::Instant::now();

        // Get indexer and perform indexing
        let mut indexer = self.indexer.write().await;
        indexer.index_project(&path, params.force).await?;

        // NOTE: No need to call load() here - index_project already populates
        // in-memory caches via index_file_incremental(). Calling load() would
        // create duplicates in name_index and other indices.

        let duration_ms = start.elapsed().as_millis() as u64;
        let symbol_count = indexer.symbol_count();

        info!("Indexing completed in {}ms. Total symbols: {}", duration_ms, symbol_count);

        Ok(json!({
            "status": "completed",
            "path": path.to_string_lossy(),
            "symbols_indexed": symbol_count,
            "duration_ms": duration_ms,
            "forced": params.force
        }))
    }

    // === System Health Handler ===

    async fn handle_system_health(&self, _args: Value) -> Result<Value> {
        use sysinfo::System;

        // Calculate uptime
        let uptime_seconds = self.start_time.elapsed().as_secs();

        // Get progress task count
        let progress_manager = self.progress_manager.read().await;
        let progress_tasks_total = progress_manager.count_all().await?;
        let progress_stats = progress_manager.get_progress(None).await?;

        // Get memory statistics (stub)
        let memory_stats = json!({
            "episodes": 0,
            "total_size_mb": 0.0
        });

        // Get metrics count (if available)
        let metrics_info = if let Some(ref collector) = self.metrics_collector {
            let tool_names = collector.get_tool_names();
            let snapshot = collector.take_snapshot();
            let total_tool_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();

            json!({
                "available": true,
                "unique_tools_called": tool_names.len(),
                "total_tool_calls": total_tool_calls,
                "total_input_tokens": snapshot.tokens.total_input_tokens,
                "total_output_tokens": snapshot.tokens.total_output_tokens,
            })
        } else {
            json!({
                "available": false
            })
        };

        // Get system memory usage
        let mut sys = System::new_all();
        sys.refresh_all();

        let process_memory_mb = 0.0; // Stub
        let total_memory_mb = (sys.total_memory() as f64) / 1_048_576.0;
        let used_memory_mb = (sys.used_memory() as f64) / 1_048_576.0;

        // Get indexer stats
        let indexer = self.indexer.read().await;
        let total_symbols = indexer.symbol_count();
        let (l1_size, l2_size) = indexer.cache_sizes();
        let cache_stats = indexer.cache_stats();
        let total_hits = cache_stats.l1_hits + cache_stats.l2_hits + cache_stats.l3_hits;
        let index_stats = json!({
            "total_symbols": total_symbols,
            "total_files": indexer.file_count(),
            "cache": {
                "l1_size": l1_size,
                "l2_size": l2_size,
                "l1_hits": cache_stats.l1_hits,
                "l2_hits": cache_stats.l2_hits,
                "l3_hits": cache_stats.l3_hits,
                "total_hits": total_hits,
                "misses": cache_stats.misses,
                "hit_rate": format!("{:.2}%", cache_stats.hit_rate() * 100.0),
                "avg_latency_ms": format!("{:.2}", cache_stats.avg_latency_ms())
            }
        });

        // Get session count (stub)
        let session_count = 0;

        info!("Health check: uptime={}s, tasks={}, memory={}MB",
              uptime_seconds, progress_tasks_total, process_memory_mb);

        Ok(json!({
            "status": "healthy",
            "uptime_seconds": uptime_seconds,
            "uptime_formatted": format_duration(uptime_seconds),
            "memory": {
                "process_mb": format!("{:.2}", process_memory_mb),
                "system_total_mb": format!("{:.2}", total_memory_mb),
                "system_used_mb": format!("{:.2}", used_memory_mb),
                "system_used_percent": format!("{:.1}", (used_memory_mb / total_memory_mb) * 100.0),
            },
            "metrics": metrics_info,
            "task": {
                "total_tasks": progress_tasks_total,
                "pending": progress_stats.pending,
                "in_progress": progress_stats.in_progress,
                "completed": progress_stats.done,
                "blocked": progress_stats.blocked,
            },
            "memory_system": memory_stats,
            "indexer": index_stats,
            "sessions": {
                "active": session_count,
            },
        }))
    }

    // === Metrics Handlers ===

    async fn handle_metrics_get_summary(&self, _args: Value) -> Result<Value> {
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        // Calculate aggregate statistics
        let total_tool_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
        let total_errors: u64 = snapshot.tools.values().map(|m| m.error_count).sum();
        let total_input_tokens: u64 = snapshot.tools.values().map(|m| m.total_input_tokens).sum();
        let total_output_tokens: u64 = snapshot.tools.values().map(|m| m.total_output_tokens).sum();

        let success_rate = if total_tool_calls > 0 {
            ((total_tool_calls - total_errors) as f64 / total_tool_calls as f64) * 100.0
        } else {
            100.0
        };

        info!("Metrics summary: {} tool calls, {:.2}% success rate",
              total_tool_calls, success_rate);

        Ok(json!({
            "timestamp": snapshot.timestamp.to_rfc3339(),
            "summary": {
                "total_tool_calls": total_tool_calls,
                "total_errors": total_errors,
                "success_rate_percent": format!("{:.2}", success_rate),
                "unique_tools_used": snapshot.tools.len(),
            },
            "tokens": {
                "total_input_tokens": total_input_tokens,
                "total_output_tokens": total_output_tokens,
                "total_tokens": total_input_tokens + total_output_tokens,
                "avg_tokens_per_call": if total_tool_calls > 0 {
                    (total_input_tokens + total_output_tokens) as f64 / total_tool_calls as f64
                } else {
                    0.0
                }
            },
            "memory": snapshot.memory,
            "search": snapshot.search,
            "sessions": snapshot.sessions,
            "system": snapshot.system,
        }))
    }

    async fn handle_metrics_get_tool_stats(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            tool_name: Option<String>,
            top_n: Option<usize>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for metrics.get_tool_stats")?;

        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        if let Some(tool_name) = params.tool_name {
            // Get stats for specific tool
            let metrics = collector.get_tool_metrics(&tool_name)
                .ok_or_else(|| anyhow!("No metrics found for tool: {}", tool_name))?;

            info!("Retrieved metrics for tool: {}", tool_name);

            Ok(json!({
                "tool_name": tool_name,
                "metrics": {
                    "total_calls": metrics.total_calls,
                    "success_count": metrics.success_count,
                    "error_count": metrics.error_count,
                    "success_rate_percent": format!("{:.2}", metrics.success_rate * 100.0),
                    "latency": {
                        "mean_ms": format!("{:.2}", metrics.latency_histogram.mean()),
                        "p50_ms": metrics.latency_histogram.p50(),
                        "p95_ms": metrics.latency_histogram.p95(),
                        "p99_ms": metrics.latency_histogram.p99(),
                    },
                    "tokens": {
                        "total_input_tokens": metrics.total_input_tokens,
                        "total_output_tokens": metrics.total_output_tokens,
                        "avg_input_per_call": if metrics.total_calls > 0 {
                            metrics.total_input_tokens as f64 / metrics.total_calls as f64
                        } else {
                            0.0
                        },
                        "avg_output_per_call": if metrics.total_calls > 0 {
                            metrics.total_output_tokens as f64 / metrics.total_calls as f64
                        } else {
                            0.0
                        }
                    },
                    "errors": metrics.error_breakdown,
                    "last_24h_calls": metrics.last_24h_calls,
                }
            }))
        } else {
            // Get stats for all tools, optionally limited to top N
            let snapshot = collector.take_snapshot();
            let mut tool_stats: Vec<_> = snapshot.tools.into_iter().collect();

            // Sort by total calls descending
            tool_stats.sort_by(|a, b| b.1.total_calls.cmp(&a.1.total_calls));

            // Limit to top N if specified
            if let Some(n) = params.top_n {
                tool_stats.truncate(n);
            }

            let stats: Vec<Value> = tool_stats.into_iter().map(|(name, metrics)| {
                json!({
                    "tool_name": name,
                    "total_calls": metrics.total_calls,
                    "success_rate_percent": format!("{:.2}", metrics.success_rate * 100.0),
                    "mean_latency_ms": format!("{:.2}", metrics.latency_histogram.mean()),
                    "p95_latency_ms": metrics.latency_histogram.p95(),
                    "total_tokens": metrics.total_input_tokens + metrics.total_output_tokens,
                })
            }).collect();

            info!("Retrieved metrics for {} tools", stats.len());

            Ok(json!({
                "tools": stats
            }))
        }
    }

    async fn handle_metrics_get_time_range(&self, _args: Value) -> Result<Value> {
        // Note: This feature requires metrics storage which is not yet integrated into ToolHandlers
        // For now, return current snapshot only
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        info!("Retrieved current metric snapshot (historical storage not yet available)");

        let total_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
        let total_tokens: u64 = snapshot.tools.values()
            .map(|m| m.total_input_tokens + m.total_output_tokens).sum();

        Ok(json!({
            "note": "Historical metrics storage not yet integrated. Showing current snapshot only.",
            "timestamp": snapshot.timestamp.to_rfc3339(),
            "snapshot": {
                "total_tool_calls": total_calls,
                "total_tokens": total_tokens,
                "unique_tools": snapshot.tools.len(),
            }
        }))
    }

    async fn handle_metrics_list_slow_tools(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct Params {
            threshold_ms: Option<f64>,
            min_calls: Option<u64>,
        }

        let params: Params = serde_json::from_value(args)
            .context("Invalid parameters for metrics.list_slow_tools")?;

        let threshold_ms = params.threshold_ms.unwrap_or(500.0);
        let min_calls = params.min_calls.unwrap_or(1);

        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        let mut slow_tools: Vec<_> = snapshot.tools.into_iter()
            .filter(|(_, metrics)| {
                metrics.total_calls >= min_calls && metrics.latency_histogram.p95() > threshold_ms
            })
            .map(|(name, metrics)| {
                (name, metrics.latency_histogram.p95(), metrics)
            })
            .collect();

        // Sort by p95 latency descending
        slow_tools.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let slow_tool_list: Vec<Value> = slow_tools.into_iter().map(|(name, p95, metrics)| {
            json!({
                "tool_name": name,
                "p95_latency_ms": p95,
                "p99_latency_ms": metrics.latency_histogram.p99(),
                "mean_latency_ms": format!("{:.2}", metrics.latency_histogram.mean()),
                "total_calls": metrics.total_calls,
                "success_rate_percent": format!("{:.2}", metrics.success_rate * 100.0),
            })
        }).collect();

        info!("Found {} slow tools (p95 > {}ms, min {} calls)",
              slow_tool_list.len(), threshold_ms, min_calls);

        Ok(json!({
            "threshold_ms": threshold_ms,
            "min_calls": min_calls,
            "slow_tools": slow_tool_list
        }))
    }

    async fn handle_metrics_get_token_efficiency(&self, _args: Value) -> Result<Value> {
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        // Calculate token efficiency per tool
        let mut tool_efficiency: Vec<_> = snapshot.tools.iter()
            .map(|(name, metrics)| {
                let total_tokens = metrics.total_input_tokens + metrics.total_output_tokens;
                let tokens_per_call = if metrics.total_calls > 0 {
                    total_tokens as f64 / metrics.total_calls as f64
                } else {
                    0.0
                };
                let tokens_per_ms = if metrics.latency_histogram.mean() > 0.0 {
                    total_tokens as f64 / (metrics.latency_histogram.mean() * metrics.total_calls as f64)
                } else {
                    0.0
                };

                (name.clone(), total_tokens, tokens_per_call, tokens_per_ms, metrics.total_calls)
            })
            .collect();

        // Sort by total tokens descending
        tool_efficiency.sort_by(|a, b| b.1.cmp(&a.1));

        let efficiency_list: Vec<Value> = tool_efficiency.into_iter().map(|(name, total, per_call, per_ms, calls)| {
            json!({
                "tool_name": name,
                "total_tokens": total,
                "tokens_per_call": format!("{:.2}", per_call),
                "tokens_per_ms": format!("{:.4}", per_ms),
                "total_calls": calls,
            })
        }).collect();

        let total_tokens = snapshot.tokens.total_input_tokens + snapshot.tokens.total_output_tokens;

        info!("Token efficiency analysis: {} total tokens across {} tools",
              total_tokens, efficiency_list.len());

        Ok(json!({
            "summary": {
                "total_input_tokens": snapshot.tokens.total_input_tokens,
                "total_output_tokens": snapshot.tokens.total_output_tokens,
                "total_tokens": total_tokens,
                "avg_compression_ratio": snapshot.tokens.avg_compression_ratio,
                "tokens_saved_compression": snapshot.tokens.tokens_saved_compression,
                "tokens_saved_deduplication": snapshot.tokens.tokens_saved_deduplication,
            },
            "by_tool": efficiency_list
        }))
    }

    async fn handle_metrics_export_prometheus(&self, _args: Value) -> Result<Value> {
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();
        let mut prometheus_text = String::new();

        // Export tool metrics
        for (tool_name, metrics) in &snapshot.tools {
            let _safe_name = tool_name.replace(".", "_");

            prometheus_text.push_str(&format!(
                "# HELP meridian_tool_calls_total Total number of calls to {}\n", tool_name
            ));
            prometheus_text.push_str(&"# TYPE meridian_tool_calls_total counter\n".to_string());
            prometheus_text.push_str(&format!(
                "meridian_tool_calls_total{{tool=\"{}\"}} {}\n", tool_name, metrics.total_calls
            ));

            prometheus_text.push_str(&format!(
                "# HELP meridian_tool_errors_total Total number of errors for {}\n", tool_name
            ));
            prometheus_text.push_str(&"# TYPE meridian_tool_errors_total counter\n".to_string());
            prometheus_text.push_str(&format!(
                "meridian_tool_errors_total{{tool=\"{}\"}} {}\n", tool_name, metrics.error_count
            ));

            prometheus_text.push_str(&format!(
                "# HELP meridian_tool_latency_seconds Latency histogram for {}\n", tool_name
            ));
            prometheus_text.push_str(&"# TYPE meridian_tool_latency_seconds histogram\n".to_string());

            let histogram = &metrics.latency_histogram;
            let mut cumulative = 0u64;
            for (i, &bucket) in histogram.buckets.iter().enumerate() {
                cumulative += histogram.counts[i];
                prometheus_text.push_str(&format!(
                    "meridian_tool_latency_seconds_bucket{{tool=\"{}\",le=\"{}\"}} {}\n",
                    tool_name, bucket / 1000.0, cumulative
                ));
            }
            prometheus_text.push_str(&format!(
                "meridian_tool_latency_seconds_bucket{{tool=\"{}\",le=\"+Inf\"}} {}\n",
                tool_name, histogram.count
            ));
            prometheus_text.push_str(&format!(
                "meridian_tool_latency_seconds_sum{{tool=\"{}\"}} {}\n",
                tool_name, histogram.sum / 1000.0
            ));
            prometheus_text.push_str(&format!(
                "meridian_tool_latency_seconds_count{{tool=\"{}\"}} {}\n",
                tool_name, histogram.count
            ));

            prometheus_text.push_str(&format!(
                "# HELP meridian_tool_tokens_total Total tokens used by {}\n", tool_name
            ));
            prometheus_text.push_str(&"# TYPE meridian_tool_tokens_total counter\n".to_string());
            prometheus_text.push_str(&format!(
                "meridian_tool_tokens_total{{tool=\"{}\",direction=\"input\"}} {}\n",
                tool_name, metrics.total_input_tokens
            ));
            prometheus_text.push_str(&format!(
                "meridian_tool_tokens_total{{tool=\"{}\",direction=\"output\"}} {}\n",
                tool_name, metrics.total_output_tokens
            ));
        }

        info!("Exported Prometheus metrics for {} tools ({} bytes)",
              snapshot.tools.len(), prometheus_text.len());

        Ok(json!({
            "format": "prometheus",
            "timestamp": snapshot.timestamp.to_rfc3339(),
            "metrics": prometheus_text,
            "tool_count": snapshot.tools.len(),
        }))
    }

    async fn handle_metrics_analyze_trends(&self, _args: Value) -> Result<Value> {
        // Note: This feature requires metrics storage which is not yet integrated into ToolHandlers
        // For now, return current snapshot only
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        info!("Trend analysis not yet available (requires historical storage). Showing current stats.");

        let total_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
        let total_tokens: u64 = snapshot.tools.values()
            .map(|m| m.total_input_tokens + m.total_output_tokens).sum();

        Ok(json!({
            "note": "Historical metrics storage not yet integrated. Trend analysis not available.",
            "current_snapshot": {
                "timestamp": snapshot.timestamp.to_rfc3339(),
                "total_calls": total_calls,
                "total_tokens": total_tokens,
                "unique_tools": snapshot.tools.len(),
            }
        }))
    }

    async fn handle_metrics_get_health(&self, _args: Value) -> Result<Value> {
        let collector = self.metrics_collector.as_ref()
            .ok_or_else(|| anyhow!("Metrics collector not initialized"))?;

        let snapshot = collector.take_snapshot();

        // Calculate health indicators
        let total_calls: u64 = snapshot.tools.values().map(|m| m.total_calls).sum();
        let total_errors: u64 = snapshot.tools.values().map(|m| m.error_count).sum();
        let overall_success_rate = if total_calls > 0 {
            ((total_calls - total_errors) as f64 / total_calls as f64) * 100.0
        } else {
            100.0
        };

        // Find tools with high error rates
        let error_tools: Vec<_> = snapshot.tools.iter()
            .filter(|(_, m)| m.total_calls > 10 && m.success_rate < 0.95)
            .map(|(name, m)| {
                json!({
                    "tool_name": name,
                    "success_rate_percent": format!("{:.2}", m.success_rate * 100.0),
                    "error_count": m.error_count,
                    "total_calls": m.total_calls,
                })
            })
            .collect();

        // Find tools with high latency
        let slow_tools: Vec<_> = snapshot.tools.iter()
            .filter(|(_, m)| m.total_calls > 5 && m.latency_histogram.p95() > 1000.0)
            .map(|(name, m)| {
                json!({
                    "tool_name": name,
                    "p95_latency_ms": m.latency_histogram.p95(),
                    "mean_latency_ms": format!("{:.2}", m.latency_histogram.mean()),
                })
            })
            .collect();

        let health_status = if overall_success_rate >= 99.0 && slow_tools.is_empty() {
            "healthy"
        } else if overall_success_rate >= 95.0 {
            "degraded"
        } else {
            "unhealthy"
        };

        info!("Metrics health check: status={}, success_rate={:.2}%",
              health_status, overall_success_rate);

        Ok(json!({
            "status": health_status,
            "timestamp": snapshot.timestamp.to_rfc3339(),
            "metrics": {
                "total_tool_calls": total_calls,
                "total_errors": total_errors,
                "overall_success_rate_percent": format!("{:.2}", overall_success_rate),
                "unique_tools": snapshot.tools.len(),
            },
            "storage": {
                "note": "Metrics storage not yet integrated into ToolHandlers",
                "in_memory_only": true,
            },
            "issues": {
                "high_error_rate_tools": error_tools,
                "high_latency_tools": slow_tools,
            }
        }))
    }

    // === Backup Management Handlers ===

    async fn handle_backup_create(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupCreateParams {
            description: Option<String>,
            tags: Option<Vec<String>>,
        }

        let params: BackupCreateParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.create")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let metadata = backup_manager.write().await.create_backup(
            crate::storage::BackupType::Manual,
            params.description,
            params.tags.unwrap_or_default(),
        ).await?;

        info!("Manual backup created: {}", metadata.id);

        Ok(json!({
            "backup_id": metadata.id,
            "created_at": metadata.created_at.to_rfc3339(),
            "size_bytes": metadata.size_bytes,
            "file_count": metadata.file_count,
            "verified": metadata.verified
        }))
    }

    async fn handle_backup_list(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupListParams {
            backup_type: Option<String>,
            verified_only: Option<bool>,
        }

        let params: BackupListParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.list")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let mut backups = backup_manager.read().await.list_backups().await?;

        // Filter by type if specified
        if let Some(backup_type_str) = params.backup_type {
            let filter_type = match backup_type_str.as_str() {
                "manual" => crate::storage::BackupType::Manual,
                "scheduled" => crate::storage::BackupType::Scheduled,
                "pre_migration" => crate::storage::BackupType::PreMigration,
                "incremental" => crate::storage::BackupType::Incremental,
                _ => return Err(anyhow!("Invalid backup type")),
            };
            backups.retain(|b| b.backup_type == filter_type);
        }

        // Filter by verification status if specified
        if params.verified_only.unwrap_or(false) {
            backups.retain(|b| b.verified);
        }

        let total_count = backups.len();

        let backup_list: Vec<Value> = backups.iter().map(|b| {
            json!({
                "id": b.id,
                "backup_type": b.backup_type.as_str(),
                "created_at": b.created_at.to_rfc3339(),
                "size_bytes": b.size_bytes,
                "file_count": b.file_count,
                "verified": b.verified,
                "description": b.description
            })
        }).collect();

        Ok(json!({
            "backups": backup_list,
            "total_count": total_count
        }))
    }

    async fn handle_backup_restore(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupRestoreParams {
            backup_id: String,
            target_path: Option<String>,
        }

        let params: BackupRestoreParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.restore")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let target_path = params.target_path.map(PathBuf::from);

        info!("Restoring backup: {}", params.backup_id);

        backup_manager.write().await.restore_backup(&params.backup_id, target_path).await?;

        info!("Backup restored successfully: {}", params.backup_id);

        Ok(json!({
            "success": true,
            "restored_from": params.backup_id
        }))
    }

    async fn handle_backup_verify(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupVerifyParams {
            backup_id: String,
        }

        let params: BackupVerifyParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.verify")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        info!("Verifying backup: {}", params.backup_id);

        backup_manager.write().await.verify_backup(&params.backup_id).await?;

        let metadata = backup_manager.read().await.get_backup(&params.backup_id).await?;

        Ok(json!({
            "verified": metadata.verified,
            "verified_at": metadata.verified_at.map(|t| t.to_rfc3339()),
            "checksum_valid": true
        }))
    }

    async fn handle_backup_delete(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupDeleteParams {
            backup_id: String,
        }

        let params: BackupDeleteParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.delete")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        info!("Deleting backup: {}", params.backup_id);

        backup_manager.write().await.delete_backup(&params.backup_id).await?;

        Ok(json!({
            "success": true,
            "deleted_backup_id": params.backup_id
        }))
    }

    async fn handle_backup_get_stats(&self, _args: Value) -> Result<Value> {
        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let stats = backup_manager.read().await.get_stats().await?;

        Ok(json!({
            "total_backups": stats.total_backups,
            "total_size_bytes": stats.total_size_bytes,
            "by_type": stats.by_type,
            "oldest_backup": stats.oldest_backup.map(|t| t.to_rfc3339()),
            "newest_backup": stats.newest_backup.map(|t| t.to_rfc3339()),
            "verified_count": stats.verified_count,
            "unverified_count": stats.unverified_count
        }))
    }

    async fn handle_backup_create_scheduled(&self, _args: Value) -> Result<Value> {
        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let metadata = backup_manager.write().await.create_scheduled_backup().await?;

        info!("Scheduled backup created: {}", metadata.id);

        Ok(json!({
            "backup_id": metadata.id,
            "created_at": metadata.created_at.to_rfc3339()
        }))
    }

    async fn handle_backup_create_pre_migration(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct BackupPreMigrationParams {
            schema_version: u32,
            description: Option<String>,
        }

        let params: BackupPreMigrationParams = serde_json::from_value(args)
            .context("Invalid parameters for backup.create_pre_migration")?;

        let backup_manager = self.backup_manager.as_ref()
            .ok_or_else(|| anyhow!("Backup manager not initialized"))?;

        let metadata = backup_manager.write().await.create_pre_migration_backup(
            params.schema_version,
            params.description,
        ).await?;

        info!("Pre-migration backup created: {} for schema version {}", metadata.id, params.schema_version);

        Ok(json!({
            "backup_id": metadata.id,
            "schema_version": params.schema_version
        }))
    }

    // === Graph Analysis Handlers ===

    async fn handle_graph_find_dependencies(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindDependenciesParams {
            symbol_id: String,
            #[serde(default = "default_depth")]
            depth: u32,
        }
        fn default_depth() -> u32 { 3 }

        let params: FindDependenciesParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.find_dependencies")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let graph = analyzer.find_dependencies(&params.symbol_id, params.depth).await?;

        Ok(json!({
            "root": graph.root,
            "nodes": graph.nodes,
            "edges": graph.edges,
            "total_dependencies": graph.count()
        }))
    }

    async fn handle_graph_find_dependents(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindDependentsParams {
            symbol_id: String,
        }

        let params: FindDependentsParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.find_dependents")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let dependents = analyzer.find_dependents(&params.symbol_id).await?;

        Ok(json!({
            "symbol_id": params.symbol_id,
            "dependents": dependents,
            "count": dependents.len()
        }))
    }

    async fn handle_graph_semantic_search(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct SemanticSearchParams {
            query: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize { 10 }

        let params: SemanticSearchParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.semantic_search")?;

        let _analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        // Semantic search requires embedder which may not be configured
        // For now, return a helpful message
        Err(anyhow!("Semantic search requires embedder configuration (not yet implemented)"))
    }

    async fn handle_graph_find_similar_patterns(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindSimilarPatternsParams {
            symbol_id: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize { 20 }

        let params: FindSimilarPatternsParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.find_similar_patterns")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let patterns = analyzer.find_similar_patterns(&params.symbol_id, params.limit).await?;

        Ok(json!({
            "symbol_id": params.symbol_id,
            "patterns": patterns,
            "count": patterns.len()
        }))
    }

    async fn handle_graph_impact_analysis(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct ImpactAnalysisParams {
            changed_symbols: Vec<String>,
        }

        let params: ImpactAnalysisParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.impact_analysis")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let report = analyzer.impact_analysis(params.changed_symbols).await?;

        Ok(json!({
            "changed_symbols": report.changed_symbols,
            "affected_symbols": report.affected_symbols,
            "affected_files": report.affected_files,
            "total_affected": report.total_affected,
            "impact_depth": report.impact_depth
        }))
    }

    async fn handle_graph_code_lineage(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct CodeLineageParams {
            symbol_id: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize { 10 }

        let params: CodeLineageParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.code_lineage")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let lineage = analyzer.find_code_lineage(&params.symbol_id, params.limit).await?;

        Ok(json!({
            "symbol_id": params.symbol_id,
            "lineage": lineage,
            "count": lineage.len()
        }))
    }

    async fn handle_graph_get_call_graph(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetCallGraphParams {
            symbol_id: String,
        }

        let params: GetCallGraphParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.get_call_graph")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let calls = analyzer.get_call_graph(&params.symbol_id).await?;

        Ok(json!({
            "symbol_id": params.symbol_id,
            "calls": calls,
            "count": calls.len()
        }))
    }

    async fn handle_graph_get_callers(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetCallersParams {
            symbol_id: String,
        }

        let params: GetCallersParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.get_callers")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let callers = analyzer.get_callers(&params.symbol_id).await?;

        Ok(json!({
            "symbol_id": params.symbol_id,
            "callers": callers,
            "count": callers.len()
        }))
    }

    async fn handle_graph_get_stats(&self, _args: Value) -> Result<Value> {
        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let stats = analyzer.get_graph_stats().await?;

        Ok(json!({
            "total_symbols": stats.total_symbols,
            "total_dependencies": stats.total_dependencies,
            "avg_out_degree": stats.avg_out_degree,
            "avg_in_degree": stats.avg_in_degree
        }))
    }

    async fn handle_graph_find_hubs(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct FindHubsParams {
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize { 20 }

        let params: FindHubsParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.find_hubs")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let hubs = analyzer.find_hubs(params.limit).await?;

        Ok(json!({
            "hubs": hubs,
            "count": hubs.len()
        }))
    }

    async fn handle_graph_find_circular_dependencies(&self, _args: Value) -> Result<Value> {
        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let cycles = analyzer.find_circular_dependencies().await?;

        Ok(json!({
            "circular_dependencies": cycles,
            "count": cycles.len()
        }))
    }

    async fn handle_graph_get_symbol_full(&self, args: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GetSymbolFullParams {
            symbol_id: String,
        }

        let params: GetSymbolFullParams = serde_json::from_value(args)
            .context("Invalid parameters for graph.get_symbol_full")?;

        let analyzer = self.graph_analyzer.as_ref()
            .ok_or_else(|| anyhow!("Graph analyzer not initialized"))?;

        let symbol_full = analyzer.get_symbol_full(&params.symbol_id).await?;

        Ok(json!({
            "symbol": symbol_full.symbol,
            "dependencies": symbol_full.dependencies,
            "dependents": symbol_full.dependents,
            "calls": symbol_full.calls,
            "called_by": symbol_full.called_by
        }))
    }
}


// ==================== Pattern Search Helper Functions ====================

/// Configuration for pattern search operation
struct PatternSearchConfig {
    scope_path: PathBuf,
    page_size: usize,
    offset: usize,
    target_matches: usize,
}

/// Result of pattern search execution
struct PatternSearchResult {
    matches: Vec<crate::indexer::PatternMatch>,
    searched_all_files: bool,
}

/// Paginated pattern search results
struct PaginatedPatternResults {
    matches: Vec<crate::indexer::PatternMatch>,
    total_found: usize,
    has_more: bool,
    searched_all_files: bool,
}

impl ToolHandlers {
    /// Build search configuration from parameters
    fn build_pattern_search_config<T>(params: &T) -> PatternSearchConfig 
    where
        T: serde::de::DeserializeOwned + serde::Serialize,
    {
        const MAX_RESULTS_HARD_LIMIT: usize = 1000;
        const DEFAULT_PAGE_SIZE: usize = 100;

        // Extract parameters via JSON round-trip (simple but works)
        let json = serde_json::to_value(params).unwrap();
        
        let page_size = json.get("page_size")
            .and_then(|v| v.as_u64().map(|n| n as usize))
            .or_else(|| json.get("max_results").and_then(|v| v.as_u64().map(|n| n as usize)))
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .min(MAX_RESULTS_HARD_LIMIT);
            
        let offset = json.get("offset")
            .and_then(|v| v.as_u64().map(|n| n as usize))
            .unwrap_or(0);
            
        let scope_path = json.get("scope")
            .and_then(|v| v.as_str())
            .map(|s| std::path::Path::new(s).to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        PatternSearchConfig {
            scope_path,
            page_size,
            offset,
            target_matches: offset.saturating_add(page_size),
        }
    }

    /// Collect files to search based on scope and language filter
    fn collect_pattern_search_files(
        scope_path: &PathBuf,
        language: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        const MAX_FILES: usize = 10_000;
        let mut files_to_search = Vec::new();

        if scope_path.is_file() {
            files_to_search.push(scope_path.clone());
        } else if scope_path.is_dir() {
            Self::walk_directory_for_patterns(scope_path, &mut files_to_search, language)?;
        }

        // Limit number of files to search for performance
        if files_to_search.len() > MAX_FILES {
            files_to_search.truncate(MAX_FILES);
        }

        Ok(files_to_search)
    }

    /// Recursively walk directory and collect files matching language filter
    fn walk_directory_for_patterns(
        dir: &std::path::Path,
        files: &mut Vec<PathBuf>,
        lang: Option<&str>,
    ) -> Result<()> {
        use std::fs;
        
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip hidden directories and common ignore patterns
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "node_modules" || name == "target" || name == "dist" {
                    continue;
                }
            }

            if path.is_dir() {
                Self::walk_directory_for_patterns(&path, files, lang)?;
            } else if path.is_file() {
                // Filter by language if specified
                if let Some(lang_filter) = lang {
                    if let Ok(detected) = crate::indexer::PatternSearchEngine::detect_language(&path) {
                        if detected == lang_filter {
                            files.push(path);
                        }
                    }
                } else {
                    // Try to detect language, skip if unsupported
                    if crate::indexer::PatternSearchEngine::detect_language(&path).is_ok() {
                        files.push(path);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Execute pattern searches across files with early-exit optimization
    fn execute_pattern_search(
        &self,
        files: &[PathBuf],
        pattern: &str,
        language: Option<&str>,
        target_matches: usize,
    ) -> Result<PatternSearchResult> {
        let mut all_matches = Vec::new();
        let mut searched_all_files = true;

        for file_path in files {
            // Early exit optimization: stop when we have enough matches
            if all_matches.len() >= target_matches {
                searched_all_files = false;
                break;
            }

            // Read file content
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Detect language
            let file_language = match language.map(|s| s.to_string()).or_else(|| {
                crate::indexer::PatternSearchEngine::detect_language(file_path)
                    .ok()
                    .map(|s| s.to_string())
            }) {
                Some(lang) => lang,
                None => continue,
            };

            // Search for pattern
            match self.pattern_engine.search_in_file(pattern, &file_language, &content, file_path) {
                Ok(matches) => {
                    all_matches.extend(matches);
                }
                Err(e) => {
                    debug!("Pattern search failed for {}: {}", file_path.display(), e);
                    continue;
                }
            }
        }

        Ok(PatternSearchResult {
            matches: all_matches,
            searched_all_files,
        })
    }

    /// Apply pagination to search results
    fn apply_pattern_pagination(
        all_matches: Vec<crate::indexer::PatternMatch>,
        searched_all_files: bool,
        offset: usize,
        page_size: usize,
    ) -> PaginatedPatternResults {
        let total_found = all_matches.len();
        
        let paginated_matches: Vec<_> = all_matches
            .into_iter()
            .skip(offset)
            .take(page_size)
            .collect();

        // Calculate has_more accurately:
        // - If we searched all files, has_more = total > offset + page_size
        // - If we stopped early, has_more = true (there might be more)
        let has_more = if searched_all_files {
            total_found > offset + paginated_matches.len()
        } else {
            // We stopped early, so there are definitely more matches possible
            true
        };

        PaginatedPatternResults {
            matches: paginated_matches,
            total_found,
            has_more,
            searched_all_files,
        }
    }

    /// Build JSON response for pattern search
    fn build_pattern_search_response(
        paginated: PaginatedPatternResults,
        files_searched: usize,
        pattern: &str,
        language: Option<String>,
        offset: usize,
        cache_stats: (usize, usize),
    ) -> Value {
        let matches_json: Vec<Value> = paginated
            .matches
            .iter()
            .map(|m| {
                json!({
                    "location": {
                        "file": m.location.file,
                        "line_start": m.location.line_start,
                        "line_end": m.location.line_end,
                        "column_start": m.location.column_start,
                        "column_end": m.location.column_end
                    },
                    "matched_text": m.matched_text,
                    "captures": m.captures,
                    "score": m.score
                })
            })
            .collect();

        let (cache_used, cache_capacity) = cache_stats;

        json!({
            "matches": matches_json,
            "pagination": {
                "offset": offset,
                "page_size": paginated.matches.len(),
                "has_more": paginated.has_more,
                "total_found": paginated.total_found,
                "searched_all_files": paginated.searched_all_files
            },
            "summary": {
                "files_searched": files_searched,
                "pattern": pattern,
                "language": language
            },
            "cache_stats": {
                "used": cache_used,
                "capacity": cache_capacity
            }
        })
    }
}


// Helper functions for attention.retrieve handler

/// Represents a symbol with its attention weight and focus status
struct SymbolWithWeight {
    symbol: CodeSymbol,
    weight: f32,
    is_focused: bool,
}

/// Represents attention categorization for a symbol
struct AttentionCategory {
    high_attention: Vec<Value>,
    medium_attention: Vec<Value>,
    context_symbols: Vec<Value>,
    total_tokens: usize,
}

/// Extract focused symbol names from attention pattern
fn extract_focused_symbols(attention_pattern: &Value) -> Vec<String> {
    attention_pattern
        .get("focused_symbols")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Load active symbols from memory with their attention weights
async fn load_symbols_with_weights(
    memory: &MemorySystem,
    indexer: &CodeIndexer,
    focused_symbols: &[String],
) -> Result<Vec<SymbolWithWeight>> {
    use crate::indexer::Indexer;
    let active_ids = memory.working.active_symbols().clone();
    let mut symbols_with_weights = Vec::new();

    for symbol_id in active_ids.iter() {
        if let Ok(Some(symbol)) = indexer.get_symbol(&symbol_id.0).await {
            let weight = memory.working.get_attention_weight(symbol_id).unwrap_or(1.0);
            let is_focused = focused_symbols.contains(&symbol.name);
            symbols_with_weights.push(SymbolWithWeight {
                symbol,
                weight,
                is_focused,
            });
        }
    }

    Ok(symbols_with_weights)
}

/// Apply attention boosts and sort by weight
fn apply_attention_boost(symbols: &mut [SymbolWithWeight]) {
    // Boost weights for currently focused symbols
    for item in symbols.iter_mut() {
        if item.is_focused {
            item.weight *= 1.5;
        }
    }

    // Sort by weight (descending)
    symbols.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Categorize symbols by attention level within token budget
fn categorize_symbols_by_attention(
    symbols: &[SymbolWithWeight],
    main_budget: usize,
) -> AttentionCategory {
    let mut high_attention = Vec::new();
    let mut medium_attention = Vec::new();
    let mut context_symbols = Vec::new();
    let mut total_tokens = 0usize;

    for item in symbols {
        let token_cost = item.symbol.metadata.token_cost.0 as usize;
        if total_tokens + token_cost > main_budget {
            break;
        }

        let symbol_json = json!({
            "id": item.symbol.id.0,
            "name": item.symbol.name,
            "kind": item.symbol.kind.as_str(),
            "weight": item.weight,
            "token_cost": token_cost,
            "location": {
                "file": item.symbol.location.file,
                "line_start": item.symbol.location.line_start
            }
        });

        // Categorize based on attention weight thresholds
        if item.weight > 1.5 {
            high_attention.push(symbol_json);
        } else if item.weight > 0.8 {
            medium_attention.push(symbol_json);
        } else {
            context_symbols.push(symbol_json);
        }

        total_tokens += token_cost;
    }

    AttentionCategory {
        high_attention,
        medium_attention,
        context_symbols,
        total_tokens,
    }
}

/// Prefetch related symbols for high-attention symbols
async fn prefetch_related_symbols(
    symbols: &[SymbolWithWeight],
    memory: &MemorySystem,
    indexer: &CodeIndexer,
    active_ids: &std::collections::BTreeSet<crate::types::SymbolId>,
    prefetch_budget: usize,
) -> Result<(Vec<Value>, usize)> {
    let mut prefetched_symbols = Vec::new();
    let mut prefetch_tokens = 0usize;
    let mut prefetched_ids = std::collections::HashSet::new();

    // Only prefetch for high-attention symbols (weight > 1.5)
    for item in symbols.iter().filter(|s| s.weight > 1.5) {
        if prefetch_tokens >= prefetch_budget {
            break;
        }

        // Get related symbols using semantic memory
        let related = memory.semantic.find_related_symbols(&item.symbol.id);

        for rel in related.iter().take(3) {
            // Skip if already in working memory or already prefetched
            if active_ids.contains(&rel.to) || prefetched_ids.contains(&rel.to.0) {
                continue;
            }

            if let Ok(Some(related_symbol)) = indexer.get_symbol(&rel.to.0).await {
                let token_cost = related_symbol.metadata.token_cost.0 as usize;
                if prefetch_tokens + token_cost <= prefetch_budget {
                    prefetched_symbols.push(json!({
                        "id": related_symbol.id.0,
                        "name": related_symbol.name,
                        "kind": related_symbol.kind.as_str(),
                        "relationship": format!("{:?}", rel.relationship_type),
                        "strength": rel.strength,
                        "token_cost": token_cost
                    }));

                    prefetched_ids.insert(rel.to.0.clone());
                    prefetch_tokens += token_cost;
                }
            }
        }
    }

    Ok((prefetched_symbols, prefetch_tokens))
}

/// Build the final attention retrieve response
fn build_attention_response(
    category: AttentionCategory,
    prefetched_symbols: Vec<Value>,
    prefetch_tokens: usize,
    recently_evicted: Vec<String>,
    token_budget: usize,
) -> Value {
    let total_tokens = category.total_tokens + prefetch_tokens;

    json!({
        "high_attention": category.high_attention,
        "medium_attention": category.medium_attention,
        "context_symbols": category.context_symbols,
        "prefetched_symbols": prefetched_symbols,
        "total_tokens": total_tokens,
        "budget_utilization": (total_tokens as f32 / token_budget as f32),
        "recently_evicted": recently_evicted
    })
}


/// Format duration in human-readable format
fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}
