# Cortex: Multi-Agent Data Layer

## Overview

The Multi-Agent Data Layer provides isolated data access and conflict resolution for multiple agents working concurrently on the same codebase. While **Axon** handles agent orchestration, workflow execution, and coordination, **Cortex** provides the underlying data isolation, versioning, and merge capabilities that enable safe concurrent operations.

**Key Principle:** Cortex is the data layer, Axon is the orchestration layer.

## Scope & Responsibilities

### What Cortex Provides (Data Layer)

- ✅ **Session Management** - Isolated namespaces for agent data access
- ✅ **Lock Management** - Fine-grained data-level locking
- ✅ **Conflict Resolution** - Three-way merge and conflict detection
- ✅ **Storage for Agents** - Persistent state and change tracking
- ✅ **Versioning** - History tracking for merge operations

### What Axon Provides (Orchestration Layer)

- ❌ Workflow Orchestration → Axon
- ❌ Agent Communication → Axon
- ❌ Task Assignment → Axon
- ❌ Coordination Protocols → Axon
- ❌ Agent Lifecycle Management → Axon

## Core Concepts

### Data Isolation Architecture

```
Axon Orchestration Layer
    ├─ Workflow Engine (DAG execution)
    ├─ Agent Manager (lifecycle)
    └─ Message Bus (communication)
         ↓ REST API
Cortex Data Layer
    ├─ Session A (Agent 1) - Isolated namespace
    ├─ Session B (Agent 2) - Isolated namespace
    └─ Session C (Agent 3) - Isolated namespace
```

### Lock Hierarchy

Fine-grained locking prevents data conflicts:

```
Workspace Lock
    ├─ Directory Lock
    │   ├─ File Lock
    │   │   └─ Unit Lock (function/class level)
```

## Session Management

### Session Architecture

```rust
struct Session {
    id: SessionId,
    agent_id: AgentId,
    workspace_id: WorkspaceId,

    // Isolation configuration
    isolation_level: IsolationLevel,
    base_version: Version,

    // Scope definition
    scope: SessionScope,

    // Change tracking
    changes: ChangeSet,

    // State
    status: SessionStatus,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

enum IsolationLevel {
    Snapshot,       // Complete isolation, changes invisible to others
    ReadCommitted,  // See committed changes from other sessions
    Serializable,   // Strong consistency guarantees
}

struct SessionScope {
    paths: Vec<PathPattern>,           // Which paths can be accessed
    units: Vec<UnitId>,                // Specific units to work on
    read_only_paths: Vec<PathPattern>, // Read-only access
}
```

### Session Lifecycle

#### Creation via REST API

Axon creates sessions for agents via Cortex API:

```rust
// POST /sessions
{
    "agent_id": "agent-123",
    "workspace_id": "ws-456",
    "isolation_level": "Snapshot",
    "scope": {
        "paths": ["src/auth/**", "tests/auth/**"],
        "read_only_paths": ["src/common/**"]
    },
    "ttl": 3600
}
```

Implementation:

```rust
impl SessionManager {
    async fn create_session(
        &self,
        agent_id: AgentId,
        config: SessionConfig
    ) -> Result<Session> {
        // 1. Verify scope permissions
        self.verify_permissions(&agent_id, &config.scope)?;

        // 2. Create session namespace in SurrealDB
        let namespace = format!("session_{}", generate_id());
        self.db.use_ns(&namespace).await?;

        // 3. Fork current state (copy-on-write)
        let base_version = self.get_current_version().await?;
        self.fork_state(&namespace, base_version, &config.scope).await?;

        // 4. Create session record
        let session = Session {
            id: SessionId::new(),
            agent_id,
            workspace_id: self.workspace_id,
            isolation_level: config.isolation_level,
            base_version,
            scope: config.scope,
            changes: ChangeSet::new(),
            status: SessionStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + config.ttl,
        };

        // 5. Register session
        self.register_session(&session).await?;

        Ok(session)
    }

    async fn fork_state(
        &self,
        namespace: &str,
        version: Version,
        scope: &SessionScope
    ) -> Result<()> {
        // Copy only relevant vnodes (copy-on-write)
        for pattern in &scope.paths {
            self.db.query(&format!("
                USE NS {};
                INSERT INTO vnode
                SELECT * FROM cortex.knowledge.vnode  -- Legacy: was 'meridian' namespace
                WHERE path MATCHES $pattern
                  AND version <= $version
            ", namespace), &[
                ("pattern", pattern),
                ("version", &version),
            ]).await?;
        }

        // Copy relevant code units
        self.db.query(&format!("
            USE NS {};
            INSERT INTO code_unit
            SELECT * FROM cortex.knowledge.code_unit  -- Legacy: was 'meridian' namespace
            WHERE file_node IN (SELECT id FROM vnode)
        ", namespace)).await?;

        // Copy relationships
        self.copy_relationships(namespace).await?;

        Ok(())
    }
}
```

