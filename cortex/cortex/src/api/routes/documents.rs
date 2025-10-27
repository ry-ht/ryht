//! Document management endpoints

use crate::api::{
    error::{ApiError, ApiResult},
    middleware::AuthUser,
    types::{ApiResponse, PaginationParams},
    pagination::{LinkBuilder, build_pagination_info, generate_next_cursor},
};
use crate::services::{
    DocumentService,
    document::*,
};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use cortex_core::CortexId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

/// Document context
#[derive(Clone)]
pub struct DocumentContext {
    pub document_service: Arc<DocumentService>,
}

/// Create document routes
pub fn document_routes(context: DocumentContext) -> Router {
    Router::new()
        // Document CRUD
        .route("/api/v1/documents", get(list_documents))
        .route("/api/v1/documents", post(create_document))
        .route("/api/v1/documents/search", get(search_documents))
        .route("/api/v1/documents/{id}", get(get_document))
        .route("/api/v1/documents/{id}", put(update_document))
        .route("/api/v1/documents/{id}", delete(delete_document))
        .route("/api/v1/documents/{id}/publish", post(publish_document))
        .route("/api/v1/documents/{id}/archive", post(archive_document))
        // Section management
        .route("/api/v1/documents/{id}/sections", get(list_sections))
        .route("/api/v1/documents/{id}/sections", post(create_section))
        .route("/api/v1/sections/{id}", put(update_section))
        .route("/api/v1/sections/{id}", delete(delete_section))
        // Link management
        .route("/api/v1/documents/{id}/links", get(list_links))
        .route("/api/v1/documents/{id}/links", post(create_link))
        .route("/api/v1/links/{id}", delete(delete_link))
        // Version management
        .route("/api/v1/documents/{id}/versions", get(list_versions))
        .route("/api/v1/documents/{id}/versions", post(create_version))
        .with_state(context)
}

// =============================================================================
// Document Handlers
// =============================================================================

