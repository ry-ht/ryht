# Axon: Cortex Integration - Complete Specification

## Overview

The Cortex integration is the central nervous system of Axon, providing persistent memory, session isolation, and shared knowledge across all agents. This document specifies every integration point between Axon (orchestration) and Cortex (data layer).

**Architectural Principle**: Axon orchestrates, Cortex persists. Agents are stateless executors that rely on Cortex for all data operations, memory, and learning.

## Integration Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Axon Desktop App                          │
│                    (Tauri + React + Rust)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │               Agent Orchestration Layer                 │    │
│  │                                                          │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │    │
│  │  │Developer │  │ Reviewer │  │  Tester  │  │ Agents │ │    │
│  │  │  Agent   │  │  Agent   │  │  Agent   │  │  ...   │ │    │
│  │  └─────┬────┘  └─────┬────┘  └─────┬────┘  └────┬───┘ │    │
│  │        │             │             │            │      │    │
│  │        └─────────────┼─────────────┼────────────┘      │    │
│  └──────────────────────┼─────────────┼───────────────────┘    │
│                         │             │                         │
│  ┌──────────────────────▼─────────────▼───────────────────┐    │
│  │               CortexBridge Layer                        │    │
│  │                                                          │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │   Session    │  │    Memory    │  │   Event      │ │    │
│  │  │   Manager    │  │    Cache     │  │   Stream     │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │    │
│  │                                                          │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │   HTTP       │  │  WebSocket   │  │  Connection  │ │    │
│  │  │   Client     │  │   Client     │  │     Pool     │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │    │
│  └──────────────────────┬─────────────────────────────────┘    │
└─────────────────────────┼──────────────────────────────────────┘
                          │
                  REST API + WebSocket
                          │
┌─────────────────────────▼──────────────────────────────────────┐
│                    Cortex REST API                              │
│                    (Data & Memory Layer)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  API Endpoints                            │  │
│  │                                                           │  │
│  │  /sessions      /memory        /workspaces      │  │
│  │  /files         /tasks         /search          │  │
│  │  /units         /analysis      /locks           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Cognitive Memory Core                        │  │
│  │                                                           │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐ │  │
│  │  │  Virtual   │  │  Episodic  │  │   Knowledge        │ │  │
│  │  │Filesystem  │  │   Memory   │  │      Graph         │ │  │
│  │  └────────────┘  └────────────┘  └────────────────────┘ │  │
│  │                                                           │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐ │  │
│  │  │  Sessions  │  │    Lock    │  │    Semantic        │ │  │
│  │  │  & Merge   │  │  Manager   │  │     Search         │ │  │
│  │  └────────────┘  └────────────┘  └────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│                   SurrealDB Persistence                          │
└──────────────────────────────────────────────────────────────────┘
```

## CortexBridge: Complete Implementation

### Core Structure

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::Client as HttpClient;
use tungstenite::WebSocket;

pub struct CortexBridge {
    // Core HTTP client
    client: HttpClient,
    base_url: String,

    // Performance optimization
    cache: Arc<RwLock<MemoryCache>>,
    connection_pool: ConnectionPool,

    // Session management
    active_sessions: Arc<RwLock<HashMap<AgentId, SessionId>>>,
    session_manager: SessionManager,

    // Real-time coordination
    websocket: Arc<RwLock<Option<WebSocketClient>>>,
    event_stream: Arc<EventStream>,

    // Metrics & monitoring
    metrics: Arc<BridgeMetrics>,

    // Configuration
    config: CortexConfig,
}

#[derive(Debug, Clone)]
pub struct CortexConfig {
    pub base_url: String,              // http://localhost:8081
    pub api_version: String,           // v3
    pub auth_token: Option<String>,

    // Performance
    pub cache_size_mb: usize,          // 100
    pub cache_ttl_seconds: u64,        // 3600
    pub connection_pool_size: usize,   // 10

    // Reliability
    pub request_timeout_secs: u64,     // 30
    pub max_retries: u32,              // 3
    pub retry_delay_ms: u64,           // 1000

    // Real-time
    pub enable_websocket: bool,        // true
    pub reconnect_websocket: bool,     // true
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8081".to_string(),
            api_version: "v3".to_string(),
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
```

### Initialization and Connection

