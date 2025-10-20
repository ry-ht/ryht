# Cortex: Implementation Architecture

## System Architecture

### High-Level Design

```
┌──────────────────────────────────────────────────────────┐
│                    Client Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Claude Agent │  │  Other LLM   │  │   CLI Tool   │  │
│  │     SDK      │  │    Agents    │  │              │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                    MCP Protocol Layer                     │
│                    (JSON-RPC over stdio)                  │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                  Cortex Core Server                     │
│  ┌────────────────────────────────────────────────────┐  │
│  │               Request Router & Handler              │  │
│  └────────────────────────────────────────────────────┘  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐            │  │
│  │   Auth   │  │   Rate   │  │  Metrics │            │  │
│  │  Layer   │  │  Limiter │  │ Collector│            │  │
│  └──────────┘  └──────────┘  └──────────┘            │  │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                     Service Layer                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │   VFS    │  │ Semantic │  │  Session │  │ Memory │  │
│  │ Manager  │  │  Graph   │  │  Manager │  │ System │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │   Code   │  │   Lock   │  │  Merge   │  │ Flush  │  │
│  │  Parser  │  │  Manager │  │  Engine  │  │ Engine │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘  │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                    Storage Layer                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │         SurrealDB (Local/Remote Server)            │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐       │  │
│  │  │Connection│  │  Vector  │  │  Cache   │       │  │
│  │  │   Pool   │  │  Index   │  │  Layer   │       │  │
│  │  └──────────┘  └──────────┘  └──────────┘       │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                 External Systems                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │
│  │   Git    │  │  Build   │  │  File    │  │  Cloud │  │
│  │  Repos   │  │  Systems │  │  System  │  │ Storage│  │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘  │
└──────────────────────────────────────────────────────────┘
```

## Project Structure

```
cortex/
├── Cargo.toml                 # Workspace definition
├── crates/
│   ├── cortex-server/       # Main MCP server
│   │   ├── src/
│   │   │   ├── main.rs       # Entry point
│   │   │   ├── server.rs     # MCP server implementation
│   │   │   ├── handlers/     # Tool handlers
│   │   │   └── middleware/   # Auth, logging, etc.
│   │   └── Cargo.toml
│   │
│   ├── cortex-core/         # Core business logic
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── vfs/          # Virtual filesystem
│   │   │   ├── graph/        # Semantic graph
│   │   │   ├── session/      # Session management
│   │   │   ├── memory/       # Cognitive memory
│   │   │   └── merge/        # Merge engine
│   │   └── Cargo.toml
│   │
│   ├── cortex-storage/      # Storage abstraction
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── surreal/      # SurrealDB integration
│   │   │   ├── cache/        # Caching layer
│   │   │   └── migrations/   # Schema migrations
│   │   └── Cargo.toml
│   │
│   ├── cortex-parser/       # Code parsing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── tree_sitter/  # Tree-sitter integration
│   │   │   ├── languages/    # Language-specific parsers
│   │   │   └── ast/          # AST manipulation
│   │   └── Cargo.toml
│   │
│   ├── cortex-mcp/          # MCP protocol
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── protocol.rs   # Protocol types
│   │   │   ├── codec.rs      # Message encoding/decoding
│   │   │   └── transport.rs  # Transport layer
│   │   └── Cargo.toml
│   │
│   └── cortex-cli/          # CLI tools
│       ├── src/
│       │   ├── main.rs
│       │   └── commands/
│       └── Cargo.toml
│
├── migrations/                # Database migrations
├── configs/                   # Configuration files
├── scripts/                   # Build and deployment scripts
├── tests/                     # Integration tests
└── docs/                      # Documentation
```

## Core Components

### MCP Server Implementation

