//! Production-ready connection pool for SurrealDB with advanced features.
//!
//! This module provides enterprise-grade connection pooling with:
//! - Multiple connection modes (local, remote, hybrid)
//! - Load balancing strategies
//! - Health monitoring and auto-reconnect
//! - Connection lifecycle management
//! - Metrics and observability
//! - Session management for multi-agent access
//! - Retry logic with exponential backoff
//! - Circuit breaker for fault tolerance

use anyhow::Context;
use cortex_core::error::{CortexError, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ==============================================================================
// Configuration Types
// ==============================================================================

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_mode: ConnectionMode,
    pub credentials: Credentials,
    pub pool_config: PoolConfig,
    pub namespace: String,
    pub database: String,
}

/// Connection mode for the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionMode {
    /// In-memory embedded mode - for testing, no server required
    InMemory,
    /// Local development mode - single SurrealDB instance
    Local {
        endpoint: String, // ws://localhost:8000
    },
    /// Remote production mode - multiple endpoints with load balancing
    Remote {
        endpoints: Vec<String>,
        load_balancing: LoadBalancingStrategy,
    },
    /// Hybrid mode - local cache with remote sync
    Hybrid {
        local_cache: String,
        remote_sync: Vec<String>,
        sync_interval: Duration,
    },
}

/// Load balancing strategy for remote connections
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LoadBalancingStrategy {
    /// Round-robin: cycle through endpoints in order
    RoundRobin,
    /// Least connections: prefer endpoint with fewest active connections
    LeastConnections,
    /// Random: randomly select endpoint
    Random,
    /// Health-based: prefer healthier endpoints
    HealthBased,
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum number of connections to maintain
    pub min_connections: usize,
    /// Maximum number of connections allowed
    pub max_connections: usize,
    /// Timeout for acquiring a connection from pool
    pub connection_timeout: Duration,
    /// Idle timeout before connection is closed
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection
    pub max_lifetime: Option<Duration>,
    /// Retry policy for failed operations
    pub retry_policy: RetryPolicy,
    /// Enable connection warming on startup
    pub warm_connections: bool,
    /// Validate connections before use
    pub validate_on_checkout: bool,
    /// Enable connection recycling (close and recreate after certain uses)
    pub recycle_after_uses: Option<usize>,
    /// Grace period for shutdown (wait for in-flight operations)
    pub shutdown_grace_period: Duration,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
}

// ==============================================================================
// Connection Manager
// ==============================================================================

/// Main connection manager handling connection lifecycle
pub struct ConnectionManager {
    config: DatabaseConfig,
    pool: Arc<ConnectionPool>,
    health_monitor: Arc<HealthMonitor>,
    metrics: Arc<PoolMetrics>,
    circuit_breaker: Arc<CircuitBreaker>,
    shutdown_signal: Arc<AtomicBool>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        info!("Initializing connection manager");

        // Create connection pool based on mode
        let pool = match &config.connection_mode {
            ConnectionMode::InMemory => {
                ConnectionPool::in_memory(
                    &config.credentials,
                    &config.namespace,
                    &config.database,
                    config.pool_config.clone(),
                )
                .await?
            }
            ConnectionMode::Local { endpoint } => {
                ConnectionPool::single(
                    endpoint.clone(),
                    &config.credentials,
                    &config.namespace,
                    &config.database,
                    config.pool_config.clone(),
                )
                .await?
            }
            ConnectionMode::Remote {
                endpoints,
                load_balancing,
            } => {
                ConnectionPool::multi(
                    endpoints.clone(),
                    *load_balancing,
                    &config.credentials,
                    &config.namespace,
                    &config.database,
                    config.pool_config.clone(),
                )
                .await?
            }
            ConnectionMode::Hybrid {
                local_cache,
                remote_sync,
                sync_interval,
            } => {
                ConnectionPool::hybrid(
                    local_cache.clone(),
                    remote_sync.clone(),
                    *sync_interval,
                    &config.credentials,
                    &config.namespace,
                    &config.database,
                    config.pool_config.clone(),
                )
                .await?
            }
        };

