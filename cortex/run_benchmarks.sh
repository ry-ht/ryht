#!/bin/bash
#
# Cortex Performance Benchmarking Suite Runner
#
# This script runs all performance benchmarks across Cortex subsystems
# and generates a comprehensive performance report.

set -e

echo "=========================================="
echo "Cortex Performance Benchmarking Suite"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

BENCHMARK_DIR="target/criterion"
REPORT_DIR="benchmark_reports"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
REPORT_FILE="${REPORT_DIR}/performance_report_${TIMESTAMP}.md"

# Create report directory
mkdir -p "${REPORT_DIR}"

echo "Starting benchmark suite at $(date)"
echo ""

# Function to run benchmark and capture output
run_benchmark() {
    local crate=$1
    local bench_name=$2
    local description=$3

    echo -e "${YELLOW}Running: ${description}${NC}"
    echo "  Crate: ${crate}"
    echo "  Benchmark: ${bench_name}"
    echo ""

    if [ -n "${crate}" ]; then
        cargo bench --package "${crate}" --bench "${bench_name}" -- --verbose
    else
        cargo bench --bench "${bench_name}" -- --verbose
    fi

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ ${description} completed${NC}"
    else
        echo -e "${RED}✗ ${description} failed${NC}"
    fi
    echo ""
}

# Start performance report
cat > "${REPORT_FILE}" << 'EOF'
# Cortex Performance Benchmark Report

**Generated:** $(date)
**Benchmark Suite Version:** 1.0.0

## Executive Summary

This report presents comprehensive performance benchmarks across all Cortex subsystems,
comparing actual performance against specification targets.

### Performance Targets (from spec)

- **Navigation Operations:** <50ms
- **Semantic Search:** <100ms
- **Code Manipulation:** <200ms
- **Flush 10K LOC to Disk:** <5s
- **Token Reduction:** 75%+ vs traditional approach

---

EOF

# ==============================================================================
# Part 1: Storage Layer Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 1: Storage Layer Benchmarks"
echo "=========================================="
echo ""

run_benchmark "cortex-storage" "storage_performance" "Storage Layer Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 1. Storage Layer Performance

### 1.1 Connection Pool Performance

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Connection Acquisition (single) | <5ms | See report | - |
| Concurrent Acquisition (10) | <10ms | See report | - |
| Concurrent Acquisition (50) | <50ms | See report | - |
| Pool Saturation (100) | <100ms | See report | - |
| Connection Recycling (10x) | <20ms | See report | - |

### 1.2 Query Performance

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| SELECT by ID | <1ms | See report | - |
| SELECT indexed | <1ms | See report | - |
| Range query (100) | <10ms | See report | - |
| Aggregation | <100ms | See report | - |
| Full scan (10K) | <50ms | See report | - |
| Text search | <50ms | See report | - |

### 1.3 Write Performance

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Single insert | <5ms | See report | - |
| Batch insert (10) | <50ms | See report | - |
| Batch insert (100) | <200ms | See report | - |
| Batch insert (1000) | <500ms | See report | - |
| Update single | <10ms | See report | - |
| Update bulk (100) | <100ms | See report | - |

---

EOF

# ==============================================================================
# Part 2: VFS Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 2: VFS Benchmarks"
echo "=========================================="
echo ""

run_benchmark "cortex-vfs" "vfs_performance" "VFS Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 2. Virtual Filesystem Performance

### 2.1 Navigation Operations

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| List directory (10 entries) | <50ms | See report | - |
| List directory (100 entries) | <50ms | See report | - |
| Recursive list (1000 entries) | <50ms | See report | - |
| Path resolution | <10ms | See report | - |
| Metadata retrieval | <5ms | See report | - |
| Path exists check | <5ms | See report | - |

### 2.2 File Operations

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Read file (512B) | <10ms | See report | - |
| Read file (10KB) | <10ms | See report | - |
| Read file (1MB) | <100ms | See report | - |
| Write file (512B) | <50ms | See report | - |
| Write file (10KB) | <50ms | See report | - |
| Delete file | <10ms | See report | - |
| Rename file | <20ms | See report | - |
| Copy file | <30ms | See report | - |

