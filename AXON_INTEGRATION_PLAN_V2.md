# Axon Integration Plan V2 - Direct Integration (Variant B)

**Date:** 2025-11-04
**Status:** REVISED - Pivot to direct integration
**Architecture:** Single binary with shared subsystems

## Architecture Decision

**WRONG APPROACH (V1):** CortexBridge as HTTP client connecting to REST API
**CORRECT APPROACH (V2):** Direct integration via `Arc<T>` to Cortex subsystems

### Rationale

REST API is **ONLY** for external clients (Dashboard UI, CLI tools, external integrations). Internal agents should use direct Arc references to avoid:
- HTTP overhead
- Serialization/deserialization
- Network latency
- Additional error handling
- Unnecessary abstraction layers

## Current State Analysis

### ✅ What We Have
1. Cortex core subsystems (VFS, Memory, Semantic, Sessions, Locks)
2. MCP server infrastructure
3. Axon agent implementations (Developer, Tester, Reviewer, etc.)
4. Axon orchestration components (LeadAgent, StrategyLibrary, etc.)
5. Tool structures defined (agent_launch.rs, orchestrate.rs, session.rs, etc.)

### ❌ What's Missing
1. **Context structures** for MCP tools (AgentLaunchContext, etc.)
2. **Tool trait implementations** for MCP registration
3. **Direct subsystem wiring** to agents
4. **AgentRegistry** for tracking executions
5. **Integration of Axon components** into Cortex binary

## Integration Strategy

### Phase 1: Core Infrastructure (HIGH PRIORITY)

#### 1.1 Remove CortexBridge HTTP Layer
- [x] CortexBridge implemented (but wrong approach)
- [ ] Remove HTTP client dependencies from tools
- [ ] Replace with direct Arc<> references

#### 1.2 Create Unified Context
```rust
#[derive(Clone)]
pub struct AxonContext {
    // Core Cortex subsystems (direct access)
    pub vfs: Arc<VirtualFileSystem>,
    pub memory: Arc<SemanticMemorySystem>,
    pub sessions: Arc<SessionManager>,
    pub locks: Arc<LockManager>,
    pub storage: Arc<ConnectionManager>,

    // Axon-specific components
    pub registry: Arc<AgentRegistry>,
    pub orchestrator: Arc<LeadAgent>,
    pub strategy_library: Arc<StrategyLibrary>,
    pub message_bus: Arc<UnifiedMessageBus>,

    // Configuration
    pub config: Arc<AxonConfig>,
}
```

#### 1.3 Implement AgentRegistry
```rust
pub struct AgentRegistry {
    executions: Arc<RwLock<HashMap<String, AgentExecution>>>,
    storage: Arc<ConnectionManager>, // Persist to DB
}

// Track agent executions with state machine:
// Queued -> Running -> (Completed | Failed)
```

### Phase 2: Tool Context Structures (CRITICAL)

#### 2.1 Create Context for Each Tool
```rust
// session.rs
#[derive(Clone)]
pub struct SessionContext {
    sessions: Arc<SessionManager>,
    locks: Arc<LockManager>,
    storage: Arc<ConnectionManager>,
}

// agent_launch.rs
#[derive(Clone)]
pub struct AgentLaunchContext {
    registry: Arc<AgentRegistry>,
    vfs: Arc<VirtualFileSystem>,
    memory: Arc<SemanticMemorySystem>,
    agents: Arc<AgentFactory>, // Factory for creating agents
}

// orchestrate.rs
#[derive(Clone)]
pub struct OrchestrateContext {
    orchestrator: Arc<LeadAgent>,
    registry: Arc<AgentRegistry>,
    message_bus: Arc<UnifiedMessageBus>,
}

// cortex_query.rs
#[derive(Clone)]
pub struct CortexQueryContext {
    semantic: Arc<SemanticMemorySystem>,
    vfs: Arc<VirtualFileSystem>,
}
```