```rust
impl CortexBridge {
    /// Create new CortexBridge and connect to Cortex
    pub async fn new(config: CortexConfig) -> Result<Self> {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .pool_max_idle_per_host(config.connection_pool_size)
            .build()?;

        let cache = Arc::new(RwLock::new(MemoryCache::new(
            config.cache_size_mb * 1024 * 1024, // Convert to bytes
            Duration::from_secs(config.cache_ttl_seconds),
        )));

        let mut bridge = Self {
            client,
            base_url: format!("{}/{}", config.base_url, config.api_version),
            cache,
            connection_pool: ConnectionPool::new(config.connection_pool_size),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            session_manager: SessionManager::new(),
            websocket: Arc::new(RwLock::new(None)),
            event_stream: Arc::new(EventStream::new()),
            metrics: Arc::new(BridgeMetrics::new()),
            config,
        };

        // Verify connection
        bridge.health_check().await?;

        // Connect WebSocket if enabled
        if bridge.config.enable_websocket {
            bridge.connect_websocket().await?;
        }

        Ok(bridge)
    }

    /// Connect to Cortex WebSocket for real-time events
    async fn connect_websocket(&mut self) -> Result<()> {
        let ws_url = self.config.base_url.replace("http", "ws");
        let ws_client = WebSocketClient::connect(&format!("{}/ws", ws_url)).await?;

        *self.websocket.write().await = Some(ws_client);

        // Start event handler
        self.start_event_handler().await;

        info!("WebSocket connected to Cortex at {}", ws_url);
        Ok(())
    }

    /// Health check to verify Cortex is reachable
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let response = self.client
            .get(&format!("{}/health", self.base_url))
            .send()
            .await
            .map_err(|e| Error::CortexUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::CortexUnavailable(
                format!("Health check failed: {}", response.status())
            ));
        }

        let health: HealthStatus = response.json().await?;
        Ok(health)
    }

    /// Unwrap Cortex API response envelope
    /// All Cortex API responses are wrapped in: { success: bool, data: T, error: Option<String> }
    async fn unwrap_response<T: DeserializeOwned>(response: Response) -> Result<T> {
        #[derive(Deserialize)]
        struct ApiResponse<T> {
            success: bool,
            data: Option<T>,
            error: Option<String>,
        }

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::CortexError(format!("HTTP {}: {}", status, error_text)));
        }

        let envelope: ApiResponse<T> = response.json().await
            .map_err(|e| Error::CortexError(format!("Failed to parse response: {}", e)))?;

        if !envelope.success {
            return Err(Error::CortexError(
                envelope.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        envelope.data.ok_or_else(|| Error::CortexError("Missing data in response".to_string()))
    }
}
```

## Session Management - Complete API

### Session Creation

```rust
impl CortexBridge {
    /// POST /sessions - Create isolated session for agent
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

        let start = Instant::now();
        let response = self.client
            .post(&format!("{}/sessions", self.base_url))
            .json(&request)
            .send()
            .await?;

        let session: CreateSessionResponse = Self::unwrap_response(response).await?;
        let session_id = SessionId::from(session.session_id);

        // Track active session
        self.active_sessions.write().await.insert(agent_id.clone(), session_id.clone());

        // Subscribe to session events via WebSocket
        if let Some(ws) = self.websocket.read().await.as_ref() {
            ws.subscribe(format!("session:{}", session_id)).await?;
        }

        // Metrics
        self.metrics.sessions_created.inc();
        self.metrics.record_request_duration("create_session", start.elapsed());

        info!("Created session {} for agent {}", session_id, agent_id);
        Ok(session_id)
    }

    /// GET /sessions/{id} - Get session status
    pub async fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus> {
        let response = self.client
            .get(&format!("{}/sessions/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::SessionNotFound(session_id.clone()));
        }

        let status: SessionStatusResponse = response.json().await?;
        Ok(status.into())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionScope {
    pub paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateSessionRequest {
    agent_id: String,
    workspace_id: String,
    scope: SessionScopeRequest,
    isolation_level: String,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SessionScopeRequest {
    paths: Vec<String>,
    read_only_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateSessionResponse {
    session_id: String,
    token: String,
    expires_at: String,
    base_version: u64,
}
```

### File Operations in Sessions

```rust
impl CortexBridge {
    /// GET /sessions/{id}/files/{path} - Read file from session
    pub async fn read_file_in_session(
        &self,
        session_id: &SessionId,
        path: &str,
    ) -> Result<String> {
        let encoded_path = urlencoding::encode(path);
        let response = self.client
            .get(&format!("{}/sessions/{}/files/{}",
                self.base_url, session_id, encoded_path))
            .send()
            .await?;

        let file_content: FileContentResponse = Self::unwrap_response(response).await?;

        self.metrics.files_read.inc();
        Ok(file_content.content)
    }

    /// PUT /sessions/{id}/files/{path} - Write file to session
    pub async fn write_file_in_session(
        &self,
        session_id: &SessionId,
        path: &str,
        content: &str,
    ) -> Result<()> {
        let request = UpdateFileRequest {
            content: content.to_string(),
            expected_version: None,
        };

        let encoded_path = urlencoding::encode(path);
        let response = self.client
            .put(&format!("{}/sessions/{}/files/{}",
                self.base_url, session_id, encoded_path))
            .json(&request)
            .send()
            .await?;

        // Unwrap response (returns empty data for successful write)
        let _: serde_json::Value = Self::unwrap_response(response).await?;

        self.metrics.files_written.inc();
        info!("Wrote file {} to session {}", path, session_id);
        Ok(())
    }

    /// GET /sessions/{id}/files - List files in session
    pub async fn list_files_in_session(
        &self,
        session_id: &SessionId,
        path: &str,
    ) -> Result<Vec<FileInfo>> {
        let response = self.client
            .get(&format!("{}/sessions/{}/files", self.base_url, session_id))
            .query(&[("path", path), ("recursive", "true")])
            .send()
            .await?;

        let files: ListFilesResponse = response.json().await?;
        Ok(files.files)
    }
}

#[derive(Debug, Clone, Serialize)]
struct UpdateFileRequest {
    content: String,
    expected_version: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct FileContentResponse {
    content: String,
    encoding: String,
    version: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ListFilesResponse {
    files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub file_type: String,
    pub size_bytes: u64,
    pub modified_at: String,
}
```