#### Working in Session

Agents read/write data within their isolated session:

```rust
impl Session {
    async fn read(&self, path: &str) -> Result<VNode> {
        // Check scope permissions
        if !self.can_read(path) {
            return Err(Error::OutOfScope);
        }

        // Read from session namespace
        let query = format!("
            USE NS session_{};
            SELECT * FROM vnode WHERE path = $path
        ", self.id);

        self.db.query(&query, &[("path", path)]).await
    }

    async fn write(&mut self, path: &str, content: &str) -> Result<()> {
        // Check write permissions
        if !self.can_write(path) {
            return Err(Error::ReadOnly);
        }

        // Write to session namespace
        let vnode = self.get_or_create_vnode(path).await?;

        // Track change for merge
        self.changes.record_change(Change {
            path: path.to_string(),
            operation: if vnode.is_new {
                Operation::Create
            } else {
                Operation::Modify
            },
            old_content: vnode.content_hash.clone(),
            new_content: hash(content),
        });

        // Update vnode in session namespace
        self.update_vnode(&vnode, content).await?;

        Ok(())
    }

    fn can_read(&self, path: &str) -> bool {
        self.scope.paths.iter().any(|p| p.matches(path)) ||
        self.scope.read_only_paths.iter().any(|p| p.matches(path))
    }

    fn can_write(&self, path: &str) -> bool {
        self.scope.paths.iter().any(|p| p.matches(path)) &&
        !self.scope.read_only_paths.iter().any(|p| p.matches(path))
    }
}
```

#### REST API Endpoints

```rust
// GET /sessions/{id}
// Returns session status and metadata

// GET /sessions/{id}/files/{path}
// Read file from session namespace

// PUT /sessions/{id}/files/{path}
// Write file to session namespace

// POST /sessions/{id}/merge
// Merge session changes back to main
{
    "strategy": "Auto" | "Manual" | "Theirs" | "Mine" | "Force"
}

// DELETE /sessions/{id}
// Close and cleanup session
```

#### Merging Changes

Axon triggers merge when agent completes its work:

```rust
impl SessionManager {
    async fn merge_session(
        &self,
        session_id: SessionId,
        strategy: MergeStrategy
    ) -> Result<MergeReport> {
        let session = self.get_session(session_id)?;

        // 1. Detect conflicts by comparing versions
        let conflicts = self.detect_conflicts(&session).await?;

        if !conflicts.is_empty() && strategy != MergeStrategy::Force {
            return match strategy {
                MergeStrategy::Auto => self.auto_resolve_conflicts(conflicts).await,
                MergeStrategy::Manual => Err(Error::ConflictsRequireResolution(conflicts)),
                MergeStrategy::Theirs => self.use_theirs(conflicts).await,
                MergeStrategy::Mine => self.use_mine(conflicts).await,
                _ => unreachable!()
            };
        }

        // 2. Apply changes to main branch
        let mut report = MergeReport::new();

        for change in session.changes.iter() {
            match self.apply_change(change).await {
                Ok(_) => report.successful += 1,
                Err(e) => {
                    report.failed += 1;
                    report.errors.push(e);
                }
            }
        }

        // 3. Update version tracking
        self.update_versions(&session).await?;

        // 4. Clean up session namespace
        self.close_session(session_id).await?;

        Ok(report)
    }

    async fn detect_conflicts(&self, session: &Session) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        for change in session.changes.iter() {
            // Check if entity was modified after session started
            let current_version = self.get_version(&change.path).await?;

            if current_version > session.base_version {
                // Get all three versions for three-way merge
                let session_content = self.get_session_content(&change.path, session).await?;
                let main_content = self.get_main_content(&change.path).await?;
                let base_content = self.get_base_content(&change.path, session.base_version).await?;

                conflicts.push(Conflict {
                    path: change.path.clone(),
                    base: base_content,
                    mine: session_content,
                    theirs: main_content,
                });
            }
        }

        Ok(conflicts)
    }
}
```

