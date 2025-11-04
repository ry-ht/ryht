# Axon MCP Tools - Critical Errors Report

**Generated:** 2025-11-04
**Severity:** CRITICAL - Complete integration non-functional
**Impact:** All Axon orchestration features unavailable via MCP

## Executive Summary

Axon MCP tools are registered in Cortex MCP server but are completely non-functional due to missing implementations. This is a **blocking issue** preventing any multi-agent orchestration functionality.

## Critical Errors Identified

### ERROR #1: CortexBridge is a Stub Implementation
**Location:** `cortex/cortex/src/cortex_bridge.rs:78-81`
**Severity:** CRITICAL
**Status:** 🔴 BLOCKING

**Problem:**
```rust
/// Stub CortexBridge for MCP tool compatibility
pub struct CortexBridge {
    // TODO: Implement actual bridge to cortex subsystems
    _placeholder: (),
}
```

CortexBridge has NO implementation - it's an empty placeholder. All Axon tools depend on it.

**Required Methods (from usage in tools):**
- `ensure_initialized() -> Result<()>`
- `create_session(agent_id, workspace_id, scope) -> Result<SessionId>`
- `merge_session(session_id, strategy) -> Result<MergeReport>`
- `semantic_search(query, workspace_id, filters) -> Result<Vec<SearchResult>>`
- And many more...

**Impact:** ALL Axon MCP tools cannot function

**Files Affected:**
- `cortex/cortex/src/mcp/tools/session.rs` (SessionCreateTool, SessionMergeTool)
- `cortex/cortex/src/mcp/tools/agent_launch.rs` (AgentLaunchTool)
- `cortex/cortex/src/mcp/tools/orchestrate.rs` (OrchestrateTool)
- `cortex/cortex/src/mcp/tools/cortex_query.rs` (CortexQueryTool)

---

### ERROR #2: Missing Context Structures
**Location:** `cortex/cortex/src/mcp/server.rs:144-150`
**Severity:** CRITICAL
**Status:** 🔴 BLOCKING

**Problem:**
Server attempts to create contexts that don't exist:
```rust
let agent_launch_ctx = AgentLaunchContext::new();  // ❌ NOT DEFINED
let agent_status_ctx = AgentStatusContext::new();   // ❌ NOT DEFINED
let agent_stop_ctx = AgentStopContext::new();       // ❌ NOT DEFINED
let orchestrate_ctx = OrchestrateContext::new();    // ❌ NOT DEFINED
let cortex_query_ctx = CortexQueryContext::new();   // ❌ NOT DEFINED
let session_create_ctx = SessionCreateContext::new(); // ❌ NOT DEFINED
let session_merge_ctx = SessionMergeContext::new(); // ❌ NOT DEFINED
```

**Required Structures:**
Each tool needs a Context struct following the pattern:
```rust
#[derive(Clone)]
pub struct AgentLaunchContext {
    cortex: Arc<CortexBridge>,
    registry: Arc<AgentRegistry>,
    config: Arc<McpServerConfig>,
}

impl AgentLaunchContext {
    pub fn new(/* dependencies */) -> Self { ... }
}
```

**Impact:** MCP server fails to compile/initialize

---

### ERROR #3: Missing AgentRegistry
**Location:** Referenced in `cortex/cortex/src/mcp/tools/agent_launch.rs`
**Severity:** CRITICAL
**Status:** 🔴 BLOCKING

**Problem:**
`AgentRegistry` is used but not defined in Cortex codebase.

**Required Implementation:**
```rust
pub struct AgentRegistry {
    executions: Arc<RwLock<HashMap<String, AgentExecution>>>,
}

// Required methods:
- register(execution: AgentExecution) -> Result<()>
- get(agent_id: &str) -> Option<AgentExecution>
- update_status(agent_id: &str, status: ExecutionStatus) -> Result<()>
- set_result(agent_id: &str, result: serde_json::Value) -> Result<()>
- set_error(agent_id: &str, error: String) -> Result<()>
```

**Import Error:** `use crate::mcp_server::{AgentExecution, AgentRegistry, ...}`

**Impact:** AgentLaunchTool and AgentStatusTool cannot compile

---

### ERROR #4: Missing McpServerConfig
**Location:** Referenced in `cortex/cortex/src/mcp/tools/agent_launch.rs`
**Severity:** HIGH
**Status:** 🔴 BLOCKING

**Problem:**
`McpServerConfig` type not found in codebase.

**Required Implementation:**
```rust
pub struct McpServerConfig {
    pub max_concurrent_agents: usize,
    pub default_timeout: Duration,
    pub enable_telemetry: bool,
    // ... other config fields
}
```

**Impact:** AgentLaunchTool cannot be instantiated

---

