//! SurrealDB storage implementation.

use crate::pool::ConnectionPool;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use cortex_core::traits::Storage;
use cortex_core::types::*;
use async_trait::async_trait;
use std::sync::Arc;

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
}

#[async_trait]
impl Storage for SurrealStorage {
    async fn store_project(&self, project: &Project) -> Result<()> {
        let db = self.pool.get().await?;

        // Create a map without the id field to avoid SurrealDB conflicts
        let mut content = serde_json::to_value(project).map_err(|e| {
            CortexError::storage(format!("Failed to serialize project: {}", e))
        })?;

        // Remove the id field from content since we're specifying it in the record locator
        if let Some(obj) = content.as_object_mut() {
            obj.remove("id");
        }

        // Use upsert with the cleaned content
        let _: Option<serde_json::Value> = db
            .upsert(("projects", project.id.to_string()))
            .content(content)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to store project: {}", e)))?;

        Ok(())
    }

    async fn get_project(&self, id: CortexId) -> Result<Option<Project>> {
        let db = self.pool.get().await?;

        let project: Option<Project> = db
            .select(("projects", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to get project: {}", e)))?;

        Ok(project)
    }

    async fn list_projects(&self) -> Result<Vec<Project>> {
        let db = self.pool.get().await?;

        let projects: Vec<Project> = db
            .select("projects")
            .await
            .map_err(|e| CortexError::storage(format!("Failed to list projects: {}", e)))?;

        Ok(projects)
    }

    async fn delete_project(&self, id: CortexId) -> Result<()> {
        let db = self.pool.get().await?;

        let _: Option<Project> = db
            .delete(("projects", id.to_string()))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to delete project: {}", e)))?;

        Ok(())
    }

    async fn store_document(&self, document: &Document) -> Result<()> {
        let db = self.pool.get().await?;

        // Use upsert to avoid ID conflicts
        let _: Option<Document> = db.upsert(("documents", document.id.to_string()))
            .content(document.clone())
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

        let _: Option<Embedding> = db.upsert(("embeddings", embedding.id.to_string()))
            .content(embedding.clone())
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

        let _: Option<Episode> = db.upsert(("episodes", episode.id.to_string()))
            .content(episode.clone())
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

        Ok(SystemStats {
            total_projects: total_projects.unwrap_or(0) as u64,
            total_documents: total_documents.unwrap_or(0) as u64,
            total_chunks: total_chunks.unwrap_or(0) as u64,
            total_embeddings: total_embeddings.unwrap_or(0) as u64,
            total_episodes: total_episodes.unwrap_or(0) as u64,
            storage_size_bytes: 0, // TODO: Implement storage size calculation
            last_updated: chrono::Utc::now(),
        })
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
        let pool = Arc::new(ConnectionPool::new(config));
        pool.initialize().await.unwrap();

        let storage = SurrealStorage::with_schema(pool).await.unwrap();

        let project = Project::new("Test Project".to_string(), PathBuf::from("/test"));

        // Create
        storage.store_project(&project).await.unwrap();

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
