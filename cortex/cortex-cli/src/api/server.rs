//! REST API Server implementation

use super::middleware::{cors_layer, RequestLogger, RateLimiter};
use super::routes::{
    auth::AuthContext,
    build::BuildContext,
    dashboard::DashboardContext,
    dependencies::DependencyContext,
    export::ExportContext,
    health::AppState,
    memory::MemoryContext,
    search::SearchContext,
    sessions::SessionContext,
    tasks::TaskContext,
    units::CodeUnitContext,
    vfs::VfsContext,
    workspaces::WorkspaceContext,
};
use super::websocket::WsManager;
use anyhow::{Context, Result};
use axum::{middleware, Router};
use cortex_core::config::GlobalConfig;
use cortex_memory::CognitiveManager;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig};
use cortex_vfs::VirtualFileSystem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

/// REST API Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: None,
        }
    }
}

/// REST API Server
pub struct RestApiServer {
    config: ServerConfig,
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    memory: Arc<CognitiveManager>,
    ws_manager: WsManager,
    rate_limiter: RateLimiter,
    start_time: Instant,
}

impl RestApiServer {
    /// Create a new REST API server using global configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(ServerConfig::default()).await
    }

    /// Create a new REST API server with custom configuration
    pub async fn with_config(config: ServerConfig) -> Result<Self> {
        info!("Initializing REST API Server");

        // Load global configuration
        let global_config = GlobalConfig::load_or_create_default().await?;
        info!("Configuration loaded from {}", GlobalConfig::config_path()?.display());

        // Initialize database connection
        let storage = Self::create_storage(&global_config).await?;
        info!("Database connection established");

        // Create VFS
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

        // Create cognitive memory manager
        let memory = Arc::new(CognitiveManager::new(storage.clone()));

        // Initialize authentication schema
        if let Err(e) = super::db_schema::initialize_auth_schema(&storage).await {
            tracing::warn!("Failed to initialize auth schema: {}", e);
        }

        // Create default admin user if needed
        if let Err(e) = super::db_schema::create_default_admin(&storage).await {
            tracing::warn!("Failed to create default admin: {}", e);
        }

        // Create WebSocket manager
        let ws_manager = WsManager::new();

        // Create rate limiter
        let rate_limiter = RateLimiter::new();

        info!("REST API Server initialized successfully");

        Ok(Self {
            config,
            storage,
            vfs,
            memory,
            ws_manager,
            rate_limiter,
            start_time: Instant::now(),
        })
    }

    /// Start the REST API server
    pub async fn serve(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let socket_addr: SocketAddr = addr.parse()
            .context("Failed to parse socket address")?;

        info!("Starting REST API server on {}", addr);

        // Build the application router
        let app = self.build_app();

        // Create the server
        let listener = tokio::net::TcpListener::bind(&socket_addr)
            .await
            .context("Failed to bind to address")?;

        info!("REST API server listening on http://{}", addr);
        info!("Available endpoints:");
        info!("");
        info!("Authentication:");
        info!("  POST /api/v3/auth/login");
        info!("  POST /api/v3/auth/refresh");
        info!("  POST /api/v3/auth/logout");
        info!("  POST /api/v3/auth/api-key");
        info!("  GET  /api/v3/auth/me");
        info!("");
        info!("WebSocket:");
        info!("  WS   /api/v3/ws");
        info!("");
        info!("Health & Monitoring:");
        info!("  GET  /api/v3/health");
        info!("  GET  /api/v3/metrics");
        info!("");
        info!("Workspaces & Files:");
        info!("  GET    /api/v3/workspaces");
        info!("  POST   /api/v3/workspaces");
        info!("  GET    /api/v3/workspaces/:id");
        info!("  PUT    /api/v3/workspaces/:id");
        info!("  DELETE /api/v3/workspaces/:id");
        info!("  POST   /api/v3/workspaces/:id/sync");
        info!("  GET    /api/v3/workspaces/:id/files");
        info!("  POST   /api/v3/workspaces/:id/files");
        info!("  GET    /api/v3/workspaces/:id/tree");
        info!("  GET    /api/v3/files/:id");
        info!("  PUT    /api/v3/files/:id");
        info!("  DELETE /api/v3/files/:id");
        info!("");
        info!("Sessions & Search:");
        info!("  GET  /api/v3/sessions");
        info!("  POST /api/v3/sessions");
        info!("  GET  /api/v3/search");
        info!("  GET  /api/v3/search/references/:unit_id");
        info!("  POST /api/v3/search/pattern");
        info!("");
        info!("Memory & Code Units:");
        info!("  GET  /api/v3/memory/episodes");
        info!("  GET  /api/v3/memory/episodes/:id");
        info!("  POST /api/v3/memory/consolidate");
        info!("  POST /api/v3/memory/search");
        info!("  GET  /api/v3/memory/patterns");
        info!("  GET  /api/v3/workspaces/:id/units");
        info!("  GET  /api/v3/units/:id");
        info!("  PUT  /api/v3/units/:id");
        info!("");
        info!("Analysis & Build:");
        info!("  GET  /api/v3/workspaces/:id/dependencies");
        info!("  POST /api/v3/analysis/impact");
        info!("  GET  /api/v3/analysis/cycles");
        info!("  POST /api/v3/build/trigger");
        info!("  GET  /api/v3/build/:id/status");
        info!("  POST /api/v3/test/run");
        info!("  GET  /api/v3/test/:id/results");
        info!("");
        info!("Dashboard:");
        info!("  GET  /api/v3/dashboard/overview");
        info!("  GET  /api/v3/dashboard/activity");
        info!("  GET  /api/v3/dashboard/metrics");
        info!("  GET  /api/v3/dashboard/health");
        info!("");
        info!("Tasks:");
        info!("  GET  /api/v3/tasks");
        info!("  POST /api/v3/tasks");
        info!("  GET  /api/v3/tasks/:id");
        info!("  PUT  /api/v3/tasks/:id");
        info!("  DELETE /api/v3/tasks/:id");
        info!("");
        info!("Export/Import:");
        info!("  POST /api/v3/export");
        info!("  GET  /api/v3/export/:id");
        info!("  GET  /api/v3/export/:id/download");
        info!("  POST /api/v3/import");
        info!("  GET  /api/v3/import/:id");
        info!("");
        info!("Locks & Merge:");
        info!("  GET  /api/v3/locks");
        info!("  POST /api/v3/sessions/:id/merge");
        info!("");
        info!("Press Ctrl+C to stop");

        // Run the server
        axum::serve(listener, app)
            .await
            .context("Server error")?;

        Ok(())
    }

    /// Build the application router with all routes and middleware
    fn build_app(self) -> Router {
        // Create shared state for health endpoints
        let app_state = Arc::new(AppState {
            start_time: self.start_time,
            storage: self.storage.clone(),
        });

        // Create contexts for different route groups
        let workspace_context = WorkspaceContext {
            vfs: self.vfs.clone(),
            storage: self.storage.clone(),
        };

        let vfs_context = VfsContext {
            vfs: self.vfs.clone(),
            storage: self.storage.clone(),
        };

        let session_context = SessionContext {
            storage: self.storage.clone(),
            vfs: self.vfs.clone(),
        };

        let search_context = SearchContext {
            storage: self.storage.clone(),
            memory: self.memory.clone(),
        };

        let memory_context = MemoryContext {
            storage: self.storage.clone(),
            memory: self.memory.clone(),
        };

        let code_unit_context = CodeUnitContext {
            storage: self.storage.clone(),
            vfs: self.vfs.clone(),
        };

        let dependency_context = DependencyContext {
            storage: self.storage.clone(),
        };

        let build_context = BuildContext::new(self.storage.clone());

        // Create authentication context
        let auth_context = AuthContext::new(self.storage.clone());

        // Create dashboard context
        let dashboard_context = DashboardContext {
            storage: self.storage.clone(),
        };

        // Create task context
        let task_context = TaskContext {
            storage: self.storage.clone(),
        };

        // Create export context
        let export_context = ExportContext {
            storage: self.storage.clone(),
        };

        // Build the router with all routes
        Router::new()
            // Public routes (no auth required)
            .merge(super::routes::health_routes(app_state))
            .merge(super::routes::auth_routes(auth_context))
            // WebSocket route
            .merge(super::websocket::websocket_routes(self.ws_manager.clone()))
            // Protected routes (auth required)
            .merge(super::routes::workspace_routes(workspace_context))
            .merge(super::routes::vfs_routes(vfs_context))
            .merge(super::routes::session_routes(session_context))
            .merge(super::routes::search_routes(search_context))
            .merge(super::routes::memory_routes(memory_context))
            .merge(super::routes::code_unit_routes(code_unit_context))
            .merge(super::routes::dependency_routes(dependency_context))
            .merge(super::routes::build_routes(build_context))
            .merge(super::routes::dashboard_routes(dashboard_context))
            .merge(super::routes::task_routes(task_context))
            .merge(super::routes::export_routes(export_context))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(cors_layer())
                    .layer(middleware::from_fn(RequestLogger::log))
            )
    }

    /// Creates storage connection manager from configuration
    async fn create_storage(config: &GlobalConfig) -> Result<Arc<ConnectionManager>> {
        use cortex_storage::PoolConnectionMode;

        let db_config = config.database();
        let pool_config = config.pool();

        // Convert config to ConnectionManager format
        let connection_mode = if db_config.mode == "local" {
            PoolConnectionMode::Local {
                endpoint: format!("ws://{}", db_config.local_bind),
            }
        } else {
            PoolConnectionMode::Remote {
                endpoints: db_config.remote_urls.clone(),
                load_balancing: cortex_storage::LoadBalancingStrategy::RoundRobin,
            }
        };

        let credentials = Credentials {
            username: Some(db_config.username.clone()),
            password: Some(db_config.password.clone()),
        };

        let pool_cfg = PoolConfig {
            min_connections: pool_config.min_connections as usize,
            max_connections: pool_config.max_connections as usize,
            connection_timeout: std::time::Duration::from_millis(pool_config.connection_timeout_ms),
            idle_timeout: Some(std::time::Duration::from_millis(pool_config.idle_timeout_ms)),
            ..Default::default()
        };

        let database_config = DatabaseConfig {
            connection_mode,
            credentials,
            pool_config: pool_cfg,
            namespace: db_config.namespace.clone(),
            database: db_config.database.clone(),
        };

        let manager = ConnectionManager::new(database_config)
            .await
            .context("Failed to create storage connection")?;

        Ok(Arc::new(manager))
    }
}
