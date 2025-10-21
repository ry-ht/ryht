-- Migration: Code Unit Schema Enhancement
-- Version: 001
-- Description: Comprehensive code unit schema with support for all languages and types
-- Date: 2025-10-21

-- ============================================================================
-- Code Unit Table - Enhanced Schema
-- ============================================================================

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
DEFINE FIELD name ON code_unit TYPE string;
DEFINE FIELD qualified_name ON code_unit TYPE string;
DEFINE FIELD display_name ON code_unit TYPE string;
DEFINE FIELD file_path ON code_unit TYPE string;
DEFINE FIELD language ON code_unit TYPE string
    ASSERT $value IN [
        'rust', 'typescript', 'javascript', 'python', 'go', 'java',
        'cpp', 'c', 'c_sharp', 'ruby', 'php', 'swift', 'kotlin',
        'scala', 'haskell', 'elixir', 'clojure', 'zig', 'unknown'
    ];

-- Location
DEFINE FIELD start_line ON code_unit TYPE int;
DEFINE FIELD end_line ON code_unit TYPE int;
DEFINE FIELD start_column ON code_unit TYPE int;
DEFINE FIELD end_column ON code_unit TYPE int;
DEFINE FIELD start_byte ON code_unit TYPE int;
DEFINE FIELD end_byte ON code_unit TYPE int;

-- Content
DEFINE FIELD signature ON code_unit TYPE string;
DEFINE FIELD body ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD docstring ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD comments ON code_unit TYPE array<string> DEFAULT [];

-- Type Information
DEFINE FIELD return_type ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD parameters ON code_unit TYPE array<object> DEFAULT [];
DEFINE FIELD type_parameters ON code_unit TYPE array<object> DEFAULT [];
DEFINE FIELD generic_constraints ON code_unit TYPE array<string> DEFAULT [];
DEFINE FIELD throws ON code_unit TYPE array<string> DEFAULT [];

-- Metadata
DEFINE FIELD visibility ON code_unit TYPE string DEFAULT 'private'
    ASSERT $value IN ['public', 'private', 'protected', 'internal', 'package'];
DEFINE FIELD attributes ON code_unit TYPE array<object> DEFAULT [];
DEFINE FIELD modifiers ON code_unit TYPE array<string> DEFAULT [];
DEFINE FIELD is_async ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_unsafe ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_const ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_static ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_abstract ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_virtual ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_override ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_final ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_exported ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD is_default_export ON code_unit TYPE bool DEFAULT false;

-- Metrics
DEFINE FIELD complexity ON code_unit TYPE object DEFAULT {
    cyclomatic: 1,
    cognitive: 1,
    nesting: 0,
    lines: 0,
    parameters: 0,
    returns: 0
};
DEFINE FIELD test_coverage ON code_unit TYPE option<float> DEFAULT NONE;
DEFINE FIELD has_tests ON code_unit TYPE bool DEFAULT false;
DEFINE FIELD has_documentation ON code_unit TYPE bool DEFAULT false;

-- Language-specific metadata
DEFINE FIELD language_specific ON code_unit TYPE object DEFAULT {};

-- Embedding for semantic search
DEFINE FIELD embedding ON code_unit TYPE option<array<float>> DEFAULT NONE;
DEFINE FIELD embedding_model ON code_unit TYPE option<string> DEFAULT NONE;

-- Semantic analysis
DEFINE FIELD summary ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD purpose ON code_unit TYPE option<string> DEFAULT NONE;

-- Tree-sitter AST reference
DEFINE FIELD ast_node_type ON code_unit TYPE option<string> DEFAULT NONE;
DEFINE FIELD ast_metadata ON code_unit TYPE option<object> DEFAULT NONE;

-- Versioning and status
DEFINE FIELD status ON code_unit TYPE string DEFAULT 'active'
    ASSERT $value IN ['active', 'deprecated', 'deleted', 'moved'];
DEFINE FIELD version ON code_unit TYPE int DEFAULT 1;
DEFINE FIELD created_at ON code_unit TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON code_unit TYPE datetime DEFAULT time::now();
DEFINE FIELD created_by ON code_unit TYPE string DEFAULT 'system';
DEFINE FIELD updated_by ON code_unit TYPE string DEFAULT 'system';

-- Additional metadata
DEFINE FIELD tags ON code_unit TYPE array<string> DEFAULT [];
DEFINE FIELD metadata ON code_unit TYPE object DEFAULT {};

-- ============================================================================
-- Indexes for Performance
-- ============================================================================

-- Primary lookup indexes
DEFINE INDEX code_unit_id_idx ON code_unit FIELDS id UNIQUE;
DEFINE INDEX code_unit_qualified_name_idx ON code_unit FIELDS qualified_name;
DEFINE INDEX code_unit_name_idx ON code_unit FIELDS name;

-- File and location indexes
DEFINE INDEX code_unit_file_idx ON code_unit FIELDS file_path;
DEFINE INDEX code_unit_file_line_idx ON code_unit FIELDS file_path, start_line;

-- Type and language indexes
DEFINE INDEX code_unit_type_idx ON code_unit FIELDS unit_type;
DEFINE INDEX code_unit_language_idx ON code_unit FIELDS language;
DEFINE INDEX code_unit_type_language_idx ON code_unit FIELDS unit_type, language;

-- Status and visibility indexes
DEFINE INDEX code_unit_status_idx ON code_unit FIELDS status;
DEFINE INDEX code_unit_visibility_idx ON code_unit FIELDS visibility;

-- Quality metrics indexes
DEFINE INDEX code_unit_has_tests_idx ON code_unit FIELDS has_tests;
DEFINE INDEX code_unit_has_docs_idx ON code_unit FIELDS has_documentation;
DEFINE INDEX code_unit_complexity_idx ON code_unit FIELDS complexity.cyclomatic;