#### 2.2 Update MCP Server Builder
```rust
// In cortex/cortex/src/mcp/server.rs
async fn build_server(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Result<McpServer> {
    // Initialize all subsystems
    let memory = Arc::new(SemanticMemorySystem::new(storage.clone()));
    let sessions = Arc::new(SessionManager::new(storage.clone()));
    let locks = Arc::new(LockManager::new(storage.clone()));

    // Initialize Axon components
    let registry = Arc::new(AgentRegistry::new(storage.clone()));
    let message_bus = Arc::new(UnifiedMessageBus::new());
    let strategy_library = Arc::new(StrategyLibrary::new(...));
    let orchestrator = Arc::new(LeadAgent::new(...));

    // Create contexts
    let session_ctx = SessionContext::new(sessions.clone(), locks.clone(), storage.clone());
    let agent_launch_ctx = AgentLaunchContext::new(registry.clone(), vfs.clone(), memory.clone());
    let orchestrate_ctx = OrchestrateContext::new(orchestrator.clone(), registry.clone(), message_bus.clone());
    let cortex_query_ctx = CortexQueryContext::new(memory.clone(), vfs.clone());

    // Build server with tools
    McpServer::builder()
        // ... existing tools ...
        .tool(SessionCreateTool::new(session_ctx.clone()))
        .tool(AgentLaunchTool::new(agent_launch_ctx.clone()))
        .tool(OrchestrateTool::new(orchestrate_ctx.clone()))
        .tool(CortexQueryTool::new(cortex_query_ctx.clone()))
        .build()
}
```

### Phase 3: Tool Trait Implementations (CRITICAL)

Each tool needs `impl Tool for XxxTool`:

```rust
#[async_trait]
impl Tool for AgentLaunchTool {
    fn name(&self) -> &str {
        "axon.agent.launch"
    }

    fn description(&self) -> Option<&str> {
        Some("Launch a specialized agent for a specific task")
    }

    fn input_schema(&self) -> schemars::schema::RootSchema {
        schemars::schema_for!(AgentLaunchInput)
    }

    async fn call(&self, input: serde_json::Value) -> Result<CallToolResult, Error> {
        let input: AgentLaunchInput = serde_json::from_value(input)?;
        let output = self.launch(input).await?;

        Ok(CallToolResult {
            content: vec![Content::text(serde_json::to_string_pretty(&output)?)],
            is_error: false,
        })
    }
}
```

Tools needing implementation:
- [ ] SessionCreateTool
- [ ] SessionMergeTool
- [ ] AgentLaunchTool
- [ ] AgentStatusTool
- [ ] AgentStopTool
- [ ] OrchestrateTool
- [ ] CortexQueryTool

### Phase 4: Agent Integration (HIGH PRIORITY)

#### 4.1 Update Agent Constructors
Agents currently use CortexBridge - replace with direct subsystems:

```rust
// Before (WRONG):
impl DeveloperAgent {
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>) -> Self { ... }
}

// After (CORRECT):
impl DeveloperAgent {
    pub fn new(
        name: String,
        vfs: Arc<VirtualFileSystem>,
        memory: Arc<SemanticMemorySystem>,
        sessions: Arc<SessionManager>,
    ) -> Self { ... }
}
```

#### 4.2 Update Agent Implementations
Replace all `cortex.semantic_search()` calls with direct `memory.search()` calls:

```rust
// Before:
let results = cortex.semantic_search(query, workspace_id, filters).await?;

// After:
let results = memory.semantic_search(query, workspace_id, filters).await?;
```

### Phase 5: Move Axon Components (STRUCTURAL)

#### 5.1 Move from axon/ to cortex/
```
axon/src/agents/          -> cortex/cortex-agents/src/
axon/src/orchestration/   -> cortex/cortex-orchestration/src/
axon/src/coordination/    -> cortex/cortex-coordination/src/
axon/src/cli/             -> cortex/cortex/src/cli/ (merge with existing)
```

#### 5.2 Update Cargo.toml Dependencies
```toml
# cortex/Cargo.toml workspace
[workspace]
members = [
    "cortex",
    "cortex-core",
    "cortex-vfs",
    "cortex-memory",
    "cortex-semantic",
    "cortex-agents",        # ← moved from axon
    "cortex-orchestration", # ← moved from axon
    "cortex-coordination",  # ← moved from axon
    # ...
]
```