        let pool = Arc::new(pool);
        let metrics = Arc::new(PoolMetrics::new());
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(60)));

        // Start health monitoring
        let health_monitor = HealthMonitor::start(
            pool.clone(),
            Duration::from_secs(30),
            3,
            metrics.clone(),
        );

        info!("Connection manager initialized successfully");

        Ok(Self {
            config,
            pool,
            health_monitor: Arc::new(health_monitor),
            metrics,
            circuit_breaker,
            shutdown_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection> {
        // Check if shutting down
        if self.shutdown_signal.load(Ordering::Relaxed) {
            return Err(CortexError::database("Connection manager is shutting down"));
        }

        // Check circuit breaker
        if !self.circuit_breaker.can_proceed() {
            return Err(CortexError::database(
                "Circuit breaker open - too many failures",
            ));
        }

        let conn = self.pool.acquire().await?;

        // Validate connection if configured
        if self.config.pool_config.validate_on_checkout {
            if !conn.check_health().await {
                warn!("Connection {} failed validation, retrying", conn.id());
                self.metrics.record_error();
                // Try to get another connection
                return self.pool.acquire().await;
            }
        }

        // Check if connection needs recycling using atomic compare-and-swap
        if let Some(max_uses) = self.config.pool_config.recycle_after_uses {
            let uses = conn.uses();
            if uses >= max_uses {
                // Use atomic compare-and-swap to ensure only one thread marks for recycling
                // This prevents race conditions where multiple threads check and mark simultaneously
                if conn.inner.recycle.compare_exchange(
                    false,
                    true,
                    Ordering::SeqCst,
                    Ordering::SeqCst
                ).is_ok() {
                    debug!("Connection {} exceeded max uses ({}), marked for recycling", conn.id(), uses);
                }
            }
        }

        Ok(conn)
    }

    /// Execute a query with retry logic
    pub async fn execute_with_retry<F, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> futures::future::BoxFuture<'static, Result<T>>,
    {
        let retry_policy = &self.config.pool_config.retry_policy;
        let mut attempts = 0;

        loop {
            match operation().await {
                Ok(result) => {
                    self.circuit_breaker.record_success();
                    self.metrics.record_success();
                    return Ok(result);
                }
                Err(e) if attempts < retry_policy.max_attempts && Self::is_retryable(&e) => {
                    attempts += 1;
                    let delay = retry_policy.calculate_delay(attempts);

                    warn!(
                        "Operation failed (attempt {}/{}), retrying in {:?}: {}",
                        attempts, retry_policy.max_attempts, delay, e
                    );

                    self.metrics.record_retry();
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    self.circuit_breaker.record_failure();
                    self.metrics.record_error();
                    return Err(e);
                }
            }
        }
    }

    /// Check if an error is retryable
    fn is_retryable(error: &CortexError) -> bool {
        // Add logic to determine if error is transient
        matches!(
            error,
            CortexError::Database(_) | CortexError::Internal(_)
        )
    }

    /// Get pool metrics
    pub fn metrics(&self) -> &PoolMetrics {
        &self.metrics
    }

    /// Get health status
    pub fn health_status(&self) -> HealthStatus {
        HealthStatus {
            healthy: !self.circuit_breaker.is_open(),
            pool_size: self.pool.current_size(),
            available_connections: self.pool.available_count(),
            total_connections: self.metrics.connections_created.load(Ordering::Relaxed),
            failed_connections: self.metrics.errors.load(Ordering::Relaxed),
            circuit_breaker_state: self.circuit_breaker.state(),
        }
    }

    /// Shutdown the connection manager gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown of connection manager");

        // Signal shutdown
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // Stop accepting new connections
        self.health_monitor.stop();

        // Wait for grace period to allow in-flight operations to complete
        let grace_period = self.config.pool_config.shutdown_grace_period;
        info!("Waiting {:?} for in-flight operations to complete", grace_period);

        let start = Instant::now();
        while start.elapsed() < grace_period {
            let available = self.pool.available_count();
            let total = self.pool.current_size();

            if available == total {
                info!("All connections returned to pool");
                break;
            }

            debug!("Waiting for connections: {}/{} returned", available, total);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Close all connections
        self.pool.close_all().await;

        info!("Connection manager shut down successfully");
        Ok(())
    }

    /// Check if the manager is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_signal.load(Ordering::Relaxed)
    }

    /// Get pool statistics
    pub fn pool_stats(&self) -> PoolStatistics {
        let metrics = self.metrics.snapshot();
        let health = self.health_status();

        PoolStatistics {
            total_connections: health.pool_size,
            available_connections: health.available_connections,
            in_use_connections: health.pool_size.saturating_sub(health.available_connections),
            connections_created: metrics.connections_created,
            connections_reused: metrics.connections_reused,
            connections_closed: metrics.connections_closed,
            health_check_pass_rate: Self::calculate_pass_rate(
                metrics.health_checks_passed,
                metrics.health_checks_failed,
            ),
            acquisition_success_rate: Self::calculate_pass_rate(
                metrics.successes,
                metrics.errors,
            ),
            average_reuse_ratio: Self::calculate_reuse_ratio(
                metrics.connections_reused,
                metrics.connections_created,
            ),
        }
    }

    fn calculate_pass_rate(passed: u64, failed: u64) -> f64 {
        let total = passed + failed;
        if total == 0 {
            0.0
        } else {
            (passed as f64 / total as f64) * 100.0
        }
    }

    fn calculate_reuse_ratio(reused: u64, created: u64) -> f64 {
        let total = reused + created;
        if total == 0 {
            0.0
        } else {
            (reused as f64 / total as f64) * 100.0
        }
    }
}

// ==============================================================================
// Connection Pool
// ==============================================================================

/// Connection pool managing multiple database connections
pub struct ConnectionPool {
    connections: DashMap<Uuid, PooledConnectionInner>,
    available: Arc<Semaphore>,
    config: PoolConfig,
    endpoints: Vec<String>,
    load_balancer: Arc<LoadBalancer>,
    credentials: Credentials,
    namespace: String,
    database: String,
    metrics: Arc<PoolMetrics>,
}