### Session Caching

Performance optimization for session operations:

```rust
struct SessionCache {
    cache: Arc<RwLock<LruCache<SessionId, SessionState>>>,
}

struct SessionState {
    vnodes: HashMap<PathBuf, VNode>,
    units: HashMap<UnitId, CodeUnit>,
    last_accessed: DateTime<Utc>,
}

impl SessionCache {
    async fn get_vnode(&self, session_id: &SessionId, path: &Path) -> Option<VNode> {
        let mut cache = self.cache.write().await;

        if let Some(state) = cache.get_mut(session_id) {
            state.last_accessed = Utc::now();
            return state.vnodes.get(path).cloned();
        }

        None
    }

    async fn prefetch(&self, session_id: &SessionId, paths: Vec<PathBuf>) {
        // Batch fetch from database
        let vnodes = self.batch_fetch_vnodes(&paths).await?;

        let mut cache = self.cache.write().await;
        let state = cache.entry(session_id.clone())
            .or_insert_with(|| SessionState::new());

        for (path, vnode) in paths.into_iter().zip(vnodes) {
            state.vnodes.insert(path, vnode);
        }
    }
}
```

## Lock Management

### Lock System Architecture

Prevents concurrent data modifications at fine-grained level:

```rust
struct LockManager {
    locks: Arc<RwLock<HashMap<EntityId, Lock>>>,
    wait_queue: Arc<RwLock<HashMap<EntityId, Vec<LockRequest>>>>,
}

struct Lock {
    id: LockId,
    entity_id: EntityId,
    lock_type: LockType,
    owner: AgentId,
    session_id: SessionId,
    acquired_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    scope: LockScope,
}

enum LockType {
    Exclusive,       // Write lock
    Shared,          // Read lock
    IntentExclusive, // Intent to acquire exclusive
    IntentShared,    // Intent to acquire shared
}

enum LockScope {
    Entity,    // Just this entity
    Subtree,   // Entity and all children
    File,      // Entire file
    Directory, // Entire directory
}
```

### Lock Acquisition

```rust
impl LockManager {
    async fn acquire_lock(
        &self,
        request: LockRequest
    ) -> Result<LockHandle> {
        // 1. Check compatibility with existing locks
        let compatible = self.check_compatibility(&request).await?;

        if !compatible {
            // 2. Add to wait queue if needed
            if request.wait {
                return self.wait_for_lock(request).await;
            } else {
                return Err(Error::LockConflict);
            }
        }

        // 3. Check for deadlock
        if self.would_cause_deadlock(&request).await? {
            return Err(Error::DeadlockDetected);
        }

        // 4. Grant lock
        let lock = Lock {
            id: LockId::new(),
            entity_id: request.entity_id,
            lock_type: request.lock_type,
            owner: request.agent_id,
            session_id: request.session_id,
            acquired_at: Utc::now(),
            expires_at: Utc::now() + request.timeout,
            scope: request.scope,
        };

        // 5. Register lock
        self.register_lock(lock.clone()).await?;

        // 6. Return handle
        Ok(LockHandle {
            lock_id: lock.id,
            manager: self.clone(),
        })
    }

    async fn check_compatibility(&self, request: &LockRequest) -> Result<bool> {
        let locks = self.locks.read().await;

        for existing in locks.values() {
            if self.locks_conflict(existing, request) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn locks_conflict(&self, existing: &Lock, request: &LockRequest) -> bool {
        // Check if entities overlap
        if !self.entities_overlap(&existing.entity_id, &request.entity_id, &existing.scope, &request.scope) {
            return false;
        }

        // Check lock type compatibility
        match (existing.lock_type, request.lock_type) {
            (LockType::Shared, LockType::Shared) => false,
            (LockType::IntentShared, LockType::Shared) => false,
            (LockType::IntentShared, LockType::IntentShared) => false,
            _ => true,  // All other combinations conflict
        }
    }
}
```

