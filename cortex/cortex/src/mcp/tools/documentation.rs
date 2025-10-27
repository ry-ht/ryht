//! Documentation MCP Tools - Comprehensive Document Management
//!
//! This module provides a complete set of tools for managing documents through the MCP protocol.
//! It includes CRUD operations, section management, link management, versioning, search, and
//! AI-assisted documentation generation.

use async_trait::async_trait;
use cortex_core::{
    CortexId, Document, DocumentSection, DocumentLink, DocumentVersion,
    DocumentType, DocumentStatus, LinkType, LinkTarget,
};
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

use crate::services::{
    DocumentService,
    document::*,
};

// =============================================================================
// Context
// =============================================================================

#[derive(Clone)]
pub struct DocumentationContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    service: Arc<DocumentService>,
}

impl DocumentationContext {
    pub fn new(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Self {
        let service = Arc::new(DocumentService::new(storage.clone(), vfs.clone()));
        Self {
            storage,
            vfs,
            service,
        }
    }
}

// =============================================================================
// cortex.document.create - Create new document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentCreateInput {
    title: String,
    content: String,
    #[serde(default)]
    doc_type: Option<String>,
    description: Option<String>,
    parent_id: Option<String>,
    tags: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    author: Option<String>,
    language: Option<String>,
    workspace_id: Option<String>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentCreateOutput {
    document_id: String,
    title: String,
    slug: String,
    created_at: String,
}

pub struct DocumentCreateTool {
    ctx: DocumentationContext,
}

impl DocumentCreateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentCreateTool {
    fn name(&self) -> &str {
        "cortex.document.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a new document with title, content, and optional metadata")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentCreateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Creating document: {}", input.title);

        // Parse document type
        let doc_type = if let Some(dt) = input.doc_type {
            parse_document_type(&dt)?
        } else {
            DocumentType::General
        };

