# Cortex Memory System

A comprehensive cognitive memory architecture implementing episodic, semantic, working, and procedural memory systems based on cognitive science principles.

## Overview

The Cortex Memory System provides a complete cognitive memory architecture for AI agents, enabling them to:

- **Remember** development experiences and outcomes
- **Learn** from successful patterns and failures
- **Reason** about code structure and relationships
- **Adapt** based on accumulated knowledge
- **Forget** low-value information to maintain efficiency

## Architecture

### Memory Systems

#### 1. Episodic Memory

Stores complete development episodes with rich context including:

- Task descriptions and goals
- Entities created, modified, and deleted
- Tools used and queries made
- Outcomes and success metrics
- Errors encountered and lessons learned
- Performance statistics (duration, tokens used)

**Key Features:**
- Importance-based retention
- Pattern extraction from successful episodes
- Similarity search using embeddings
- Outcome-based retrieval
- Automatic decay of low-importance memories

```rust
use cortex_memory::prelude::*;

// Create an episode
let mut episode = EpisodicMemory::new(
    "Implement authentication".to_string(),
    "agent-001".to_string(),
    workspace_id,
    EpisodeType::Feature,
);

episode.outcome = EpisodeOutcome::Success;
episode.entities_created = vec!["auth.rs".to_string()];
episode.lessons_learned = vec!["Use JWT for stateless auth".to_string()];

// Store in memory
manager.remember_episode(&episode).await?;
```

#### 2. Semantic Memory

Stores code understanding including:

- Code units (functions, classes, methods, etc.)
- Complexity metrics
- Documentation and test coverage
- Type information and signatures
- Dependency relationships
- Cross-references

**Key Features:**
- Full semantic code representation
- Dependency graph tracking
- Complexity analysis
- Quality metrics (test coverage, documentation)
- Reference and definition tracking
- Semantic similarity search

```rust
// Create a semantic unit
let unit = SemanticUnit {
    id: CortexId::new(),
    unit_type: CodeUnitType::Function,
    name: "authenticate_user".to_string(),
    qualified_name: "auth::authenticate_user".to_string(),
    signature: "pub fn authenticate_user(...) -> Result<Token>".to_string(),
    complexity: ComplexityMetrics { cyclomatic: 5, ... },
    has_tests: true,
    has_documentation: true,
    ...
};

// Store and query
manager.remember_unit(&unit).await?;
let deps = manager.semantic().get_dependencies(unit.id).await?;
```

#### 3. Working Memory

Fast, temporary storage with:

- Priority-based retention (Critical, High, Medium, Low)
- Automatic eviction based on retention scores
- Capacity limits (items and bytes)
- Access pattern tracking
- Cache hit/miss statistics

**Key Features:**
- Fixed capacity with smart eviction
- Priority-based retention
- Recency and frequency tracking
- Automatic memory management
- High-performance concurrent access

```rust
// Store with priority
manager.working().store(
    "temp_data".to_string(),
    vec![1, 2, 3],
    Priority::High
);

// Retrieve and update access stats
if let Some(data) = manager.working().retrieve("temp_data") {
    // Use data
}

// Statistics
let stats = manager.working().get_statistics();
println!("Hit rate: {}", stats.cache_hit_rate);
```

#### 4. Procedural Memory

Stores learned patterns and procedures:

- Code patterns
- Architecture patterns
- Refactoring patterns
- Optimization patterns
- Error recovery patterns
- Success rate tracking
- Usage statistics

**Key Features:**
- Pattern learning from episodes
- Success rate tracking
- Example episode linking
- Pattern similarity search
- Application statistics

```rust
// Create a learned pattern
let pattern = LearnedPattern::new(
    PatternType::Refactor,
    "Extract method".to_string(),
    "Extract long methods into smaller functions".to_string(),
    "Complexity reduction".to_string(),
);

manager.remember_pattern(&pattern).await?;

// Record successful application
manager.procedural().record_success(pattern.id).await?;
```

### Memory Consolidation

Transfers memories from working to long-term storage with:

- Importance scoring algorithm
- Memory decay simulation
- Association strengthening
- Pattern extraction ("dreaming")

