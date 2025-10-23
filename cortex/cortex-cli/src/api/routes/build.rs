//! Build and CI/CD API routes

use crate::api::types::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use cortex_storage::ConnectionManager;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

/// Build job status
#[derive(Debug, Clone)]
struct BuildJob {
    id: String,
    workspace_id: String,
    build_type: String,
    status: String,
    progress: f64,
    current_step: Option<String>,
    started_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
    artifacts: Vec<BuildArtifact>,
}

/// Test run status
#[derive(Debug, Clone)]
struct TestRun {
    id: String,
    workspace_id: String,
    status: String,
    started_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
    total_tests: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    failures: Vec<TestFailure>,
}

/// Context for build routes
#[derive(Clone)]
pub struct BuildContext {
    pub storage: Arc<ConnectionManager>,
    pub build_jobs: Arc<RwLock<HashMap<String, BuildJob>>>,
    pub test_runs: Arc<RwLock<HashMap<String, TestRun>>>,
}

impl BuildContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            storage,
            build_jobs: Arc::new(RwLock::new(HashMap::new())),
            test_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Create build routes
pub fn build_routes(context: BuildContext) -> Router {
    Router::new()
        .route("/api/v1/build/trigger", post(trigger_build))
        .route("/api/v1/build/{id}/status", get(get_build_status))
        .route("/api/v1/test/run", post(run_tests))
        .route("/api/v1/test/{id}/results", get(get_test_results))
        .with_state(context)
}

/// POST /api/v1/build/trigger - Trigger a build
async fn trigger_build(
    State(context): State<BuildContext>,
    Json(request): Json<BuildRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        workspace_id = %request.workspace_id,
        build_type = %request.build_type,
        "Triggering build"
    );

    match trigger_build_impl(&context, request).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::ACCEPTED, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to trigger build");
            let api_response =
                ApiResponse::<BuildResponse>::error(e.to_string(), request_id);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn trigger_build_impl(
    context: &BuildContext,
    request: BuildRequest,
) -> anyhow::Result<BuildResponse> {
    let job_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();

    // Create build job
    let job = BuildJob {
        id: job_id.clone(),
        workspace_id: request.workspace_id.clone(),
        build_type: request.build_type.clone(),
        status: "queued".to_string(),
        progress: 0.0,
        current_step: Some("Initializing build".to_string()),
        started_at,
        completed_at: None,
        artifacts: vec![],
    };

    // Store the job
    {
        let mut jobs = context.build_jobs.write().await;
        jobs.insert(job_id.clone(), job.clone());
    }

    // Spawn background task to run the build
    let context_clone = context.clone();
    let job_id_clone = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = execute_build(&context_clone, &job_id_clone).await {
            error!(job_id = %job_id_clone, error = %e, "Build failed");

            // Update job status to failed
            let mut jobs = context_clone.build_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id_clone) {
                job.status = "failed".to_string();
                job.completed_at = Some(Utc::now());
            }
        }
    });

    Ok(BuildResponse {
        job_id,
        workspace_id: request.workspace_id,
        build_type: request.build_type,
        status: "queued".to_string(),
        started_at,
    })
}

/// Background build execution
async fn execute_build(context: &BuildContext, job_id: &str) -> anyhow::Result<()> {
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
        // Update job status
        {
            let mut jobs = context.build_jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = "running".to_string();
                job.progress = progress;
                job.current_step = Some(step.to_string());
            }
        }

        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    // Mark as completed
    {
        let mut jobs = context.build_jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = "completed".to_string();
            job.progress = 1.0;
            job.completed_at = Some(Utc::now());
            job.current_step = Some("Build completed successfully".to_string());

            // Add mock artifacts
            job.artifacts = vec![
                BuildArtifact {
                    name: "cortex-cli".to_string(),
                    artifact_type: "binary".to_string(),
                    size_bytes: 15_234_567,
                    url: format!("/artifacts/{}/cortex-cli", job_id),
                },
            ];
        }
    }

    Ok(())
}

/// GET /api/v1/build/{id}/status - Get build status
async fn get_build_status(
    State(context): State<BuildContext>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        job_id = %job_id,
        "Getting build status"
    );

    match get_build_status_impl(&context, &job_id).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to get build status");
            let api_response =
                ApiResponse::<BuildStatusResponse>::error(e.to_string(), request_id);
            (StatusCode::NOT_FOUND, Json(api_response)).into_response()
        }
    }
}