```rust
// cortex-server/src/server.rs

use cortex_mcp::{Request, Response, ToolCall};
use cortex_core::ToolRegistry;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct CortexServer {
    registry: Arc<ToolRegistry>,
    config: ServerConfig,
    metrics: Arc<MetricsCollector>,
}

impl CortexServer {
    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut writer = stdout;

        // Send initialization
        self.send_init(&mut writer).await?;

        // Main request loop
        let mut line = String::new();
        loop {
            line.clear();

            // Read request
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            // Parse request
            let request: Request = serde_json::from_str(&line)?;

            // Handle request
            let response = self.handle_request(request).await;

            // Send response
            let response_json = serde_json::to_string(&response)?;
            writer.write_all(response_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: Request) -> Response {
        let start = Instant::now();

        let result = match request {
            Request::Initialize(params) => {
                self.handle_initialize(params).await
            },
            Request::ToolCall(call) => {
                self.handle_tool_call(call).await
            },
            Request::Shutdown => {
                self.handle_shutdown().await
            },
            _ => Err(Error::UnknownRequest),
        };

        // Record metrics
        self.metrics.record_request(
            &request.method(),
            start.elapsed(),
            result.is_ok()
        );

        // Convert to response
        match result {
            Ok(value) => Response::success(request.id, value),
            Err(e) => Response::error(request.id, e),
        }
    }

    async fn handle_tool_call(&self, call: ToolCall) -> Result<Value> {
        // Get tool handler
        let handler = self.registry
            .get_handler(&call.tool)
            .ok_or(Error::ToolNotFound)?;

        // Validate parameters
        handler.validate_params(&call.arguments)?;

        // Execute tool
        let context = self.build_context(&call)?;
        handler.execute(context, call.arguments).await
    }
}
```

### Storage Layer

```rust
// cortex-storage/src/surreal/mod.rs

use surrealdb::{Surreal, engine::remote::ws::Ws};

pub struct SurrealStorage {
    connection_manager: Arc<ConnectionManager>,
    namespace: String,
    cache: Arc<CacheLayer>,
}

impl SurrealStorage {
    pub async fn new(config: StorageConfig) -> Result<Self> {
        // Initialize connection manager with pool
        let conn_config = DatabaseConfig {
            connection_mode: match config.connection_url.as_str() {
                url if url.starts_with("ws://") || url.starts_with("wss://") => {
                    ConnectionMode::Remote {
                        endpoints: vec![url.to_string()],
                        load_balancing: LoadBalancingStrategy::RoundRobin,
                    }
                },
                _ => ConnectionMode::Local {
                    endpoint: config.connection_url.clone(),
                },
            },
            credentials: config.credentials,
            pool_config: PoolConfig {
                min_connections: 2,
                max_connections: 10,
                connection_timeout: Duration::from_secs(5),
                idle_timeout: Some(Duration::from_secs(300)),
                max_lifetime: Some(Duration::from_secs(3600)),
                retry_policy: RetryPolicy::exponential_backoff(),
            },
            namespace: config.namespace.clone(),
            database: config.database,
        };

        let connection_manager = Arc::new(ConnectionManager::new(conn_config).await?);

        // Initialize cache
        let cache = Arc::new(CacheLayer::new(config.cache_config));

        // Run migrations
        Self::run_migrations(&connection_manager).await?;

        Ok(Self {
            connection_manager,
            namespace: config.namespace,
            cache,
        })
    }

    pub async fn get_vnode(&self, path: &str) -> Result<Option<VNode>> {
        // Check cache first
        if let Some(vnode) = self.cache.get::<VNode>(&format!("vnode:{}", path)).await {
            return Ok(Some(vnode));
        }

        // Query database through connection manager
        let query = Query::Select {
            table: "vnode".to_string(),
            id: path.to_string(),
        };

        let result: Option<VNode> = self.connection_manager
            .execute(query)
            .await?;

        // Cache result
        if let Some(ref vnode) = result {
            self.cache.set(&format!("vnode:{}", path), vnode.clone()).await;
        }

        Ok(result)
    }

    pub async fn create_vnode(&self, vnode: VNode) -> Result<()> {
        // Use connection manager for transaction
        let queries = vec![
            Query::Create {
                table: "vnode".to_string(),
                content: serde_json::to_value(&vnode)?,
            },
            Query::Create {
                table: "vnode_version".to_string(),
                content: serde_json::to_value(&VNodeVersion::from(&vnode))?,
            },
        ];

        // Execute in transaction
        self.connection_manager
            .execute_transaction(queries)
            .await?;

        // Invalidate cache
        self.cache.invalidate(&format!("vnode:{}", vnode.path)).await;

        Ok(())
    }

    pub async fn query_semantic(&self, embedding: &[f32], limit: usize) -> Result<Vec<CodeUnit>> {
        let query_str = r#"
            SELECT *,
                   vector::similarity::cosine(embedding, $embedding) as similarity
            FROM code_unit
            WHERE embedding != NONE
            ORDER BY similarity DESC
            LIMIT $limit
        "#;

        let query = Query::Raw {
            query: query_str.to_string(),
            bindings: vec![
                ("embedding".to_string(), embedding.into()),
                ("limit".to_string(), limit.into()),
            ],
        };

        let result: Vec<CodeUnit> = self.connection_manager
            .execute(query)
            .await?;

        Ok(result)
    }
}
```