### Session Merge

```rust
impl CortexBridge {
    /// POST /sessions/{id}/merge - Merge session changes back to workspace
    pub async fn merge_session(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeReport> {
        let request = MergeSessionRequest {
            strategy: strategy.to_string(),
            conflict_resolution: None,
        };

        let start = Instant::now();
        let response = self.client
            .post(&format!("{}/sessions/{}/merge", self.base_url, session_id))
            .json(&request)
            .send()
            .await?;

        let report: MergeReportResponse = Self::unwrap_response(response).await?;

        // Metrics
        self.metrics.successful_merges.inc();
        if report.conflicts_resolved > 0 {
            self.metrics.merge_conflicts.add(report.conflicts_resolved as u64);
        }
        self.metrics.record_request_duration("merge_session", start.elapsed());

        info!("Merged session {} with {} conflicts resolved",
            session_id, report.conflicts_resolved);

        Ok(MergeReport {
            changes_merged: report.changes_merged,
            conflicts_resolved: report.conflicts_resolved,
            new_version: report.new_version,
        })
    }

    /// DELETE /sessions/{id} - Close and cleanup session
    pub async fn close_session(
        &self,
        session_id: &SessionId,
        agent_id: &AgentId,
    ) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/sessions/{}", self.base_url, session_id))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to close session {}: {}", session_id, response.status());
        }

        // Cleanup tracking
        self.active_sessions.write().await.remove(agent_id);

        // Unsubscribe from events
        if let Some(ws) = self.websocket.read().await.as_ref() {
            ws.unsubscribe(format!("session:{}", session_id)).await?;
        }

        info!("Closed session {} for agent {}", session_id, agent_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Serialize)]
struct MergeSessionRequest {
    strategy: String,
    conflict_resolution: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct MergeReportResponse {
    changes_merged: u32,
    conflicts_resolved: u32,
    new_version: u64,
}

#[derive(Debug, Clone)]
pub struct MergeReport {
    pub changes_merged: u32,
    pub conflicts_resolved: u32,
    pub new_version: u64,
}
```

## Memory & Episodic Learning

### Episode Management

