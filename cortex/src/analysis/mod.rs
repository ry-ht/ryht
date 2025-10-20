/// Code analysis module
///
/// Provides automated code health analysis and improvement recommendations.

pub mod code_health;

pub use code_health::{
    CodeHealthAnalyzer, HealthAnalysisResult, HealthIssue, HealthSummary,
    IssueSeverity, IssueCategory, Recommendation,
};