### ERROR #5: Missing Tool Trait Implementations
**Location:** All Axon tools in `cortex/cortex/src/mcp/tools/`
**Severity:** CRITICAL
**Status:** 🔴 BLOCKING

**Problem:**
None of the Axon tools implement the `mcp_sdk::Tool` trait, which is required for MCP registration.

**Required Implementation Pattern:**
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

**Tools Needing Implementation:**
- AgentLaunchTool
- AgentStatusTool
- AgentStopTool
- OrchestrateTool
- CortexQueryTool
- SessionCreateTool (in session.rs)
- SessionMergeTool (in session.rs)

**Impact:** Tools cannot be registered with MCP server

---

### ERROR #6: Incomplete SessionScope Type
**Location:** `cortex/cortex/src/cortex_bridge.rs:25-30`
**Severity:** MEDIUM
**Status:** ⚠️ INCORRECT TYPE

**Problem:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionScope {
    Workspace,
    Global,
    Temporary,
}
```

But `session.rs:42-45` expects:
```rust
let scope = SessionScope {
    paths: vec!["/".to_string()],
    read_only_paths: vec![],
};
```

**Required Fix:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScope {
    pub paths: Vec<String>,
    pub read_only_paths: Vec<String>,
}
```

**Impact:** SessionCreateTool has type mismatch

---

## Dependency Chain Analysis

```
Axon MCP Tools Dependency Chain:
┌─────────────────────┐
│ MCP Server (stdio)  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│   Cortex MCP        │  ← Tools registered but contexts missing
│   Server Builder    │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ Axon Tool Contexts (❌ NOT DEFINED) │
├─────────────────────────────────────┤
│ - AgentLaunchContext               │
│ - AgentStatusContext               │
│ - OrchestrateCont │
│ - SessionCreateContext              │
│ - CortexQueryContext                │
└──────────┬──────────────────────────┘
           │
           ▼
┌────────────────────────────────────────┐
│ Dependencies (❌ STUBS/MISSING)        │
├────────────────────────────────────────┤
│ • CortexBridge (stub - empty)          │
│ • AgentRegistry (missing)              │
│ • McpServerConfig (missing)            │
│ • AgentExecution (defined but unused)  │
│ • ExecutionStatus (defined but unused) │
└────────────────────────────────────────┘
```

---

## Root Cause Analysis

The integration was **partially implemented**:

1. ✅ Tool structures defined (agent_launch.rs, session.rs, etc.)
2. ✅ Tool business logic implemented (launch, status, orchestrate)
3. ✅ Input/Output types defined
4. ❌ **Context structures NOT created**
5. ❌ **CortexBridge NOT implemented** (empty stub)
6. ❌ **Tool trait NOT implemented**
7. ❌ **Supporting types missing** (AgentRegistry, McpServerConfig)

**Result:** Code compiles but MCP tools are **completely non-functional**.

---

## Recommended Fix Strategy

### Phase 1: Core Infrastructure (CRITICAL)
1. ✅ Implement full CortexBridge with real Cortex subsystem integration
2. ✅ Create AgentRegistry with execution tracking
3. ✅ Create McpServerConfig
4. ✅ Fix SessionScope type mismatch

### Phase 2: Context Layer (CRITICAL)
5. ✅ Implement all 7 Context structures
6. ✅ Wire dependencies correctly

### Phase 3: MCP Integration (CRITICAL)
7. ✅ Implement Tool trait for all 7 Axon tools
8. ✅ Register tools with proper context initialization

### Phase 4: Testing (HIGH)
9. ✅ Test each tool individually
10. ✅ Test orchestration workflow end-to-end
11. ✅ Verify with real agent tasks

---

## Testing Plan

### Unit Tests
- [ ] CortexBridge initialization
- [ ] AgentRegistry operations
- [ ] Each tool's business logic

### Integration Tests
- [ ] MCP tool discovery (list tools)
- [ ] Session creation via MCP
- [ ] Agent launch via MCP
- [ ] Status checking via MCP
- [ ] Orchestration workflow

### E2E Tests
- [ ] Full developer agent task
- [ ] Multi-agent orchestration
- [ ] Error handling and recovery

---

## Estimated Effort

- **Phase 1:** 4-6 hours (core infrastructure)
- **Phase 2:** 2-3 hours (context layer)
- **Phase 3:** 3-4 hours (MCP integration)
- **Phase 4:** 2-3 hours (testing)

**Total:** 11-16 hours of focused development

---

## Next Steps

1. **IMMEDIATE:** Implement CortexBridge with real subsystem integration
2. **HIGH PRIORITY:** Create all Context structures
3. **HIGH PRIORITY:** Implement Tool traits
4. **CRITICAL PATH:** Test end-to-end orchestration

**Blocking Dependency:** CortexBridge must be implemented first, as everything depends on it.