```rust
impl CortexBridge {
    /// POST /memory/episodes - Store development episode
    pub async fn store_episode(&self, episode: Episode) -> Result<EpisodeId> {
        let request = CreateEpisodeRequest {
            episode_type: format!("{:?}", episode.episode_type).to_lowercase(),
            task_description: episode.task_description,
            agent_id: episode.agent_id,
            session_id: episode.session_id,
            workspace_id: episode.workspace_id,
            entities_created: episode.entities_created,
            entities_modified: episode.entities_modified,
            entities_deleted: episode.entities_deleted,
            files_touched: episode.files_touched,
            queries_made: episode.queries_made,
            tools_used: episode.tools_used,
            solution_summary: episode.solution_summary,
            outcome: format!("{:?}", episode.outcome).to_lowercase(),
            success_metrics: episode.success_metrics,
            errors_encountered: episode.errors_encountered,
            lessons_learned: episode.lessons_learned,
            duration_seconds: episode.duration_seconds,
            tokens_used: episode.tokens_used,
        };

        let response = self.client
            .post(&format!("{}/memory/episodes", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: CreateEpisodeResponse = Self::unwrap_response(response).await?;
        let episode_id = EpisodeId::from(result.episode_id);

        // Invalidate related caches
        self.cache.write().await.invalidate_pattern("episodes:*");

        self.metrics.episodes_stored.inc();
        info!("Stored episode {}", episode_id);

        Ok(episode_id)
    }

    /// POST /memory/search - Search for similar episodes
    pub async fn search_episodes(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Episode>> {
        // Check cache first
        let cache_key = format!("episodes:{}", query);
        if let Some(cached) = self.cache.read().await.get(&cache_key) {
            self.metrics.cache_hits.inc();
            return Ok(serde_json::from_slice(&cached)?);
        }

        let request = SearchEpisodesRequest {
            query: query.to_string(),
            limit,
            min_similarity: 0.7,
        };

        let response = self.client
            .post(&format!("{}/memory/search", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: SearchEpisodesResponse = Self::unwrap_response(response).await?;

        // Cache results
        let cached_data = serde_json::to_vec(&result.episodes)?;
        self.cache.write().await.insert(cache_key, cached_data);

        self.metrics.cache_misses.inc();
        Ok(result.episodes)
    }

    /// GET /memory/patterns - Retrieve learned patterns
    pub async fn get_patterns(&self) -> Result<Vec<Pattern>> {
        // Check cache
        let cache_key = "patterns:all";
        if let Some(cached) = self.cache.read().await.get(cache_key) {
            self.metrics.cache_hits.inc();
            return Ok(serde_json::from_slice(&cached)?);
        }

        let response = self.client
            .get(&format!("{}/memory/patterns", self.base_url))
            .send()
            .await?;

        let result: PatternsResponse = Self::unwrap_response(response).await?;

        // Cache patterns
        let cached_data = serde_json::to_vec(&result.patterns)?;
        self.cache.write().await.insert(cache_key.to_string(), cached_data);

        self.metrics.cache_misses.inc();
        Ok(result.patterns)
    }
}

//
// CANONICAL EPISODE STRUCTURE - MATCHES CORTEX DATABASE SCHEMA
// Source of truth: docs/spec/cortex-system/02-data-model.md (episode table)
//
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    // Identity
    pub id: String,
    pub episode_type: EpisodeType,

    // Context
    pub task_description: String,
    pub agent_id: String,
    pub session_id: Option<String>,  // record<session>
    pub workspace_id: String,        // record<workspace>

    // Work performed
    pub entities_created: Vec<String>,
    pub entities_modified: Vec<String>,
    pub entities_deleted: Vec<String>,
    pub files_touched: Vec<String>,
    pub queries_made: Vec<String>,
    pub tools_used: Vec<ToolUsage>,

    // Outcome
    pub solution_summary: String,
    pub outcome: EpisodeOutcome,
    pub success_metrics: serde_json::Value,  // object
    pub errors_encountered: Vec<String>,
    pub lessons_learned: Vec<String>,

    // Performance
    pub duration_seconds: i32,
    pub tokens_used: TokenUsage,

    // Semantic representation
    pub embedding: Vec<f32>,  // array<float> - dimension 1536

    // Versioning
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeType {
    Task,
    Refactor,
    Bugfix,
    Feature,
    Exploration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeOutcome {
    Success,
    Partial,
    Failure,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub tool_name: String,
    pub invocations: u32,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub total: u64,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            input: 0,
            output: 0,
            total: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct CreateEpisodeRequest {
    episode_type: String,
    task_description: String,
    agent_id: String,
    session_id: Option<String>,
    workspace_id: String,
    entities_created: Vec<String>,
    entities_modified: Vec<String>,
    entities_deleted: Vec<String>,
    files_touched: Vec<String>,
    queries_made: Vec<String>,
    tools_used: Vec<ToolUsage>,
    solution_summary: String,
    outcome: String,
    success_metrics: serde_json::Value,
    errors_encountered: Vec<String>,
    lessons_learned: Vec<String>,
    duration_seconds: i32,
    tokens_used: TokenUsage,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateEpisodeResponse {
    episode_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct SearchEpisodesRequest {
    query: String,
    limit: usize,
    min_similarity: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchEpisodesResponse {
    episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Deserialize)]
struct PatternsResponse {
    patterns: Vec<Pattern>,
}

//
// CANONICAL PATTERN STRUCTURE - MATCHES CORTEX DATABASE SCHEMA
// Source of truth: docs/spec/cortex-system/02-data-model.md (pattern table)
//
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    // Identity
    pub id: String,
    pub pattern_type: PatternType,

    // Description
    pub name: String,
    pub description: String,
    pub context: String,

    // Pattern definition
    pub before_state: serde_json::Value,  // object
    pub after_state: serde_json::Value,   // object
    pub transformation: serde_json::Value, // object

    // Usage statistics
    pub times_applied: i32,
    pub success_rate: f32,
    pub average_improvement: serde_json::Value, // object

    // Examples
    pub example_episodes: Vec<String>,  // array<record<episode>>

    // Semantic search
    pub embedding: Vec<f32>,  // array<float> - dimension 1536
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatternType {
    Code,
    Architecture,
    Refactor,
    Optimization,
    ErrorRecovery,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            id: String::new(),
            pattern_type: PatternType::Code,
            name: String::new(),
            description: String::new(),
            context: String::new(),
            before_state: serde_json::Value::Object(Default::default()),
            after_state: serde_json::Value::Object(Default::default()),
            transformation: serde_json::Value::Object(Default::default()),
            times_applied: 0,
            success_rate: 0.0,
            average_improvement: serde_json::Value::Object(Default::default()),
            example_episodes: Vec::new(),
            embedding: Vec::new(),
        }
    }
}
```

