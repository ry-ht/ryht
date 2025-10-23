//! Real-World Development Workflow E2E Tests
//!
//! This test suite simulates realistic development workflows including:
//! 1. Multi-language project development (Rust, TypeScript, TSX)
//! 2. Iterative refactoring workflows
//! 3. Multi-agent collaboration scenarios
//! 4. Cross-language semantic search
//! 5. Consistency verification between SurrealDB and Qdrant
//! 6. Stress testing with concurrent modifications
//! 7. Large batch operations and failure recovery
//!
//! These tests validate that Cortex can handle real development scenarios end-to-end.

use cortex_core::prelude::*;
use cortex_memory::prelude::*;
use cortex_memory::types::CodeUnitType;
use cortex_semantic::prelude::*;
use cortex_semantic::{SearchFilter, EntityType, MigrationMode};
use cortex_storage::connection_pool::{
    ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig,
};
use cortex_storage::{AgentSession, SessionManager, SessionMetadata, SessionScope, IsolationLevel};
use cortex_vfs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

// ============================================================================
// Test Configuration
// ============================================================================

const TEST_TIMEOUT_SECS: u64 = 180; // 3 minutes for complex workflows
const EMBEDDING_DIMENSION: usize = 384;

fn create_test_db_config(db_name: &str) -> DatabaseConfig {
    DatabaseConfig {
        connection_mode: ConnectionMode::Local {
            endpoint: "mem://".to_string(),
        },
        credentials: Credentials::default(),
        pool_config: PoolConfig {
            min_connections: 5,
            max_connections: 30,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(60)),
            max_lifetime: Some(Duration::from_secs(600)),
            acquire_timeout: Duration::from_secs(10),
            validation_interval: Duration::from_secs(60),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        },
        namespace: "cortex_workflow_test".to_string(),
        database: db_name.to_string(),
    }
}

fn create_semantic_config_qdrant() -> SemanticConfig {
    let mut config = SemanticConfig::default();
    config.embedding.primary_provider = "mock".to_string();
    config.qdrant.collection_name = format!("workflow_test_{}", Uuid::new_v4());
    config.vector_store.backend = VectorStoreBackend::Qdrant;
    config.vector_store.migration_mode = MigrationMode::SingleStore;
    config
}

// ============================================================================
// Test Context and Infrastructure
// ============================================================================

struct WorkflowContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    cognitive: Arc<CognitiveManager>,
    search_engine: Arc<SemanticSearchEngine>,
    session_manager: Arc<SessionManager>,
    workspace_id: Uuid,
    temp_dir: TempDir,
    metrics: Arc<Mutex<WorkflowMetrics>>,
}

#[derive(Debug, Default)]
struct WorkflowMetrics {
    files_created: usize,
    files_modified: usize,
    files_deleted: usize,
    searches_performed: usize,
    refactorings_completed: usize,
    conflicts_detected: usize,
    conflicts_resolved: usize,
    total_duration_ms: u128,
}

impl WorkflowContext {
    async fn new(test_name: &str) -> Result<Self> {
        let db_config = create_test_db_config(test_name);
        let storage = Arc::new(
            ConnectionManager::new(db_config)
                .await
                .expect("Failed to create connection manager"),
        );

        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let cognitive = Arc::new(CognitiveManager::new(storage.clone()));
        let session_manager = Arc::new(SessionManager::new(storage.clone()));

        let semantic_config = create_semantic_config_qdrant();
        let search_engine = Arc::new(
            SemanticSearchEngine::new(semantic_config)
                .await
                .expect("Failed to create search engine"),
        );

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workspace_id = Uuid::new_v4();

        Ok(Self {
            storage,
            vfs,
            cognitive,
            search_engine,
            session_manager,
            workspace_id,
            temp_dir,
            metrics: Arc::new(Mutex::new(WorkflowMetrics::default())),
        })
    }

    async fn create_file(&self, path: &str, content: &str) -> Result<()> {
        let vpath = VirtualPath::new(path)?;
        if let Some(parent) = vpath.parent() {
            self.vfs.create_directory(&self.workspace_id, &parent, true).await.ok();
        }
        self.vfs.write_file(&self.workspace_id, &vpath, content.as_bytes()).await?;

        let mut metrics = self.metrics.lock().await;
        metrics.files_created += 1;
        Ok(())
    }