#### 5.3 Remove axon/ Directory
After integration is complete and tested:
```bash
git mv axon/src/agents cortex/cortex-agents
git mv axon/src/orchestration cortex/cortex-orchestration
git mv axon/src/coordination cortex/cortex-coordination
git rm -rf axon/
```

## Implementation Order

### Week 1: Core Infrastructure
1. ✅ Document errors
2. ✅ Create CortexBridge (realized it's wrong approach)
3. [ ] **Remove CortexBridge HTTP dependencies**
4. [ ] Create AxonContext with direct subsystems
5. [ ] Implement AgentRegistry

### Week 2: MCP Integration
6. [ ] Create all Context structures (Session, AgentLaunch, Orchestrate, etc.)
7. [ ] Implement Tool traits for all 7 Axon tools
8. [ ] Wire contexts in MCP server builder
9. [ ] Test MCP tool discovery

### Week 3: Agent Integration
10. [ ] Update all agent constructors to use direct subsystems
11. [ ] Replace cortex.method() calls with direct subsystem calls
12. [ ] Fix compilation errors in cortex-agents
13. [ ] Unit test each agent

### Week 4: Orchestration & Testing
14. [ ] Integrate LeadAgent into Cortex binary
15. [ ] Test single agent execution (developer)
16. [ ] Test multi-agent orchestration
17. [ ] E2E workflow testing

### Week 5: Structural Cleanup
18. [ ] Move axon/src/agents to cortex/cortex-agents
19. [ ] Move axon/src/orchestration to cortex/cortex-orchestration
20. [ ] Move axon/src/coordination to cortex/cortex-coordination
21. [ ] Update all imports and dependencies
22. [ ] Remove axon/ directory
23. [ ] Update documentation

## Success Criteria

### Functional Requirements
- [ ] Can create session via MCP
- [ ] Can launch developer agent via MCP
- [ ] Can check agent status via MCP
- [ ] Can query semantic memory via MCP
- [ ] Can orchestrate multi-agent tasks via MCP
- [ ] Agents can read/write VFS
- [ ] Agents can query semantic memory
- [ ] Agents can acquire/release locks
- [ ] Sessions properly isolate agent work
- [ ] Session merging works with conflict resolution

### Non-Functional Requirements
- [ ] Zero HTTP overhead for internal operations
- [ ] Compilation succeeds with zero warnings
- [ ] All tests pass
- [ ] Performance baseline: <100ms for simple operations
- [ ] Memory efficient: no unnecessary clones

### Integration Requirements
- [ ] Single cortex binary
- [ ] axon/ directory removed
- [ ] All functionality preserved
- [ ] MCP tools work end-to-end
- [ ] REST API still works for external clients

## Risk Mitigation

### Risk 1: Breaking Existing Cortex Tools
**Mitigation:** Don't touch existing tool implementations, only add new Axon tools

### Risk 2: Complex Dependencies
**Mitigation:** Use feature flags to isolate Axon functionality during development

### Risk 3: Compilation Failures
**Mitigation:** Fix incrementally, one subsystem at a time

### Risk 4: Test Failures
**Mitigation:** Write integration tests before refactoring

## Rollback Plan

If integration fails:
1. Revert MCP server changes
2. Keep axon/ as separate binary
3. Use REST API bridge (original plan) as fallback
4. Document lessons learned

## Next Actions (Immediate)

1. **NOW:** Remove CortexBridge HTTP client from tool implementations
2. **TODAY:** Create AxonContext with direct subsystem references
3. **TODAY:** Implement AgentRegistry
4. **TOMORROW:** Create all Context structures
5. **THIS WEEK:** Implement Tool traits

## Notes

- REST API remains for external clients (Dashboard, CLI)
- Internal agents use direct Arc<> references
- No HTTP overhead for internal operations
- Single process, shared memory, efficient
- Follows Rust best practices (Arc, RwLock, async)
