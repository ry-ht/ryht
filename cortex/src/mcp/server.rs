use super::handlers::ToolHandlers;
use super::tools::{get_all_resources, get_all_tools, ServerCapabilities};
use super::transport::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, StdioTransport};
use super::global_client::GlobalServerClient;
use crate::config::Config;
use crate::context::ContextManager;
use crate::indexer::{CodeIndexer, DeltaIndexer, Indexer};
use crate::indexer::watcher::WatcherConfig;
use crate::links::{LinksStorage, RocksDBLinksStorage};
use crate::memory::MemorySystem;
use crate::metrics::{MetricsCollector, MetricsStorage};
use crate::tasks::{TaskManager, TaskStorage};
use crate::project::ProjectManager;
use crate::global::registry::MonorepoContext as ProjectMonorepoContext;
use crate::session::SessionManager;
use crate::specs::SpecificationManager;
use crate::storage::create_default_storage;
use crate::types::{LLMAdapter, Query};
use crate::IndexStats;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Server mode - single project or multi-project
enum ServerMode {
    /// Single project mode (stdio, socket) - legacy mode
    SingleProject {
        memory_system: Arc<tokio::sync::RwLock<MemorySystem>>,
        context_manager: Arc<tokio::sync::RwLock<ContextManager>>,
        indexer: Arc<tokio::sync::RwLock<CodeIndexer>>,
        delta_indexer: Option<Arc<DeltaIndexer>>,
        session_manager: Arc<SessionManager>,
        doc_indexer: Arc<crate::docs::DocIndexer>,
        spec_manager: Arc<tokio::sync::RwLock<SpecificationManager>>,
        progress_manager: Arc<tokio::sync::RwLock<TaskManager>>,
        links_storage: Arc<tokio::sync::RwLock<dyn LinksStorage>>,
        handlers: Option<Arc<ToolHandlers>>,
        // Global architecture support
        global_client: Option<Arc<GlobalServerClient>>,
        global_registry: Option<Arc<crate::global::registry::ProjectRegistryManager>>,
        #[allow(dead_code)]
        monorepo_context: Option<ProjectMonorepoContext>,
        offline_mode: Arc<AtomicBool>,
        // Metrics collection
        metrics_collector: Arc<MetricsCollector>,
        metrics_storage: Arc<MetricsStorage>,
        snapshot_task: Option<JoinHandle<()>>,
    },
    /// Multi-project mode (HTTP)
    MultiProject {
        project_manager: Arc<ProjectManager>,
    },
}

/// Main Meridian MCP server
pub struct MeridianServer {
    mode: ServerMode,
    config: Config,
}

impl MeridianServer {
    /// Create a new Meridian server instance in legacy mode (single-project, no global server)
    pub async fn new_legacy(config: Config) -> Result<Self> {
        Self::new_internal(config, None).await
    }

    /// Create a new Meridian server instance with global server support
    pub async fn new_global(config: Config, global_url: String, _project_path: PathBuf) -> Result<Self> {
        info!("Initializing Meridian server in global mode");

        // Create global server client
        let global_client = Arc::new(GlobalServerClient::new(global_url)?);

        // Check if global server is available
        let available = global_client.is_available().await;
        if !available {
            warn!("Global server not available, will start in offline mode");
        }

        Self::new_internal(
            config,
            Some(global_client),
        ).await
    }

    /// Create a new Meridian server instance in single-project mode (legacy compatibility)
    pub async fn new(config: Config) -> Result<Self> {
        Self::new_legacy(config).await
    }