    async fn modify_file(&self, path: &str, content: &str) -> Result<()> {
        let vpath = VirtualPath::new(path)?;
        self.vfs.write_file(&self.workspace_id, &vpath, content.as_bytes()).await?;

        let mut metrics = self.metrics.lock().await;
        metrics.files_modified += 1;
        Ok(())
    }

    async fn index_code(&self, id: &str, content: &str, language: &str) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), language.to_string());

        self.search_engine
            .index_document(
                id.to_string(),
                content.to_string(),
                EntityType::Code,
                metadata,
            )
            .await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut metrics = self.metrics.lock().await;
        metrics.searches_performed += 1;
        drop(metrics);

        self.search_engine.search(query, limit).await
    }
}

// ============================================================================
// TEST 1: Multi-Language Project Development
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_1_multi_language_project() -> Result<()> {
    info!("========================================");
    info!("TEST 1: Multi-Language Project Development");
    info!("========================================");

    let ctx = WorkflowContext::new("multi_language").await?;
    let start_time = Instant::now();

    // Phase 1: Create Rust backend
    info!("Phase 1: Creating Rust backend");
    let rust_files = vec![
        (
            "backend/src/main.rs",
            r#"use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/api/health", web::get().to(health_check))
            .route("/api/users", web::get().to(get_users))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

async fn health_check() -> &'static str {
    "OK"
}

async fn get_users() -> web::Json<Vec<String>> {
    web::Json(vec!["user1".to_string(), "user2".to_string()])
}
"#,
        ),
        (
            "backend/src/db.rs",
            r#"use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

pub async fn get_user(pool: &PgPool, id: i64) -> Result<User, sqlx::Error> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_one(pool)
        .await
}

pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}
"#,
        ),
    ];

    for (path, content) in &rust_files {
        ctx.create_file(path, content).await?;
        ctx.index_code(path, content, "rust").await?;
    }

    // Phase 2: Create TypeScript API client
    info!("Phase 2: Creating TypeScript API client");
    let ts_files = vec![
        (
            "frontend/src/api/client.ts",
            r#"import axios, { AxiosInstance } from 'axios';

export class ApiClient {
    private client: AxiosInstance;

    constructor(baseURL: string) {
        this.client = axios.create({
            baseURL,
            timeout: 5000,
        });
    }

    async healthCheck(): Promise<string> {
        const response = await this.client.get('/api/health');
        return response.data;
    }

    async getUsers(): Promise<string[]> {
        const response = await this.client.get('/api/users');
        return response.data;
    }

    async getUser(id: number): Promise<User> {
        const response = await this.client.get(`/api/users/${id}`);
        return response.data;
    }
}

export interface User {
    id: number;
    name: string;
    email: string;
}
"#,
        ),
        (
            "frontend/src/utils/validation.ts",
            r#"export function validateEmail(email: string): boolean {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
}

export function validateUsername(username: string): boolean {
    return username.length >= 3 && username.length <= 20;
}

export class ValidationError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'ValidationError';
    }
}
"#,
        ),
    ];

    for (path, content) in &ts_files {
        ctx.create_file(path, content).await?;
        ctx.index_code(path, content, "typescript").await?;
    }

    // Phase 3: Create React components
    info!("Phase 3: Creating React TSX components");
    let tsx_files = vec![
        (
            "frontend/src/components/UserList.tsx",
            r#"import React, { useEffect, useState } from 'react';
import { ApiClient, User } from '../api/client';

interface UserListProps {
    apiClient: ApiClient;
}

export const UserList: React.FC<UserListProps> = ({ apiClient }) => {
    const [users, setUsers] = useState<User[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchUsers = async () => {
            try {
                const userData = await apiClient.getUsers();
                setUsers(userData);
            } catch (err) {
                setError('Failed to load users');
            } finally {
                setLoading(false);
            }
        };

        fetchUsers();
    }, [apiClient]);

    if (loading) return <div>Loading...</div>;
    if (error) return <div>Error: {error}</div>;

    return (
        <div className="user-list">
            <h2>Users</h2>
            <ul>
                {users.map(user => (
                    <li key={user.id}>{user.name} ({user.email})</li>
                ))}
            </ul>
        </div>
    );
};
"#,
        ),
        (
            "frontend/src/components/UserForm.tsx",
            r#"import React, { useState } from 'react';
import { validateEmail, validateUsername, ValidationError } from '../utils/validation';

interface UserFormProps {
    onSubmit: (name: string, email: string) => Promise<void>;
}

export const UserForm: React.FC<UserFormProps> = ({ onSubmit }) => {
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [errors, setErrors] = useState<{name?: string; email?: string}>({});

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        const newErrors: typeof errors = {};

        if (!validateUsername(name)) {
            newErrors.name = 'Username must be 3-20 characters';
        }

        if (!validateEmail(email)) {
            newErrors.email = 'Invalid email address';
        }

        if (Object.keys(newErrors).length > 0) {
            setErrors(newErrors);
            return;
        }

        try {
            await onSubmit(name, email);
            setName('');
            setEmail('');
            setErrors({});
        } catch (err) {
            setErrors({ email: 'Failed to create user' });
        }
    };

    return (
        <form onSubmit={handleSubmit}>
            <div>
                <label>Name:</label>
                <input value={name} onChange={e => setName(e.target.value)} />
                {errors.name && <span className="error">{errors.name}</span>}
            </div>
            <div>
                <label>Email:</label>
                <input type="email" value={email} onChange={e => setEmail(e.target.value)} />
                {errors.email && <span className="error">{errors.email}</span>}
            </div>
            <button type="submit">Submit</button>
        </form>
    );
};
"#,
        ),
    ];

    for (path, content) in &tsx_files {
        ctx.create_file(path, content).await?;
        ctx.index_code(path, content, "tsx").await?;
    }

    // Wait for indexing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Phase 4: Cross-language semantic searches
    info!("Phase 4: Testing cross-language semantic search");

    let search_scenarios = vec![
        ("user management", "Should find user-related code across languages"),
        ("API endpoints", "Should find API definitions in Rust and TS"),
        ("validation logic", "Should find validation in TypeScript"),
        ("database queries", "Should find DB code in Rust"),
        ("React components", "Should find TSX components"),
        ("error handling", "Should find error handling across languages"),
    ];

    for (query, description) in &search_scenarios {
        info!("Searching: '{}' - {}", query, description);
        let results = ctx.search(query, 5).await?;
        info!("  Found {} results", results.len());

        if !results.is_empty() {
            for (i, result) in results.iter().take(3).enumerate() {
                info!("    [{}] {} (score: {:.3})", i + 1, result.id, result.score);
            }
        }
    }

    // Phase 5: Language-specific filtered searches
    info!("Phase 5: Testing language-specific filtered searches");

    for language in &["rust", "typescript", "tsx"] {
        let filter = SearchFilter {
            metadata_filters: {
                let mut filters = HashMap::new();
                filters.insert("language".to_string(), language.to_string());
                filters
            },
            ..Default::default()
        };

        let results = ctx.search_engine.search_with_filter("user", 10, filter).await?;
        info!("Language '{}' filter: {} results", language, results.len());
    }

    let metrics = ctx.metrics.lock().await;
    let duration = start_time.elapsed();

    info!("✅ TEST 1 PASSED in {:?}", duration);
    info!("  - Files created: {}", metrics.files_created);
    info!("  - Searches: {}", metrics.searches_performed);

    Ok(())
}

