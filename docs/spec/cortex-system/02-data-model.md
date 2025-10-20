# Cortex: Data Model & SurrealDB Schema

## Conceptual Architecture

The Cortex data model represents a complete codebase as a multi-layered semantic graph with full versioning, cognitive memory, and virtual filesystem capabilities.

### Core Principles

1. **Everything is a Node**: Files, directories, functions, types—all are nodes in the graph
2. **Semantic Relationships**: Edges represent meaningful relationships (contains, depends_on, implements)
3. **Immutable Versions**: Every change creates a new version, preserving full history
4. **Lazy Materialization**: Physical files generated on-demand from graph state
5. **Multi-Tenant Isolation**: Agents work in isolated namespaces with merge capabilities

## Schema Architecture

### Namespace Structure

```surrealql
-- Each project gets its own namespace for isolation
-- Format: cortex_<project_id>
USE NS cortex_project_abc123;
USE DB knowledge;

-- Global namespace for cross-project data
USE NS cortex_global;
USE DB registry;
```

## Core Tables

### 1. Workspace Table

Represents a complete project or monorepo.

```surrealql
DEFINE TABLE workspace SCHEMAFULL;

-- Identity
DEFINE FIELD id ON workspace TYPE string;
DEFINE FIELD name ON workspace TYPE string;
DEFINE FIELD workspace_type ON workspace TYPE string
    ASSERT $value IN ['rust_cargo', 'typescript_turborepo', 'typescript_nx', 'python_poetry', 'go_modules', 'mixed'];
DEFINE FIELD source_type ON workspace TYPE string DEFAULT 'local'
    ASSERT $value IN ['local', 'external_readonly', 'fork', 'imported_document'];

-- Configuration
DEFINE FIELD root_path ON workspace TYPE string;  -- Original filesystem path
DEFINE FIELD config ON workspace TYPE object;      -- Build system config (Cargo.toml, package.json, etc)
DEFINE FIELD environment ON workspace TYPE object; -- Environment variables, tools versions
DEFINE FIELD parent_workspace ON workspace TYPE option<record<workspace>> DEFAULT NONE; -- For forks
DEFINE FIELD read_only ON workspace TYPE bool DEFAULT false;

-- Metadata
DEFINE FIELD description ON workspace TYPE string;
DEFINE FIELD tags ON workspace TYPE array<string> DEFAULT [];
DEFINE FIELD metadata ON workspace TYPE object DEFAULT {};

-- Git Integration
DEFINE FIELD git_remote ON workspace TYPE option<string> DEFAULT NONE;
DEFINE FIELD default_branch ON workspace TYPE string DEFAULT 'main';
DEFINE FIELD current_commit ON workspace TYPE option<string> DEFAULT NONE;

-- State
DEFINE FIELD status ON workspace TYPE string DEFAULT 'active'
    ASSERT $value IN ['active', 'archived', 'migrating'];
DEFINE FIELD last_synchronized ON workspace TYPE datetime DEFAULT time::now();
DEFINE FIELD materialization_state ON workspace TYPE string DEFAULT 'memory_only'
    ASSERT $value IN ['memory_only', 'synchronized', 'diverged', 'conflict'];

-- Versioning
DEFINE FIELD created_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE FIELD version ON workspace TYPE int DEFAULT 1;

-- Indexes
DEFINE INDEX workspace_name_idx ON workspace FIELDS name UNIQUE;
DEFINE INDEX workspace_status_idx ON workspace FIELDS status;
```

### 2. Virtual Node Table

Core abstraction for all entities in the virtual filesystem.

