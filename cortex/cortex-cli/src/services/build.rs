//! Build and test execution service
//!
//! Provides unified build and test operations for both API and MCP modules.
//! Handles build jobs, test runs, artifacts, and coverage reporting.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Build service for managing build jobs and test runs
#[derive(Clone)]
pub struct BuildService {
    storage: Arc<ConnectionManager>,
    build_jobs: Arc<RwLock<HashMap<String, BuildJob>>>,
    test_runs: Arc<RwLock<HashMap<String, TestRun>>>,
}

impl BuildService {
    /// Create a new build service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            build_jobs: Arc::new(RwLock::new(HashMap::new())),
            test_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ========================================================================
    // Build Management
    // ========================================================================

    /// Trigger a new build
    pub async fn trigger_build(
        &self,
        workspace_id: Uuid,
        build_config: BuildConfig,
    ) -> Result<BuildJob> {
        info!(
            "Triggering {} build for workspace: {}",
            build_config.build_type, workspace_id
        );

        let job_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        let job = BuildJob {
            id: job_id.clone(),
            workspace_id,
            build_type: build_config.build_type.clone(),
            target: build_config.target,
            features: build_config.features,
            status: BuildStatus::Queued,
            progress: 0.0,
            current_step: Some("Initializing build".to_string()),
            started_at,
            completed_at: None,
            duration_seconds: None,
            artifacts: vec![],
            error_message: None,
        };

        // Store the job
        {
            let mut jobs = self.build_jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }

        // Spawn background task to run the build
        let service = self.clone();
        let job_id_clone = job_id.clone();
        tokio::spawn(async move {
            if let Err(e) = service.execute_build(&job_id_clone).await {
                error!(job_id = %job_id_clone, error = %e, "Build failed");

                // Update job status to failed
                let mut jobs = service.build_jobs.write().await;
                if let Some(job) = jobs.get_mut(&job_id_clone) {
                    job.status = BuildStatus::Failed;
                    job.completed_at = Some(Utc::now());
                    job.duration_seconds = Some(
                        (job.completed_at.unwrap() - job.started_at)
                            .num_seconds() as u64
                    );
                    job.error_message = Some(e.to_string());
                }
            }
        });

        info!("Build job queued: {}", job_id);

        Ok(job)
    }

    /// Get build status
    pub async fn get_build_status(&self, build_id: &str) -> Result<Option<BuildStatus>> {
        debug!("Getting build status: {}", build_id);

        let jobs = self.build_jobs.read().await;
        Ok(jobs.get(build_id).map(|job| job.status.clone()))
    }

    /// Get build job details
    pub async fn get_build(&self, build_id: &str) -> Result<Option<BuildJob>> {
        debug!("Getting build: {}", build_id);

        let jobs = self.build_jobs.read().await;
        Ok(jobs.get(build_id).cloned())
    }

    /// List builds for a workspace
    pub async fn list_builds(&self, workspace_id: Uuid, limit: usize) -> Result<Vec<BuildInfo>> {
        debug!("Listing builds for workspace: {}", workspace_id);

        let jobs = self.build_jobs.read().await;

        let mut builds: Vec<BuildInfo> = jobs
            .values()
            .filter(|job| job.workspace_id == workspace_id)
            .map(|job| BuildInfo {
                id: job.id.clone(),
                workspace_id: job.workspace_id,
                build_type: job.build_type.clone(),
                status: job.status.clone(),
                started_at: job.started_at,
                completed_at: job.completed_at,
                duration_seconds: job.duration_seconds,
            })
            .collect();

        // Sort by started_at descending
        builds.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        // Apply limit
        builds.truncate(limit);

        Ok(builds)
    }

    /// Cancel a build
    pub async fn cancel_build(&self, build_id: &str) -> Result<()> {
        info!("Cancelling build: {}", build_id);

        let mut jobs = self.build_jobs.write().await;
        let job = jobs
            .get_mut(build_id)
            .ok_or_else(|| anyhow!("Build job not found"))?;

        if job.status == BuildStatus::Queued || job.status == BuildStatus::Running {
            job.status = BuildStatus::Cancelled;
            job.completed_at = Some(Utc::now());
            job.duration_seconds = Some(
                (job.completed_at.unwrap() - job.started_at)
                    .num_seconds() as u64
            );
            info!("Build cancelled: {}", build_id);
        } else {
            warn!("Build {} cannot be cancelled (status: {:?})", build_id, job.status);
            return Err(anyhow!("Build cannot be cancelled in current state"));
        }

        Ok(())
    }

    /// Get build logs
    pub async fn get_build_logs(
        &self,
        build_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<LogEntry>> {
        debug!("Getting build logs: {} (offset: {}, limit: {})", build_id, offset, limit);

        // In a real implementation, logs would be stored in the database or file system
        // For now, return mock logs
        let mock_logs = vec![
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Build started".to_string(),
            },
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Compiling project...".to_string(),
            },
            LogEntry {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                message: "Build completed successfully".to_string(),
            },
        ];