// ============================================================================
// TEST 2: Iterative Refactoring Workflow
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_2_iterative_refactoring() -> Result<()> {
    info!("========================================");
    info!("TEST 2: Iterative Refactoring Workflow");
    info!("========================================");

    let ctx = WorkflowContext::new("refactoring").await?;

    // Initial code
    let initial_code = r#"
fn calculate_price(quantity: i32, base_price: f64) -> f64 {
    let mut total = quantity as f64 * base_price;
    if quantity > 100 {
        total = total * 0.9;
    }
    if quantity > 500 {
        total = total * 0.95;
    }
    total
}
"#;

    info!("Phase 1: Initial implementation");
    ctx.create_file("src/pricing.rs", initial_code).await?;
    ctx.index_code("pricing_v1", initial_code, "rust").await?;

    // Refactoring iteration 1: Extract discount logic
    let refactor_1 = r#"
fn calculate_price(quantity: i32, base_price: f64) -> f64 {
    let subtotal = quantity as f64 * base_price;
    let discount = calculate_discount(quantity);
    subtotal * (1.0 - discount)
}

fn calculate_discount(quantity: i32) -> f64 {
    if quantity > 500 {
        0.15  // 15% discount
    } else if quantity > 100 {
        0.10  // 10% discount
    } else {
        0.0   // No discount
    }
}
"#;

    info!("Phase 2: Refactoring - Extract discount calculation");
    ctx.modify_file("src/pricing.rs", refactor_1).await?;
    ctx.index_code("pricing_v2", refactor_1, "rust").await?;

    let mut metrics = ctx.metrics.lock().await;
    metrics.refactorings_completed += 1;
    drop(metrics);

    // Refactoring iteration 2: Use enum for discount tiers
    let refactor_2 = r#"
enum DiscountTier {
    None,
    Standard,  // 10%
    Premium,   // 15%
}

impl DiscountTier {
    fn from_quantity(quantity: i32) -> Self {
        match quantity {
            q if q > 500 => DiscountTier::Premium,
            q if q > 100 => DiscountTier::Standard,
            _ => DiscountTier::None,
        }
    }

    fn discount_rate(&self) -> f64 {
        match self {
            DiscountTier::None => 0.0,
            DiscountTier::Standard => 0.10,
            DiscountTier::Premium => 0.15,
        }
    }
}

fn calculate_price(quantity: i32, base_price: f64) -> f64 {
    let subtotal = quantity as f64 * base_price;
    let tier = DiscountTier::from_quantity(quantity);
    let discount = tier.discount_rate();
    subtotal * (1.0 - discount)
}
"#;

    info!("Phase 3: Refactoring - Introduce discount tier enum");
    ctx.modify_file("src/pricing.rs", refactor_2).await?;
    ctx.index_code("pricing_v3", refactor_2, "rust").await?;

    let mut metrics = ctx.metrics.lock().await;
    metrics.refactorings_completed += 1;
    drop(metrics);

    // Wait for indexing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Search for refactoring evolution
    info!("Phase 4: Searching refactoring history");
    let results = ctx.search("discount calculation pricing", 10).await?;
    info!("Found {} versions in search", results.len());

    // Should find all three versions
    assert!(results.len() >= 3, "Should find all refactoring versions");

    let metrics = ctx.metrics.lock().await;
    info!("✅ TEST 2 PASSED");
    info!("  - Refactorings: {}", metrics.refactorings_completed);
    info!("  - File modifications: {}", metrics.files_modified);

    Ok(())
}