## Code Search & Analysis

### Semantic Search

```rust
impl CortexBridge {
    /// POST /search/semantic - Semantic code search
    pub async fn semantic_search(
        &self,
        query: &str,
        workspace_id: &WorkspaceId,
        filters: SearchFilters,
    ) -> Result<Vec<CodeSearchResult>> {
        let request = SemanticSearchRequest {
            query: query.to_string(),
            workspace_id: Some(workspace_id.to_string()),
            filters: SearchFiltersRequest {
                types: filters.types.clone(),
                languages: filters.languages.clone(),
                visibility: filters.visibility.clone(),
                min_relevance: filters.min_relevance,
            },
            limit: 20,
        };

        let response = self.client
            .post(&format!("{}/search/semantic", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: SemanticSearchResponse = Self::unwrap_response(response).await?;

        self.metrics.semantic_searches.inc();
        Ok(result.results)
    }

    /// GET /workspaces/{id}/units - Get code units
    pub async fn get_code_units(
        &self,
        workspace_id: &WorkspaceId,
        filters: UnitFilters,
    ) -> Result<Vec<CodeUnit>> {
        let mut query_params = vec![];

        if let Some(unit_type) = &filters.unit_type {
            query_params.push(("unit_type", unit_type.as_str()));
        }
        if let Some(language) = &filters.language {
            query_params.push(("language", language.as_str()));
        }
        if let Some(visibility) = &filters.visibility {
            query_params.push(("visibility", visibility.as_str()));
        }

        let response = self.client
            .get(&format!("{}/workspaces/{}/units", self.base_url, workspace_id))
            .query(&query_params)
            .send()
            .await?;

        let result: UnitsResponse = Self::unwrap_response(response).await?;
        Ok(result.units)
    }
}

#[derive(Debug, Clone)]
pub struct SearchFilters {
    pub types: Vec<String>,
    pub languages: Vec<String>,
    pub visibility: Option<String>,
    pub min_relevance: f32,
}

impl Default for SearchFilters {
    fn default() -> Self {
        Self {
            types: vec![],
            languages: vec![],
            visibility: None,
            min_relevance: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SemanticSearchRequest {
    query: String,
    workspace_id: Option<String>,
    filters: SearchFiltersRequest,
    limit: usize,
}

#[derive(Debug, Clone, Serialize)]
struct SearchFiltersRequest {
    types: Vec<String>,
    languages: Vec<String>,
    visibility: Option<String>,
    min_relevance: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct SemanticSearchResponse {
    results: Vec<CodeSearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeSearchResult {
    pub unit_id: String,
    pub unit_type: String,
    pub name: String,
    pub qualified_name: String,
    pub signature: String,
    pub relevance_score: f32,
    pub file: String,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct UnitFilters {
    pub unit_type: Option<String>,
    pub language: Option<String>,
    pub visibility: Option<String>,
}

impl Default for UnitFilters {
    fn default() -> Self {
        Self {
            unit_type: None,
            language: None,
            visibility: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct UnitsResponse {
    units: Vec<CodeUnit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeUnit {
    pub id: String,
    pub unit_type: String,
    pub name: String,
    pub qualified_name: String,
    pub signature: String,
    pub file: String,
    pub lines: LineRange,
    pub visibility: String,
    pub complexity: Complexity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Complexity {
    pub cyclomatic: u32,
    pub cognitive: u32,
}
```

## Task Management

```rust
impl CortexBridge {
    /// POST /tasks - Create task in Cortex
    pub async fn create_task(&self, task: TaskDefinition) -> Result<TaskId> {
        let request = CreateTaskRequest {
            title: task.title,
            description: task.description,
            workspace_id: task.workspace_id,
            estimated_hours: task.estimated_hours,
        };

        let response = self.client
            .post(&format!("{}/tasks", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: CreateTaskResponse = Self::unwrap_response(response).await?;
        Ok(TaskId::from(result.task_id))
    }

    /// PUT /tasks/{id} - Update task status
    pub async fn update_task(
        &self,
        task_id: &TaskId,
        status: TaskStatus,
        metadata: TaskMetadata,
    ) -> Result<()> {
        let request = UpdateTaskRequest {
            status: status.to_string(),
            actual_hours: metadata.duration.as_secs_f64() / 3600.0,
            completion_note: metadata.notes,
        };

        let response = self.client
            .put(&format!("{}/tasks/{}", self.base_url, task_id))
            .json(&request)
            .send()
            .await?;

        // Unwrap response (returns empty data for successful update)
        let _: serde_json::Value = Self::unwrap_response(response).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub title: String,
    pub description: String,
    pub workspace_id: String,
    pub estimated_hours: f64,
}

#[derive(Debug, Clone, Serialize)]
struct CreateTaskRequest {
    title: String,
    description: String,
    workspace_id: String,
    estimated_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateTaskResponse {
    task_id: String,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Pending => "pending".to_string(),
            TaskStatus::InProgress => "in_progress".to_string(),
            TaskStatus::Completed => "done".to_string(),
            TaskStatus::Failed => "failed".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub duration: Duration,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateTaskRequest {
    status: String,
    actual_hours: f64,
    completion_note: Option<String>,
}
```