### Virtual Filesystem

```rust
// cortex-core/src/vfs/mod.rs

pub struct VirtualFileSystem {
    storage: Arc<SurrealStorage>,
    parser: Arc<CodeParser>,
    content_cache: Arc<ContentCache>,
}

impl VirtualFileSystem {
    pub async fn read_file(&self, path: &VirtualPath) -> Result<String> {
        // Virtual path is already normalized and relative to repo root
        let path_str = path.to_string();

        // Get vnode
        let vnode = self.storage
            .get_vnode(&path_str)?
            .ok_or(Error::FileNotFound)?;

        // Check if file
        if vnode.node_type != NodeType::File {
            return Err(Error::NotAFile);
        }

        // Get content
        let content = self.get_content(&vnode.content_hash).await?;

        Ok(content)
    }

    pub async fn write_file(&self, path: &VirtualPath, content: &str) -> Result<()> {
        let path_str = path.to_string();

        // Hash content for deduplication
        let content_hash = sha256(content);

        // Check if content already exists
        if !self.content_exists(&content_hash).await? {
            // Store new content
            self.store_content(&content_hash, content).await?;
        }

        // Get or create vnode
        let mut vnode = match self.storage.get_vnode(&normalized).await? {
            Some(v) => v,
            None => self.create_vnode(&normalized, NodeType::File)?,
        };

        // Update vnode
        let old_version = vnode.version;
        vnode.content_hash = Some(content_hash);
        vnode.size_bytes = content.len();
        vnode.version += 1;
        vnode.updated_at = Utc::now();
        vnode.status = Status::Modified;

        // Detect language
        if let Some(lang) = detect_language(&normalized) {
            vnode.language = Some(lang);

            // Parse and extract units
            self.parse_file(&mut vnode, content, lang).await?;
        }

        // Save vnode
        self.storage.update_vnode(vnode).await?;

        // Create version record
        self.storage.create_version(VNodeVersion {
            vnode_id: vnode.id,
            version: old_version,
            operation: Operation::Update,
            content_hash: Some(content_hash),
            changed_by: current_agent_id(),
            changed_at: Utc::now(),
        }).await?;

        Ok(())
    }

    async fn parse_file(&self, vnode: &mut VNode, content: &str, language: Language) -> Result<()> {
        // Parse with tree-sitter
        let parse_result = self.parser.parse(content, language)?;

        // Extract code units
        let units = self.parser.extract_units(&parse_result)?;

        // Store units
        for unit in units {
            self.storage.create_code_unit(CodeUnit {
                file_node: vnode.id.clone(),
                language,
                ..unit
            }).await?;
        }

        Ok(())
    }
}
```

### Semantic Graph Engine

```rust
// cortex-core/src/graph/mod.rs

pub struct SemanticGraph {
    storage: Arc<SurrealStorage>,
    analyzer: Arc<DependencyAnalyzer>,
    embedder: Arc<EmbeddingGenerator>,
}

impl SemanticGraph {
    pub async fn build_dependencies(&self, unit: &CodeUnit) -> Result<()> {
        // Analyze dependencies
        let deps = self.analyzer.analyze(unit).await?;

        // Store relationships
        for dep in deps {
            self.storage.create_relationship(
                RelationType::DependsOn,
                &unit.id,
                &dep.target_id,
                dep.metadata
            ).await?;
        }

        Ok(())
    }

    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let embedding = self.embedder.embed_text(query).await?;

        // Search in vector space
        let units = self.storage.query_semantic(&embedding, limit * 2).await?;

        // Rank results
        let mut results = Vec::new();
        for unit in units {
            let score = self.calculate_relevance(&unit, query, &embedding)?;
            results.push(SearchResult {
                unit,
                score,
            });
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    pub async fn impact_analysis(&self, changed_units: &[UnitId]) -> Result<ImpactReport> {
        // Find direct dependents
        let direct = self.storage.query(r#"
            SELECT DISTINCT out as dependent
            FROM depends_on
            WHERE in IN $units
        "#, &[("units", changed_units)]).await?;

        // Find transitive dependents
        let transitive = self.storage.query(r#"
            SELECT DISTINCT id
            FROM code_unit
            WHERE id IN (
                SELECT out
                FROM depends_on
                WHERE in IN $units
                FETCH out, out<-depends_on<-code_unit RECURSIVE MAXDEPTH 10
            )
        "#, &[("units", changed_units)]).await?;

        // Categorize by risk
        let risk_level = self.calculate_risk(&direct, &transitive)?;

        Ok(ImpactReport {
            directly_affected: direct,
            transitively_affected: transitive,
            risk_level,
        })
    }
}
```