### Deadlock Prevention

```rust
impl LockManager {
    async fn would_cause_deadlock(&self, request: &LockRequest) -> Result<bool> {
        // Build wait-for graph
        let graph = self.build_wait_for_graph().await?;

        // Add edge for this request
        let blocks = self.get_blocking_agents(&request).await?;
        for blocker in blocks {
            graph.add_edge(request.agent_id, blocker);
        }

        // Check for cycles
        Ok(graph.has_cycle())
    }

    async fn build_wait_for_graph(&self) -> Result<WaitForGraph> {
        let mut graph = WaitForGraph::new();

        let queue = self.wait_queue.read().await;
        let locks = self.locks.read().await;

        for (entity_id, waiters) in queue.iter() {
            if let Some(lock) = locks.get(entity_id) {
                for waiter in waiters {
                    // Waiter is waiting for lock owner
                    graph.add_edge(waiter.agent_id, lock.owner);
                }
            }
        }

        Ok(graph)
    }
}

struct WaitForGraph {
    edges: HashMap<AgentId, HashSet<AgentId>>,
}

impl WaitForGraph {
    fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.edges.keys() {
            if !visited.contains(node) {
                if self.has_cycle_util(node, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn has_cycle_util(
        &self,
        node: &AgentId,
        visited: &mut HashSet<AgentId>,
        rec_stack: &mut HashSet<AgentId>
    ) -> bool {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(neighbors) = self.edges.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_util(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;  // Cycle detected
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}
```

### Lock Escalation

```rust
impl LockManager {
    async fn escalate_lock(
        &self,
        lock_id: LockId,
        new_type: LockType
    ) -> Result<()> {
        let mut locks = self.locks.write().await;

        let lock = locks.get_mut(&lock_id)
            .ok_or(Error::LockNotFound)?;

        // Check if escalation is valid
        if !self.can_escalate(lock.lock_type, new_type) {
            return Err(Error::InvalidEscalation);
        }

        // Check if escalation would conflict
        let temp_request = LockRequest {
            entity_id: lock.entity_id.clone(),
            lock_type: new_type,
            agent_id: lock.owner.clone(),
            ..Default::default()
        };

        if !self.check_compatibility(&temp_request).await? {
            return Err(Error::EscalationConflict);
        }

        // Perform escalation
        lock.lock_type = new_type;

        Ok(())
    }

    fn can_escalate(&self, from: LockType, to: LockType) -> bool {
        match (from, to) {
            (LockType::IntentShared, LockType::Shared) => true,
            (LockType::IntentExclusive, LockType::Exclusive) => true,
            (LockType::Shared, LockType::Exclusive) => true,
            _ => false,
        }
    }
}
```

### REST API Endpoints

```rust
// POST /locks
// Acquire a lock
{
    "entity_id": "file:src/auth.rs",
    "lock_type": "Exclusive",
    "agent_id": "agent-123",
    "session_id": "session-456",
    "scope": "File",
    "timeout": 300,
    "wait": true
}

// DELETE /locks/{id}
// Release a lock

// PUT /locks/{id}/escalate
// Escalate lock type
{
    "new_type": "Exclusive"
}

// GET /locks
// List active locks
```

## Conflict Resolution

### Three-Way Merge

Core algorithm for merging concurrent changes:

```rust
struct MergeEngine {
    async fn three_way_merge(
        &self,
        base: &str,
        mine: &str,
        theirs: &str,
        language: Language
    ) -> Result<MergeResult> {
        // For code files, use semantic merge
        if language != Language::Unknown {
            return self.semantic_merge(base, mine, theirs, language).await;
        }

        // For text files, use line-based merge
        self.text_merge(base, mine, theirs)
    }

    async fn semantic_merge(
        &self,
        base: &str,
        mine: &str,
        theirs: &str,
        language: Language
    ) -> Result<MergeResult> {
        // Parse all three versions
        let base_ast = parse_code(base, language)?;
        let mine_ast = parse_code(mine, language)?;
        let theirs_ast = parse_code(theirs, language)?;

        // Extract semantic units
        let base_units = extract_units(&base_ast)?;
        let mine_units = extract_units(&mine_ast)?;
        let theirs_units = extract_units(&theirs_ast)?;

        // Perform three-way diff at unit level
        let mut merged_units = Vec::new();
        let mut conflicts = Vec::new();

        // Process each unit
        for unit in union_of_units(&base_units, &mine_units, &theirs_units) {
            match (
                base_units.get(&unit.id),
                mine_units.get(&unit.id),
                theirs_units.get(&unit.id)
            ) {
                // No conflict cases
                (Some(b), Some(m), Some(t)) if m == b && t == b => {
                    // No changes
                    merged_units.push(b.clone());
                },
                (Some(b), Some(m), Some(t)) if m == t => {
                    // Both made same change
                    merged_units.push(m.clone());
                },
                (Some(b), Some(m), Some(t)) if m == b => {
                    // Only theirs changed
                    merged_units.push(t.clone());
                },
                (Some(b), Some(m), Some(t)) if t == b => {
                    // Only mine changed
                    merged_units.push(m.clone());
                },

                // Conflict case
                (Some(_), Some(m), Some(t)) => {
                    conflicts.push(UnitConflict {
                        unit_id: unit.id,
                        mine: m.clone(),
                        theirs: t.clone(),
                    });
                },

                // Addition cases
                (None, Some(m), None) => merged_units.push(m.clone()),
                (None, None, Some(t)) => merged_units.push(t.clone()),

                // Deletion cases
                (Some(_), None, Some(_)) | (Some(_), Some(_), None) => {
                    // One side deleted, conflict
                    conflicts.push(UnitConflict {
                        unit_id: unit.id,
                        mine: mine_units.get(&unit.id).cloned(),
                        theirs: theirs_units.get(&unit.id).cloned(),
                    });
                },

                _ => {}
            }
        }

        if conflicts.is_empty() {
            // Reconstruct code from merged units
            let merged_code = self.reconstruct_code(merged_units, language)?;
            Ok(MergeResult::Success(merged_code))
        } else {
            Ok(MergeResult::Conflicts(conflicts))
        }
    }
}
```

### Conflict Types

```rust
enum ConflictType {
    TextConflict,       // Different text changes
    SemanticConflict,   // Conflicting semantic changes
    DependencyConflict, // Breaking dependency changes
    TypeConflict,       // Type system conflicts
    TestConflict,       // Test failures after merge
}

struct ConflictResolver {
    async fn resolve_conflict(
        &self,
        conflict: &Conflict,
        strategy: ResolutionStrategy
    ) -> Result<Resolution> {
        match conflict.conflict_type {
            ConflictType::TextConflict => {
                self.resolve_text_conflict(conflict, strategy).await
            },
            ConflictType::SemanticConflict => {
                self.resolve_semantic_conflict(conflict, strategy).await
            },
            ConflictType::DependencyConflict => {
                self.resolve_dependency_conflict(conflict, strategy).await
            },
            ConflictType::TypeConflict => {
                self.resolve_type_conflict(conflict, strategy).await
            },
            ConflictType::TestConflict => {
                self.resolve_test_conflict(conflict, strategy).await
            },
        }
    }

    async fn resolve_semantic_conflict(
        &self,
        conflict: &Conflict,
        strategy: ResolutionStrategy
    ) -> Result<Resolution> {
        // Parse both versions
        let mine_ast = parse_code(&conflict.mine, conflict.language)?;
        let theirs_ast = parse_code(&conflict.theirs, conflict.language)?;

        // Analyze semantic changes
        let mine_changes = self.analyze_changes(&conflict.base, &conflict.mine).await?;
        let theirs_changes = self.analyze_changes(&conflict.base, &conflict.theirs).await?;

        // Check if changes are compatible
        if self.changes_compatible(&mine_changes, &theirs_changes) {
            // Merge both changes
            return self.merge_compatible_changes(
                &conflict.base,
                &mine_changes,
                &theirs_changes
            ).await;
        }

        // Apply strategy for incompatible changes
        match strategy {
            ResolutionStrategy::PreferMine => {
                Ok(Resolution::UseMine(conflict.mine.clone()))
            },
            ResolutionStrategy::PreferTheirs => {
                Ok(Resolution::UseTheirs(conflict.theirs.clone()))
            },
            ResolutionStrategy::Interactive => {
                // Axon handles interactive resolution via UI
                self.notify_axon_for_resolution(conflict).await
            },
            ResolutionStrategy::AI => {
                self.ai_resolution(conflict).await
            },
        }
    }
}
```

