-- Meridian Knowledge Graph Schema
-- This schema defines tables, fields, and relationships for the cognitive memory system

-- ============================================================================
-- Core Entity Tables
-- ============================================================================

-- Code Symbols: functions, classes, interfaces, variables, etc.
DEFINE TABLE IF NOT EXISTS code_symbol SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON TABLE code_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS name ON TABLE code_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS symbol_type ON TABLE code_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS file_path ON TABLE code_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS start_line ON TABLE code_symbol TYPE int;
DEFINE FIELD IF NOT EXISTS end_line ON TABLE code_symbol TYPE int;
DEFINE FIELD IF NOT EXISTS scope ON TABLE code_symbol TYPE option<string>;
DEFINE FIELD IF NOT EXISTS signature ON TABLE code_symbol TYPE option<string>;
DEFINE FIELD IF NOT EXISTS body ON TABLE code_symbol TYPE option<string>;
DEFINE FIELD IF NOT EXISTS language ON TABLE code_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS metadata ON TABLE code_symbol TYPE option<object>;
DEFINE FIELD IF NOT EXISTS embedding ON TABLE code_symbol TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE code_symbol TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE code_symbol TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_symbol_name ON TABLE code_symbol COLUMNS name;
DEFINE INDEX IF NOT EXISTS idx_symbol_type ON TABLE code_symbol COLUMNS symbol_type;
DEFINE INDEX IF NOT EXISTS idx_symbol_file ON TABLE code_symbol COLUMNS file_path;

-- Documentation entries
DEFINE TABLE IF NOT EXISTS documentation SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON TABLE documentation TYPE string;
DEFINE FIELD IF NOT EXISTS title ON TABLE documentation TYPE string;
DEFINE FIELD IF NOT EXISTS content ON TABLE documentation TYPE string;
DEFINE FIELD IF NOT EXISTS doc_type ON TABLE documentation TYPE string;
DEFINE FIELD IF NOT EXISTS file_path ON TABLE documentation TYPE string;
DEFINE FIELD IF NOT EXISTS section ON TABLE documentation TYPE option<string>;
DEFINE FIELD IF NOT EXISTS tags ON TABLE documentation TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS metadata ON TABLE documentation TYPE option<object>;
DEFINE FIELD IF NOT EXISTS embedding ON TABLE documentation TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE documentation TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE documentation TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_doc_title ON TABLE documentation COLUMNS title;
DEFINE INDEX IF NOT EXISTS idx_doc_type ON TABLE documentation COLUMNS doc_type;
DEFINE INDEX IF NOT EXISTS idx_doc_tags ON TABLE documentation COLUMNS tags;

-- Episodic memories: completed tasks and their solutions
DEFINE TABLE IF NOT EXISTS episode SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON TABLE episode TYPE string;
DEFINE FIELD IF NOT EXISTS task_description ON TABLE episode TYPE string;
DEFINE FIELD IF NOT EXISTS solution_summary ON TABLE episode TYPE string;
DEFINE FIELD IF NOT EXISTS solution_detail ON TABLE episode TYPE option<string>;
DEFINE FIELD IF NOT EXISTS files_touched ON TABLE episode TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS queries_made ON TABLE episode TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS tools_used ON TABLE episode TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS success_score ON TABLE episode TYPE float DEFAULT 1.0;
DEFINE FIELD IF NOT EXISTS duration_seconds ON TABLE episode TYPE option<int>;
DEFINE FIELD IF NOT EXISTS commit_hash ON TABLE episode TYPE option<string>;
DEFINE FIELD IF NOT EXISTS metadata ON TABLE episode TYPE option<object>;
DEFINE FIELD IF NOT EXISTS embedding ON TABLE episode TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE episode TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_episode_task ON TABLE episode COLUMNS task_description;
DEFINE INDEX IF NOT EXISTS idx_episode_score ON TABLE episode COLUMNS success_score;
DEFINE INDEX IF NOT EXISTS idx_episode_created ON TABLE episode COLUMNS created_at;

-- Tasks: current and historical task tracking
DEFINE TABLE IF NOT EXISTS task SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON TABLE task TYPE string;
DEFINE FIELD IF NOT EXISTS title ON TABLE task TYPE string;
DEFINE FIELD IF NOT EXISTS description ON TABLE task TYPE string;
DEFINE FIELD IF NOT EXISTS status ON TABLE task TYPE string;
DEFINE FIELD IF NOT EXISTS priority ON TABLE task TYPE string DEFAULT "medium";
DEFINE FIELD IF NOT EXISTS spec_name ON TABLE task TYPE option<string>;
DEFINE FIELD IF NOT EXISTS spec_section ON TABLE task TYPE option<string>;
DEFINE FIELD IF NOT EXISTS estimated_hours ON TABLE task TYPE option<float>;
DEFINE FIELD IF NOT EXISTS actual_hours ON TABLE task TYPE option<float>;
DEFINE FIELD IF NOT EXISTS tags ON TABLE task TYPE array<string> DEFAULT [];
DEFINE FIELD IF NOT EXISTS metadata ON TABLE task TYPE option<object>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS completed_at ON TABLE task TYPE option<datetime>;