### Session Management

```rust
// cortex-core/src/session/mod.rs

pub struct SessionManager {
    storage: Arc<SurrealStorage>,
    lock_manager: Arc<LockManager>,
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
}

impl SessionManager {
    pub async fn create_session(&self, agent_id: &AgentId, config: SessionConfig) -> Result<Session> {
        // Create session namespace
        let session_ns = format!("session_{}", Uuid::new_v4());

        // Fork data into session namespace
        self.fork_workspace(&session_ns, &config.scope).await?;

        // Create session
        let session = Session {
            id: SessionId::new(),
            agent_id: agent_id.clone(),
            namespace: session_ns,
            isolation_level: config.isolation_level,
            scope: config.scope,
            changes: Arc::new(RwLock::new(Vec::new())),
            status: SessionStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + config.ttl,
        };

        // Register session
        self.sessions.write().await.insert(session.id.clone(), session.clone());

        Ok(session)
    }

    async fn fork_workspace(&self, namespace: &str, scope: &Scope) -> Result<()> {
        // Create namespace
        self.storage.create_namespace(namespace).await?;

        // Copy vnodes in scope
        for path_pattern in &scope.paths {
            self.storage.execute(format!(r#"
                USE NS {};
                INSERT INTO vnode
                SELECT * FROM {}.vnode
                WHERE path MATCHES $pattern
            "#, namespace, self.storage.namespace()), &[
                ("pattern", path_pattern)
            ]).await?;
        }

        // Copy code units
        self.storage.execute(format!(r#"
            USE NS {};
            INSERT INTO code_unit
            SELECT cu.*
            FROM {}.code_unit as cu
            WHERE cu.file_node IN (
                SELECT id FROM vnode
            )
        "#, namespace, self.storage.namespace())).await?;

        Ok(())
    }

    pub async fn merge_session(&self, session_id: &SessionId, strategy: MergeStrategy) -> Result<MergeReport> {
        let session = self.sessions.read().await
            .get(session_id)
            .cloned()
            .ok_or(Error::SessionNotFound)?;

        // Get changes
        let changes = session.changes.read().await.clone();

        // Check for conflicts
        let conflicts = self.detect_conflicts(&session, &changes).await?;

        if !conflicts.is_empty() {
            match strategy {
                MergeStrategy::Auto => {
                    self.auto_resolve(&conflicts).await?
                },
                MergeStrategy::Manual => {
                    return Err(Error::ManualResolutionRequired(conflicts));
                },
                _ => {}
            }
        }

        // Apply changes to main namespace
        let mut report = MergeReport::default();

        for change in changes {
            match self.apply_change(&change).await {
                Ok(_) => report.successful += 1,
                Err(e) => {
                    report.failed += 1;
                    report.errors.push(e);
                }
            }
        }

        // Cleanup session
        self.cleanup_session(session_id).await?;

        Ok(report)
    }
}
```

### Code Parser

