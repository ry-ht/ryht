//! Session filtering and search functionality.
//!
//! This module provides advanced filtering and search capabilities for sessions,
//! including date range filters, content search, and various sorting options.
//!
//! # Examples
//!
//! ```no_run
//! use crate::cc::session::filter::{SessionFilter, SortBy, search_sessions};
//! use chrono::{Utc, Duration};
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Create a filter for recent sessions
//! let filter = SessionFilter::default()
//!     .with_date_range(
//!         Some(Utc::now() - Duration::days(7)),
//!         Some(Utc::now())
//!     )
//!     .with_sort_by(SortBy::CreatedDesc);
//!
//! let sessions = search_sessions(filter).await?;
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use regex::Regex;

use crate::cc::result::Result;
use crate::cc::messages::Message;

use crate::cc::discovery::{list_projects, list_sessions, load_session_history};
use crate::cc::types::Session;

/// Sort order for sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort by creation time (oldest first)
    CreatedAsc,
    /// Sort by creation time (newest first)
    CreatedDesc,
    /// Sort by last modified (oldest first)
    ModifiedAsc,
    /// Sort by last modified (newest first)
    ModifiedDesc,
    /// Sort by message count (lowest first)
    MessageCountAsc,
    /// Sort by message count (highest first)
    MessageCountDesc,
}

impl Default for SortBy {
    fn default() -> Self {
        Self::CreatedDesc
    }
}

/// Filter options for session queries.
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Filter by project ID
    pub project_id: Option<String>,
    /// Filter by creation date range (start, end)
    pub date_range: Option<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)>,
    /// Search for text in session messages
    pub content_search: Option<String>,
    /// Use regex for content search
    pub regex_search: bool,
    /// Case-sensitive search
    pub case_sensitive: bool,
    /// Filter by minimum message count
    pub min_messages: Option<usize>,
    /// Filter by maximum message count
    pub max_messages: Option<usize>,
    /// Sort order
    pub sort_by: SortBy,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Skip first N results (for pagination)
    pub offset: Option<usize>,
}

impl SessionFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by project ID.
    pub fn with_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Filter by date range.
    ///
    /// Both start and end are optional. If start is None, no lower bound is applied.
    /// If end is None, no upper bound is applied.
    pub fn with_date_range(
        mut self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        self.date_range = Some((start, end));
        self
    }

    /// Search for text in session messages.
    pub fn with_content_search(mut self, search: impl Into<String>) -> Self {
        self.content_search = Some(search.into());
        self
    }

    /// Enable regex search.
    pub fn with_regex(mut self, enabled: bool) -> Self {
        self.regex_search = enabled;
        self
    }

    /// Set case sensitivity.
    pub fn with_case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Filter by minimum message count.
    pub fn with_min_messages(mut self, min: usize) -> Self {
        self.min_messages = Some(min);
        self
    }

    /// Filter by maximum message count.
    pub fn with_max_messages(mut self, max: usize) -> Self {
        self.max_messages = Some(max);
        self
    }

    /// Set sort order.
    pub fn with_sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    /// Limit number of results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip first N results.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Extended session information with metadata.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// The session
    pub session: Session,
    /// Number of messages in the session
    pub message_count: usize,
    /// Last modified time (if available)
    pub last_modified: Option<DateTime<Utc>>,
}