impl ConnectionPool {
    /// Create a pool for embedded in-memory database (testing only)
    async fn in_memory(
        credentials: &Credentials,
        namespace: &str,
        database: &str,
        mut config: PoolConfig,
    ) -> Result<Self> {
        // Disable connection warming for in-memory mode to avoid hanging
        // Connections will be created on-demand instead
        config.warm_connections = false;
        config.min_connections = 0;

        // Use "memory" endpoint for embedded in-memory mode
        Self::new(
            vec!["memory".to_string()],
            LoadBalancingStrategy::RoundRobin,
            credentials,
            namespace,
            database,
            config,
        )
        .await
    }

    /// Create a pool for a single endpoint
    async fn single(
        endpoint: String,
        credentials: &Credentials,
        namespace: &str,
        database: &str,
        config: PoolConfig,
    ) -> Result<Self> {
        Self::new(
            vec![endpoint],
            LoadBalancingStrategy::RoundRobin,
            credentials,
            namespace,
            database,
            config,
        )
        .await
    }

    /// Create a pool for multiple endpoints
    async fn multi(
        endpoints: Vec<String>,
        strategy: LoadBalancingStrategy,
        credentials: &Credentials,
        namespace: &str,
        database: &str,
        config: PoolConfig,
    ) -> Result<Self> {
        Self::new(endpoints, strategy, credentials, namespace, database, config).await
    }

    /// Create a hybrid pool (local + remote)
    async fn hybrid(
        local_cache: String,
        remote_sync: Vec<String>,
        _sync_interval: Duration,
        credentials: &Credentials,
        namespace: &str,
        database: &str,
        config: PoolConfig,
    ) -> Result<Self> {
        let mut endpoints = vec![local_cache];
        endpoints.extend(remote_sync);

        Self::new(
            endpoints,
            LoadBalancingStrategy::HealthBased,
            credentials,
            namespace,
            database,
            config,
        )
        .await
    }

    /// Create a new connection pool
    async fn new(
        endpoints: Vec<String>,
        strategy: LoadBalancingStrategy,
        credentials: &Credentials,
        namespace: &str,
        database: &str,
        config: PoolConfig,
    ) -> Result<Self> {
        let pool = Self {
            connections: DashMap::new(),
            available: Arc::new(Semaphore::new(config.max_connections)),
            config: config.clone(),
            endpoints: endpoints.clone(),
            load_balancer: Arc::new(LoadBalancer::new(endpoints, strategy)),
            credentials: credentials.clone(),
            namespace: namespace.to_string(),
            database: database.to_string(),
            metrics: Arc::new(PoolMetrics::new()),
        };

        // Warm up connections if enabled
        if config.warm_connections {
            pool.warm_up().await?;
        }

        Ok(pool)
    }

    /// Warm up the pool with minimum connections
    async fn warm_up(&self) -> Result<()> {
        info!("Warming up connection pool with {} connections", self.config.min_connections);

        for _ in 0..self.config.min_connections {
            let conn = self.create_connection().await?;
            self.connections.insert(conn.id, conn);
        }

        info!("Connection pool warmed up successfully");
        Ok(())
    }