```rust
// cortex-parser/src/tree_sitter/mod.rs

use tree_sitter::{Parser, Language, Query, QueryCursor};

pub struct TreeSitterParser {
    parsers: HashMap<LanguageType, Parser>,
    queries: HashMap<LanguageType, QuerySet>,
}

impl TreeSitterParser {
    pub fn new() -> Result<Self> {
        let mut parsers = HashMap::new();
        let mut queries = HashMap::new();

        // Initialize Rust parser
        let mut rust_parser = Parser::new();
        rust_parser.set_language(tree_sitter_rust::language())?;
        parsers.insert(LanguageType::Rust, rust_parser);
        queries.insert(LanguageType::Rust, QuerySet::rust());

        // Initialize TypeScript parser
        let mut ts_parser = Parser::new();
        ts_parser.set_language(tree_sitter_typescript::language_typescript())?;
        parsers.insert(LanguageType::TypeScript, ts_parser);
        queries.insert(LanguageType::TypeScript, QuerySet::typescript());

        // ... other languages

        Ok(Self { parsers, queries })
    }

    pub fn parse(&mut self, content: &str, language: LanguageType) -> Result<ParseResult> {
        let parser = self.parsers
            .get_mut(&language)
            .ok_or(Error::UnsupportedLanguage)?;

        let tree = parser.parse(content, None)
            .ok_or(Error::ParseError)?;

        Ok(ParseResult {
            tree,
            language,
            content: content.to_string(),
        })
    }

    pub fn extract_units(&self, result: &ParseResult) -> Result<Vec<CodeUnit>> {
        let queries = self.queries
            .get(&result.language)
            .ok_or(Error::NoQueries)?;

        let mut units = Vec::new();
        let mut cursor = QueryCursor::new();

        // Extract functions
        for match_ in cursor.matches(
            &queries.functions,
            result.tree.root_node(),
            result.content.as_bytes()
        ) {
            units.push(self.build_function_unit(match_, &result.content)?);
        }

        // Extract classes
        for match_ in cursor.matches(
            &queries.classes,
            result.tree.root_node(),
            result.content.as_bytes()
        ) {
            units.push(self.build_class_unit(match_, &result.content)?);
        }

        // ... other unit types

        Ok(units)
    }

    fn build_function_unit(&self, match_: Match, content: &str) -> Result<CodeUnit> {
        let mut unit = CodeUnit {
            unit_type: UnitType::Function,
            ..Default::default()
        };

        for capture in match_.captures {
            let text = capture.node.utf8_text(content.as_bytes())?;

            match self.queries.function_captures[capture.index] {
                "name" => unit.name = text.to_string(),
                "params" => unit.parameters = self.parse_parameters(capture.node, content)?,
                "return" => unit.return_type = Some(text.to_string()),
                "body" => unit.body = text.to_string(),
                _ => {}
            }
        }

        // Calculate metrics
        unit.complexity = self.calculate_complexity(&unit.body)?;
        unit.start_line = match_.captures[0].node.start_position().row;
        unit.end_line = match_.captures[0].node.end_position().row;

        Ok(unit)
    }
}
```

## Concurrency Model

### Actor System

```rust
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct ActorSystem {
    actors: HashMap<String, ActorHandle>,
}

pub struct ActorHandle {
    tx: mpsc::Sender<ActorMessage>,
    handle: JoinHandle<()>,
}

pub trait Actor: Send + Sync + 'static {
    type Message: Send + 'static;
    type State: Send + Sync + 'static;

    async fn handle_message(
        &mut self,
        msg: Self::Message,
        state: &mut Self::State
    ) -> Result<()>;
}

impl ActorSystem {
    pub fn spawn<A: Actor>(&mut self, name: String, actor: A) -> mpsc::Sender<A::Message> {
        let (tx, mut rx) = mpsc::channel(100);

        let handle = tokio::spawn(async move {
            let mut state = A::State::default();

            while let Some(msg) = rx.recv().await {
                if let Err(e) = actor.handle_message(msg, &mut state).await {
                    error!("Actor {} error: {}", name, e);
                }
            }
        });

        self.actors.insert(name.clone(), ActorHandle {
            tx: tx.clone(),
            handle,
        });

        tx
    }
}
```

### Connection Pooling