    /// Internal constructor with optional global components
    async fn new_internal(
        config: Config,
        global_client: Option<Arc<GlobalServerClient>>,
    ) -> Result<Self> {
        let mode_str = if global_client.is_some() { "global" } else { "legacy" };
        info!("Initializing Meridian server in {} mode", mode_str);

        // Initialize storage (uses SurrealDB by default)
        let storage = create_default_storage(&config.storage.path).await?;

        // Initialize memory system with HNSW index path
        let hnsw_index_path = config.storage.hnsw_index_path.clone()
            .unwrap_or_else(|| config.storage.path.join("hnsw_index"));
        let mut memory_system = MemorySystem::with_index_path(
            storage.clone(),
            config.memory.clone(),
            Some(hnsw_index_path),
        )?;
        memory_system.init().await?;

        // Initialize context manager
        let context_manager = ContextManager::new(LLMAdapter::claude3());

        // Initialize indexer
        let mut indexer = CodeIndexer::new(storage.clone(), config.index.clone())?;
        indexer.load().await?;

        // Auto-index project on first run if index is empty
        if indexer.symbol_count() == 0 {
            if let Ok(cwd) = std::env::current_dir() {
                let src_path = cwd.join("src");
                if src_path.exists() {
                    info!("Index is empty, auto-indexing project at {:?}/src", cwd);
                    match indexer.index_project(&src_path, false).await {
                        Ok(_) => info!("Auto-indexing completed successfully"),
                        Err(e) => warn!("Auto-indexing failed: {}", e),
                    }
                }
            }
        } else {
            info!("Loaded {} symbols from existing index", indexer.symbol_count());
        }

        // Initialize delta indexer for incremental updates
        let indexer_arc = Arc::new(tokio::sync::RwLock::new(indexer));
        let delta_indexer = match DeltaIndexer::new(
            indexer_arc.clone(),
            WatcherConfig::default(), // Use default config (50ms debounce, common extensions)
            None, // No WAL for now
        ) {
            Ok(di) => {
                info!("Delta indexer initialized successfully");
                Some(Arc::new(di))
            }
            Err(e) => {
                warn!("Failed to initialize delta indexer: {}", e);
                None
            }
        };

        // Initialize documentation indexer
        let doc_indexer = Arc::new(crate::docs::DocIndexer::new());

        // Initialize global project registry for cross-monorepo support
        // This enables global.* and external.* MCP tools
        let (global_registry, current_project_specs) = {
            use crate::config::get_meridian_home;
            use crate::global::{GlobalStorage, ProjectRegistryManager};

            let data_dir = get_meridian_home().join("data");

            match GlobalStorage::new(&data_dir).await {
                Ok(storage) => {
                    info!("Initialized global project registry at {:?}", data_dir);
                    let storage_arc = Arc::new(storage);
                    let manager = Arc::new(ProjectRegistryManager::new(storage_arc));

                    // Auto-register current project if cwd is a valid project
                    if let Ok(cwd) = std::env::current_dir() {
                        match manager.register(cwd.clone()).await {
                            Ok(registry) => {
                                info!("Auto-registered project: {} at {:?}", registry.identity.full_id, cwd);
                                // Set as current project
                                if let Err(e) = manager.set_current_project(&registry.identity.full_id).await {
                                    warn!("Failed to set current project: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to auto-register project: {}", e);
                            }
                        }
                    }

                    // Try to get current project's specs path
                    let specs_path = manager.get_current_project_registry()
                        .await
                        .ok()
                        .flatten()
                        .and_then(|proj| proj.specs_path)
                        .filter(|p| p.exists());

                    (Some(manager), specs_path)
                }
                Err(e) => {
                    warn!("Failed to initialize global project registry: {}. Global tools will be unavailable.", e);
                    (None, None)
                }
            }
        };

        // Initialize specification manager
        // Try multiple locations to find specs directory:
        // 1. Environment variable MERIDIAN_SPECS_PATH
        // 2. Global project registry (current project)
        // 3. Current working directory/specs
        // 4. Current working directory/meridian/specs (for monorepo setups)
        // 5. Executable directory/../specs
        // 6. Storage path parent/specs (fallback - creates if needed)
        let specs_path = if let Ok(path) = std::env::var("MERIDIAN_SPECS_PATH") {
            info!("Using specs directory from MERIDIAN_SPECS_PATH: {:?}", path);
            PathBuf::from(path)
        } else if let Some(path) = current_project_specs {
            info!("Using specs directory from project registry: {:?}", path);
            path
        } else if let Ok(cwd) = std::env::current_dir() {
            // Try direct cwd/specs first
            let cwd_specs = cwd.join("specs");
            if cwd_specs.exists() && cwd_specs.is_dir() {
                info!("Using specs directory from current working directory: {:?}", cwd_specs);
                cwd_specs
            } else {
                // Try cwd/meridian/specs (for monorepo setups like omni/meridian)
                let meridian_specs = cwd.join("meridian").join("specs");
                if meridian_specs.exists() && meridian_specs.is_dir() {
                    info!("Using specs directory from meridian subdirectory: {:?}", meridian_specs);
                    meridian_specs
                } else {
                    // Try to find specs relative to executable
                    let exe_specs_result = std::env::current_exe()
                        .ok()
                        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
                        .and_then(|exe_parent| {
                            let exe_specs = exe_parent.join("..").join("specs");
                            exe_specs.canonicalize().ok()
                        })
                        .filter(|p| p.exists() && p.is_dir());

                    if let Some(exe_specs) = exe_specs_result {
                        info!("Using specs directory relative to executable: {:?}", exe_specs);
                        exe_specs
                    } else {
                        // Final fallback to storage path parent (legacy behavior)
                        let fallback = config.storage.path.parent()
                            .unwrap_or_else(|| std::path::Path::new("."))
                            .join("specs");
                        warn!("No specs directory found, using fallback: {:?}", fallback);
                        fallback
                    }
                }
            }
        } else {
            // Fallback to storage path parent (legacy behavior)
            let fallback = config.storage.path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("specs");
            warn!("Could not determine current directory, using fallback: {:?}", fallback);
            fallback
        };

        info!("Initializing SpecificationManager with path: {:?}", specs_path);
        let spec_manager = SpecificationManager::new(specs_path);

        // Initialize session manager
        let session_config = crate::session::SessionConfig {
            max_sessions: config.session.max_sessions,
            timeout: chrono::Duration::hours(1),
            auto_cleanup: true,
        };
        let session_manager = SessionManager::new(storage.clone(), session_config)?;

        // Initialize progress manager
        let progress_storage = Arc::new(TaskStorage::new(storage.clone()));
        let progress_manager = TaskManager::new(progress_storage);

        // Initialize links storage
        let links_storage = RocksDBLinksStorage::new(storage.clone());

        // Determine offline mode based on global client availability
        let offline = if let Some(ref client) = global_client {
            !client.is_available().await
        } else {
            false // Legacy mode doesn't have offline concept
        };

        // Initialize metrics system
        let metrics_collector = Arc::new(MetricsCollector::new());
        let metrics_path = config.storage.path.join("metrics");
        let metrics_storage = Arc::new(MetricsStorage::new(&metrics_path, Some(30)).await?);

        // Start background snapshot writer
        let snapshot_task = Some(Self::start_snapshot_writer(
            metrics_collector.clone(),
            metrics_storage.clone(),
        ));

        // Wrap components in Arc<RwLock<>> for shared access
        // Note: indexer_arc is already wrapped from delta indexer initialization above
        Ok(Self {
            mode: ServerMode::SingleProject {
                memory_system: Arc::new(tokio::sync::RwLock::new(memory_system)),
                context_manager: Arc::new(tokio::sync::RwLock::new(context_manager)),
                indexer: indexer_arc,
                delta_indexer,
                session_manager: Arc::new(session_manager),
                doc_indexer,
                spec_manager: Arc::new(tokio::sync::RwLock::new(spec_manager)),
                progress_manager: Arc::new(tokio::sync::RwLock::new(progress_manager)),
                links_storage: Arc::new(tokio::sync::RwLock::new(links_storage)),
                handlers: None,
                global_client,
                global_registry,
                monorepo_context: None, // TODO: Detect monorepo context
                offline_mode: Arc::new(AtomicBool::new(offline)),
                metrics_collector,
                metrics_storage,
                snapshot_task,
            },
            config,
        })
    }

    /// Start background snapshot writer task with error recovery
    ///
    /// This task runs every 60 seconds and saves a metrics snapshot to storage.
    /// Uses error recovery to handle transient failures without crashing.
    fn start_snapshot_writer(
        collector: Arc<MetricsCollector>,
        storage: Arc<MetricsStorage>,
    ) -> JoinHandle<()> {
        use crate::error_recovery::run_background_task_with_recovery;

        tokio::spawn(async move {
            run_background_task_with_recovery(
                "metrics_snapshot_writer",
                std::time::Duration::from_secs(60),
                || {
                    let collector = collector.clone();
                    let storage = storage.clone();
                    async move {
                        let snapshot = collector.take_snapshot();
                        storage.save_snapshot(&snapshot).await
                    }
                },
            )
            .await
        })
    }

    // Removed get_specs_path_from_registry_async() - now handled directly in new_internal()

    /// Create a new Meridian server instance in multi-project mode for HTTP
    pub fn new_for_http(config: Config) -> Result<Self> {
        info!("Initializing Meridian server in multi-project mode for HTTP");

        let max_projects = config
            .mcp
            .http
            .as_ref().map(|h| h.max_connections)
            .unwrap_or(10);

        let project_manager = Arc::new(ProjectManager::new(config.clone(), max_projects));

        Ok(Self {
            mode: ServerMode::MultiProject { project_manager },
            config,
        })
    }

    /// Initialize tool handlers (only for single-project mode)
    /// Uses existing components (already wrapped in Arc<RwLock<>>) to avoid RocksDB lock conflicts
    fn init_handlers(&mut self) -> Result<Arc<ToolHandlers>> {
        match &mut self.mode {
            ServerMode::SingleProject {
                handlers,
                memory_system,
                context_manager,
                indexer,
                delta_indexer,
                session_manager,
                doc_indexer,
                spec_manager,
                progress_manager,
                links_storage,
                global_registry,
                metrics_collector,
                ..
            } => {
                if let Some(h) = handlers {
                    return Ok(h.clone());
                }

                // Clone Arc pointers (cheap operation)
                // Initialize pattern search engine
                let pattern_engine = Arc::new(crate::indexer::PatternSearchEngine::new()
                    .expect("Failed to initialize pattern search engine"));

                // Create handlers with or without global registry
                let mut new_handlers = if let Some(registry) = global_registry {
                    info!("Creating ToolHandlers with global project registry support");
                    ToolHandlers::new_with_registry(
                        memory_system.clone(),
                        context_manager.clone(),
                        indexer.clone(),
                        session_manager.clone(),
                        doc_indexer.clone(),
                        spec_manager.clone(),
                        registry.clone(),
                        progress_manager.clone(),
                        links_storage.clone(),
                        pattern_engine,
                    )
                } else {
                    warn!("Creating ToolHandlers without global registry - cross-monorepo features disabled");
                    ToolHandlers::new(
                        memory_system.clone(),
                        context_manager.clone(),
                        indexer.clone(),
                        session_manager.clone(),
                        doc_indexer.clone(),
                        spec_manager.clone(),
                        progress_manager.clone(),
                        links_storage.clone(),
                        pattern_engine,
                    )
                };

                // Set delta indexer if available
                if let Some(di) = delta_indexer {
                    info!("Setting delta indexer on ToolHandlers");
                    new_handlers.set_delta_indexer(di.clone());
                }

                // Set metrics collector
                debug!("Setting metrics collector on ToolHandlers");
                new_handlers.set_metrics_collector(metrics_collector.clone());

                let new_handlers = Arc::new(new_handlers);

                *handlers = Some(new_handlers.clone());
                debug!("ToolHandlers initialized and stored");
                Ok(new_handlers)
            }
            ServerMode::MultiProject { .. } => {
                anyhow::bail!("init_handlers should not be called in multi-project mode")
            }
        }
    }

    /// Check if server is in offline mode
    pub fn is_offline(&self) -> bool {
        match &self.mode {
            ServerMode::SingleProject { offline_mode, .. } => offline_mode.load(Ordering::SeqCst),
            ServerMode::MultiProject { .. } => false,
        }
    }

    /// Get global client if available
    pub fn get_global_client(&self) -> Option<Arc<GlobalServerClient>> {
        match &self.mode {
            ServerMode::SingleProject { global_client, .. } => global_client.clone(),
            ServerMode::MultiProject { .. } => None,
        }
    }

    /// Serve via stdio transport
    pub async fn serve_stdio(&mut self) -> Result<()> {
        info!("Starting MCP server with stdio transport");

        let handlers = self.init_handlers()?;
        let mut transport = StdioTransport::new();

        info!("Meridian MCP server ready on stdio");

        // Main event loop
        while let Some(request) = transport.recv().await {
            let response = self.handle_request(request.clone(), &handlers).await;

            // JSON-RPC 2.0: Only send response if request had an id (not a notification)
            // Notifications (no id) MUST NOT receive responses
            if response.id.is_some() {
                if let Err(e) = transport.send(response) {
                    error!("Failed to send response: {}", e);
                    break;
                }
            } else {
                debug!("Skipping response for notification (no id)");
            }
        }

        info!("Stdio transport closed");
        Ok(())
    }

    /// Serve via Unix socket (placeholder)
    pub async fn serve_socket(&self, socket_path: PathBuf) -> Result<()> {
        info!("Starting MCP server with socket transport at {:?}", socket_path);
        warn!("Socket transport not yet implemented");
        Ok(())
    }

    /// Serve via HTTP/SSE transport
    pub async fn serve_http(&mut self) -> Result<()> {
        info!("Starting MCP server with HTTP/SSE transport");

        let http_config = self
            .config
            .mcp
            .http
            .clone()
            .unwrap_or_default();

        if !http_config.enabled {
            anyhow::bail!("HTTP transport is not enabled in configuration");
        }

        match &self.mode {
            ServerMode::MultiProject { project_manager } => {
                // Use project manager for multi-project mode
                let transport = super::http_transport::HttpTransport::new_with_project_manager(
                    project_manager.clone(),
                    http_config,
                );
                transport.serve().await
            }
            ServerMode::SingleProject { .. } => {
                // Fallback to single-project mode
                let handlers = self.init_handlers()?;
                let transport = super::http_transport::HttpTransport::new(handlers, http_config);
                transport.serve().await
            }
        }
    }

    /// Handle a JSON-RPC request
    async fn handle_request(
        &self,
        request: JsonRpcRequest,
        handlers: &Arc<ToolHandlers>,
    ) -> JsonRpcResponse {
        let request_id = request.id.clone();

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request_id, request.params),
            "initialized" | "notifications/initialized" => {
                // MCP initialization notification - JSON-RPC 2.0: notifications MUST NOT receive responses
                info!("Received initialized notification - handshake complete");
                // Return empty response with no id (will be filtered out in main loop)
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,  // No id = no response will be sent
                    result: None,
                    error: None,
                }
            }
            "tools/list" => self.handle_list_tools(request_id),
            "tools/call" => self.handle_call_tool(request_id, request.params, handlers).await,
            "resources/list" => self.handle_list_resources(request_id),
            "resources/read" => self.handle_read_resource(request_id, request.params).await,
            "ping" => JsonRpcResponse::success(request_id, json!({"status": "ok"})),
            _ => JsonRpcResponse::error(
                request_id,
                JsonRpcError::method_not_found(format!("Method not found: {}", request.method)),
            ),
        }
    }

    /// Handle initialize request with protocol version negotiation
    pub fn handle_initialize(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        info!("Handling initialize request");

        // Extract client's requested protocol version
        let client_version = params
            .as_ref()
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or("2024-11-05");

        // Negotiate protocol version (use minimum of client and server versions)
        // Server supports: 2024-11-05, 2025-03-26, 2025-06-18
        // Use LATEST stable: 2025-03-26 (matching official SDK)
        let server_latest = "2025-03-26";

        let negotiated_version = match client_version {
            // If client supports newer or equal version, use server's latest stable
            v if v >= server_latest => server_latest,
            // If client supports older version, use client's version (backward compat)
            v if ["2024-11-05", "2025-03-26"].contains(&v) => v,
            // Unknown version, fallback to oldest stable
            _ => {
                warn!(
                    "Unknown client protocol version: {}, using fallback",
                    client_version
                );
                "2024-11-05"
            }
        };

        info!("Client requested protocol version: {}", client_version);
        info!("Server negotiated protocol version: {}", negotiated_version);

        let result = json!({
            "protocolVersion": negotiated_version,  // Now uses negotiated version
            "capabilities": ServerCapabilities::default(),
            "serverInfo": {
                "name": "meridian",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        JsonRpcResponse::success(id, result)
    }

    /// Handle tools/list request
    pub fn handle_list_tools(&self, id: Option<Value>) -> JsonRpcResponse {
        info!("Handling tools/list request");

        let tools = get_all_tools();
        let result = json!({ "tools": tools });

        JsonRpcResponse::success(id, result)
    }

    /// Handle tools/call request
    async fn handle_call_tool(
        &self,
        id: Option<Value>,
        params: Option<Value>,
        handlers: &Arc<ToolHandlers>,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing parameters".to_string()),
                )
            }
        };

        // Extract tool name and arguments
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing tool name".to_string()),
                )
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        info!("Calling tool: {}", tool_name);

        // Call the tool handler
        match handlers.handle_tool_call(tool_name, arguments).await {
            Ok(result) => {
                let response = json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                        }
                    ]
                });
                JsonRpcResponse::success(id, response)
            }
            Err(e) => {
                error!("Tool call failed: {}", e);
                JsonRpcResponse::error(
                    id,
                    JsonRpcError::internal_error(format!("Tool execution failed: {}", e)),
                )
            }
        }
    }

    /// Handle resources/list request
    pub fn handle_list_resources(&self, id: Option<Value>) -> JsonRpcResponse {
        info!("Handling resources/list request");

        let resources = get_all_resources();
        let result = json!({ "resources": resources });

        JsonRpcResponse::success(id, result)
    }

    /// Handle resources/read request
    async fn handle_read_resource(
        &self,
        id: Option<Value>,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing parameters".to_string()),
                )
            }
        };

        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(uri) => uri,
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing resource URI".to_string()),
                )
            }
        };

        info!("Reading resource: {}", uri);

        // Handle different resource URIs
        let content = match uri {
            "meridian://index/current" => {
                json!({
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json!({
                        "status": "active",
                        "total_symbols": 0,
                        "total_files": 0
                    }).to_string()
                })
            }
            "meridian://memory/episodes" => {
                json!({
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json!({
                        "episodes": []
                    }).to_string()
                })
            }
            "meridian://memory/working" => {
                json!({
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json!({
                        "active_symbols": [],
                        "token_count": 0
                    }).to_string()
                })
            }
            "meridian://sessions/active" => {
                match &self.mode {
                    ServerMode::SingleProject {
                        session_manager, ..
                    } => {
                        let sessions = session_manager.list_sessions().await;
                        json!({
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": json!({
                                "sessions": sessions.iter().map(|s| {
                                    json!({
                                        "id": s.id.0,
                                        "task": s.task_description,
                                        "started_at": s.started_at.to_rfc3339()
                                    })
                                }).collect::<Vec<_>>()
                            }).to_string()
                        })
                    }
                    ServerMode::MultiProject { .. } => {
                        json!({
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": json!({
                                "sessions": [],
                                "note": "Multi-project mode: specify project_path to query sessions"
                            }).to_string()
                        })
                    }
                }
            }
            "improvement://dashboard" => {
                match &self.mode {
                    ServerMode::SingleProject { memory_system, .. } => {
                        // Get SurrealDB instance from memory system
                        let memory = memory_system.read().await;
                        let db = match memory.get_db() {
                            Ok(db) => db,
                            Err(e) => {
                                return JsonRpcResponse::error(
                                    id,
                                    JsonRpcError::internal_error(format!("Failed to get database: {}", e)),
                                )
                            }
                        };

                        // Collect metrics using the self-improvement collector
                        use crate::metrics::SelfImprovementCollector;
                        use crate::analysis::CodeHealthAnalyzer;

                        let collector = SelfImprovementCollector::new(db.clone());
                        let health_analyzer = CodeHealthAnalyzer::new(db.clone());

                        // Collect current metrics
                        let metrics_result = collector.collect().await;
                        let health_result = health_analyzer.analyze().await;

                        match (metrics_result, health_result) {
                            (Ok(metrics), Ok(health)) => {
                                json!({
                                    "uri": uri,
                                    "mimeType": "application/json",
                                    "text": json!({
                                        "timestamp": metrics.timestamp.to_rfc3339(),
                                        "health_score": metrics.health_score,
                                        "health_rating": metrics.health_rating(),
                                        "trend": metrics.trend_direction,
                                        "metrics": {
                                            "code_quality_score": metrics.code_quality_score,
                                            "test_coverage_percent": metrics.test_coverage_percent,
                                            "avg_cyclomatic_complexity": metrics.avg_cyclomatic_complexity,
                                            "technical_debt_score": metrics.technical_debt_score,
                                        },
                                        "issues": {
                                            "circular_dependencies": metrics.circular_dependencies_count,
                                            "untested_symbols": metrics.untested_symbols_count,
                                            "undocumented_symbols": metrics.undocumented_symbols_count,
                                            "high_complexity_symbols": metrics.high_complexity_symbols_count,
                                        },
                                        "improvements": {
                                            "per_week": metrics.improvements_per_week,
                                            "avg_time_hours": metrics.avg_improvement_time_hours,
                                        },
                                        "language_breakdown": metrics.language_breakdown,
                                        "summary": health.summary,
                                        "top_issues": health.issues.iter().take(10).collect::<Vec<_>>(),
                                        "recommendations": health.recommendations,
                                    }).to_string()
                                })
                            }
                            (Err(e), _) | (_, Err(e)) => {
                                json!({
                                    "uri": uri,
                                    "mimeType": "application/json",
                                    "text": json!({
                                        "error": format!("Failed to collect metrics: {}", e),
                                        "health_score": 0.0,
                                        "metrics": {}
                                    }).to_string()
                                })
                            }
                        }
                    }
                    ServerMode::MultiProject { .. } => {
                        json!({
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": json!({
                                "error": "Multi-project mode not yet supported for improvement dashboard"
                            }).to_string()
                        })
                    }
                }
            }
            _ => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("Unknown resource: {}", uri)),
                )
            }
        };

        JsonRpcResponse::success(id, json!({ "contents": [content] }))
    }

    // === Non-MCP utility methods ===

    /// Index a project (only works in single-project mode)
    pub async fn index_project(&mut self, path: PathBuf, force: bool) -> Result<()> {
        match &mut self.mode {
            ServerMode::SingleProject { indexer, .. } => {
                let mut indexer_guard = indexer.write().await;
                indexer_guard.index_project(&path, force).await
            }
            ServerMode::MultiProject { .. } => {
                anyhow::bail!("index_project not supported in multi-project mode")
            }
        }
    }

    /// Query the index (only works in single-project mode)
    pub async fn query(&self, query_text: &str, limit: usize) -> Result<Vec<String>> {
        match &self.mode {
            ServerMode::SingleProject { indexer, .. } => {
                let query = Query::new(query_text.to_string());
                let indexer_guard = indexer.read().await;
                let results = indexer_guard.search_symbols(&query).await?;

                Ok(results
                    .symbols
                    .iter()
                    .take(limit)
                    .map(|s| format!("{} ({})", s.name, s.kind.as_str()))
                    .collect())
            }
            ServerMode::MultiProject { .. } => {
                anyhow::bail!("query not supported in multi-project mode")
            }
        }
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<IndexStats> {
        // TODO: Collect actual statistics from components
        Ok(IndexStats::empty())
    }

    /// Initialize a new index
    pub async fn initialize(&self, _path: PathBuf) -> Result<()> {
        info!("Initializing new index");
        // TODO: Create index structure
        Ok(())
    }

    /// Get project manager (only for multi-project mode)
    pub fn get_project_manager(&self) -> Option<Arc<ProjectManager>> {
        match &self.mode {
            ServerMode::MultiProject { project_manager } => Some(project_manager.clone()),
            ServerMode::SingleProject { .. } => None,
        }
    }

    /// Get metrics collector (only for single-project mode)
    pub fn get_metrics_collector(&self) -> Option<Arc<MetricsCollector>> {
        match &self.mode {
            ServerMode::SingleProject { metrics_collector, .. } => Some(metrics_collector.clone()),
            ServerMode::MultiProject { .. } => None,
        }
    }
}

