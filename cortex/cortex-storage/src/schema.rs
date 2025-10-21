//! Database schema definitions and migrations.

/// SurrealQL schema for the Cortex system
pub const SCHEMA: &str = r#"
-- Define tables
DEFINE TABLE projects SCHEMAFULL;
DEFINE TABLE documents SCHEMAFULL;
DEFINE TABLE chunks SCHEMAFULL;
DEFINE TABLE embeddings SCHEMAFULL;
DEFINE TABLE symbols SCHEMAFULL;
DEFINE TABLE relations SCHEMAFULL;
DEFINE TABLE episodes SCHEMAFULL;

-- Projects table
DEFINE FIELD name ON projects TYPE string;
DEFINE FIELD path ON projects TYPE string;
DEFINE FIELD description ON projects TYPE option<string>;
DEFINE FIELD created_at ON projects TYPE datetime;
DEFINE FIELD updated_at ON projects TYPE datetime;
DEFINE FIELD metadata ON projects TYPE object;

DEFINE INDEX projects_name ON projects FIELDS name UNIQUE;
DEFINE INDEX projects_path ON projects FIELDS path UNIQUE;

-- Documents table
DEFINE FIELD project_id ON documents TYPE record<projects>;
DEFINE FIELD path ON documents TYPE string;
DEFINE FIELD content_hash ON documents TYPE string;
DEFINE FIELD size ON documents TYPE int;
DEFINE FIELD mime_type ON documents TYPE string;
DEFINE FIELD created_at ON documents TYPE datetime;
DEFINE FIELD updated_at ON documents TYPE datetime;
DEFINE FIELD metadata ON documents TYPE object;

DEFINE INDEX documents_project ON documents FIELDS project_id;
DEFINE INDEX documents_hash ON documents FIELDS content_hash;

-- Chunks table
DEFINE FIELD document_id ON chunks TYPE record<documents>;
DEFINE FIELD content ON chunks TYPE string;
DEFINE FIELD start_offset ON chunks TYPE int;
DEFINE FIELD end_offset ON chunks TYPE int;
DEFINE FIELD chunk_index ON chunks TYPE int;
DEFINE FIELD metadata ON chunks TYPE object;

DEFINE INDEX chunks_document ON chunks FIELDS document_id;

-- Embeddings table
DEFINE FIELD entity_id ON embeddings TYPE string;
DEFINE FIELD entity_type ON embeddings TYPE string;
DEFINE FIELD vector ON embeddings TYPE array;
DEFINE FIELD model ON embeddings TYPE string;
DEFINE FIELD created_at ON embeddings TYPE datetime;

DEFINE INDEX embeddings_entity ON embeddings FIELDS entity_id, entity_type;

-- Symbols table
DEFINE FIELD document_id ON symbols TYPE record<documents>;
DEFINE FIELD name ON symbols TYPE string;
DEFINE FIELD kind ON symbols TYPE string;
DEFINE FIELD range ON symbols TYPE object;
DEFINE FIELD signature ON symbols TYPE option<string>;
DEFINE FIELD documentation ON symbols TYPE option<string>;
DEFINE FIELD metadata ON symbols TYPE object;

DEFINE INDEX symbols_document ON symbols FIELDS document_id;
DEFINE INDEX symbols_name ON symbols FIELDS name;

-- Relations table
DEFINE FIELD source_id ON relations TYPE string;
DEFINE FIELD target_id ON relations TYPE string;
DEFINE FIELD relation_type ON relations TYPE string;
DEFINE FIELD weight ON relations TYPE float;
DEFINE FIELD metadata ON relations TYPE object;

DEFINE INDEX relations_source ON relations FIELDS source_id;
DEFINE INDEX relations_target ON relations FIELDS target_id;

-- Episodes table
DEFINE FIELD project_id ON episodes TYPE record<projects>;
DEFINE FIELD session_id ON episodes TYPE option<string>;
DEFINE FIELD content ON episodes TYPE string;
DEFINE FIELD context ON episodes TYPE object;
DEFINE FIELD importance ON episodes TYPE float;
DEFINE FIELD created_at ON episodes TYPE datetime;
DEFINE FIELD accessed_count ON episodes TYPE int;
DEFINE FIELD last_accessed_at ON episodes TYPE option<datetime>;

DEFINE INDEX episodes_project ON episodes FIELDS project_id;
DEFINE INDEX episodes_session ON episodes FIELDS session_id;
DEFINE INDEX episodes_importance ON episodes FIELDS importance;
DEFINE INDEX episodes_created_at ON episodes FIELDS created_at;
DEFINE INDEX episodes_outcome ON episodes FIELDS outcome;
"#;

/// Initialize the database schema
pub async fn init_schema(db: &surrealdb::Surreal<impl surrealdb::Connection>) -> cortex_core::error::Result<()> {
    tracing::info!("Initializing database schema");

    db.query(SCHEMA)
        .await
        .map_err(|e| cortex_core::error::CortexError::database(format!("Failed to initialize schema: {}", e)))?;

    tracing::info!("Database schema initialized successfully");
    Ok(())
}