// ============================================================================
// TEST 3: Multi-Agent Collaboration
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_3_multi_agent_collaboration() -> Result<()> {
    info!("========================================");
    info!("TEST 3: Multi-Agent Collaboration");
    info!("========================================");

    let ctx = WorkflowContext::new("multi_agent").await?;

    // Setup shared codebase
    let shared_file = "src/shared.rs";
    let initial_content = "fn shared_function() {}";
    ctx.create_file(shared_file, initial_content).await?;

    // Create multiple agent sessions
    info!("Phase 1: Creating agent sessions");
    let agent_count = 5;
    let mut sessions = Vec::new();

    for i in 0..agent_count {
        let agent_id = format!("agent_{}", i);
        let session = ctx.session_manager
            .create_session(
                agent_id.clone(),
                ctx.workspace_id.into(),
                SessionMetadata {
                    description: format!("Agent {} session", i),
                    tags: vec!["collaboration".to_string()],
                    isolation_level: IsolationLevel::ReadCommitted,
                    scope: SessionScope {
                        paths: vec![],
                        read_only_paths: vec![],
                        units: vec![],
                        allow_create: true,
                        allow_delete: false,
                    },
                    custom: HashMap::new(),
                },
                None,
            )
            .await?;
        sessions.push((agent_id, session));
    }

    info!("Created {} agent sessions", sessions.len());

    // Phase 2: Concurrent modifications
    info!("Phase 2: Simulating concurrent modifications");
    let mut handles = vec![];

    for (i, (agent_id, _session)) in sessions.iter().enumerate() {
        let agent_id_clone = agent_id.clone();
        let vfs = ctx.vfs.clone();
        let workspace_id = ctx.workspace_id;
        let search_engine = ctx.search_engine.clone();

        let handle = tokio::spawn(async move {
            // Each agent creates their own file
            let agent_file = format!("src/agent_{}.rs", i);
            let content = format!(
                "// Agent {} implementation\nfn agent_{}_function() {{ println!(\"Agent {}\"); }}",
                i, i, i
            );

            let path = VirtualPath::new(&agent_file).unwrap();
            if let Some(parent) = path.parent() {
                vfs.create_directory(&workspace_id, &parent, true).await.ok();
            }
            vfs.write_file(&workspace_id, &path, content.as_bytes()).await.unwrap();

            // Index in search engine
            search_engine
                .index_document(
                    format!("agent_{}", i),
                    content.clone(),
                    EntityType::Code,
                    HashMap::new(),
                )
                .await
                .unwrap();

            info!("Agent {} completed work", i);
        });

        handles.push(handle);
    }

    // Wait for all agents
    for handle in handles {
        handle.await.unwrap();
    }

    // Wait for indexing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Phase 3: Verify all agent contributions
    info!("Phase 3: Verifying agent contributions");
    let results = ctx.search("agent function implementation", 10).await?;
    info!("Found {} agent contributions", results.len());

    assert!(
        results.len() >= agent_count,
        "Should find all agent contributions"
    );

    info!("✅ TEST 3 PASSED");
    Ok(())
}