```surrealql
DEFINE TABLE vnode SCHEMAFULL;

-- Identity
DEFINE FIELD id ON vnode TYPE string;
DEFINE FIELD node_type ON vnode TYPE string
    ASSERT $value IN ['directory', 'file', 'symlink', 'virtual', 'document'];
DEFINE FIELD name ON vnode TYPE string;
DEFINE FIELD path ON vnode TYPE string;  -- Full virtual path (relative to repo root)
DEFINE FIELD source_path ON vnode TYPE option<string> DEFAULT NONE;  -- Original physical path for external files
DEFINE FIELD read_only ON vnode TYPE bool DEFAULT false;  -- For external/imported content

-- Content
DEFINE FIELD content_type ON vnode TYPE string DEFAULT 'none'
    ASSERT $value IN ['none', 'text', 'binary', 'generated'];
DEFINE FIELD content_hash ON vnode TYPE option<string> DEFAULT NONE;  -- SHA256 of content
DEFINE FIELD size_bytes ON vnode TYPE int DEFAULT 0;

-- Filesystem Attributes
DEFINE FIELD permissions ON vnode TYPE string DEFAULT '644';
DEFINE FIELD is_executable ON vnode TYPE bool DEFAULT false;
DEFINE FIELD is_hidden ON vnode TYPE bool DEFAULT false;

-- File-specific
DEFINE FIELD mime_type ON vnode TYPE option<string> DEFAULT NONE;
DEFINE FIELD encoding ON vnode TYPE string DEFAULT 'utf-8';
DEFINE FIELD language ON vnode TYPE option<string> DEFAULT NONE
    ASSERT $value IN NONE OR $value IN ['rust', 'typescript', 'javascript', 'python', 'go', 'toml', 'json', 'yaml', 'markdown', 'other'];

-- State Management
DEFINE FIELD status ON vnode TYPE string DEFAULT 'synchronized'
    ASSERT $value IN ['synchronized', 'modified', 'created', 'deleted', 'moved', 'conflict'];
DEFINE FIELD modification_state ON vnode TYPE object DEFAULT {
    in_memory: false,
    on_disk: false,
    conflicts: []
};

-- Metadata
DEFINE FIELD metadata ON vnode TYPE object DEFAULT {};
DEFINE FIELD tags ON vnode TYPE array<string> DEFAULT [];

-- Versioning
DEFINE FIELD created_at ON vnode TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON vnode TYPE datetime DEFAULT time::now();
DEFINE FIELD version ON vnode TYPE int DEFAULT 1;
DEFINE FIELD created_by ON vnode TYPE string DEFAULT 'system';
DEFINE FIELD updated_by ON vnode TYPE string DEFAULT 'system';

-- Indexes
DEFINE INDEX vnode_path_idx ON vnode FIELDS path UNIQUE;
DEFINE INDEX vnode_type_idx ON vnode FIELDS node_type;
DEFINE INDEX vnode_status_idx ON vnode FIELDS status;
DEFINE INDEX vnode_language_idx ON vnode FIELDS language;
```

### 3. Document Content Table

Stores processed document content with chunks for semantic search.

```surrealql
DEFINE TABLE document_content SCHEMAFULL;

-- Identity
DEFINE FIELD id ON document_content TYPE string;
DEFINE FIELD vnode_id ON document_content TYPE record<vnode>;
DEFINE FIELD document_type ON document_content TYPE string
    ASSERT $value IN ['pdf', 'docx', 'doc', 'txt', 'md', 'rtf', 'html', 'epub', 'odt'];

-- Content
DEFINE FIELD full_text ON document_content TYPE string;  -- Full extracted text
DEFINE FIELD structured_content ON document_content TYPE option<object> DEFAULT NONE;  -- JSON representation
DEFINE FIELD chunks ON document_content TYPE array<object> DEFAULT [];  -- Semantic chunks

-- Metadata
DEFINE FIELD title ON document_content TYPE option<string> DEFAULT NONE;
DEFINE FIELD author ON document_content TYPE option<string> DEFAULT NONE;
DEFINE FIELD created_date ON document_content TYPE option<datetime> DEFAULT NONE;
DEFINE FIELD modified_date ON document_content TYPE option<datetime> DEFAULT NONE;
DEFINE FIELD metadata ON document_content TYPE object DEFAULT {};

-- Processing
DEFINE FIELD processing_status ON document_content TYPE string DEFAULT 'pending'
    ASSERT $value IN ['pending', 'processing', 'completed', 'failed'];
DEFINE FIELD processing_errors ON document_content TYPE array<string> DEFAULT [];
DEFINE FIELD processed_at ON document_content TYPE option<datetime> DEFAULT NONE;

-- Embeddings
DEFINE FIELD embeddings ON document_content TYPE array<array<float>> DEFAULT [];  -- Multiple embeddings for chunks
DEFINE FIELD embedding_model ON document_content TYPE string DEFAULT 'text-embedding-3-small';

-- Indexes
DEFINE INDEX document_vnode_idx ON document_content FIELDS vnode_id;
DEFINE INDEX document_type_idx ON document_content FIELDS document_type;
```

### 4. Content Chunk Table

Individual chunks from documents for fine-grained search.

```surrealql
DEFINE TABLE content_chunk SCHEMAFULL;

-- Identity
DEFINE FIELD id ON content_chunk TYPE string;
DEFINE FIELD document_id ON content_chunk TYPE record<document_content>;
DEFINE FIELD chunk_index ON content_chunk TYPE int;  -- Position in document

-- Content
DEFINE FIELD content ON content_chunk TYPE string;
DEFINE FIELD chunk_type ON content_chunk TYPE string
    ASSERT $value IN ['title', 'heading', 'paragraph', 'list', 'code', 'table', 'image_caption', 'footnote'];

-- Location
DEFINE FIELD page_number ON content_chunk TYPE option<int> DEFAULT NONE;
DEFINE FIELD section_path ON content_chunk TYPE array<string> DEFAULT [];  -- Hierarchical path

-- Semantic
DEFINE FIELD embedding ON content_chunk TYPE array<float> DEFAULT [];
DEFINE FIELD summary ON content_chunk TYPE option<string> DEFAULT NONE;
DEFINE FIELD keywords ON content_chunk TYPE array<string> DEFAULT [];

-- Metadata
DEFINE FIELD metadata ON content_chunk TYPE object DEFAULT {};

-- Indexes
DEFINE INDEX chunk_document_idx ON content_chunk FIELDS document_id;
DEFINE INDEX chunk_embedding_idx ON content_chunk FIELDS embedding MTREE DIMENSION 1536;
```