-- Semantic search index (vector index for embeddings)
DEFINE INDEX code_unit_embedding_idx ON code_unit FIELDS embedding MTREE DIMENSION 1536;

-- Tag search
DEFINE INDEX code_unit_tags_idx ON code_unit FIELDS tags;

-- ============================================================================
-- Parameter and Type Parameter Schemas
-- ============================================================================

-- Parameter object structure:
-- {
--   name: string,
--   param_type: option<string>,
--   default_value: option<string>,
--   is_optional: bool,
--   is_variadic: bool,
--   attributes: array<object>
-- }

-- Type parameter object structure:
-- {
--   name: string,
--   bounds: array<string>,
--   default_type: option<string>,
--   variance: option<string>  -- 'covariant' | 'contravariant' | 'invariant'
-- }

-- Attribute object structure:
-- {
--   name: string,
--   arguments: array<string>,
--   metadata: object
-- }

-- ============================================================================
-- Migration from Old Schema
-- ============================================================================

-- Update existing records to new schema (if any exist)
-- This handles backward compatibility for any existing semantic_unit records

UPDATE code_unit SET
    start_byte = 0,
    end_byte = 0,
    is_abstract = false,
    is_virtual = false,
    is_override = false,
    is_final = false,
    is_exported = false,
    is_default_export = false,
    language_specific = {},
    ast_node_type = NONE,
    ast_metadata = NONE,
    created_by = 'system',
    updated_by = 'system',
    tags = [],
    metadata = {}
WHERE start_byte IS NONE;

-- ============================================================================
-- Views and Computed Fields
-- ============================================================================

-- View for public API surface
DEFINE TABLE public_api AS
    SELECT
        id,
        unit_type,
        name,
        qualified_name,
        file_path,
        language,
        signature,
        docstring,
        return_type,
        parameters,
        visibility,
        has_documentation,
        has_tests,
        complexity
    FROM code_unit
    WHERE visibility = 'public' AND status = 'active';

-- View for untested units
DEFINE TABLE untested_units AS
    SELECT
        id,
        name,
        qualified_name,
        file_path,
        unit_type,
        visibility,
        complexity
    FROM code_unit
    WHERE has_tests = false
      AND unit_type IN ['function', 'method', 'async_function']
      AND visibility = 'public'
      AND status = 'active';

-- View for undocumented units
DEFINE TABLE undocumented_units AS
    SELECT
        id,
        name,
        qualified_name,
        file_path,
        unit_type,
        visibility,
        complexity
    FROM code_unit
    WHERE has_documentation = false
      AND visibility = 'public'
      AND status = 'active';

-- View for complex units needing refactoring
DEFINE TABLE complex_units AS
    SELECT
        id,
        name,
        qualified_name,
        file_path,
        unit_type,
        complexity,
        has_tests
    FROM code_unit
    WHERE complexity.cyclomatic > 10
      AND status = 'active'
    ORDER BY complexity.cyclomatic DESC;

-- ============================================================================
-- Validation Functions
-- ============================================================================

-- Function to validate complexity metrics
DEFINE FUNCTION fn::validate_complexity($complexity) {
    RETURN $complexity.cyclomatic >= 1
        AND $complexity.cognitive >= 0
        AND $complexity.nesting >= 0
        AND $complexity.lines >= 0;
};

-- Function to check if unit needs tests
DEFINE FUNCTION fn::needs_tests($unit) {
    RETURN $unit.unit_type IN ['function', 'method', 'async_function']
        AND $unit.visibility = 'public'
        AND $unit.has_tests = false
        AND $unit.status = 'active';
};

-- Function to check if unit needs documentation
DEFINE FUNCTION fn::needs_documentation($unit) {
    RETURN $unit.visibility = 'public'
        AND $unit.has_documentation = false
        AND $unit.status = 'active';
};

-- Function to calculate complexity score (0.0 - 1.0)
DEFINE FUNCTION fn::complexity_score($complexity) {
    LET $cyclo_score = math::min(1.0, $complexity.cyclomatic / 50.0);
    LET $cognitive_score = math::min(1.0, $complexity.cognitive / 100.0);
    LET $nesting_score = math::min(1.0, $complexity.nesting / 10.0);
    LET $lines_score = math::min(1.0, $complexity.lines / 500.0);

    RETURN ($cyclo_score * 0.3 + $cognitive_score * 0.4 + $nesting_score * 0.2 + $lines_score * 0.1);
};

-- ============================================================================
-- Query Examples
-- ============================================================================

-- Find all functions in a file
-- SELECT * FROM code_unit
-- WHERE file_path = '/path/to/file.rs'
--   AND unit_type IN ['function', 'method', 'async_function']
-- ORDER BY start_line;

-- Find all public API without documentation
-- SELECT * FROM undocumented_units;

-- Find all complex units
-- SELECT * FROM complex_units
-- LIMIT 20;

-- Semantic search for similar code
-- SELECT *, vector::distance::cosine(embedding, $query_embedding) AS similarity
-- FROM code_unit
-- WHERE embedding IS NOT NONE
--   AND vector::distance::cosine(embedding, $query_embedding) <= 0.3
-- ORDER BY similarity ASC
-- LIMIT 10;

-- Find all tests for a specific unit
-- SELECT * FROM code_unit
-- WHERE qualified_name CONTAINS 'test_'
--   AND unit_type = 'test';

-- Language-specific queries
-- SELECT * FROM code_unit
-- WHERE language = 'rust'
--   AND unit_type = 'trait';

-- Find generic functions
-- SELECT * FROM code_unit
-- WHERE array::len(type_parameters) > 0
--   AND unit_type IN ['function', 'method'];