```rust
pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    config: PoolConfig,
    endpoints: Vec<String>,
    load_balancer: LoadBalancer,
}

impl ConnectionPool {
    pub async fn acquire(&self) -> Result<PooledConnection> {
        // Acquire semaphore permit
        let permit = self.semaphore.acquire().await?;

        // Get connection from pool
        let mut connections = self.connections.write().await;

        let conn = if let Some(conn) = connections.pop() {
            // Verify connection health
            if conn.is_healthy().await {
                conn
            } else {
                // Create new connection if unhealthy
                self.create_connection().await?
            }
        } else {
            // Create new connection
            self.create_connection().await?
        };

        Ok(PooledConnection {
            conn,
            pool: self.clone(),
            _permit: permit,
        })
    }

    async fn create_connection(&self) -> Result<PooledConnection> {
        // Select endpoint based on load balancing strategy
        let endpoint = self.load_balancer.select_endpoint(&self.endpoints)?;

        let conn = match endpoint.scheme() {
            "ws" | "wss" => {
                // Remote SurrealDB server
                Surreal::new::<Ws>(endpoint).await?
            },
            _ => {
                // Local SurrealDB server
                Surreal::new::<Ws>(&format!("ws://localhost:8000")).await?
            }
        };

        conn.use_ns(&self.config.namespace)
            .use_db(&self.config.database)
            .await?;

        Ok(PooledConnection::new(conn))
    }
}

pub struct PooledConnection {
    conn: SurrealConnection,
    pool: ConnectionPool,
    _permit: SemaphorePermit<'static>,
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        // Return connection to pool
        let conn = std::mem::replace(&mut self.conn, SurrealConnection::default());
        let pool = self.pool.clone();

        tokio::spawn(async move {
            let mut connections = pool.connections.write().await;
            connections.push(conn);
        });
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CortexError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Lock conflict: {0}")]
    LockConflict(String),

    #[error("Merge conflict: {conflicts:?}")]
    MergeConflict {
        conflicts: Vec<Conflict>,
    },

    #[error("Session expired: {0}")]
    SessionExpired(SessionId),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

pub type Result<T> = std::result::Result<T, CortexError>;
```

### Error Recovery

```rust
pub struct ErrorRecovery {
    strategies: HashMap<ErrorType, Box<dyn RecoveryStrategy>>,
}

#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    async fn recover(&self, error: &CortexError) -> Result<RecoveryAction>;
}

pub enum RecoveryAction {
    Retry { delay: Duration },
    Fallback { handler: Box<dyn FnOnce() -> Result<Value>> },
    Escalate,
    Ignore,
}

impl ErrorRecovery {
    pub async fn handle_error(&self, error: CortexError) -> Result<Value> {
        let error_type = self.classify_error(&error);

        if let Some(strategy) = self.strategies.get(&error_type) {
            match strategy.recover(&error).await? {
                RecoveryAction::Retry { delay } => {
                    tokio::time::sleep(delay).await;
                    // Retry operation
                },
                RecoveryAction::Fallback { handler } => {
                    return handler();
                },
                RecoveryAction::Escalate => {
                    return Err(error);
                },
                RecoveryAction::Ignore => {
                    return Ok(Value::Null);
                },
            }
        }

        Err(error)
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_vfs_read_file() {
        // Setup mock storage
        let mut mock_storage = MockStorage::new();
        mock_storage
            .expect_get_vnode()
            .with(eq("/test.rs"))
            .returning(|_| Ok(Some(VNode {
                path: "/test.rs".to_string(),
                node_type: NodeType::File,
                content_hash: Some("abc123".to_string()),
                ..Default::default()
            })));

        // Create VFS with mock
        let vfs = VirtualFileSystem::new(Arc::new(mock_storage));

        // Test read
        let content = vfs.read_file(Path::new("/test.rs")).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_session_isolation() {
        let manager = SessionManager::new(test_config());

        // Create two sessions
        let session1 = manager.create_session(&agent1_id(), config1()).await.unwrap();
        let session2 = manager.create_session(&agent2_id(), config2()).await.unwrap();

        // Make changes in session1
        session1.write_file("/test.rs", "session1 content").await.unwrap();

        // Verify isolation - session2 shouldn't see changes
        let content = session2.read_file("/test.rs").await;
        assert!(content.is_err() || content.unwrap() != "session1 content");
    }
}
```

### Integration Tests

```rust
// tests/integration/mcp_tools.rs

#[tokio::test]
async fn test_full_tool_workflow() {
    let server = setup_test_server().await;

    // Create workspace
    let workspace_id = call_tool(&server, "cortex.workspace.create", json!({
        "name": "test-project",
        "root_path": "/tmp/test-project",
        "workspace_type": "rust_cargo"
    })).await.unwrap();

    // Create file
    call_tool(&server, "cortex.vfs.create_file", json!({
        "path": "/src/main.rs",
        "content": "fn main() { println!(\"Hello\"); }"
    })).await.unwrap();

    // Parse and analyze
    let units = call_tool(&server, "cortex.code.list_units", json!({
        "path": "/src/main.rs"
    })).await.unwrap();

    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["name"], "main");

    // Semantic search
    let results = call_tool(&server, "cortex.search.semantic", json!({
        "query": "print hello message",
        "limit": 5
    })).await.unwrap();

    assert!(!results.is_empty());
}
```

