-- Enhanced Task Schema for SurrealDB
-- Supports complete task management with graph relationships

-- ============================================================================
-- Task Table (Enhanced)
-- ============================================================================

DEFINE TABLE IF NOT EXISTS task SCHEMAFULL;

-- Core fields
DEFINE FIELD IF NOT EXISTS id ON TABLE task TYPE string;
DEFINE FIELD IF NOT EXISTS title ON TABLE task TYPE string ASSERT $value != NONE;
DEFINE FIELD IF NOT EXISTS description ON TABLE task TYPE option<string>;

-- Status and priority with validation
DEFINE FIELD IF NOT EXISTS status ON TABLE task TYPE string
    ASSERT $value IN ['pending', 'in_progress', 'blocked', 'done', 'cancelled'];
DEFINE FIELD IF NOT EXISTS priority ON TABLE task TYPE string
    ASSERT $value IN ['low', 'medium', 'high', 'critical']
    DEFAULT 'medium';

-- Timestamps
DEFINE FIELD IF NOT EXISTS created_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE task TYPE datetime VALUE time::now();
DEFINE FIELD IF NOT EXISTS started_at ON TABLE task TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS last_activity ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS completed_at ON TABLE task TYPE option<datetime>;

-- Spec reference (embedded object)
DEFINE FIELD IF NOT EXISTS spec_ref ON TABLE task TYPE option<object>;

-- Session tracking
DEFINE FIELD IF NOT EXISTS session_id ON TABLE task TYPE option<string>;
DEFINE FIELD IF NOT EXISTS active_session_id ON TABLE task TYPE option<string>;
DEFINE FIELD IF NOT EXISTS timeout_hours ON TABLE task TYPE option<int>;

-- Effort tracking
DEFINE FIELD IF NOT EXISTS estimated_hours ON TABLE task TYPE option<float>;
DEFINE FIELD IF NOT EXISTS actual_hours ON TABLE task TYPE option<float>;

-- Tags and metadata
DEFINE FIELD IF NOT EXISTS tags ON TABLE task TYPE array<string> DEFAULT [];

-- History (array of status transitions)
DEFINE FIELD IF NOT EXISTS history ON TABLE task TYPE array<object> DEFAULT [];

-- Git and episode links
DEFINE FIELD IF NOT EXISTS commit_hash ON TABLE task TYPE option<string>;
DEFINE FIELD IF NOT EXISTS episode_id ON TABLE task TYPE option<string>;

-- Schema versioning
DEFINE FIELD IF NOT EXISTS schema_version ON TABLE task TYPE int DEFAULT 2;

-- ============================================================================
-- Indexes for Performance
-- ============================================================================

DEFINE INDEX IF NOT EXISTS idx_task_status ON TABLE task COLUMNS status;
DEFINE INDEX IF NOT EXISTS idx_task_priority ON TABLE task COLUMNS priority;
DEFINE INDEX IF NOT EXISTS idx_task_created ON TABLE task COLUMNS created_at;
DEFINE INDEX IF NOT EXISTS idx_task_updated ON TABLE task COLUMNS updated_at;
DEFINE INDEX IF NOT EXISTS idx_task_tags ON TABLE task COLUMNS tags;

-- ============================================================================
-- Graph Relationships
-- ============================================================================

-- Task dependency relationships (for depends_on)
DEFINE TABLE IF NOT EXISTS task_depends_on SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE task_depends_on TYPE record<task>;
DEFINE FIELD IF NOT EXISTS out ON TABLE task_depends_on TYPE record<task>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE task_depends_on TYPE datetime DEFAULT time::now();

-- Task relationships (for related_tasks)
DEFINE TABLE IF NOT EXISTS task_related_to SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON TABLE task_related_to TYPE record<task>;
DEFINE FIELD IF NOT EXISTS out ON TABLE task_related_to TYPE record<task>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE task_related_to TYPE datetime DEFAULT time::now();

-- ============================================================================
-- Utility Functions
-- ============================================================================

-- Get all dependencies for a task (recursive)
DEFINE FUNCTION IF NOT EXISTS fn::get_task_dependencies($task_id: string, $depth: int) {
    LET $task = SELECT * FROM type::thing('task', $task_id);
    LET $deps = SELECT out.* FROM task_depends_on WHERE in = type::thing('task', $task_id);
    RETURN $deps;
};

-- Get all dependents of a task (who depends on this task)
DEFINE FUNCTION IF NOT EXISTS fn::get_task_dependents($task_id: string) {
    RETURN SELECT in.* FROM task_depends_on WHERE out = type::thing('task', $task_id);
};

-- Check if task has circular dependencies
DEFINE FUNCTION IF NOT EXISTS fn::has_circular_dependency($from_id: string, $to_id: string) {
    -- This is a simplified check - full implementation would traverse the graph
    LET $direct = SELECT * FROM task_depends_on WHERE in = type::thing('task', $from_id) AND out = type::thing('task', $to_id);
    LET $reverse = SELECT * FROM task_depends_on WHERE in = type::thing('task', $to_id) AND out = type::thing('task', $from_id);
    RETURN count($direct) > 0 OR count($reverse) > 0;
};

-- ============================================================================
-- Events for Automatic Updates
-- ============================================================================

-- Update updated_at timestamp on task changes
DEFINE EVENT IF NOT EXISTS update_task_timestamp ON TABLE task WHEN $event = "UPDATE" THEN (
    UPDATE $after SET updated_at = time::now()
);

-- Update last_activity on any task modification
DEFINE EVENT IF NOT EXISTS update_task_activity ON TABLE task WHEN $event = "UPDATE" THEN (
    UPDATE $after SET last_activity = time::now()
);