### 5. File Content Table

Stores actual file content with deduplication.

```surrealql
DEFINE TABLE file_content SCHEMAFULL;

-- Identity
DEFINE FIELD content_hash ON file_content TYPE string;  -- Primary key (SHA256)

-- Content
DEFINE FIELD content ON file_content TYPE string;       -- Actual file content
DEFINE FIELD content_binary ON file_content TYPE option<bytes> DEFAULT NONE; -- For binary files
DEFINE FIELD size_bytes ON file_content TYPE int;
DEFINE FIELD line_count ON file_content TYPE int DEFAULT 0;

-- Compression
DEFINE FIELD is_compressed ON file_content TYPE bool DEFAULT false;
DEFINE FIELD compression_type ON file_content TYPE option<string> DEFAULT NONE;
DEFINE FIELD compressed_size ON file_content TYPE option<int> DEFAULT NONE;

-- Parse Cache
DEFINE FIELD tree_sitter_ast ON file_content TYPE option<object> DEFAULT NONE;
DEFINE FIELD ast_version ON file_content TYPE option<string> DEFAULT NONE;  -- Tree-sitter version
DEFINE FIELD parse_errors ON file_content TYPE array<object> DEFAULT [];

-- Usage tracking
DEFINE FIELD reference_count ON file_content TYPE int DEFAULT 1;
DEFINE FIELD last_accessed ON file_content TYPE datetime DEFAULT time::now();

-- Indexes
DEFINE INDEX content_hash_idx ON file_content FIELDS content_hash UNIQUE;
```

### 4. Code Unit Table

Semantic units extracted from code files.

```surrealql
DEFINE TABLE code_unit SCHEMAFULL;

-- Identity
DEFINE FIELD id ON code_unit TYPE string;
DEFINE FIELD unit_type ON code_unit TYPE string
    ASSERT $value IN [
        'function', 'method', 'async_function', 'generator', 'lambda',
        'class', 'struct', 'enum', 'union', 'interface', 'trait',
        'type_alias', 'typedef', 'const', 'static', 'variable',
        'module', 'namespace', 'package',
        'impl_block', 'decorator', 'macro', 'template',
        'test', 'benchmark', 'example'
    ];

-- Identification
DEFINE FIELD name ON code_unit TYPE string;
DEFINE FIELD qualified_name ON code_unit TYPE string;  -- Full path: module::Class::method
DEFINE FIELD display_name ON code_unit TYPE string;     -- Human readable

-- Location
DEFINE FIELD file_node ON code_unit TYPE record<vnode>;
DEFINE FIELD start_line ON code_unit TYPE int;
DEFINE FIELD start_column ON code_unit TYPE int;
DEFINE FIELD end_line ON code_unit TYPE int;
DEFINE FIELD end_column ON code_unit TYPE int;
DEFINE FIELD span_bytes ON code_unit TYPE object DEFAULT { start: 0, end: 0 };

-- Code Content
DEFINE FIELD signature ON code_unit TYPE string;        -- Function/method signature
DEFINE FIELD body ON code_unit TYPE string;            -- Actual implementation
DEFINE FIELD source_range ON code_unit TYPE string;    -- Full source including decorators
DEFINE FIELD docstring ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD comments ON code_unit TYPE array<object> DEFAULT [];

-- Semantic Information
DEFINE FIELD visibility ON code_unit TYPE string DEFAULT 'private'
    ASSERT $value IN ['public', 'private', 'protected', 'internal', 'package'];
DEFINE FIELD modifiers ON code_unit TYPE array<string> DEFAULT [];  -- static, async, const, etc.
DEFINE FIELD annotations ON code_unit TYPE array<object> DEFAULT []; -- Decorators, attributes
DEFINE FIELD generic_params ON code_unit TYPE array<object> DEFAULT [];
DEFINE FIELD parameters ON code_unit TYPE array<object> DEFAULT [];
DEFINE FIELD return_type ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD throws ON code_unit TYPE array<string> DEFAULT [];

-- Language-Specific
DEFINE FIELD language ON code_unit TYPE string;
DEFINE FIELD language_specific ON code_unit TYPE object DEFAULT {};
-- Rust: lifetimes, unsafe, trait bounds
-- TypeScript: decorators, type guards
-- Python: type hints, async/await

-- Semantic Analysis
DEFINE FIELD summary ON code_unit TYPE string;         -- AI-generated summary
DEFINE FIELD purpose ON code_unit TYPE string;         -- What this unit does
DEFINE FIELD complexity ON code_unit TYPE object DEFAULT {
    cyclomatic: 1,
    cognitive: 1,
    nesting: 0,
    lines: 0
};

-- Embedding for Semantic Search
DEFINE FIELD embedding ON code_unit TYPE array<float> DEFAULT [];
DEFINE FIELD embedding_model ON code_unit TYPE string DEFAULT 'text-embedding-3-small';

-- Quality Metrics
DEFINE FIELD test_coverage ON code_unit TYPE option<float> DEFAULT NONE;
DEFINE FIELD has_tests ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD has_documentation ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD last_modified_by ON code_unit TYPE string;

-- Tree-sitter AST Node
DEFINE FIELD ast_node ON code_unit TYPE object;
DEFINE FIELD ast_node_type ON code_unit TYPE string;

-- State
DEFINE FIELD status ON code_unit TYPE string DEFAULT 'active'
    ASSERT $value IN ['active', 'deprecated', 'deleted', 'moved'];

-- Metadata
DEFINE FIELD tags ON code_unit TYPE array<string> DEFAULT [];
DEFINE FIELD metadata ON code_unit TYPE object DEFAULT {};

-- Versioning
DEFINE FIELD created_at ON code_unit TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON code_unit TYPE datetime DEFAULT time::now();
DEFINE FIELD version ON code_unit TYPE int DEFAULT 1;

-- Indexes
DEFINE INDEX code_unit_qualified_name_idx ON code_unit FIELDS qualified_name;
DEFINE INDEX code_unit_type_idx ON code_unit FIELDS unit_type;
DEFINE INDEX code_unit_file_idx ON code_unit FIELDS file_node;
DEFINE INDEX code_unit_embedding_idx ON code_unit FIELDS embedding MTREE DIMENSION 1536;
```

