//! Build and CI/CD API routes

use crate::api::types::*;
use crate::services::build::{BuildService, BuildConfig, TestConfig};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use cortex_storage::ConnectionManager;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

// Note: BuildJob and TestRun are now in BuildService
// We don't need duplicate definitions here

/// Context for build routes
#[derive(Clone)]
pub struct BuildContext {
    pub build_service: Arc<BuildService>,
}

impl BuildContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self {
            build_service: Arc::new(BuildService::new(storage)),
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
    let workspace_uuid = Uuid::parse_str(&request.workspace_id)?;

    let build_config = BuildConfig {
        build_type: request.build_type.clone(),
        target: None,
        features: None,
    };

    // Use BuildService to trigger build
    let job = context.build_service.trigger_build(workspace_uuid, build_config).await?;

    Ok(BuildResponse {
        job_id: job.id.clone(),
        workspace_id: request.workspace_id,
        build_type: request.build_type,
        status: format!("{:?}", job.status).to_lowercase(),
        started_at: job.started_at,
    })
}

// Note: execute_build is now in BuildService - no need for duplicate implementation

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
    // Use BuildService to get build status
    let job = context.build_service.get_build(job_id).await?
        .ok_or_else(|| anyhow::anyhow!("Build job not found"))?;

    Ok(BuildStatusResponse {
        job_id: job.id.clone(),
        status: format!("{:?}", job.status).to_lowercase(),
        progress: job.progress,
        current_step: job.current_step.clone(),
        logs_url: Some(format!("/api/v1/build/{}/logs", job_id)),
        started_at: job.started_at,
        completed_at: job.completed_at,
        duration_seconds: job.duration_seconds,
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
    let workspace_uuid = Uuid::parse_str(&request.workspace_id)?;

    let test_config = TestConfig {
        test_pattern: None,
        test_type: None,
        coverage: Some(false),
    };

    // Use BuildService to run tests
    let test_run = context.build_service.run_tests(workspace_uuid, test_config).await?;

    Ok(TestRunResponse {
        run_id: test_run.id.clone(),
        workspace_id: request.workspace_id,
        status: format!("{:?}", test_run.status).to_lowercase(),
        started_at: test_run.started_at,
    })
}

// Note: execute_tests is now in BuildService - no need for duplicate implementation

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
    // Use BuildService to get test results
    let results = context.build_service.get_test_results(run_id).await?
        .ok_or_else(|| anyhow::anyhow!("Test run not found"))?;

    // Get coverage if available
    let coverage = context.build_service.get_test_coverage(run_id).await.ok();

    Ok(TestResultsResponse {
        run_id: results.run_id,
        status: format!("{:?}", results.status).to_lowercase(),
        total_tests: results.total_tests,
        passed: results.passed,
        failed: results.failed,
        skipped: results.skipped,
        duration_seconds: results.duration_seconds,
        coverage,
        failures: results.failures,
    })
}
