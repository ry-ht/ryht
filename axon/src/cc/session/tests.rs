//! Comprehensive tests for the session module.
//!
//! These tests cover caching, write operations, filtering, and management features.

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::cc::core::SessionId;
    use crate::cc::messages::{Message, UserMessage};
    use crate::cc::session::{
        SessionCache, Session, CacheConfig, SessionFilter, SortBy,
        ExportFormat, SessionStats, CreateSessionOptions, SessionInfo,
        SessionMetadata, clear_cache, set_cached_projects, get_cached_projects,
    };
    use crate::Project;
    use chrono::{Duration, Utc};
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a test user message
    fn create_test_user_message(content: &str) -> Message {
        Message::User {
            message: UserMessage {
                content: content.to_string(),
            },
        }
    }

    #[test]
    fn test_cache_basic_lifecycle() {
        let cache = SessionCache::default();

        // Initially empty
        assert!(cache.is_empty());
        assert!(cache.get_projects().is_none());

        // Set projects
        let projects = vec![Project::new(
            "test-project".to_string(),
            PathBuf::from("/test"),
            vec![],
        )];
        cache.set_projects(projects.clone());

        // Should be cached
        assert!(!cache.is_empty());
        let cached = cache.get_projects();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        // Clear cache
        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.get_projects().is_none());
    }

    #[test]
    fn test_cache_sessions() {
        let cache = SessionCache::default();

        let sessions = vec![Session::new(
            SessionId::new("session-1"),
            PathBuf::from("/test"),
            Utc::now(),
            Some("Test message".to_string()),
        )];

        // Cache sessions for project
        cache.set_sessions("project-1".to_string(), sessions.clone());

        // Should be cached
        let cached = cache.get_sessions("project-1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);

        // Different project should not be cached
        assert!(cache.get_sessions("project-2").is_none());

        // Clear specific project
        cache.clear_sessions("project-1");
        assert!(cache.get_sessions("project-1").is_none());
    }

    #[test]
    fn test_cache_expiration() {
        use std::thread;
        use std::time::Duration as StdDuration;

        let config = CacheConfig {
            ttl: StdDuration::from_millis(100),
            enabled: true,
        };
        let cache = SessionCache::new(config);

        // Cache some data
        cache.set_projects(vec![]);
        assert!(cache.get_projects().is_some());

        // Wait for expiration
        thread::sleep(StdDuration::from_millis(150));

        // Should be expired now
        assert!(cache.get_projects().is_none());
    }

    #[test]
    fn test_cache_cleanup() {
        use std::thread;
        use std::time::Duration as StdDuration;

        let config = CacheConfig {
            ttl: StdDuration::from_millis(100),
            enabled: true,
        };
        let cache = SessionCache::new(config);

        // Add multiple entries
        cache.set_projects(vec![]);
        cache.set_sessions("project-1".to_string(), vec![]);
        cache.set_sessions("project-2".to_string(), vec![]);

        let (p, s) = cache.len();
        assert_eq!(p, 1);
        assert_eq!(s, 2);

        // Wait for expiration
        thread::sleep(StdDuration::from_millis(150));

        // Cleanup should remove all expired entries
        let removed = cache.cleanup();
        assert_eq!(removed, 3);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig {
            ttl: std::time::Duration::from_secs(300),
            enabled: false,
        };
        let cache = SessionCache::new(config);

        // Try to cache
        cache.set_projects(vec![]);

        // Should not be cached when disabled
        assert!(cache.get_projects().is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn test_session_filter_builder() {
        let now = Utc::now();
        let filter = SessionFilter::new()
            .with_project_id("test-project")
            .with_date_range(Some(now - Duration::days(7)), Some(now))
            .with_content_search("error")
            .with_regex(true)
            .with_case_sensitive(false)
            .with_min_messages(5)
            .with_max_messages(100)
            .with_sort_by(SortBy::CreatedDesc)
            .with_limit(10)
            .with_offset(5);

        assert_eq!(filter.project_id, Some("test-project".to_string()));
        assert!(filter.date_range.is_some());
        assert_eq!(filter.content_search, Some("error".to_string()));
        assert!(filter.regex_search);
        assert!(!filter.case_sensitive);
        assert_eq!(filter.min_messages, Some(5));
        assert_eq!(filter.max_messages, Some(100));
        assert_eq!(filter.sort_by, SortBy::CreatedDesc);
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, Some(5));
    }

    #[test]
    fn test_sort_by_variants() {
        // Test all sort variants exist
        let variants = vec![
            SortBy::CreatedAsc,
            SortBy::CreatedDesc,
            SortBy::ModifiedAsc,
            SortBy::ModifiedDesc,
            SortBy::MessageCountAsc,
            SortBy::MessageCountDesc,
        ];

        // Default should be CreatedDesc
        assert_eq!(SortBy::default(), SortBy::CreatedDesc);

        // All variants should be distinct
        for (i, v1) in variants.iter().enumerate() {
            for (j, v2) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(v1, v2);
                } else {
                    assert_ne!(v1, v2);
                }
            }
        }
    }

    #[test]
    fn test_export_format_variants() {
        let formats = vec![
            ExportFormat::Json,
            ExportFormat::Jsonl,
            ExportFormat::Markdown,
            ExportFormat::Text,
        ];

        // All formats should be distinct
        for (i, f1) in formats.iter().enumerate() {
            for (j, f2) in formats.iter().enumerate() {
                if i == j {
                    assert_eq!(f1, f2);
                } else {
                    assert_ne!(f1, f2);
                }
            }
        }
    }

    #[test]
    fn test_session_stats_structure() {
        let stats = SessionStats {
            session_id: SessionId::new("test-session"),
            message_count: 20,
            user_message_count: 10,
            assistant_message_count: 9,
            tool_use_count: 5,
            tool_result_count: 5,
            created_at: Utc::now(),
            first_message_at: Some(Utc::now()),
            last_message_at: Some(Utc::now()),
            size_bytes: 2048,
            top_tools: vec![
                ("Bash".to_string(), 3),
                ("Read".to_string(), 2),
            ],
        };

        assert_eq!(stats.message_count, 20);
        assert_eq!(stats.user_message_count, 10);
        assert_eq!(stats.assistant_message_count, 9);
        assert_eq!(stats.tool_use_count, 5);
        assert_eq!(stats.top_tools.len(), 2);
        assert_eq!(stats.top_tools[0].0, "Bash");
    }

    #[test]
    fn test_create_session_options_default() {
        let options = CreateSessionOptions::default();

        assert!(options.initial_message.is_none());
        assert!(options.created_at.is_none());
        assert!(!options.overwrite);
    }

    #[test]
    fn test_create_session_options_custom() {
        let message = create_test_user_message("Hello");
        let now = Utc::now();

        let options = CreateSessionOptions {
            initial_message: Some(message),
            created_at: Some(now),
            overwrite: true,
        };

        assert!(options.initial_message.is_some());
        assert!(options.created_at.is_some());
        assert!(options.overwrite);
    }

    #[test]
    fn test_session_info_structure() {
        let session = Session::new(
            SessionId::new("test-session"),
            PathBuf::from("/test"),
            Utc::now(),
            Some("Test".to_string()),
        );

        let info = SessionInfo {
            session: session.clone(),
            message_count: 10,
            last_modified: Some(Utc::now()),
        };

        assert_eq!(info.session.id.as_str(), "test-session");
        assert_eq!(info.message_count, 10);
        assert!(info.last_modified.is_some());
    }

    #[test]
    fn test_global_cache_independence() {
        // Clear global cache
        clear_cache();

        // Set some data
        set_cached_projects(vec![Project::new(
            "global-test".to_string(),
            PathBuf::from("/test"),
            vec![],
        )]);

        // Should be retrievable
        let projects = get_cached_projects();
        assert!(projects.is_some());
        assert_eq!(projects.unwrap()[0].id, "global-test");

        // Clear again
        clear_cache();
        assert!(get_cached_projects().is_none());
    }

    #[test]
    fn test_session_metadata_add_message() {
        let mut metadata = SessionMetadata::new(SessionId::new("test"), Utc::now());

        assert_eq!(metadata.message_count, 0);
        assert!(metadata.first_message.is_none());

        // Add a user message
        let message = create_test_user_message("Hello, world!");
        let timestamp = Utc::now();
        metadata.add_message(&message, timestamp);

        assert_eq!(metadata.message_count, 1);
        assert_eq!(metadata.first_message, Some("Hello, world!".to_string()));
        assert_eq!(metadata.last_updated, timestamp);

        // Add another message
        let message2 = create_test_user_message("Second message");
        let timestamp2 = Utc::now();
        metadata.add_message(&message2, timestamp2);

        assert_eq!(metadata.message_count, 2);
        // First message should remain unchanged
        assert_eq!(metadata.first_message, Some("Hello, world!".to_string()));
        assert_eq!(metadata.last_updated, timestamp2);
    }

    #[test]
    fn test_project_creation() {
        let project = Project::new(
            "test-project".to_string(),
            PathBuf::from("/path/to/project"),
            vec![SessionId::new("session-1"), SessionId::new("session-2")],
        );

        assert_eq!(project.id, "test-project");
        assert_eq!(project.path, PathBuf::from("/path/to/project"));
        assert_eq!(project.sessions.len(), 2);
    }

    #[test]
    fn test_session_creation_and_builder() {
        let session_id = SessionId::new("test-session");
        let created_at = Utc::now();

        let session = Session::new(
            session_id.clone(),
            PathBuf::from("/project"),
            created_at,
            Some("First message".to_string()),
        )
        .with_file_path(PathBuf::from("/path/to/session.jsonl"));

        assert_eq!(session.id, session_id);
        assert_eq!(session.project_path, PathBuf::from("/project"));
        assert_eq!(session.created_at, created_at);
        assert_eq!(session.first_message, Some("First message".to_string()));
        assert_eq!(
            session.file_path,
            Some(PathBuf::from("/path/to/session.jsonl"))
        );
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, std::time::Duration::from_secs(300));
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig {
            ttl: std::time::Duration::from_secs(600),
            enabled: false,
        };

        let cache = SessionCache::new(config.clone());
        assert_eq!(cache.config().ttl, std::time::Duration::from_secs(600));
        assert!(!cache.config().enabled);
    }

    #[test]
    fn test_cache_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(SessionCache::default());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cache_clone = Arc::clone(&cache);
                thread::spawn(move || {
                    let project = Project::new(
                        format!("project-{}", i),
                        PathBuf::from(format!("/test/{}", i)),
                        vec![],
                    );
                    cache_clone.set_projects(vec![project]);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Cache should have some data (last write wins)
        assert!(!cache.is_empty());
    }
}