/// GET /api/v1/documents - List all documents
async fn list_documents(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Query(params): Query<ListDocumentsQuery>,
) -> ApiResult<Json<ApiResponse<Vec<DocumentResponseItem>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::info!(
        user_id = %auth_user.user_id,
        "User listing documents"
    );

    let filters = ListDocumentFilters {
        status: params.status.as_ref().and_then(|s| parse_status(s).ok()),
        doc_type: params.doc_type.as_ref().and_then(|dt| parse_doc_type(dt).ok()),
        parent_id: params.parent_id.as_ref().and_then(|id| CortexId::from_str(id).ok()),
        workspace_id: params.workspace_id,
        limit: Some(params.limit.unwrap_or(50)),
    };

    let documents = ctx.document_service
        .list_documents(filters)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<DocumentResponseItem> = documents
        .into_iter()
        .map(|doc| DocumentResponseItem {
            id: doc.id.to_string(),
            title: doc.title,
            slug: doc.slug,
            doc_type: format!("{:?}", doc.doc_type),
            status: format!("{:?}", doc.status),
            author: Some(doc.author),
            created_at: doc.created_at.to_rfc3339(),
            updated_at: doc.updated_at.to_rfc3339(),
        })
        .collect();

    let duration = start.elapsed();
    let response = ApiResponse::success(
        items,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents - Create document
async fn create_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Json(payload): Json<CreateDocumentPayload>,
) -> ApiResult<Json<ApiResponse<DocumentResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    tracing::info!(
        user_id = %auth_user.user_id,
        title = %payload.title,
        "User creating document"
    );

    let request = CreateDocumentRequest {
        title: payload.title,
        content: payload.content,
        doc_type: payload.doc_type.as_ref().and_then(|dt| parse_doc_type(dt).ok()),
        description: payload.description,
        parent_id: payload.parent_id.as_ref().and_then(|id| CortexId::from_str(id).ok()),
        tags: payload.tags,
        keywords: payload.keywords,
        author: Some(auth_user.email),
        language: payload.language,
        workspace_id: payload.workspace_id,
        metadata: payload.metadata,
    };

    let document = ctx.document_service
        .create_document(request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = DocumentResponse {
        id: document.id.to_string(),
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

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// GET /api/v1/documents/:id - Get document
async fn get_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DocumentResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let document = ctx.document_service
        .get_document(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Document not found".to_string()))?;

    let response_data = DocumentResponse {
        id: document.id.to_string(),
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

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// PUT /api/v1/documents/:id - Update document
async fn update_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDocumentPayload>,
) -> ApiResult<Json<ApiResponse<DocumentResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let request = UpdateDocumentRequest {
        title: payload.title,
        content: payload.content,
        description: payload.description,
        doc_type: payload.doc_type.as_ref().and_then(|dt| parse_doc_type(dt).ok()),
        tags: payload.tags,
        keywords: payload.keywords,
        metadata: payload.metadata,
    };

    let document = ctx.document_service
        .update_document(&document_id, request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = DocumentResponse {
        id: document.id.to_string(),
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

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// DELETE /api/v1/documents/:id - Delete document
async fn delete_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DeleteResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    ctx.document_service
        .delete_document(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let duration = start.elapsed();
    let response = ApiResponse::success(
        DeleteResponse {
            id,
            deleted: true,
        },
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents/{id}/publish - Publish document
async fn publish_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DocumentResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let document = ctx.document_service
        .publish_document(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = DocumentResponse {
        id: document.id.to_string(),
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

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents/{id}/archive - Archive document
async fn archive_document(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DocumentResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let document = ctx.document_service
        .archive_document(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = DocumentResponse {
        id: document.id.to_string(),
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

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// GET /api/v1/documents/search - Search documents
async fn search_documents(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Query(params): Query<SearchDocumentsQuery>,
) -> ApiResult<Json<ApiResponse<Vec<DocumentResponseItem>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let query = params.q.ok_or_else(|| ApiError::BadRequest("Missing 'q' parameter".to_string()))?;
    let limit = params.limit.unwrap_or(20);

    let documents = ctx.document_service
        .search_documents(&query, limit)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<DocumentResponseItem> = documents
        .into_iter()
        .map(|doc| DocumentResponseItem {
            id: doc.id.to_string(),
            title: doc.title,
            slug: doc.slug,
            doc_type: format!("{:?}", doc.doc_type),
            status: format!("{:?}", doc.status),
            author: Some(doc.author),
            created_at: doc.created_at.to_rfc3339(),
            updated_at: doc.updated_at.to_rfc3339(),
        })
        .collect();

    let duration = start.elapsed();
    let response = ApiResponse::success(
        items,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

// =============================================================================
// Section Handlers
// =============================================================================

/// GET /api/v1/documents/{id}/sections - List sections
async fn list_sections(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<SectionResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let sections = ctx.document_service
        .get_document_sections(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<SectionResponse> = sections
        .into_iter()
        .map(|s| SectionResponse {
            id: s.id.to_string(),
            document_id: s.document_id.to_string(),
            title: s.title,
            content: s.content,
            level: s.level,
            order: s.order,
            parent_section_id: s.parent_section_id,
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        })
        .collect();

    let duration = start.elapsed();
    let response = ApiResponse::success(
        items,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents/{id}/sections - Create section
async fn create_section(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
    Json(payload): Json<CreateSectionPayload>,
) -> ApiResult<Json<ApiResponse<SectionResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let request = CreateSectionRequest {
        title: payload.title,
        content: payload.content,
        level: payload.level,
        parent_section_id: payload.parent_section_id,
        order: payload.order,
    };

    let section = ctx.document_service
        .create_section(&document_id, request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = SectionResponse {
        id: section.id.to_string(),
        document_id: section.document_id.to_string(),
        title: section.title,
        content: section.content,
        level: section.level,
        order: section.order,
        parent_section_id: section.parent_section_id,
        created_at: section.created_at.to_rfc3339(),
        updated_at: section.updated_at.to_rfc3339(),
    };

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// PUT /api/v1/sections/:id - Update section
async fn update_section(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSectionPayload>,
) -> ApiResult<Json<ApiResponse<SectionResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let section_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid section ID: {}", e)))?;

    let request = UpdateSectionRequest {
        title: payload.title,
        content: payload.content,
        order: payload.order,
    };

    let section = ctx.document_service
        .update_section(&section_id, request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = SectionResponse {
        id: section.id.to_string(),
        document_id: section.document_id.to_string(),
        title: section.title,
        content: section.content,
        level: section.level,
        order: section.order,
        parent_section_id: section.parent_section_id,
        created_at: section.created_at.to_rfc3339(),
        updated_at: section.updated_at.to_rfc3339(),
    };

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// DELETE /api/v1/sections/:id - Delete section
async fn delete_section(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DeleteResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let section_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid section ID: {}", e)))?;

    ctx.document_service
        .delete_section(&section_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let duration = start.elapsed();
    let response = ApiResponse::success(
        DeleteResponse {
            id,
            deleted: true,
        },
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

// =============================================================================
// Link Handlers
// =============================================================================

/// GET /api/v1/documents/{id}/links - List links
async fn list_links(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<LinkResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let links = ctx.document_service
        .get_document_links(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<LinkResponse> = links
        .into_iter()
        .map(|l| LinkResponse {
            id: l.id.to_string(),
            source_document_id: l.source_document_id.to_string(),
            link_type: format!("{:?}", l.link_type),
            target: format!("{:?}", l.target),
            created_at: l.created_at.to_rfc3339(),
        })
        .collect();

    let duration = start.elapsed();
    let response = ApiResponse::success(
        items,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents/{id}/links - Create link
async fn create_link(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
    Json(payload): Json<CreateLinkPayload>,
) -> ApiResult<Json<ApiResponse<LinkResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let source_document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let link_type = parse_link_type(&payload.link_type)
        .map_err(|e| ApiError::BadRequest(e))?;

    let target = parse_link_target(&payload.target_type, &payload.target_id)
        .map_err(|e| ApiError::BadRequest(e))?;

    let request = CreateLinkRequest {
        source_document_id,
        link_type,
        target,
    };

    let link = ctx.document_service
        .create_link(request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = LinkResponse {
        id: link.id.to_string(),
        source_document_id: link.source_document_id.to_string(),
        link_type: format!("{:?}", link.link_type),
        target: format!("{:?}", link.target),
        created_at: link.created_at.to_rfc3339(),
    };

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// DELETE /api/v1/links/:id - Delete link
async fn delete_link(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<DeleteResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let link_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid link ID: {}", e)))?;

    ctx.document_service
        .delete_link(&link_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let duration = start.elapsed();
    let response = ApiResponse::success(
        DeleteResponse {
            id,
            deleted: true,
        },
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

// =============================================================================
// Version Handlers
// =============================================================================

/// GET /api/v1/documents/{id}/versions - List versions
async fn list_versions(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiResponse<Vec<VersionResponse>>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let versions = ctx.document_service
        .get_document_versions(&document_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<VersionResponse> = versions
        .into_iter()
        .map(|v| VersionResponse {
            id: v.id.to_string(),
            document_id: v.document_id.to_string(),
            version: v.version,
            author: v.author,
            message: v.message,
            created_at: v.created_at.to_rfc3339(),
        })
        .collect();

    let duration = start.elapsed();
    let response = ApiResponse::success(
        items,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

/// POST /api/v1/documents/{id}/versions - Create version
async fn create_version(
    auth_user: AuthUser,
    State(ctx): State<DocumentContext>,
    Path(id): Path<String>,
    Json(payload): Json<CreateVersionPayload>,
) -> ApiResult<Json<ApiResponse<VersionResponse>>> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start = Instant::now();

    let document_id = CortexId::from_str(&id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid document ID: {}", e)))?;

    let request = CreateVersionRequest {
        version: payload.version,
        author: payload.author.unwrap_or_else(|| auth_user.email.clone()),
        message: payload.message,
    };

    let version = ctx.document_service
        .create_version(&document_id, request)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response_data = VersionResponse {
        id: version.id.to_string(),
        document_id: version.document_id.to_string(),
        version: version.version,
        author: version.author,
        message: version.message,
        created_at: version.created_at.to_rfc3339(),
    };

    let duration = start.elapsed();
    let response = ApiResponse::success(
        response_data,
        request_id,
        duration.as_millis() as u64,
    );

    Ok(Json(response))
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
struct ListDocumentsQuery {
    status: Option<String>,
    doc_type: Option<String>,
    parent_id: Option<String>,
    workspace_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchDocumentsQuery {
    q: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CreateDocumentPayload {
    title: String,
    content: String,
    doc_type: Option<String>,
    description: Option<String>,
    parent_id: Option<String>,
    tags: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    language: Option<String>,
    workspace_id: Option<String>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct UpdateDocumentPayload {
    title: Option<String>,
    content: Option<String>,
    description: Option<String>,
    doc_type: Option<String>,
    tags: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct CreateSectionPayload {
    title: String,
    content: String,
    level: u32,
    parent_section_id: Option<String>,
    order: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct UpdateSectionPayload {
    title: Option<String>,
    content: Option<String>,
    order: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct CreateLinkPayload {
    link_type: String,
    target_type: String,
    target_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateVersionPayload {
    version: String,
    author: Option<String>,
    message: String,
}

#[derive(Debug, Serialize)]
struct DocumentResponseItem {
    id: String,
    title: String,
    slug: String,
    doc_type: String,
    status: String,
    author: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct DocumentResponse {
    id: String,
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

#[derive(Debug, Serialize)]
struct SectionResponse {
    id: String,
    document_id: String,
    title: String,
    content: String,
    level: u32,
    order: i32,
    parent_section_id: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct LinkResponse {
    id: String,
    source_document_id: String,
    link_type: String,
    target: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    id: String,
    document_id: String,
    version: String,
    author: String,
    message: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct DeleteResponse {
    id: String,
    deleted: bool,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn parse_status(s: &str) -> Result<cortex_core::DocumentStatus, String> {
    use cortex_core::DocumentStatus;
    match s.to_lowercase().as_str() {
        "draft" => Ok(DocumentStatus::Draft),
        "review" => Ok(DocumentStatus::Review),
        "published" => Ok(DocumentStatus::Published),
        "archived" => Ok(DocumentStatus::Archived),
        _ => Err(format!("Invalid status: {}", s)),
    }
}

fn parse_doc_type(s: &str) -> Result<cortex_core::DocumentType, String> {
    use cortex_core::DocumentType;
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
        _ => Err(format!("Invalid doc_type: {}", s)),
    }
}

fn parse_link_type(s: &str) -> Result<cortex_core::LinkType, String> {
    use cortex_core::LinkType;
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
        _ => Err(format!("Invalid link_type: {}", s)),
    }
}

fn parse_link_target(target_type: &str, target_id: &str) -> Result<cortex_core::LinkTarget, String> {
    use cortex_core::LinkTarget;
    match target_type.to_lowercase().as_str() {
        "document" => {
            let document_id = CortexId::from_str(target_id)
                .map_err(|e| format!("Invalid document ID: {}", e))?;
            Ok(LinkTarget::Document {
                document_id,
                section_id: None,
            })
        }
        "codeunit" | "code_unit" => {
            let code_unit_id = CortexId::from_str(target_id)
                .map_err(|e| format!("Invalid code unit ID: {}", e))?;
            Ok(LinkTarget::CodeUnit { code_unit_id })
        }
        "external" => Ok(LinkTarget::External {
            url: target_id.to_string(),
        }),
        "file" => Ok(LinkTarget::File {
            path: target_id.to_string(),
        }),
        _ => Err(format!("Invalid target_type: {}", target_type)),
    }
}
