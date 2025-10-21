//! Integration tests for session filtering and search.
//!
//! Tests filtering by date, content, message count, and sorting.

use cc_sdk::core::SessionId;
use cc_sdk::session::{SessionFilter, SortBy, Session};
use chrono::{Duration, Utc};
use std::path::PathBuf;

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
fn test_session_filter_default() {
    let filter = SessionFilter::default();

    assert!(filter.project_id.is_none());
    assert!(filter.date_range.is_none());
    assert!(filter.content_search.is_none());
    assert!(!filter.regex_search);
    assert!(!filter.case_sensitive);
    assert!(filter.min_messages.is_none());
    assert!(filter.max_messages.is_none());
    assert_eq!(filter.sort_by, SortBy::CreatedDesc);
    assert!(filter.limit.is_none());
    assert!(filter.offset.is_none());
}

#[test]
fn test_session_filter_project_id_only() {
    let filter = SessionFilter::new().with_project_id("my-project");

    assert_eq!(filter.project_id, Some("my-project".to_string()));
    assert!(filter.date_range.is_none());
    assert!(filter.content_search.is_none());
}

#[test]
fn test_session_filter_date_range() {
    let now = Utc::now();
    let week_ago = now - Duration::days(7);

    let filter = SessionFilter::new().with_date_range(Some(week_ago), Some(now));

    assert!(filter.date_range.is_some());
    let (start, end) = filter.date_range.unwrap();
    assert_eq!(start, Some(week_ago));
    assert_eq!(end, Some(now));
}

#[test]
fn test_session_filter_date_range_start_only() {
    let week_ago = Utc::now() - Duration::days(7);

    let filter = SessionFilter::new().with_date_range(Some(week_ago), None);

    assert!(filter.date_range.is_some());
    let (start, end) = filter.date_range.unwrap();
    assert_eq!(start, Some(week_ago));
    assert_eq!(end, None);
}

#[test]
fn test_session_filter_date_range_end_only() {
    let now = Utc::now();

    let filter = SessionFilter::new().with_date_range(None, Some(now));

    assert!(filter.date_range.is_some());
    let (start, end) = filter.date_range.unwrap();
    assert_eq!(start, None);
    assert_eq!(end, Some(now));
}

#[test]
fn test_session_filter_content_search() {
    let filter = SessionFilter::new()
        .with_content_search("error")
        .with_regex(false)
        .with_case_sensitive(true);

    assert_eq!(filter.content_search, Some("error".to_string()));
    assert!(!filter.regex_search);
    assert!(filter.case_sensitive);
}

#[test]
fn test_session_filter_regex_search() {
    let filter = SessionFilter::new()
        .with_content_search(r"error\s+\d+")
        .with_regex(true);

    assert_eq!(filter.content_search, Some(r"error\s+\d+".to_string()));
    assert!(filter.regex_search);
}

#[test]
fn test_session_filter_message_count_range() {
    let filter = SessionFilter::new()
        .with_min_messages(10)
        .with_max_messages(50);

    assert_eq!(filter.min_messages, Some(10));
    assert_eq!(filter.max_messages, Some(50));
}

#[test]
fn test_session_filter_min_messages_only() {
    let filter = SessionFilter::new().with_min_messages(5);

    assert_eq!(filter.min_messages, Some(5));
    assert!(filter.max_messages.is_none());
}

#[test]
fn test_session_filter_max_messages_only() {
    let filter = SessionFilter::new().with_max_messages(20);

    assert!(filter.min_messages.is_none());
    assert_eq!(filter.max_messages, Some(20));
}