### AI-Powered Resolution

```rust
impl ConflictResolver {
    async fn ai_resolution(&self, conflict: &Conflict) -> Result<Resolution> {
        // Prepare context
        let context = self.build_context(conflict).await?;

        // Generate resolution prompt
        let prompt = format!(
            "Resolve the following code conflict:

            Base version:
            ```{}
            {}
            ```

            Version A (mine):
            ```{}
            {}
            ```

            Version B (theirs):
            ```{}
            {}
            ```

            Context:
            - File: {}
            - Mine changes: {}
            - Theirs changes: {}
            - Affected dependencies: {:?}

            Generate a merged version that preserves the intent of both changes.",
            conflict.language, conflict.base,
            conflict.language, conflict.mine,
            conflict.language, conflict.theirs,
            conflict.path,
            self.summarize_changes(&conflict.mine_changes),
            self.summarize_changes(&conflict.theirs_changes),
            context.affected_dependencies
        );

        // Call AI model
        let response = self.ai_client.complete(prompt).await?;

        // Validate generated code
        let merged = self.extract_code_from_response(&response)?;

        if !self.validate_merged_code(&merged, conflict.language).await? {
            return Err(Error::InvalidAIResolution);
        }

        // Verify no regression
        if !self.verify_no_regression(&merged, conflict).await? {
            return Err(Error::AIResolutionRegression);
        }

        Ok(Resolution::Merged(merged))
    }

    async fn verify_no_regression(
        &self,
        merged: &str,
        conflict: &Conflict
    ) -> Result<bool> {
        // Parse merged code
        let merged_ast = parse_code(merged, conflict.language)?;
        let merged_units = extract_units(&merged_ast)?;

        // Check that key functionality is preserved
        let base_units = extract_units(&parse_code(&conflict.base, conflict.language)?)?;
        let mine_units = extract_units(&parse_code(&conflict.mine, conflict.language)?)?;
        let theirs_units = extract_units(&parse_code(&conflict.theirs, conflict.language)?)?;

        // Verify public API is preserved
        for unit in base_units.iter().filter(|u| u.visibility == Visibility::Public) {
            if !merged_units.iter().any(|m| m.signature == unit.signature) {
                return Ok(false);  // Public API broken
            }
        }

        // Verify new additions are included
        for unit in mine_units.iter().filter(|u| !base_units.contains(u)) {
            if !merged_units.contains(unit) {
                return Ok(false);  // Mine addition lost
            }
        }

        for unit in theirs_units.iter().filter(|u| !base_units.contains(u)) {
            if !merged_units.contains(unit) {
                return Ok(false);  // Theirs addition lost
            }
        }

        Ok(true)
    }
}
```

## Performance Optimizations

### Parallel Merge Processing

```rust
impl ParallelMerger {
    async fn parallel_merge(&self, changes: Vec<Change>) -> Result<Vec<MergeResult>> {
        // Group changes by file
        let mut by_file: HashMap<PathBuf, Vec<Change>> = HashMap::new();

        for change in changes {
            by_file.entry(change.path.clone()).or_default().push(change);
        }

        // Process files in parallel
        let mut handles = Vec::new();

        for (path, file_changes) in by_file {
            let handle = tokio::spawn(async move {
                self.merge_file_changes(path, file_changes).await
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }

        Ok(results)
    }
}
```

## Monitoring & Metrics

### Session and Lock Metrics