DEFINE INDEX IF NOT EXISTS idx_task_status ON TABLE task COLUMNS status;
DEFINE INDEX IF NOT EXISTS idx_task_priority ON TABLE task COLUMNS priority;
DEFINE INDEX IF NOT EXISTS idx_task_spec ON TABLE task COLUMNS spec_name;
DEFINE INDEX IF NOT EXISTS idx_task_tags ON TABLE task COLUMNS tags;

-- Specifications: requirements and design documents
DEFINE TABLE IF NOT EXISTS specification SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS id ON TABLE specification TYPE string;
DEFINE FIELD IF NOT EXISTS name ON TABLE specification TYPE string;
DEFINE FIELD IF NOT EXISTS title ON TABLE specification TYPE string;
DEFINE FIELD IF NOT EXISTS file_path ON TABLE specification TYPE string;
DEFINE FIELD IF NOT EXISTS sections ON TABLE specification TYPE array<object> DEFAULT [];
DEFINE FIELD IF NOT EXISTS metadata ON TABLE specification TYPE option<object>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE specification TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE specification TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_spec_name ON TABLE specification COLUMNS name UNIQUE;

-- ============================================================================
-- Relationship Tables (Graph Edges)
-- ============================================================================

-- Symbol calls another symbol
DEFINE TABLE IF NOT EXISTS calls SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE calls TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS out ON TABLE calls TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS call_count ON TABLE calls TYPE int DEFAULT 1;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE calls TYPE datetime DEFAULT time::now();

-- Symbol depends on another symbol (imports, inherits, uses)
DEFINE TABLE IF NOT EXISTS depends_on SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE depends_on TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS out ON TABLE depends_on TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS dependency_type ON TABLE depends_on TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE depends_on TYPE datetime DEFAULT time::now();

-- Documentation documents a symbol
DEFINE TABLE IF NOT EXISTS documents SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE documents TYPE record<documentation>;
DEFINE FIELD IF NOT EXISTS out ON TABLE documents TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS relevance_score ON TABLE documents TYPE float DEFAULT 1.0;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE documents TYPE datetime DEFAULT time::now();

-- Episode references a symbol (symbol was used in the solution)
DEFINE TABLE IF NOT EXISTS references_symbol SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE references_symbol TYPE record<episode>;
DEFINE FIELD IF NOT EXISTS out ON TABLE references_symbol TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS reference_type ON TABLE references_symbol TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE references_symbol TYPE datetime DEFAULT time::now();

-- Episode solves a task
DEFINE TABLE IF NOT EXISTS solves SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE solves TYPE record<episode>;
DEFINE FIELD IF NOT EXISTS out ON TABLE solves TYPE record<task>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE solves TYPE datetime DEFAULT time::now();

-- Task implements a specification section
DEFINE TABLE IF NOT EXISTS implements SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE implements TYPE record<task>;
DEFINE FIELD IF NOT EXISTS out ON TABLE implements TYPE record<specification>;
DEFINE FIELD IF NOT EXISTS section_name ON TABLE implements TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE implements TYPE datetime DEFAULT time::now();

-- Symbol implements a specification requirement
DEFINE TABLE IF NOT EXISTS implements_spec SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE implements_spec TYPE record<code_symbol>;
DEFINE FIELD IF NOT EXISTS out ON TABLE implements_spec TYPE record<specification>;
DEFINE FIELD IF NOT EXISTS section_name ON TABLE implements_spec TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE implements_spec TYPE datetime DEFAULT time::now();

-- Episode is similar to another episode (semantic similarity)
DEFINE TABLE IF NOT EXISTS similar_to SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE similar_to TYPE record<episode>;
DEFINE FIELD IF NOT EXISTS out ON TABLE similar_to TYPE record<episode>;
DEFINE FIELD IF NOT EXISTS similarity_score ON TABLE similar_to TYPE float;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE similar_to TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS idx_similar_score ON TABLE similar_to COLUMNS similarity_score;

-- Task depends on another task
DEFINE TABLE IF NOT EXISTS task_depends_on SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE task_depends_on TYPE record<task>;
DEFINE FIELD IF NOT EXISTS out ON TABLE task_depends_on TYPE record<task>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE task_depends_on TYPE datetime DEFAULT time::now();

-- ============================================================================
-- Vector Indexes for Semantic Search
-- ============================================================================

-- Vector index for code symbol embeddings (384 dimensions for MiniLM, 768 for BERT)
-- MTREE is used for efficient k-NN search
-- Note: Uncomment when SurrealDB version supports vector indexes (2.0+)
-- Current SurrealDB embedded version may not support MTREE syntax yet
-- For now, vector similarity is computed via vector::similarity::cosine() function

-- TODO: Enable when upgrading to SurrealDB 2.0+
-- DEFINE INDEX IF NOT EXISTS idx_symbol_embedding ON TABLE code_symbol FIELDS embedding MTREE DIMENSION 384 DIST COSINE;
-- DEFINE INDEX IF NOT EXISTS idx_doc_embedding ON TABLE documentation FIELDS embedding MTREE DIMENSION 384 DIST COSINE;
-- DEFINE INDEX IF NOT EXISTS idx_episode_embedding ON TABLE episode FIELDS embedding MTREE DIMENSION 384 DIST COSINE;