### 5. Version History Tables

Complete version tracking for all entities.

```surrealql
-- Version history for virtual nodes
DEFINE TABLE vnode_version SCHEMAFULL;

DEFINE FIELD id ON vnode_version TYPE string;
DEFINE FIELD vnode_id ON vnode_version TYPE record<vnode>;
DEFINE FIELD version ON vnode_version TYPE int;
DEFINE FIELD operation ON vnode_version TYPE string
    ASSERT $value IN ['create', 'update', 'delete', 'move', 'rename'];

-- Snapshot of state at this version
DEFINE FIELD snapshot ON vnode_version TYPE object;
DEFINE FIELD content_hash ON vnode_version TYPE option<string> DEFAULT NONE;

-- Change information
DEFINE FIELD changed_by ON vnode_version TYPE string;
DEFINE FIELD change_description ON vnode_version TYPE string;
DEFINE FIELD changed_at ON vnode_version TYPE datetime DEFAULT time::now();

-- Diff information
DEFINE FIELD diff ON vnode_version TYPE option<object> DEFAULT NONE;
DEFINE FIELD lines_added ON vnode_version TYPE int DEFAULT 0;
DEFINE FIELD lines_removed ON vnode_version TYPE int DEFAULT 0;

DEFINE INDEX vnode_version_idx ON vnode_version FIELDS vnode_id, version UNIQUE;


-- Version history for code units
DEFINE TABLE code_unit_version SCHEMAFULL;

DEFINE FIELD id ON code_unit_version TYPE string;
DEFINE FIELD unit_id ON code_unit_version TYPE record<code_unit>;
DEFINE FIELD version ON code_unit_version TYPE int;
DEFINE FIELD operation ON code_unit_version TYPE string;

-- Code snapshot
DEFINE FIELD signature ON code_unit_version TYPE string;
DEFINE FIELD body ON code_unit_version TYPE string;
DEFINE FIELD metadata ON code_unit_version TYPE object;

-- Semantic changes
DEFINE FIELD breaking_change ON code_unit_version TYPE bool DEFAULT false;
DEFINE FIELD api_change ON code_unit_version TYPE bool DEFAULT false;
DEFINE FIELD behavior_change ON code_unit_version TYPE bool DEFAULT false;

-- Change metadata
DEFINE FIELD changed_by ON code_unit_version TYPE string;
DEFINE FIELD change_description ON code_unit_version TYPE string;
DEFINE FIELD changed_at ON code_unit_version TYPE datetime DEFAULT time::now();

DEFINE INDEX unit_version_idx ON code_unit_version FIELDS unit_id, version UNIQUE;
```

### 6. Cognitive Memory Tables

Episodic and semantic memory for learning.

