//! Document service layer
//!
//! Provides unified document management operations for the documentation system.

use anyhow::Result;
use cortex_core::{
    CortexId, Document, DocumentSection, DocumentLink, DocumentVersion,
    DocumentType, DocumentStatus, LinkType, LinkTarget,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Document service for managing documentation
#[derive(Clone)]
pub struct DocumentService {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
}

impl DocumentService {
    /// Create a new document service
    pub fn new(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Self {
        Self { storage, vfs }
    }

    // ============================================================================
    // Document Operations
    // ============================================================================

    /// Create a new document
    pub async fn create_document(&self, request: CreateDocumentRequest) -> Result<Document> {
        info!("Creating document: {}", request.title);

        let mut document = Document::new(request.title, request.content);

        // Apply optional fields
        if let Some(doc_type) = request.doc_type {
            document.doc_type = doc_type;
        }
        if let Some(description) = request.description {
            document.description = Some(description);
        }
        if let Some(parent_id) = request.parent_id {
            document.parent_id = Some(parent_id);
        }
        if let Some(tags) = request.tags {
            document.tags = tags;
        }
        if let Some(keywords) = request.keywords {
            document.keywords = keywords;
        }
        if let Some(author) = request.author {
            document.author = author;
        }
        if let Some(language) = request.language {
            document.language = language;
        }
        if let Some(workspace_id) = request.workspace_id {
            document.workspace_id = Some(workspace_id);
        }
        if let Some(metadata) = request.metadata {
            document.metadata = metadata;
        }

        // Save to database
        let conn = self.storage.acquire().await?;
        let document_json = serde_json::to_value(&document)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("document", document.id.to_string()))
            .content(document_json)
            .await?;

        info!("Created document: {} ({})", document.title, document.id);

        Ok(document)
    }

    /// Get document by ID
    pub async fn get_document(&self, document_id: &CortexId) -> Result<Option<Document>> {
        debug!("Getting document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let document: Option<Document> = conn
            .connection()
            .select(("document", document_id.to_string()))
            .await?;

        Ok(document)
    }

    /// Get document by slug
    pub async fn get_document_by_slug(&self, slug: &str) -> Result<Option<Document>> {
        debug!("Getting document by slug: {}", slug);

        let conn = self.storage.acquire().await?;

        let query = format!("SELECT * FROM document WHERE slug = '{}' LIMIT 1", slug);
        let mut response = conn.connection().query(&query).await?;
        let documents: Vec<Document> = response.take(0)?;

        Ok(documents.into_iter().next())
    }

    /// Update document
    pub async fn update_document(
        &self,
        document_id: &CortexId,
        request: UpdateDocumentRequest,
    ) -> Result<Document> {
        debug!("Updating document: {}", document_id);

        let conn = self.storage.acquire().await?;

        // Get existing document
        let document: Option<Document> = conn
            .connection()
            .select(("document", document_id.to_string()))
            .await?;

        let mut document = document
            .ok_or_else(|| anyhow::anyhow!("Document {} not found", document_id))?;

        // Update fields
        if let Some(title) = request.title {
            document.title = title;
        }
        if let Some(content) = request.content {
            document.update_content(content);
        }
        if let Some(description) = request.description {
            document.description = Some(description);
        }
        if let Some(doc_type) = request.doc_type {
            document.doc_type = doc_type;
        }
        if let Some(tags) = request.tags {
            document.tags = tags;
        }
        if let Some(keywords) = request.keywords {
            document.keywords = keywords;
        }
        if let Some(metadata) = request.metadata {
            document.metadata = metadata;
        }

        // Save updated document
        let document_json = serde_json::to_value(&document)?;
        let _: Option<serde_json::Value> = conn
            .connection()
            .update(("document", document_id.to_string()))
            .content(document_json)
            .await?;

        info!("Updated document: {}", document_id);

        Ok(document)
    }

    /// Delete document
    pub async fn delete_document(&self, document_id: &CortexId) -> Result<()> {
        info!("Deleting document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let _: Option<Document> = conn
            .connection()
            .delete(("document", document_id.to_string()))
            .await?;

        info!("Deleted document: {}", document_id);

        Ok(())
    }

    /// List documents with filters
    pub async fn list_documents(&self, filters: ListDocumentFilters) -> Result<Vec<Document>> {
        debug!("Listing documents with filters: {:?}", filters);

        let conn = self.storage.acquire().await?;

        let mut query = String::from("SELECT * FROM document WHERE 1=1");

        if let Some(status) = filters.status {
            query.push_str(&format!(" AND status = '{:?}'", status));
        }
        if let Some(doc_type) = filters.doc_type {
            query.push_str(&format!(" AND doc_type = '{:?}'", doc_type));
        }
        if let Some(parent_id) = filters.parent_id {
            query.push_str(&format!(" AND parent_id = '{}'", parent_id));
        }
        if let Some(workspace_id) = filters.workspace_id {
            query.push_str(&format!(" AND workspace_id = '{}'", workspace_id));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filters.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut response = conn.connection().query(&query).await?;
        let documents: Vec<Document> = response.take(0)?;

        Ok(documents)
    }

    /// Publish a document
    pub async fn publish_document(&self, document_id: &CortexId) -> Result<Document> {
        info!("Publishing document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let document: Option<Document> = conn
            .connection()
            .select(("document", document_id.to_string()))
            .await?;

        let mut document = document
            .ok_or_else(|| anyhow::anyhow!("Document {} not found", document_id))?;

        document.publish();

        let document_json = serde_json::to_value(&document)?;
        let _: Option<serde_json::Value> = conn
            .connection()
            .update(("document", document_id.to_string()))
            .content(document_json)
            .await?;

        info!("Published document: {}", document_id);

        Ok(document)
    }

    /// Archive a document
    pub async fn archive_document(&self, document_id: &CortexId) -> Result<Document> {
        info!("Archiving document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let document: Option<Document> = conn
            .connection()
            .select(("document", document_id.to_string()))
            .await?;

        let mut document = document
            .ok_or_else(|| anyhow::anyhow!("Document {} not found", document_id))?;

        document.archive();

        let document_json = serde_json::to_value(&document)?;
        let _: Option<serde_json::Value> = conn
            .connection()
            .update(("document", document_id.to_string()))
            .content(document_json)
            .await?;

        info!("Archived document: {}", document_id);

        Ok(document)
    }

    // ============================================================================
    // Section Operations
    // ============================================================================

    /// Create a new section
    pub async fn create_section(
        &self,
        document_id: &CortexId,
        request: CreateSectionRequest,
    ) -> Result<DocumentSection> {
        info!("Creating section in document: {}", document_id);

        let mut section = DocumentSection::new(
            *document_id,
            request.title,
            request.content,
            request.level,
        );

        if let Some(parent_section_id) = request.parent_section_id {
            section.parent_section_id = Some(parent_section_id);
        }
        if let Some(order) = request.order {
            section.order = order;
        }

        let conn = self.storage.acquire().await?;
        let section_json = serde_json::to_value(&section)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("document_section", section.id.to_string()))
            .content(section_json)
            .await?;

        info!("Created section: {} in document: {}", section.id, document_id);

        Ok(section)
    }

    /// Get sections for a document
    pub async fn get_document_sections(
        &self,
        document_id: &CortexId,
    ) -> Result<Vec<DocumentSection>> {
        debug!("Getting sections for document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let query = format!(
            "SELECT * FROM document_section WHERE document_id = '{}' ORDER BY level, order",
            document_id
        );
        let mut response = conn.connection().query(&query).await?;
        let sections: Vec<DocumentSection> = response.take(0)?;

        Ok(sections)
    }

    /// Update section
    pub async fn update_section(
        &self,
        section_id: &CortexId,
        request: UpdateSectionRequest,
    ) -> Result<DocumentSection> {
        debug!("Updating section: {}", section_id);

        let conn = self.storage.acquire().await?;

        let section: Option<DocumentSection> = conn
            .connection()
            .select(("document_section", section_id.to_string()))
            .await?;

        let mut section = section
            .ok_or_else(|| anyhow::anyhow!("Section {} not found", section_id))?;

        if let Some(title) = request.title {
            section.title = title;
        }
        if let Some(content) = request.content {
            section.content = content;
        }
        if let Some(order) = request.order {
            section.order = order;
        }

        section.updated_at = chrono::Utc::now();

        let section_json = serde_json::to_value(&section)?;
        let _: Option<serde_json::Value> = conn
            .connection()
            .update(("document_section", section_id.to_string()))
            .content(section_json)
            .await?;

        Ok(section)
    }

    /// Delete section
    pub async fn delete_section(&self, section_id: &CortexId) -> Result<()> {
        info!("Deleting section: {}", section_id);

        let conn = self.storage.acquire().await?;

        let _: Option<DocumentSection> = conn
            .connection()
            .delete(("document_section", section_id.to_string()))
            .await?;

        Ok(())
    }

    // ============================================================================
    // Link Operations
    // ============================================================================

    /// Create a new link
    pub async fn create_link(&self, request: CreateLinkRequest) -> Result<DocumentLink> {
        info!("Creating link from document: {}", request.source_document_id);

        let link = DocumentLink::new(
            request.source_document_id,
            request.link_type,
            request.target,
        );

        let conn = self.storage.acquire().await?;
        let link_json = serde_json::to_value(&link)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("document_link", link.id.to_string()))
            .content(link_json)
            .await?;

        Ok(link)
    }

    /// Get links for a document
    pub async fn get_document_links(&self, document_id: &CortexId) -> Result<Vec<DocumentLink>> {
        debug!("Getting links for document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let query = format!(
            "SELECT * FROM document_link WHERE source_document_id = '{}'",
            document_id
        );
        let mut response = conn.connection().query(&query).await?;
        let links: Vec<DocumentLink> = response.take(0)?;

        Ok(links)
    }

    /// Delete link
    pub async fn delete_link(&self, link_id: &CortexId) -> Result<()> {
        info!("Deleting link: {}", link_id);

        let conn = self.storage.acquire().await?;

        let _: Option<DocumentLink> = conn
            .connection()
            .delete(("document_link", link_id.to_string()))
            .await?;

        Ok(())
    }

    // ============================================================================
    // Version Operations
    // ============================================================================

    /// Create a new version
    pub async fn create_version(
        &self,
        document_id: &CortexId,
        request: CreateVersionRequest,
    ) -> Result<DocumentVersion> {
        info!("Creating version for document: {}", document_id);

        // Get current document
        let document = self
            .get_document(document_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document {} not found", document_id))?;

        let version = DocumentVersion::new(
            *document_id,
            request.version,
            document.content,
            request.author,
            request.message,
        );

        let conn = self.storage.acquire().await?;
        let version_json = serde_json::to_value(&version)?;

        let _: Option<serde_json::Value> = conn
            .connection()
            .create(("document_version", version.id.to_string()))
            .content(version_json)
            .await?;

        Ok(version)
    }

    /// Get versions for a document
    pub async fn get_document_versions(
        &self,
        document_id: &CortexId,
    ) -> Result<Vec<DocumentVersion>> {
        debug!("Getting versions for document: {}", document_id);

        let conn = self.storage.acquire().await?;

        let query = format!(
            "SELECT * FROM document_version WHERE document_id = '{}' ORDER BY created_at DESC",
            document_id
        );
        let mut response = conn.connection().query(&query).await?;
        let versions: Vec<DocumentVersion> = response.take(0)?;

        Ok(versions)
    }

    /// Get specific version
    pub async fn get_version(&self, version_id: &CortexId) -> Result<Option<DocumentVersion>> {
        debug!("Getting version: {}", version_id);

        let conn = self.storage.acquire().await?;

        let version: Option<DocumentVersion> = conn
            .connection()
            .select(("document_version", version_id.to_string()))
            .await?;

        Ok(version)
    }

    // ============================================================================
    // Search Operations
    // ============================================================================

    /// Search documents
    pub async fn search_documents(&self, query: &str, limit: usize) -> Result<Vec<Document>> {
        info!("Searching documents: {}", query);

        let conn = self.storage.acquire().await?;

        // Simple text search - can be enhanced with full-text search later
        let search_query = format!(
            "SELECT * FROM document WHERE title CONTAINS '{}' OR content CONTAINS '{}' OR keywords CONTAINS '{}' LIMIT {}",
            query, query, query, limit
        );

        let mut response = conn.connection().query(&search_query).await?;
        let documents: Vec<Document> = response.take(0)?;

        Ok(documents)
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: String,
    pub doc_type: Option<DocumentType>,
    pub description: Option<String>,
    pub parent_id: Option<CortexId>,
    pub tags: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub workspace_id: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request to update a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDocumentRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub doc_type: Option<DocumentType>,
    pub tags: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Filters for listing documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDocumentFilters {
    pub status: Option<DocumentStatus>,
    pub doc_type: Option<DocumentType>,
    pub parent_id: Option<CortexId>,
    pub workspace_id: Option<String>,
    pub limit: Option<usize>,
}

/// Request to create a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSectionRequest {
    pub title: String,
    pub content: String,
    pub level: u32,
    pub parent_section_id: Option<String>,
    pub order: Option<i32>,
}

/// Request to update a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSectionRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub order: Option<i32>,
}

/// Request to create a link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLinkRequest {
    pub source_document_id: CortexId,
    pub link_type: LinkType,
    pub target: LinkTarget,
}

/// Request to create a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub version: String,
    pub author: String,
    pub message: String,
}
