//! Type conversions between API, Service, and MCP layers
//!
//! This module provides traits and implementations to convert between different
//! representations of the same data across the different layers of the application.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

// ============================================================================
// Conversion Traits
// ============================================================================

/// Trait for converting from service types to API response types
pub trait ToApiResponse<T> {
    fn to_api_response(self) -> T;
}

/// Trait for converting from API request types to service types
pub trait ToServiceRequest<T> {
    fn to_service_request(self) -> T;
}

/// Trait for converting from MCP input types to service types
pub trait ToServiceInput<T> {
    fn to_service_input(self) -> T;
}

/// Trait for converting from service types to MCP output types
pub trait ToMcpOutput<T> {
    fn to_mcp_output(self) -> T;
}

// ============================================================================
// Workspace Conversions
// ============================================================================

use crate::api::types::{WorkspaceResponse, CreateWorkspaceRequest as ApiCreateRequest};
use crate::services::workspace::{WorkspaceDetails, CreateWorkspaceRequest as ServiceCreateRequest};

impl ToApiResponse<WorkspaceResponse> for WorkspaceDetails {
    fn to_api_response(self) -> WorkspaceResponse {
        WorkspaceResponse {
            id: self.id,
            name: self.name,
            workspace_type: self.workspace_type,
            source_type: self.source_type,
            namespace: self.namespace,
            source_path: self.source_path,
            read_only: self.read_only,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl ToServiceRequest<ServiceCreateRequest> for ApiCreateRequest {
    fn to_service_request(self) -> ServiceCreateRequest {
        ServiceCreateRequest {
            name: self.name,
            workspace_type: self.workspace_type,
            source_path: self.source_path,
            read_only: Some(false), // Default to read-write
        }
    }
}

// ============================================================================
// Build Conversions
// ============================================================================

use crate::api::types::{BuildResponse, BuildRequest};
use crate::services::build::{BuildJob, BuildConfig};

impl ToApiResponse<BuildResponse> for BuildJob {
    fn to_api_response(self) -> BuildResponse {
        BuildResponse {
            job_id: self.id,
            workspace_id: self.workspace_id.to_string(),
            build_type: self.build_type,
            status: format!("{:?}", self.status).to_lowercase(),
            started_at: self.started_at,
        }
    }
}

impl ToServiceRequest<BuildConfig> for BuildRequest {
    fn to_service_request(self) -> BuildConfig {
        BuildConfig {
            build_type: self.build_type,
            target: None,
            features: None,
        }
    }
}

// ============================================================================
// Session Conversions
// ============================================================================

use crate::api::types::{SessionResponse, CreateSessionRequest as ApiCreateSessionRequest};
use crate::services::sessions::{WorkSession, SessionMetadata};

impl ToApiResponse<SessionResponse> for WorkSession {
    fn to_api_response(self) -> SessionResponse {
        SessionResponse {
            id: self.id.to_string(),
            name: self.name,
            agent_type: self.agent_type,
            status: format!("{:?}", self.status).to_lowercase(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

// ============================================================================
// Auth Conversions
// ============================================================================

use crate::api::routes::auth::{UserInfo, ApiKeyResponse};
use crate::services::auth::{User, ApiKey};

impl ToApiResponse<UserInfo> for User {
    fn to_api_response(self) -> UserInfo {
        UserInfo {
            id: self.id,
            email: self.email,
            roles: self.roles,
            created_at: self.created_at,
        }
    }
}

impl ToApiResponse<ApiKeyResponse> for ApiKey {
    fn to_api_response(self) -> ApiKeyResponse {
        ApiKeyResponse {
            key_id: self.id,
            api_key: self.key,
            name: self.name,
            scopes: self.scopes,
            expires_at: self.expires_at,
            created_at: self.created_at,
        }
    }
}

// ============================================================================
// Search Conversions
// ============================================================================

use crate::api::types::SearchResult;
use crate::services::search::{SearchResult as ServiceSearchResult};

impl ToApiResponse<SearchResult> for ServiceSearchResult {
    fn to_api_response(self) -> SearchResult {
        SearchResult {
            id: self.id,
            title: self.title,
            content: self.content,
            score: self.score as f64,
            result_type: self.result_type,
            metadata: serde_json::to_value(self.metadata).unwrap_or(serde_json::Value::Null),
        }
    }
}

// ============================================================================
// VFS Conversions
// ============================================================================

use crate::api::types::{FileResponse, DirectoryTreeResponse};

// Note: VfsService doesn't export FileMetadata or DirectoryEntry types directly
// These conversions would need to be implemented when those types are properly exposed

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a vector of service types to API responses
pub fn convert_vec<S, T>(items: Vec<S>) -> Vec<T>
where
    S: ToApiResponse<T>,
{
    items.into_iter().map(|item| item.to_api_response()).collect()
}

/// Convert an Option<Service> to Option<API>
pub fn convert_option<S, T>(item: Option<S>) -> Option<T>
where
    S: ToApiResponse<T>,
{
    item.map(|i| i.to_api_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_conversion() {
        let details = WorkspaceDetails {
            id: "test-id".to_string(),
            name: "test-workspace".to_string(),
            workspace_type: "code".to_string(),
            source_type: "local".to_string(),
            namespace: "main".to_string(),
            source_path: Some("/path/to/workspace".to_string()),
            read_only: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: WorkspaceResponse = details.to_api_response();
        assert_eq!(response.id, "test-id");
        assert_eq!(response.name, "test-workspace");
    }

    #[test]
    fn test_vector_conversion() {
        let items = vec![
            WorkspaceDetails {
                id: "1".to_string(),
                name: "ws1".to_string(),
                workspace_type: "code".to_string(),
                source_type: "local".to_string(),
                namespace: "main".to_string(),
                source_path: None,
                read_only: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            WorkspaceDetails {
                id: "2".to_string(),
                name: "ws2".to_string(),
                workspace_type: "docs".to_string(),
                source_type: "local".to_string(),
                namespace: "main".to_string(),
                source_path: None,
                read_only: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let responses: Vec<WorkspaceResponse> = convert_vec(items);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, "1");
        assert_eq!(responses[1].id, "2");
    }
}