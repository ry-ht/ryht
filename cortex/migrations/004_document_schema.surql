-- ============================================================================
-- Document System Schema
-- ============================================================================
-- This migration creates tables for the documentation system including:
-- - Documents with hierarchical structure
-- - Document sections for internal structure
-- - Document links for relationships
-- - Document versions for history tracking
-- - Document search index for full-text search

-- ============================================================================
-- Document Table
-- ============================================================================
-- Main table for documentation documents

DEFINE TABLE document SCHEMAFULL;

-- Core identification
DEFINE FIELD id ON document TYPE string;
DEFINE FIELD title ON document TYPE string;
DEFINE FIELD slug ON document TYPE string;
DEFINE FIELD doc_type ON document TYPE string;
DEFINE FIELD status ON document TYPE string;

-- Content
DEFINE FIELD description ON document TYPE option<string>;
DEFINE FIELD content ON document TYPE string;
DEFINE FIELD content_hash ON document TYPE string;

-- Hierarchy
DEFINE FIELD parent_id ON document TYPE option<string>;
DEFINE FIELD order ON document TYPE int DEFAULT 0;

-- Metadata
DEFINE FIELD tags ON document TYPE array DEFAULT [];
DEFINE FIELD keywords ON document TYPE array DEFAULT [];
DEFINE FIELD author ON document TYPE string DEFAULT "system";
DEFINE FIELD contributors ON document TYPE array DEFAULT [];
DEFINE FIELD version ON document TYPE string DEFAULT "1.0.0";
DEFINE FIELD language ON document TYPE string DEFAULT "en";

-- Source and workspace
DEFINE FIELD source_path ON document TYPE option<string>;
DEFINE FIELD workspace_id ON document TYPE option<string>;
DEFINE FIELD metadata ON document TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON document TYPE datetime;
DEFINE FIELD updated_at ON document TYPE datetime;
DEFINE FIELD published_at ON document TYPE option<datetime>;

-- Indexes
DEFINE INDEX document_slug ON document FIELDS slug UNIQUE;
DEFINE INDEX document_status ON document FIELDS status;
DEFINE INDEX document_type ON document FIELDS doc_type;
DEFINE INDEX document_parent ON document FIELDS parent_id;
DEFINE INDEX document_workspace ON document FIELDS workspace_id;
DEFINE INDEX document_created ON document FIELDS created_at;
DEFINE INDEX document_published ON document FIELDS published_at;

-- ============================================================================
-- Document Section Table
-- ============================================================================
-- Sections within documents for hierarchical content structure

DEFINE TABLE document_section SCHEMAFULL;

-- Core identification
DEFINE FIELD id ON document_section TYPE string;
DEFINE FIELD document_id ON document_section TYPE string;
DEFINE FIELD section_id ON document_section TYPE string;
DEFINE FIELD title ON document_section TYPE string;

-- Content
DEFINE FIELD content ON document_section TYPE string;
DEFINE FIELD level ON document_section TYPE int DEFAULT 1;
DEFINE FIELD parent_section_id ON document_section TYPE option<string>;
DEFINE FIELD order ON document_section TYPE int DEFAULT 0;
DEFINE FIELD anchor ON document_section TYPE string;

-- Metadata
DEFINE FIELD metadata ON document_section TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON document_section TYPE datetime;
DEFINE FIELD updated_at ON document_section TYPE datetime;

-- Indexes
DEFINE INDEX section_document ON document_section FIELDS document_id;
DEFINE INDEX section_document_section ON document_section FIELDS document_id, section_id;
DEFINE INDEX section_parent ON document_section FIELDS parent_section_id;
DEFINE INDEX section_level_order ON document_section FIELDS document_id, level, order;

-- ============================================================================
-- Document Link Table
-- ============================================================================
-- Links between documents and to external resources

DEFINE TABLE document_link SCHEMAFULL;

-- Core identification
DEFINE FIELD id ON document_link TYPE string;
DEFINE FIELD source_document_id ON document_link TYPE string;
DEFINE FIELD source_section_id ON document_link TYPE option<string>;