```surrealql
-- Episodes of development work
DEFINE TABLE episode SCHEMAFULL;

DEFINE FIELD id ON episode TYPE string;
DEFINE FIELD episode_type ON episode TYPE string
    ASSERT $value IN ['task', 'refactor', 'bugfix', 'feature', 'exploration'];

-- Context
DEFINE FIELD task_description ON episode TYPE string;
DEFINE FIELD agent_id ON episode TYPE string;
DEFINE FIELD session_id ON episode TYPE option<record<session>> DEFAULT NONE;
DEFINE FIELD workspace_id ON episode TYPE record<workspace>;

-- Work performed
DEFINE FIELD entities_created ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD entities_modified ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD entities_deleted ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD files_touched ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD queries_made ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD tools_used ON episode TYPE array<object> DEFAULT [];

-- Outcome
DEFINE FIELD solution_summary ON episode TYPE string;
DEFINE FIELD outcome ON episode TYPE string
    ASSERT $value IN ['success', 'partial', 'failure', 'abandoned'];
DEFINE FIELD success_metrics ON episode TYPE object DEFAULT {};
DEFINE FIELD errors_encountered ON episode TYPE array<string> DEFAULT [];
DEFINE FIELD lessons_learned ON episode TYPE array<string> DEFAULT [];

-- Performance
DEFINE FIELD duration_seconds ON episode TYPE int;
DEFINE FIELD tokens_used ON episode TYPE object DEFAULT {
    input: 0,
    output: 0,
    total: 0
};

-- Semantic representation
DEFINE FIELD embedding ON episode TYPE array<float> DEFAULT [];

-- Versioning
DEFINE FIELD created_at ON episode TYPE datetime DEFAULT time::now();
DEFINE FIELD completed_at ON episode TYPE option<datetime> DEFAULT NONE;

DEFINE INDEX episode_embedding_idx ON episode FIELDS embedding MTREE DIMENSION 1536;
DEFINE INDEX episode_agent_idx ON episode FIELDS agent_id;
DEFINE INDEX episode_outcome_idx ON episode FIELDS outcome;


-- Learned patterns
DEFINE TABLE pattern SCHEMAFULL;

DEFINE FIELD id ON pattern TYPE string;
DEFINE FIELD pattern_type ON pattern TYPE string
    ASSERT $value IN ['code', 'architecture', 'refactor', 'optimization', 'error_recovery'];

DEFINE FIELD name ON pattern TYPE string;
DEFINE FIELD description ON pattern TYPE string;
DEFINE FIELD context ON pattern TYPE string;

-- Pattern definition
DEFINE FIELD before_state ON pattern TYPE object;
DEFINE FIELD after_state ON pattern TYPE object;
DEFINE FIELD transformation ON pattern TYPE object;

-- Usage statistics
DEFINE FIELD times_applied ON pattern TYPE int DEFAULT 0;
DEFINE FIELD success_rate ON pattern TYPE float DEFAULT 0.0;
DEFINE FIELD average_improvement ON pattern TYPE object DEFAULT {};

-- Examples
DEFINE FIELD example_episodes ON pattern TYPE array<record<episode>> DEFAULT [];

-- Semantic search
DEFINE FIELD embedding ON pattern TYPE array<float> DEFAULT [];

DEFINE INDEX pattern_embedding_idx ON pattern FIELDS embedding MTREE DIMENSION 1536;
DEFINE INDEX pattern_type_idx ON pattern FIELDS pattern_type;
```

### 7. Relationship Tables

Graph edges connecting entities.

```surrealql
-- Virtual filesystem hierarchy
DEFINE TABLE CONTAINS SCHEMAFULL;
DEFINE FIELD in ON CONTAINS TYPE record<vnode | workspace>;
DEFINE FIELD out ON CONTAINS TYPE record<vnode>;
DEFINE FIELD order_index ON CONTAINS TYPE int DEFAULT 0;  -- For ordering siblings
DEFINE FIELD metadata ON CONTAINS TYPE object DEFAULT {};

DEFINE INDEX contains_parent_idx ON CONTAINS FIELDS in;
DEFINE INDEX contains_child_idx ON CONTAINS FIELDS out;


-- Code unit containment
DEFINE TABLE DEFINES SCHEMAFULL;
DEFINE FIELD in ON DEFINES TYPE record<vnode>;          -- File
DEFINE FIELD out ON DEFINES TYPE record<code_unit>;     -- Unit defined in file
DEFINE FIELD order_index ON DEFINES TYPE int;           -- Order in file
DEFINE FIELD is_exported ON DEFINES TYPE bool DEFAULT false;
DEFINE FIELD is_default ON DEFINES TYPE bool DEFAULT false;


-- Code dependencies
DEFINE TABLE DEPENDS_ON SCHEMAFULL;
DEFINE FIELD in ON DEPENDS_ON TYPE record<code_unit | vnode>;
DEFINE FIELD out ON DEPENDS_ON TYPE record<code_unit | vnode>;
DEFINE FIELD dependency_type ON DEPENDS_ON TYPE string
    ASSERT $value IN [
        'imports', 'requires', 'includes',
        'calls', 'invokes', 'instantiates',
        'extends', 'implements', 'inherits',
        'uses_type', 'uses_trait', 'uses_interface',
        'reads', 'writes', 'modifies'
    ];
DEFINE FIELD is_direct ON DEPENDS_ON TYPE bool DEFAULT true;
DEFINE FIELD is_runtime ON DEPENDS_ON TYPE bool DEFAULT false;
DEFINE FIELD is_dev ON DEPENDS_ON TYPE bool DEFAULT false;
DEFINE FIELD metadata ON DEPENDS_ON TYPE object DEFAULT {};

DEFINE INDEX depends_source_idx ON DEPENDS_ON FIELDS in;
DEFINE INDEX depends_target_idx ON DEPENDS_ON FIELDS out;
DEFINE INDEX depends_type_idx ON DEPENDS_ON FIELDS dependency_type;


-- Semantic links
DEFINE TABLE LINKS_TO SCHEMAFULL;
DEFINE FIELD in ON LINKS_TO TYPE record;               -- Any entity
DEFINE FIELD out ON LINKS_TO TYPE record;              -- Any entity
DEFINE FIELD link_type ON LINKS_TO TYPE string
    ASSERT $value IN [
        'documents', 'implements', 'specifies',
        'tests', 'tested_by', 'validates',
        'examples', 'references', 'related_to',
        'generated_from', 'generates'
    ];
DEFINE FIELD confidence ON LINKS_TO TYPE float DEFAULT 1.0;
DEFINE FIELD metadata ON LINKS_TO TYPE object DEFAULT {};
DEFINE FIELD created_at ON LINKS_TO TYPE datetime DEFAULT time::now();
DEFINE FIELD created_by ON LINKS_TO TYPE string;


-- Version relationships
DEFINE TABLE DERIVED_FROM SCHEMAFULL;
DEFINE FIELD in ON DERIVED_FROM TYPE record;          -- New version
DEFINE FIELD out ON DERIVED_FROM TYPE record;         -- Previous version
DEFINE FIELD derivation_type ON DERIVED_FROM TYPE string
    ASSERT $value IN ['version', 'fork', 'merge', 'cherry_pick'];
DEFINE FIELD metadata ON DERIVED_FROM TYPE object DEFAULT {};
```