```rust
struct DataLayerMonitor {
    metrics: Arc<RwLock<DataLayerMetrics>>,
}

struct DataLayerMetrics {
    active_sessions: Gauge,
    session_duration: Histogram,
    active_locks: Gauge,
    lock_wait_time: Histogram,
    merge_conflicts: Counter,
    successful_merges: Counter,
    failed_merges: Counter,
}

impl DataLayerMonitor {
    async fn record_session_created(&self, session_id: &SessionId) {
        let mut metrics = self.metrics.write().await;
        metrics.active_sessions.inc();
    }

    async fn record_session_merged(&self, duration: Duration, conflicts: usize) {
        let mut metrics = self.metrics.write().await;
        metrics.active_sessions.dec();
        metrics.session_duration.observe(duration.as_secs_f64());

        if conflicts > 0 {
            metrics.merge_conflicts.inc_by(conflicts as u64);
        } else {
            metrics.successful_merges.inc();
        }
    }

    async fn record_lock_acquired(&self, wait_time: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.active_locks.inc();
        metrics.lock_wait_time.observe(wait_time.as_secs_f64());
    }

    async fn export_metrics(&self) -> String {
        let metrics = self.metrics.read().await;

        // Format as Prometheus metrics
        format!(
            "# HELP cortex_active_sessions Active agent sessions
# TYPE cortex_active_sessions gauge
cortex_active_sessions {{}} {}

# HELP cortex_session_duration Session duration in seconds
# TYPE cortex_session_duration histogram
{}

# HELP cortex_active_locks Active locks
# TYPE cortex_active_locks gauge
cortex_active_locks {{}} {}

# HELP cortex_merge_conflicts Total merge conflicts detected
# TYPE cortex_merge_conflicts counter
cortex_merge_conflicts {{}} {}",
            metrics.active_sessions.value(),
            metrics.session_duration.format(),
            metrics.active_locks.value(),
            metrics.merge_conflicts.value()
        )
    }
}
```

## Integration with Axon

### Typical Workflow

```
1. Axon receives task from user
   ↓
2. Axon creates session via POST /sessions
   ← Cortex returns session_id
   ↓
3. Axon assigns agent to work in session
   ↓
4. Agent reads/writes via GET/PUT /sessions/{id}/files/{path}
   ← Cortex provides isolated data access
   ↓
5. Axon triggers merge via POST /sessions/{id}/merge
   ← Cortex performs three-way merge, returns conflicts
   ↓
6. If conflicts: Axon resolves via UI or AI
   ↓
7. Cortex finalizes merge and closes session
```

### REST API Summary

```rust
// Session Management
POST   /sessions                    // Create session
GET    /sessions/{id}               // Get session info
DELETE /sessions/{id}               // Close session
POST   /sessions/{id}/merge         // Merge session changes
GET    /sessions/{id}/files/{path}  // Read file in session
PUT    /sessions/{id}/files/{path}  // Write file in session

// Lock Management
POST   /locks                       // Acquire lock
DELETE /locks/{id}                  // Release lock
PUT    /locks/{id}/escalate         // Escalate lock
GET    /locks                       // List active locks

// Conflict Resolution
GET    /sessions/{id}/conflicts     // List conflicts
POST   /conflicts/{id}/resolve      // Resolve specific conflict

// Metrics
GET    /metrics                     // Prometheus metrics
```

### WebSocket Events

```rust
// Real-time notifications for Axon

ws://cortex:8081/ws

Events:
- session.created
- session.merged
- session.conflict_detected
- lock.acquired
- lock.released
- lock.deadlock_detected
```

## Conclusion

The Multi-Agent Data Layer provides:

1. **Isolated Development** - Agents work in separate namespaces without interference
2. **Fine-Grained Locking** - Prevent data conflicts at entity level
3. **Intelligent Merging** - Semantic three-way merge with conflict detection
4. **Deadlock Prevention** - Proactive cycle detection in wait-for graph
5. **Performance** - Parallel merge processing with efficient caching
6. **Observability** - Complete metrics for sessions and locks

This data layer architecture enables **Axon** to orchestrate complex multi-agent workflows while **Cortex** ensures data consistency and conflict-free operations.