### 2.3 Cache Performance

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Cache hit | <1ms | See report | - |
| Cache miss | <10ms | See report | - |
| Metadata cache hit | <1ms | See report | - |

### 2.4 Materialization

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Flush 100 files (~10K LOC) | <5s | See report | - |
| Flush 1000 files (~100K LOC) | <30s | See report | - |

---

EOF

# ==============================================================================
# Part 3: Semantic Search Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 3: Semantic Search Benchmarks"
echo "=========================================="
echo ""

run_benchmark "cortex-semantic" "search_performance" "Semantic Search Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 3. Semantic Search Performance

### 3.1 Vector Search

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Search 100 vectors | <100ms | See report | - |
| Search 1K vectors | <100ms | See report | - |
| Search 10K vectors | <100ms | See report | - |
| Top-1 retrieval | <50ms | See report | - |
| Top-10 retrieval | <100ms | See report | - |
| Top-50 retrieval | <150ms | See report | - |

### 3.2 Hybrid Search

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Hybrid (keyword + semantic) | <150ms | See report | - |
| Keyword only | <50ms | See report | - |
| Re-rank top 100 | <20ms | See report | - |

### 3.3 Index Building

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Build HNSW (100 vectors) | <1s | See report | - |
| Build HNSW (1K vectors) | <5s | See report | - |
| Build HNSW (10K vectors) | <30s | See report | - |

### 3.4 Incremental Updates

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Insert single vector | <10ms | See report | - |
| Insert batch (100) | <100ms | See report | - |
| Delete single vector | <10ms | See report | - |
| Update single vector | <20ms | See report | - |

---

EOF

# ==============================================================================
# Part 4: Code Manipulation Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 4: Code Manipulation Benchmarks"
echo "=========================================="
echo ""

run_benchmark "cortex-code-analysis" "manipulation_performance" "Code Manipulation Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 4. Code Manipulation Performance

### 4.1 Parsing

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Parse 100 LOC (Rust) | <10ms | See report | - |
| Parse 1K LOC (Rust) | <50ms | See report | - |
| Parse 10K LOC (Rust) | <500ms | See report | - |
| Parse complex module (1K LOC) | <50ms | See report | - |
| Parse TypeScript (100 LOC) | <10ms | See report | - |

### 4.2 AST Queries

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Find all functions | <10ms | See report | - |
| Find all structs | <10ms | See report | - |
| Find all imports | <5ms | See report | - |
| Find node at position | <5ms | See report | - |
| Get function signature | <5ms | See report | - |

### 4.3 AST Editing

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Add function | <20ms | See report | - |
| Rename identifier | <50ms | See report | - |
| Delete function | <20ms | See report | - |
| Modify function body | <30ms | See report | - |
| Add parameter | <30ms | See report | - |
| Extract method | <100ms | See report | - |
| Add import | <15ms | See report | - |
| Inline variable | <40ms | See report | - |

### 4.4 Code Generation

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Generate simple function | <5ms | See report | - |
| Generate 100 LOC | <10ms | See report | - |
| Generate 1K LOC | <100ms | See report | - |
| Generate struct with methods | <10ms | See report | - |

---

EOF

# ==============================================================================
# Part 5: Memory System Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 5: Memory System Benchmarks"
echo "=========================================="
echo ""

run_benchmark "cortex-memory" "memory_performance" "Memory System Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 5. Memory System Performance

### 5.1 Working Memory

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Store item | <1ms | See report | - |
| Retrieve item | <1ms | See report | - |
| Update item | <2ms | See report | - |
| Delete item | <2ms | See report | - |
| Evict LRU | <5ms | See report | - |
| Batch store (100) | <10ms | See report | - |

