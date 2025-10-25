//! SurrealDB storage implementation.

use crate::pool::ConnectionPool;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::traits::Storage;
use cortex_core::types::*;
use async_trait::async_trait;
use std::sync::Arc;
use surrealdb::sql::Datetime;

/// Storage implementation using SurrealDB
pub struct SurrealStorage {
    pool: Arc<ConnectionPool>,
}

impl SurrealStorage {
    /// Create a new SurrealDB storage instance
    pub fn new(pool: Arc<ConnectionPool>) -> Self {
        Self { pool }
    }

    /// Create a new storage instance and initialize the schema
    pub async fn with_schema(pool: Arc<ConnectionPool>) -> Result<Self> {
        let conn = pool.get().await?;
        crate::schema::init_schema(&*conn).await?;
        Ok(Self::new(pool))
    }

    // ============================================================================
    // Dual-Storage Coordination Methods
    // ============================================================================

    /// Update vector sync status for an entity
    pub async fn update_vector_sync_status(
        &self,
        id: CortexId,
        table: &str,
        synced: bool,
    ) -> Result<()> {
        let db = self.pool.get().await?;
        let query = format!(
            "UPDATE {}:{} SET vector_synced = $synced, last_synced_at = time::now()",
            table, id
        );

        db.query(query)
            .bind(("synced", synced))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to update sync status: {}", e)))?;

        Ok(())
    }

