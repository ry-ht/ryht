# Agent Orchestration Testing Plan

**Date:** 2025-11-04
**Status:** Ready for Execution
**Integration:** Axon → Cortex Complete

---

## Overview

This document outlines the comprehensive testing strategy for verifying Cortex's agent orchestration system with integrated Axon MCP tools.

## Prerequisites

- ✅ Axon directory removed
- ✅ All code migrated to Cortex
- ✅ Compilation successful (0 errors)
- ✅ 7 MCP tools registered
- ✅ Backup tag created: `axon-final-backup`

## Test Environment

```bash
# Build Cortex binary
cargo build --release --package cortex --bin cortex

# Verify binary location
ls -lh target/release/cortex

# Set environment
export RUST_LOG=cortex=debug,cortex_agents=debug,cortex_orchestration=debug
```

## Phase 1: MCP Server Health Check

### Test 1.1: MCP Server Startup
```bash
# Start MCP server
./target/release/cortex mcp stdio

# Expected: Server starts without errors
# Expected: Shows "Registered 187 tools"
```

### Test 1.2: Tool Registration Verification
```bash
# Query available tools (via Claude Code or MCP client)
# Expected tools:
# - cortex.agent.launch
# - cortex.agent.status
# - cortex.agent.stop
# - cortex.orchestrate
# - cortex.cortex_query
# - cortex.session.create
# - cortex.session.merge
```

### Test 1.3: Database Connectivity
```bash
# Check SurrealDB connection
./target/release/cortex db status

# Check Qdrant connection
curl -X GET http://localhost:6333/collections
```

---

## Phase 2: Individual Agent Testing

### Test 2.1: Developer Agent Launch
**Goal:** Verify Developer agent can be launched and execute tasks

**MCP Tool Call:**
```json
{
  "tool": "cortex.agent.launch",
  "input": {
    "agent_type": "developer",
    "task": "Create a simple hello world function in Rust",
    "workspace_id": "test-workspace-001"
  }
}
```

**Expected Results:**
- ✅ Agent ID returned (format: `developer-{uuid}`)
- ✅ Status: "Running" or "Completed"
- ✅ No error messages
- ✅ Agent registered in AgentRegistry

**Verification:**
```json
{
  "tool": "cortex.agent.status",
  "input": {
    "agent_id": "{returned_agent_id}"
  }
}
```

### Test 2.2: Tester Agent Launch
**Goal:** Verify Tester agent can generate tests

**MCP Tool Call:**
```json
{
  "tool": "cortex.agent.launch",
  "input": {
    "agent_type": "tester",
    "task": "Generate unit tests for the hello world function",
    "workspace_id": "test-workspace-001",
    "params": {
      "test_type": "unit",
      "coverage_target": 80
    }
  }
}
```

**Expected Results:**
- ✅ Tester agent launches
- ✅ Tests generated
- ✅ Status updates tracked

### Test 2.3: Reviewer Agent Launch
**Goal:** Verify Reviewer agent can review code

**MCP Tool Call:**
```json
{
  "tool": "cortex.agent.launch",
  "input": {
    "agent_type": "reviewer",
    "task": "Review the hello world function for quality and security",
    "workspace_id": "test-workspace-001"
  }
}
```

**Expected Results:**
- ✅ Review report generated
- ✅ Issues identified (if any)
- ✅ Recommendations provided

---

## Phase 3: Orchestration Testing

### Test 3.1: Simple Orchestration
**Goal:** Orchestrate multiple agents sequentially

**MCP Tool Call:**
```json
{
  "tool": "cortex.orchestrate",
  "input": {
    "task": "Implement a Fibonacci calculator: write code, generate tests, and review",
    "workspace_id": "test-workspace-001",
    "agent_types": ["developer", "tester", "reviewer"]
  }
}
```

**Expected Results:**
- ✅ All 3 agents execute in sequence
- ✅ Each agent receives context from previous agent
- ✅ Final consolidated result returned
- ✅ All intermediate results stored

**Verification Steps:**
1. Check Developer agent output (Fibonacci function)
2. Check Tester agent output (test cases)
3. Check Reviewer agent output (review report)
4. Verify all agents completed successfully

### Test 3.2: Parallel Orchestration
**Goal:** Verify parallel agent execution

**MCP Tool Call:**
```json
{
  "tool": "cortex.orchestrate",
  "input": {
    "task": "Analyze the codebase from multiple perspectives",
    "workspace_id": "test-workspace-001",
    "agent_types": ["architect", "researcher", "documenter"],
    "execution_mode": "parallel"
  }
}
```

**Expected Results:**
- ✅ All 3 agents start concurrently
- ✅ No blocking between agents
- ✅ Results merged correctly

---

## Phase 4: Session Management Testing

### Test 4.1: Session Creation
**Goal:** Verify isolated session creation

**MCP Tool Call:**
```json
{
  "tool": "cortex.session.create",
  "input": {
    "agent_id": "test-agent-001",
    "workspace_id": "test-workspace-001",
    "isolation_level": "snapshot"
  }
}
```

**Expected Results:**
- ✅ Session ID returned
- ✅ Isolated VFS snapshot created
- ✅ Session tracked in SessionManager

### Test 4.2: Session Merge
**Goal:** Verify changes can be merged back

**MCP Tool Call:**
```json
{
  "tool": "cortex.session.merge",
  "input": {
    "session_id": "{created_session_id}",
    "target_namespace": "main"
  }
}
```

**Expected Results:**
- ✅ Changes merged successfully
- ✅ Conflicts detected (if any)
- ✅ Merge report returned

---

## Phase 5: Semantic Query Testing

### Test 5.1: Code Search
**Goal:** Verify semantic search across codebase