async fn get_build_status_impl(
    context: &BuildContext,
    job_id: &str,
) -> anyhow::Result<BuildStatusResponse> {
    let jobs = context.build_jobs.read().await;
    let job = jobs
        .get(job_id)
        .ok_or_else(|| anyhow::anyhow!("Build job not found"))?;

    let duration_seconds = job.completed_at.map(|completed| {
        (completed - job.started_at).num_seconds() as u64
    });

    Ok(BuildStatusResponse {
        job_id: job.id.clone(),
        status: job.status.clone(),
        progress: job.progress,
        current_step: job.current_step.clone(),
        logs_url: Some(format!("/api/v1/build/{}/logs", job_id)),
        started_at: job.started_at,
        completed_at: job.completed_at,
        duration_seconds,
        artifacts: job.artifacts.clone(),
    })
}

/// POST /api/v1/test/run - Run tests
async fn run_tests(
    State(context): State<BuildContext>,
    Json(request): Json<TestRunRequest>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        workspace_id = %request.workspace_id,
        "Running tests"
    );

    match run_tests_impl(&context, request).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::ACCEPTED, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to run tests");
            let api_response =
                ApiResponse::<TestRunResponse>::error(e.to_string(), request_id);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(api_response)).into_response()
        }
    }
}

async fn run_tests_impl(
    context: &BuildContext,
    request: TestRunRequest,
) -> anyhow::Result<TestRunResponse> {
    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();

    // Create test run
    let test_run = TestRun {
        id: run_id.clone(),
        workspace_id: request.workspace_id.clone(),
        status: "running".to_string(),
        started_at,
        completed_at: None,
        total_tests: 0,
        passed: 0,
        failed: 0,
        skipped: 0,
        failures: vec![],
    };

    // Store the test run
    {
        let mut runs = context.test_runs.write().await;
        runs.insert(run_id.clone(), test_run);
    }

    // Spawn background task to run tests
    let context_clone = context.clone();
    let run_id_clone = run_id.clone();
    tokio::spawn(async move {
        if let Err(e) = execute_tests(&context_clone, &run_id_clone).await {
            error!(run_id = %run_id_clone, error = %e, "Test execution failed");

            let mut runs = context_clone.test_runs.write().await;
            if let Some(run) = runs.get_mut(&run_id_clone) {
                run.status = "failed".to_string();
                run.completed_at = Some(Utc::now());
            }
        }
    });

    Ok(TestRunResponse {
        run_id,
        workspace_id: request.workspace_id,
        status: "running".to_string(),
        started_at,
    })
}

/// Background test execution
async fn execute_tests(context: &BuildContext, run_id: &str) -> anyhow::Result<()> {
    // Simulate test execution
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Update with mock results
    {
        let mut runs = context.test_runs.write().await;
        if let Some(run) = runs.get_mut(run_id) {
            run.status = "completed".to_string();
            run.completed_at = Some(Utc::now());
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
        }
    }

    Ok(())
}

/// GET /api/v1/test/{id}/results - Get test results
async fn get_test_results(
    State(context): State<BuildContext>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();

    info!(
        request_id = %request_id,
        run_id = %run_id,
        "Getting test results"
    );

    match get_test_results_impl(&context, &run_id).await {
        Ok(response) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let api_response = ApiResponse::success(response, request_id, duration_ms);
            (StatusCode::OK, Json(api_response)).into_response()
        }
        Err(e) => {
            error!(request_id = %request_id, error = %e, "Failed to get test results");
            let api_response =
                ApiResponse::<TestResultsResponse>::error(e.to_string(), request_id);
            (StatusCode::NOT_FOUND, Json(api_response)).into_response()
        }
    }
}

async fn get_test_results_impl(
    context: &BuildContext,
    run_id: &str,
) -> anyhow::Result<TestResultsResponse> {
    let runs = context.test_runs.read().await;
    let run = runs
        .get(run_id)
        .ok_or_else(|| anyhow::anyhow!("Test run not found"))?;

    let duration_seconds = run.completed_at.map_or(0.0, |completed| {
        (completed - run.started_at).num_milliseconds() as f64 / 1000.0
    });

    // Mock coverage data
    let coverage = if run.status == "completed" {
        Some(CoverageReport {
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
        })
    } else {
        None
    };

    Ok(TestResultsResponse {
        run_id: run.id.clone(),
        status: run.status.clone(),
        total_tests: run.total_tests,
        passed: run.passed,
        failed: run.failed,
        skipped: run.skipped,
        duration_seconds,
        coverage,
        failures: run.failures.clone(),
    })
}