## Lock Management

```rust
impl CortexBridge {
    /// POST /locks - Acquire lock on entity
    pub async fn acquire_lock(
        &self,
        entity_id: &str,
        lock_type: LockType,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<LockId> {
        let request = AcquireLockRequest {
            entity_id: entity_id.to_string(),
            lock_type: lock_type.to_string(),
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            scope: "entity".to_string(),
            timeout: 300,
            wait: true,
        };

        let response = self.client
            .post(&format!("{}/locks", self.base_url))
            .json(&request)
            .send()
            .await?;

        let result: AcquireLockResponse = Self::unwrap_response(response).await?;
        let lock_id = LockId::from(result.lock_id);

        self.metrics.locks_acquired.inc();
        info!("Acquired {} lock {} on entity {}", lock_type.to_string(), lock_id, entity_id);

        Ok(lock_id)
    }

    /// DELETE /locks/{id} - Release lock
    pub async fn release_lock(&self, lock_id: &LockId) -> Result<()> {
        let response = self.client
            .delete(&format!("{}/locks/{}", self.base_url, lock_id))
            .send()
            .await?;

        // Unwrap response (returns empty data for successful release)
        let _: serde_json::Value = Self::unwrap_response(response).await?;

        self.metrics.locks_released.inc();
        info!("Released lock {}", lock_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum LockType {
    Shared,
    Exclusive,
}

impl ToString for LockType {
    fn to_string(&self) -> String {
        match self {
            LockType::Shared => "shared".to_string(),
            LockType::Exclusive => "exclusive".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct AcquireLockRequest {
    entity_id: String,
    lock_type: String,
    agent_id: String,
    session_id: String,
    scope: String,
    timeout: u32,
    wait: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct AcquireLockResponse {
    lock_id: String,
}
```

## WebSocket Event Stream

```rust
impl CortexBridge {
    /// Start event handler for WebSocket events
    async fn start_event_handler(&self) {
        let ws = self.websocket.clone();
        let event_stream = self.event_stream.clone();

        tokio::spawn(async move {
            loop {
                if let Some(ws_client) = ws.read().await.as_ref() {
                    if let Ok(event) = ws_client.receive().await {
                        event_stream.emit(event).await;
                    }
                } else {
                    // WebSocket not connected, wait and retry
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });
    }

    /// Subscribe to specific event types
    pub async fn subscribe_events<F>(&self, filter: EventFilter, handler: F)
    where
        F: Fn(CortexEvent) + Send + Sync + 'static,
    {
        self.event_stream.subscribe(filter, handler).await;
    }
}

#[derive(Debug, Clone)]
pub enum CortexEvent {
    SessionCreated { session_id: String },
    SessionMerged { session_id: String, conflicts: u32 },
    SessionClosed { session_id: String },
    LockAcquired { lock_id: String, entity_id: String },
    LockReleased { lock_id: String },
    LockDeadlock { entity_id: String, agents: Vec<String> },
    ConflictDetected { session_id: String, files: Vec<String> },
    FileChanged { path: String, workspace_id: String },
    PatternDetected { pattern: String, confidence: f32 },
}

#[derive(Debug, Clone)]
pub enum EventFilter {
    All,
    Sessions,
    Locks,
    Conflicts,
    FileChanges,
    Patterns,
}
```

## Performance Optimization

### Caching Layer

```rust
pub struct MemoryCache {
    store: LruCache<String, Vec<u8>>,
    ttl: Duration,
    statistics: CacheStatistics,
}

impl MemoryCache {
    pub fn new(max_size_bytes: usize, ttl: Duration) -> Self {
        Self {
            store: LruCache::new(max_size_bytes),
            ttl,
            statistics: CacheStatistics::default(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.store.get(key) {
            self.statistics.hits += 1;
            Some(entry.clone())
        } else {
            self.statistics.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, key: String, value: Vec<u8>) {
        self.store.put(key, value);
    }

    pub fn invalidate_pattern(&mut self, pattern: &str) {
        let keys_to_remove: Vec<_> = self.store.iter()
            .filter(|(k, _)| k.starts_with(pattern.trim_end_matches('*')))
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            self.store.pop(&key);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStatistics {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            return 0.0;
        }
        self.hits as f64 / (self.hits + self.misses) as f64
    }
}
```