### 8. Session & Coordination Tables

Multi-agent coordination support.

```surrealql
-- Agent work sessions
DEFINE TABLE session SCHEMAFULL;

DEFINE FIELD id ON session TYPE string;
DEFINE FIELD agent_id ON session TYPE string;
DEFINE FIELD workspace_id ON session TYPE record<workspace>;

-- Session configuration
DEFINE FIELD isolation_level ON session TYPE string DEFAULT 'snapshot'
    ASSERT $value IN ['snapshot', 'read_committed', 'repeatable_read', 'serializable'];
DEFINE FIELD session_type ON session TYPE string DEFAULT 'development'
    ASSERT $value IN ['development', 'analysis', 'review', 'experimentation'];

-- Scope
DEFINE FIELD scope_paths ON session TYPE array<string> DEFAULT [];    -- Paths this session can access
DEFINE FIELD scope_units ON session TYPE array<string> DEFAULT [];    -- Specific units

-- State
DEFINE FIELD status ON session TYPE string DEFAULT 'active'
    ASSERT $value IN ['active', 'suspended', 'merging', 'completed', 'aborted'];
DEFINE FIELD base_version ON session TYPE int;                       -- Version session started from

-- Changes tracking
DEFINE FIELD changes ON session TYPE object DEFAULT {
    created: [],
    modified: [],
    deleted: [],
    moved: []
};
DEFINE FIELD change_count ON session TYPE int DEFAULT 0;

-- Conflict management
DEFINE FIELD conflicts ON session TYPE array<object> DEFAULT [];
DEFINE FIELD conflict_resolution ON session TYPE string DEFAULT 'manual'
    ASSERT $value IN ['manual', 'theirs', 'mine', 'merge'];

-- Metadata
DEFINE FIELD metadata ON session TYPE object DEFAULT {};
DEFINE FIELD created_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD expires_at ON session TYPE option<datetime> DEFAULT NONE;

DEFINE INDEX session_agent_idx ON session FIELDS agent_id;
DEFINE INDEX session_status_idx ON session FIELDS status;


-- Locks for coordination
DEFINE TABLE lock_record SCHEMAFULL;

DEFINE FIELD id ON lock_record TYPE string;
DEFINE FIELD locked_entity ON lock_record TYPE record;             -- What is locked
DEFINE FIELD lock_type ON lock_record TYPE string
    ASSERT $value IN ['exclusive', 'shared', 'intent_exclusive', 'intent_shared'];
DEFINE FIELD lock_scope ON lock_record TYPE string
    ASSERT $value IN ['entity', 'subtree', 'file', 'unit'];

-- Ownership
DEFINE FIELD owner_session ON lock_record TYPE record<session>;
DEFINE FIELD owner_agent ON lock_record TYPE string;

-- Timing
DEFINE FIELD acquired_at ON lock_record TYPE datetime DEFAULT time::now();
DEFINE FIELD expires_at ON lock_record TYPE datetime;
DEFINE FIELD released_at ON lock_record TYPE option<datetime> DEFAULT NONE;

-- State
DEFINE FIELD status ON lock_record TYPE string DEFAULT 'active'
    ASSERT $value IN ['active', 'waiting', 'released', 'expired'];

DEFINE INDEX lock_entity_idx ON lock_record FIELDS locked_entity;
DEFINE INDEX lock_owner_idx ON lock_record FIELDS owner_session;
DEFINE INDEX lock_status_idx ON lock_record FIELDS status;
```