-- Link properties
DEFINE FIELD link_type ON document_link TYPE string;
DEFINE FIELD target ON document_link TYPE object;
DEFINE FIELD description ON document_link TYPE option<string>;
DEFINE FIELD weight ON document_link TYPE float DEFAULT 1.0;

-- Metadata
DEFINE FIELD metadata ON document_link TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON document_link TYPE datetime;

-- Indexes
DEFINE INDEX link_source ON document_link FIELDS source_document_id;
DEFINE INDEX link_source_section ON document_link FIELDS source_document_id, source_section_id;
DEFINE INDEX link_type ON document_link FIELDS link_type;

-- ============================================================================
-- Document Version Table
-- ============================================================================
-- Version history for documents

DEFINE TABLE document_version SCHEMAFULL;

-- Core identification
DEFINE FIELD id ON document_version TYPE string;
DEFINE FIELD document_id ON document_version TYPE string;
DEFINE FIELD version ON document_version TYPE string;

-- Content at this version
DEFINE FIELD content ON document_version TYPE string;
DEFINE FIELD content_hash ON document_version TYPE string;

-- Version metadata
DEFINE FIELD author ON document_version TYPE string;
DEFINE FIELD message ON document_version TYPE string;
DEFINE FIELD parent_version_id ON document_version TYPE option<string>;
DEFINE FIELD tags ON document_version TYPE array DEFAULT [];
DEFINE FIELD metadata ON document_version TYPE object DEFAULT {};

-- Timestamps
DEFINE FIELD created_at ON document_version TYPE datetime;

-- Indexes
DEFINE INDEX version_document ON document_version FIELDS document_id;
DEFINE INDEX version_document_version ON document_version FIELDS document_id, version;
DEFINE INDEX version_created ON document_version FIELDS created_at;
DEFINE INDEX version_hash ON document_version FIELDS content_hash;

-- ============================================================================
-- Document Search Index Table
-- ============================================================================
-- Full-text search index for documents

DEFINE TABLE document_search_index SCHEMAFULL;

-- Core identification
DEFINE FIELD document_id ON document_search_index TYPE string;
DEFINE FIELD section_id ON document_search_index TYPE option<string>;

-- Searchable content
DEFINE FIELD title ON document_search_index TYPE string;
DEFINE FIELD content ON document_search_index TYPE string;
DEFINE FIELD keywords ON document_search_index TYPE array DEFAULT [];
DEFINE FIELD tags ON document_search_index TYPE array DEFAULT [];

-- Document metadata for filtering
DEFINE FIELD doc_type ON document_search_index TYPE string;
DEFINE FIELD status ON document_search_index TYPE string;
DEFINE FIELD language ON document_search_index TYPE string;
DEFINE FIELD workspace_id ON document_search_index TYPE option<string>;

-- Timestamps
DEFINE FIELD indexed_at ON document_search_index TYPE datetime;
DEFINE FIELD updated_at ON document_search_index TYPE datetime;

-- Indexes for search
DEFINE INDEX search_document ON document_search_index FIELDS document_id;
DEFINE INDEX search_type ON document_search_index FIELDS doc_type;
DEFINE INDEX search_status ON document_search_index FIELDS status;
DEFINE INDEX search_language ON document_search_index FIELDS language;
DEFINE INDEX search_workspace ON document_search_index FIELDS workspace_id;
DEFINE INDEX search_updated ON document_search_index FIELDS updated_at;

-- Full-text search indexes (if supported by SurrealDB)
-- Note: These may need to be adjusted based on SurrealDB version and capabilities
DEFINE INDEX search_title_fulltext ON document_search_index FIELDS title SEARCH ANALYZER simple BM25;
DEFINE INDEX search_content_fulltext ON document_search_index FIELDS content SEARCH ANALYZER simple BM25;

-- ============================================================================
-- Additional Relations and Constraints
-- ============================================================================

-- Ensure document slugs are URL-safe and lowercase
-- This should be enforced at application level, but noted here for reference

-- Cascade delete rules (to be implemented at application level):
-- - When a document is deleted, delete all its sections
-- - When a document is deleted, delete all its links
-- - When a document is deleted, optionally archive its versions
-- - When a section is deleted, update parent_section_id for child sections