/// Search for sessions matching the given filter.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::filter::{SessionFilter, search_sessions};
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let filter = SessionFilter::new()
///     .with_project_id("my-project")
///     .with_limit(10);
///
/// let sessions = search_sessions(filter).await?;
/// println!("Found {} sessions", sessions.len());
/// # Ok(())
/// # }
/// ```
pub async fn search_sessions(filter: SessionFilter) -> Result<Vec<SessionInfo>> {
    // Get all projects or filter by specific project
    let projects = if let Some(ref project_id) = filter.project_id {
        let all_projects = list_projects().await?;
        all_projects
            .into_iter()
            .filter(|p| p.id == *project_id)
            .collect()
    } else {
        list_projects().await?
    };

    let mut results = Vec::new();

    // Collect sessions from all matching projects
    for project in projects {
        let sessions = list_sessions(&project.id).await?;

        for session in sessions {
            // Apply date range filter
            if let Some((start, end)) = filter.date_range {
                if let Some(start) = start {
                    if session.created_at < start {
                        continue;
                    }
                }
                if let Some(end) = end {
                    if session.created_at > end {
                        continue;
                    }
                }
            }

            // For content search and message count, we need to load the session
            let needs_full_load = filter.content_search.is_some()
                || filter.min_messages.is_some()
                || filter.max_messages.is_some();

            let (message_count, last_modified) = if needs_full_load {
                let messages = load_session_history(&session.id).await?;
                let count = messages.len();

                // Apply message count filters
                if let Some(min) = filter.min_messages {
                    if count < min {
                        continue;
                    }
                }
                if let Some(max) = filter.max_messages {
                    if count > max {
                        continue;
                    }
                }

                // Apply content search
                if let Some(ref search_text) = filter.content_search {
                    if !matches_content_search(&messages, search_text, &filter) {
                        continue;
                    }
                }

                // Get file metadata for last modified time
                let last_modified = if let Some(ref file_path) = session.file_path {
                    tokio::fs::metadata(file_path)
                        .await
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                        .and_then(|modified| {
                            use std::time::SystemTime;
                            let duration = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
                            DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                        })
                } else {
                    None
                };

                (count, last_modified)
            } else {
                // Even without full load, try to get last modified from file metadata
                let last_modified = if let Some(ref file_path) = session.file_path {
                    tokio::fs::metadata(file_path)
                        .await
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                        .and_then(|modified| {
                            use std::time::SystemTime;
                            let duration = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
                            DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                        })
                } else {
                    None
                };

                (0, last_modified)
            };

            results.push(SessionInfo {
                session,
                message_count,
                last_modified,
            });
        }
    }

    // Sort results
    sort_sessions(&mut results, filter.sort_by);

    // Apply pagination
    if let Some(offset) = filter.offset {
        if offset < results.len() {
            results = results.into_iter().skip(offset).collect();
        } else {
            results.clear();
        }
    }

    if let Some(limit) = filter.limit {
        results.truncate(limit);
    }

    Ok(results)
}

/// Search for sessions by content.
///
/// This is a convenience function that searches all sessions for the given text.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::filter::search_by_content;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let sessions = search_by_content("error", false, false).await?;
/// # Ok(())
/// # }
/// ```
pub async fn search_by_content(
    search_text: &str,
    regex: bool,
    case_sensitive: bool,
) -> Result<Vec<SessionInfo>> {
    let filter = SessionFilter::new()
        .with_content_search(search_text)
        .with_regex(regex)
        .with_case_sensitive(case_sensitive);

    search_sessions(filter).await
}