### 9. Task & Project Management Tables

Integration with task tracking.

```surrealql
-- Tasks
DEFINE TABLE task SCHEMAFULL;

DEFINE FIELD id ON task TYPE string;
DEFINE FIELD title ON task TYPE string;
DEFINE FIELD description ON task TYPE string;

-- Classification
DEFINE FIELD task_type ON task TYPE string
    ASSERT $value IN ['feature', 'bugfix', 'refactor', 'documentation', 'test', 'chore'];
DEFINE FIELD priority ON task TYPE string DEFAULT 'medium'
    ASSERT $value IN ['critical', 'high', 'medium', 'low'];
DEFINE FIELD status ON task TYPE string DEFAULT 'pending'
    ASSERT $value IN ['pending', 'in_progress', 'blocked', 'review', 'done', 'cancelled'];

-- Assignment
DEFINE FIELD assigned_to ON task TYPE array<string> DEFAULT [];
DEFINE FIELD created_by ON task TYPE string;

-- Tracking
DEFINE FIELD estimated_hours ON task TYPE option<float> DEFAULT NONE;
DEFINE FIELD actual_hours ON task TYPE option<float> DEFAULT NONE;
DEFINE FIELD progress_percentage ON task TYPE int DEFAULT 0;

-- Relationships
DEFINE FIELD parent_task ON task TYPE option<record<task>> DEFAULT NONE;
DEFINE FIELD blocking_tasks ON task TYPE array<record<task>> DEFAULT [];
DEFINE FIELD related_episodes ON task TYPE array<record<episode>> DEFAULT [];
DEFINE FIELD affected_units ON task TYPE array<record<code_unit>> DEFAULT [];
DEFINE FIELD spec_reference ON task TYPE option<object> DEFAULT NONE;

-- Metadata
DEFINE FIELD tags ON task TYPE array<string> DEFAULT [];
DEFINE FIELD metadata ON task TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON task TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON task TYPE datetime DEFAULT time::now();
DEFINE FIELD started_at ON task TYPE option<datetime> DEFAULT NONE;
DEFINE FIELD completed_at ON task TYPE option<datetime> DEFAULT NONE;
DEFINE FIELD due_date ON task TYPE option<datetime> DEFAULT NONE;

DEFINE INDEX task_status_idx ON task FIELDS status;
DEFINE INDEX task_assigned_idx ON task FIELDS assigned_to;
```

## Materialization Schema

Tables for managing filesystem synchronization.

```surrealql
-- Materialization operations
DEFINE TABLE flush_operation SCHEMAFULL;

DEFINE FIELD id ON flush_operation TYPE string;
DEFINE FIELD workspace_id ON flush_operation TYPE record<workspace>;
DEFINE FIELD operation_type ON flush_operation TYPE string
    ASSERT $value IN ['flush', 'sync', 'import', 'export'];

-- Scope
DEFINE FIELD scope_paths ON flush_operation TYPE array<string> DEFAULT [];
DEFINE FIELD scope_all ON flush_operation TYPE bool DEFAULT false;

-- Changes
DEFINE FIELD changes ON flush_operation TYPE object DEFAULT {
    files_created: [],
    files_modified: [],
    files_deleted: [],
    total_changes: 0
};

-- Results
DEFINE FIELD status ON flush_operation TYPE string DEFAULT 'pending'
    ASSERT $value IN ['pending', 'running', 'success', 'partial', 'failed'];
DEFINE FIELD errors ON flush_operation TYPE array<object> DEFAULT [];
DEFINE FIELD warnings ON flush_operation TYPE array<object> DEFAULT [];

-- Performance
DEFINE FIELD files_processed ON flush_operation TYPE int DEFAULT 0;
DEFINE FIELD bytes_written ON flush_operation TYPE int DEFAULT 0;
DEFINE FIELD duration_ms ON flush_operation TYPE option<int> DEFAULT NONE;

-- Metadata
DEFINE FIELD triggered_by ON flush_operation TYPE string;
DEFINE FIELD trigger_reason ON flush_operation TYPE string;
DEFINE FIELD created_at ON flush_operation TYPE datetime DEFAULT time::now();
DEFINE FIELD completed_at ON flush_operation TYPE option<datetime> DEFAULT NONE;


-- File system watch events
DEFINE TABLE fs_event SCHEMAFULL;

DEFINE FIELD id ON fs_event TYPE string;
DEFINE FIELD event_type ON fs_event TYPE string
    ASSERT $value IN ['created', 'modified', 'deleted', 'renamed', 'moved'];
DEFINE FIELD file_path ON fs_event TYPE string;
DEFINE FIELD vnode_id ON fs_event TYPE option<record<vnode>> DEFAULT NONE;

-- Event details
DEFINE FIELD old_path ON fs_event TYPE option<string> DEFAULT NONE;  -- For rename/move
DEFINE FIELD size_bytes ON fs_event TYPE option<int> DEFAULT NONE;
DEFINE FIELD content_hash ON fs_event TYPE option<string> DEFAULT NONE;

-- Processing
DEFINE FIELD status ON fs_event TYPE string DEFAULT 'pending'
    ASSERT $value IN ['pending', 'processing', 'processed', 'ignored', 'failed'];
DEFINE FIELD processed_at ON fs_event TYPE option<datetime> DEFAULT NONE;
DEFINE FIELD error ON fs_event TYPE option<string> DEFAULT NONE;

-- Metadata
DEFINE FIELD detected_at ON fs_event TYPE datetime DEFAULT time::now();

DEFINE INDEX fs_event_status_idx ON fs_event FIELDS status;
DEFINE INDEX fs_event_path_idx ON fs_event FIELDS file_path;
```