    /// Create a new connection
    async fn create_connection(&self) -> Result<PooledConnectionInner> {
        let endpoint = self.load_balancer.select_endpoint()?.to_string();

        debug!("Creating connection to endpoint: {}", endpoint);

        // Special handling for in-memory mode
        let db: Surreal<Any> = if endpoint == "memory" {
            // For in-memory mode, we need to use a special connection approach
            // The Any engine doesn't support direct "memory" connections,
            // so we use "mem://" which the Any engine interprets as in-memory
            surrealdb::engine::any::connect("mem://")
                .await
                .context("Failed to connect to in-memory SurrealDB")?
        } else {
            // Connect using Any engine - this accepts the connection string
            surrealdb::engine::any::connect(endpoint)
                .await
                .context("Failed to connect to SurrealDB")?
        };

        // Authenticate first (required before using namespace/database)
        if let (Some(username), Some(password)) = (&self.credentials.username, &self.credentials.password) {
            db.signin(surrealdb::opt::auth::Root {
                username,
                password,
            })
            .await
            .context("Authentication failed")?;
        }

        // Then use namespace and database
        db.use_ns(&self.namespace)
            .use_db(&self.database)
            .await
            .context("Failed to set namespace/database")?;

        let conn = PooledConnectionInner {
            id: Uuid::new_v4(),
            conn: Arc::new(db),
            created_at: Instant::now(),
            last_used: Arc::new(RwLock::new(Instant::now())),
            uses: Arc::new(AtomicUsize::new(0)),
            healthy: Arc::new(AtomicBool::new(true)),
            recycle: Arc::new(AtomicBool::new(false)),
        };

        self.metrics.connections_created.fetch_add(1, Ordering::Relaxed);
        debug!("Connection created: {}", conn.id);

        Ok(conn)
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection> {
        // Wait for available slot with timeout
        let permit = timeout(
            self.config.connection_timeout,
            self.available.clone().acquire_owned(),
        )
        .await
        .map_err(|_| CortexError::database("Connection acquisition timeout"))?
        .map_err(|_| CortexError::database("Semaphore closed"))?;

        // Try to reuse an existing connection
        if let Some(conn) = self.get_healthy_connection().await {
            self.metrics.connections_reused.fetch_add(1, Ordering::Relaxed);
            return Ok(PooledConnection {
                inner: conn,
                pool: Arc::new(self.clone()),
                _permit: permit,
            });
        }

        // Create new connection if we haven't hit the limit
        let conn = self.create_connection().await?;
        let id = conn.id;
        self.connections.insert(id, conn.clone());

        Ok(PooledConnection {
            inner: conn,
            pool: Arc::new(self.clone()),
            _permit: permit,
        })
    }

    /// Get a healthy existing connection
    async fn get_healthy_connection(&self) -> Option<PooledConnectionInner> {
        for entry in self.connections.iter() {
            let conn = entry.value();

            // Check if connection is healthy
            if !conn.is_healthy() {
                continue;
            }

            // Check if connection has exceeded max lifetime
            if let Some(max_lifetime) = self.config.max_lifetime {
                if conn.created_at.elapsed() > max_lifetime {
                    continue;
                }
            }

            // Check if connection has been idle too long
            if let Some(idle_timeout) = self.config.idle_timeout {
                if conn.last_used.read().elapsed() > idle_timeout {
                    continue;
                }
            }

            return Some(conn.clone());
        }

        None
    }

    /// Return a connection to the pool
    fn return_connection(&self, conn: PooledConnectionInner) {
        // Don't return if marked for recycling
        if conn.should_recycle() {
            debug!("Connection {} marked for recycling, discarding", conn.id);
            self.connections.remove(&conn.id);
            self.metrics.connections_closed.fetch_add(1, Ordering::Relaxed);
            return;
        }

        if conn.is_healthy() && !conn.is_expired(&self.config) {
            *conn.last_used.write() = Instant::now();
            self.connections.insert(conn.id, conn);
        } else {
            debug!("Connection {} discarded (unhealthy or expired)", conn.id);
            self.connections.remove(&conn.id);
            self.metrics.connections_closed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get current pool size
    pub fn current_size(&self) -> usize {
        self.connections.len()
    }

    /// Get number of available connections
    pub fn available_count(&self) -> usize {
        self.available.available_permits()
    }

    /// Cleanup expired connections
    async fn cleanup_expired(&self) {
        let mut to_remove = Vec::new();

        for entry in self.connections.iter() {
            let conn = entry.value();
            if conn.is_expired(&self.config) || !conn.is_healthy() {
                to_remove.push(*entry.key());
            }
        }

        for id in to_remove {
            debug!("Removing expired connection: {}", id);
            self.connections.remove(&id);
        }
    }

    /// Close all connections
    pub async fn close_all(&self) {
        info!("Closing all connections in pool");
        self.connections.clear();
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            available: self.available.clone(),
            config: self.config.clone(),
            endpoints: self.endpoints.clone(),
            load_balancer: self.load_balancer.clone(),
            credentials: self.credentials.clone(),
            namespace: self.namespace.clone(),
            database: self.database.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

// ==============================================================================
// Pooled Connection
// ==============================================================================

/// Internal connection data
#[derive(Clone)]
struct PooledConnectionInner {
    id: Uuid,
    conn: Arc<Surreal<Any>>,
    created_at: Instant,
    last_used: Arc<RwLock<Instant>>,
    uses: Arc<AtomicUsize>,
    healthy: Arc<AtomicBool>,
    recycle: Arc<AtomicBool>,
}

impl PooledConnectionInner {
    fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    fn should_recycle(&self) -> bool {
        self.recycle.load(Ordering::Relaxed)
    }

    fn is_expired(&self, config: &PoolConfig) -> bool {
        if let Some(max_lifetime) = config.max_lifetime {
            if self.created_at.elapsed() > max_lifetime {
                return true;
            }
        }

        if let Some(idle_timeout) = config.idle_timeout {
            if self.last_used.read().elapsed() > idle_timeout {
                return true;
            }
        }

        false
    }
}

/// A connection from the pool with automatic return on drop
pub struct PooledConnection {
    inner: PooledConnectionInner,
    pool: Arc<ConnectionPool>,
    _permit: OwnedSemaphorePermit,
}

impl PooledConnection {
    /// Get the underlying Surreal connection
    pub fn connection(&self) -> &Surreal<Any> {
        &self.inner.conn
    }

    /// Get connection ID
    pub fn id(&self) -> Uuid {
        self.inner.id
    }

    /// Get number of times connection has been used
    pub fn uses(&self) -> usize {
        self.inner.uses.load(Ordering::Relaxed)
    }

    /// Increment use counter
    pub fn increment_uses(&self) {
        self.inner.uses.fetch_add(1, Ordering::Relaxed);
        *self.inner.last_used.write() = Instant::now();
    }

    /// Check connection health
    pub async fn check_health(&self) -> bool {
        // Try a simple info query to verify connection health
        // Using INFO FOR DB which is lightweight and always available
        let result = self.connection()
            .query("INFO FOR DB")
            .await;

        let healthy = result.is_ok();
        self.inner.healthy.store(healthy, Ordering::Relaxed);
        healthy
    }

    /// Mark connection for recycling
    pub fn mark_for_recycling(&self) {
        self.inner.recycle.store(true, Ordering::Relaxed);
    }

    /// Check if connection is marked for recycling
    pub fn is_marked_for_recycling(&self) -> bool {
        self.inner.recycle.load(Ordering::Relaxed)
    }

    /// Begin a transaction
    pub async fn begin_transaction(&self) -> Result<()> {
        self.connection()
            .query("BEGIN TRANSACTION")
            .await
            .context("Failed to begin transaction")?;
        Ok(())
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self) -> Result<()> {
        self.connection()
            .query("COMMIT TRANSACTION")
            .await
            .context("Failed to commit transaction")?;
        Ok(())
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self) -> Result<()> {
        self.connection()
            .query("CANCEL TRANSACTION")
            .await
            .context("Failed to rollback transaction")?;
        Ok(())
    }

    /// Create a savepoint
    pub async fn savepoint(&self, name: &str) -> Result<()> {
        let query = format!("DEFINE SAVEPOINT {}", name);
        self.connection()
            .query(&query)
            .await
            .context("Failed to create savepoint")?;
        Ok(())
    }

    /// Rollback to a savepoint
    pub async fn rollback_to_savepoint(&self, name: &str) -> Result<()> {
        let query = format!("ROLLBACK TO SAVEPOINT {}", name);
        self.connection()
            .query(&query)
            .await
            .context("Failed to rollback to savepoint")?;
        Ok(())
    }

    /// Execute a query within a transaction with automatic rollback on error
    pub async fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&PooledConnection) -> futures::future::BoxFuture<'_, Result<T>>,
    {
        self.begin_transaction().await?;

        match f(self).await {
            Ok(result) => {
                self.commit_transaction().await?;
                Ok(result)
            }
            Err(e) => {
                if let Err(rollback_err) = self.rollback_transaction().await {
                    warn!("Failed to rollback transaction: {}", rollback_err);
                }
                Err(e)
            }
        }
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        self.pool.return_connection(self.inner.clone());
    }
}

// ==============================================================================
// Load Balancer
// ==============================================================================

/// Load balancer for distributing connections across endpoints
struct LoadBalancer {
    endpoints: Vec<String>,
    strategy: LoadBalancingStrategy,
    round_robin_counter: AtomicUsize,
    endpoint_stats: DashMap<String, EndpointStats>,
}

#[derive(Default)]
struct EndpointStats {
    active_connections: AtomicUsize,
    #[allow(dead_code)]
    total_requests: AtomicU64,
    failures: AtomicU64,
}

impl LoadBalancer {
    fn new(endpoints: Vec<String>, strategy: LoadBalancingStrategy) -> Self {
        let endpoint_stats = DashMap::new();
        for endpoint in &endpoints {
            endpoint_stats.insert(endpoint.clone(), EndpointStats::default());
        }

        Self {
            endpoints,
            strategy,
            round_robin_counter: AtomicUsize::new(0),
            endpoint_stats,
        }
    }

    fn select_endpoint(&self) -> Result<&str> {
        if self.endpoints.is_empty() {
            return Err(CortexError::config("No endpoints available"));
        }

        let endpoint = match self.strategy {
            LoadBalancingStrategy::RoundRobin => self.round_robin(),
            LoadBalancingStrategy::LeastConnections => self.least_connections(),
            LoadBalancingStrategy::Random => self.random(),
            LoadBalancingStrategy::HealthBased => self.health_based(),
        };

        Ok(endpoint)
    }

    fn round_robin(&self) -> &str {
        let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
        &self.endpoints[index % self.endpoints.len()]
    }

    fn least_connections(&self) -> &str {
        self.endpoints
            .iter()
            .min_by_key(|endpoint| {
                self.endpoint_stats
                    .get(*endpoint)
                    .map(|stats| stats.active_connections.load(Ordering::Relaxed))
                    .unwrap_or(0)
            })
            .map(|s| s.as_str())
            .unwrap_or(&self.endpoints[0])
    }

    fn random(&self) -> &str {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let hasher = RandomState::new();
        let mut hasher = hasher.build_hasher();
        Instant::now().hash(&mut hasher);
        let index = (hasher.finish() as usize) % self.endpoints.len();

        &self.endpoints[index]
    }

    fn health_based(&self) -> &str {
        self.endpoints
            .iter()
            .min_by_key(|endpoint| {
                self.endpoint_stats
                    .get(*endpoint)
                    .map(|stats| stats.failures.load(Ordering::Relaxed))
                    .unwrap_or(0)
            })
            .map(|s| s.as_str())
            .unwrap_or(&self.endpoints[0])
    }

    #[allow(dead_code)]
    fn record_connection(&self, endpoint: &str) {
        if let Some(stats) = self.endpoint_stats.get(endpoint) {
            stats.active_connections.fetch_add(1, Ordering::Relaxed);
            stats.total_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[allow(dead_code)]
    fn record_failure(&self, endpoint: &str) {
        if let Some(stats) = self.endpoint_stats.get(endpoint) {
            stats.failures.fetch_add(1, Ordering::Relaxed);
        }
    }
}

// ==============================================================================
// Health Monitor
// ==============================================================================

/// Health monitor for connections
pub struct HealthMonitor {
    pool: Arc<ConnectionPool>,
    check_interval: Duration,
    #[allow(dead_code)]
    unhealthy_threshold: u32,
    metrics: Arc<PoolMetrics>,
    running: Arc<AtomicBool>,
}

impl HealthMonitor {
    /// Start health monitoring
    fn start(
        pool: Arc<ConnectionPool>,
        check_interval: Duration,
        unhealthy_threshold: u32,
        metrics: Arc<PoolMetrics>,
    ) -> Self {
        let monitor = Self {
            pool: pool.clone(),
            check_interval,
            unhealthy_threshold,
            metrics: metrics.clone(),
            running: Arc::new(AtomicBool::new(true)),
        };

        // Spawn monitoring task
        let monitor_clone = Self {
            pool,
            check_interval,
            unhealthy_threshold,
            metrics,
            running: monitor.running.clone(),
        };

        tokio::spawn(async move {
            monitor_clone.run().await;
        });

        monitor
    }

    /// Run health check loop
    async fn run(&self) {
        let mut interval = tokio::time::interval(self.check_interval);

        // Skip the first immediate tick to allow connections to stabilize
        interval.tick().await;

        while self.running.load(Ordering::Relaxed) {
            interval.tick().await;
            self.check_health().await;
            self.pool.cleanup_expired().await;
        }
    }

    /// Check health of all connections
    async fn check_health(&self) {
        debug!("Running health check on connection pool");

        for entry in self.pool.connections.iter() {
            let conn = entry.value();

            // Create a temporary pooled connection for health check
            if let Ok(permit) = self.pool.available.clone().try_acquire_owned() {
                let pooled = PooledConnection {
                    inner: conn.clone(),
                    pool: self.pool.clone(),
                    _permit: permit,
                };

                let healthy = pooled.check_health().await;

                if healthy {
                    self.metrics.health_checks_passed.fetch_add(1, Ordering::Relaxed);
                } else {
                    self.metrics.health_checks_failed.fetch_add(1, Ordering::Relaxed);
                    warn!("Connection {} failed health check", conn.id);
                }
            }
        }
    }

    /// Stop health monitoring
    fn stop(&self) {
        info!("Stopping health monitor");
        self.running.store(false, Ordering::Relaxed);
    }
}

// ==============================================================================
// Metrics
// ==============================================================================

/// Pool metrics for monitoring
#[derive(Default)]
pub struct PoolMetrics {
    pub connections_created: AtomicU64,
    pub connections_reused: AtomicU64,
    pub connections_closed: AtomicU64,
    pub acquisitions: AtomicU64,
    pub acquisition_timeouts: AtomicU64,
    pub health_checks_passed: AtomicU64,
    pub health_checks_failed: AtomicU64,
    pub retries: AtomicU64,
    pub successes: AtomicU64,
    pub errors: AtomicU64,
}

impl PoolMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_success(&self) {
        self.successes.fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connections_created: self.connections_created.load(Ordering::Relaxed),
            connections_reused: self.connections_reused.load(Ordering::Relaxed),
            connections_closed: self.connections_closed.load(Ordering::Relaxed),
            acquisitions: self.acquisitions.load(Ordering::Relaxed),
            acquisition_timeouts: self.acquisition_timeouts.load(Ordering::Relaxed),
            health_checks_passed: self.health_checks_passed.load(Ordering::Relaxed),
            health_checks_failed: self.health_checks_failed.load(Ordering::Relaxed),
            retries: self.retries.load(Ordering::Relaxed),
            successes: self.successes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub connections_created: u64,
    pub connections_reused: u64,
    pub connections_closed: u64,
    pub acquisitions: u64,
    pub acquisition_timeouts: u64,
    pub health_checks_passed: u64,
    pub health_checks_failed: u64,
    pub retries: u64,
    pub successes: u64,
    pub errors: u64,
}

// ==============================================================================
// Circuit Breaker
// ==============================================================================

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_threshold: u32,
    timeout: Duration,
    failures: AtomicU32,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_threshold,
            timeout,
            failures: AtomicU32::new(0),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    fn can_proceed(&self) -> bool {
        let mut state = self.state.write();

        match *state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read() {
                    if last_failure.elapsed() > self.timeout {
                        *state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    fn record_success(&self) {
        let mut state = self.state.write();

        if *state == CircuitBreakerState::HalfOpen {
            *state = CircuitBreakerState::Closed;
            self.failures.store(0, Ordering::Relaxed);
        }
    }

    fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure_time.write() = Some(Instant::now());

        if failures >= self.failure_threshold {
            let mut state = self.state.write();
            *state = CircuitBreakerState::Open;
            warn!("Circuit breaker opened after {} failures", failures);
        }
    }

    fn is_open(&self) -> bool {
        *self.state.read() == CircuitBreakerState::Open
    }

    fn state(&self) -> CircuitBreakerState {
        *self.state.read()
    }
}

// ==============================================================================
// Session Management
// ==============================================================================

/// Resource limits for agent sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum number of concurrent connections per session
    pub max_concurrent_connections: usize,
    /// Maximum total operations allowed per session
    pub max_operations: u64,
    /// Maximum transaction log size
    pub max_transaction_log_size: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_concurrent_connections: 5,
            max_operations: 10000,
            max_transaction_log_size: 1000,
        }
    }
}

/// Session metrics for tracking
#[derive(Default)]
struct SessionMetrics {
    active_connections: AtomicUsize,
    total_operations: AtomicU64,
}

impl SessionMetrics {
    fn new() -> Self {
        Self::default()
    }
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub agent_id: String,
    pub session_id: Uuid,
    pub namespace: String,
    pub active_connections: usize,
    pub total_operations: u64,
    pub total_transactions: usize,
    pub committed_transactions: usize,
    pub aborted_transactions: usize,
    pub resource_limits: ResourceLimits,
}

/// Agent session for multi-agent access
pub struct AgentSession {
    pub agent_id: String,
    pub session_id: Uuid,
    connection: Arc<ConnectionManager>,
    pub namespace: String,
    transaction_log: Arc<RwLock<Vec<Transaction>>>,
    resource_limits: ResourceLimits,
    session_metrics: Arc<SessionMetrics>,
}

/// Transaction record
#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub timestamp: Instant,
    pub operation: TransactionOperation,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOperation {
    Read { path: String },
    Write { path: String, content_hash: String },
    Delete { path: String },
    Query { query: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Committed,
    Aborted,
}

impl AgentSession {
    /// Create a new agent session
    pub async fn create(
        agent_id: String,
        connection: Arc<ConnectionManager>,
        namespace: String,
    ) -> Result<Self> {
        Self::create_with_limits(agent_id, connection, namespace, ResourceLimits::default()).await
    }

    /// Create a new agent session with custom resource limits
    pub async fn create_with_limits(
        agent_id: String,
        connection: Arc<ConnectionManager>,
        namespace: String,
        resource_limits: ResourceLimits,
    ) -> Result<Self> {
        let session_id = Uuid::new_v4();

        info!("Creating agent session {} for agent {} with limits {:?}",
            session_id, agent_id, resource_limits);

        Ok(Self {
            agent_id,
            session_id,
            connection,
            namespace,
            transaction_log: Arc::new(RwLock::new(Vec::new())),
            resource_limits,
            session_metrics: Arc::new(SessionMetrics::new()),
        })
    }

    /// Record a transaction
    pub fn record_transaction(&self, operation: TransactionOperation) -> Uuid {
        let transaction = Transaction {
            id: Uuid::new_v4(),
            timestamp: Instant::now(),
            operation,
            status: TransactionStatus::Pending,
        };

        let id = transaction.id;
        self.transaction_log.write().push(transaction);
        id
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, transaction_id: Uuid) {
        let mut log = self.transaction_log.write();
        if let Some(txn) = log.iter_mut().find(|t| t.id == transaction_id) {
            txn.status = TransactionStatus::Committed;
        }
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, transaction_id: Uuid) {
        let mut log = self.transaction_log.write();
        if let Some(txn) = log.iter_mut().find(|t| t.id == transaction_id) {
            txn.status = TransactionStatus::Aborted;
        }
    }

    /// Get transaction history
    pub fn transaction_history(&self) -> Vec<Transaction> {
        self.transaction_log.read().clone()
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnection> {
        // Check resource limits
        if self.session_metrics.active_connections.load(Ordering::Relaxed)
            >= self.resource_limits.max_concurrent_connections {
            return Err(CortexError::concurrency(
                format!("Session {} exceeded max concurrent connections limit", self.session_id)
            ));
        }

        if self.session_metrics.total_operations.load(Ordering::Relaxed)
            >= self.resource_limits.max_operations {
            return Err(CortexError::concurrency(
                format!("Session {} exceeded max operations limit", self.session_id)
            ));
        }

        let conn = self.connection.acquire().await?;
        self.session_metrics.active_connections.fetch_add(1, Ordering::Relaxed);
        self.session_metrics.total_operations.fetch_add(1, Ordering::Relaxed);

        Ok(conn)
    }

    /// Execute a query in this session's namespace
    pub async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnMut() -> futures::future::BoxFuture<'static, Result<T>>,
    {
        self.connection.execute_with_retry(operation).await
    }

    /// Get session statistics
    pub fn session_stats(&self) -> SessionStatistics {
        SessionStatistics {
            agent_id: self.agent_id.clone(),
            session_id: self.session_id,
            namespace: self.namespace.clone(),
            active_connections: self.session_metrics.active_connections.load(Ordering::Relaxed),
            total_operations: self.session_metrics.total_operations.load(Ordering::Relaxed),
            total_transactions: self.transaction_log.read().len(),
            committed_transactions: self.transaction_log.read().iter()
                .filter(|t| t.status == TransactionStatus::Committed)
                .count(),
            aborted_transactions: self.transaction_log.read().iter()
                .filter(|t| t.status == TransactionStatus::Aborted)
                .count(),
            resource_limits: self.resource_limits.clone(),
        }
    }

    /// Check if session is within resource limits
    pub fn is_within_limits(&self) -> bool {
        self.session_metrics.active_connections.load(Ordering::Relaxed)
            < self.resource_limits.max_concurrent_connections
            && self.session_metrics.total_operations.load(Ordering::Relaxed)
            < self.resource_limits.max_operations
    }
}

// ==============================================================================
// Retry Policy Implementation
// ==============================================================================

impl RetryPolicy {
    /// Calculate backoff delay for given attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.initial_backoff.as_secs_f64()
            * self.multiplier.powi(attempt as i32 - 1);

        let delay = Duration::from_secs_f64(delay.min(self.max_backoff.as_secs_f64()));

        delay
    }
}

// ==============================================================================
// Health Status
// ==============================================================================

/// Health status of the connection pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub pool_size: usize,
    pub available_connections: usize,
    pub total_connections: u64,
    pub failed_connections: u64,
    pub circuit_breaker_state: CircuitBreakerState,
}

/// Pool statistics for monitoring and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatistics {
    pub total_connections: usize,
    pub available_connections: usize,
    pub in_use_connections: usize,
    pub connections_created: u64,
    pub connections_reused: u64,
    pub connections_closed: u64,
    pub health_check_pass_rate: f64,
    pub acquisition_success_rate: f64,
    pub average_reuse_ratio: f64,
}

// ==============================================================================
// Default Implementations
// ==============================================================================

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,                             // Optimized: More min connections (was 2)
            max_connections: 20,                            // Optimized: Higher max for concurrency (was 10)
            connection_timeout: Duration::from_secs(10),    // Optimized: Faster timeout (was 30)
            idle_timeout: Some(Duration::from_secs(300)), // 5 minutes
            max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes
            retry_policy: RetryPolicy::default(),
            warm_connections: true,
            validate_on_checkout: false,                    // Optimized: Disabled for speed
            recycle_after_uses: Some(10000),               // Optimized: Recycle after 10k uses
            shutdown_grace_period: Duration::from_secs(30),
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(50),     // Optimized: Faster initial retry (was 100)
            max_backoff: Duration::from_secs(5),            // Optimized: Lower max backoff (was 10)
            multiplier: 1.5,                                 // Optimized: Gentler backoff (was 2.0)
        }
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Self {
            username: None,
            password: None,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            connection_mode: ConnectionMode::Local {
                endpoint: "ws://localhost:8000".to_string(),
            },
            credentials: Credentials::default(),
            pool_config: PoolConfig::default(),
            namespace: "cortex".to_string(),
            database: "main".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert_eq!(config.min_connections, 5);  // Updated to match optimized default
        assert_eq!(config.max_connections, 20); // Updated to match optimized default
        assert!(config.warm_connections);
    }

    #[tokio::test]
    async fn test_retry_policy() {
        let policy = RetryPolicy::default();

        let delay1 = policy.calculate_delay(1);
        let delay2 = policy.calculate_delay(2);
        let delay3 = policy.calculate_delay(3);

        assert!(delay2 > delay1);
        assert!(delay3 > delay2);
        assert!(delay3 <= policy.max_backoff);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        assert!(cb.can_proceed());
        assert_eq!(cb.state(), CircuitBreakerState::Closed);

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitBreakerState::Open);
        assert!(!cb.can_proceed());
    }

    #[tokio::test]
    async fn test_load_balancer_round_robin() {
        let endpoints = vec![
            "endpoint1".to_string(),
            "endpoint2".to_string(),
            "endpoint3".to_string(),
        ];

        let lb = LoadBalancer::new(endpoints, LoadBalancingStrategy::RoundRobin);

        let e1 = lb.select_endpoint().unwrap();
        let e2 = lb.select_endpoint().unwrap();
        let e3 = lb.select_endpoint().unwrap();
        let e4 = lb.select_endpoint().unwrap();

        assert_eq!(e1, "endpoint1");
        assert_eq!(e2, "endpoint2");
        assert_eq!(e3, "endpoint3");
        assert_eq!(e4, "endpoint1");
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = PoolMetrics::new();

        metrics.connections_created.store(5, Ordering::Relaxed);
        metrics.connections_reused.store(10, Ordering::Relaxed);
        metrics.successes.store(15, Ordering::Relaxed);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.connections_created, 5);
        assert_eq!(snapshot.connections_reused, 10);
        assert_eq!(snapshot.successes, 15);
    }
}