        let logs = mock_logs
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(logs)
    }

    /// Execute build (background task)
    async fn execute_build(&self, job_id: &str) -> Result<()> {
        // Simulate build steps
        let steps = vec![
            ("Preparing workspace", 0.1),
            ("Running cargo check", 0.3),
            ("Compiling project", 0.6),
            ("Running tests", 0.8),
            ("Creating artifacts", 0.9),
            ("Finalizing", 1.0),
        ];

        for (step, progress) in steps {
            // Check if build was cancelled
            {
                let jobs = self.build_jobs.read().await;
                if let Some(job) = jobs.get(job_id) {
                    if job.status == BuildStatus::Cancelled {
                        return Ok(());
                    }
                }
            }

            // Update job status
            {
                let mut jobs = self.build_jobs.write().await;
                if let Some(job) = jobs.get_mut(job_id) {
                    job.status = BuildStatus::Running;
                    job.progress = progress;
                    job.current_step = Some(step.to_string());
                }
            }

            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        // Mark as completed
        {
            let mut jobs = self.build_jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = BuildStatus::Completed;
                job.progress = 1.0;
                job.completed_at = Some(Utc::now());
                job.duration_seconds = Some(
                    (job.completed_at.unwrap() - job.started_at)
                        .num_seconds() as u64
                );
                job.current_step = Some("Build completed successfully".to_string());

                // Add mock artifacts
                job.artifacts = vec![
                    BuildArtifact {
                        name: "cortex-cli".to_string(),
                        artifact_type: "binary".to_string(),
                        size_bytes: 15_234_567,
                        url: format!("/artifacts/{}/cortex-cli", job_id),
                        hash: Some("sha256:abc123...".to_string()),
                    },
                ];
            }
        }

        info!("Build completed successfully: {}", job_id);

        Ok(())
    }

    // ========================================================================
    // Test Management
    // ========================================================================

    /// Run tests
    pub async fn run_tests(&self, workspace_id: Uuid, test_config: TestConfig) -> Result<TestRun> {
        info!("Running tests for workspace: {}", workspace_id);

        let run_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        let test_run = TestRun {
            id: run_id.clone(),
            workspace_id,
            test_pattern: test_config.test_pattern,
            test_type: test_config.test_type,
            status: TestStatus::Running,
            started_at,
            completed_at: None,
            duration_seconds: None,
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            failures: vec![],
            coverage: None,
        };

        // Store the test run
        {
            let mut runs = self.test_runs.write().await;
            runs.insert(run_id.clone(), test_run.clone());
        }

        // Spawn background task to run tests
        let service = self.clone();
        let run_id_clone = run_id.clone();
        let coverage_enabled = test_config.coverage.unwrap_or(false);
        tokio::spawn(async move {
            if let Err(e) = service.execute_tests(&run_id_clone, coverage_enabled).await {
                error!(run_id = %run_id_clone, error = %e, "Test execution failed");

                let mut runs = service.test_runs.write().await;
                if let Some(run) = runs.get_mut(&run_id_clone) {
                    run.status = TestStatus::Failed;
                    run.completed_at = Some(Utc::now());
                    run.duration_seconds = Some(
                        (run.completed_at.unwrap() - run.started_at)
                            .num_milliseconds() as f64 / 1000.0
                    );
                }
            }
        });

        info!("Test run started: {}", run_id);

        Ok(test_run)
    }

    /// Get test results
    pub async fn get_test_results(&self, test_run_id: &str) -> Result<Option<TestResults>> {
        debug!("Getting test results: {}", test_run_id);

        let runs = self.test_runs.read().await;
        let run = runs.get(test_run_id);

        Ok(run.map(|r| TestResults {
            run_id: r.id.clone(),
            status: r.status.clone(),
            total_tests: r.total_tests,
            passed: r.passed,
            failed: r.failed,
            skipped: r.skipped,
            duration_seconds: r.duration_seconds.unwrap_or(0.0),
            failures: r.failures.clone(),
        }))
    }

    /// List test runs for a workspace
    pub async fn list_test_runs(&self, workspace_id: Uuid, limit: usize) -> Result<Vec<TestRunInfo>> {
        debug!("Listing test runs for workspace: {}", workspace_id);

        let runs = self.test_runs.read().await;

        let mut test_runs: Vec<TestRunInfo> = runs
            .values()
            .filter(|run| run.workspace_id == workspace_id)
            .map(|run| TestRunInfo {
                id: run.id.clone(),
                workspace_id: run.workspace_id,
                status: run.status.clone(),
                started_at: run.started_at,
                completed_at: run.completed_at,
                total_tests: run.total_tests,
                passed: run.passed,
                failed: run.failed,
            })
            .collect();

        // Sort by started_at descending
        test_runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        // Apply limit
        test_runs.truncate(limit);

        Ok(test_runs)
    }

    /// Get test coverage report
    pub async fn get_test_coverage(&self, test_run_id: &str) -> Result<CoverageReport> {
        debug!("Getting test coverage: {}", test_run_id);

        let runs = self.test_runs.read().await;
        let run = runs
            .get(test_run_id)
            .ok_or_else(|| anyhow!("Test run not found"))?;

        run.coverage
            .clone()
            .ok_or_else(|| anyhow!("Coverage not available for this test run"))
    }

    /// Execute tests (background task)
    async fn execute_tests(&self, run_id: &str, coverage_enabled: bool) -> Result<()> {
        // Simulate test execution
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Update with mock results
        {
            let mut runs = self.test_runs.write().await;
            if let Some(run) = runs.get_mut(run_id) {
                run.status = TestStatus::Completed;
                run.completed_at = Some(Utc::now());
                run.duration_seconds = Some(
                    (run.completed_at.unwrap() - run.started_at)
                        .num_milliseconds() as f64 / 1000.0
                );
                run.total_tests = 150;
                run.passed = 145;
                run.failed = 3;
                run.skipped = 2;

                // Add mock failures
                run.failures = vec![
                    TestFailure {
                        test_name: "test_api_endpoint".to_string(),
                        error_message: "Expected 200, got 404".to_string(),
                        stack_trace: Some("at api_tests.rs:42".to_string()),
                        file_path: Some("tests/api_tests.rs".to_string()),
                        line_number: Some(42),
                    },
                    TestFailure {
                        test_name: "test_database_connection".to_string(),
                        error_message: "Connection timeout".to_string(),
                        stack_trace: Some("at db_tests.rs:15".to_string()),
                        file_path: Some("tests/db_tests.rs".to_string()),
                        line_number: Some(15),
                    },
                    TestFailure {
                        test_name: "test_parsing_edge_case".to_string(),
                        error_message: "Assertion failed: expected true, got false".to_string(),
                        stack_trace: Some("at parser_tests.rs:88".to_string()),
                        file_path: Some("tests/parser_tests.rs".to_string()),
                        line_number: Some(88),
                    },
                ];

                // Add coverage if enabled
                if coverage_enabled {
                    run.coverage = Some(CoverageReport {
                        lines_covered: 1250,
                        lines_total: 1500,
                        percentage: 83.3,
                        by_file: vec![
                            FileCoverage {
                                file_path: "src/main.rs".to_string(),
                                lines_covered: 150,
                                lines_total: 180,
                                percentage: 83.3,
                            },
                            FileCoverage {
                                file_path: "src/lib.rs".to_string(),
                                lines_covered: 200,
                                lines_total: 220,
                                percentage: 90.9,
                            },
                        ],
                    });
                }
            }
        }

        info!("Tests completed: {}", run_id);

        Ok(())
    }
}