**MCP Tool Call:**
```json
{
  "tool": "cortex.cortex_query",
  "input": {
    "query": "Find all functions that handle file I/O",
    "workspace_id": "test-workspace-001",
    "limit": 10
  }
}
```

**Expected Results:**
- ✅ Relevant code units returned
- ✅ Similarity scores provided
- ✅ Results ranked by relevance

---

## Phase 6: Error Handling Testing

### Test 6.1: Invalid Agent Type
**Goal:** Verify graceful error handling

**MCP Tool Call:**
```json
{
  "tool": "cortex.agent.launch",
  "input": {
    "agent_type": "nonexistent_agent",
    "task": "Do something"
  }
}
```

**Expected Results:**
- ✅ Error message: "Unknown agent type"
- ✅ No crash
- ✅ Server remains operational

### Test 6.2: Agent Stop
**Goal:** Verify agents can be stopped mid-execution

**Steps:**
1. Launch long-running agent
2. Call agent.stop with agent ID
3. Verify agent terminated

**Expected Results:**
- ✅ Agent stops gracefully
- ✅ Status updates to "Stopped"
- ✅ Cleanup performed

---

## Phase 7: Integration Testing

### Test 7.1: Full Workflow
**Scenario:** Complete feature development cycle

**Steps:**
1. **Architect** analyzes requirements
2. **Developer** implements feature
3. **Tester** generates tests
4. **Reviewer** reviews code
5. **Documenter** creates documentation

**MCP Tool Calls:**
```json
{
  "tool": "cortex.orchestrate",
  "input": {
    "task": "Implement user authentication with JWT tokens",
    "workspace_id": "test-workspace-001",
    "agent_types": ["architect", "developer", "tester", "reviewer", "documenter"],
    "execution_mode": "sequential"
  }
}
```

**Expected Results:**
- ✅ All agents execute successfully
- ✅ Complete implementation delivered
- ✅ Tests pass
- ✅ Documentation generated
- ✅ Code reviewed and approved

### Test 7.2: Claude Code SDK Integration
**Goal:** Verify agents use Claude Code correctly

**Expected Behaviors:**
- ✅ Agents can read files from workspace
- ✅ Agents can write code
- ✅ Agents can execute tests
- ✅ Agents can query semantic memory
- ✅ Agents communicate via MCP protocol

---

## Phase 8: Performance Testing

### Test 8.1: Concurrent Agent Execution
**Goal:** Verify system handles multiple concurrent agents

**Steps:**
1. Launch 5+ agents simultaneously
2. Monitor resource usage
3. Verify all complete successfully

**Metrics to Track:**
- CPU usage
- Memory usage
- Response time
- Agent throughput

### Test 8.2: Memory Consolidation
**Goal:** Verify working memory consolidates to long-term

**Steps:**
1. Execute multiple agent tasks
2. Trigger memory consolidation
3. Verify patterns extracted
4. Verify episodic memory stored

---

## Phase 9: Known Issues Monitoring

### Issues to Watch For

1. **Tool Hanging:**
   - `cortex.workspace.list` may hang indefinitely
   - **Workaround:** Use timeout or kill process

2. **Agent Compilation Errors:**
   - cortex-agents has 171 pre-existing errors
   - **Impact:** Direct agent calls may fail
   - **Workaround:** Use orchestrator pattern

3. **CortexBridge Lock Methods:**
   - `acquire_lock`, `release_lock`, `is_locked` are stubs
   - **Impact:** Resource locking not fully implemented
   - **TODO:** Implement proper lock management

---

## Success Criteria

### Phase 1-2: Basic Functionality
- [ ] MCP server starts successfully
- [ ] All 7 agent tools registered
- [ ] Individual agents can be launched
- [ ] Agent status can be queried

### Phase 3-5: Orchestration
- [ ] Sequential orchestration works
- [ ] Parallel orchestration works
- [ ] Session management functional
- [ ] Semantic queries return results

### Phase 6-7: Robustness
- [ ] Error handling graceful
- [ ] Full workflow completes end-to-end
- [ ] Claude Code SDK integration works

### Phase 8: Performance
- [ ] Concurrent execution stable
- [ ] Memory usage acceptable (<2GB)
- [ ] Response times reasonable (<5s per agent)

---

## Test Execution Log

### Session 1: 2025-11-04

**Test 1.1: MCP Server Startup**
- Status: PENDING
- Command: `./target/release/cortex mcp stdio`
- Result: [TO BE FILLED]

**Test 2.1: Developer Agent Launch**
- Status: PENDING
- Result: [TO BE FILLED]

**Test 3.1: Simple Orchestration**
- Status: PENDING
- Result: [TO BE FILLED]

---

## Troubleshooting Guide

### Issue: MCP Server Won't Start
```bash
# Check database status
./target/release/cortex db status

# Check ports
lsof -i :6333  # Qdrant
lsof -i :8000  # SurrealDB

# Check logs
tail -f ~/.ryht/cortex/logs/cortex.log
```

### Issue: Agent Launch Fails
```bash
# Check agent registry
# Via MCP: cortex.agent.status with empty ID to list all

# Check workspace exists
# Via MCP: cortex.workspace.get

# Enable debug logging
export RUST_LOG=debug
```

### Issue: Orchestration Hangs
```bash
# Check running agents
ps aux | grep cortex

# Check agent status
# Via MCP: cortex.agent.status

# Force stop agent
# Via MCP: cortex.agent.stop
```

---

## Next Steps

1. Execute Phase 1 tests
2. Document results in log section
3. Fix any discovered issues
4. Proceed to Phase 2
5. Continue through all phases
6. Generate final test report

---

**Report Generated:** 2025-11-04
**Tester:** Claude (Sonnet 4.5)
**Status:** Ready for Execution