### Error Handling and Retry Logic

```rust
impl CortexBridge {
    /// Execute request with retry logic
    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> BoxFuture<'static, Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = Duration::from_millis(self.config.retry_delay_ms);

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.config.max_retries && Self::is_retryable(&e) => {
                    warn!("Request failed (attempt {}/{}): {}",
                        attempt + 1, self.config.max_retries, e);

                    tokio::time::sleep(delay).await;
                    delay = delay.mul_f32(2.0); // Exponential backoff
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn is_retryable(error: &Error) -> bool {
        matches!(error,
            Error::NetworkError(_) |
            Error::Timeout(_) |
            Error::CortexUnavailable(_)
        )
    }
}
```

## Metrics and Monitoring

```rust
#[derive(Debug, Clone)]
pub struct BridgeMetrics {
    // Sessions
    pub sessions_created: Counter,
    pub successful_merges: Counter,
    pub merge_conflicts: Counter,

    // Files
    pub files_read: Counter,
    pub files_written: Counter,

    // Memory
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub semantic_searches: Counter,
    pub episodes_stored: Counter,

    // Locks
    pub locks_acquired: Counter,
    pub locks_released: Counter,

    // Performance
    request_durations: HashMap<String, Vec<Duration>>,
}

impl BridgeMetrics {
    pub fn new() -> Self {
        Self {
            sessions_created: Counter::new(),
            successful_merges: Counter::new(),
            merge_conflicts: Counter::new(),
            files_read: Counter::new(),
            files_written: Counter::new(),
            cache_hits: Counter::new(),
            cache_misses: Counter::new(),
            semantic_searches: Counter::new(),
            episodes_stored: Counter::new(),
            locks_acquired: Counter::new(),
            locks_released: Counter::new(),
            request_durations: HashMap::new(),
        }
    }

    pub fn record_request_duration(&mut self, operation: &str, duration: Duration) {
        self.request_durations
            .entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }

    pub fn export(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            sessions_created: self.sessions_created.get(),
            successful_merges: self.successful_merges.get(),
            merge_conflicts: self.merge_conflicts.get(),
            files_read: self.files_read.get(),
            files_written: self.files_written.get(),
            cache_hit_rate: self.cache_hit_rate(),
            semantic_searches: self.semantic_searches.get(),
            episodes_stored: self.episodes_stored.get(),
            locks_acquired: self.locks_acquired.get(),
            locks_released: self.locks_released.get(),
        }
    }

    fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.get();
        let misses = self.cache_misses.get();

        if hits + misses == 0 {
            return 0.0;
        }

        hits as f64 / (hits + misses) as f64
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub sessions_created: u64,
    pub successful_merges: u64,
    pub merge_conflicts: u64,
    pub files_read: u64,
    pub files_written: u64,
    pub cache_hit_rate: f64,
    pub semantic_searches: u64,
    pub episodes_stored: u64,
    pub locks_acquired: u64,
    pub locks_released: u64,
}

pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self { value: AtomicU64::new(0) }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, val: u64) {
        self.value.fetch_add(val, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}
```

## Usage Patterns

### Pattern 1: Agent Task Execution

```rust
pub async fn execute_agent_task(
    bridge: &CortexBridge,
    agent: &DeveloperAgent,
    task: Task,
) -> Result<TaskResult> {
    // 1. Create isolated session
    let session_id = bridge.create_session(
        agent.id.clone(),
        task.workspace_id.clone(),
        SessionScope {
            paths: vec![task.target_path.clone()],
            read_only_paths: vec!["tests/**".to_string()],
        },
    ).await?;

    // 2. Retrieve context (parallel for performance)
    let (episodes, patterns, units, similar_code) = tokio::join!(
        bridge.search_episodes(&task.description, 5),
        bridge.get_patterns(),
        bridge.get_code_units(&task.workspace_id, UnitFilters::default()),
        bridge.semantic_search(&task.description, &task.workspace_id, SearchFilters::default()),
    );

    let context = AgentContext {
        episodes: episodes?,
        patterns: patterns?,
        units: units?,
        similar_code: similar_code?,
    };

    // 3. Execute task
    let result = agent.execute_with_context(task.clone(), session_id.clone(), context).await?;

    // 4. Merge changes
    let merge_report = bridge.merge_session(&session_id, MergeStrategy::Auto).await?;

    if merge_report.conflicts_resolved > 0 {
        warn!("Resolved {} conflicts during merge", merge_report.conflicts_resolved);
    }

    // 5. Store episode for learning
    let episode = Episode {
        id: uuid::Uuid::new_v4().to_string(),
        episode_type: EpisodeType::Task,
        task_description: task.description.clone(),
        agent_id: agent.id.to_string(),
        session_id: Some(session_id.to_string()),
        workspace_id: task.workspace_id.to_string(),
        entities_created: result.created_entities.clone(),
        entities_modified: result.modified_entities.clone(),
        entities_deleted: result.deleted_entities.clone(),
        files_touched: result.modified_files.clone(),
        queries_made: result.queries_executed.clone(),
        tools_used: result.tools_used.clone(),
        solution_summary: result.summary.clone(),
        outcome: if result.success {
            EpisodeOutcome::Success
        } else {
            EpisodeOutcome::Failure
        },
        success_metrics: serde_json::json!({
            "tests_passed": result.tests_passed,
            "code_quality": result.quality_score,
        }),
        errors_encountered: result.errors.clone(),
        lessons_learned: result.patterns_used.clone(),
        duration_seconds: result.duration.as_secs() as i32,
        tokens_used: TokenUsage {
            input: result.tokens_in,
            output: result.tokens_out,
            total: result.tokens_in + result.tokens_out,
        },
        embedding: Vec::new(),  // Generated by Cortex
        created_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };
    bridge.store_episode(episode).await?;

    // 6. Cleanup
    bridge.close_session(&session_id, &agent.id).await?;

    Ok(result)
}
```

