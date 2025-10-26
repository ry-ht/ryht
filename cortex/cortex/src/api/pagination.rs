//! Pagination and HATEOAS link generation helpers

use super::types::{CursorData, HateoasLinks, PaginationInfo};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Encode cursor data to base64 string
pub fn encode_cursor(data: &CursorData) -> Result<String, String> {
    let json = serde_json::to_string(data)
        .map_err(|e| format!("Failed to serialize cursor: {}", e))?;
    Ok(BASE64.encode(json.as_bytes()))
}

/// Decode cursor string to cursor data
pub fn decode_cursor(cursor: &str) -> Result<CursorData, String> {
    let bytes = BASE64
        .decode(cursor.as_bytes())
        .map_err(|e| format!("Failed to decode cursor: {}", e))?;
    let json = String::from_utf8(bytes)
        .map_err(|e| format!("Invalid cursor encoding: {}", e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse cursor: {}", e))
}

/// Build pagination info for a list response
pub fn build_pagination_info(
    items_count: usize,
    limit: usize,
    total: Option<usize>,
    next_cursor: Option<String>,
) -> PaginationInfo {
    PaginationInfo {
        cursor: next_cursor.clone(),
        has_more: next_cursor.is_some(),
        total,
        count: items_count,
        limit,
    }
}

/// Generate next cursor for pagination
pub fn generate_next_cursor(
    last_id: String,
    last_timestamp: DateTime<Utc>,
    offset: usize,
) -> Option<String> {
    let cursor_data = CursorData {
        last_id,
        last_timestamp,
        offset,
    };
    encode_cursor(&cursor_data).ok()
}

/// HATEOAS link builder
pub struct LinkBuilder {
    base_url: String,
}

impl LinkBuilder {
    /// Create a new link builder with base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// Create a link builder from request path
    pub fn from_path(path: impl AsRef<str>) -> Self {
        // Extract path without query parameters
        let path = path.as_ref();
        let clean_path = path.split('?').next().unwrap_or(path);
        Self::new(format!("/api/v1{}", clean_path))
    }

    /// Build HATEOAS links for a list endpoint
    pub fn build_list_links(
        &self,
        cursor: Option<&str>,
        next_cursor: Option<&str>,
        limit: usize,
    ) -> HateoasLinks {
        let self_link = if let Some(c) = cursor {
            format!("{}?cursor={}&limit={}", self.base_url, c, limit)
        } else {
            format!("{}?limit={}", self.base_url, limit)
        };

        let next = next_cursor.map(|c| {
            format!("{}?cursor={}&limit={}", self.base_url, c, limit)
        });

        HateoasLinks {
            self_link,
            next,
            prev: None, // Cursor-based pagination typically doesn't support prev
            related: None,
        }
    }

    /// Build HATEOAS links for a single resource
    pub fn build_resource_links(
        &self,
        resource_id: &str,
        related: Option<HashMap<String, String>>,
    ) -> HateoasLinks {
        HateoasLinks {
            self_link: format!("{}/{}", self.base_url, resource_id),
            next: None,
            prev: None,
            related,
        }
    }

    /// Build workspace-specific links
    pub fn build_workspace_links(workspace_id: &str) -> HateoasLinks {
        let mut related = HashMap::new();
        related.insert(
            "files".to_string(),
            format!("/api/v1/workspaces/{}/files", workspace_id),
        );
        related.insert(
            "units".to_string(),
            format!("/api/v1/workspaces/{}/units", workspace_id),
        );
        related.insert(
            "dependencies".to_string(),
            format!("/api/v1/workspaces/{}/dependencies", workspace_id),
        );
        related.insert(
            "tree".to_string(),
            format!("/api/v1/workspaces/{}/tree", workspace_id),
        );
        related.insert(
            "sync".to_string(),
            format!("/api/v1/workspaces/{}/sync", workspace_id),
        );

        HateoasLinks {
            self_link: format!("/api/v1/workspaces/{}", workspace_id),
            next: None,
            prev: None,
            related: Some(related),
        }
    }

    /// Build file-specific links
    pub fn build_file_links(file_id: &str, workspace_id: Option<&str>) -> HateoasLinks {
        let mut related = HashMap::new();

        if let Some(ws_id) = workspace_id {
            related.insert(
                "workspace".to_string(),
                format!("/api/v1/workspaces/{}", ws_id),
            );
        }

        HateoasLinks {
            self_link: format!("/api/v1/files/{}", file_id),
            next: None,
            prev: None,
            related: if related.is_empty() { None } else { Some(related) },
        }
    }

    /// Build session-specific links
    pub fn build_session_links(session_id: &str, workspace_id: Option<&str>) -> HateoasLinks {
        let mut related = HashMap::new();

        if let Some(ws_id) = workspace_id {
            related.insert(
                "workspace".to_string(),
                format!("/api/v1/workspaces/{}", ws_id),
            );
            related.insert(
                "files".to_string(),
                format!("/api/v1/workspaces/{}/files?session_id={}", ws_id, session_id),
            );
        }

        HateoasLinks {
            self_link: format!("/api/v1/sessions/{}", session_id),
            next: None,
            prev: None,
            related: if related.is_empty() { None } else { Some(related) },
        }
    }

    /// Build code unit-specific links
    pub fn build_unit_links(unit_id: &str, workspace_id: Option<&str>) -> HateoasLinks {
        let mut related = HashMap::new();

        if let Some(ws_id) = workspace_id {
            related.insert(
                "workspace".to_string(),
                format!("/api/v1/workspaces/{}", ws_id),
            );
        }

        related.insert(
            "references".to_string(),
            format!("/api/v1/search/references/{}", unit_id),
        );

        HateoasLinks {
            self_link: format!("/api/v1/units/{}", unit_id),
            next: None,
            prev: None,
            related: Some(related),
        }
    }

    /// Build memory episode-specific links
    pub fn build_episode_links(episode_id: &str) -> HateoasLinks {
        HateoasLinks {
            self_link: format!("/api/v1/memory/episodes/{}", episode_id),
            next: None,
            prev: None,
            related: None,
        }
    }

    /// Build task-specific links
    pub fn build_task_links(task_id: &str, workspace_id: Option<&str>) -> HateoasLinks {
        let mut related = HashMap::new();

        if let Some(ws_id) = workspace_id {
            related.insert(
                "workspace".to_string(),
                format!("/api/v1/workspaces/{}", ws_id),
            );
        }

        HateoasLinks {
            self_link: format!("/api/v1/tasks/{}", task_id),
            next: None,
            prev: None,
            related: if related.is_empty() { None } else { Some(related) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encoding_decoding() {
        let cursor_data = CursorData {
            last_id: "test-id-123".to_string(),
            last_timestamp: Utc::now(),
            offset: 20,
        };

        let encoded = encode_cursor(&cursor_data).unwrap();
        let decoded = decode_cursor(&encoded).unwrap();

        assert_eq!(decoded.last_id, cursor_data.last_id);
        assert_eq!(decoded.offset, cursor_data.offset);
    }

    #[test]
    fn test_build_pagination_info() {
        let pagination = build_pagination_info(20, 20, Some(100), Some("cursor123".to_string()));

        assert_eq!(pagination.count, 20);
        assert_eq!(pagination.limit, 20);
        assert_eq!(pagination.total, Some(100));
        assert!(pagination.has_more);
        assert!(pagination.cursor.is_some());
    }

    #[test]
    fn test_link_builder() {
        let builder = LinkBuilder::new("/api/v1/workspaces");
        let links = builder.build_list_links(None, Some("next-cursor"), 20);

        assert_eq!(links.self_link, "/api/v1/workspaces?limit=20");
        assert!(links.next.is_some());
        assert!(links.next.unwrap().contains("next-cursor"));
    }

    #[test]
    fn test_workspace_links() {
        let links = LinkBuilder::build_workspace_links("ws-123");

        assert_eq!(links.self_link, "/api/v1/workspaces/ws-123");
        assert!(links.related.is_some());

        let related = links.related.unwrap();
        assert!(related.contains_key("files"));
        assert!(related.contains_key("units"));
        assert!(related.contains_key("dependencies"));
    }
}