// ============================================================================
// Types
// ============================================================================

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub build_type: String, // debug, release, test
    pub target: Option<String>,
    pub features: Option<Vec<String>>,
}

/// Build job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildJob {
    pub id: String,
    pub workspace_id: Uuid,
    pub build_type: String,
    pub target: Option<String>,
    pub features: Option<Vec<String>>,
    pub status: BuildStatus,
    pub progress: f64,
    pub current_step: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub artifacts: Vec<BuildArtifact>,
    pub error_message: Option<String>,
}

/// Build status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BuildStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Build information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub id: String,
    pub workspace_id: Uuid,
    pub build_type: String,
    pub status: BuildStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
}

/// Build artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    pub name: String,
    pub artifact_type: String,
    pub size_bytes: u64,
    pub url: String,
    pub hash: Option<String>,
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub test_pattern: Option<String>,
    pub test_type: Option<String>, // unit, integration, all
    pub coverage: Option<bool>,
}

/// Test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub id: String,
    pub workspace_id: Uuid,
    pub test_pattern: Option<String>,
    pub test_type: Option<String>,
    pub status: TestStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<f64>,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failures: Vec<TestFailure>,
    pub coverage: Option<CoverageReport>,
}

/// Test status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Running,
    Completed,
    Failed,
}

/// Test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub run_id: String,
    pub status: TestStatus,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_seconds: f64,
    pub failures: Vec<TestFailure>,
}

/// Test run information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunInfo {
    pub id: String,
    pub workspace_id: Uuid,
    pub status: TestStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
}

/// Test failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<usize>,
}

/// Coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub lines_covered: usize,
    pub lines_total: usize,
    pub percentage: f64,
    pub by_file: Vec<FileCoverage>,
}

/// File coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub file_path: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub percentage: f64,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_status_serialization() {
        let status = BuildStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"running\"");

        let deserialized: BuildStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, BuildStatus::Running);
    }

    #[test]
    fn test_test_status_serialization() {
        let status = TestStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");

        let deserialized: TestStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TestStatus::Completed);
    }

    #[test]
    fn test_coverage_calculation() {
        let coverage = CoverageReport {
            lines_covered: 833,
            lines_total: 1000,
            percentage: 83.3,
            by_file: vec![],
        };

        assert_eq!(coverage.lines_covered, 833);
        assert_eq!(coverage.lines_total, 1000);
        assert!((coverage.percentage - 83.3).abs() < 0.01);
    }
}