-- Alternative: Use HNSW index for better performance (if supported)
-- DEFINE INDEX IF NOT EXISTS idx_symbol_embedding_hnsw ON TABLE code_symbol FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
-- DEFINE INDEX IF NOT EXISTS idx_doc_embedding_hnsw ON TABLE documentation FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
-- DEFINE INDEX IF NOT EXISTS idx_episode_embedding_hnsw ON TABLE episode FIELDS embedding HNSW DIMENSION 384 DIST COSINE;

-- ============================================================================
-- Utility Functions
-- ============================================================================

-- Function to find symbols by name pattern
DEFINE FUNCTION IF NOT EXISTS fn::find_symbols($pattern: string) {
    RETURN SELECT * FROM code_symbol WHERE name ~ $pattern;
};

-- Function to get symbol dependencies (recursive)
DEFINE FUNCTION IF NOT EXISTS fn::get_dependencies($symbol_id: string, $depth: int) {
    LET $deps = SELECT out.* FROM depends_on WHERE in = $symbol_id;
    RETURN $deps;
};

-- Function to find similar episodes by embedding
-- Note: Actual vector similarity search syntax depends on SurrealDB version
DEFINE FUNCTION IF NOT EXISTS fn::find_similar_episodes($embedding: array<float>, $limit: int) {
    RETURN SELECT * FROM episode ORDER BY vector::similarity::cosine(embedding, $embedding) DESC LIMIT $limit;
};

-- ============================================================================
-- Triggers for Automatic Updates
-- ============================================================================

-- Update updated_at timestamp on code_symbol changes
DEFINE EVENT IF NOT EXISTS update_symbol_timestamp ON TABLE code_symbol WHEN $event = "UPDATE" THEN (
    UPDATE $after SET updated_at = time::now()
);

-- Update updated_at timestamp on documentation changes
DEFINE EVENT IF NOT EXISTS update_doc_timestamp ON TABLE documentation WHEN $event = "UPDATE" THEN (
    UPDATE $after SET updated_at = time::now()
);

-- Update updated_at timestamp on task changes
DEFINE EVENT IF NOT EXISTS update_task_timestamp ON TABLE task WHEN $event = "UPDATE" THEN (
    UPDATE $after SET updated_at = time::now()
);

-- ============================================================================
-- Indexes for Performance
-- ============================================================================

-- Additional composite indexes for common queries
DEFINE INDEX IF NOT EXISTS idx_symbol_file_type ON TABLE code_symbol COLUMNS file_path, symbol_type;
DEFINE INDEX IF NOT EXISTS idx_task_status_priority ON TABLE task COLUMNS status, priority;
DEFINE INDEX IF NOT EXISTS idx_episode_score_date ON TABLE episode COLUMNS success_score, created_at;

-- Indexes for graph edge tables (improve traversal performance)
DEFINE INDEX IF NOT EXISTS idx_depends_on_in ON TABLE depends_on COLUMNS in;
DEFINE INDEX IF NOT EXISTS idx_depends_on_out ON TABLE depends_on COLUMNS out;
DEFINE INDEX IF NOT EXISTS idx_calls_in ON TABLE calls COLUMNS in;
DEFINE INDEX IF NOT EXISTS idx_calls_out ON TABLE calls COLUMNS out;
DEFINE INDEX IF NOT EXISTS idx_documents_out ON TABLE documents COLUMNS out;

-- Indexes for relationship queries
DEFINE INDEX IF NOT EXISTS idx_task_depends_in ON TABLE task_depends_on COLUMNS in;
DEFINE INDEX IF NOT EXISTS idx_task_depends_out ON TABLE task_depends_on COLUMNS out;

-- ============================================================================
-- Self-Improvement Metrics Tables
-- ============================================================================

-- Self-improvement metrics snapshots
DEFINE TABLE IF NOT EXISTS improvement_metrics SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS timestamp ON TABLE improvement_metrics TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS health_score ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS code_quality_score ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS test_coverage_percent ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS avg_cyclomatic_complexity ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS circular_dependencies_count ON TABLE improvement_metrics TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS untested_symbols_count ON TABLE improvement_metrics TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS undocumented_symbols_count ON TABLE improvement_metrics TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS high_complexity_symbols_count ON TABLE improvement_metrics TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS improvements_per_week ON TABLE improvement_metrics TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS avg_improvement_time_hours ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS technical_debt_score ON TABLE improvement_metrics TYPE float DEFAULT 0.0;
DEFINE FIELD IF NOT EXISTS language_breakdown ON TABLE improvement_metrics TYPE option<object>;
DEFINE FIELD IF NOT EXISTS trend_direction ON TABLE improvement_metrics TYPE string DEFAULT 'Stable';

DEFINE INDEX IF NOT EXISTS idx_improvement_timestamp ON TABLE improvement_metrics COLUMNS timestamp;
DEFINE INDEX IF NOT EXISTS idx_improvement_health ON TABLE improvement_metrics COLUMNS health_score;