```rust
// Consolidate memories
let consolidated = manager.consolidate().await?;

// Extract patterns through dreaming
let patterns = manager.dream().await?;
```

### Cognitive Manager

Central orchestrator providing unified access to all memory systems through cognitive operations:

#### Cognitive Operations

**Remember** - Store new memories:
```rust
manager.remember_episode(&episode).await?;
manager.remember_unit(&unit).await?;
manager.remember_pattern(&pattern).await?;
```

**Recall** - Retrieve relevant memories:
```rust
let episodes = manager.recall_episodes(&query, &embedding).await?;
let units = manager.recall_units(&query, &embedding).await?;
let patterns = manager.recall_patterns(&query, &embedding).await?;
```

**Associate** - Link related memories:
```rust
manager.associate(source_id, target_id, DependencyType::Calls).await?;
```

**Forget** - Remove low-importance memories:
```rust
let forgotten = manager.forget(0.3).await?; // Forget below 30% importance
```

**Dream** - Pattern extraction and consolidation:
```rust
let patterns = manager.dream().await?;
```

## Database Schema

The memory system uses SurrealDB with the following schema (from `02-data-model.md`):

### Episode Table
```sql
CREATE episode CONTENT {
    id, episode_type, task_description, agent_id, session_id,
    workspace_id, entities_created, entities_modified, entities_deleted,
    files_touched, queries_made, tools_used, solution_summary, outcome,
    success_metrics, errors_encountered, lessons_learned,
    duration_seconds, tokens_used, embedding, created_at, completed_at
}
```

### Code Unit Table
```sql
CREATE code_unit CONTENT {
    id, unit_type, name, qualified_name, display_name,
    file_path, start_line, start_column, end_line, end_column,
    signature, body, docstring, visibility, modifiers,
    parameters, return_type, summary, purpose, complexity,
    test_coverage, has_tests, has_documentation,
    embedding, created_at, updated_at
}
```

### Dependency Table
```sql
CREATE DEPENDS_ON CONTENT {
    id, in, out, dependency_type,
    is_direct, is_runtime, is_dev, metadata
}
```

### Pattern Table
```sql
CREATE pattern CONTENT {
    id, pattern_type, name, description, context,
    before_state, after_state, transformation,
    times_applied, success_rate, average_improvement,
    example_episodes, embedding, created_at, updated_at
}
```

## Integration with Storage

The memory system integrates with `cortex-storage` for database operations:

```rust
use cortex_storage::{ConnectionManager, DatabaseConfig, PoolConfig};

// Create connection manager
let config = DatabaseConfig::new("127.0.0.1:8000", "cortex", "knowledge");
let pool_config = PoolConfig::default();
let connection_manager = Arc::new(
    ConnectionManager::new(config, pool_config).await?
);

// Create cognitive manager
let manager = CognitiveManager::new(connection_manager);
```

## Usage Examples

### Complete Workflow

```rust
use cortex_memory::prelude::*;
use cortex_storage::{ConnectionManager, DatabaseConfig, PoolConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize
    let config = DatabaseConfig::memory();
    let pool_config = PoolConfig::default();
    let conn_manager = Arc::new(
        ConnectionManager::new(config, pool_config).await?
    );
    let manager = CognitiveManager::new(conn_manager);

    // Store an episode
    let episode = EpisodicMemory::new(
        "Implement feature X".to_string(),
        "agent-001".to_string(),
        workspace_id,
        EpisodeType::Feature,
    );
    manager.remember_episode(&episode).await?;

    // Store semantic information
    let unit = SemanticUnit { /* ... */ };
    manager.remember_unit(&unit).await?;

    // Use working memory for temporary data
    manager.working().store(
        "cache_key".to_string(),
        data,
        Priority::Medium
    );

    // Search similar episodes
    let query = MemoryQuery::new("authentication".to_string())
        .with_limit(10)
        .with_threshold(0.7);
    let similar = manager.recall_episodes(&query, &embedding).await?;

    // Extract patterns
    let patterns = manager.dream().await?;

    // Get statistics
    let stats = manager.get_statistics().await?;
    println!("Episodes: {}", stats.episodic.total_episodes);
    println!("Code units: {}", stats.semantic.total_units);
    println!("Cache hit rate: {}", stats.working.cache_hit_rate);
    println!("Patterns: {}", stats.procedural.total_patterns);

    Ok(())
}
```