    /// Mark entity as having a vector embedding
    pub async fn mark_entity_with_vector(
        &self,
        id: CortexId,
        table: &str,
        vector_id: CortexId,
    ) -> Result<()> {
        let db = self.pool.get().await?;
        let query = format!(
            "UPDATE {}:{} SET has_vector = true, vector_id = $vector_id, vector_synced = true, last_synced_at = time::now()",
            table, id
        );

        db.query(query)
            .bind(("vector_id", vector_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to mark entity with vector: {}", e)))?;

        Ok(())
    }

    /// Get entities that need vector sync
    pub async fn get_unsynced_entities(&self, table: &str, limit: usize) -> Result<Vec<CortexId>> {
        let db = self.pool.get().await?;
        let query = format!(
            "SELECT meta::id(id) AS id_str FROM {} WHERE vector_synced = false OR vector_synced IS NONE LIMIT {}",
            table, limit
        );

        let mut response = db.query(&query).await
            .map_err(|e| CortexError::storage(format!("Failed to get unsynced entities: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct IdRecord {
            id_str: String,
        }

        let records: Vec<IdRecord> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to parse IDs: {}", e)))?;

        Ok(records.into_iter()
            .filter_map(|r| CortexId::parse(&r.id_str).ok())
            .collect::<Vec<_>>())
    }

    /// Get vector_id reference for an entity
    pub async fn get_vector_id(&self, id: CortexId, table: &str) -> Result<Option<String>> {
        let db = self.pool.get().await?;
        let query = format!("SELECT vector_id FROM {}:{}", table, id);

        let mut response = db.query(&query).await
            .map_err(|e| CortexError::storage(format!("Failed to get vector_id: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct VectorIdRecord {
            vector_id: Option<String>,
        }

        let records: Vec<VectorIdRecord> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to parse vector_id: {}", e)))?;

        Ok(records.first().and_then(|r| r.vector_id.clone()))
    }

    /// Check if entity has synced vector
    pub async fn has_synced_vector(&self, id: CortexId, table: &str) -> Result<bool> {
        let db = self.pool.get().await?;
        let query = format!("SELECT vector_synced FROM {}:{}", table, id);

        let mut response = db.query(&query).await
            .map_err(|e| CortexError::storage(format!("Failed to check sync status: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct SyncRecord {
            vector_synced: Option<bool>,
        }

        let records: Vec<SyncRecord> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to parse sync status: {}", e)))?;

        Ok(records.first().and_then(|r| r.vector_synced).unwrap_or(false))
    }
}

#[async_trait]
impl Storage for SurrealStorage {
    async fn store_project(&self, project: &Project) -> Result<()> {
        let db = self.pool.get().await?;

        // Clone the data to avoid lifetime issues
        let id = project.id.to_string();
        let name = project.name.clone();
        let path = project.path.to_string_lossy().to_string();
        let description = project.description.clone();
        let created_at = project.created_at.to_rfc3339();
        let updated_at = project.updated_at.to_rfc3339();
        let metadata = project.metadata.clone();
        // Convert serde_json::Value to string for SurrealDB
        let _metadata_json = serde_json::to_string(&metadata).unwrap_or_default();

        // Use INSERT ... ON DUPLICATE KEY UPDATE pattern to upsert
        let query = format!(r#"
            CREATE projects:⟨{}⟩ CONTENT {{
                name: $name,
                path: $path,
                description: $description,
                created_at: <datetime> $created_at,
                updated_at: <datetime> $updated_at,
                metadata: $metadata
            }}
        "#, id);

        db.query(query)
            .bind(("name", name))
            .bind(("path", path))
            .bind(("description", description))
            .bind(("created_at", created_at))
            .bind(("updated_at", updated_at))
            .bind(("metadata", metadata))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store project: {}", e)))?;

        Ok(())
    }

    async fn get_project(&self, id: CortexId) -> Result<Option<Project>> {
        let db = self.pool.get().await?;

        // Use a query to retrieve with proper field conversion
        let id_str = id.to_string();
        let query = format!("SELECT name, path, description, created_at, updated_at, metadata FROM projects:⟨{}⟩", id_str);

        let mut response = db
            .query(query)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get project: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ProjectRow {
            name: String,
            path: String,
            description: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            metadata: std::collections::HashMap<String, String>,
        }

        let rows: Vec<ProjectRow> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to extract project: {}", e)))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(Project {
                id,
                name: row.name,
                path: std::path::PathBuf::from(row.path),
                description: row.description,
                created_at: row.created_at,
                updated_at: row.updated_at,
                metadata: row.metadata,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let db = self.pool.get().await?;

        // Use a query to retrieve all projects
        let query = "SELECT name, path, description, created_at, updated_at, metadata, meta::id(id) AS id_str FROM projects";

        let mut response = db
            .query(query)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to list projects: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ProjectRow {
            name: String,
            path: String,
            description: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            metadata: std::collections::HashMap<String, String>,
            id_str: String,
        }

        let rows: Vec<ProjectRow> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to extract projects: {}", e)))?;

        let mut projects = Vec::new();
        for row in rows {
            let id = CortexId::parse(&row.id_str)
                .map_err(|e| CortexError::storage(format!("Failed to parse ID: {}", e)))?;

            projects.push(Project {
                id,
                name: row.name,
                path: std::path::PathBuf::from(row.path),
                description: row.description,
                created_at: row.created_at,
                updated_at: row.updated_at,
                metadata: row.metadata,
            });
        }

        Ok(projects)
    }

    async fn delete_project(&self, id: CortexId) -> Result<()> {
        let db = self.pool.get().await?;

        let id_str = id.to_string();
        let query = format!("DELETE projects:⟨{}⟩", id_str);

        db.query(query)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to delete project: {}", e)))?;

        Ok(())
    }

    async fn store_document(&self, document: &Document) -> Result<()> {
        let db = self.pool.get().await?;

        // Construct the content map manually to ensure proper datetime serialization
        // Added vector_id for dual-storage coordination with Qdrant
        let content = serde_json::json!({
            "project_id": format!("projects:{}", document.project_id),
            "path": document.path,
            "content_hash": document.content_hash,
            "size": document.size,
            "mime_type": document.mime_type,
            "created_at": Datetime::from(document.created_at),
            "updated_at": Datetime::from(document.updated_at),
            "metadata": document.metadata,
            "vector_id": document.id.to_string(), // Reference to Qdrant vector ID (if embedded)
            "has_vector": false, // Flag indicating if this document has a vector embedding
            "vector_synced": false, // Sync status with Qdrant
        });

        // Use upsert to avoid ID conflicts
        let _: Option<serde_json::Value> = db.upsert(("documents", document.id.to_string()))
            .content(content)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store document: {}", e)))?;

        Ok(())
    }

    async fn get_document(&self, id: CortexId) -> Result<Option<Document>> {
        let db = self.pool.get().await?;

        let document: Option<Document> = db
            .select(("documents", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get document: {}", e)))?;

        Ok(document)
    }

    async fn list_documents(&self, project_id: CortexId) -> Result<Vec<Document>> {
        let db = self.pool.get().await?;

        let mut result = db
            .query("SELECT * FROM documents WHERE project_id = $project_id")
            .bind(("project_id", format!("projects:{}", project_id)))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to list documents: {}", e)))?;

        let documents: Vec<Document> = result.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to parse documents: {}", e)))?;

        Ok(documents)
    }

    async fn delete_document(&self, id: CortexId) -> Result<()> {
        let db = self.pool.get().await?;

        let _: Option<Document> = db
            .delete(("documents", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to delete document: {}", e)))?;

        Ok(())
    }

    async fn store_embedding(&self, embedding: &Embedding) -> Result<()> {
        let db = self.pool.get().await?;

        // Construct the content map manually to ensure proper datetime serialization
        // Added vector_id for dual-storage coordination with Qdrant
        let content = serde_json::json!({
            "entity_id": embedding.entity_id.to_string(),
            "entity_type": embedding.entity_type,
            "vector": embedding.vector,
            "model": embedding.model,
            "created_at": Datetime::from(embedding.created_at),
            "vector_id": embedding.id.to_string(), // Reference to Qdrant vector ID
            "vector_synced": true, // Flag indicating sync status with Qdrant
            "last_synced_at": Datetime::from(chrono::Utc::now()),
        });

        let _: Option<serde_json::Value> = db.upsert(("embeddings", embedding.id.to_string()))
            .content(content)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store embedding: {}", e)))?;

        Ok(())
    }

    async fn get_embeddings(&self, entity_id: CortexId) -> Result<Vec<Embedding>> {
        let db = self.pool.get().await?;

        let mut result = db
            .query("SELECT * FROM embeddings WHERE entity_id = $entity_id")
            .bind(("entity_id", entity_id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get embeddings: {}", e)))?;

        let embeddings: Vec<Embedding> = result.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to parse embeddings: {}", e)))?;

        Ok(embeddings)
    }

    async fn store_episode(&self, episode: &Episode) -> Result<()> {
        let db = self.pool.get().await?;

        // Construct the content map manually to ensure proper datetime serialization
        // Added vector_id for dual-storage coordination with Qdrant
        let content = serde_json::json!({
            "project_id": format!("projects:{}", episode.project_id),
            "session_id": episode.session_id,
            "content": episode.content,
            "context": episode.context,
            "importance": episode.importance,
            "created_at": Datetime::from(episode.created_at),
            "accessed_count": episode.accessed_count,
            "last_accessed_at": episode.last_accessed_at.map(Datetime::from),
            "vector_id": episode.id.to_string(), // Reference to Qdrant vector ID
            "has_vector": false, // Flag indicating if this episode has a vector embedding
            "vector_synced": false, // Sync status with Qdrant
        });

        let _: Option<serde_json::Value> = db.upsert(("episodes", episode.id.to_string()))
            .content(content)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store episode: {}", e)))?;

        Ok(())
    }

    async fn get_episode(&self, id: CortexId) -> Result<Option<Episode>> {
        let db = self.pool.get().await?;

        let episode: Option<Episode> = db
            .select(("episodes", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get episode: {}", e)))?;

        Ok(episode)
    }

    async fn get_stats(&self) -> Result<SystemStats> {
        let db = self.pool.get().await?;

        // Count projects
        let mut result = db.query("SELECT count() FROM projects GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to get stats: {}", e)))?;
        let total_projects: Option<i64> = result.take("count").ok().flatten().unwrap_or(Some(0));

        // Count documents
        let mut result = db.query("SELECT count() FROM documents GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to get stats: {}", e)))?;
        let total_documents: Option<i64> = result.take("count").ok().flatten().unwrap_or(Some(0));

        // Count chunks
        let mut result = db.query("SELECT count() FROM chunks GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to get stats: {}", e)))?;
        let total_chunks: Option<i64> = result.take("count").ok().flatten().unwrap_or(Some(0));

        // Count embeddings
        let mut result = db.query("SELECT count() FROM embeddings GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to get stats: {}", e)))?;
        let total_embeddings: Option<i64> = result.take("count").ok().flatten().unwrap_or(Some(0));

        // Count episodes
        let mut result = db.query("SELECT count() FROM episodes GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to get stats: {}", e)))?;
        let total_episodes: Option<i64> = result.take("count").ok().flatten().unwrap_or(Some(0));

        // Calculate approximate storage size by summing document sizes
        let mut result = db.query("SELECT math::sum(size) AS total_size FROM documents GROUP ALL").await
            .map_err(|e| CortexError::storage(format!("Failed to calculate storage size: {}", e)))?;
        let storage_size: Option<i64> = result.take("total_size").ok().flatten().unwrap_or(Some(0));

        Ok(SystemStats {
            total_projects: total_projects.unwrap_or(0) as u64,
            total_documents: total_documents.unwrap_or(0) as u64,
            total_chunks: total_chunks.unwrap_or(0) as u64,
            total_embeddings: total_embeddings.unwrap_or(0) as u64,
            total_episodes: total_episodes.unwrap_or(0) as u64,
            storage_size_bytes: storage_size.unwrap_or(0) as u64,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn create_agent_session(
        &self,
        session_id: String,
        name: String,
        agent_type: String,
    ) -> Result<AgentSession> {
        let db = self.pool.get().await?;

        let session = AgentSession::new(session_id.clone(), name, agent_type);

        // Construct the content map manually to ensure proper datetime serialization
        let content = serde_json::json!({
            "id": session.id,
            "name": session.name,
            "agent_type": session.agent_type,
            "created_at": Datetime::from(session.created_at),
            "last_active": Datetime::from(session.last_active),
            "metadata": session.metadata,
        });

        let _: Option<serde_json::Value> = db.upsert(("agent_sessions", session_id))
            .content(content)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to create agent session: {}", e)))?;

        Ok(session)
    }

    async fn delete_agent_session(&self, session_id: &str) -> Result<()> {
        let db = self.pool.get().await?;

        let _: Option<serde_json::Value> = db
            .delete(("agent_sessions", session_id))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to delete agent session: {}", e)))?;

        Ok(())
    }

    async fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSession>> {
        let db = self.pool.get().await?;

        let query = format!("SELECT id, name, agent_type, created_at, last_active, metadata FROM agent_sessions:⟨{}⟩", session_id);

        let mut response = db
            .query(query)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get agent session: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct AgentSessionRow {
            id: String,
            name: String,
            agent_type: String,
            created_at: chrono::DateTime<chrono::Utc>,
            last_active: chrono::DateTime<chrono::Utc>,
            metadata: std::collections::HashMap<String, String>,
        }

        let rows: Vec<AgentSessionRow> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to extract agent session: {}", e)))?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(AgentSession {
                id: row.id,
                name: row.name,
                agent_type: row.agent_type,
                created_at: row.created_at,
                last_active: row.last_active,
                metadata: row.metadata,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_agent_sessions(&self) -> Result<Vec<AgentSession>> {
        let db = self.pool.get().await?;

        let query = "SELECT id, name, agent_type, created_at, last_active, metadata FROM agent_sessions ORDER BY created_at DESC";

        let mut response = db
            .query(query)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to list agent sessions: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct AgentSessionRow {
            id: String,
            name: String,
            agent_type: String,
            created_at: chrono::DateTime<chrono::Utc>,
            last_active: chrono::DateTime<chrono::Utc>,
            metadata: std::collections::HashMap<String, String>,
        }

        let rows: Vec<AgentSessionRow> = response.take(0)
            .map_err(|e| CortexError::storage(format!("Failed to extract agent sessions: {}", e)))?;

        let sessions = rows.into_iter().map(|row| AgentSession {
            id: row.id,
            name: row.name,
            agent_type: row.agent_type,
            created_at: row.created_at,
            last_active: row.last_active,
            metadata: row.metadata,
        }).collect();

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionConfig;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_project_crud() {
        let config = ConnectionConfig::memory();
        let pool = Arc::new(ConnectionPool::new(config).expect("Failed to create pool"));
        pool.initialize().await.unwrap();

        let storage = SurrealStorage::with_schema(pool).await.unwrap();

        let project = Project::new("Test Project".to_string(), PathBuf::from("/test"));

        // Create
        storage.store_project(&project).await.unwrap();

        // List first to check if it was stored
        let all_projects = storage.list_projects().await.unwrap();
        eprintln!("All projects: {:?}", all_projects);

        // Read
        let retrieved = storage.get_project(project.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Project");

        // List
        let projects = storage.list_projects().await.unwrap();
        assert_eq!(projects.len(), 1);

        // Delete
        storage.delete_project(project.id).await.unwrap();
        let deleted = storage.get_project(project.id).await.unwrap();
        assert!(deleted.is_none());
    }
}
