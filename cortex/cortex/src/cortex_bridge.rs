//! Bridge module for Cortex types and functionality
//!
//! This module re-exports types from cortex-types and provides the
//! CortexBridge implementation for communicating with the Cortex REST API.

// Re-export types from cortex-types
pub use cortex_types::{AgentId, SessionId, WorkspaceId, AgentType, AgentStatus};

// Re-export Episode and Pattern from cortex-intelligence
pub use cortex_intelligence::{Episode, EpisodeOutcome, Pattern, UnitFilters};

use cortex_core::error::{CortexError, Result};
use cortex_core::types::CodeUnit;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the CortexBridge
#[derive(Debug, Clone)]
pub struct CortexConfig {
    pub base_url: String,
    pub api_version: String,
    pub auth_token: Option<String>,

    // Performance
    pub cache_size_mb: usize,
    pub cache_ttl_seconds: u64,
    pub connection_pool_size: usize,

    // Reliability
    pub request_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,

    // Real-time
    pub enable_websocket: bool,
    pub reconnect_websocket: bool,
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080".to_string(),
            api_version: "v1".to_string(),
            auth_token: None,
            cache_size_mb: 100,
            cache_ttl_seconds: 3600,
            connection_pool_size: 10,
            request_timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_websocket: true,
            reconnect_websocket: true,
        }
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Session scope - defines which paths are accessible in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScope {
    pub paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}

impl Default for SessionScope {
    fn default() -> Self {
        Self {
            paths: vec![],
            read_only_paths: vec![],
        }
    }
}

/// Merge strategy for sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    Auto,
    Manual,
    Theirs,
    Mine,
}

impl ToString for MergeStrategy {
    fn to_string(&self) -> String {
        match self {
            MergeStrategy::Auto => "auto".to_string(),
            MergeStrategy::Manual => "manual".to_string(),
            MergeStrategy::Theirs => "theirs".to_string(),
            MergeStrategy::Mine => "mine".to_string(),
        }
    }
}

/// Report from merging a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReport {
    pub changes_merged: u32,
    pub conflicts_resolved: u32,
    pub new_version: u64,
}

/// Search result from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub unit_id: String,
    pub unit_type: String,
    pub name: String,
    pub qualified_name: String,
    pub signature: String,
    pub relevance_score: f32,
    pub file: String,
    pub snippet: String,
}

/// Health status of Cortex server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Search filters for semantic search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub workspace_id: Option<String>,
    pub session_id: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
    pub types: Vec<String>,
    pub languages: Vec<String>,
    pub visibility: Option<String>,
    pub min_relevance: Option<f32>,
}

/// Episode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeType {
    Task,
    Query,
    Learning,
    Error,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    Code,
    Architecture,
    Bug,
    Optimization,
}

/// Unit filters for code units
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitFilters {
    pub workspace_id: Option<String>,
    pub language: Option<String>,
    pub unit_type: Option<String>,
    pub min_score: Option<f32>,
    pub limit: Option<usize>,
}

/// Token usage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

// ============================================================================
// Memory Cache (Stub)
// ============================================================================

/// Simple in-memory cache (stub implementation)
pub struct MemoryCache {
    store: HashMap<String, Vec<u8>>,
    _ttl: Duration,
}

impl MemoryCache {
    pub fn new(_max_size_bytes: usize, ttl: Duration) -> Self {
        Self {
            store: HashMap::new(),
            _ttl: ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.store.get(key).cloned()
    }

    pub fn insert(&mut self, key: String, value: Vec<u8>) {
        self.store.insert(key, value);
    }

    pub fn invalidate_pattern(&mut self, pattern: &str) {
        let prefix = pattern.trim_end_matches('*');
        self.store.retain(|k, _| !k.starts_with(prefix));
    }
}

// ============================================================================
// Connection Pool (Stub)
// ============================================================================

/// Connection pool manager (stub implementation)
#[derive(Clone)]
pub struct ConnectionPool {
    _size: usize,
}

impl ConnectionPool {
    pub fn new(size: usize) -> Self {
        Self { _size: size }
    }
}

// ============================================================================
// API Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct CreateSessionRequest {
    agent_id: String,
    workspace_id: String,
    scope: SessionScopeRequest,
    isolation_level: String,
    ttl_seconds: u64,
}

