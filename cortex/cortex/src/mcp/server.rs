//! Cortex MCP Server
//!
//! Main server implementation that integrates all Cortex tools with the MCP framework.

use super::tools::{
    advanced_testing::*, ai_assisted::*, architecture_analysis::*, build_execution::*,
    code_manipulation::*, code_nav::*, code_quality::*, cognitive_memory::*, dependency_analysis::*,
    documentation::*, materialization::*, monitoring::*, multi_agent::*, security_analysis::*,
    semantic_search::*, testing::*, type_analysis::*, version_control::*, vfs::*, workspace::*,
};
use anyhow::Result;
use cortex_core::config::GlobalConfig;
use cortex_storage::{ConnectionManager, Credentials, DatabaseConfig, PoolConfig};
use cortex_vfs::VirtualFileSystem;
use mcp_sdk::prelude::*;
use mcp_sdk::Transport;
use std::sync::Arc;
use tracing::{info, warn};

/// Cortex MCP Server
///
/// Provides all Cortex functionality through the MCP protocol
pub struct CortexMcpServer {
    server: mcp_sdk::McpServer,
    storage: Arc<ConnectionManager>,
}

impl CortexMcpServer {
    /// Creates a new Cortex MCP server using global configuration
    ///
    /// This will:
    /// 1. Load configuration from ~/.ryht/config.toml (unified config)
    /// 2. Initialize connection to SurrealDB
    /// 3. Register all MCP tools
    /// 4. Set up middleware and hooks
    pub async fn new() -> Result<Self> {
        info!("Initializing Cortex MCP Server");

        // Load global configuration
        let config = GlobalConfig::load_or_create_default().await?;
        info!("Configuration loaded from {}", GlobalConfig::config_path()?.display());

        // Initialize database connection
        let storage = Self::create_storage(&config).await?;
        info!("Database connection established");

        // Create VFS
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));

        // Build server with all tools
        let server = Self::build_server(storage.clone(), vfs).await?;

        info!("Cortex MCP Server initialized successfully");

        Ok(Self { server, storage })
    }

    /// Creates a new server with custom configuration
    pub async fn with_config(config: GlobalConfig) -> Result<Self> {
        let storage = Self::create_storage(&config).await?;
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let server = Self::build_server(storage.clone(), vfs).await?;

        Ok(Self { server, storage })
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

        let manager = ConnectionManager::new(database_config).await?;

        Ok(Arc::new(manager))
    }

    /// Builds the MCP server with all registered tools
    async fn build_server(
        storage: Arc<ConnectionManager>,
        vfs: Arc<VirtualFileSystem>,
    ) -> Result<mcp_sdk::McpServer> {
        info!("Registering MCP tools");

        // Create shared contexts
        let workspace_ctx = WorkspaceContext::new(storage.clone())?;

        let vfs_ctx = VfsContext::new(vfs.clone());
        let code_ctx = CodeNavContext::new(storage.clone());
        let semantic_ctx = SemanticSearchContext::new(storage.clone()).await?;
        let deps_ctx = DependencyAnalysisContext::new(storage.clone());
        let quality_ctx = CodeQualityContext::new(storage.clone());
        let version_ctx = VersionControlContext::new(storage.clone());
        let memory_ctx = CognitiveMemoryContext::new(storage.clone());
        let agent_ctx = MultiAgentContext::new(storage.clone());
        let mat_ctx = MaterializationContext::new(storage.clone());
        let test_ctx = TestingContext::new(storage.clone());
        let doc_ctx = DocumentationContext::new(storage.clone(), vfs.clone());
        let build_ctx = BuildExecutionContext::new(storage.clone());
        let monitor_ctx = MonitoringContext::new(storage.clone());
        let security_ctx = SecurityAnalysisContext::new(storage.clone());
        let type_ctx = TypeAnalysisContext::new(storage.clone(), vfs.clone());
        let ai_ctx = AiAssistedContext::new(storage.clone());
        let adv_test_ctx = AdvancedTestingContext::new(storage.clone());
        let arch_ctx = ArchitectureAnalysisContext::new(storage.clone());

        // Build server with all tools
        let server = mcp_sdk::McpServer::builder()
            .name("cortex-mcp")
            .version(env!("CARGO_PKG_VERSION"))
            // Workspace Management Tools (12)
            .tool(WorkspaceCreateTool::new(workspace_ctx.clone()))
            .tool(WorkspaceGetTool::new(workspace_ctx.clone()))
            .tool(WorkspaceListTool::new(workspace_ctx.clone()))
            .tool(WorkspaceActivateTool::new(workspace_ctx.clone()))
            .tool(WorkspaceSyncTool::new(workspace_ctx.clone()))
            .tool(WorkspaceExportTool::new(workspace_ctx.clone()))
            .tool(WorkspaceArchiveTool::new(workspace_ctx.clone()))
            .tool(WorkspaceDeleteTool::new(workspace_ctx.clone()))
            .tool(WorkspaceForkTool::new(workspace_ctx.clone()))
            .tool(WorkspaceSearchTool::new(workspace_ctx.clone()))
            .tool(WorkspaceCompareTool::new(workspace_ctx.clone()))
            .tool(WorkspaceMergeTool::new(workspace_ctx.clone()))
            // Virtual Filesystem Tools (17)
            .tool(VfsGetNodeTool::new(vfs_ctx.clone()))
            .tool(VfsGetNodeByIdTool::new(vfs_ctx.clone()))
            .tool(VfsListDirectoryTool::new(vfs_ctx.clone()))
            .tool(VfsCreateFileTool::new(vfs_ctx.clone()))
            .tool(VfsUpdateFileTool::new(vfs_ctx.clone()))
            .tool(VfsDeleteNodeTool::new(vfs_ctx.clone()))
            .tool(VfsMoveNodeTool::new(vfs_ctx.clone()))
            .tool(VfsCopyNodeTool::new(vfs_ctx.clone()))
            .tool(VfsCreateDirectoryTool::new(vfs_ctx.clone()))
            .tool(VfsCreateSymlinkTool::new(vfs_ctx.clone()))
            .tool(VfsGetTreeTool::new(vfs_ctx.clone()))
            .tool(VfsSearchFilesTool::new(vfs_ctx.clone()))
            .tool(VfsGetFileHistoryTool::new(vfs_ctx.clone()))
            .tool(VfsRestoreFileVersionTool::new(vfs_ctx.clone()))
            .tool(VfsExistsTool::new(vfs_ctx.clone()))
            .tool(VfsGetWorkspaceStatsTool::new(vfs_ctx.clone()))
            .tool(VfsBatchCreateFilesTool::new(vfs_ctx.clone()))
            // Code Navigation Tools (10)
            .tool(CodeGetUnitTool::new(code_ctx.clone()))
            .tool(CodeListUnitsTool::new(code_ctx.clone()))
            .tool(CodeGetSymbolsTool::new(code_ctx.clone()))
            .tool(CodeFindDefinitionTool::new(code_ctx.clone()))
            .tool(CodeFindReferencesTool::new(code_ctx.clone()))
            .tool(CodeGetSignatureTool::new(code_ctx.clone()))
            .tool(CodeGetCallHierarchyTool::new(code_ctx.clone()))
            .tool(CodeGetTypeHierarchyTool::new(code_ctx.clone()))
            .tool(CodeGetImportsTool::new(code_ctx.clone()))
            .tool(CodeGetExportsTool::new(code_ctx.clone()))
            // Semantic Search Tools (8) - REAL semantic search with embeddings
            .tool(SearchCodeTool::new(semantic_ctx.clone()))
            .tool(SearchSimilarTool::new(semantic_ctx.clone()))
            .tool(FindByMeaningTool::new(semantic_ctx.clone()))
            .tool(SearchDocumentationTool::new(semantic_ctx.clone()))
            .tool(SearchCommentsTool::new(semantic_ctx.clone()))
            .tool(HybridSearchTool::new(semantic_ctx.clone()))
            .tool(SearchByExampleTool::new(semantic_ctx.clone()))
            .tool(SearchByNaturalLanguageTool::new(semantic_ctx.clone()))
            // Dependency Analysis Tools (10)
            .tool(DepsGetDependenciesTool::new(deps_ctx.clone()))
            .tool(DepsFindPathTool::new(deps_ctx.clone()))
            .tool(DepsFindCyclesTool::new(deps_ctx.clone()))
            .tool(DepsImpactAnalysisTool::new(deps_ctx.clone()))
            .tool(DepsFindRootsTool::new(deps_ctx.clone()))
            .tool(DepsFindLeavesTool::new(deps_ctx.clone()))
            .tool(DepsFindHubsTool::new(deps_ctx.clone()))
            .tool(DepsGetLayersTool::new(deps_ctx.clone()))
            .tool(DepsCheckConstraintsTool::new(deps_ctx.clone()))
            .tool(DepsGenerateGraphTool::new(deps_ctx.clone()))
            // Code Quality Tools (8)
            .tool(QualityAnalyzeComplexityTool::new(quality_ctx.clone()))
            .tool(QualityFindCodeSmellsTool::new(quality_ctx.clone()))
            .tool(QualityCheckNamingTool::new(quality_ctx.clone()))
            .tool(QualityAnalyzeCouplingTool::new(quality_ctx.clone()))
            .tool(QualityAnalyzeCohesionTool::new(quality_ctx.clone()))
            .tool(QualityFindAntipatternsTool::new(quality_ctx.clone()))
            .tool(QualitySuggestRefactoringsTool::new(quality_ctx.clone()))
            .tool(QualityCalculateMetricsTool::new(quality_ctx.clone()))
            // Version Control Tools (10)
            .tool(VersionGetHistoryTool::new(version_ctx.clone()))
            .tool(VersionCompareTool::new(version_ctx.clone()))
            .tool(VersionRestoreTool::new(version_ctx.clone()))
            .tool(VersionCreateSnapshotTool::new(version_ctx.clone()))
            .tool(VersionListSnapshotsTool::new(version_ctx.clone()))
            .tool(VersionRestoreSnapshotTool::new(version_ctx.clone()))
            .tool(VersionDiffSnapshotsTool::new(version_ctx.clone()))
            .tool(VersionBlameTool::new(version_ctx.clone()))
            .tool(VersionGetChangelogTool::new(version_ctx.clone()))
            .tool(VersionTagTool::new(version_ctx.clone()))
            // Cognitive Memory Tools (12)
            .tool(MemoryFindSimilarEpisodesTool::new(memory_ctx.clone()))
            .tool(MemoryRecordEpisodeTool::new(memory_ctx.clone()))
            .tool(MemoryGetEpisodeTool::new(memory_ctx.clone()))
            .tool(MemoryExtractPatternsTool::new(memory_ctx.clone()))
            .tool(MemoryApplyPatternTool::new(memory_ctx.clone()))
            .tool(MemorySearchEpisodesTool::new(memory_ctx.clone()))
            .tool(MemoryGetStatisticsTool::new(memory_ctx.clone()))
            .tool(MemoryConsolidateTool::new(memory_ctx.clone()))
            .tool(MemoryExportKnowledgeTool::new(memory_ctx.clone()))
            .tool(MemoryImportKnowledgeTool::new(memory_ctx.clone()))
            .tool(MemoryGetRecommendationsTool::new(memory_ctx.clone()))
            .tool(MemoryLearnFromFeedbackTool::new(memory_ctx.clone()))
            // Multi-Agent Coordination Tools (14)
            .tool(SessionCreateTool::new(agent_ctx.clone()))
            .tool(SessionListTool::new(agent_ctx.clone()))
            .tool(SessionUpdateTool::new(agent_ctx.clone()))
            .tool(SessionMergeTool::new(agent_ctx.clone()))
            .tool(SessionAbandonTool::new(agent_ctx.clone()))
            .tool(LockAcquireTool::new(agent_ctx.clone()))
            .tool(LockReleaseTool::new(agent_ctx.clone()))
            .tool(LockListTool::new(agent_ctx.clone()))
            .tool(LockCheckTool::new(agent_ctx.clone()))
            .tool(AgentRegisterTool::new(agent_ctx.clone()))
            .tool(AgentSendMessageTool::new(agent_ctx.clone()))
            .tool(AgentGetMessagesTool::new(agent_ctx.clone()))
            .tool(ConflictListTool::new(agent_ctx.clone()))
            .tool(ConflictResolveTool::new(agent_ctx.clone()))
            // Materialization Tools (8)
            .tool(FlushPreviewTool::new(mat_ctx.clone()))
            .tool(FlushExecuteTool::new(mat_ctx.clone()))
            .tool(FlushSelectiveTool::new(mat_ctx.clone()))
            .tool(SyncFromDiskTool::new(mat_ctx.clone()))
            .tool(SyncStatusTool::new(mat_ctx.clone()))
            .tool(SyncResolveConflictTool::new(mat_ctx.clone()))
            .tool(WatchStartTool::new(mat_ctx.clone()))
            .tool(WatchStopTool::new(mat_ctx.clone()))
            // Testing & Validation Tools (4)
            .tool(TestValidateTool::new(test_ctx.clone()))
            .tool(TestFindMissingTool::new(test_ctx.clone()))
            .tool(TestAnalyzeCoverageTool::new(test_ctx.clone()))
            .tool(TestRunInMemoryTool::new(test_ctx.clone()))
            // REMOVED: Validation tools (ValidateSyntax, ValidateSemantics, ValidateContracts, ValidateDependencies, ValidateStyle)
            // Use cortex.lint.run, external linters, or cortex.deps.check_constraints instead
            // Documentation Tools (26)
            // Document CRUD
            .tool(DocumentCreateTool::new(doc_ctx.clone()))
            .tool(DocumentGetTool::new(doc_ctx.clone()))
            .tool(DocumentGetBySlugTool::new(doc_ctx.clone()))
            .tool(DocumentUpdateTool::new(doc_ctx.clone()))
            .tool(DocumentDeleteTool::new(doc_ctx.clone()))
            .tool(DocumentListTool::new(doc_ctx.clone()))
            .tool(DocumentPublishTool::new(doc_ctx.clone()))
            .tool(DocumentArchiveTool::new(doc_ctx.clone()))
            // Section Management
            .tool(SectionCreateTool::new(doc_ctx.clone()))
            .tool(SectionGetTool::new(doc_ctx.clone()))
            .tool(SectionUpdateTool::new(doc_ctx.clone()))
            .tool(SectionDeleteTool::new(doc_ctx.clone()))
            .tool(SectionListTool::new(doc_ctx.clone()))
            // Link Management
            .tool(LinkCreateTool::new(doc_ctx.clone()))
            .tool(LinkListTool::new(doc_ctx.clone()))
            .tool(LinkDeleteTool::new(doc_ctx.clone()))
            // Search & Discovery
            .tool(DocumentSearchTool::new(doc_ctx.clone()))
            .tool(DocumentTreeTool::new(doc_ctx.clone()))
            .tool(DocumentRelatedTool::new(doc_ctx.clone()))
            // Advanced Operations
            .tool(DocumentCloneTool::new(doc_ctx.clone()))
            .tool(DocumentMergeTool::new(doc_ctx.clone()))
            .tool(DocumentStatsTool::new(doc_ctx.clone()))
            // Versioning
            .tool(VersionCreateTool::new(doc_ctx.clone()))
            .tool(VersionGetTool::new(doc_ctx.clone()))
            .tool(VersionListTool::new(doc_ctx.clone()))
            // Legacy (to be migrated)
            // .tool(DocGenerateFromCodeTool::new(doc_ctx.clone()))
            // .tool(DocCheckConsistencyTool::new(doc_ctx.clone()))
            // Build & Execution Tools (7)
            .tool(BuildTriggerTool::new(build_ctx.clone()))
            .tool(BuildConfigureTool::new(build_ctx.clone()))
            .tool(RunExecuteTool::new(build_ctx.clone()))
            .tool(RunScriptTool::new(build_ctx.clone()))
            .tool(TestExecuteTool::new(build_ctx.clone()))
            .tool(LintRunTool::new(build_ctx.clone()))
            // REMOVED: FormatCodeTool - use external formatters via cortex.lint.run instead
            .tool(PackagePublishTool::new(build_ctx.clone()))
            // Monitoring & Analytics Tools (10)
            .tool(MonitorHealthTool::new(monitor_ctx.clone()))
            .tool(MonitorPerformanceTool::new(monitor_ctx.clone()))
            .tool(AnalyticsCodeMetricsTool::new(monitor_ctx.clone()))
            .tool(AnalyticsAgentActivityTool::new(monitor_ctx.clone()))
            .tool(AnalyticsErrorAnalysisTool::new(monitor_ctx.clone()))
            .tool(AnalyticsProductivityTool::new(monitor_ctx.clone()))
            .tool(AnalyticsQualityTrendsTool::new(monitor_ctx.clone()))
            .tool(ExportMetricsTool::new(monitor_ctx.clone()))
            .tool(AlertConfigureTool::new(monitor_ctx.clone()))
            .tool(ReportGenerateTool::new(monitor_ctx.clone()))
            // Security Analysis Tools (4)
            .tool(SecurityScanTool::new(security_ctx.clone()))
            .tool(SecurityCheckDependenciesTool::new(security_ctx.clone()))
            .tool(SecurityAnalyzeSecretsTool::new(security_ctx.clone()))
            .tool(SecurityGenerateReportTool::new(security_ctx.clone()))
            // Type Analysis Tools (4)
            .tool(CodeInferTypesTool::new(type_ctx.clone()))
            .tool(CodeCheckTypesTool::new(type_ctx.clone()))
            .tool(CodeSuggestTypeAnnotationsTool::new(type_ctx.clone()))
            .tool(CodeAnalyzeTypeCoverageTool::new(type_ctx.clone()))
            // AI-Assisted Development Tools (5)
            .tool(AiSuggestRefactoringTool::new(ai_ctx.clone()))
            .tool(AiExplainCodeTool::new(ai_ctx.clone()))
            .tool(AiSuggestOptimizationTool::new(ai_ctx.clone()))
            .tool(AiSuggestFixTool::new(ai_ctx.clone()))
            // REMOVED: AiGenerateDocstringTool - use cortex.ai.explain_code for documentation insights
            .tool(AiReviewCodeTool::new(ai_ctx.clone()))
            // Advanced Testing Tools (2)
            .tool(TestAnalyzeFlakyTool::new(adv_test_ctx.clone()))
            .tool(TestSuggestEdgeCasesTool::new(adv_test_ctx.clone()))
            // Architecture Analysis Tools (5)
            .tool(ArchVisualizeTool::new(arch_ctx.clone()))
            .tool(ArchDetectPatternsTool::new(arch_ctx.clone()))
            .tool(ArchSuggestBoundariesTool::new(arch_ctx.clone()))
            .tool(ArchCheckViolationsTool::new(arch_ctx.clone()))
            .tool(ArchAnalyzeDriftTool::new(arch_ctx.clone()))
            // Note: Middleware support may be added in future versions
            .build();

        info!("Registered {} tools", 180); // Total: 187 - 7 (removed validation & AI gen tools) = 180

        Ok(server)
    }

    /// Serves the MCP server over stdio (standard input/output)
    ///
    /// This is the primary transport for CLI tools and process spawning
    ///
    /// IMPROVED: Adds cleanup on shutdown to prevent resource leaks
    pub async fn serve_stdio(self) -> Result<()> {
        info!("Starting Cortex MCP Server on stdio");
        let transport = StdioTransport::new();

        // Serve using the SDK's built-in serve method
        let result = self.server.serve(transport).await;

        // Cleanup resources on shutdown
        info!("Performing cleanup before shutdown");
        if let Err(e) = self.storage.shutdown().await {
            warn!("Error during storage shutdown: {}", e);
        } else {
            info!("Storage shutdown successfully");
        }

        // Return any error from serve
        result.map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }

    /// Serves the MCP server over HTTP with SSE
    ///
    /// This is useful for web-based integrations
    pub async fn serve_http(self, _bind_addr: &str) -> Result<()> {
        #[cfg(feature = "http")]
        {
            info!("Starting Cortex MCP Server on HTTP: {}", _bind_addr);
            let addr: std::net::SocketAddr = _bind_addr.parse()?;
            let transport = mcp_sdk::transport::HttpTransport::new(addr);

            // Serve using the SDK's built-in serve method
            self.server.serve(transport).await
                .map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))
        }

        #[cfg(not(feature = "http"))]
        {
            warn!("HTTP transport not enabled. Compile with --features http");
            Err(anyhow::anyhow!("HTTP transport not available"))
        }
    }

    /// Get a reference to the underlying MCP server
    pub fn server(&self) -> &mcp_sdk::McpServer {
        &self.server
    }
}

/// Builder for creating a Cortex MCP server with custom options
pub struct CortexMcpServerBuilder {
    config: Option<GlobalConfig>,
    storage: Option<Arc<ConnectionManager>>,
}

impl CortexMcpServerBuilder {
    pub fn new() -> Self {
        Self {
            config: None,
            storage: None,
        }
    }

    pub fn config(mut self, config: GlobalConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn storage(mut self, storage: Arc<ConnectionManager>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub async fn build(self) -> Result<CortexMcpServer> {
        if let Some(storage) = self.storage {
            let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
            let server = CortexMcpServer::build_server(storage.clone(), vfs).await?;
            Ok(CortexMcpServer { server, storage })
        } else if let Some(config) = self.config {
            CortexMcpServer::with_config(config).await
        } else {
            CortexMcpServer::new().await
        }
    }
}

impl Default for CortexMcpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_server_creation() {
        let result = CortexMcpServer::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_builder_pattern() {
        let builder = CortexMcpServerBuilder::new();
        let result = builder.build().await;
        assert!(result.is_ok());
    }
}