### Pattern 2: Multi-Agent Coordination

```rust
pub async fn coordinate_multi_agent_task(
    bridge: &CortexBridge,
    agents: Vec<Agent>,
    task: ComplexTask,
) -> Result<TaskResult> {
    let mut sessions = Vec::new();
    let mut locks = Vec::new();

    // Create sessions for all agents
    for agent in &agents {
        let session_id = bridge.create_session(
            agent.id.clone(),
            task.workspace_id.clone(),
            SessionScope {
                paths: task.shared_files.clone(),
                read_only_paths: vec![],
            },
        ).await?;
        sessions.push((agent.id.clone(), session_id));
    }

    // Acquire locks on shared resources
    for file in &task.shared_files {
        let lock_id = bridge.acquire_lock(
            file,
            LockType::Exclusive,
            &agents[0].id,
            &sessions[0].1,
        ).await?;
        locks.push(lock_id);
    }

    // Execute agents in parallel
    let handles: Vec<_> = agents.iter().zip(sessions.iter()).map(|(agent, (_, session_id))| {
        let agent = agent.clone();
        let session_id = session_id.clone();
        let task = task.clone();

        tokio::spawn(async move {
            agent.execute_in_session(&session_id, task).await
        })
    }).collect();

    // Wait for completion
    let results = futures::future::join_all(handles).await;

    // Release locks
    for lock_id in locks {
        bridge.release_lock(&lock_id).await?;
    }

    // Merge all sessions
    for (agent_id, session_id) in sessions {
        bridge.merge_session(&session_id, MergeStrategy::Auto).await?;
        bridge.close_session(&session_id, &agent_id).await?;
    }

    Ok(TaskResult::from_results(results))
}
```

## Complete API Endpoint Reference

| Endpoint | Method | Purpose | Agent Usage |
|----------|--------|---------|-------------|
| `/sessions` | POST | Create session | All agents - task isolation |
| `/sessions/{id}` | GET | Session status | Monitor session state |
| `/sessions/{id}` | DELETE | Close session | Cleanup after task |
| `/sessions/{id}/files/{path}` | GET | Read file | Developer, Reviewer, Tester |
| `/sessions/{id}/files/{path}` | PUT | Write file | Developer, Documenter |
| `/sessions/{id}/merge` | POST | Merge changes | All agents - commit work |
| `/memory/episodes` | POST | Store episode | All agents - learning |
| `/memory/search` | POST | Search episodes | All agents - context |
| `/memory/patterns` | GET | Get patterns | All agents - best practices |
| `/search/semantic` | POST | Semantic search | Developer, Architect |
| `/workspaces/{id}/units` | GET | Code structure | Reviewer, Tester, Architect |
| `/tasks` | POST | Create task | Orchestrator - tracking |
| `/tasks/{id}` | PUT | Update task | All agents - status |
| `/locks` | POST | Acquire lock | Multi-agent coordination |
| `/locks/{id}` | DELETE | Release lock | Multi-agent coordination |
| `/health` | GET | Health check | CortexBridge - monitoring |

## Conclusion

The CortexBridge provides a complete, production-ready integration layer between Axon's orchestration and Cortex's data layer. Key features:

1. **Session Isolation**: Safe concurrent agent execution
2. **Episodic Learning**: Shared knowledge across all agents
3. **Semantic Search**: Context-aware code discovery
4. **Performance**: Caching and connection pooling
5. **Reliability**: Retry logic and error handling
6. **Monitoring**: Comprehensive metrics
7. **Real-time**: WebSocket event stream

Every agent interaction with data flows through CortexBridge, ensuring consistency, learning, and coordination across the entire multi-agent system.