/// Filter sessions by date range.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::filter::filter_by_date_range;
/// use chrono::{Utc, Duration};
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let start = Utc::now() - Duration::days(7);
/// let end = Utc::now();
/// let sessions = filter_by_date_range(Some(start), Some(end)).await?;
/// # Ok(())
/// # }
/// ```
pub async fn filter_by_date_range(
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<Vec<SessionInfo>> {
    let filter = SessionFilter::new().with_date_range(start, end);
    search_sessions(filter).await
}

/// Filter sessions by project.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::filter::filter_by_project;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let sessions = filter_by_project("my-project").await?;
/// # Ok(())
/// # }
/// ```
pub async fn filter_by_project(project_id: &str) -> Result<Vec<SessionInfo>> {
    let filter = SessionFilter::new().with_project_id(project_id);
    search_sessions(filter).await
}

/// Check if messages match the content search criteria.
fn matches_content_search(messages: &[Message], search_text: &str, filter: &SessionFilter) -> bool {
    if filter.regex_search {
        // Build regex with case sensitivity flag
        let pattern = if filter.case_sensitive {
            search_text.to_string()
        } else {
            format!("(?i){}", search_text)
        };

        let Ok(regex) = Regex::new(&pattern) else {
            return false;
        };

        messages.iter().any(|msg| {
            let content = extract_message_content(msg);
            regex.is_match(&content)
        })
    } else {
        // Simple text search
        messages.iter().any(|msg| {
            let content = extract_message_content(msg);
            if filter.case_sensitive {
                content.contains(search_text)
            } else {
                content.to_lowercase().contains(&search_text.to_lowercase())
            }
        })
    }
}

/// Extract text content from a message.
fn extract_message_content(message: &Message) -> String {
    match message {
        Message::User { message } => message.content.clone(),
        Message::Assistant { message } => {
            // Extract text from all content blocks
            message
                .content
                .iter()
                .filter_map(|block| {
                    if let crate::messages::ContentBlock::Text(text_content) = block {
                        Some(text_content.text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        _ => String::new(),
    }
}

/// Sort sessions by the specified criteria.
fn sort_sessions(sessions: &mut [SessionInfo], sort_by: SortBy) {
    match sort_by {
        SortBy::CreatedAsc => {
            sessions.sort_by(|a, b| a.session.created_at.cmp(&b.session.created_at));
        }
        SortBy::CreatedDesc => {
            sessions.sort_by(|a, b| b.session.created_at.cmp(&a.session.created_at));
        }
        SortBy::ModifiedAsc => {
            sessions.sort_by(|a, b| {
                match (a.last_modified, b.last_modified) {
                    (Some(a_mod), Some(b_mod)) => a_mod.cmp(&b_mod),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        SortBy::ModifiedDesc => {
            sessions.sort_by(|a, b| {
                match (a.last_modified, b.last_modified) {
                    (Some(a_mod), Some(b_mod)) => b_mod.cmp(&a_mod),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        SortBy::MessageCountAsc => {
            sessions.sort_by(|a, b| a.message_count.cmp(&b.message_count));
        }
        SortBy::MessageCountDesc => {
            sessions.sort_by(|a, b| b.message_count.cmp(&a.message_count));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_builder() {
        let filter = SessionFilter::new()
            .with_project_id("test")
            .with_limit(10)
            .with_offset(5)
            .with_case_sensitive(true);

        assert_eq!(filter.project_id, Some("test".to_string()));
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, Some(5));
        assert!(filter.case_sensitive);
    }

    #[test]
    fn test_sort_by_default() {
        assert_eq!(SortBy::default(), SortBy::CreatedDesc);
    }

    // Property-based tests
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // Strategy for generating SortBy
        fn sort_by_strategy() -> impl Strategy<Value = SortBy> {
            prop_oneof![
                Just(SortBy::CreatedAsc),
                Just(SortBy::CreatedDesc),
                Just(SortBy::ModifiedAsc),
                Just(SortBy::ModifiedDesc),
                Just(SortBy::MessageCountAsc),
                Just(SortBy::MessageCountDesc),
            ]
        }

        proptest! {
            // SessionFilter builder property tests
            fn filter_builder_project_id_preserved(project_id in "[a-zA-Z0-9_-]{1,50}") {
                let filter = SessionFilter::new().with_project_id(project_id.clone());
                prop_assert_eq!(filter.project_id, Some(project_id));
            }


            fn filter_builder_limit_preserved(limit in 1usize..1000) {
                let filter = SessionFilter::new().with_limit(limit);
                prop_assert_eq!(filter.limit, Some(limit));
            }


            fn filter_builder_offset_preserved(offset in 0usize..1000) {
                let filter = SessionFilter::new().with_offset(offset);
                prop_assert_eq!(filter.offset, Some(offset));
            }


            fn filter_builder_min_messages_preserved(min in 0usize..100) {
                let filter = SessionFilter::new().with_min_messages(min);
                prop_assert_eq!(filter.min_messages, Some(min));
            }


            fn filter_builder_max_messages_preserved(max in 1usize..1000) {
                let filter = SessionFilter::new().with_max_messages(max);
                prop_assert_eq!(filter.max_messages, Some(max));
            }


            fn filter_builder_content_search_preserved(search in "\\PC{1,100}") {
                let filter = SessionFilter::new().with_content_search(search.clone());
                prop_assert_eq!(filter.content_search, Some(search));
            }


            fn filter_builder_regex_preserved(regex in prop::bool::ANY) {
                let filter = SessionFilter::new().with_regex(regex);
                prop_assert_eq!(filter.regex_search, regex);
            }


            fn filter_builder_case_sensitive_preserved(case_sensitive in prop::bool::ANY) {
                let filter = SessionFilter::new().with_case_sensitive(case_sensitive);
                prop_assert_eq!(filter.case_sensitive, case_sensitive);
            }


            fn filter_builder_sort_by_preserved(sort_by in sort_by_strategy()) {
                let filter = SessionFilter::new().with_sort_by(sort_by);
                prop_assert_eq!(filter.sort_by, sort_by);
            }

            // Filter composition property tests

            fn filter_builder_composition_order_independent(
                project_id in "[a-zA-Z0-9_-]{1,30}",
                limit in 1usize..100,
                offset in 0usize..50
            ) {
                // Build filter in different orders
                let filter1 = SessionFilter::new()
                    .with_project_id(project_id.clone())
                    .with_limit(limit)
                    .with_offset(offset);

                let filter2 = SessionFilter::new()
                    .with_offset(offset)
                    .with_project_id(project_id.clone())
                    .with_limit(limit);

                let filter3 = SessionFilter::new()
                    .with_limit(limit)
                    .with_offset(offset)
                    .with_project_id(project_id.clone());

                // All should have the same final values
                prop_assert_eq!(&filter1.project_id, &filter2.project_id);
                prop_assert_eq!(&filter2.project_id, &filter3.project_id);
                prop_assert_eq!(&filter1.limit, &filter2.limit);
                prop_assert_eq!(&filter2.limit, &filter3.limit);
                prop_assert_eq!(&filter1.offset, &filter2.offset);
                prop_assert_eq!(&filter2.offset, &filter3.offset);
            }


            fn filter_builder_chaining_consistent(
                search in "\\PC{1,50}",
                min in 1usize..50,
                max in 51usize..100
            ) {
                let filter = SessionFilter::new()
                    .with_content_search(search.clone())
                    .with_min_messages(min)
                    .with_max_messages(max);

                prop_assert_eq!(filter.content_search, Some(search));
                prop_assert_eq!(filter.min_messages, Some(min));
                prop_assert_eq!(filter.max_messages, Some(max));
            }

            // SortBy property tests

            fn sort_by_all_variants_distinct(sort1 in sort_by_strategy(), sort2 in sort_by_strategy()) {
                // If they're equal, they should be the same variant
                if sort1 == sort2 {
                    prop_assert!(matches!(
                        (sort1, sort2),
                        (SortBy::CreatedAsc, SortBy::CreatedAsc) |
                        (SortBy::CreatedDesc, SortBy::CreatedDesc) |
                        (SortBy::ModifiedAsc, SortBy::ModifiedAsc) |
                        (SortBy::ModifiedDesc, SortBy::ModifiedDesc) |
                        (SortBy::MessageCountAsc, SortBy::MessageCountAsc) |
                        (SortBy::MessageCountDesc, SortBy::MessageCountDesc)
                    ));
                }
            }

            // Filter validation property tests

            fn filter_min_max_messages_relationship(
                min in 0usize..50,
                max in 51usize..100
            ) {
                let filter = SessionFilter::new()
                    .with_min_messages(min)
                    .with_max_messages(max);

                // Min should be less than max (this is a logical constraint, not enforced by the type)
                prop_assert!(filter.min_messages.unwrap() < filter.max_messages.unwrap());
            }


            fn filter_default_has_sensible_values(_dummy in 0u32..1) {
                let filter = SessionFilter::default();

                prop_assert!(filter.project_id.is_none());
                prop_assert!(filter.date_range.is_none());
                prop_assert!(filter.content_search.is_none());
                prop_assert!(!filter.regex_search);
                prop_assert!(!filter.case_sensitive);
                prop_assert!(filter.min_messages.is_none());
                prop_assert!(filter.max_messages.is_none());
                prop_assert_eq!(filter.sort_by, SortBy::CreatedDesc);
                prop_assert!(filter.limit.is_none());
                prop_assert!(filter.offset.is_none());
            }

            // SessionFilter immutability tests

            fn filter_builder_immutable_original(
                project_id1 in "[a-zA-Z0-9_-]{1,30}",
                project_id2 in "[a-zA-Z0-9_-]{1,30}"
            ) {
                let filter1 = SessionFilter::new().with_project_id(project_id1.clone());
                let filter2 = filter1.clone().with_project_id(project_id2.clone());

                // Original should be unchanged
                prop_assert_eq!(filter1.project_id, Some(project_id1));
                // New filter should have new value
                prop_assert_eq!(filter2.project_id, Some(project_id2));
            }

            // Complex filter composition tests

            fn filter_all_options_can_be_set(
                project_id in "[a-zA-Z0-9_-]{1,30}",
                search in "\\PC{1,50}",
                min_msg in 1usize..20,
                max_msg in 21usize..50,
                limit in 1usize..100,
                offset in 0usize..50,
                regex in prop::bool::ANY,
                case_sensitive in prop::bool::ANY,
                sort_by in sort_by_strategy()
            ) {
                let filter = SessionFilter::new()
                    .with_project_id(project_id.clone())
                    .with_content_search(search.clone())
                    .with_min_messages(min_msg)
                    .with_max_messages(max_msg)
                    .with_limit(limit)
                    .with_offset(offset)
                    .with_regex(regex)
                    .with_case_sensitive(case_sensitive)
                    .with_sort_by(sort_by);

                // Verify all values are set correctly
                prop_assert_eq!(filter.project_id, Some(project_id));
                prop_assert_eq!(filter.content_search, Some(search));
                prop_assert_eq!(filter.min_messages, Some(min_msg));
                prop_assert_eq!(filter.max_messages, Some(max_msg));
                prop_assert_eq!(filter.limit, Some(limit));
                prop_assert_eq!(filter.offset, Some(offset));
                prop_assert_eq!(filter.regex_search, regex);
                prop_assert_eq!(filter.case_sensitive, case_sensitive);
                prop_assert_eq!(filter.sort_by, sort_by);
            }
        }
    }
}