### 5.2 Episodic Memory

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Store episode | <50ms | See report | - |
| Store complex episode | <100ms | See report | - |
| Query recent (10) | <100ms | See report | - |
| Query by type | <100ms | See report | - |
| Query time range | <150ms | See report | - |
| Find similar episodes | <200ms | See report | - |
| Extract patterns | <500ms | See report | - |

### 5.3 Semantic Memory

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Store code unit | <50ms | See report | - |
| Retrieve by ID | <10ms | See report | - |
| Search by name | <50ms | See report | - |
| Find dependencies | <100ms | See report | - |
| Find dependents | <100ms | See report | - |
| Build dependency graph (100) | <200ms | See report | - |

### 5.4 Memory Consolidation

| Operation | Target | Actual | Status |
|-----------|--------|--------|--------|
| Working → Episodic | <200ms | See report | - |
| Episodic → Semantic | <500ms | See report | - |
| Full consolidation cycle | <1s | See report | - |

---

EOF

# ==============================================================================
# Part 6: E2E Workflow Benchmarks
# ==============================================================================

echo "=========================================="
echo "Part 6: E2E Workflow Benchmarks"
echo "=========================================="
echo ""

run_benchmark "" "e2e_workflows" "E2E Workflow Performance"

cat >> "${REPORT_FILE}" << 'EOF'
## 6. End-to-End Workflow Performance

### 6.1 Code Analysis Workflows

| Workflow | Target | Actual | Status |
|----------|--------|--------|--------|
| Find all callers (100 files) | <2s | See report | - |
| Find all callers (500 files) | <5s | See report | - |
| Find all callers (1000 files) | <5s | See report | - |

### 6.2 Refactoring Workflows

| Workflow | Target | Actual | Status |
|----------|--------|--------|--------|
| Rename across 100 files | <3s | See report | - |
| Rename across 500 files | <8s | See report | - |
| Rename across 1000 files | <10s | See report | - |

### 6.3 Feature Implementation

| Workflow | Target | Actual | Status |
|----------|--------|--------|--------|
| Implement feature (5 files + 3 mods) | <2s | See report | - |

### 6.4 Search & Navigation

| Workflow | Target | Actual | Status |
|----------|--------|--------|--------|
| Semantic search + open | <200ms | See report | - |
| Text search (1000 files) | <500ms | See report | - |
| Go to definition | <100ms | See report | - |

### 6.5 Token Efficiency

| Approach | Tokens (approx) | Reduction | Status |
|----------|-----------------|-----------|--------|
| Cortex context | See report | - | - |
| Traditional full context | See report | - | - |
| **Reduction ratio** | - | **75%+ target** | - |

### 6.6 Multi-Agent Performance

| Workflow | Target | Actual | Status |
|----------|--------|--------|--------|
| 5 agents concurrent | <3s | See report | - |

---

## Summary & Recommendations

### Performance Targets Met

- [ ] Navigation operations <50ms
- [ ] Semantic search <100ms
- [ ] Code manipulation <200ms
- [ ] Flush 10K LOC <5s
- [ ] Token reduction 75%+

### Performance Gaps

*(To be filled after benchmark analysis)*

### Optimization Recommendations

*(To be filled after benchmark analysis)*

---

## Benchmark Details

For detailed statistical analysis including mean, median, standard deviation, and
confidence intervals, see the Criterion HTML reports in:

```
target/criterion/
```

View reports:
```bash
open target/criterion/report/index.html
```

EOF

echo ""
echo "=========================================="
echo "Benchmark Suite Complete"
echo "=========================================="
echo ""
echo -e "${GREEN}✓ All benchmarks completed${NC}"
echo ""
echo "Reports generated:"
echo "  - Markdown: ${REPORT_FILE}"
echo "  - HTML: target/criterion/report/index.html"
echo ""
echo "View HTML report:"
echo "  open target/criterion/report/index.html"
echo ""
echo "Compare benchmarks:"
echo "  cargo bench -- --save-baseline baseline_name"
echo "  cargo bench -- --baseline baseline_name"
echo ""