        // Parse parent_id if provided
        let parent_id = if let Some(pid) = input.parent_id {
            Some(CortexId::from_str(&pid)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid parent_id: {}", e)))?)
        } else {
            None
        };

        let request = CreateDocumentRequest {
            title: input.title,
            content: input.content,
            doc_type: Some(doc_type),
            description: input.description,
            parent_id,
            tags: input.tags,
            keywords: input.keywords,
            author: input.author,
            language: input.language,
            workspace_id: input.workspace_id,
            metadata: input.metadata,
        };

        let document = self.ctx.service.create_document(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create document: {}", e)))?;

        let output = DocumentCreateOutput {
            document_id: document.id.to_string(),
            title: document.title,
            slug: document.slug,
            created_at: document.created_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.get - Get document by ID
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentGetInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentGetOutput {
    document_id: String,
    title: String,
    content: String,
    slug: String,
    doc_type: String,
    status: String,
    description: Option<String>,
    parent_id: Option<String>,
    tags: Vec<String>,
    keywords: Vec<String>,
    author: Option<String>,
    language: String,
    workspace_id: Option<String>,
    version: String,
    created_at: String,
    updated_at: String,
    published_at: Option<String>,
    metadata: HashMap<String, serde_json::Value>,
}

pub struct DocumentGetTool {
    ctx: DocumentationContext,
}

impl DocumentGetTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentGetTool {
    fn name(&self) -> &str {
        "cortex.document.get"
    }

    fn description(&self) -> Option<&str> {
        Some("Get a document by its ID")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentGetInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentGetInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let document = self.ctx.service.get_document(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get document: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Document not found".to_string()))?;

        let output = DocumentGetOutput {
            document_id: document.id.to_string(),
            title: document.title,
            content: document.content,
            slug: document.slug,
            doc_type: format!("{:?}", document.doc_type),
            status: format!("{:?}", document.status),
            description: document.description,
            parent_id: document.parent_id.map(|id| id.to_string()),
            tags: document.tags,
            keywords: document.keywords,
            author: Some(document.author),
            language: document.language,
            workspace_id: document.workspace_id,
            version: document.version,
            created_at: document.created_at.to_rfc3339(),
            updated_at: document.updated_at.to_rfc3339(),
            published_at: document.published_at.map(|dt| dt.to_rfc3339()),
            metadata: document.metadata,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.update - Update document metadata
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentUpdateInput {
    document_id: String,
    title: Option<String>,
    content: Option<String>,
    description: Option<String>,
    doc_type: Option<String>,
    tags: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentUpdateOutput {
    document_id: String,
    updated: bool,
    version: String,
}

pub struct DocumentUpdateTool {
    ctx: DocumentationContext,
}

impl DocumentUpdateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentUpdateTool {
    fn name(&self) -> &str {
        "cortex.document.update"
    }

    fn description(&self) -> Option<&str> {
        Some("Update document metadata and content")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentUpdateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentUpdateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let doc_type = if let Some(dt) = input.doc_type {
            Some(parse_document_type(&dt)?)
        } else {
            None
        };

        let request = UpdateDocumentRequest {
            title: input.title,
            content: input.content,
            description: input.description,
            doc_type,
            tags: input.tags,
            keywords: input.keywords,
            metadata: input.metadata,
        };

        let document = self.ctx.service.update_document(&document_id, request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update document: {}", e)))?;

        let output = DocumentUpdateOutput {
            document_id: document.id.to_string(),
            updated: true,
            version: document.version,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.delete - Delete document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentDeleteInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentDeleteOutput {
    document_id: String,
    deleted: bool,
}

pub struct DocumentDeleteTool {
    ctx: DocumentationContext,
}

impl DocumentDeleteTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentDeleteTool {
    fn name(&self) -> &str {
        "cortex.document.delete"
    }

    fn description(&self) -> Option<&str> {
        Some("Delete a document by its ID")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentDeleteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentDeleteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        self.ctx.service.delete_document(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete document: {}", e)))?;

        let output = DocumentDeleteOutput {
            document_id: input.document_id,
            deleted: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.list - List documents with filters
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentListInput {
    status: Option<String>,
    doc_type: Option<String>,
    parent_id: Option<String>,
    workspace_id: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentListOutput {
    documents: Vec<DocumentSummary>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentSummary {
    document_id: String,
    title: String,
    slug: String,
    doc_type: String,
    status: String,
    author: Option<String>,
    created_at: String,
    updated_at: String,
}

pub struct DocumentListTool {
    ctx: DocumentationContext,
}

impl DocumentListTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentListTool {
    fn name(&self) -> &str {
        "cortex.document.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List documents with optional filters for status, type, parent, and workspace")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let status = if let Some(s) = input.status {
            Some(parse_document_status(&s)?)
        } else {
            None
        };

        let doc_type = if let Some(dt) = input.doc_type {
            Some(parse_document_type(&dt)?)
        } else {
            None
        };

        let parent_id = if let Some(pid) = input.parent_id {
            Some(CortexId::from_str(&pid)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid parent_id: {}", e)))?)
        } else {
            None
        };

        let filters = ListDocumentFilters {
            status,
            doc_type,
            parent_id,
            workspace_id: input.workspace_id,
            limit: Some(input.limit),
        };

        let documents = self.ctx.service.list_documents(filters)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list documents: {}", e)))?;

        let document_summaries: Vec<DocumentSummary> = documents
            .iter()
            .map(|doc| DocumentSummary {
                document_id: doc.id.to_string(),
                title: doc.title.clone(),
                slug: doc.slug.clone(),
                doc_type: format!("{:?}", doc.doc_type),
                status: format!("{:?}", doc.status),
                author: Some(doc.author.clone()),
                created_at: doc.created_at.to_rfc3339(),
                updated_at: doc.updated_at.to_rfc3339(),
            })
            .collect();

        let output = DocumentListOutput {
            total_count: document_summaries.len(),
            documents: document_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.section.create - Create new section
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SectionCreateInput {
    document_id: String,
    title: String,
    content: String,
    level: u32,
    parent_section_id: Option<String>,
    order: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionCreateOutput {
    section_id: String,
    document_id: String,
    title: String,
    level: u32,
    order: i32,
}

pub struct SectionCreateTool {
    ctx: DocumentationContext,
}

impl SectionCreateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SectionCreateTool {
    fn name(&self) -> &str {
        "cortex.document.section.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a new section within a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SectionCreateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SectionCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let request = CreateSectionRequest {
            title: input.title,
            content: input.content,
            level: input.level,
            parent_section_id: input.parent_section_id,
            order: input.order,
        };

        let section = self.ctx.service.create_section(&document_id, request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create section: {}", e)))?;

        let output = SectionCreateOutput {
            section_id: section.id.to_string(),
            document_id: section.document_id.to_string(),
            title: section.title,
            level: section.level,
            order: section.order,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.section.update - Update section content
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SectionUpdateInput {
    section_id: String,
    title: Option<String>,
    content: Option<String>,
    order: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionUpdateOutput {
    section_id: String,
    updated: bool,
}

pub struct SectionUpdateTool {
    ctx: DocumentationContext,
}

impl SectionUpdateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SectionUpdateTool {
    fn name(&self) -> &str {
        "cortex.document.section.update"
    }

    fn description(&self) -> Option<&str> {
        Some("Update section content and metadata")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SectionUpdateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SectionUpdateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let section_id = CortexId::from_str(&input.section_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid section_id: {}", e)))?;

        let request = UpdateSectionRequest {
            title: input.title,
            content: input.content,
            order: input.order,
        };

        self.ctx.service.update_section(&section_id, request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update section: {}", e)))?;

        let output = SectionUpdateOutput {
            section_id: input.section_id,
            updated: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.section.delete - Delete section
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SectionDeleteInput {
    section_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionDeleteOutput {
    section_id: String,
    deleted: bool,
}

pub struct SectionDeleteTool {
    ctx: DocumentationContext,
}

impl SectionDeleteTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SectionDeleteTool {
    fn name(&self) -> &str {
        "cortex.document.section.delete"
    }

    fn description(&self) -> Option<&str> {
        Some("Delete a section from a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SectionDeleteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SectionDeleteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let section_id = CortexId::from_str(&input.section_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid section_id: {}", e)))?;

        self.ctx.service.delete_section(&section_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete section: {}", e)))?;

        let output = SectionDeleteOutput {
            section_id: input.section_id,
            deleted: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.section.list - List sections for document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SectionListInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionListOutput {
    sections: Vec<SectionSummary>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionSummary {
    section_id: String,
    title: String,
    level: u32,
    order: i32,
    parent_section_id: Option<String>,
}

pub struct SectionListTool {
    ctx: DocumentationContext,
}

impl SectionListTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SectionListTool {
    fn name(&self) -> &str {
        "cortex.document.section.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List all sections for a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SectionListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SectionListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let sections = self.ctx.service.get_document_sections(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list sections: {}", e)))?;

        let section_summaries: Vec<SectionSummary> = sections
            .iter()
            .map(|s| SectionSummary {
                section_id: s.id.to_string(),
                title: s.title.clone(),
                level: s.level,
                order: s.order,
                parent_section_id: s.parent_section_id.clone(),
            })
            .collect();

        let output = SectionListOutput {
            total_count: section_summaries.len(),
            sections: section_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.link.create - Create link from document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkCreateInput {
    source_document_id: String,
    link_type: String,
    target_type: String,
    target_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LinkCreateOutput {
    link_id: String,
    source_document_id: String,
    link_type: String,
    target: String,
}

pub struct LinkCreateTool {
    ctx: DocumentationContext,
}

impl LinkCreateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LinkCreateTool {
    fn name(&self) -> &str {
        "cortex.document.link.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a link from a document to another resource (document, code unit, external URL)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LinkCreateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LinkCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let source_document_id = CortexId::from_str(&input.source_document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid source_document_id: {}", e)))?;

        let link_type = parse_link_type(&input.link_type)?;
        let target = parse_link_target(&input.target_type, &input.target_id)?;

        let request = CreateLinkRequest {
            source_document_id,
            link_type,
            target: target.clone(),
        };

        let link = self.ctx.service.create_link(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create link: {}", e)))?;

        let output = LinkCreateOutput {
            link_id: link.id.to_string(),
            source_document_id: link.source_document_id.to_string(),
            link_type: format!("{:?}", link.link_type),
            target: format!("{:?}", link.target),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.link.list - List links for document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkListInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LinkListOutput {
    links: Vec<LinkSummary>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LinkSummary {
    link_id: String,
    link_type: String,
    target: String,
    created_at: String,
}

pub struct LinkListTool {
    ctx: DocumentationContext,
}

impl LinkListTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LinkListTool {
    fn name(&self) -> &str {
        "cortex.document.link.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List all links for a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LinkListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LinkListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let links = self.ctx.service.get_document_links(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list links: {}", e)))?;

        let link_summaries: Vec<LinkSummary> = links
            .iter()
            .map(|l| LinkSummary {
                link_id: l.id.to_string(),
                link_type: format!("{:?}", l.link_type),
                target: format!("{:?}", l.target),
                created_at: l.created_at.to_rfc3339(),
            })
            .collect();

        let output = LinkListOutput {
            total_count: link_summaries.len(),
            links: link_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.link.delete - Delete link
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkDeleteInput {
    link_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LinkDeleteOutput {
    link_id: String,
    deleted: bool,
}

pub struct LinkDeleteTool {
    ctx: DocumentationContext,
}

impl LinkDeleteTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LinkDeleteTool {
    fn name(&self) -> &str {
        "cortex.document.link.delete"
    }

    fn description(&self) -> Option<&str> {
        Some("Delete a link from a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LinkDeleteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LinkDeleteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let link_id = CortexId::from_str(&input.link_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid link_id: {}", e)))?;

        self.ctx.service.delete_link(&link_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to delete link: {}", e)))?;

        let output = LinkDeleteOutput {
            link_id: input.link_id,
            deleted: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.search - Search documents
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentSearchInput {
    query: String,
    #[serde(default = "default_search_limit")]
    limit: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentSearchOutput {
    documents: Vec<DocumentSummary>,
    total_count: usize,
    query: String,
}

pub struct DocumentSearchTool {
    ctx: DocumentationContext,
}

impl DocumentSearchTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentSearchTool {
    fn name(&self) -> &str {
        "cortex.document.search"
    }

    fn description(&self) -> Option<&str> {
        Some("Search documents by title, content, or keywords")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let documents = self.ctx.service.search_documents(&input.query, input.limit)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to search documents: {}", e)))?;

        let document_summaries: Vec<DocumentSummary> = documents
            .iter()
            .map(|doc| DocumentSummary {
                document_id: doc.id.to_string(),
                title: doc.title.clone(),
                slug: doc.slug.clone(),
                doc_type: format!("{:?}", doc.doc_type),
                status: format!("{:?}", doc.status),
                author: Some(doc.author.clone()),
                created_at: doc.created_at.to_rfc3339(),
                updated_at: doc.updated_at.to_rfc3339(),
            })
            .collect();

        let output = DocumentSearchOutput {
            total_count: document_summaries.len(),
            documents: document_summaries,
            query: input.query,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.version.create - Create version snapshot
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VersionCreateInput {
    document_id: String,
    version: String,
    author: String,
    message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionCreateOutput {
    version_id: String,
    document_id: String,
    version: String,
    created_at: String,
}

pub struct VersionCreateTool {
    ctx: DocumentationContext,
}

impl VersionCreateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionCreateTool {
    fn name(&self) -> &str {
        "cortex.document.version.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a version snapshot of a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(VersionCreateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: VersionCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let request = CreateVersionRequest {
            version: input.version,
            author: input.author,
            message: input.message,
        };

        let version = self.ctx.service.create_version(&document_id, request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create version: {}", e)))?;

        let output = VersionCreateOutput {
            version_id: version.id.to_string(),
            document_id: version.document_id.to_string(),
            version: version.version,
            created_at: version.created_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.version.list - List versions
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VersionListInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionListOutput {
    versions: Vec<VersionSummary>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionSummary {
    version_id: String,
    version: String,
    author: String,
    message: String,
    created_at: String,
}

pub struct VersionListTool {
    ctx: DocumentationContext,
}

impl VersionListTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionListTool {
    fn name(&self) -> &str {
        "cortex.document.version.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List all versions for a document")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(VersionListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: VersionListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let versions = self.ctx.service.get_document_versions(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list versions: {}", e)))?;

        let version_summaries: Vec<VersionSummary> = versions
            .iter()
            .map(|v| VersionSummary {
                version_id: v.id.to_string(),
                version: v.version.clone(),
                author: v.author.clone(),
                message: v.message.clone(),
                created_at: v.created_at.to_rfc3339(),
            })
            .collect();

        let output = VersionListOutput {
            total_count: version_summaries.len(),
            versions: version_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.publish - Publish document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentPublishInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentPublishOutput {
    document_id: String,
    status: String,
    published_at: String,
}

pub struct DocumentPublishTool {
    ctx: DocumentationContext,
}

impl DocumentPublishTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentPublishTool {
    fn name(&self) -> &str {
        "cortex.document.publish"
    }

    fn description(&self) -> Option<&str> {
        Some("Publish a document (set status to Published)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentPublishInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentPublishInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let document = self.ctx.service.publish_document(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to publish document: {}", e)))?;

        let output = DocumentPublishOutput {
            document_id: document.id.to_string(),
            status: format!("{:?}", document.status),
            published_at: document.published_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.archive - Archive document
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentArchiveInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentArchiveOutput {
    document_id: String,
    status: String,
}

pub struct DocumentArchiveTool {
    ctx: DocumentationContext,
}

impl DocumentArchiveTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentArchiveTool {
    fn name(&self) -> &str {
        "cortex.document.archive"
    }

    fn description(&self) -> Option<&str> {
        Some("Archive a document (set status to Archived)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentArchiveInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentArchiveInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let document = self.ctx.service.archive_document(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to archive document: {}", e)))?;

        let output = DocumentArchiveOutput {
            document_id: document.id.to_string(),
            status: format!("{:?}", document.status),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.get-by-slug - Get document by slug
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentGetBySlugInput {
    slug: String,
}

pub struct DocumentGetBySlugTool {
    ctx: DocumentationContext,
}

impl DocumentGetBySlugTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentGetBySlugTool {
    fn name(&self) -> &str {
        "cortex.document.get-by-slug"
    }

    fn description(&self) -> Option<&str> {
        Some("Get a document by its URL slug")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentGetBySlugInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentGetBySlugInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document = self.ctx.service.get_document_by_slug(&input.slug)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get document: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Document not found".to_string()))?;

        let output = DocumentGetOutput {
            document_id: document.id.to_string(),
            title: document.title,
            content: document.content,
            slug: document.slug,
            doc_type: format!("{:?}", document.doc_type),
            status: format!("{:?}", document.status),
            description: document.description,
            parent_id: document.parent_id.map(|id| id.to_string()),
            tags: document.tags,
            keywords: document.keywords,
            author: Some(document.author),
            language: document.language,
            workspace_id: document.workspace_id,
            version: document.version,
            created_at: document.created_at.to_rfc3339(),
            updated_at: document.updated_at.to_rfc3339(),
            published_at: document.published_at.map(|dt| dt.to_rfc3339()),
            metadata: document.metadata,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.version.get - Get specific version
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VersionGetInput {
    version_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionGetOutput {
    version_id: String,
    document_id: String,
    version: String,
    content: String,
    author: String,
    message: String,
    created_at: String,
}

pub struct VersionGetTool {
    ctx: DocumentationContext,
}

impl VersionGetTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for VersionGetTool {
    fn name(&self) -> &str {
        "cortex.document.version.get"
    }

    fn description(&self) -> Option<&str> {
        Some("Get a specific document version by its ID")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(VersionGetInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: VersionGetInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let version_id = CortexId::from_str(&input.version_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid version_id: {}", e)))?;

        let version = self.ctx.service.get_version(&version_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get version: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Version not found".to_string()))?;

        let output = VersionGetOutput {
            version_id: version.id.to_string(),
            document_id: version.document_id.to_string(),
            version: version.version,
            content: version.content,
            author: version.author,
            message: version.message,
            created_at: version.created_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.section.get - Get specific section
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SectionGetInput {
    section_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionGetOutput {
    section_id: String,
    document_id: String,
    title: String,
    content: String,
    level: u32,
    order: i32,
    parent_section_id: Option<String>,
    created_at: String,
    updated_at: String,
}

pub struct SectionGetTool {
    ctx: DocumentationContext,
}

impl SectionGetTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SectionGetTool {
    fn name(&self) -> &str {
        "cortex.document.section.get"
    }

    fn description(&self) -> Option<&str> {
        Some("Get a specific section by its ID")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SectionGetInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SectionGetInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let section_id = CortexId::from_str(&input.section_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid section_id: {}", e)))?;

        let section = self.ctx.service.get_section(&section_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get section: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Section not found".to_string()))?;

        let output = SectionGetOutput {
            section_id: section.id.to_string(),
            document_id: section.document_id.to_string(),
            title: section.title,
            content: section.content,
            level: section.level,
            order: section.order,
            parent_section_id: section.parent_section_id,
            created_at: section.created_at.to_rfc3339(),
            updated_at: section.updated_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.tree - Get document tree with children
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentTreeInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentTreeOutput {
    document: DocumentGetOutput,
    children: Vec<DocumentSummary>,
    sections_count: usize,
    links_count: usize,
}

pub struct DocumentTreeTool {
    ctx: DocumentationContext,
}

impl DocumentTreeTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentTreeTool {
    fn name(&self) -> &str {
        "cortex.document.tree"
    }

    fn description(&self) -> Option<&str> {
        Some("Get document hierarchy tree with children, sections count, and links count")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentTreeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentTreeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let tree = self.ctx.service.get_document_tree(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get document tree: {}", e)))?;

        let document_output = DocumentGetOutput {
            document_id: tree.document.id.to_string(),
            title: tree.document.title.clone(),
            content: tree.document.content.clone(),
            slug: tree.document.slug.clone(),
            doc_type: format!("{:?}", tree.document.doc_type),
            status: format!("{:?}", tree.document.status),
            description: tree.document.description.clone(),
            parent_id: tree.document.parent_id.map(|id| id.to_string()),
            tags: tree.document.tags.clone(),
            keywords: tree.document.keywords.clone(),
            author: Some(tree.document.author.clone()),
            language: tree.document.language.clone(),
            workspace_id: tree.document.workspace_id.clone(),
            version: tree.document.version.clone(),
            created_at: tree.document.created_at.to_rfc3339(),
            updated_at: tree.document.updated_at.to_rfc3339(),
            published_at: tree.document.published_at.map(|dt| dt.to_rfc3339()),
            metadata: tree.document.metadata.clone(),
        };

        let children_output: Vec<DocumentSummary> = tree.children
            .iter()
            .map(|doc| DocumentSummary {
                document_id: doc.id.to_string(),
                title: doc.title.clone(),
                slug: doc.slug.clone(),
                doc_type: format!("{:?}", doc.doc_type),
                status: format!("{:?}", doc.status),
                author: Some(doc.author.clone()),
                created_at: doc.created_at.to_rfc3339(),
                updated_at: doc.updated_at.to_rfc3339(),
            })
            .collect();

        let output = DocumentTreeOutput {
            document: document_output,
            children: children_output,
            sections_count: tree.sections_count,
            links_count: tree.links_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.clone - Clone document with sections
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentCloneInput {
    document_id: String,
    new_title: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentCloneOutput {
    new_document_id: String,
    original_document_id: String,
    title: String,
    slug: String,
    sections_cloned: usize,
}

pub struct DocumentCloneTool {
    ctx: DocumentationContext,
}

impl DocumentCloneTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentCloneTool {
    fn name(&self) -> &str {
        "cortex.document.clone"
    }

    fn description(&self) -> Option<&str> {
        Some("Clone a document with all its sections (creates as Draft)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentCloneInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentCloneInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        // Get sections count before cloning
        let sections = self.ctx.service.get_document_sections(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get sections: {}", e)))?;
        let sections_count = sections.len();

        let new_doc = self.ctx.service.clone_document(&document_id, input.new_title)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to clone document: {}", e)))?;

        let output = DocumentCloneOutput {
            new_document_id: new_doc.id.to_string(),
            original_document_id: document_id.to_string(),
            title: new_doc.title,
            slug: new_doc.slug,
            sections_cloned: sections_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.related - Get related documents
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentRelatedInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentRelatedOutput {
    related_documents: Vec<RelatedDocumentInfo>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RelatedDocumentInfo {
    document: DocumentSummary,
    link_type: String,
    link_id: String,
}

pub struct DocumentRelatedTool {
    ctx: DocumentationContext,
}

impl DocumentRelatedTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentRelatedTool {
    fn name(&self) -> &str {
        "cortex.document.related"
    }

    fn description(&self) -> Option<&str> {
        Some("Get all related documents through links")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentRelatedInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentRelatedInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let related = self.ctx.service.get_related_documents(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get related documents: {}", e)))?;

        let related_info: Vec<RelatedDocumentInfo> = related
            .iter()
            .map(|r| RelatedDocumentInfo {
                document: DocumentSummary {
                    document_id: r.document.id.to_string(),
                    title: r.document.title.clone(),
                    slug: r.document.slug.clone(),
                    doc_type: format!("{:?}", r.document.doc_type),
                    status: format!("{:?}", r.document.status),
                    author: Some(r.document.author.clone()),
                    created_at: r.document.created_at.to_rfc3339(),
                    updated_at: r.document.updated_at.to_rfc3339(),
                },
                link_type: format!("{:?}", r.link_type),
                link_id: r.link_id.to_string(),
            })
            .collect();

        let output = DocumentRelatedOutput {
            total_count: related_info.len(),
            related_documents: related_info,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.document.stats - Get document statistics
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocumentStatsInput {
    document_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocumentStatsOutput {
    document_id: String,
    content_length: usize,
    word_count: usize,
    line_count: usize,
    sections_count: usize,
    links_count: usize,
    versions_count: usize,
    tags_count: usize,
    keywords_count: usize,
}

pub struct DocumentStatsTool {
    ctx: DocumentationContext,
}

impl DocumentStatsTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocumentStatsTool {
    fn name(&self) -> &str {
        "cortex.document.stats"
    }

    fn description(&self) -> Option<&str> {
        Some("Get document statistics including content metrics, sections, links, and versions")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocumentStatsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocumentStatsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let document_id = CortexId::from_str(&input.document_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document_id: {}", e)))?;

        let stats = self.ctx.service.get_document_stats(&document_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get document stats: {}", e)))?;

        let output = DocumentStatsOutput {
            document_id: stats.document_id.to_string(),
            content_length: stats.content_length,
            word_count: stats.word_count,
            line_count: stats.line_count,
            sections_count: stats.sections_count,
            links_count: stats.links_count,
            versions_count: stats.versions_count,
            tags_count: stats.tags_count,
            keywords_count: stats.keywords_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Legacy compatibility tools - refactored from old documentation.rs
// =============================================================================

// cortex.document.generate_from_code - Generate docs from code
// cortex.document.check_consistency - Check doc/code consistency

// These are kept for backward compatibility but should use the new document system

// =============================================================================
// Helper functions
// =============================================================================

fn parse_document_type(s: &str) -> std::result::Result<DocumentType, ToolError> {
    match s.to_lowercase().as_str() {
        "guide" => Ok(DocumentType::Guide),
        "api_reference" | "api" => Ok(DocumentType::ApiReference),
        "architecture" => Ok(DocumentType::Architecture),
        "tutorial" => Ok(DocumentType::Tutorial),
        "explanation" => Ok(DocumentType::Explanation),
        "troubleshooting" => Ok(DocumentType::Troubleshooting),
        "faq" => Ok(DocumentType::Faq),
        "release_notes" => Ok(DocumentType::ReleaseNotes),
        "example" => Ok(DocumentType::Example),
        "general" => Ok(DocumentType::General),
        _ => Err(ToolError::ExecutionFailed(format!("Invalid document type: {}", s))),
    }
}

fn parse_document_status(s: &str) -> std::result::Result<DocumentStatus, ToolError> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(DocumentStatus::Draft),
        "review" => Ok(DocumentStatus::Review),
        "published" => Ok(DocumentStatus::Published),
        "archived" => Ok(DocumentStatus::Archived),
        _ => Err(ToolError::ExecutionFailed(format!("Invalid document status: {}", s))),
    }
}

fn parse_link_type(s: &str) -> std::result::Result<LinkType, ToolError> {
    match s.to_lowercase().as_str() {
        "reference" => Ok(LinkType::Reference),
        "related" => Ok(LinkType::Related),
        "prerequisite" => Ok(LinkType::Prerequisite),
        "next" => Ok(LinkType::Next),
        "previous" => Ok(LinkType::Previous),
        "parent" => Ok(LinkType::Parent),
        "child" => Ok(LinkType::Child),
        "external" => Ok(LinkType::External),
        "api_reference" | "api" => Ok(LinkType::ApiReference),
        "example" => Ok(LinkType::Example),
        _ => Err(ToolError::ExecutionFailed(format!("Invalid link type: {}", s))),
    }
}

fn parse_link_target(target_type: &str, target_id: &str) -> std::result::Result<LinkTarget, ToolError> {
    match target_type.to_lowercase().as_str() {
        "document" => {
            let document_id = CortexId::from_str(target_id)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid document ID: {}", e)))?;
            Ok(LinkTarget::Document {
                document_id,
                section_id: None,
            })
        }
        "codeunit" | "code_unit" => {
            let code_unit_id = CortexId::from_str(target_id)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid code unit ID: {}", e)))?;
            Ok(LinkTarget::CodeUnit { code_unit_id })
        }
        "external" => Ok(LinkTarget::External {
            url: target_id.to_string(),
        }),
        "file" => Ok(LinkTarget::File {
            path: target_id.to_string(),
        }),
        _ => Err(ToolError::ExecutionFailed(format!("Invalid target type: {}", target_type))),
    }
}

fn default_limit() -> usize {
    50
}

fn default_search_limit() -> usize {
    20
}