### Pattern Extraction from Episodes

```rust
// Store multiple successful episodes
for i in 0..10 {
    let mut episode = EpisodicMemory::new(
        format!("Task {}", i),
        "agent-001".to_string(),
        workspace_id,
        EpisodeType::Task,
    );
    episode.outcome = EpisodeOutcome::Success;
    episode.tools_used = vec![
        ToolUsage {
            tool_name: "code_analyzer".to_string(),
            usage_count: 1,
            total_duration_ms: 500,
            parameters: HashMap::new(),
        }
    ];
    manager.remember_episode(&episode).await?;
}

// Extract common patterns
let patterns = manager.episodic().extract_patterns(0.6).await?;
println!("Extracted {} patterns", patterns.len());
```

### Code Quality Analysis

```rust
// Find complex code units
let complex = manager.semantic().find_complex_units(15).await?;
println!("Found {} complex units", complex.len());

// Find untested code
let untested = manager.semantic().find_untested_units().await?;
println!("Found {} untested units", untested.len());

// Find undocumented public APIs
let undocumented = manager.semantic().find_undocumented_units().await?;
println!("Found {} undocumented units", undocumented.len());

// Analyze file complexity
let metrics = manager.semantic()
    .analyze_file_complexity("src/main.rs").await?;
println!("Complexity: {:?}", metrics);
```

### Dependency Analysis

```rust
// Store dependencies
manager.associate(
    function_id,
    dependency_id,
    DependencyType::Calls
).await?;

// Get dependency graph
let unit_ids = vec![id1, id2, id3];
let graph = manager.semantic()
    .get_dependency_graph(&unit_ids).await?;

// Find references
let refs = manager.semantic().find_references(unit_id).await?;
println!("Found {} references", refs.len());

// Find definitions
let defs = manager.semantic().find_definitions(unit_id).await?;
println!("Uses {} definitions", defs.len());
```

## Performance Considerations

### Memory Management

- **Working Memory**: Limited to configured capacity (default: 1000 items, 100MB)
- **Eviction Strategy**: Priority-based with recency and access frequency
- **Database Connections**: Pooled with automatic retry and circuit breaker

### Optimization Strategies

1. **Lazy Loading**: Only load requested data
2. **Batch Operations**: Use batch inserts for multiple memories
3. **Caching**: Working memory serves as L1 cache
4. **Async Operations**: All database operations are async
5. **Vector Search**: Efficient similarity search using SurrealDB MTREE indexes

### Scalability

- Designed for millions of episodes
- Supports massive codebases (100M+ code units)
- Concurrent access through DashMap
- Connection pooling with load balancing

## Testing

Comprehensive test suite covering:

- Unit tests for each memory system
- Integration tests for workflows
- Performance tests for eviction
- Pattern extraction tests
- Statistics verification

Run tests:
```bash
cargo test --package cortex-memory
```

Run integration tests:
```bash
cargo test --package cortex-memory --test integration_tests
```

## Implementation Status

- ✅ Complete type system for all memory types
- ✅ Episodic memory with pattern extraction
- ✅ Semantic memory with dependency tracking
- ✅ Working memory with priority-based eviction
- ✅ Procedural memory for learned patterns
- ✅ Memory consolidation and decay
- ✅ Cognitive manager with unified operations
- ✅ Comprehensive integration tests
- ✅ Database schema alignment with specification

## Future Enhancements

1. **Embedding Generation**: Integrate with embedding models for semantic search
2. **Advanced Pattern Mining**: ML-based pattern extraction
3. **Memory Compression**: Compress old episodes for storage efficiency
4. **Distributed Memory**: Multi-agent memory sharing
5. **Memory Visualization**: Dashboard for memory exploration
6. **Advanced Forgetting**: Sophisticated decay algorithms
7. **Memory Transfer**: Export/import memory between systems

## References

- Specification: `docs/spec/cortex-system/02-data-model.md`
- Storage Layer: `cortex-storage`
- Core Types: `cortex-core`

## License

Same as the Cortex project.