## Performance Optimizations

### Zero-Copy Operations

```rust
use bytes::Bytes;

pub struct ZeroCopyBuffer {
    data: Bytes,
}

impl ZeroCopyBuffer {
    pub fn slice(&self, start: usize, end: usize) -> Bytes {
        self.data.slice(start..end)
    }

    pub fn split_at(&mut self, index: usize) -> Bytes {
        self.data.split_to(index)
    }
}
```

### Lazy Loading

```rust
pub struct LazyField<T> {
    loader: Option<Box<dyn FnOnce() -> Result<T> + Send>>,
    value: OnceCell<T>,
}

impl<T> LazyField<T> {
    pub async fn get(&self) -> Result<&T> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }

        if let Some(loader) = self.loader.take() {
            let value = loader()?;
            self.value.set(value).map_err(|_| Error::AlreadyLoaded)?;
        }

        self.value.get().ok_or(Error::NotLoaded)
    }
}
```

### Batch Processing

```rust
pub struct BatchProcessor<T> {
    batch: Vec<T>,
    max_size: usize,
    flush_interval: Duration,
    processor: Arc<dyn Fn(Vec<T>) -> Result<()> + Send + Sync>,
}

impl<T: Send + 'static> BatchProcessor<T> {
    pub async fn add(&mut self, item: T) -> Result<()> {
        self.batch.push(item);

        if self.batch.len() >= self.max_size {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let batch = std::mem::take(&mut self.batch);
        (self.processor)(batch)?;

        Ok(())
    }

    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.flush_interval);

        loop {
            interval.tick().await;
            if let Err(e) = self.flush().await {
                error!("Batch flush error: {}", e);
            }
        }
    }
}
```

## Deployment

### Docker Configuration

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cortex-server /usr/local/bin/

ENV RUST_LOG=info
ENV CORTEX_DATA=/data

VOLUME ["/data"]

ENTRYPOINT ["cortex-server"]
CMD ["serve", "--stdio"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: cortex
spec:
  serviceName: cortex
  replicas: 1
  selector:
    matchLabels:
      app: cortex
  template:
    metadata:
      labels:
        app: cortex
    spec:
      containers:
      - name: cortex
        image: cortex:v3
        ports:
        - containerPort: 8080
        volumeMounts:
        - name: data
          mountPath: /data
        env:
        - name: RUST_LOG
          value: info
        resources:
          requests:
            memory: "2Gi"
            cpu: "1"
          limits:
            memory: "8Gi"
            cpu: "4"
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 100Gi
```

## Monitoring

### Metrics Collection

```rust
use prometheus::{Encoder, TextEncoder, Counter, Histogram, register_counter, register_histogram};

pub struct MetricsCollector {
    requests_total: Counter,
    request_duration: Histogram,
    errors_total: Counter,
}

impl MetricsCollector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            requests_total: register_counter!(
                "cortex_requests_total",
                "Total number of requests"
            )?,
            request_duration: register_histogram!(
                "cortex_request_duration_seconds",
                "Request duration in seconds"
            )?,
            errors_total: register_counter!(
                "cortex_errors_total",
                "Total number of errors"
            )?,
        })
    }

    pub fn record_request(&self, duration: Duration, success: bool) {
        self.requests_total.inc();
        self.request_duration.observe(duration.as_secs_f64());

        if !success {
            self.errors_total.inc();
        }
    }

    pub fn export(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
```

## Conclusion

This implementation architecture provides:

1. **Modular Design**: Clear separation of concerns with distinct crates
2. **Scalability**: Actor model and connection pooling for high concurrency
3. **Reliability**: Comprehensive error handling and recovery
4. **Performance**: Zero-copy operations, lazy loading, and batching
5. **Observability**: Metrics, logging, and tracing throughout
6. **Testability**: Mock-friendly interfaces and comprehensive test coverage

The architecture is designed to handle millions of code units, thousands of concurrent sessions, and provide sub-100ms response times for most operations.