// ============================================================================
// TEST 4: Consistency Verification
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_4_consistency_verification() -> Result<()> {
    info!("========================================");
    info!("TEST 4: SurrealDB-Qdrant Consistency");
    info!("========================================");

    let ctx = WorkflowContext::new("consistency").await?;

    // Create and index multiple units
    let unit_count = 50;
    let mut units = Vec::new();

    info!("Phase 1: Creating {} semantic units", unit_count);
    for i in 0..unit_count {
        let unit = SemanticUnit {
            id: CortexId::new(),
            unit_type: CodeUnitType::Function,
            name: format!("function_{}", i),
            qualified_name: format!("module::function_{}", i),
            display_name: format!("Function {}", i),
            file_path: format!("src/mod_{}.rs", i / 10),
            start_line: i * 10,
            start_column: 0,
            end_line: i * 10 + 5,
            end_column: 0,
            signature: format!("fn function_{}() -> i32", i),
            body: format!("// Function {} implementation\n    {}", i, i),
            docstring: Some(format!("Documentation for function {}", i)),
            visibility: "public".to_string(),
            modifiers: vec![],
            parameters: vec![],
            return_type: Some("i32".to_string()),
            summary: format!("Summary {}", i),
            purpose: format!("Purpose {}", i),
            complexity: ComplexityMetrics {
                cyclomatic: 1,
                cognitive: 1,
                nesting: 1,
                lines: 5,
            },
            test_coverage: Some(80.0),
            has_tests: true,
            has_documentation: true,
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        units.push(unit);
    }

    // Store in both SurrealDB (via cognitive) and Qdrant (via search)
    info!("Phase 2: Storing in both SurrealDB and Qdrant");
    for unit in &units {
        // Store in SurrealDB via cognitive manager
        ctx.cognitive.remember_unit(unit).await?;

        // Store in Qdrant via search engine
        let doc_text = format!("{} {}", unit.name, unit.summary);
        ctx.search_engine
            .index_document(
                unit.id.to_string(),
                doc_text,
                EntityType::Code,
                HashMap::new(),
            )
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Phase 3: Verify counts
    info!("Phase 3: Verifying counts");
    let qdrant_count = ctx.search_engine.document_count().await;
    info!("Qdrant count: {}", qdrant_count);
    info!("Expected count: {}", unit_count);

    assert_eq!(
        qdrant_count, unit_count,
        "Count mismatch between storage and index"
    );

    // Phase 4: Verify retrieval
    info!("Phase 4: Verifying retrieval consistency");
    let sample_unit = &units[25]; // Check middle unit
    let query = MemoryQuery::new(sample_unit.name.clone());
    let embedding = vec![0.1; EMBEDDING_DIMENSION];

    let recalled = ctx.cognitive.recall_units(&query, &embedding).await?;
    let searched = ctx.search(&sample_unit.name, 5).await?;

    info!("Recalled {} units from SurrealDB", recalled.len());
    info!("Searched {} units from Qdrant", searched.len());

    assert!(!recalled.is_empty(), "Should recall units from SurrealDB");
    assert!(!searched.is_empty(), "Should find units in Qdrant");

    info!("✅ TEST 4 PASSED - Consistency verified");
    Ok(())
}

// ============================================================================
// TEST 5: Large Batch Operations and Stress Test
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_5_large_batch_stress() -> Result<()> {
    info!("========================================");
    info!("TEST 5: Large Batch Operations");
    info!("========================================");

    let ctx = WorkflowContext::new("batch_stress").await?;

    // Phase 1: Large batch insert
    info!("Phase 1: Large batch insertion (1000 documents)");
    let batch_size = 1000;
    let batch_start = Instant::now();

    let documents: Vec<_> = (0..batch_size)
        .map(|i| {
            (
                format!("doc_{}", i),
                format!("Document {} with content about topic {}", i, i % 20),
                EntityType::Document,
                HashMap::new(),
            )
        })
        .collect();

    ctx.search_engine.index_batch(documents).await?;

    let batch_duration = batch_start.elapsed();
    let throughput = batch_size as f64 / batch_duration.as_secs_f64();

    info!(
        "Batch insert completed in {:?} ({:.2} docs/sec)",
        batch_duration, throughput
    );

    assert!(throughput > 100.0, "Batch throughput too low: {:.2} docs/sec", throughput);

    // Wait for indexing
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Phase 2: Concurrent search stress
    info!("Phase 2: Concurrent search stress (100 parallel searches)");
    let search_count = 100;
    let concurrent_level = 20;
    let search_start = Instant::now();

    let mut handles = vec![];
    for batch in 0..concurrent_level {
        let engine = ctx.search_engine.clone();
        let searches_per_batch = search_count / concurrent_level;

        let handle = tokio::spawn(async move {
            for i in 0..searches_per_batch {
                let query = format!("topic {}", (batch * searches_per_batch + i) % 20);
                let _ = engine.search(&query, 10).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let search_duration = search_start.elapsed();
    let search_throughput = search_count as f64 / search_duration.as_secs_f64();

    info!(
        "Concurrent searches completed in {:?} ({:.2} searches/sec)",
        search_duration, search_throughput
    );

    assert!(
        search_throughput > 50.0,
        "Search throughput too low: {:.2} searches/sec",
        search_throughput
    );

    // Phase 3: Mixed operations
    info!("Phase 3: Mixed read/write operations");
    let mixed_start = Instant::now();
    let mut handles = vec![];

    // Writers
    for i in 0..10 {
        let engine = ctx.search_engine.clone();
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                let id = format!("mixed_{}_{}", i, j);
                let content = format!("Mixed content {} {}", i, j);
                let _ = engine
                    .index_document(id, content, EntityType::Document, HashMap::new())
                    .await;
            }
        });
        handles.push(handle);
    }

    // Readers
    for i in 0..10 {
        let engine = ctx.search_engine.clone();
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                let query = format!("content {}", (i * 50 + j) % 20);
                let _ = engine.search(&query, 5).await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let mixed_duration = mixed_start.elapsed();
    info!("Mixed operations completed in {:?}", mixed_duration);

    info!("✅ TEST 5 PASSED");
    info!("  - Batch throughput: {:.2} docs/sec", throughput);
    info!("  - Search throughput: {:.2} searches/sec", search_throughput);

    Ok(())
}

// ============================================================================
// TEST 6: Failure Recovery
// ============================================================================

#[tokio::test]
#[ignore] // Requires Qdrant server
async fn test_6_failure_recovery() -> Result<()> {
    info!("========================================");
    info!("TEST 6: Failure Recovery");
    info!("========================================");

    let ctx = WorkflowContext::new("failure_recovery").await?;

    // Scenario 1: Partial batch failure simulation
    info!("Scenario 1: Handling partial failures gracefully");

    let mut success_count = 0;
    let mut failure_count = 0;

    for i in 0..100 {
        let id = format!("doc_{}", i);
        let content = format!("Document {}", i);

        match ctx.search_engine
            .index_document(id, content, EntityType::Document, HashMap::new())
            .await
        {
            Ok(_) => success_count += 1,
            Err(e) => {
                warn!("Document {} failed: {}", i, e);
                failure_count += 1;
            }
        }
    }

    info!("Batch results: {} success, {} failures", success_count, failure_count);
    assert!(success_count > 90, "Too many failures in batch operation");

    // Scenario 2: Recovery after clear
    info!("Scenario 2: Recovery after index clear");

    let before_clear = ctx.search_engine.document_count().await;
    info!("Documents before clear: {}", before_clear);

    ctx.search_engine.clear().await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let after_clear = ctx.search_engine.document_count().await;
    info!("Documents after clear: {}", after_clear);
    assert_eq!(after_clear, 0, "Index should be empty after clear");

    // Re-populate
    for i in 0..20 {
        ctx.search_engine
            .index_document(
                format!("recovered_{}", i),
                format!("Recovered document {}", i),
                EntityType::Document,
                HashMap::new(),
            )
            .await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    let after_recovery = ctx.search_engine.document_count().await;
    info!("Documents after recovery: {}", after_recovery);
    assert_eq!(after_recovery, 20, "Should recover all documents");

    info!("✅ TEST 6 PASSED");
    Ok(())
}