/// Implement Drop for graceful shutdown
impl Drop for MeridianServer {
    fn drop(&mut self) {
        if let ServerMode::SingleProject {
            snapshot_task,
            metrics_collector,
            metrics_storage,
            memory_system,
            ..
        } = &mut self.mode
        {
            info!("Gracefully shutting down Meridian server...");

            // Cancel background snapshot task
            if let Some(task) = snapshot_task.take() {
                task.abort();
                info!("Cancelled background snapshot writer");
            }

            // Save HNSW index to disk for fast startup next time
            // Use blocking call since we're in Drop (no async context)
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let memory_clone = Arc::clone(memory_system);
                let _ = handle.spawn(async move {
                    let memory_guard = memory_clone.read().await;
                    if let Err(e) = memory_guard.save_index() {
                        error!("Failed to save HNSW index: {}", e);
                    } else {
                        info!("HNSW index saved successfully");
                    }
                });
            }

            // Take final snapshot
            let snapshot = metrics_collector.take_snapshot();
            info!("Taking final metrics snapshot with {} tools tracked", snapshot.tools.len());

            // Try to save final snapshot using spawn_blocking to avoid nested runtime issues
            let storage = Arc::clone(metrics_storage);
            let snapshot_clone = snapshot.clone();

            // Try to spawn a task to save the snapshot
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                // Spawn a blocking task that won't block the current thread
                let _ = handle.spawn(async move {
                    if let Err(e) = storage.save_snapshot(&snapshot_clone).await {
                        error!("Failed to save final metrics snapshot: {}", e);
                    } else {
                        info!("Final metrics snapshot saved successfully");
                    }
                });
                // Note: We can't wait for this to complete in Drop
                // The snapshot will be saved eventually by the runtime
            } else {
                warn!("No tokio runtime available for final snapshot save");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_server() -> (MeridianServer, TempDir) {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::SystemTime;
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let temp_dir = TempDir::new().unwrap();
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = temp_dir.path().join(format!("db_{}_{}", timestamp, counter));
        std::fs::create_dir_all(&db_path).unwrap();

        let config = Config {
            index: crate::config::IndexConfig {
                languages: vec!["rust".to_string()],
                ignore: vec![],
                max_file_size: "1MB".to_string(),
            },
            storage: crate::config::StorageConfig {
                path: db_path,
                cache_size: "256MB".to_string(),
                hnsw_index_path: None,
            },
            memory: crate::config::MemoryConfig {
                episodic_retention_days: 30,
                working_memory_size: "10MB".to_string(),
                consolidation_interval: "1h".to_string(),
            },
            session: crate::config::SessionConfig {
                max_sessions: 10,
                session_timeout: "1h".to_string(),
            },
            monorepo: crate::config::MonorepoConfig::default(),
            learning: crate::config::LearningConfig::default(),
            mcp: crate::config::McpConfig::default(),
        };

        let server = MeridianServer::new(config).await.unwrap();
        (server, temp_dir)
    }

    #[tokio::test]
    async fn test_server_initialization() {
        let (_server, _temp) = create_test_server().await;
        // Server should initialize without errors
    }

    #[tokio::test]
    async fn test_initialize_request() {
        let (server, _temp) = create_test_server().await;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };

        // Test initialize request directly - no handlers needed
        let response = server.handle_initialize(request.id, request.params);

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_list_tools_request() {
        let (server, _temp) = create_test_server().await;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        // Test list_tools request directly - no handlers needed
        let response = server.handle_list_tools(request.id);

        assert!(response.result.is_some());

        let result = response.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let (server, _temp) = create_test_server().await;

        // Create a dummy handler just for testing unknown method
        use crate::storage::MemoryStorage;
        let storage = Arc::new(MemoryStorage::new()) as Arc<dyn crate::storage::Storage>;
        let memory_system = MemorySystem::new(storage.clone(), server.config.memory.clone()).unwrap();
        let context_manager = ContextManager::new(LLMAdapter::claude3());
        let indexer = CodeIndexer::new(storage.clone(), server.config.index.clone()).unwrap();
        let session_config = crate::session::SessionConfig {
            max_sessions: 10,
            timeout: chrono::Duration::hours(1),
            auto_cleanup: true,
        };
        let session_manager = SessionManager::new(storage.clone(), session_config).unwrap();
        let doc_indexer = Arc::new(crate::docs::DocIndexer::new());
        let specs_path = _temp.path().join("specs");
        let spec_manager = SpecificationManager::new(specs_path);

        // Initialize progress manager and links storage for tests
        let progress_storage = Arc::new(TaskStorage::new(storage.clone()));
        let progress_manager = TaskManager::new(progress_storage);
        let links_storage = RocksDBLinksStorage::new(storage.clone());

        let pattern_engine = Arc::new(crate::indexer::PatternSearchEngine::new().unwrap());

        let handlers = Arc::new(ToolHandlers::new(
            Arc::new(tokio::sync::RwLock::new(memory_system)),
            Arc::new(tokio::sync::RwLock::new(context_manager)),
            Arc::new(tokio::sync::RwLock::new(indexer)),
            Arc::new(session_manager),
            doc_indexer,
            Arc::new(tokio::sync::RwLock::new(spec_manager)),
            Arc::new(tokio::sync::RwLock::new(progress_manager)),
            Arc::new(tokio::sync::RwLock::new(links_storage)),
            pattern_engine,
        ));

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "unknown/method".to_string(),
            params: None,
        };

        let response = server.handle_request(request, &handlers).await;

        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601); // Method not found
    }
}