#[derive(Debug, Serialize)]
struct SessionScopeRequest {
    paths: Vec<String>,
    read_only_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateSessionResponse {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct MergeSessionRequest {
    strategy: String,
    conflict_resolution: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct MergeReportResponse {
    changes_merged: u32,
    conflicts_resolved: u32,
    new_version: u64,
}

#[derive(Debug, Serialize)]
struct SemanticSearchRequest {
    query: String,
    workspace_id: Option<String>,
    filters: SearchFiltersRequest,
    limit: usize,
}

#[derive(Debug, Serialize)]
struct SearchFiltersRequest {
    types: Vec<String>,
    languages: Vec<String>,
    visibility: Option<String>,
    min_relevance: f32,
}

#[derive(Debug, Deserialize)]
struct SemanticSearchResponse {
    results: Vec<SearchResult>,
}

/// Generic API response wrapper
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

// ============================================================================
// CortexBridge Implementation
// ============================================================================

/// Bridge to Cortex REST API
pub struct CortexBridge {
    /// Core HTTP client
    client: HttpClient,
    base_url: String,

    /// Performance optimization
    cache: Arc<RwLock<MemoryCache>>,
    _connection_pool: ConnectionPool,

    /// Session management
    active_sessions: Arc<RwLock<HashMap<AgentId, SessionId>>>,

    /// Configuration
    config: CortexConfig,
}

impl CortexBridge {
    /// Create new CortexBridge and connect to Cortex
    pub async fn new(config: CortexConfig) -> Result<Self> {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .pool_max_idle_per_host(config.connection_pool_size)
            .build()
            .map_err(|e| CortexError::internal(format!("Failed to create HTTP client: {}", e)))?;

        let cache = Arc::new(RwLock::new(MemoryCache::new(
            config.cache_size_mb * 1024 * 1024,
            Duration::from_secs(config.cache_ttl_seconds),
        )));

        let base_url = format!("{}/api/{}", config.base_url, config.api_version);

        let bridge = Self {
            client,
            base_url,
            cache,
            _connection_pool: ConnectionPool::new(config.connection_pool_size),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        // Verify connection
        bridge.ensure_initialized().await?;

        Ok(bridge)
    }

    /// Verify Cortex server is reachable
    pub async fn ensure_initialized(&self) -> Result<()> {
        match self.health_check().await {
            Ok(status) => {
                tracing::info!("Connected to Cortex server: {} ({})", status.version, status.status);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to connect to Cortex server: {}", e);
                Err(CortexError::connection(format!("Cortex server unreachable: {}", e)))
            }
        }
    }

    /// Health check to verify Cortex is reachable
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| CortexError::connection(format!("Health check failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(CortexError::connection(format!(
                "Health check failed: HTTP {}",
                response.status()
            )));
        }

        let health: HealthStatus = response.json().await
            .map_err(|e| CortexError::serialization(format!("Failed to parse health response: {}", e)))?;

        Ok(health)
    }

    /// Create isolated session for agent
    pub async fn create_session(
        &self,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        scope: SessionScope,
    ) -> Result<SessionId> {
        let request = CreateSessionRequest {
            agent_id: agent_id.to_string(),
            workspace_id: workspace_id.to_string(),
            scope: SessionScopeRequest {
                paths: scope.paths.clone(),
                read_only_paths: scope.read_only_paths.clone(),
            },
            isolation_level: "snapshot".to_string(),
            ttl_seconds: 3600,
        };

        let response = self.client
            .post(&format!("{}/sessions", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| CortexError::connection(format!("Failed to create session: {}", e)))?;

        let session: CreateSessionResponse = Self::unwrap_response(response).await?;
        let session_id = SessionId::from(session.session_id);

        // Track active session
        self.active_sessions.write().await.insert(agent_id.clone(), session_id.clone());

        tracing::info!("Created session {} for agent {}", session_id, agent_id);
        Ok(session_id)
    }

    /// Merge session changes back to workspace
    pub async fn merge_session(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeReport> {
        let request = MergeSessionRequest {
            strategy: strategy.to_string(),
            conflict_resolution: None,
        };

        let response = self.client
            .post(&format!("{}/sessions/{}/merge", self.base_url, session_id))
            .json(&request)
            .send()
            .await
            .map_err(|e| CortexError::connection(format!("Failed to merge session: {}", e)))?;

        let report: MergeReportResponse = Self::unwrap_response(response).await?;

        tracing::info!(
            "Merged session {} with {} conflicts resolved",
            session_id,
            report.conflicts_resolved
        );

        Ok(MergeReport {
            changes_merged: report.changes_merged,
            conflicts_resolved: report.conflicts_resolved,
            new_version: report.new_version,
        })
    }

    /// Semantic code search
    pub async fn semantic_search(
        &self,
        query: &str,
        workspace_id: &WorkspaceId,
        filters: SearchFilters,
    ) -> Result<Vec<SearchResult>> {
        let request = SemanticSearchRequest {
            query: query.to_string(),
            workspace_id: Some(workspace_id.to_string()),
            filters: SearchFiltersRequest {
                types: filters.types.clone(),
                languages: filters.languages.clone(),
                visibility: filters.visibility.clone(),
                min_relevance: filters.min_relevance.unwrap_or(0.7),
            },
            limit: filters.limit.unwrap_or(20),
        };

        let response = self.client
            .post(&format!("{}/search/semantic", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| CortexError::connection(format!("Failed to search: {}", e)))?;

        let result: SemanticSearchResponse = Self::unwrap_response(response).await?;

        Ok(result.results)
    }

    /// Close and cleanup session
    pub async fn close_session(
        &self,
        session_id: &SessionId,
    ) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/sessions/{}", self.base_url, session_id))
            .send()
            .await
            .map_err(|e| CortexError::connection(format!("Failed to close session: {}", e)))?;

        if !response.status().is_success() {
            tracing::warn!("Failed to close session {}: {}", session_id, response.status());
        }

        tracing::info!("Closed session {}", session_id);
        Ok(())
    }

    /// Unwrap Cortex API response envelope
    /// All Cortex API responses are wrapped in: { success: bool, data: T, error: Option<String> }
    async fn unwrap_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CortexError::internal(format!(
                "HTTP {}: {}",
                status,
                error_text
            )));
        }

        let envelope: ApiResponse<T> = response.json().await
            .map_err(|e| CortexError::serialization(format!("Failed to parse response: {}", e)))?;

        if !envelope.success {
            return Err(CortexError::internal(
                envelope.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        envelope.data.ok_or_else(|| {
            CortexError::internal("Missing data in response".to_string())
        })
    }

    /// Get the base URL for API requests
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get a reference to the HTTP client
    pub fn client(&self) -> &HttpClient {
        &self.client
    }

    /// Query the knowledge graph (stub implementation)
    pub async fn query_graph(
        &self,
        _query: &str,
        _params: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>> {
        // TODO: Implement actual graph query
        Ok(Vec::new())
    }

    /// Search for patterns (stub implementation)
    pub async fn search_patterns(
        &self,
        _query: &str,
        _limit: Option<usize>,
    ) -> Result<Vec<Pattern>> {
        // TODO: Implement actual pattern search
        Ok(Vec::new())
    }

    /// Get code units for a workspace (stub implementation)
    pub async fn get_code_units(
        &self,
        _workspace_id: &WorkspaceId,
        _filters: UnitFilters,
    ) -> Result<Vec<CodeUnit>> {
        // TODO: Implement actual code unit retrieval
        Ok(Vec::new())
    }

    /// Write file to workspace (stub implementation)
    pub async fn write_file(
        &self,
        _session_id: &SessionId,
        _file_path: &str,
        _content: &str,
    ) -> Result<()> {
        // TODO: Implement actual file write
        Ok(())
    }
}

impl Default for CortexBridge {
    fn default() -> Self {
        // Create a minimal bridge for compatibility
        let config = CortexConfig::default();
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .build()
            .expect("Failed to create default HTTP client");

        Self {
            client,
            base_url: format!("{}/api/{}", config.base_url, config.api_version),
            cache: Arc::new(RwLock::new(MemoryCache::new(
                config.cache_size_mb * 1024 * 1024,
                Duration::from_secs(config.cache_ttl_seconds),
            ))),
            _connection_pool: ConnectionPool::new(config.connection_pool_size),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
}
