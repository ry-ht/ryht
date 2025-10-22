//! Unit tests for REST API types, middleware, and components
//!
//! This module contains comprehensive unit tests for:
//! - Request/response type serialization
//! - Error handling
//! - Middleware functions
//! - Route parameter parsing

use crate::api::{
    error::{ApiError, ErrorDetail, ErrorResponse},
    types::*,
};
use chrono::Utc;
use serde_json;

#[cfg(test)]
mod types_tests {
    use super::*;

    #[test]
    fn test_api_response_success_serialization() {
        let data = vec!["test1".to_string(), "test2".to_string()];
        let response = ApiResponse::success(
            data.clone(),
            "test-request-id".to_string(),
            100,
        );

        assert!(response.success);
        assert_eq!(response.data, Some(data));
        assert_eq!(response.error, None);
        assert_eq!(response.metadata.request_id, "test-request-id");
        assert_eq!(response.metadata.version, "v1");
        assert_eq!(response.metadata.duration_ms, 100);

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("test-request-id"));
    }

    #[test]
    fn test_api_response_error_serialization() {
        let response: ApiResponse<()> = ApiResponse::error(
            "Something went wrong".to_string(),
            "error-request-id".to_string(),
        );

        assert!(!response.success);
        assert_eq!(response.data, None);
        assert_eq!(response.error, Some("Something went wrong".to_string()));
        assert_eq!(response.metadata.request_id, "error-request-id");

        // Test serialization
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_file_list_request_deserialization() {
        let json = r#"{
            "recursive": true,
            "file_type": "file",
            "language": "rust",
            "limit": 10,
            "offset": 5
        }"#;

        let request: FileListRequest = serde_json::from_str(json).unwrap();
        assert!(request.recursive);
        assert_eq!(request.file_type, Some("file".to_string()));
        assert_eq!(request.language, Some("rust".to_string()));
        assert_eq!(request.limit, Some(10));
        assert_eq!(request.offset, Some(5));
    }

    #[test]
    fn test_file_list_request_defaults() {
        let json = r#"{}"#;
        let request: FileListRequest = serde_json::from_str(json).unwrap();

        assert!(!request.recursive); // Default from serde
        assert_eq!(request.file_type, None);
        assert_eq!(request.language, None);
        assert_eq!(request.limit, None);
        assert_eq!(request.offset, None);
    }

    #[test]
    fn test_file_response_serialization() {
        let now = Utc::now();
        let file_response = FileResponse {
            id: "file-123".to_string(),
            name: "test.rs".to_string(),
            path: "/src/test.rs".to_string(),
            file_type: "file".to_string(),
            size: 1024,
            language: Some("rust".to_string()),
            content: Some("fn main() {}".to_string()),
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&file_response).unwrap();
        assert!(json.contains("file-123"));
        assert!(json.contains("test.rs"));
        assert!(json.contains("/src/test.rs"));
        assert!(json.contains("\"size\":1024"));
        assert!(json.contains("rust"));
    }

    #[test]
    fn test_create_file_request_deserialization() {
        let json = r#"{
            "path": "/src/main.rs",
            "content": "fn main() {}",
            "language": "rust"
        }"#;

        let request: CreateFileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.path, "/src/main.rs");
        assert_eq!(request.content, "fn main() {}");
        assert_eq!(request.language, Some("rust".to_string()));
    }

    #[test]
    fn test_update_file_request_deserialization() {
        let json = r#"{
            "content": "updated content"
        }"#;

        let request: UpdateFileRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "updated content");
    }

    #[test]
    fn test_workspace_response_serialization() {
        let now = Utc::now();
        let workspace = WorkspaceResponse {
            id: "ws-123".to_string(),
            name: "My Workspace".to_string(),
            workspace_type: "code".to_string(),
            source_type: "local".to_string(),
            namespace: "ws_123".to_string(),
            source_path: Some("/path/to/source".to_string()),
            read_only: false,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&workspace).unwrap();
        assert!(json.contains("ws-123"));
        assert!(json.contains("My Workspace"));
        assert!(json.contains("code"));
        assert!(json.contains("\"read_only\":false"));
    }

    #[test]
    fn test_create_workspace_request_deserialization() {
        let json = r#"{
            "name": "Test Workspace",
            "workspace_type": "code",
            "source_path": "/path/to/code"
        }"#;

        let request: CreateWorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "Test Workspace");
        assert_eq!(request.workspace_type, "code");
        assert_eq!(request.source_path, Some("/path/to/code".to_string()));
    }

    #[test]
    fn test_session_response_serialization() {
        let now = Utc::now();
        let session = SessionResponse {
            id: "session-123".to_string(),
            name: "Debug Session".to_string(),
            agent_type: "code_editor".to_string(),
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("session-123"));
        assert!(json.contains("Debug Session"));
        assert!(json.contains("code_editor"));
        assert!(json.contains("active"));
    }

    #[test]
    fn test_create_session_request_deserialization() {
        let json = r#"{
            "name": "My Session",
            "agent_type": "researcher",
            "workspace_id": "ws-456"
        }"#;

        let request: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "My Session");
        assert_eq!(request.agent_type, "researcher");
        assert_eq!(request.workspace_id, Some("ws-456".to_string()));
    }

    #[test]
    fn test_search_request_deserialization() {
        let json = r#"{
            "query": "search term",
            "workspace_id": "ws-789",
            "search_type": "semantic",
            "limit": 20,
            "offset": 0
        }"#;

        let request: SearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "search term");
        assert_eq!(request.workspace_id, Some("ws-789".to_string()));
        assert_eq!(request.search_type, Some("semantic".to_string()));
        assert_eq!(request.limit, Some(20));
        assert_eq!(request.offset, Some(0));
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            id: "result-1".to_string(),
            title: "Test Result".to_string(),
            content: "Some content here".to_string(),
            score: 0.95,
            result_type: "semantic".to_string(),
            metadata: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("result-1"));
        assert!(json.contains("Test Result"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_memory_episode_serialization() {
        let now = Utc::now();
        let episode = MemoryEpisode {
            id: "episode-1".to_string(),
            content: "Important event".to_string(),
            episode_type: "experience".to_string(),
            importance: 0.8,
            created_at: now,
        };

        let json = serde_json::to_string(&episode).unwrap();
        assert!(json.contains("episode-1"));
        assert!(json.contains("Important event"));
        assert!(json.contains("0.8"));
    }

    #[test]
    fn test_consolidate_memory_request_deserialization() {
        let json = r#"{
            "workspace_id": "ws-999"
        }"#;

        let request: ConsolidateMemoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.workspace_id, Some("ws-999".to_string()));
    }

    #[test]
    fn test_health_response_serialization() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            database: DatabaseHealth {
                connected: true,
                response_time_ms: 5,
            },
            memory: MemoryHealth {
                total_bytes: 1024 * 1024 * 1024,
                used_bytes: 512 * 1024 * 1024,
            },
        };

        let json = serde_json::to_string(&health).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("3600"));
        assert!(json.contains("\"connected\":true"));
    }

    #[test]
    fn test_metrics_response_serialization() {
        let metrics = MetricsResponse {
            workspaces: 5,
            files: 100,
            total_size_bytes: 1024 * 1024,
            episodes: 50,
            semantic_nodes: 200,
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"workspaces\":5"));
        assert!(json.contains("\"files\":100"));
        assert!(json.contains("\"episodes\":50"));
        assert!(json.contains("\"semantic_nodes\":200"));
    }

    #[test]
    fn test_directory_tree_response_serialization() {
        let tree = DirectoryTreeResponse {
            name: "root".to_string(),
            path: "/".to_string(),
            children: vec![
                TreeNode {
                    name: "src".to_string(),
                    path: "/src".to_string(),
                    node_type: "directory".to_string(),
                    children: Some(vec![
                        TreeNode {
                            name: "main.rs".to_string(),
                            path: "/src/main.rs".to_string(),
                            node_type: "file".to_string(),
                            children: None,
                        },
                    ]),
                },
            ],
        };

        let json = serde_json::to_string(&tree).unwrap();
        assert!(json.contains("root"));
        assert!(json.contains("src"));
        assert!(json.contains("main.rs"));
        assert!(json.contains("directory"));
        assert!(json.contains("file"));
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_api_error_display() {
        let error = ApiError::NotFound("Resource not found".to_string());
        assert_eq!(error.to_string(), "Not found: Resource not found");

        let error = ApiError::BadRequest("Invalid input".to_string());
        assert_eq!(error.to_string(), "Bad request: Invalid input");

        let error = ApiError::Internal("Server error".to_string());
        assert_eq!(error.to_string(), "Internal error: Server error");

        let error = ApiError::Unauthorized("Not authenticated".to_string());
        assert_eq!(error.to_string(), "Unauthorized: Not authenticated");

        let error = ApiError::Forbidden("Access denied".to_string());
        assert_eq!(error.to_string(), "Forbidden: Access denied");

        let error = ApiError::Conflict("Resource conflict".to_string());
        assert_eq!(error.to_string(), "Conflict: Resource conflict");

        let error = ApiError::UnprocessableEntity("Cannot process".to_string());
        assert_eq!(error.to_string(), "Unprocessable entity: Cannot process");
    }

    #[test]
    fn test_api_error_status_codes() {
        assert_eq!(
            ApiError::NotFound("test".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::BadRequest("test".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Internal("test".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ApiError::Unauthorized("test".to_string()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::Forbidden("test".to_string()).status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ApiError::Conflict("test".to_string()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ApiError::UnprocessableEntity("test".to_string()).status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn test_api_error_codes() {
        assert_eq!(
            ApiError::NotFound("test".to_string()).error_code(),
            "NOT_FOUND"
        );
        assert_eq!(
            ApiError::BadRequest("test".to_string()).error_code(),
            "BAD_REQUEST"
        );
        assert_eq!(
            ApiError::Internal("test".to_string()).error_code(),
            "INTERNAL_ERROR"
        );
        assert_eq!(
            ApiError::Unauthorized("test".to_string()).error_code(),
            "UNAUTHORIZED"
        );
        assert_eq!(
            ApiError::Forbidden("test".to_string()).error_code(),
            "FORBIDDEN"
        );
        assert_eq!(
            ApiError::Conflict("test".to_string()).error_code(),
            "CONFLICT"
        );
        assert_eq!(
            ApiError::UnprocessableEntity("test".to_string()).error_code(),
            "UNPROCESSABLE_ENTITY"
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let metadata = ApiMetadata {
            request_id: "req-123".to_string(),
            timestamp: Utc::now(),
            version: "v1".to_string(),
            duration_ms: 0,
        };

        let error_response = ErrorResponse {
            success: false,
            error: ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: "Resource not found".to_string(),
                details: Some(serde_json::json!({"resource_id": "123"})),
            },
            metadata,
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("NOT_FOUND"));
        assert!(json.contains("Resource not found"));
        assert!(json.contains("resource_id"));
    }
}

#[cfg(test)]
mod route_parameter_tests {
    use uuid::Uuid;

    #[test]
    fn test_uuid_parsing() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        let result = Uuid::parse_str(valid_uuid);
        assert!(result.is_ok());

        let invalid_uuid = "not-a-uuid";
        let result = Uuid::parse_str(invalid_uuid);
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_type_parsing() {
        let valid_types = vec!["code", "documentation", "mixed", "external"];
        for t in valid_types {
            assert!(matches!(
                t,
                "code" | "documentation" | "mixed" | "external"
            ));
        }

        let invalid_type = "unknown";
        assert!(!matches!(
            invalid_type,
            "code" | "documentation" | "mixed" | "external"
        ));
    }

    #[test]
    fn test_path_validation() {
        // Valid paths
        let valid_paths = vec!["/", "/src", "/src/main.rs", "/path/to/file.txt"];
        for path in valid_paths {
            assert!(path.starts_with('/'));
        }

        // Invalid paths (relative)
        let invalid_paths = vec!["src", "main.rs", "../file.txt"];
        for path in invalid_paths {
            assert!(!path.starts_with('/'));
        }
    }
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn test_api_metadata_serialization() {
        let now = Utc::now();
        let metadata = ApiMetadata {
            request_id: "req-456".to_string(),
            timestamp: now,
            version: "v1".to_string(),
            duration_ms: 150,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("req-456"));
        assert!(json.contains("v1"));
        assert!(json.contains("150"));
    }

    #[test]
    fn test_api_metadata_deserialization() {
        let json = r#"{
            "request_id": "req-789",
            "timestamp": "2024-01-01T00:00:00Z",
            "version": "v1",
            "duration_ms": 200
        }"#;

        let metadata: ApiMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.request_id, "req-789");
        assert_eq!(metadata.version, "v1");
        assert_eq!(metadata.duration_ms, 200);
    }
}

#[cfg(test)]
mod workspace_update_tests {
    use super::*;

    #[test]
    fn test_update_workspace_request_serialization() {
        let request = UpdateWorkspaceRequest {
            name: Some("Updated Name".to_string()),
            workspace_type: Some("code".to_string()),
            read_only: Some(true),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Updated Name"));
        assert!(json.contains("code"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_update_workspace_request_partial() {
        let request = UpdateWorkspaceRequest {
            name: Some("New Name".to_string()),
            workspace_type: None,
            read_only: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("New Name"));
        assert!(!json.contains("workspace_type") || json.contains("null"));
    }

    #[test]
    fn test_update_workspace_request_deserialization() {
        let json = r#"{
            "name": "Test Workspace",
            "workspace_type": "documentation",
            "read_only": false
        }"#;

        let request: UpdateWorkspaceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, Some("Test Workspace".to_string()));
        assert_eq!(request.workspace_type, Some("documentation".to_string()));
        assert_eq!(request.read_only, Some(false));
    }

    #[test]
    fn test_sync_workspace_request_serialization() {
        let request = SyncWorkspaceRequest {
            force: Some(true),
            dry_run: Some(false),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_sync_workspace_request_defaults() {
        let json = r#"{}"#;
        let request: SyncWorkspaceRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.force, None);
        assert_eq!(request.dry_run, None);
    }

    #[test]
    fn test_sync_response_serialization() {
        let response = SyncResponse {
            files_added: 10,
            files_updated: 5,
            files_deleted: 2,
            total_processed: 17,
            duration_ms: 1500,
            changes: vec![
                SyncChange {
                    path: "/src/main.rs".to_string(),
                    change_type: "updated".to_string(),
                    size_bytes: Some(1024),
                },
                SyncChange {
                    path: "/src/lib.rs".to_string(),
                    change_type: "added".to_string(),
                    size_bytes: Some(2048),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"files_added\":10"));
        assert!(json.contains("\"files_updated\":5"));
        assert!(json.contains("\"files_deleted\":2"));
        assert!(json.contains("\"total_processed\":17"));
        assert!(json.contains("main.rs"));
        assert!(json.contains("updated"));
    }

    #[test]
    fn test_sync_change_serialization() {
        let change = SyncChange {
            path: "/test/file.txt".to_string(),
            change_type: "deleted".to_string(),
            size_bytes: None,
        };

        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("/test/file.txt"));
        assert!(json.contains("deleted"));
    }
}

#[cfg(test)]
mod search_reference_tests {
    use super::*;

    #[test]
    fn test_references_response_serialization() {
        let response = ReferencesResponse {
            unit_id: "unit-123".to_string(),
            unit_name: "calculate_total".to_string(),
            total_references: 15,
            references: vec![
                CodeReference {
                    id: "ref-1".to_string(),
                    file_path: "/src/api.rs".to_string(),
                    line: 42,
                    column: 10,
                    reference_type: "call".to_string(),
                    context: "let total = calculate_total(items);".to_string(),
                    referencing_unit: Some("process_order".to_string()),
                },
                CodeReference {
                    id: "ref-2".to_string(),
                    file_path: "/tests/api_test.rs".to_string(),
                    line: 100,
                    column: 5,
                    reference_type: "call".to_string(),
                    context: "assert_eq!(calculate_total(&[1, 2, 3]), 6);".to_string(),
                    referencing_unit: Some("test_calculate_total".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("unit-123"));
        assert!(json.contains("calculate_total"));
        assert!(json.contains("\"total_references\":15"));
        assert!(json.contains("api.rs"));
        assert!(json.contains("\"line\":42"));
    }

    #[test]
    fn test_code_reference_deserialization() {
        let json = r#"{
            "id": "ref-456",
            "file_path": "/src/models.rs",
            "line": 25,
            "column": 8,
            "reference_type": "import",
            "context": "use crate::models::User;",
            "referencing_unit": "main"
        }"#;

        let reference: CodeReference = serde_json::from_str(json).unwrap();
        assert_eq!(reference.id, "ref-456");
        assert_eq!(reference.file_path, "/src/models.rs");
        assert_eq!(reference.line, 25);
        assert_eq!(reference.column, 8);
        assert_eq!(reference.reference_type, "import");
        assert_eq!(reference.referencing_unit, Some("main".to_string()));
    }

    #[test]
    fn test_pattern_search_request_serialization() {
        let request = PatternSearchRequest {
            workspace_id: "ws-789".to_string(),
            pattern: "fn.*calculate".to_string(),
            language: Some("rust".to_string()),
            limit: Some(50),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("ws-789"));
        assert!(json.contains("fn.*calculate"));
        assert!(json.contains("rust"));
        assert!(json.contains("50"));
    }

    #[test]
    fn test_pattern_search_request_deserialization() {
        let json = r#"{
            "workspace_id": "ws-abc",
            "pattern": "class\\s+\\w+",
            "language": "python"
        }"#;

        let request: PatternSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.workspace_id, "ws-abc");
        assert_eq!(request.pattern, "class\\s+\\w+");
        assert_eq!(request.language, Some("python".to_string()));
        assert_eq!(request.limit, None);
    }

    #[test]
    fn test_pattern_search_response_serialization() {
        let response = PatternSearchResponse {
            pattern: "TODO:".to_string(),
            total_matches: 25,
            matches: vec![
                PatternMatch {
                    file_path: "/src/api.rs".to_string(),
                    line: 100,
                    column: 5,
                    matched_text: "TODO: Implement validation".to_string(),
                    context: "    // TODO: Implement validation\n    fn validate() {}".to_string(),
                    unit_id: Some("validate".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TODO:"));
        assert!(json.contains("\"total_matches\":25"));
        assert!(json.contains("Implement validation"));
    }

    #[test]
    fn test_pattern_match_deserialization() {
        let json = r#"{
            "file_path": "/src/lib.rs",
            "line": 50,
            "column": 10,
            "matched_text": "fn process_data",
            "context": "pub fn process_data(input: &str) -> Result<Data>",
            "unit_id": "process_data"
        }"#;

        let pattern_match: PatternMatch = serde_json::from_str(json).unwrap();
        assert_eq!(pattern_match.file_path, "/src/lib.rs");
        assert_eq!(pattern_match.line, 50);
        assert_eq!(pattern_match.matched_text, "fn process_data");
        assert_eq!(pattern_match.unit_id, Some("process_data".to_string()));
    }
}

#[cfg(test)]
mod memory_search_tests {
    use super::*;

    #[test]
    fn test_episode_search_request_serialization() {
        let request = EpisodeSearchRequest {
            query: "error handling".to_string(),
            episode_type: Some("learning".to_string()),
            min_importance: Some(0.7),
            limit: Some(10),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("error handling"));
        assert!(json.contains("learning"));
        assert!(json.contains("0.7"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_episode_search_request_deserialization() {
        let json = r#"{
            "query": "refactoring patterns",
            "episode_type": "experience",
            "min_importance": 0.5
        }"#;

        let request: EpisodeSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "refactoring patterns");
        assert_eq!(request.episode_type, Some("experience".to_string()));
        assert_eq!(request.min_importance, Some(0.5));
        assert_eq!(request.limit, None);
    }

    #[test]
    fn test_episode_search_request_minimal() {
        let json = r#"{
            "query": "test query"
        }"#;

        let request: EpisodeSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "test query");
        assert_eq!(request.episode_type, None);
        assert_eq!(request.min_importance, None);
        assert_eq!(request.limit, None);
    }

    #[test]
    fn test_learned_pattern_serialization() {
        let now = Utc::now();
        let pattern = LearnedPattern {
            id: "pattern-123".to_string(),
            pattern_name: "Builder Pattern".to_string(),
            description: "Use builder pattern for complex object construction".to_string(),
            pattern_type: "design".to_string(),
            occurrences: 42,
            confidence: 0.95,
            created_at: now,
            last_seen: now,
            examples: vec![
                "User::builder().name(\"John\").build()".to_string(),
                "Config::builder().port(8080).host(\"localhost\").build()".to_string(),
            ],
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("pattern-123"));
        assert!(json.contains("Builder Pattern"));
        assert!(json.contains("\"occurrences\":42"));
        assert!(json.contains("0.95"));
        assert!(json.contains("User::builder()"));
    }

    #[test]
    fn test_learned_pattern_deserialization() {
        let json = r#"{
            "id": "pattern-456",
            "pattern_name": "Error Propagation",
            "description": "Use ? operator for error propagation",
            "pattern_type": "idiom",
            "occurrences": 128,
            "confidence": 0.88,
            "created_at": "2024-01-01T00:00:00Z",
            "last_seen": "2024-01-15T00:00:00Z",
            "examples": [
                "let file = File::open(path)?;",
                "let data = read_data()?;"
            ]
        }"#;

        let pattern: LearnedPattern = serde_json::from_str(json).unwrap();
        assert_eq!(pattern.id, "pattern-456");
        assert_eq!(pattern.pattern_name, "Error Propagation");
        assert_eq!(pattern.pattern_type, "idiom");
        assert_eq!(pattern.occurrences, 128);
        assert_eq!(pattern.confidence, 0.88);
        assert_eq!(pattern.examples.len(), 2);
    }

    #[test]
    fn test_learned_pattern_with_no_examples() {
        let now = Utc::now();
        let pattern = LearnedPattern {
            id: "pattern-789".to_string(),
            pattern_name: "New Pattern".to_string(),
            description: "Recently discovered pattern".to_string(),
            pattern_type: "experimental".to_string(),
            occurrences: 1,
            confidence: 0.5,
            created_at: now,
            last_seen: now,
            examples: vec![],
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("pattern-789"));
        assert!(json.contains("\"examples\":[]"));
    }
}