#[test]
fn test_sort_by_variants() {
    let variants = vec![
        SortBy::CreatedAsc,
        SortBy::CreatedDesc,
        SortBy::ModifiedAsc,
        SortBy::ModifiedDesc,
        SortBy::MessageCountAsc,
        SortBy::MessageCountDesc,
    ];

    // All should be distinct
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
fn test_sort_by_default() {
    let sort = SortBy::default();
    assert_eq!(sort, SortBy::CreatedDesc);
}

#[test]
fn test_session_filter_with_sort_by() {
    let sorts = vec![
        SortBy::CreatedAsc,
        SortBy::CreatedDesc,
        SortBy::ModifiedAsc,
        SortBy::ModifiedDesc,
        SortBy::MessageCountAsc,
        SortBy::MessageCountDesc,
    ];

    for sort in sorts {
        let filter = SessionFilter::new().with_sort_by(sort);
        assert_eq!(filter.sort_by, sort);
    }
}

#[test]
fn test_session_filter_pagination() {
    let filter = SessionFilter::new().with_limit(20).with_offset(10);

    assert_eq!(filter.limit, Some(20));
    assert_eq!(filter.offset, Some(10));
}

#[test]
fn test_session_filter_limit_only() {
    let filter = SessionFilter::new().with_limit(10);

    assert_eq!(filter.limit, Some(10));
    assert!(filter.offset.is_none());
}

#[test]
fn test_session_filter_offset_only() {
    let filter = SessionFilter::new().with_offset(5);

    assert!(filter.limit.is_none());
    assert_eq!(filter.offset, Some(5));
}

#[test]
fn test_session_filter_chaining() {
    let now = Utc::now();

    let filter = SessionFilter::new()
        .with_project_id("project-1")
        .with_date_range(Some(now - Duration::days(30)), Some(now))
        .with_content_search("TODO")
        .with_regex(false)
        .with_case_sensitive(false)
        .with_min_messages(1)
        .with_max_messages(1000)
        .with_sort_by(SortBy::ModifiedDesc)
        .with_limit(50)
        .with_offset(0);

    assert_eq!(filter.project_id, Some("project-1".to_string()));
    assert!(filter.date_range.is_some());
    assert_eq!(filter.content_search, Some("TODO".to_string()));
    assert!(!filter.regex_search);
    assert!(!filter.case_sensitive);
    assert_eq!(filter.min_messages, Some(1));
    assert_eq!(filter.max_messages, Some(1000));
    assert_eq!(filter.sort_by, SortBy::ModifiedDesc);
    assert_eq!(filter.limit, Some(50));
    assert_eq!(filter.offset, Some(0));
}

#[test]
fn test_session_filter_clone() {
    let filter = SessionFilter::new()
        .with_project_id("test")
        .with_limit(10);

    let cloned = filter.clone();

    assert_eq!(cloned.project_id, filter.project_id);
    assert_eq!(cloned.limit, filter.limit);
}

#[test]
fn test_sort_by_copy() {
    let sort1 = SortBy::CreatedDesc;
    let sort2 = sort1; // Should copy

    assert_eq!(sort1, sort2);
    assert_eq!(sort1, SortBy::CreatedDesc); // sort1 still usable
}

#[tokio::test]
#[ignore = "Requires mock session data"]
async fn test_search_sessions_with_filter() {
    // This test would require setting up mock session data
    // In a real implementation, we would:
    // 1. Create mock sessions in a temporary directory
    // 2. Apply various filters
    // 3. Verify results match expectations

    println!("Search sessions test would run with mock data");
}

#[test]
fn test_filter_application_logic() {
    // Test the logic for applying filters (without actual search)
    let sessions = vec![
        Session::new(
            SessionId::new("session-1"),
            PathBuf::from("/project"),
            Utc::now() - Duration::days(5),
            Some("Message with error".to_string()),
        ),
        Session::new(
            SessionId::new("session-2"),
            PathBuf::from("/project"),
            Utc::now() - Duration::days(3),
            Some("Normal message".to_string()),
        ),
        Session::new(
            SessionId::new("session-3"),
            PathBuf::from("/project"),
            Utc::now() - Duration::days(1),
            Some("Another message with error".to_string()),
        ),
    ];

    // Simulate filtering by content (would be in real search_sessions function)
    let search_term = "error";
    let filtered: Vec<_> = sessions
        .iter()
        .filter(|s| {
            s.first_message
                .as_ref()
                .map(|m| m.contains(search_term))
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].id.as_str(), "session-1");
    assert_eq!(filtered[1].id.as_str(), "session-3");
}

#[test]
fn test_filter_by_date_range_logic() {
    let now = Utc::now();
    let week_ago = now - Duration::days(7);
    let month_ago = now - Duration::days(30);

    let sessions = vec![
        Session::new(
            SessionId::new("old"),
            PathBuf::from("/project"),
            month_ago,
            None,
        ),
        Session::new(
            SessionId::new("recent"),
            PathBuf::from("/project"),
            now - Duration::days(3),
            None,
        ),
        Session::new(
            SessionId::new("very-recent"),
            PathBuf::from("/project"),
            now,
            None,
        ),
    ];

    // Simulate date range filtering
    let filtered: Vec<_> = sessions
        .iter()
        .filter(|s| s.created_at >= week_ago && s.created_at <= now)
        .collect();

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].id.as_str(), "recent");
    assert_eq!(filtered[1].id.as_str(), "very-recent");
}

#[test]
fn test_filter_by_message_count_logic() {
    // Simulate sessions with different message counts
    let sessions_with_counts = vec![
        ("session-1", 5),
        ("session-2", 15),
        ("session-3", 25),
        ("session-4", 50),
    ];

    let min_messages = 10;
    let max_messages = 30;

    // Simulate filtering
    let filtered: Vec<_> = sessions_with_counts
        .iter()
        .filter(|(_, count)| *count >= min_messages && *count <= max_messages)
        .collect();

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].0, "session-2");
    assert_eq!(filtered[1].0, "session-3");
}

#[test]
fn test_sorting_logic() {
    let now = Utc::now();

    let mut sessions = vec![
        Session::new(
            SessionId::new("newest"),
            PathBuf::from("/project"),
            now,
            None,
        ),
        Session::new(
            SessionId::new("oldest"),
            PathBuf::from("/project"),
            now - Duration::days(10),
            None,
        ),
        Session::new(
            SessionId::new("middle"),
            PathBuf::from("/project"),
            now - Duration::days(5),
            None,
        ),
    ];

    // Sort by created ascending
    sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    assert_eq!(sessions[0].id.as_str(), "oldest");
    assert_eq!(sessions[1].id.as_str(), "middle");
    assert_eq!(sessions[2].id.as_str(), "newest");

    // Sort by created descending
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    assert_eq!(sessions[0].id.as_str(), "newest");
    assert_eq!(sessions[1].id.as_str(), "middle");
    assert_eq!(sessions[2].id.as_str(), "oldest");
}

#[test]
fn test_pagination_logic() {
    let sessions: Vec<_> = (0..50)
        .map(|i| {
            Session::new(
                SessionId::new(&format!("session-{}", i)),
                PathBuf::from("/project"),
                Utc::now(),
                None,
            )
        })
        .collect();

    // Simulate pagination: limit=10, offset=20
    let limit = 10;
    let offset = 20;

    let paginated: Vec<_> = sessions.iter().skip(offset).take(limit).collect();

    assert_eq!(paginated.len(), 10);
    assert_eq!(paginated[0].id.as_str(), "session-20");
    assert_eq!(paginated[9].id.as_str(), "session-29");
}