## Aggregation Views

Pre-computed views for performance.

```surrealql
-- Workspace statistics
DEFINE TABLE workspace_stats AS
    SELECT
        id,
        count(<-CONTAINS<-vnode) as total_nodes,
        count(<-CONTAINS<-vnode[WHERE node_type = 'file']) as file_count,
        count(<-CONTAINS<-vnode[WHERE node_type = 'directory']) as dir_count,
        count(SELECT * FROM code_unit WHERE file_node IN <-CONTAINS<-vnode) as unit_count,
        sum(<-CONTAINS<-vnode.size_bytes) as total_bytes
    FROM workspace
    GROUP BY id;

-- Code complexity metrics
DEFINE TABLE complexity_stats AS
    SELECT
        file_node as file_id,
        count(*) as unit_count,
        avg(complexity.cyclomatic) as avg_cyclomatic,
        max(complexity.cyclomatic) as max_cyclomatic,
        avg(complexity.cognitive) as avg_cognitive,
        sum(complexity.lines) as total_lines
    FROM code_unit
    GROUP BY file_node;

-- Dependency statistics
DEFINE TABLE dependency_stats AS
    SELECT
        in as source,
        count(*) as dependency_count,
        array::distinct(dependency_type) as dependency_types
    FROM DEPENDS_ON
    GROUP BY in;
```

## Index Strategy

### Primary Indexes
- Unique paths for fast lookups
- Content hashes for deduplication
- Qualified names for symbol resolution

### Search Indexes
- MTREE vector indexes for semantic search
- Full-text indexes on content and documentation
- Type-based indexes for filtered queries

### Performance Indexes
- Status fields for queue processing
- Timestamp fields for time-based queries
- Relationship indexes for graph traversal

## Data Integrity Rules

### Constraints
1. **Path Uniqueness**: No duplicate paths in vnode table
2. **Version Monotonicity**: Versions always increment
3. **Content Integrity**: Content hash must match actual content
4. **Relationship Consistency**: Both ends of edges must exist
5. **Lock Safety**: No entity can have conflicting locks

### Triggers
1. **Auto-versioning**: Create version entry on update
2. **Reference Counting**: Update content reference count
3. **Cascading Deletes**: Remove orphaned relationships
4. **Conflict Detection**: Flag conflicts on concurrent edits
5. **Metric Updates**: Recalculate stats on changes

## Migration Path from v2

### Data Import Process
1. Scan filesystem and create vnode hierarchy
2. Parse all code files with tree-sitter
3. Extract code units and build dependency graph
4. Import existing episodes from v2 database
5. Generate embeddings for semantic search
6. Verify integrity and create indexes

### Backwards Compatibility
- v2 MCP tools continue working via adapter layer
- Gradual migration of tools to v3 schema
- Dual-write during transition period
- Rollback capability maintained

## Performance Considerations

### Expected Scale
- 10M+ vnodes (files/directories)
- 100M+ code units
- 1B+ dependency edges
- 10M+ episodes
- 100+ concurrent sessions

### Optimization Strategies
1. **Lazy Loading**: Only load requested data
2. **Caching**: Memory cache for hot paths
3. **Partitioning**: Separate namespaces per project
4. **Compression**: LZ4 for large content
5. **Batch Operations**: Bulk inserts/updates
6. **Async Processing**: Background indexing

## Security Model

### Access Control
- Row-level security per workspace
- Agent-specific permissions
- Session isolation boundaries
- Read/write/execute permissions

### Audit Trail
- All changes logged with actor
- Immutable version history
- Cryptographic content verification
- Tamper detection via hash chains

## Conclusion

This comprehensive data model provides the foundation for Cortex. It enables:

1. **Complete Virtual Filesystem**: Full project representation in memory
2. **Semantic Code Understanding**: Deep AST-based code analysis
3. **Multi-Agent Coordination**: Isolated sessions with merge capabilities
4. **Continuous Learning**: Episodic memory and pattern extraction
5. **Bidirectional Sync**: Seamless filesystem materialization

The schema is designed for massive scale, high concurrency, and intelligent operations—making it the ideal foundation for next-generation AI-powered development.